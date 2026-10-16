//! Loopback integration tests for the UDP lockstep session.
//!
//! Deterministic throughout: logical milliseconds (no wall clock), the
//! seeded shim network (same seed = same loss pattern), fixed 60 Hz steps.
//! `real_udp_loopback_sync` additionally exercises the real
//! `std::net::UdpSocket` path on 127.0.0.1.

use std::net::SocketAddr;

use nf_net::net::{Transport, UdpTransport};
use nf_net::shim::Shim;
use nf_net::session::{Config, DropPolicy, Event, LeaveReason, RefuseReason, Session};
use nf_net::{Input, MatchSettings, NEUTRAL};

fn addr(port: u16) -> SocketAddr {
    format!("127.0.0.1:{port}").parse().unwrap()
}

fn test_cfg() -> Config {
    Config {
        build_id: 77,
        input_delay: 3,
        redundancy: 4,
        hash_every: 30,
        timeout_ms: 2000,
        keepalive_ms: 100,
        hello_retry_ms: 100,
        resend_ms: 20,
        ..Default::default()
    }
}

/// Deterministic per-agent "controls" for (slot, tick).
fn agent_input(slot: u8, tick: u32) -> Input {
    let mut inp = NEUTRAL;
    let x = (slot as u32).wrapping_mul(0x9E37_79B9) ^ tick.wrapping_mul(0x85EB_CA6B);
    inp.forward = ((x & 0xFF) as i32 - 128) as f32 / 128.0;
    inp.turn = (((x >> 8) & 0xFF) as i32 - 128) as f32 / 128.0;
    inp.fire = x & 0x100 != 0;
    inp.jump = x & 0x200 != 0;
    inp.weapon_cycle = (x & 0x0F) as i8 - 8;
    inp
}

/// The fake deterministic sim: a hash chain folded from all 4 inputs of a
/// tick. Same inputs on every machine = same hash (the lockstep premise);
/// a one-bit input difference changes it (the sister-test premise).
fn sim_hash(tick: u32, inputs: &[Input; 4], prev: u64) -> u64 {
    let mut h = prev ^ 0xC0FF_EE00;
    for (s, i) in inputs.iter().enumerate() {
        h = h.rotate_left(17) ^ (i.forward.to_bits() as u64).wrapping_add(s as u64 + 1);
        h = h.rotate_left(13) ^ (i.fire as u64).wrapping_mul(31);
        h = h.rotate_left(7) ^ (i.weapon_cycle as u8 as u64);
        h = h.rotate_left(11) ^ ((i.jump as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    }
    h ^ (tick as u64)
}

struct World {
    shim: Shim,
    host: Session<nf_net::shim::ShimEndpoint>,
    clients: Vec<Session<nf_net::shim::ShimEndpoint>>,
    host_sim: AgentSim,
    client_sims: Vec<AgentSim>,
    /// While set, every datagram to/from this address is destroyed (dead link).
    outage: Option<SocketAddr>,
    /// Corrupt one input sample: (slot, tick) — forces hash divergence.
    corrupt: Option<(usize, u32)>,
    scheduled: Vec<u32>,
}

struct AgentSim {
    executed: u32,
    prev_hash: u64,
    desync_ticks: Vec<u32>,
    /// (tick, hash) every 50 ticks, for divergence hunting.
    marks: Vec<(u32, u64)>,
}

impl AgentSim {
    fn new() -> AgentSim {
        AgentSim { executed: 0, prev_hash: 0, desync_ticks: Vec::new(), marks: Vec::new() }
    }
}

impl World {
    fn new(n_clients: usize, drop_per_mille: u32, latency: u64, jitter: u64, seed: u64) -> World {
        let shim = Shim::new(seed);
        shim.set_loss(drop_per_mille);
        shim.set_latency(latency, jitter);
        let host = Session::new_host(
            test_cfg(),
            shim.endpoint(addr(7000)),
            MatchSettings::default(),
            0x1234_5678,
        );
        let mut clients = Vec::new();
        for p in 0..n_clients {
            clients.push(Session::new_client(
                test_cfg(),
                shim.endpoint(addr(7100 + p as u16)),
                addr(7000),
            ));
        }
        World {
            shim,
            host,
            clients,
            host_sim: AgentSim::new(),
            client_sims: (0..n_clients).map(|_| AgentSim::new()).collect(),
            outage: None,
            corrupt: None,
            scheduled: vec![0; n_clients + 1],
        }
    }

    /// One 60 Hz frame: deliver due datagrams, poll every session, sample
    /// inputs, run every ready tick. Returns all events.
    fn step(&mut self) -> Vec<Event> {
        self.shim.advance(16);
        if let Some(dead) = self.outage {
            self.shim.cut(dead);
        }
        let mut events = Vec::new();
        for c in self.clients.iter_mut() {
            events.extend(c.poll(self.shim.now_ms()));
        }
        events.extend(self.host.poll(self.shim.now_ms()));
        if let Some(dead) = self.outage {
            self.shim.cut(dead); // also kill what this step's polls just queued
        }
        self.run_sims();
        events
    }

    fn run_sims(&mut self) {
        // Sample: each agent schedules one input per frame up to
        // executed + input_delay + slack (inputs ahead are sampled from
        // present controls — never invented).
        for slot in 0..self.scheduled.len() {
            let cap = if slot == 0 {
                self.host_sim.executed + 4 + 40
            } else {
                match self.client_sims.get(slot - 1) {
                    Some(s) => s.executed + 4 + 40,
                    None => continue, // client gone (timeout test)
                }
            };
            // A client samples only once its slot is assigned (post-handshake);
            // its agent identity IS its slot.
            if slot > 0 && self.clients.get(slot - 1).map(|c| c.local_slot()).unwrap_or(0) == 0 {
                continue;
            }
            while self.scheduled[slot] < cap {
                let t = self.scheduled[slot];
                let inp = agent_input(slot as u8, t);
                if slot == 0 {
                    self.host.local_input(t, inp);
                } else {
                    self.clients[slot - 1].local_input(t, inp);
                }
                self.scheduled[slot] += 1;
            }
        }
        // Run: each session advances its own copy of the sim while ready —
        // but only over ticks whose LOCAL input has been sampled (a real app
        // never queries past its own lookahead; probing ahead would report a
        // bogus stall on the local slot).
        let mut host_sim = std::mem::replace(&mut self.host_sim, AgentSim::new());
        while self.scheduled[0] > host_sim.executed {
            match self.host.ready_inputs(host_sim.executed) {
                Some(inputs) => {
                    let h = sim_hash(host_sim.executed, &inputs, host_sim.prev_hash);
                    self.host.submit_hash(host_sim.executed, h);
                    host_sim.prev_hash = h;
                    if host_sim.executed % 1 == 0 {
                        host_sim.marks.push((host_sim.executed, h));
                    }
                    host_sim.executed += 1;
                }
                None => break,
            }
        }
        self.host_sim = host_sim;
        for ci in 0..self.clients.len() {
            let mut sim = std::mem::replace(&mut self.client_sims[ci], AgentSim::new());
            while self.scheduled[ci + 1] > sim.executed {
                match self.clients[ci].ready_inputs(sim.executed) {
                    Some(mut inputs) => {
                        // A desync is a SIM-side divergence: flip the input in
                        // ONE machine's view only (it still sends the clean
                        // value on the wire, like a real sim bug would).
                        if self.corrupt == Some((ci + 1, sim.executed)) {
                            let ls = self.clients[ci].local_slot() as usize;
                            inputs[ls].fire = !inputs[ls].fire;
                        }
                        let h = sim_hash(sim.executed, &inputs, sim.prev_hash);
                        self.clients[ci].submit_hash(sim.executed, h);
                        sim.prev_hash = h;
                        if sim.executed % 1 == 0 {
                            sim.marks.push((sim.executed, h));
                        }
                        sim.executed += 1;
                    }
                    None => break,
                }
            }
            self.client_sims[ci] = sim;
        }
    }

    /// Run `n` frames, returning all events.
    fn run(&mut self, n: usize) -> Vec<Event> {
        let mut events = Vec::new();
        for _ in 0..n {
            events.extend(self.step());
        }
        events
    }

    /// Handshake until every client is Connected, then start the match.
    fn connect_and_start(&mut self) -> Vec<Event> {
        let want = self.clients.len();
        let mut events = Vec::new();
        for _ in 0..200 {
            let connected = self
                .clients
                .iter()
                .filter(|c| c.local_slot() != 0)
                .count();
            if connected == want && self.clients.iter().all(|c| c.is_started()) && self.host.is_started() {
                break;
            }
            events.extend(self.step());
        }
        self.host.start_match(0).unwrap();
        for _ in 0..100 {
            if self.clients.iter().all(|c| c.is_started()) {
                break;
            }
            events.extend(self.step());
        }
        events
    }
}

fn drain_desync(events: &[Event]) -> Vec<(u32, u64, u64)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Desync { tick, local, remote } => Some((*tick, *local, *remote)),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------- tests

#[test]
fn handshake_delivers_settings_seed_and_go() {
    let mut w = World::new(3, 0, 0, 0, 1);
    let mut events = w.run(60);
    w.host.start_match(0).unwrap();
    events.extend(w.run(60));
    assert!(
        events.iter().any(|e| matches!(e, Event::Listening)),
        "host must report Listening"
    );
    for s in 1..=3u8 {
        assert!(
            events.iter().any(|e| matches!(e, Event::ClientJoined { slot, .. } if *slot == s)),
            "host must report ClientJoined slot {s}"
        );
    }
    for s in 1..=3u8 {
        let ev = events.iter().find(|e| matches!(e, Event::Connected { slot, .. } if *slot == s));
        match ev {
            Some(Event::Connected { seed, settings, input_delay, .. }) => {
                assert_eq!(*seed, 0x1234_5678, "seed must reach the client");
                assert_eq!(*input_delay, 3);
                assert_eq!(*settings, MatchSettings::default());
            }
            other => panic!("client {s} never Connected: {other:?}"),
        }
    }
    // SLOTMAP: host human + 3 clients.
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::SlotMapChanged { human_slots } if *human_slots == [true, true, true, true])));
    // GO: everyone started at tick 0.
    assert!(events
        .iter()
        .filter(|e| matches!(e, Event::MatchStarted { first_tick: 0 }))
        .count() >= 3);
}

#[test]
fn version_and_build_mismatch_refused() {
    for (field, expect) in [("version", RefuseReason::Version), ("build", RefuseReason::Build)] {
        let mut w = World::new(1, 0, 0, 0, 2);
        match field {
            "version" => {
                if let Some(c) = w.clients.first_mut() {
                    let mut cfg = test_cfg();
                    cfg.protocol_version = nf_net::PROTOCOL_VERSION + 7;
                    // Rebuild the client with the bad config.
                    let ep = w.shim.endpoint(addr(7100));
                    *c = Session::new_client(cfg, ep, addr(7000));
                }
            }
            _ => {
                if let Some(c) = w.clients.first_mut() {
                    let mut cfg = test_cfg();
                    cfg.build_id = 12345;
                    let ep = w.shim.endpoint(addr(7100));
                    *c = Session::new_client(cfg, ep, addr(7000));
                }
            }
        }
        let events = w.run(60);
        assert!(
            events.iter().any(|e| matches!(e, Event::Refused { reason } if *reason == expect)),
            "{field} mismatch must be refused with {expect:?}, got {events:?}"
        );
        assert!(
            !events.iter().any(|e| matches!(e, Event::ClientJoined { .. })),
            "host must not accept a mismatched client"
        );
    }
}

#[test]
fn four_agents_2000_ticks_lossy_stay_in_sync() {
    // 5% drop, 30 ms one-way latency + 20 ms jitter — hostile but realistic.
    let mut w = World::new(3, 50, 30, 20, 0xDEAD_BEEF);
    w.connect_and_start();
    let target = 2000;
    let mut desyncs = Vec::new();
    for step in 0..400_000 {
        let events = w.step();
        desyncs.extend(drain_desync(&events));
        let all_done = w.host_sim.executed >= target
            && w.client_sims.iter().all(|s| s.executed >= target);
        if all_done {
            break;
        }
    }
    assert!(
        w.host_sim.executed >= target,
        "host reached only tick {} (of {target})",
        w.host_sim.executed
    );
    for (i, s) in w.client_sims.iter().enumerate() {
        assert!(s.executed >= target, "client {i} reached only tick {}", s.executed);
    }
    assert!(
        desyncs.is_empty(),
        "no desync allowed on identical input streams, got {desyncs:?}"
    );
    // All four machines must agree at every common tick (the host may run
    // a few ticks past the target — compare hashes AT tick 1999, and every
    // recorded tick for good measure).
    let h = w.host_sim.marks.iter().find(|(t, _)| *t == 1999).map(|(_, h)| *h)
        .expect("host must have a hash mark at tick 1999");
    for (i, s) in w.client_sims.iter().enumerate() {
        let ch = s.marks.iter().find(|(t, _)| *t == 1999).map(|(_, h)| *h)
            .unwrap_or_else(|| panic!("client {i} missing hash mark at tick 1999"));
        assert_eq!(ch, h, "hash mismatch at tick 1999 (client {i})");
        for (hm, cm) in w.host_sim.marks.iter().zip(s.marks.iter()) {
            assert_eq!(hm.0, cm.0, "tick sets must align (client {i})");
            assert_eq!(hm.1, cm.1, "client {i} diverged from host at tick {}", hm.0);
        }
    }
}

#[test]
fn one_flipped_input_byte_is_caught_as_desync() {
    // Sister test of the sync test: corrupt agent 2's input at tick 900;
    // the hash exchange at tick 900 (a multiple of 30) must flag it.
    let mut w = World::new(3, 50, 30, 20, 0xDEAD_BEEF);
    w.connect_and_start();
    w.corrupt = Some((2, 900));
    let mut desyncs = Vec::new();
    for _ in 0..400_000 {
        let events = w.step();
        desyncs.extend(drain_desync(&events));
        if !desyncs.is_empty() {
            break;
        }
    }
    assert!(!desyncs.is_empty(), "a one-bit input difference must surface as Desync");
    assert!(
        desyncs.iter().all(|(t, _, _)| *t >= 900),
        "desync must be at/after the corrupted tick, got {desyncs:?}"
    );
}

#[test]
fn stall_never_guesses_then_resumes() {
    let mut w = World::new(1, 0, 10, 0, 7);
    w.connect_and_start();
    w.run(300);
    let t0 = w.host_sim.executed;
    assert!(t0 >= 50, "expected progress before the outage, got {t0}");
    // Dead link: the client's inputs stop arriving entirely.
    w.outage = Some(addr(7100));
    let mut stalled_at = None;
    for _ in 0..2000 {
        let events = w.step();
        if let Some(Event::Stalled { tick, missing }) = events.iter().find(|e| matches!(e, Event::Stalled { .. })) {
            stalled_at = Some((*tick, missing.clone()));
            break;
        }
    }
    let (tick, missing) = stalled_at.expect("host must stall when inputs stop");
    assert_eq!(missing, vec![1], "only the client slot may be missing");
    assert!(tick >= t0, "stall tick must be at/after the last executed tick");
    // While the outage persists the sim must NOT advance — never a guess.
    let frozen = w.host_sim.executed;
    w.run(120); // ~2 s of logical time
    assert_eq!(w.host_sim.executed, frozen, "no tick may run while an input is missing");
    // Link back: inputs flow, the sim resumes and catches up.
    w.outage = None;
    let mut resumed = false;
    for _ in 0..2000 {
        let events = w.step();
        if events.iter().any(|e| matches!(e, Event::Resumed { .. })) {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "session must emit Resumed when inputs return");
    for _ in 0..4000 {
        w.step();
        if w.host_sim.executed >= frozen + 100 && w.client_sims[0].executed >= frozen + 100 {
            break;
        }
    }
    assert!(
        w.host_sim.executed >= frozen + 100 && w.client_sims[0].executed >= frozen + 100,
        "both sides must catch up after the stall (host {}, client {})",
        w.host_sim.executed,
        w.client_sims[0].executed
    );
}

#[test]
fn client_timeout_reported_then_neutral_continuation() {
    let mut w = World::new(1, 0, 0, 0, 11);
    w.connect_and_start();
    w.run(200);
    let t0 = w.host_sim.executed;
    // The client dies: link cut and its session stops running entirely.
    w.outage = Some(addr(7100));
    w.clients.clear();
    w.client_sims.clear();
    w.corrupt = None;
    let mut left = None;
    for _ in 0..400 {
        let events = w.step();
        if let Some(Event::ClientLeft { slot, reason }) =
            events.iter().find(|e| matches!(e, Event::ClientLeft { .. }))
        {
            left = Some((*slot, *reason));
            break;
        }
    }
    assert_eq!(
        left,
        Some((1, LeaveReason::Timeout)),
        "host must report the timed-out client"
    );
    // The game's own disconnected-pad behaviour: buttons 0 / sticks centre,
    // match runs on (psiInput_PollDevices 0x10ca78, Player_HandleDeath respawns).
    let t1 = w.host_sim.executed;
    for _ in 0..300 {
        w.step();
    }
    assert!(
        w.host_sim.executed > t1.max(t0) + 50,
        "match must continue on NEUTRAL inputs after the drop (host at {})",
        w.host_sim.executed
    );
}

#[test]
fn endmatch_policy_ends_match_instead() {
    let mut w = World::new(1, 0, 0, 0, 12);
    {
        // Host with the EndMatch policy.
        let shim = w.shim.clone();
        let ep = shim.endpoint(addr(7000));
        let mut cfg = test_cfg();
        cfg.drop_policy = DropPolicy::EndMatch;
        w.host = Session::new_host(cfg, ep, MatchSettings::default(), 0x1234_5678);
    }
    w.connect_and_start();
    w.run(200);
    w.outage = Some(addr(7100));
    w.clients.clear();
    let mut ended = false;
    for _ in 0..400 {
        let events = w.step();
        if events
            .iter()
            .any(|e| matches!(e, Event::MatchEnded { reason: LeaveReason::MatchEnded }))
        {
            ended = true;
            break;
        }
    }
    assert!(ended, "EndMatch policy must report MatchEnded on a client drop");
}

#[test]
fn real_udp_loopback_sync() {
    // Real sockets, no loss — proves the std::net::UdpSocket path end to end.
    let host_tr = UdpTransport::bind("127.0.0.1:7788").unwrap();
    let host_addr = host_tr.local_addr();
    let mut host = Session::new_host(test_cfg(), host_tr, MatchSettings::default(), 42);
    let mut clients: Vec<Session<UdpTransport>> = Vec::new();
    let mut client_addrs = Vec::new();
    for _ in 0..3 {
        let tr = UdpTransport::bind("127.0.0.1:0").unwrap();
        client_addrs.push(tr.local_addr());
        clients.push(Session::new_client(test_cfg(), tr, host_addr));
    }
    let mut sims: Vec<AgentSim> = (0..4).map(|_| AgentSim::new()).collect();
    let mut scheduled = vec![0u32; 4];
    let mut start_t = std::time::Instant::now();
    // Handshake.
    for _ in 0..2000 {
        for c in clients.iter_mut() {
            let _ = c.poll(0);
        }
        let _ = host.poll(0);
        std::thread::sleep(std::time::Duration::from_millis(1));
        if clients.iter().all(|c| c.local_slot() != 0) {
            break;
        }
    }
    assert!(
        clients.iter().all(|c| c.local_slot() != 0),
        "all clients must connect over real UDP"
    );
    host.start_match(0).unwrap();
    start_t = std::time::Instant::now();
    let mut ms = 0u64;
    for _ in 0..20_000 {
        for c in clients.iter_mut() {
            let _ = c.poll(ms);
        }
        let _ = host.poll(ms);
        ms += 16;
        std::thread::sleep(std::time::Duration::from_millis(1));
        for (slot, s) in std::iter::once(&mut host).enumerate().chain(clients.iter_mut().enumerate().map(|(i, s)| (i + 1, s))) {
            let cap = sims[slot].executed + 44;
            while scheduled[slot] < cap {
                s.local_input(scheduled[slot], agent_input(slot as u8, scheduled[slot]));
                scheduled[slot] += 1;
            }
            while scheduled[slot] > sims[slot].executed {
                match s.ready_inputs(sims[slot].executed) {
                    Some(inputs) => {
                        let h = sim_hash(sims[slot].executed, &inputs, sims[slot].prev_hash);
                        s.submit_hash(sims[slot].executed, h);
                        sims[slot].prev_hash = h;
                        let et = sims[slot].executed;
                        sims[slot].marks.push((et, h));
                        sims[slot].executed += 1;
                    }
                    None => break,
                }
            }
        }
        if sims.iter().all(|s| s.executed >= 300) || start_t.elapsed().as_secs() > 30 {
            break;
        }
    }
    assert!(sims.iter().all(|s| s.executed >= 300), "real-UDP run must reach 300 ticks: {:?}", sims.iter().map(|s| s.executed).collect::<Vec<_>>());
    // Compare hashes AT tick 299 (the host may run past 300 — chains of
    // different lengths must not be compared).
    let at = |m: &AgentSim, t: u32| m.marks.iter().find(|(mt, _)| *mt == t).map(|(_, h)| *h);
    let h = at(&sims[0], 299).expect("host hash at 299");
    for (i, s) in sims.iter().enumerate().skip(1) {
        assert_eq!(at(s, 299), Some(h), "hash mismatch at tick 299 (agent {i}) over real UDP");
    }
    let _ = client_addrs;
}
