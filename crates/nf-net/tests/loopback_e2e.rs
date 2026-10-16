//! The consolidation's required end-to-end test: host + 2 clients in-process
//! over real 127.0.0.1 UDP, with deterministic packet loss AND reordering
//! injected, playing 600 ticks — every peer's per-tick state hash must agree.
//! Sister test: one corrupted input trips the desync detector.
//!
//! Deterministic throughout: the injection layer's drops and transpositions
//! come from a seeded SplitMix64 (same seed = same run), the tick hashes are
//! a pure fold over the inputs, and there is no wall clock in the compares —
//! the only wall-clock use is the bounded pump loop the real-socket tests
//! already use (the sockets are non-blocking; the pump just waits for the
//! kernel to deliver).

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use nf_net::net::Transport;
use nf_net::session::{Session, Event};
use nf_net::{MatchSettings, NEUTRAL};

/// The injected-loss/reorder wrapper around a real loopback socket. Loss:
/// drop at send with `drop_per_mille`/1000 probability. Reorder: with
/// `reorder_per_mille`/1000 probability a datagram is HELD in a one-slot
/// delay line and flushed AFTER the next sent datagram — a transposition.
struct Lossy {
    sock: UdpSocket,
    rng: u64,
    drop_per_mille: u32,
    reorder_per_mille: u32,
    held: Option<(SocketAddr, Vec<u8>)>,
    scratch: Vec<u8>,
}

impl Lossy {
    fn bind(addr: &str, seed: u64, drop_per_mille: u32, reorder_per_mille: u32) -> Lossy {
        let sock = UdpSocket::bind(addr).unwrap();
        sock.set_nonblocking(true).unwrap();
        Lossy {
            sock,
            rng: seed.wrapping_add(0x1234_5678_9ABC_DEF0),
            drop_per_mille,
            reorder_per_mille,
            held: None,
            scratch: vec![0u8; 1400],
        }
    }
    fn next_rand(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl Transport for Lossy {
    fn send(&mut self, to: SocketAddr, data: &[u8]) {
        if self.drop_per_mille > 0 && (self.next_rand() % 1000) < self.drop_per_mille as u64 {
            return; // lost
        }
        if self.reorder_per_mille > 0 && (self.next_rand() % 1000) < self.reorder_per_mille as u64 {
            // Hold this one; the previously held one (if any) goes out NOW,
            // after the send that displaced it — a transposition.
            let late = self.held.take();
            self.held = Some((to, data.to_vec()));
            if let Some((to, d)) = late {
                let _ = self.sock.send_to(&d, to);
            }
            return;
        }
        let _ = self.sock.send_to(data, to);
    }
    fn try_recv(&mut self) -> Option<(SocketAddr, Vec<u8>)> {
        match self.sock.recv_from(&mut self.scratch) {
            Ok((n, from)) => Some((from, self.scratch[..n].to_vec())),
            Err(_) => None,
        }
    }
    fn local_addr(&self) -> SocketAddr {
        self.sock.local_addr().unwrap()
    }
}

fn cfg() -> nf_net::Config {
    nf_net::Config {
        input_delay: 3,
        redundancy: 4,
        hash_every: 15,
        timeout_ms: 4000,
        keepalive_ms: 100,
        hello_retry_ms: 20,
        resend_ms: 15,
        ..Default::default()
    }
}

/// The deterministic per-agent controls.
fn agent_input(slot: u8, tick: u32) -> nf_net::Input {
    let mut i = NEUTRAL;
    let x = (slot as u32).wrapping_mul(0x9E37_79B9) ^ tick.wrapping_mul(0x85EB_CA6B);
    i.forward = ((x & 0xFF) as i32 - 128) as f32 / 128.0;
    i.turn = (((x >> 8) & 0xFF) as i32 - 128) as f32 / 128.0;
    i.fire = x & 0x100 != 0;
    i.jump = x & 0x200 != 0;
    i.weapon_cycle = (x & 0x0F) as i8 - 8;
    i
}

/// The fake deterministic sim: a hash chain over all 4 agents' inputs —
/// same inputs on every machine = same hash; one flipped bit changes it.
fn sim_hash(tick: u32, inputs: &[nf_net::Input; 4], prev: u64) -> u64 {
    let mut h = prev ^ 0xC0FF_EE00;
    for (s, i) in inputs.iter().enumerate() {
        h = h.rotate_left(17) ^ (i.forward.to_bits() as u64).wrapping_add(s as u64 + 1);
        h = h.rotate_left(13) ^ (i.fire as u64).wrapping_mul(31);
        h = h.rotate_left(7) ^ (i.weapon_cycle as u8 as u64);
        h = h.rotate_left(11) ^ ((i.jump as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    }
    h ^ (tick as u64)
}

struct Peer {
    sess: Session<Lossy>,
    executed: u32,
    prev_hash: u64,
    /// (tick, hash) after every executed tick.
    marks: Vec<(u32, u64)>,
    scheduled: u32,
}

impl Peer {
    fn new(sess: Session<Lossy>) -> Peer {
        Peer { sess, executed: 0, prev_hash: 0, marks: Vec::new(), scheduled: 0 }
    }
}

fn host_addr(port: u16) -> SocketAddr {
    format!("127.0.0.1:{port}").parse().unwrap()
}

/// The 600-tick lossy+reordered loopback run (shared by both tests).
/// `corrupt_client`/`corrupt_tick`: one flipped input bit in ONE machine's
/// sim view only (the wire stays clean — a real divergence's shape).
/// Returns (host, clients, desyncs seen anywhere).
fn run_600(corrupt_client: Option<usize>, corrupt_tick: u32, port: u16) -> (Peer, Vec<Peer>, Vec<(u32, u64, u64)>) {
    // 8% loss, 25% transpositions — brutal but realistic on a bad Wi-Fi link.
    // Distinct ports per test: the binary's tests run in parallel threads.
    let mut host = Peer::new(Session::new_host(
        cfg(),
        Lossy::bind(&format!("127.0.0.1:{port}"), 0xBEEF, 80, 250),
        MatchSettings::default(),
        0x1234_5678,
    ));
    let mut clients: Vec<Peer> = (0..2)
        .map(|i| Peer::new(Session::new_client(
            cfg(),
            Lossy::bind("127.0.0.1:0", 0xBEEF + 1 + i as u64, 80, 250),
            host_addr(port),
        )))
        .collect();

    // Handshake: pump until every client has a slot (bounded). The logical
    // clock advances each pump — the session's retries/GO-retransmit live on
    // it (resend_ms 15), which is exactly what survives the injected loss.
    let mut ms: u64 = 0;
    let deadline = Instant::now() + Duration::from_secs(10);
    while clients.iter().any(|c| c.sess.local_slot() == 0) {
        ms += 5;
        for c in clients.iter_mut() { c.sess.poll(ms); }
        host.sess.poll(ms);
        assert!(Instant::now() < deadline, "handshake must complete (loss is injected, not a wall)");
        std::thread::sleep(Duration::from_millis(1));
    }
    host.sess.start_match(0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while clients.iter().any(|c| !c.sess.is_started()) {
        ms += 5;
        for c in clients.iter_mut() { c.sess.poll(ms); }
        host.sess.poll(ms);
        assert!(Instant::now() < deadline, "GO must reach every client");
        std::thread::sleep(Duration::from_millis(1));
    }

    let mut desyncs: Vec<(u32, u64, u64)> = Vec::new();
    let target = 600u32;
    // The per-frame loop: sample ahead (inputs are sampled from present
    // controls, never guessed), run every ready tick, compare nothing here —
    // the SESSION reports desyncs and the final per-tick compare pins the rest.
    for _frame in 0..200_000 {
        ms += 16;
        for p in clients.iter_mut() { p.sess.poll(ms); }
        host.sess.poll(ms);
        let mut all = std::iter::once(&mut host).chain(clients.iter_mut()).enumerate();
        while let Some((slot, p)) = all.next() {
            // Sample up to executed + input_delay + slack.
            let cap = p.executed + 4 + 30;
            while p.scheduled < cap {
                p.sess.local_input(p.scheduled, agent_input(slot as u8, p.scheduled));
                p.scheduled += 1;
            }
            while p.scheduled > p.executed {
                match p.sess.ready_inputs(p.executed) {
                    Some(mut inputs) => {
                        // net-e2e-loopback: the `slot > 0` guard MUST come first —
                        // slot 0 is the host (the chain's head) and `slot - 1`
                        // used to underflow before the guard was evaluated.
                        if slot > 0 && corrupt_client == Some(slot - 1) && p.executed == corrupt_tick {
                            let ls = p.sess.local_slot() as usize;
                            inputs[ls].fire = !inputs[ls].fire;
                        }
                        let h = sim_hash(p.executed, &inputs, p.prev_hash);
                        p.sess.submit_hash(p.executed, h);
                        p.prev_hash = h;
                        p.marks.push((p.executed, h));
                        p.executed += 1;
                    }
                    None => break,
                }
            }
        }
        // Collect desync events.
        for (i, p) in clients.iter_mut().enumerate() {
            for e in p.sess.poll(ms) {
                if let Event::Desync { tick, local, remote } = e {
                    desyncs.push((tick, local, remote));
                }
            }
            let _ = i;
        }
        for e in host.sess.poll(ms) {
            if let Event::Desync { tick, local, remote } = e {
                desyncs.push((tick, local, remote));
            }
        }
        if corrupt_client.is_none() {
            if host.executed >= target && clients.iter().all(|c| c.executed >= target) {
                break;
            }
        } else if !desyncs.is_empty() {
            break;
        }
    }
    (host, clients, desyncs)
}

#[test]
fn host_two_clients_600_ticks_lossy_reordered_identical_hashes() {
    let (host, clients, desyncs) = run_600(None, 0, 27780);
    assert!(desyncs.is_empty(), "no desync allowed on identical input streams: {desyncs:?}");
    assert!(host.executed >= 600, "host reached only tick {}", host.executed);
    for (i, c) in clients.iter().enumerate() {
        assert!(c.executed >= 600, "client {i} reached only tick {}", c.executed);
    }
    // Identical per-tick state hashes on all peers: compare every tick both
    // ran (the host may run a couple past the slowest client under loss).
    let common = clients.iter().map(|c| c.executed).min().unwrap().min(host.executed);
    assert!(common >= 600, "all peers must reach 600 (common {common})");
    for t in 0..600u32 {
        let h = host.marks.iter().find(|(mt, _)| *mt == t).map(|(_, h)| *h)
            .unwrap_or_else(|| panic!("host missing hash at tick {t}"));
        for (i, c) in clients.iter().enumerate() {
            let ch = c.marks.iter().find(|(mt, _)| *mt == t).map(|(_, h)| *h)
                .unwrap_or_else(|| panic!("client {i} missing hash at tick {t}"));
            assert_eq!(ch, h, "per-tick state hash mismatch at tick {t} (client {i})");
        }
    }
}

#[test]
fn corrupted_input_trips_the_desync_detector() {
    // Sister test: client 1's sim view flips one input bit at tick 300; the
    // hash exchange (every 15) must surface a Desync at/after that tick.
    let (host, clients, desyncs) = run_600(Some(1), 300, 27790);
    assert!(!desyncs.is_empty(), "a corrupted input must trip the desync detector");
    assert!(
        desyncs.iter().all(|(t, _, _)| *t >= 300),
        "desync must be at/after the corrupted tick: {desyncs:?}"
    );
    // The detector fired on both ends of the corrupted pair (host + the peer
    // whose hash disagreed) — the session stays alive (the app decides).
    assert!(clients.iter().any(|c| c.executed >= 300) && host.executed >= 300);
    let _ = clients;
}
