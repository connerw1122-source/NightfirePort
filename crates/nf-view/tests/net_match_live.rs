//! net-match-host: the app-level lockstep test — two app sims in one process over loopback
//! (127.0.0.1) play 1 000 ticks of scripted input through the real load path and end in the same
//! state (per-tick hash equality); a forced desync is detected; a disconnect is reported.
//! NF_ISO-gated (the arenas load from the disc like `mp_core_live`).

use nf_game::net::{NetEvent, NetSession, Phase};
use nf_game::player::Input;

fn iso() -> Option<String> {
    std::env::var("NF_ISO").ok()
}

fn scripted_input(t: u32, slot: usize) -> Input {
    Input {
        forward: ((t as i32 + slot as i32) % 97) as f32 / 97.0,
        strafe: if t % 11 == 0 { -0.5 } else { 0.0 },
        turn: ((t % 13) as f32 - 6.0) * 0.002,
        look: if t % 29 == 0 { 0.25 } else { 0.0 },
        jump: t % 61 == 0,
        crouch: t % 43 == 0,
        aiming: false,
        weapon_cycle: 0,
        gadget_cycle: 0,
        switch_to: None,
        zoom: false,
        scope: false,
        fire: false,
        mode_switch: false,
    }
}

/// Two sessions over loopback driving two real MP sims (the host's arena, the host's settings +
/// seed through `load_mp_carry`): 1 000 lockstep ticks, the hash every 15 ticks — both sims end
/// in the same state (the hashes agree at every exchange, the final poses are equal).
#[test]
fn two_sims_1000_ticks_same_state() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut host = NetSession::host(27461).expect("host binds");
    let mut client = NetSession::connect("127.0.0.1:27461", "Alec").expect("client dials");
    for _ in 0..100 { let _ = host.poll(); let _ = client.poll(); if host.agent_count() > 1 { break; } }
    assert!(host.agent_count() > 1, "client joined");

    // The host's settings + seed — the same handshake the app's MatchStarted path delivers.
    let mut st = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    st.human_count = 2;
    st.player_count = 2;
    st.total_players = 2;
    st.seed = 0x0bed_0bad;
    host.start_match(&st);
    for _ in 0..100 {
        let _ = host.poll();
        let cev = client.poll();
        if cev.iter().any(|e| matches!(e, NetEvent::MatchStarted(_))) { break; }
    }
    assert_eq!(client.phase(), Phase::InMatch);

    // Both machines build the same level from the same settings (the app's `load_mp_carry`):
    // every human slot's bond spawns in slot order from the same draws, then each machine swaps
    // ITS slot's bond in as the local one.
    let arena = format!("{:08x}.bin", st.level_code);
    let mut hs = nf_view::load_mp_carry(&path, &arena, false, None, st.clone()).expect("host sim loads");
    let mut cs = nf_view::load_mp_carry(&path, &arena, false, None, st).expect("client sim loads");
    let host_sim = hs.sim.as_mut().expect("host sim");
    host_sim.swap_local_slot(0);
    let client_sim = cs.sim.as_mut().expect("client sim");
    client_sim.swap_local_slot(1);
    let mut last_h = 0u64;
    let mut last_c = 0u64;
    for t in 0..1000u32 {
        host.local_input(t, scripted_input(t, 0));
        client.local_input(t, scripted_input(t, 1));
        let _ = host.poll();
        let _ = client.poll();
        let hi = host.ready_inputs(t).expect("host ready");
        let ci = client.ready_inputs(t).expect("client ready");
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
            host.send_state_hash(t, last_h);
            client.send_state_hash(t, last_c);
        }
        let _ = host.poll();
        let _ = client.poll();
    }
    assert!(!matches!(host.phase(), Phase::Failed), "no desync over the honest run");
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
    let mut host = NetSession::host(27463).unwrap();
    let mut client = NetSession::connect("127.0.0.1:27463", "Alec").unwrap();
    for _ in 0..100 { let _ = host.poll(); let _ = client.poll(); if host.agent_count() > 1 { break; } }
    let mut st = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    st.human_count = 2;
    st.seed = 7;
    host.start_match(&st);
    for _ in 0..100 {
        let _ = host.poll();
        if client.poll().iter().any(|e| matches!(e, NetEvent::MatchStarted(_))) { break; }
    }
    let arena = format!("{:08x}.bin", st.level_code);
    let mut hs = nf_view::load_mp_carry(&path, &arena, false, None, st.clone()).expect("host sim loads");
    let mut cs = nf_view::load_mp_carry(&path, &arena, false, None, st).expect("client sim loads");
    let host_sim = hs.sim.as_mut().unwrap();
    host_sim.swap_local_slot(0);
    let client_sim = cs.sim.as_mut().unwrap();
    client_sim.swap_local_slot(1);

    for t in 0..60u32 {
        host.local_input(t, scripted_input(t, 0));
        client.local_input(t, scripted_input(t, 1));
        let _ = host.poll();
        let _ = client.poll();
        let hi = host.ready_inputs(t).expect("ready");
        let ci = client.ready_inputs(t).expect("ready");
        host_sim.step_net_remotes(&hi, 0);
        client_sim.step_net_remotes(&ci, 1);
        let ladder = nf_game::player::LadderCtx::default();
        host_sim.bond.tick_ctx(&host_sim.world, &ladder, &hi[0]);
        client_sim.bond.tick_ctx(&client_sim.world, &ladder, &ci[1]);
        // The poke at tick 30: the host's shared stream takes an extra draw (a desynced machine's
        // symptom — the other side never sees it through the input channel).
        if t == 30 { let _ = host_sim.drones.rng.frand(100.0); }
        if t % 15 == 0 {
            host.send_state_hash(t, host_sim.net_state_hash(0));
            client.send_state_hash(t, client_sim.net_state_hash(1));
        }
        let hev = host.poll();
        let cev = client.poll();
        if hev.iter().any(|e| matches!(e, NetEvent::Desynced { .. }))
            || cev.iter().any(|e| matches!(e, NetEvent::Desynced { .. })) {
            assert!(t >= 30, "the desync came from the poke, not the script");
            assert!(t <= 45, "detected within one cadence after the poke (t={t})");
            return;
        }
    }
    panic!("the forced desync was never detected");
}

/// A disconnect: the client stops — the host reports PeerLost inside its timeout and the match
/// ends (the app's net_fail shows the message box and returns to the lobby; the session half is
/// what the session test pins, this one drives it against a real sim).
#[test]
fn disconnect_ends_the_match() {
    let mut host = NetSession::host(27464).unwrap();
    host.set_peer_timeout(std::time::Duration::from_millis(150));
    let mut client = NetSession::connect("127.0.0.1:27464", "Alec").unwrap();
    for _ in 0..100 { let _ = host.poll(); let _ = client.poll(); if host.agent_count() > 1 { break; } }
    let mut st = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    st.human_count = 2;
    host.start_match(&st);
    for _ in 0..100 {
        let _ = host.poll();
        if client.poll().iter().any(|e| matches!(e, NetEvent::MatchStarted(_))) { break; }
    }
    drop(client);
    std::thread::sleep(std::time::Duration::from_millis(250));
    let hev = host.poll();
    assert!(hev.iter().any(|e| matches!(e, NetEvent::PeerLost { slot: 1 })), "the drop is reported: {hev:?}");
    assert!(matches!(host.phase(), Phase::Failed));
}
