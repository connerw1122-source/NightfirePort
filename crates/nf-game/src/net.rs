//! The UDP lockstep session — a port feature (the PS2 game has no online mode; there is no game
//! counterpart to cite). Plan (main agent, 2026-10-15): deterministic lockstep over UDP — every
//! machine runs the same MP match from the same seed and settings and exchanges only each agent's
//! per-tick input; a per-tick state hash detects desyncs. Direct connect: the host listens on a UDP
//! port, clients join by address (internet play needs the host to forward the port; no relay).
//!
//! This is the session layer task 051 (`net-lockstep-core`) describes, kept in `nf_game::net`
//! rather than a new crate: it needs `mp::MpSettings` and `player::Input` directly, it has no
//! rendering, and 052's lobby task already showed that building the same networking module twice
//! in parallel collides. `std::net::UdpSocket` only — no new dependencies.
//!
//! Wire protocol (little-endian, one tag byte first):
//! - `HELLO 1`  client → host: protocol version u16, build id u32, codename (len u8 + bytes).
//! - `WELCOME 2` host → client: the assigned slot u8.
//! - `DENY 3`   host → client: reason u8 (1 version/build mismatch, 2 lobby full, 3 match started).
//! - `START 4`  host → all: the serialized `MpSettings` + seed — every machine builds the same
//!   level from it ([S] the port's own handover; the game's local flow builds settings in
//!   `P_MPCONFIRM` 0x20ec00 and the port's confirm page mirrors it).
//! - `INPUT 5`  both ways: n u8, then n × { tick u32, packed `Input` (20 bytes) } — each packet
//!   repeats the last `REDUNDANCY` ticks' inputs, so a lost packet is repaired by the next one
//!   without an ack/resend bookkeeping pass.
//! - `HASH 6`   both ways: tick u32, hash u64 — exchanged every N ticks (the app chooses N and
//!   calls [`NetSession::send_state_hash`]); a mismatch at the same tick is a desync.
//! - `LEAVE 7`  both ways: polite goodbye; the receiver ends the match (PeerLost).
//!
//! Lockstep rule: the sim advances tick t only when [`NetSession::ready_inputs`] returns every
//! live agent's input for t; a missing input stalls the tick (never guess — the game's own netplay
//! has no analogue to cite; [S] port-only design). Timeouts use wall-clock here, OUTSIDE the Sim:
//! the session never feeds the simulation's state, so the determinism rule (no wall clock in Sim)
//! holds.
//!
//! State hash: [`Hasher`] folds the shared deterministic state (every agent's bond pose, the MP
//! match slots/clock, the shared RNG stream, the frame counter) in a fixed slot order. The hashed
//! field set is the port's own choice ([S]); it lives in `nf_view::scene::Sim::net_state_hash`.

use crate::mp::MpSettings;
use crate::player::Input;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

/// Bump on any wire change so mismatched builds refuse cleanly (051's handshake requirement).
pub const PROTOCOL_VERSION: u16 = 1;
/// Identity of the build; a mismatch denies the join (051: "mismatched exes refuse cleanly").
pub const BUILD_ID: u32 = 1;
/// The game's MP limit: 4 agents ([M] the game's four join panels, P_MPJOIN 0x20ca00).
pub const MAX_AGENTS: usize = 4;
/// Remote slots on the host are 1..=3 (slot 0 is the host's own panel).
pub const MAX_CLIENTS: usize = MAX_AGENTS - 1;
/// Each INPUT packet repeats the last N ticks' inputs (051: redundancy).
pub const REDUNDANCY: usize = 3;
/// A silent peer for this long is dropped (051: timeout handling). Outside the Sim.
const PEER_TIMEOUT: Duration = Duration::from_secs(5);
/// A joining client's HELLO re-send interval.
const HELLO_RETRY: Duration = Duration::from_millis(400);
/// Input history kept per slot (ticks older than the current ask are pruned past this).
const HISTORY: u32 = 128;

/// Why a join was refused (DENY reasons; also used for the front end's message box text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenyReason {
    Version,
    Full,
    Started,
}

impl DenyReason {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Version),
            2 => Some(Self::Full),
            3 => Some(Self::Started),
            _ => None,
        }
    }
    fn to_u8(self) -> u8 {
        match self {
            Self::Version => 1,
            Self::Full => 2,
            Self::Started => 3,
        }
    }
}

/// What the session reports to the app. `poll()` drains the queue.
#[derive(Clone, Debug)]
pub enum NetEvent {
    /// A client joined the host's lobby (slot, codename). Host side.
    AgentJoined { slot: usize, codename: String },
    /// The local machine was assigned its slot (client side, after WELCOME).
    Joined { slot: usize },
    /// A join was refused (version/build mismatch, full lobby, match already running).
    Refused(DenyReason),
    /// The match is starting; carries the host's settings (with the seed inside). Both sides —
    /// the host gets it back so the arm path is identical on every machine.
    MatchStarted(Box<MpSettings>),
    /// The state hashes diverged at `tick`. The match is dead — the app shows the game's message
    /// box and returns to the lobby.
    Desynced { tick: u32 },
    /// A peer timed out or left. The match ends (051's choice: "a dropped client becomes a bot or
    /// the match ends — pick the game's nearest behaviour": the game's join page resets panels when
    /// an agent leaves [M P_MPJOIN's per-pad leave messages], so the port ends the match and
    /// returns to the lobby rather than impersonating the leaver with a bot).
    PeerLost { slot: usize },
}

/// A lobby member as the app sees it.
#[derive(Clone, Debug)]
pub struct NetAgent {
    pub slot: usize,
    pub codename: String,
    /// Remote endpoint (None for the local agent).
    pub addr: Option<SocketAddr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Host listening / client dialing; no match yet.
    Lobby,
    /// The match runs (START seen / armed); inputs and hashes flow.
    InMatch,
    /// Desynced or a peer lost — the app should show the message box and return to the lobby.
    Failed,
    /// `close()` was called (LEAVE sent once).
    Closed,
}

/// One remote peer's session state (host: per client; client: the host).
struct Peer {
    addr: SocketAddr,
    slot: usize,
    codename: String,
    last_heard: Instant,
    /// The peer's per-tick inputs, keyed by tick.
    inputs: HashMap<u32, Input>,
    /// The peer's last state hash, awaiting our own hash at the same tick for the compare.
    hash: Option<(u32, u64)>,
    live: bool,
}

/// The lockstep session. Host or client, selected by the constructor.
pub struct NetSession {
    sock: UdpSocket,
    host: bool,
    phase: Phase,
    /// The local machine's slot (host: 0; client: assigned in WELCOME).
    slot: usize,
    /// Host: clients by slot. Client: exactly one (the host, slot of its own excluded).
    peers: Vec<Peer>,
    codename: String,
    /// Client: the host's address for HELLO retries; host: None.
    host_addr: Option<SocketAddr>,
    next_hello: Instant,
    events: Vec<NetEvent>,
    /// Our own per-tick inputs (redundancy source and the local half of ready_inputs).
    local_inputs: HashMap<u32, Input>,
    /// Our own last hash awaiting the peers' (cleared once every live peer confirmed the tick).
    my_hash: Option<(u32, u64)>,
    /// How many live peers confirmed `my_hash`'s tick (multi-peer compare).
    hash_confirmed: u32,
    /// The armed match start (host): sent as START on the next poll, then handed to the app as an
    /// event so both machines take the same path.
    armed: Option<MpSettings>,
    desynced: bool,
    /// The peer-silence timeout ([`PEER_TIMEOUT`] by default; tests shrink it).
    peer_timeout: Duration,
}

impl NetSession {
    /// Host: listen on `port`. Slot 0 is the host's own agent; up to 3 clients join.
    pub fn host(port: u16) -> Result<Self, String> {
        let sock = UdpSocket::bind(("0.0.0.0", port)).map_err(|e| format!("bind {port}: {e}"))?;
        sock.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(Self {
            sock,
            host: true,
            phase: Phase::Lobby,
            slot: 0,
            peers: Vec::new(),
            codename: String::new(),
            host_addr: None,
            next_hello: Instant::now(),
            events: Vec::new(),
            local_inputs: HashMap::new(),
            my_hash: None,
            hash_confirmed: 0,
            armed: None,
            desynced: false,
            peer_timeout: PEER_TIMEOUT,
        })
    }

    /// Client: dial `addr` ("host:port"). Sends HELLO at once and retries until WELCOME/DENY.
    pub fn connect(addr: &str, codename: &str) -> Result<Self, String> {
        let resolved = addr
            .to_socket_addrs()
            .map_err(|e| format!("resolve {addr}: {e}"))?
            .next()
            .ok_or_else(|| format!("resolve {addr}: no address"))?;
        let sock = UdpSocket::bind(("0.0.0.0", 0)).map_err(|e| e.to_string())?;
        sock.set_nonblocking(true).map_err(|e| e.to_string())?;
        let mut s = Self {
            sock,
            host: false,
            phase: Phase::Lobby,
            slot: 0,
            peers: vec![Peer {
                addr: resolved,
                slot: 0,
                codename: String::new(),
                last_heard: Instant::now(),
                inputs: HashMap::new(),
                hash: None,
                live: true,
            }],
            codename: codename.to_string(),
            host_addr: Some(resolved),
            next_hello: Instant::now(),
            events: Vec::new(),
            local_inputs: HashMap::new(),
            my_hash: None,
            hash_confirmed: 0,
            armed: None,
            desynced: false,
            peer_timeout: PEER_TIMEOUT,
        };
        s.send_hello();
        Ok(s)
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }
    /// The peer-silence timeout (tests shrink it; the default is [`PEER_TIMEOUT`]).
    pub fn set_peer_timeout(&mut self, d: Duration) {
        self.peer_timeout = d;
    }
    pub fn local_slot(&self) -> usize {
        self.slot
    }
    pub fn set_codename(&mut self, name: &str) {
        self.codename = name.chars().take(15).collect();
    }

    /// The lobby as the app sees it: the local agent first, then the remotes.
    pub fn agents(&self) -> Vec<NetAgent> {
        let mut out = vec![NetAgent { slot: self.slot, codename: self.codename.clone(), addr: None }];
        for p in &self.peers {
            out.push(NetAgent { slot: p.slot, codename: p.codename.clone(), addr: Some(p.addr) });
        }
        out.sort_by_key(|a| a.slot);
        out
    }

    /// The number of live agents (the app builds `human_count` from this on the host).
    pub fn agent_count(&self) -> usize {
        1 + self.peers.iter().filter(|p| p.live).count()
    }

    /// Host: arm the match with the final settings (the seed rides inside `MpSettings.seed`).
    /// The next `poll()` broadcasts START and reports `MatchStarted` locally, so every machine
    /// enters the match through the same event.
    pub fn start_match(&mut self, settings: &MpSettings) {
        if self.host && self.phase == Phase::Lobby {
            self.armed = Some(settings.clone());
        }
    }

    /// The local agent's input for `tick`, taken this tick (the app captures keyboard + mouse per
    /// sim tick). Sent immediately with the last REDUNDANCY ticks repeated.
    pub fn local_input(&mut self, tick: u32, input: Input) {
        self.local_inputs.insert(tick, input);
        let mut pkt = Vec::with_capacity(1 + REDUNDANCY * 24);
        pkt.push(5u8);
        let first = tick.saturating_sub(REDUNDANCY as u32 - 1);
        let n = (tick - first + 1) as u8;
        pkt.push(n);
        for t in first..=tick {
            if let Some(inp) = self.local_inputs.get(&t) {
                pkt.extend_from_slice(&t.to_le_bytes());
                pkt.extend_from_slice(&pack_input(inp));
            }
        }
        self.broadcast(&pkt);
        // Prune old history.
        let cutoff = tick.saturating_sub(HISTORY);
        self.local_inputs.retain(|t, _| *t >= cutoff);
    }

    /// All live agents' inputs for `tick`, or `None` while any is missing (the stall: the sim does
    /// not advance that tick — never guess). Absent slots (empty panels) carry `Input::default()`
    /// so the array is total; the app only ticks bonds for live slots.
    pub fn ready_inputs(&mut self, tick: u32) -> Option<[Input; MAX_AGENTS]> {
        if self.phase != Phase::InMatch {
            return None;
        }
        for p in self.peers.iter().filter(|p| p.live) {
            if !p.inputs.contains_key(&tick) {
                return None;
            }
        }
        let mut out = [Input::default(); MAX_AGENTS];
        for p in self.peers.iter().filter(|p| p.live) {
            if p.slot < MAX_AGENTS {
                out[p.slot] = p.inputs.get(&tick).copied().unwrap_or_default();
            }
        }
        out[self.slot] = self.local_inputs.get(&tick).copied().unwrap_or_default();
        // Prune behind the ask.
        let cutoff = tick.saturating_sub(HISTORY);
        for p in self.peers.iter_mut() {
            p.inputs.retain(|t, _| *t >= cutoff);
        }
        Some(out)
    }

    /// The app's per-tick state hash, every N ticks. The peer's hash at the same tick is compared
    /// on arrival; a mismatch is a desync (reported once).
    pub fn send_state_hash(&mut self, tick: u32, hash: u64) {
        let mut pkt = Vec::with_capacity(13);
        pkt.push(6u8);
        pkt.extend_from_slice(&tick.to_le_bytes());
        pkt.extend_from_slice(&hash.to_le_bytes());
        self.broadcast(&pkt);
        // Compare against hashes that arrived early; keep our hash until every live peer
        // confirmed the tick (multi-peer: each arriving peer compares against the same my_hash).
        let live = self.peers.iter().filter(|p| p.live).count();
        for p in self.peers.iter_mut().filter(|p| p.live) {
            if let Some((pt, ph)) = p.hash {
                if pt == tick {
                    if ph != hash && !self.desynced {
                        self.desynced = true;
                        self.phase = Phase::Failed;
                        self.events.push(NetEvent::Desynced { tick });
                    }
                    p.hash = None;
                    self.hash_confirmed += 1;
                }
            }
        }
        if self.hash_confirmed >= live as u32 {
            self.my_hash = None;
            self.hash_confirmed = 0;
        } else {
            self.my_hash = Some((tick, hash));
        }
    }

    /// Leave: sends LEAVE and stops. Idempotent.
    pub fn close(&mut self) {
        if self.phase != Phase::Closed {
            self.broadcast(&[7u8]);
            self.phase = Phase::Closed;
        }
    }

    /// True while the session can carry a match (not closed / failed).
    pub fn connected(&self) -> bool {
        matches!(self.phase, Phase::Lobby | Phase::InMatch)
    }

    /// Pump the transport once per app frame: receive, timeouts, handshake retries, the armed
    /// START. Returns the events queued since the last call.
    pub fn poll(&mut self) -> Vec<NetEvent> {
        // Drain the socket.
        let mut buf = [0u8; 1024];
        loop {
            match self.sock.recv_from(&mut buf) {
                Ok((n, from)) => self.on_packet(&buf[..n], from),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        // Host: the armed start.
        if self.host && self.phase == Phase::Lobby {
            if let Some(s) = self.armed.take() {
                let mut pkt = vec![4u8];
                pkt.extend_from_slice(&pack_settings(&s));
                self.broadcast(&pkt);
                self.phase = Phase::InMatch;
                self.events.push(NetEvent::MatchStarted(Box::new(s)));
            }
        }
        // Client: HELLO retries until WELCOME.
        if !self.host && self.phase == Phase::Lobby && self.slot == 0 && self.host_addr.is_some() {
            if self.next_hello <= Instant::now() {
                self.send_hello();
            }
        }
        // Timeouts (wall clock, outside the Sim).
        let now = Instant::now();
        let dropped: Vec<usize> = self
            .peers
            .iter()
            .filter(|p| p.live && now.duration_since(p.last_heard) > self.peer_timeout)
            .map(|p| p.slot)
            .collect();
        for slot in dropped {
            self.fail_peer(slot);
        }
        std::mem::take(&mut self.events)
    }

    // ---- internals ----

    fn send_hello(&mut self) {
        self.next_hello = Instant::now() + HELLO_RETRY;
        if let Some(addr) = self.host_addr {
            let mut pkt = vec![1u8];
            pkt.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
            pkt.extend_from_slice(&BUILD_ID.to_le_bytes());
            let name = self.codename.as_bytes();
            pkt.push(name.len().min(15) as u8);
            pkt.extend_from_slice(&name[..name.len().min(15)]);
            let _ = self.sock.send_to(&pkt, addr);
        }
    }

    fn broadcast(&mut self, pkt: &[u8]) {
        let peers: Vec<SocketAddr> = self.peers.iter().map(|p| p.addr).collect();
        for addr in peers {
            let _ = self.sock.send_to(pkt, addr);
        }
    }

    fn fail_peer(&mut self, slot: usize) {
        if let Some(i) = self.peers.iter().position(|p| p.slot == slot) {
            self.peers.remove(i);
        }
        if self.phase == Phase::InMatch {
            // The match ends: the port returns to the lobby rather than impersonating the leaver
            // (the module doc's PeerLost note).
            self.phase = Phase::Failed;
        }
        self.events.push(NetEvent::PeerLost { slot });
    }

    fn on_packet(&mut self, pkt: &[u8], from: SocketAddr) {
        if pkt.is_empty() {
            return;
        }
        match pkt[0] {
            1 => self.on_hello(pkt, from),
            2 => {
                // WELCOME: slot assigned.
                if !self.host && pkt.len() >= 2 && self.phase == Phase::Lobby {
                    self.slot = pkt[1] as usize;
                    self.phase = Phase::Lobby; // still waiting for START
                    self.events.push(NetEvent::Joined { slot: self.slot });
                }
            }
            3 => {
                if !self.host {
                    if let Some(&r) = pkt.get(1) {
                        if let Some(reason) = DenyReason::from_u8(r) {
                            self.events.push(NetEvent::Refused(reason));
                            self.phase = Phase::Closed;
                        }
                    }
                }
            }
            4 => {
                // START with the host's settings + seed.
                if !self.host && self.phase == Phase::Lobby {
                    if let Some(s) = unpack_settings(&pkt[1..]) {
                        self.phase = Phase::InMatch;
                        self.events.push(NetEvent::MatchStarted(Box::new(s)));
                    }
                }
            }
            5 => self.on_input(pkt, from),
            6 => self.on_hash(pkt, from),
            7 => {
                // LEAVE.
                if let Some(p) = self.peers.iter().find(|p| p.addr == from).map(|p| p.slot) {
                    self.fail_peer(p);
                }
            }
            _ => {}
        }
    }

    fn on_hello(&mut self, pkt: &[u8], from: SocketAddr) {
        if !self.host || self.peers.len() >= MAX_CLIENTS && !self.peers.iter().any(|p| p.addr == from) {
            return;
        }
        if pkt.len() < 8 {
            return;
        }
        let ver = u16::from_le_bytes([pkt[1], pkt[2]]);
        let build = u32::from_le_bytes([pkt[3], pkt[4], pkt[5], pkt[6]]);
        let clen = *pkt.get(7).unwrap_or(&0) as usize;
        let codename = String::from_utf8_lossy(&pkt[8..8 + clen.min(15)]).to_string();
        let deny = if ver != PROTOCOL_VERSION || build != BUILD_ID {
            Some(DenyReason::Version)
        } else if self.phase != Phase::Lobby {
            Some(DenyReason::Started)
        } else if self.peers.len() >= MAX_CLIENTS {
            Some(DenyReason::Full)
        } else {
            None
        };
        if let Some(reason) = deny {
            let pkt = [3u8, reason.to_u8()];
            let _ = self.sock.send_to(&pkt, from);
            return;
        }
        // Assign the next free slot (1..=3), rejoining peers keep theirs.
        let slot = match self.peers.iter().find(|p| p.addr == from) {
            Some(p) => p.slot,
            None => {
                let used: Vec<usize> = self.peers.iter().map(|p| p.slot).collect();
                let slot = (1..MAX_AGENTS).find(|s| !used.contains(s)).unwrap_or(MAX_AGENTS);
                self.peers.push(Peer {
                    addr: from,
                    slot,
                    codename,
                    last_heard: Instant::now(),
                    inputs: HashMap::new(),
                    hash: None,
                    live: true,
                });
                let codename = self.peers.last().unwrap().codename.clone();
                self.events.push(NetEvent::AgentJoined { slot, codename });
                slot
            }
        };
        let _ = self.sock.send_to(&[2u8, slot as u8], from);
    }

    /// INPUT: n u8, then n × { tick u32, packed input }. Store every entry into the sender's ring
    /// (redundant entries overwrite identical values — the packing is exact, so that is safe).
    fn on_input(&mut self, pkt: &[u8], from: SocketAddr) {
        if pkt.len() < 2 {
            return;
        }
        let n = pkt[1] as usize;
        let mut off = 2usize;
        let Some(p) = self.peers.iter_mut().find(|p| p.addr == from) else {
            return;
        };
        p.last_heard = Instant::now();
        for _ in 0..n {
            if off + 24 > pkt.len() {
                break;
            }
            let tick = u32::from_le_bytes([pkt[off], pkt[off + 1], pkt[off + 2], pkt[off + 3]]);
            let mut ib = [0u8; 20];
            ib.copy_from_slice(&pkt[off + 4..off + 24]);
            p.inputs.insert(tick, unpack_input(&ib));
            off += 24;
        }
    }

    /// HASH: tick u32 + hash u64 from one peer. Compare against our own hash for the same tick
    /// (whichever arrives first is stored; the second one compares).
    fn on_hash(&mut self, pkt: &[u8], from: SocketAddr) {
        if pkt.len() < 13 {
            return;
        }
        let tick = u32::from_le_bytes([pkt[1], pkt[2], pkt[3], pkt[4]]);
        let hash = u64::from_le_bytes([
            pkt[5], pkt[6], pkt[7], pkt[8], pkt[9], pkt[10], pkt[11], pkt[12],
        ]);
        let Some(p) = self.peers.iter_mut().find(|p| p.addr == from) else {
            return;
        };
        p.last_heard = Instant::now();
        match self.my_hash {
            Some((mt, mh)) if mt == tick && mh != hash && !self.desynced => {
                self.desynced = true;
                self.phase = Phase::Failed;
                self.events.push(NetEvent::Desynced { tick });
            }
            Some((mt, _)) if mt == tick => {
                // Equal — consumed.
                self.my_hash = None;
            }
            _ => {
                // Our hash for this tick has not been sent yet — store for the compare in
                // `send_state_hash`.
                p.hash = Some((tick, hash));
            }
        }
    }
}

// ---- wire packing ----

/// Pack `Input` into the 20-byte wire form (f32 x4 LE, a bit byte, i8 cycle pairs, i8 switch_to).
pub fn pack_input(i: &Input) -> [u8; 20] {
    let mut o = [0u8; 20];
    o[0..4].copy_from_slice(&i.forward.to_le_bytes());
    o[4..8].copy_from_slice(&i.strafe.to_le_bytes());
    o[8..12].copy_from_slice(&i.turn.to_le_bytes());
    o[12..16].copy_from_slice(&i.look.to_le_bytes());
    let mut bits = 0u8;
    if i.jump { bits |= 1; }
    if i.crouch { bits |= 2; }
    if i.aiming { bits |= 4; }
    if i.zoom { bits |= 8; }
    if i.scope { bits |= 16; }
    if i.fire { bits |= 32; }
    if i.mode_switch { bits |= 64; }
    o[16] = bits;
    o[17] = i.weapon_cycle as i8 as u8;
    o[18] = i.gadget_cycle as i8 as u8;
    o[19] = i.switch_to.map(|v| v as i8).unwrap_or(-1) as u8;
    o
}

/// The inverse of [`pack_input`].
pub fn unpack_input(b: &[u8; 20]) -> Input {
    let f = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let bits = b[16];
    Input {
        forward: f(0),
        strafe: f(4),
        turn: f(8),
        look: f(12),
        jump: bits & 1 != 0,
        crouch: bits & 2 != 0,
        aiming: bits & 4 != 0,
        zoom: bits & 8 != 0,
        scope: bits & 16 != 0,
        fire: bits & 32 != 0,
        mode_switch: bits & 64 != 0,
        weapon_cycle: b[17] as i8,
        gadget_cycle: b[18] as i8,
        switch_to: if b[19] == 0xff { None } else { Some(b[19]) },
    }
}

/// Serialize the match-defining fields of `MpSettings` + the seed (see the module doc).
pub fn pack_settings(s: &MpSettings) -> Vec<u8> {
    let mut o = Vec::with_capacity(180);
    let mut u32f = |o: &mut Vec<u8>, v: u32| o.extend_from_slice(&v.to_le_bytes());
    u32f(&mut o, s.seed);
    u32f(&mut o, s.scenario);
    u32f(&mut o, s.gametype_mask);
    u32f(&mut o, s.level_code);
    u32f(&mut o, s.frag_limit as u32);
    u32f(&mut o, s.time_limit as u32);
    u32f(&mut o, s.human_count);
    u32f(&mut o, s.bot_count);
    u32f(&mut o, s.total_players);
    o.extend_from_slice(&s.player_count.to_le_bytes());
    u32f(&mut o, s.weapon_set);
    u32f(&mut o, s.radar);
    u32f(&mut o, s.friendly_fire);
    u32f(&mut o, s.master);
    u32f(&mut o, s.team_game);
    u32f(&mut o, s.bit30);
    u32f(&mut o, s.capture_reward);
    u32f(&mut o, s.spawn_policy);
    o.extend_from_slice(&s.registered_pickups.to_le_bytes());
    for p in &s.players {
        u32f(&mut o, p.team);
        u32f(&mut o, p.skin);
        u32f(&mut o, p.armor_bonus);
    }
    o
}

/// The inverse of [`pack_settings`]; `None` on a short packet.
pub fn unpack_settings(b: &[u8]) -> Option<MpSettings> {
    if b.len() < 72 + 96 {
        return None;
    }
    let mut i = 0usize;
    let u32f = |i: &mut usize| {
        let v = u32::from_le_bytes([b[*i], b[*i + 1], b[*i + 2], b[*i + 3]]);
        *i += 4;
        v
    };
    let u16f = |i: &mut usize| {
        let v = u16::from_le_bytes([b[*i], b[*i + 1]]);
        *i += 2;
        v
    };
    let mut s = MpSettings::deathmatch(0);
    s.seed = u32f(&mut i);
    s.scenario = u32f(&mut i);
    s.gametype_mask = u32f(&mut i);
    s.level_code = u32f(&mut i);
    s.frag_limit = u32f(&mut i) as i32;
    s.time_limit = u32f(&mut i) as i32;
    s.human_count = u32f(&mut i);
    s.bot_count = u32f(&mut i);
    s.total_players = u32f(&mut i);
    s.player_count = u16f(&mut i);
    s.weapon_set = u32f(&mut i);
    s.radar = u32f(&mut i);
    s.friendly_fire = u32f(&mut i);
    s.master = u32f(&mut i);
    s.team_game = u32f(&mut i);
    s.bit30 = u32f(&mut i);
    s.capture_reward = u32f(&mut i);
    s.spawn_policy = u32f(&mut i);
    s.registered_pickups = u16f(&mut i);
    for p in s.players.iter_mut() {
        p.team = u32f(&mut i);
        p.skin = u32f(&mut i);
        p.armor_bonus = u32f(&mut i);
    }
    Some(s)
}

/// FNV-1a 64 over the shared deterministic state. The app folds in a fixed order (slot order,
/// then the shared world), so equal states give equal hashes on every machine.
pub struct Hasher(pub u64);

impl Hasher {
    pub fn new() -> Self {
        Hasher(0xcbf2_9ce4_8422_2325)
    }
    pub fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 ^= x as u64;
            self.0 = self.0.wrapping_mul(0x100_0000_01b3);
        }
    }
    pub fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.u32(v as u32);
    }
    pub fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    pub fn bool(&mut self, v: bool) {
        self.bytes(&[v as u8]);
    }
    pub fn finish(&self) -> u64 {
        self.0
    }
}
