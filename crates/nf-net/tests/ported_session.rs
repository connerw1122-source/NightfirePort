//! net-consolidate: 053's `nf_game::net` session tests, ported to the ONE
//! session layer (the old module is deleted; every test from both layers is
//! kept or ported). Headless, over loopback (127.0.0.1) in one process:
//! the lockstep contract per-tick over real UDP, a forged hash desyncs, a
//! wrong build id is refused on the raw wire, a silent peer times out.

use std::time::Duration;

use nf_net::net::{Transport, UdpTransport};
use nf_net::session::{Event, Session};
use nf_net::{Input, MatchSettings, NEUTRAL};

fn scripted_input(t: u32, slot: u8) -> Input {
    let mut i = NEUTRAL;
    i.forward = ((t as i32 + slot as i32) % 97) as f32 / 97.0;
    i.strafe = if t % 11 == 0 { -0.5 } else { 0.0 };
    i.turn = ((t % 13) as f32 - 6.0) * 0.001;
    i.jump = t % 61 == 0;
    i.crouch = t % 43 == 0;
    i.weapon_cycle = if t % 71 == 0 { 1 } else { 0 };
    i.switch_to = if t % 37 == 0 { Some(2) } else { None };
    i.fire = t % 23 == 0;
    i
}

fn cfg() -> nf_net::Config {
    nf_net::Config {
        input_delay: 1, // 053's sample-and-use shape: the tightest lockstep
        hash_every: 15,
        timeout_ms: 4000,
        keepalive_ms: 100,
        hello_retry_ms: 20,
        resend_ms: 10,
        ..Default::default()
    }
}

fn two_peers(port: u16) -> (Session<UdpTransport>, Session<UdpTransport>) {
    two_peers_timeout(port, 4000)
}

fn two_peers_timeout(port: u16, timeout_ms: u64) -> (Session<UdpTransport>, Session<UdpTransport>) {
    let c = nf_net::Config { timeout_ms, ..cfg() };
    let host = Session::new_host(c.clone(), UdpTransport::bind(&format!("127.0.0.1:{port}")).expect("host binds"),
        MatchSettings::default(), 0x1234_5678);
    let client = Session::new_client(c, UdpTransport::bind("127.0.0.1:0").expect("client binds"),
        format!("127.0.0.1:{port}").parse().unwrap());
    (host, client)
}


/// Pump with an advancing logical clock until `f` holds (bounded). The
/// client may be gone (the timeout test's silence is the point).
fn pump_until(host: &mut Session<UdpTransport>, client: Option<&mut Session<UdpTransport>>,
              ms: &mut u64, mut f: impl FnMut(&mut Session<UdpTransport>, Option<&mut Session<UdpTransport>>, u64, &[Event], &[Event]) -> bool,
              what: &'static str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut c = client;
    loop {
        *ms += 5;
        // Poll FIRST and hand the events up — nothing is lost between pumps.
        let hev = host.poll(*ms);
        let cev = c.as_deref_mut().map(|cc| cc.poll(*ms)).unwrap_or_default();
        if f(&mut *host, c.as_deref_mut(), *ms, &hev, &cev) { return; }
        assert!(std::time::Instant::now() < deadline, "{what} never completed");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn pump2(host: &mut Session<UdpTransport>, client: &mut Session<UdpTransport>,
         ms: &mut u64, mut f: impl FnMut(&mut Session<UdpTransport>, &mut Session<UdpTransport>, u64, &[Event], &[Event]) -> bool,
         what: &'static str) {
    pump_until(host, Some(client), ms, |h, c, m, hev, cev| f(h, c.expect("two-peer pump"), m, hev, cev), what)
}

/// Drain both sessions' events (the pump eats them; collect what we assert on).
fn drain(h: &mut Session<UdpTransport>, c: &mut Session<UdpTransport>, ms: u64)
    -> (Vec<Event>, Vec<Event>) {
    (h.poll(ms), c.poll(ms))
}

/// Host + one client: handshake → the host's settings + seed reach the client → 300 lockstep
/// ticks exchange identical inputs on both sides (the e2e suite's 600-tick
/// lossy+reordered run pins the same per-tick equality under worse links;
/// this port keeps 053's clean-link per-tick contract at a bounded runtime).
#[test]
fn host_and_client_lockstep_300_ticks() {
    let (mut host, mut client) = two_peers_timeout(27810, 4000);
    let mut ms: u64 = 0;
    let mut connected: Option<(u8, u32, MatchSettings, u32)> = None;
    pump2(&mut host, &mut client, &mut ms,
        |h, c, _, _, cev| {
            if connected.is_none() {
                for e in cev {
                    if let Event::Connected { slot, seed, settings, input_delay } = e {
                        connected = Some((*slot, *seed, settings.clone(), *input_delay));
                    }
                }
            }
            c.local_slot() != 0 && h.agents().len() == 2
        }, "handshake");
    let (slot, seed, _settings, _delay) = connected.expect("client Connected with the handshake payload");
    assert_eq!(slot, 1);
    assert_eq!(seed, 0x1234_5678, "the seed rides the handshake");

    host.start_match(0).unwrap();
    pump2(&mut host, &mut client, &mut ms, |_, c, _, _, _| c.is_started(), "GO");

    // 300 lockstep ticks: each side submits its scripted input, the pump
    // delivers (a missing input is a stall, never a guess), both read the
    // same array.
    for t in 0..300u32 {
        host.local_input(t, scripted_input(t, 0));
        client.local_input(t, scripted_input(t, 1));
        pump2(&mut host, &mut client, &mut ms,
            |h, c, _, _, _| h.ready_inputs(t).is_some() && c.ready_inputs(t).is_some(),
            "tick inputs");
        let hi = host.ready_inputs(t).unwrap();
        let ci = client.ready_inputs(t).unwrap();
        assert_eq!(hi[0].forward.to_bits(), ci[0].forward.to_bits(), "tick {t} slot 0");
        assert_eq!(hi[1].forward.to_bits(), ci[1].forward.to_bits(), "tick {t} slot 1");
        assert_eq!(hi[1].fire, scripted_input(t, 1).fire, "tick {t} slot 1 fire");
        assert_eq!(hi[0].fire, scripted_input(t, 0).fire, "tick {t} slot 0 fire");
        // The hash exchange (every 15th tick, the app's cadence) stays silent.
        if t % 15 == 0 {
            host.submit_hash(t, 0x1000 + t as u64);
            client.submit_hash(t, 0x1000 + t as u64);
        }
        let _ = host.poll(ms);
        let _ = client.poll(ms);
    }
    let hev = host.poll(ms);
    let cev = client.poll(ms);
    assert!(!hev.iter().any(|e| matches!(e, Event::Desync { .. })), "host stayed in sync");
    assert!(!cev.iter().any(|e| matches!(e, Event::Desync { .. })), "client stayed in sync");
}

/// A forged state hash at a shared tick is a desync on BOTH sides.
#[test]
fn forged_hash_desyncs() {
    let (mut host, mut client) = two_peers(27820);
    let mut ms: u64 = 0;
    pump2(&mut host, &mut client, &mut ms,
        |h, c, _, _, _| c.local_slot() != 0 && h.agents().len() == 2, "handshake");
    host.start_match(0).unwrap();
    pump2(&mut host, &mut client, &mut ms,
        |_, c, _, _, _| c.is_started(), "GO");

    // 15 honest ticks, then the host lies about its hash at tick 15 — a
    // hash cadence boundary (hash_every 15: submit_hash only sends there).
    for t in 0..15u32 {
        host.local_input(t, scripted_input(t, 0));
        client.local_input(t, scripted_input(t, 1));
        pump2(&mut host, &mut client, &mut ms,
            |h, c, _, _, _| h.ready_inputs(t).is_some() && c.ready_inputs(t).is_some(), "tick inputs");
    }
    client.submit_hash(15, 0xaaaa_0000_0000_0001);
    host.submit_hash(15, 0xaaaa_0000_0000_0002);
    let mut hev: Vec<Event> = Vec::new();
    let mut cev: Vec<Event> = Vec::new();
    pump2(&mut host, &mut client, &mut ms,
        |_, _, _, he, ce| {
            hev.extend(he.iter().filter(|e| matches!(e, Event::Desync { .. })).cloned());
            cev.extend(ce.iter().filter(|e| matches!(e, Event::Desync { .. })).cloned());
            !hev.is_empty() && !cev.is_empty()
        }, "desync");
    assert!(hev.iter().any(|e| matches!(e, Event::Desync { tick: 15, .. })), "host desyncs: {hev:?}");
    assert!(cev.iter().any(|e| matches!(e, Event::Desync { tick: 15, .. })), "client desyncs: {cev:?}");
    // Sister: equal hashes at the same tick do NOT desync (the 1000-tick run above).
}

/// A mismatched build id is refused cleanly on the raw wire, before any slot is assigned.
#[test]
fn build_mismatch_refused_raw_socket() {
    let raw = std::net::UdpSocket::bind("127.0.0.1:0").expect("fake client binds");
    raw.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
    let mut host = Session::new_host(cfg(), UdpTransport::bind("127.0.0.1:27830").expect("host binds"),
        MatchSettings::default(), 1);
    // A raw HELLO with a wrong build id (nf-net's wire layout: magic, type 1,
    // proto u16, build u32, codename 32B).
    // MAGIC is little-endian on the wire: 0x4E46 -> bytes 46 4E.
    let mut hello = vec![0x46, 0x4E, 1u8, 0];
    hello.extend_from_slice(&nf_net::PROTOCOL_VERSION.to_le_bytes());
    hello.extend_from_slice(&0xdead_c0de_u32.to_le_bytes()); // wrong build
    let mut name = [0u8; 32];
    name[..4].copy_from_slice(b"Alec");
    hello.extend_from_slice(&name);
    raw.send_to(&hello, "127.0.0.1:27830").expect("hello sent");
    // The host replies from inside its poll — pump it while waiting.
    let mut buf = [0u8; 128];
    let mut ms: u64 = 0;
    let mut tries = 0u32;
    let (n, _) = loop {
        ms += 5;
        let _ = host.poll(ms);
        match raw.recv_from(&mut buf) {
            Ok(r) => break r,
            Err(_) => {
                tries += 1;
                std::thread::sleep(Duration::from_millis(2));
                assert!(tries < 500, "a REFUSE arrives");
            }
        }
    };
    assert_eq!(&buf[..2], &[0x46, 0x4E], "nf-net magic (0x4E46 little-endian)");
    assert_eq!(buf[2], 3, "the host refuses (PT_REFUSE), not welcomes");
    assert_eq!(buf[4], 2, "the reason is the build mismatch");
    let _ = n;
}

/// A silent peer is dropped (ClientLeft/Timeout) inside the shrunk timeout;
/// DropPolicy::Neutral keeps the match running (the game's dead-pad shape).
#[test]
fn silent_peer_times_out_and_match_runs_on() {
    // The silence test cannot wait out the 5 s default inside the pump
    // deadline — shrink the timeout (the Config knob the app would tune).
    let (mut host, mut client) = two_peers_timeout(27840, 400);
    let mut ms: u64 = 0;
    pump2(&mut host, &mut client, &mut ms,
        |h, c, _, _, _| c.local_slot() != 0 && h.agents().len() == 2, "handshake");
    host.start_match(0).unwrap();
    pump2(&mut host, &mut client, &mut ms, |_, c, _, _, _| c.is_started(), "GO");

    // 3 honest ticks, then the client goes silent (its session stops polling).
    for t in 0..3u32 {
        host.local_input(t, scripted_input(t, 0));
        client.local_input(t, scripted_input(t, 1));
        pump2(&mut host, &mut client, &mut ms,
            |h, c, _, _, _| h.ready_inputs(t).is_some() && c.ready_inputs(t).is_some(), "tick inputs");
    }
    drop(client);
    let mut left: Option<Event> = None;
    pump_until(&mut host, None, &mut ms, |_, _, _, hev, _| {
        for e in hev {
            if matches!(e, Event::ClientLeft { reason: nf_net::LeaveReason::Timeout, .. }) {
                left = Some(e.clone());
            }
        }
        left.is_some()
    }, "timeout");
    let e = left.expect("the host drops the silent client");
    match e {
        Event::ClientLeft { slot, reason: nf_net::LeaveReason::Timeout } => assert_eq!(slot, 1),
        other => panic!("{other:?}"),
    }
    // The match runs on: the host keeps ticking on NEUTRAL for slot 1.
    for t in 3..10u32 {
        host.local_input(t, scripted_input(t, 0));
        ms += 5;
        let _ = host.poll(ms);
        host.ready_inputs(t).expect("slot 1 is NEUTRAL now — never a stall");
    }
}
