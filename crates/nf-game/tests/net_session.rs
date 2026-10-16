//! net-match-host: the `nf_game::net` session tests — headless (no ISO), over loopback
//! (127.0.0.1) in one process. The lockstep contract: two (and three) sessions play scripted
//! ticks and every `ready_inputs` returns the same array on both sides; a forged hash desyncs;
//! a wrong build id is refused; a silent peer is dropped.

use nf_game::net::{pack_input, unpack_input, pack_settings, unpack_settings, NetEvent, NetSession, Phase};
use nf_game::player::Input;
use std::time::Duration;

fn drain(s: &mut NetSession) -> Vec<NetEvent> {
    let mut out = Vec::new();
    for _ in 0..50 {
        let evs = s.poll();
        if evs.is_empty() {
            if out.iter().any(|e| matches!(e, NetEvent::MatchStarted(_))) { break; }
            if out.is_empty() { std::thread::sleep(Duration::from_millis(2)); continue; }
            break;
        }
        out.extend(evs);
    }
    out
}

fn scripted_input(t: u32, slot: usize) -> Input {
    Input {
        forward: ((t as i32 + slot as i32) % 97) as f32 / 97.0,
        strafe: if t % 11 == 0 { -0.5 } else { 0.0 },
        turn: ((t % 13) as f32 - 6.0) * 0.001,
        look: 0.0,
        jump: t % 61 == 0,
        crouch: t % 43 == 0,
        aiming: false,
        weapon_cycle: if t % 71 == 0 { 1 } else { 0 },
        gadget_cycle: 0,
        switch_to: if t % 37 == 0 { Some(2) } else { None },
        zoom: false,
        scope: false,
        fire: t % 23 == 0,
        mode_switch: false,
    }
}

/// The wire packing round-trips exactly (the lockstep exchanges these verbatim).
#[test]
fn input_and_settings_pack_round_trip() {
    let i = scripted_input(1234, 1);
    let back = unpack_input(&pack_input(&i));
    assert_eq!(back.forward.to_bits(), i.forward.to_bits());
    assert_eq!(back.turn.to_bits(), i.turn.to_bits());
    assert_eq!(back.fire, i.fire);
    assert_eq!(back.jump, i.jump);
    assert_eq!(back.switch_to, i.switch_to);
    assert_eq!(back.weapon_cycle, i.weapon_cycle);
    // Sister: a different input does not decode to the same bytes.
    let other = scripted_input(1234, 2);
    assert_ne!(pack_input(&i), pack_input(&other));

    let mut s = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    s.scenario = 3;
    s.frag_limit = 44;
    s.time_limit = 7;
    s.human_count = 2;
    s.player_count = 2;
    s.seed = 0xdead_beef;
    s.players[1].team = 1;
    s.players[1].skin = 2;
    let back = unpack_settings(&pack_settings(&s)).expect("settings round-trip");
    assert_eq!(back.level_code, s.level_code);
    assert_eq!(back.scenario, s.scenario);
    assert_eq!(back.frag_limit, s.frag_limit);
    assert_eq!(back.time_limit, s.time_limit);
    assert_eq!(back.seed, s.seed);
    assert_eq!(back.human_count, 2);
    assert_eq!(back.players[1].team, 1);
    assert_eq!(back.players[1].skin, 2);
    // Sister: a truncated blob is rejected, not half-decoded.
    assert!(unpack_settings(&pack_settings(&s)[..60]).is_none());
}

/// Host + one client: handshake → the host's settings + seed reach the client → 1000 lockstep
/// ticks exchange identical inputs on both sides.
#[test]
fn host_and_client_lockstep_1000_ticks() {
    let mut host = NetSession::host(27451).expect("host binds");
    let mut client = NetSession::connect("127.0.0.1:27451", "Alec").expect("client dials");

    let hev = drain(&mut host);
    assert!(hev.iter().any(|e| matches!(e, NetEvent::AgentJoined { slot: 1, .. })), "host sees the join: {hev:?}");
    let cev = drain(&mut client);
    assert!(cev.iter().any(|e| matches!(e, NetEvent::Joined { slot: 1 })), "client got its slot: {cev:?}");

    let mut st = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    st.human_count = 2;
    st.player_count = 2;
    st.total_players = 2;
    st.seed = 0x1234_abcd;
    host.start_match(&st);

    let hev = drain(&mut host);
    let started = hev.iter().any(|e| matches!(e, NetEvent::MatchStarted(_)));
    assert!(started, "host arms its own start: {hev:?}");
    let cev = drain(&mut client);
    let got = cev.iter().find_map(|e| match e {
        NetEvent::MatchStarted(s) => Some((**s).clone()),
        _ => None,
    });
    let cst = got.expect("client receives the settings");
    assert_eq!(cst.seed, 0x1234_abcd);
    assert_eq!(cst.level_code, 0x0700_0023);
    assert_eq!(cst.human_count, 2);
    assert_eq!(client.phase(), Phase::InMatch);

    // 1000 lockstep ticks: each side submits its scripted input, the pump delivers (the app's
    // frame order: poll → submit → gate), and both read the same array.
    for t in 0..1000u32 {
        host.local_input(t, scripted_input(t, 0));
        client.local_input(t, scripted_input(t, 1));
        // A missing input is a stall, never a guess (the session contract): the app's frame waits
        // and polls again. Loopback UDP can land a few hundred microseconds after the send, so the
        // test pumps until both sides hold tick `t` (bounded at 1 s) instead of demanding the packet
        // arrive by the very next poll — that assumption made the test flaky (1-3 in 6 runs).
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        let (hi, ci) = loop {
            let _ = host.poll();
            let _ = client.poll();
            if let (Some(h), Some(c)) = (host.ready_inputs(t), client.ready_inputs(t)) { break (h, c); }
            assert!(std::time::Instant::now() < deadline, "tick {t}: both inputs arrive within 1 s (host {:?}, client {:?})",
                host.ready_inputs(t).is_some(), client.ready_inputs(t).is_some());
            std::thread::sleep(Duration::from_micros(200));
        };
        assert_eq!(hi[0].forward.to_bits(), ci[0].forward.to_bits(), "tick {t} slot 0");
        assert_eq!(hi[1].forward.to_bits(), ci[1].forward.to_bits(), "tick {t} slot 1");
        assert_eq!(hi[1].fire, scripted_input(t, 1).fire, "tick {t} slot 1 fire");
        assert_eq!(hi[0].fire, scripted_input(t, 0).fire, "tick {t} slot 0 fire");
        // The hash exchange (every 15th tick, the app's cadence) stays silent.
        if t % 15 == 0 {
            host.send_state_hash(t, 0x1000 + t as u64);
            client.send_state_hash(t, 0x1000 + t as u64);
        }
        let _ = host.poll();
        let _ = client.poll();
    }
    assert!(!matches!(host.phase(), Phase::Failed), "host stayed in sync");
    assert!(!matches!(client.phase(), Phase::Failed), "client stayed in sync");
}

/// A forged state hash at a shared tick is a desync on BOTH sides (the forced-desync detection).
#[test]
fn forged_hash_desyncs() {
    let mut host = NetSession::host(27452).expect("host binds");
    let mut client = NetSession::connect("127.0.0.1:27452", "Alec").expect("client dials");
    let _ = drain(&mut host);
    let _ = drain(&mut client);
    let mut st = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    st.human_count = 2;
    host.start_match(&st);
    let _ = drain(&mut host);
    let _ = drain(&mut client);

    // 5 honest ticks, then the host lies about its hash at tick 5.
    for t in 0..5u32 {
        host.local_input(t, scripted_input(t, 0));
        client.local_input(t, scripted_input(t, 1));
        let _ = host.poll();
        let _ = client.poll();
        assert!(host.ready_inputs(t).is_some() && client.ready_inputs(t).is_some());
    }
    client.send_state_hash(5, 0xaaaa_0000_0000_0001);
    host.send_state_hash(5, 0xaaaa_0000_0000_0002);
    let hev = drain(&mut host);
    let cev = drain(&mut client);
    assert!(hev.iter().any(|e| matches!(e, NetEvent::Desynced { tick: 5 })), "host desyncs: {hev:?}");
    assert!(cev.iter().any(|e| matches!(e, NetEvent::Desynced { tick: 5 })), "client desyncs: {cev:?}");
    assert_eq!(host.phase(), Phase::Failed);
    assert_eq!(client.phase(), Phase::Failed);
    // Sister: equal hashes at the same tick do NOT desync (covered by the 1000-tick run above).
}

/// A mismatched protocol version is refused cleanly, before any slot is assigned.
#[test]
fn version_mismatch_refused() {
    let host_sock = std::net::UdpSocket::bind("127.0.0.1:0").expect("fake client binds");
    host_sock.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
    let mut host = NetSession::host(27453).expect("host binds");
    // A raw HELLO with a wrong build id.
    let mut hello = vec![1u8];
    hello.extend_from_slice(&nf_game::net::PROTOCOL_VERSION.to_le_bytes());
    hello.extend_from_slice(&0xdead_c0de_u32.to_le_bytes()); // wrong build
    hello.push(4u8);
    hello.extend_from_slice(b"Alec");
    host_sock.send_to(&hello, "127.0.0.1:27453").expect("hello sent");
    std::thread::sleep(Duration::from_millis(20));
    let _ = host.poll();
    let mut buf = [0u8; 64];
    let (n, _) = host_sock.recv_from(&mut buf).expect("a DENY arrives");
    assert_eq!(buf[0], 3, "the host denies, not welcomes");
    assert_eq!(buf[1], 1, "the reason is the version/build mismatch");
    // The fake client never joined: the host has no agents beyond itself.
    assert_eq!(host.agent_count(), 1);
    let _ = n;
}

/// A silent peer is dropped and ends the match (PeerLost), inside the shrunk timeout.
#[test]
fn silent_peer_times_out() {
    let mut host = NetSession::host(27454).expect("host binds");
    host.set_peer_timeout(Duration::from_millis(120));
    let mut client = NetSession::connect("127.0.0.1:27454", "Alec").expect("client dials");
    let _ = drain(&mut host);
    let _ = drain(&mut client);
    let mut st = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    st.human_count = 2;
    host.start_match(&st);
    let _ = drain(&mut host);
    let _ = drain(&mut client);
    // The client goes silent: no inputs, no polls past the handshake.
    std::thread::sleep(Duration::from_millis(200));
    let hev = drain(&mut host);
    assert!(hev.iter().any(|e| matches!(e, NetEvent::PeerLost { slot: 1 })), "host drops the peer: {hev:?}");
    assert!(matches!(host.phase(), Phase::Failed));
}
