//! net-consolidate: the app-level lockstep test, ported from 053's
//! `nf_game::net` to the ONE session layer — two app sims in one process over
//! loopback (127.0.0.1) play 1000 ticks of scripted input through the real
//! load path and end in the same state (per-tick hash equality); a forced
//! desync is detected; a disconnect is reported. NF_ISO-gated (the arenas
//! load from the disc like `mp_core_live`).

use nf_game::mp::MpSettings;
use nf_game::player::Input;
use nf_net::net::{Transport, UdpTransport};
use nf_net::session::{Event, Session};
use nf_net::{MatchSettings, NEUTRAL};

use nf_view::mpnet::{match_to_mp, mp_to_match, to_player_input};

fn iso() -> Option<String> {
    std::env::var("NF_ISO").ok()
}

fn scripted_input(t: u32, slot: u8) -> nf_net::Input {
    let mut i = NEUTRAL;
    i.forward = ((t as i32 + slot as i32) % 97) as f32 / 97.0;
    i.strafe = if t % 11 == 0 { -0.5 } else { 0.0 };
    i.turn = ((t % 13) as f32 - 6.0) * 0.002;
    i.look = if t % 29 == 0 { 0.25 } else { 0.0 };
    i.jump = t % 61 == 0;
    i.crouch = t % 43 == 0;
    i.fire = t % 23 == 0;
    i
}

fn cfg() -> nf_net::Config {
    nf_net::Config { hash_every: 15, ..Default::default() }
}

struct Peer {
    sess: Session<UdpTransport>,
    executed: u32,
    prev_hash: u64,
}

fn host(port: u16) -> Peer {
    Peer { sess: Session::new_host(cfg(), UdpTransport::bind(&format!("127.0.0.1:{port}")).unwrap(),
        MatchSettings::default(), 0), executed: 0, prev_hash: 0 }
}
fn client(port: u16) -> Peer {
    Peer { sess: Session::new_client(cfg(), UdpTransport::bind("127.0.0.1:0").unwrap(),
        format!("127.0.0.1:{port}").parse().unwrap()), executed: 0, prev_hash: 0 }
}

/// Pump until both sides hold tick `t` (a missing input is a stall, never a
/// guess — bounded at 5 s).
fn pump_till(h: &mut Peer, c: &mut Peer, ms: &mut u64, t: u32) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        *ms += 5;
        let _ = h.sess.poll(*ms);
        let _ = c.sess.poll(*ms);
        if h.sess.ready_inputs(t).is_some() && c.sess.ready_inputs(t).is_some() { return; }
        assert!(std::time::Instant::now() < deadline, "tick {t}: both inputs arrive within 5 s");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// Two sessions over loopback driving two real MP sims (the host's arena, the host's settings +
/// seed through `load_mp_carry`): 1000 lockstep ticks, the hash every 15 ticks — both sims end
/// in the same state (the hashes agree at every exchange, the final poses are equal).
#[test]
fn two_sims_1000_ticks_same_state() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut h = host(27910);
    let mut c = client(27910);
    let mut ms: u64 = 0;
    let mut st: Option<MpSettings> = None;
    // Handshake: the host's settings + seed reach the client (the WELCOME
    // payload the app's MatchStarted path delivers).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while st.is_none() {
        ms += 5;
        let _ = h.sess.poll(ms);
        for e in c.sess.poll(ms) {
            if let Event::Connected { settings, seed, .. } = e {
                st = Some(match_to_mp(&settings, seed));
            }
        }
        if h.sess.agents().len() == 2 && st.is_some() { break; }
        assert!(std::time::Instant::now() < deadline, "handshake");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    // The host's real match settings (the arena + 2 humans + the seed) arm
    // the session the way `NetCore::start_match` does.
    let mut s = MpSettings::deathmatch(0x0700_0023);
    s.human_count = 2;
    s.player_count = 2;
    s.total_players = 2;
    s.seed = 0x0bed_0bad;
    h.sess.set_host_match(mp_to_match(&s), s.seed);
    h.sess.start_match(0).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !c.sess.is_started() {
        ms += 5;
        let _ = h.sess.poll(ms);
        let _ = c.sess.poll(ms);
        assert!(std::time::Instant::now() < deadline, "GO");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let _ = st;

    // Both machines build the same level from the same settings (the app's
    // `load_mp_carry`): every human slot's bond spawns in slot order from the
    // same draws, then each machine swaps ITS slot's bond in as the local one.
    let arena = format!("{:08x}.bin", s.level_code);
    let mut hs = nf_view::load_mp_carry(&path, &arena, false, None, s.clone()).expect("host sim loads");
    let mut cs = nf_view::load_mp_carry(&path, &arena, false, None, s).expect("client sim loads");
    let host_sim = hs.sim.as_mut().expect("host sim");
    host_sim.swap_local_slot(0);
    let client_sim = cs.sim.as_mut().expect("client sim");
    client_sim.swap_local_slot(1);
    let mut last_h = 0u64;
    let mut last_c = 0u64;
    for t in 0..1000u32 {
        h.sess.local_input(t, scripted_input(t, 0));
        c.sess.local_input(t, scripted_input(t, 1));
        pump_till(&mut h, &mut c, &mut ms, t);
        let hi = h.sess.ready_inputs(t).unwrap();
        let ci = c.sess.ready_inputs(t).unwrap();
        let hi = std::array::from_fn(|i| to_player_input(&hi[i]));
        let ci = std::array::from_fn(|i| to_player_input(&ci[i]));
        host_sim.step_net_remotes(&hi, 0);
        client_sim.step_net_remotes(&ci, 1);
        // The local agents tick through the app's per-agent core (the full local path is
        // step_bond; the movement core both machines agree on is tick_ctx — the hash covers it).
        let ladder = nf_game::player::LadderCtx::default();
        let hin = hi[0];
        let cin = ci[1];
        host_sim.bond.tick_ctx(&host_sim.world, &ladder, &hin);
        client_sim.bond.tick_ctx(&client_sim.world, &ladder, &cin);
        if let Some(m) = host_sim.mp.as_mut() {
            m.set_slot(0, host_sim.bond.pos);
            if let Some(b) = &host_sim.mp_bonds[1] { m.set_slot(1, b.pos); }
        }
        if let Some(m) = client_sim.mp.as_mut() {
            m.set_slot(1, client_sim.bond.pos);
            if let Some(b) = &client_sim.mp_bonds[0] { m.set_slot(0, b.pos); }
        }
        if t % 15 == 0 {
            last_h = host_sim.net_state_hash(0);
            last_c = client_sim.net_state_hash(1);
            assert_eq!(last_h, last_c, "tick {t}: the sims' states agree");
            h.sess.submit_hash(t, last_h);
            c.sess.submit_hash(t, last_c);
        }
        let _ = h.sess.poll(ms);
        let _ = c.sess.poll(ms);
    }
    let hev = h.sess.poll(ms);
    let cev = c.sess.poll(ms);
    assert!(!hev.iter().any(|e| matches!(e, Event::Desync { .. })), "no desync over the honest run");
    assert!(!cev.iter().any(|e| matches!(e, Event::Desync { .. })), "no desync over the honest run");
    assert_eq!(last_h, last_c, "the final hashes agree");
    // Cross-check the poses in slot order (each machine's local agent is the other's remote):
    // slot 0 lives in the host's `bond` and the client's `mp_bonds[0]`, slot 1 the mirror.
    assert_eq!(host_sim.bond.pos, client_sim.mp_bonds[0].as_ref().expect("slot 0 on the client").pos, "slot 0's pose agrees");
    assert_eq!(host_sim.mp_bonds[1].as_ref().expect("slot 1 on the host").pos, client_sim.bond.pos, "slot 1's pose agrees");
}

/// A forced desync: one machine's state is poked (an extra RNG draw — the exact symptom an
/// unshared draw causes) and the hash exchange detects it within one cadence.
#[test]
fn forced_desync_is_detected() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut h = host(27930);
    let mut c = client(27930);
    let mut ms: u64 = 0;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !(h.sess.agents().len() == 2 && c.sess.local_slot() != 0) {
        ms += 5;
        let _ = h.sess.poll(ms);
        let _ = c.sess.poll(ms);
        assert!(std::time::Instant::now() < deadline, "handshake");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let mut s = MpSettings::deathmatch(0x0700_0023);
    s.human_count = 2;
    s.player_count = 2;
    s.total_players = 2;
    s.seed = 7;
    h.sess.set_host_match(mp_to_match(&s), s.seed);
    h.sess.start_match(0).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !c.sess.is_started() {
        ms += 5;
        let _ = h.sess.poll(ms);
        let _ = c.sess.poll(ms);
        assert!(std::time::Instant::now() < deadline, "GO");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let arena = format!("{:08x}.bin", s.level_code);
    let mut hs = nf_view::load_mp_carry(&path, &arena, false, None, s.clone()).expect("host sim loads");
    let mut cs = nf_view::load_mp_carry(&path, &arena, false, None, s).expect("client sim loads");
    let host_sim = hs.sim.as_mut().unwrap();
    host_sim.swap_local_slot(0);
    let client_sim = cs.sim.as_mut().unwrap();
    client_sim.swap_local_slot(1);

    for t in 0..60u32 {
        h.sess.local_input(t, scripted_input(t, 0));
        c.sess.local_input(t, scripted_input(t, 1));
        pump_till(&mut h, &mut c, &mut ms, t);
        let hi = h.sess.ready_inputs(t).unwrap();
        let ci = c.sess.ready_inputs(t).unwrap();
        let hi = std::array::from_fn(|i| to_player_input(&hi[i]));
        let ci = std::array::from_fn(|i| to_player_input(&ci[i]));
        host_sim.step_net_remotes(&hi, 0);
        client_sim.step_net_remotes(&ci, 1);
        let ladder = nf_game::player::LadderCtx::default();
        host_sim.bond.tick_ctx(&host_sim.world, &ladder, &hi[0]);
        client_sim.bond.tick_ctx(&client_sim.world, &ladder, &ci[1]);
        // The poke at tick 30: the host's shared stream takes an extra draw (a desynced machine's
        // symptom — the other side never sees it through the input channel).
        if t == 30 { let _ = host_sim.drones.rng.frand(100.0); }
        if t % 15 == 0 {
            h.sess.submit_hash(t, host_sim.net_state_hash(0));
            c.sess.submit_hash(t, client_sim.net_state_hash(1));
        }
        let hev = h.sess.poll(ms);
        let cev = c.sess.poll(ms);
        if hev.iter().any(|e| matches!(e, Event::Desync { .. }))
            || cev.iter().any(|e| matches!(e, Event::Desync { .. })) {
            assert!(t >= 30, "the desync came from the poke, not the script");
            assert!(t <= 45, "detected within one cadence after the poke (t={t})");
            return;
        }
    }
    panic!("the forced desync was never detected");
}

/// A disconnect: the client stops — the host reports ClientLeft(Timeout)
/// inside its timeout; DropPolicy::Neutral keeps the match running on
/// NEUTRAL inputs (the game's own disconnected-pad shape, the consolidation's
/// behaviour change from the old terminal PeerLost).
#[test]
fn disconnect_reported_match_runs_on() {
    let mut h = host(27940);
    let mut c = client(27940);
    let mut ms: u64 = 0;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !(h.sess.agents().len() == 2 && c.sess.local_slot() != 0) {
        ms += 5;
        let _ = h.sess.poll(ms);
        let _ = c.sess.poll(ms);
        assert!(std::time::Instant::now() < deadline, "handshake");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let mut s = MpSettings::deathmatch(0x0700_0023);
    s.human_count = 2;
    s.player_count = 2;
    s.total_players = 2;
    h.sess.set_host_match(mp_to_match(&s), s.seed);
    h.sess.start_match(0).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !c.sess.is_started() {
        ms += 5;
        let _ = h.sess.poll(ms);
        let _ = c.sess.poll(ms);
        assert!(std::time::Instant::now() < deadline, "GO");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    for t in 0..3u32 {
        h.sess.local_input(t, scripted_input(t, 0));
        c.sess.local_input(t, scripted_input(t, 1));
        pump_till(&mut h, &mut c, &mut ms, t);
    }
    // The client dies: its session stops polling entirely.
    drop(c);
    let mut left = false;
    // 5 s timeout, the pump advances 5 ms per iteration — bounded run.
    let mut iters = 0;
    while iters < 4000 {
        ms += 5;
        iters += 1;
        for e in h.sess.poll(ms) {
            if matches!(e, Event::ClientLeft { slot: 1, reason: nf_net::LeaveReason::Timeout }) {
                left = true;
            }
        }
        if left { break; }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(left, "the host reports the timed-out client");
    // The match runs on: slot 1 rides NEUTRAL, never a stall.
    for t in 3..10u32 {
        h.sess.local_input(t, scripted_input(t, 0));
        ms += 5;
        let _ = h.sess.poll(ms);
        h.sess.ready_inputs(t).expect("NEUTRAL continuation");
    }
}
