//! The ONE session layer's app surface (net-consolidate 2026-10-16).
//!
//! Two workers built two UDP sessions (051 `nf-net` and 053 `nf_game::net`);
//! this consolidation keeps 051's crate — the stronger protocol: a contiguous
//! ack + 32-tick bitmap so the full unacked window rides every packet (an
//! isolated loss can never lose a tick), a configurable input delay 2..=4
//! carried in WELCOME, hash exchanges with the previous hash piggybacking on
//! every HASH packet, keepalives, GO retransmit, and the deterministic shim
//! that makes the lossy tests bit-reproducible. 053's unique features moved
//! in: the full `MpSettings` fidelity on the wire (per-player `radar_on`,
//! `registered_pickups`), the lobby display codename + `agents()` view, the
//! refuse-when-started gate, the FNV `Hasher` (now `nf_net::hash::Hasher`)
//! and the `MpSettings` match handover this module drives (053's START: the
//! host's final settings ride a tagged DATA tunnel — the WELCOME at join time
//! cannot know them, the confirm page arms later).
//!
//! `NetCore` owns the `nf_net::Session<UdpTransport>`; `SessionProxy` is the
//! cheap handle both the front end (`FrontEnd::mp_session`, the lobby's
//! `MpSession` trait object) and the app (`App.net`, the lockstep tick gate)
//! share — one core, two drivers, exactly the wiring 052's report promised.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use nf_game::mp::{MpPlayer, MpSettings};
use nf_game::player;
use nf_net::session::{Config, Event, Session};
use nf_net::{MatchSettings, UdpTransport};

use crate::menu::{MpNetAgent, MpNetChoices, MpNetError, MpNetLink};

// ── conversions ──────────────────────────────────────────────────────────────

/// `MpSettings` → the wire's `MatchSettings` (field mapping documented in
/// nf-net's `settings` module; the seed rides the handshake separately, so it
/// is not in `MatchSettings`).
pub fn mp_to_match(s: &MpSettings) -> MatchSettings {
    MatchSettings {
        started: s.started as u32,
        gametype_mask: s.gametype_mask,
        player_count: s.player_count as u32,
        master: s.master,
        master_copy: s.master_copy,
        team_game: s.team_game,
        bit30: s.bit30,
        total_players: s.total_players,
        frag_limit: s.frag_limit,
        time_limit: s.time_limit,
        scenario: s.scenario,
        level_code: s.level_code,
        human_count: s.human_count,
        bot_count: s.bot_count,
        weapon_set: s.weapon_set,
        capture_reward: s.capture_reward,
        spawn_policy: s.spawn_policy,
        radar: s.radar,
        friendly_fire: s.friendly_fire,
        players: s.players.each_ref().map(|p| (p.team, p.skin, p.armor_bonus, p.radar_on)),
        registered_pickups: s.registered_pickups,
    }
}

/// `MatchSettings` + the handshake's seed → the app's `MpSettings`
/// (`load_mp_carry` builds the level from it; the seed feeds the bit-exact
/// `Rng::new` on every machine).
pub fn match_to_mp(m: &MatchSettings, seed: u32) -> MpSettings {
    MpSettings {
        players: m.players.map(|(team, skin, armor_bonus, radar_on)| MpPlayer {
            team, skin, armor_bonus, radar_on, codename: String::new(),
        }),
        started: m.started != 0,
        gametype_mask: m.gametype_mask,
        player_count: m.player_count as u16,
        master: m.master,
        master_copy: m.master_copy,
        team_game: m.team_game,
        bit30: m.bit30,
        total_players: m.total_players,
        frag_limit: m.frag_limit,
        time_limit: m.time_limit,
        scenario: m.scenario,
        level_code: m.level_code,
        human_count: m.human_count,
        bot_count: m.bot_count,
        weapon_set: m.weapon_set,
        capture_reward: m.capture_reward,
        spawn_policy: m.spawn_policy,
        radar: m.radar,
        registered_pickups: m.registered_pickups,
        friendly_fire: m.friendly_fire,
        seed,
    }
}

/// The app's per-tick input + the three separate `step_bond` args → the
/// 24-byte wire input (nf-net's `Input` is the superset: it carries
/// `trigger`/`reload`/`use_key`, which 053's 20-byte packing dropped).
pub fn to_net_input(i: &player::Input, trigger: bool, reload: bool, use_key: bool) -> nf_net::Input {
    nf_net::Input {
        forward: i.forward,
        strafe: i.strafe,
        turn: i.turn,
        look: i.look,
        jump: i.jump,
        crouch: i.crouch,
        aiming: i.aiming,
        zoom: i.zoom,
        scope: i.scope,
        fire: i.fire,
        mode_switch: i.mode_switch,
        trigger,
        reload,
        use_key,
        weapon_cycle: i.weapon_cycle,
        gadget_cycle: i.gadget_cycle,
        switch_to: i.switch_to,
    }
}

/// The wire input → the sim's `player::Input` (the trigger/reload/use_key
/// lanes are the local machine's own — a remote body ticks `tick_ctx`, which
/// consumes the movement lanes only, exactly like the match-host cut did).
pub fn to_player_input(i: &nf_net::Input) -> player::Input {
    player::Input {
        forward: i.forward,
        strafe: i.strafe,
        turn: i.turn,
        look: i.look,
        jump: i.jump,
        crouch: i.crouch,
        aiming: i.aiming,
        zoom: i.zoom,
        scope: i.scope,
        fire: i.fire,
        mode_switch: i.mode_switch,
        weapon_cycle: i.weapon_cycle,
        gadget_cycle: i.gadget_cycle,
        switch_to: i.switch_to,
        // microcam-wheel-zoom: the wheel's zoom steps are local view input, never on the wire.
        zoom_cycle: 0,
    }
}

// ── the DATA tunnel codec ────────────────────────────────────────────────────
// tag 0: the host's lobby choices (13 words). tag 1: the match-start payload
// (208 B MatchSettings + the seed) — 053's START packet, riding the tunnel.

impl MpNetChoices {
    fn encode(&self) -> Vec<u8> {
        let mut o = vec![0u8];
        for v in [
            self.scenario, self.level, self.weapon_set, self.radar, self.friendly_fire,
            self.frag_limit as u32, self.time_limit as u32,
            self.player_mods[0], self.player_mods[1],
            self.envmods[0], self.envmods[1], self.envmods[2], self.envmods[3], self.envmods[4],
        ] {
            o.extend_from_slice(&v.to_le_bytes());
        }
        o
    }
    fn decode(b: &[u8]) -> Option<MpNetChoices> {
        if b.len() < 1 + 13 * 4 { return None; }
        let w = |i: usize| u32::from_le_bytes([b[1 + i * 4], b[2 + i * 4], b[3 + i * 4], b[4 + i * 4]]);
        Some(MpNetChoices {
            scenario: w(0),
            level: w(1),
            weapon_set: w(2),
            radar: w(3),
            friendly_fire: w(4),
            frag_limit: w(5) as i32,
            time_limit: w(6) as i32,
            player_mods: [w(7), w(8)],
            envmods: [w(9), w(10), w(11), w(12), w(13)],
        })
    }
}

fn encode_match_start(m: &MatchSettings, seed: u32) -> Vec<u8> {
    let mut o = vec![1u8];
    let mut buf = [0u8; nf_net::settings::SETTINGS_WIRE_SIZE];
    m.encode(&mut buf);
    o.extend_from_slice(&buf);
    o.extend_from_slice(&seed.to_le_bytes());
    o
}

fn decode_match_start(b: &[u8]) -> Option<(MatchSettings, u32)> {
    if b.len() < 1 + nf_net::settings::SETTINGS_WIRE_SIZE + 4 || b[0] != 1 {
        return None;
    }
    let m = MatchSettings::decode(&b[1..1 + nf_net::settings::SETTINGS_WIRE_SIZE])?;
    let off = 1 + nf_net::settings::SETTINGS_WIRE_SIZE;
    let seed = u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]]);
    Some((m, seed))
}

// ── the core + proxy ─────────────────────────────────────────────────────────

/// What the app must react to (the old `NetEvent` arms).
#[derive(Clone, Debug)]
pub enum CoreEvent {
    /// Host: a client took a slot (the flag path's auto-start cue).
    AgentJoined { slot: u8 },
    /// The match is starting: the host on arm, the client on the tunnel's
    /// match payload. The app loads the level from it (053's MatchStarted).
    MatchStarted(Box<MpSettings>),
    /// A human slot's client left (Neutral policy: the match runs on, the
    /// game's own disconnected-pad shape).
    ClientLeft { slot: u8, graceful: bool },
    /// The state hashes diverged at `tick` (the app shows the box, back to lobby).
    Desync { tick: u32 },
    /// Client: the host went silent past the timeout.
    HostTimeout,
    /// Client: the host refused the join.
    Refused,
}

/// The shared session state: one per match/lobby. See the module docs.
pub struct NetCore {
    sess: Session<UdpTransport>,
    is_host: bool,
    /// The session's logical clock (wall ms since the core's birth — the
    /// session layer sits OUTSIDE the Sim, so the determinism rule holds).
    epoch: Instant,
    /// The `MpSettings` handover the lobby's `match_started()` reads (the
    /// client mirrors it into `mp_client_start`; the host's own handover is
    /// the confirm page's, this is set on arm for symmetry).
    match_start: Option<MpSettings>,
    /// The host's last published lobby choices (the client's mirror source).
    choices: Option<MpNetChoices>,
    refused: bool,
    host_timed_out: bool,
    closed: bool,
    /// Events for the app (`take_app_events`).
    events: Vec<CoreEvent>,
}

fn session_cfg() -> Config {
    Config { hash_every: 15, ..Default::default() } // NET_HASH_EVERY's cadence
}

impl NetCore {
    /// Host: bind `0.0.0.0:port` and listen.
    pub fn host(port: u16) -> Result<Rc<RefCell<NetCore>>, String> {
        let tr = UdpTransport::bind(&format!("0.0.0.0:{port}")).map_err(|e| format!("bind {port}: {e}"))?;
        Ok(Rc::new(RefCell::new(NetCore {
            sess: Session::new_host(session_cfg(), tr, MatchSettings::default(), 0),
            is_host: true,
            epoch: Instant::now(),
            match_start: None,
            choices: None,
            refused: false,
            host_timed_out: false,
            closed: false,
            events: Vec::new(),
        })))
    }

    /// The lobby's re-bind, host side: a fresh listening Session on the SAME
    /// core, so every handle to this core (the app's `net`, the front end's
    /// `MpSession`, a test's clone) keeps working (net-e2e-loopback — the old
    /// replace-the-Rc shape left the other holders on the stale placeholder
    /// socket). The lobby calls it only before anything flows.
    pub fn rebind_host(&mut self, port: u16) -> Result<(), String> {
        let tr = UdpTransport::bind(&format!("0.0.0.0:{port}")).map_err(|e| format!("bind {port}: {e}"))?;
        self.sess = Session::new_host(session_cfg(), tr, MatchSettings::default(), 0);
        self.is_host = true;
        self.epoch = Instant::now();
        self.match_start = None;
        self.choices = None;
        self.refused = false;
        self.host_timed_out = false;
        self.closed = false;
        self.events.clear();
        Ok(())
    }

    /// The lobby's re-bind, client side: dial `addr` on the SAME core (see
    /// `rebind_host`).
    pub fn rebind_client(&mut self, addr: std::net::SocketAddr) -> Result<(), String> {
        let tr = UdpTransport::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        self.sess = Session::new_client(session_cfg(), tr, addr);
        self.is_host = false;
        self.epoch = Instant::now();
        self.match_start = None;
        self.choices = None;
        self.refused = false;
        self.host_timed_out = false;
        self.closed = false;
        self.events.clear();
        Ok(())
    }

    /// Client: dial `addr` ("host:port").
    pub fn connect(addr: &str) -> Result<Rc<RefCell<NetCore>>, String> {
        let a: std::net::SocketAddr = addr.parse().map_err(|e| format!("resolve {addr}: {e}"))?;
        let tr = UdpTransport::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        Ok(Rc::new(RefCell::new(NetCore {
            sess: Session::new_client(session_cfg(), tr, a),
            is_host: false,
            epoch: Instant::now(),
            match_start: None,
            choices: None,
            refused: false,
            host_timed_out: false,
            closed: false,
            events: Vec::new(),
        })))
    }

    /// Pump the transport once per frame: receive, timers, events → state.
    pub fn poll(&mut self) {
        let now = self.epoch.elapsed().as_millis() as u64;
        for ev in self.sess.poll(now) {
            match ev {
                Event::Refused { .. } => {
                    self.refused = true;
                    self.events.push(CoreEvent::Refused);
                }
                Event::ClientJoined { slot, .. } => {
                    self.events.push(CoreEvent::AgentJoined { slot });
                }
                Event::MatchStarted { .. } => {
                    // Client GO: the final settings must already have arrived
                    // through the tunnel (the host sends them on arm, before
                    // GO). If none came, the handover stays the WELCOME's.
                }
                Event::ClientLeft { slot, reason } => {
                    self.events.push(CoreEvent::ClientLeft {
                        slot,
                        graceful: reason == nf_net::LeaveReason::Graceful,
                    });
                }
                Event::HostTimeout => {
                    self.host_timed_out = true;
                    self.events.push(CoreEvent::HostTimeout);
                }
                Event::Desync { tick, .. } => {
                    self.events.push(CoreEvent::Desync { tick });
                }
                Event::Data { bytes, .. } => {
                    match bytes.first() {
                        Some(0) => {
                            self.choices = MpNetChoices::decode(&bytes);
                        }
                        Some(1) => {
                            if let Some((m, seed)) = decode_match_start(&bytes) {
                                let s = match_to_mp(&m, seed);
                                self.match_start = Some(s.clone());
                                self.events.push(CoreEvent::MatchStarted(Box::new(s)));
                            }
                        }
                        _ => {}
                    }
                }
                Event::Listening | Event::Connected { .. } | Event::SlotMapChanged { .. }
                | Event::Stalled { .. } | Event::Resumed { .. } | Event::MatchEnded { .. } => {}
            }
        }
    }

    /// The app's events since the last call.
    pub fn take_app_events(&mut self) -> Vec<CoreEvent> {
        std::mem::take(&mut self.events)
    }

    /// Host: arm the match — the settings + seed replace the handshake
    /// placeholder, GO starts every connected client, and the final payload
    /// rides the tunnel (053's START). This machine takes the same handover.
    pub fn start_match(&mut self, s: &MpSettings) {
        if !self.is_host || self.closed {
            return;
        }
        self.sess.set_host_match(mp_to_match(s), s.seed);
        self.sess.start_match(0).ok();
        self.match_start = Some(s.clone());
        self.events.push(CoreEvent::MatchStarted(Box::new(s.clone())));
        self.sess.send_data(encode_match_start(&mp_to_match(s), s.seed));
    }

    /// The lobby's agents (the trait's `agents`).
    pub fn agents(&self) -> Vec<MpNetAgent> {
        self.sess.agents().into_iter().map(|a| MpNetAgent {
            slot: a.slot as usize,
            codename: a.codename,
            ready: true, // the lobby auto-readies joined panels [S §2.1]
        }).collect()
    }

    pub fn host_choices(&self) -> Option<MpNetChoices> {
        self.choices.clone()
    }

    pub fn publish_choices(&mut self, c: &MpNetChoices) {
        self.choices = Some(c.clone());
        self.sess.send_data(c.encode());
    }

    pub fn set_codename(&mut self, name: &str) {
        self.sess.set_display_name(name);
    }

    pub fn close(&mut self) {
        if !self.closed {
            self.sess.leave();
            self.closed = true;
        }
    }

    /// The link state the join flow reads.
    pub fn link(&self) -> MpNetLink {
        if self.closed { return MpNetLink::Idle; }
        if self.is_host { return MpNetLink::Listening; }
        if self.host_timed_out { return MpNetLink::Lost; }
        if self.refused { return MpNetLink::Refused; }
        if self.sess.local_slot() != 0 || self.match_start.is_some() {
            return MpNetLink::Connected;
        }
        MpNetLink::Dialing
    }

    // ── the app's match surface (the lockstep tick gate) ────────────────

    pub fn ready_inputs(&mut self, tick: u32) -> Option<[nf_net::Input; 4]> {
        self.sess.ready_inputs(tick)
    }

    pub fn local_input(&mut self, tick: u32, input: nf_net::Input) {
        self.sess.local_input(tick, input);
    }

    pub fn submit_hash(&mut self, tick: u32, hash: u64) {
        self.sess.submit_hash(tick, hash);
    }

    pub fn local_slot(&self) -> usize {
        self.sess.local_slot() as usize
    }

    /// This machine's display codename (the re-bind path carries it over).
    pub fn display_name(&self) -> String {
        self.sess.agents().into_iter()
            .find(|a| a.slot == self.sess.local_slot())
            .map(|a| a.codename)
            .unwrap_or_default()
    }

    /// The match handover peek (the lobby's `match_started`).
    pub fn match_started(&self) -> Option<MpSettings> {
        self.match_start.clone()
    }

    /// Test-only (net-e2e-loopback): deterministic loss + reordering on this
    /// core's UDP sends. See `nf_net::Session::inject_link_fault`.
    pub fn inject_link_fault(&mut self, seed: u64, drop_per_mille: u32, reorder_per_mille: u32) {
        self.sess.inject_link_fault(seed, drop_per_mille, reorder_per_mille);
    }
}

/// A cheap cloneable handle: the front end holds one as its `MpSession`
/// trait object, the app drives the match through another. The second field
/// is the test-only link-fault recipe (`None` in production): the lobby's
/// `listen`/`connect` replace the core, and the recipe rides along so a
/// lossy proxy STAYS lossy across the re-bind (net-e2e-loopback).
#[derive(Clone)]
pub struct SessionProxy(pub Rc<RefCell<NetCore>>, pub Option<(u64, u32, u32)>);

impl SessionProxy {
    pub fn host(port: u16) -> Result<SessionProxy, String> {
        NetCore::host(port).map(|c| SessionProxy(c, None))
    }
    pub fn connect(addr: &str) -> Result<SessionProxy, String> {
        NetCore::connect(addr).map(|c| SessionProxy(c, None))
    }
    /// The lossy variant: every core this proxy builds (now and on the
    /// lobby's re-bind) drops `drop_per_mille`‰ of its sends and transposes
    /// `reorder_per_mille`‰ of them, from the seeded SplitMix64.
    pub fn host_faulty(port: u16, fault: (u64, u32, u32)) -> Result<SessionProxy, String> {
        let c = NetCore::host(port)?;
        c.borrow_mut().inject_link_fault(fault.0, fault.1, fault.2);
        Ok(SessionProxy(c, Some(fault)))
    }
    pub fn connect_faulty(addr: &str, fault: (u64, u32, u32)) -> Result<SessionProxy, String> {
        let c = NetCore::connect(addr)?;
        c.borrow_mut().inject_link_fault(fault.0, fault.1, fault.2);
        Ok(SessionProxy(c, Some(fault)))
    }
    pub fn core_is_closed(&self) -> bool {
        self.0.borrow().closed
    }
    /// True once the match is armed (host) or the match payload arrived
    /// (client) — the app's adoption cue.
    pub fn has_match(&self) -> bool {
        self.0.borrow().match_started().is_some()
    }
}

impl crate::menu::MpSession for SessionProxy {
    fn poll(&mut self) { self.0.borrow_mut().poll(); }
    fn link(&self) -> MpNetLink { self.0.borrow().link() }
    fn listen(&mut self, port: u16) -> Result<(), MpNetError> {
        // The core binds at construction; the lobby's listen row RE-BINDS the
        // same core in place (net-e2e-loopback: the old replace-the-Rc shape
        // left every other handle — the app's `net`, test clones — on the
        // stale placeholder socket). Called only before anything flows, so a
        // fresh session is behaviour-identical. The fault recipe (if any)
        // re-applies — a lossy proxy stays lossy across the re-bind.
        let name = self.0.borrow().display_name();
        let r = self.0.borrow_mut().rebind_host(port);
        match r {
            Ok(()) => {
                if let Some(f) = self.1 {
                    self.0.borrow_mut().inject_link_fault(f.0, f.1, f.2);
                }
                if !name.is_empty() { self.0.borrow_mut().set_codename(&name); }
                Ok(())
            }
            Err(_) => Err(MpNetError::Busy),
        }
    }
    fn connect(&mut self, addr: &str) -> Result<(), MpNetError> {
        let name = self.0.borrow().display_name();
        let parsed = addr.parse::<std::net::SocketAddr>();
        let r = match parsed {
            Ok(a) => self.0.borrow_mut().rebind_client(a),
            Err(_) => return Err(MpNetError::BadAddress),
        };
        match r {
            Ok(()) => {
                if let Some(f) = self.1 {
                    self.0.borrow_mut().inject_link_fault(f.0, f.1, f.2);
                }
                if !name.is_empty() { self.0.borrow_mut().set_codename(&name); }
                Ok(())
            }
            Err(_) => Err(MpNetError::BadAddress),
        }
    }
    fn agents(&self) -> Vec<MpNetAgent> { self.0.borrow().agents() }
    fn host_choices(&self) -> Option<MpNetChoices> { self.0.borrow().host_choices() }
    fn publish_choices(&mut self, c: &MpNetChoices) { self.0.borrow_mut().publish_choices(c); }
    fn start_match(&mut self, s: &nf_game::mp::MpSettings) { self.0.borrow_mut().start_match(s); }
    fn match_started(&self) -> Option<nf_game::mp::MpSettings> { self.0.borrow().match_started() }
    fn set_codename(&mut self, name: &str) { self.0.borrow_mut().set_codename(name); }
    fn close(&mut self) { self.0.borrow_mut().close(); }
}
