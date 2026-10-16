//! Test-only in-memory network shim: deterministic loss, latency, reorder.
//!
//! Used by the loopback tests (host + 3 clients, 2000 ticks with loss) in
//! `tests/lockstep.rs`. No wall clock: the shim runs on the same logical
//! millisecond timeline the sessions are polled with.
//!
//! Model: datagrams sent through a [`ShimEndpoint`] are scheduled with a
//! deterministic RNG (SplitMix64): a drop dice at send time, then a delivery
//! time `now + latency + jitter`. [`Shim::advance`] moves due datagrams into
//! the receiving endpoint's FIFO. Same seed + same call order = same run, so
//! failures reproduce exactly.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::rc::Rc;

use crate::net::Transport;

struct InFlight {
    from: SocketAddr,
    to: SocketAddr,
    deliver_at: u64,
    data: Vec<u8>,
}

struct Core {
    now_ms: u64,
    rng_state: u64,
    drop_per_mille: u32,
    latency_ms: u64,
    jitter_ms: u64,
    queues: HashMap<SocketAddr, VecDeque<(SocketAddr, Vec<u8>)>>,
    in_flight: Vec<InFlight>,
}

impl Core {
    fn next_rand(&mut self) -> u64 {
        self.rng_state = self.rng_state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Deterministic lossy network shared by all endpoints of one test.
#[derive(Clone)]
pub struct Shim {
    core: Rc<RefCell<Core>>,
}

impl Shim {
    pub fn new(seed: u64) -> Shim {
        Shim {
            core: Rc::new(RefCell::new(Core {
                now_ms: 0,
                rng_state: seed.wrapping_add(0x1234_5678_9ABC_DEF0),
                drop_per_mille: 0,
                latency_ms: 0,
                jitter_ms: 0,
                queues: HashMap::new(),
                in_flight: Vec::new(),
            })),
        }
    }

    /// Register a new endpoint with a fixed fake address.
    pub fn endpoint(&self, addr: SocketAddr) -> ShimEndpoint {
        self.core.borrow_mut().queues.entry(addr).or_default();
        ShimEndpoint { addr, core: Rc::clone(&self.core) }
    }

    /// Advance the clock and deliver everything whose time has come.
    pub fn advance(&self, dt_ms: u64) {
        let mut core = self.core.borrow_mut();
        core.now_ms += dt_ms;
        let now = core.now_ms;
        let mut i = 0;
        while i < core.in_flight.len() {
            if core.in_flight[i].deliver_at <= now {
                let pkt = core.in_flight.swap_remove(i);
                if let Some(q) = core.queues.get_mut(&pkt.to) {
                    q.push_back((pkt.from, pkt.data));
                }
            } else {
                i += 1;
            }
        }
    }

    /// Advance in 1 ms steps until no datagram is in flight or `cap_ms`
    /// logical ms have passed (bounded loops in tests).
    pub fn advance_until_quiet(&self, cap_ms: u64) {
        let mut stepped = 0u64;
        while !self.core.borrow().in_flight.is_empty() && stepped < cap_ms {
            self.advance(1);
            stepped += 1;
        }
    }

    /// Drop every datagram to/from an endpoint (hard link cut, in-flight
    /// included) — used for the timeout test.
    pub fn cut(&self, endpoint: SocketAddr) {
        let mut core = self.core.borrow_mut();
        core.in_flight.retain(|p| p.to != endpoint && p.from != endpoint);
        if let Some(q) = core.queues.get_mut(&endpoint) {
            q.clear();
        }
    }

    /// Simulate a one-way packet drop right now (for targeted loss tests).
    pub fn drop_pending_to(&self, endpoint: SocketAddr, n: usize) {
        let mut core = self.core.borrow_mut();
        let mut dropped = 0;
        core.in_flight.retain(|p| {
            if p.to == endpoint && dropped < n {
                dropped += 1;
                false
            } else {
                true
            }
        });
    }

    pub fn now_ms(&self) -> u64 {
        self.core.borrow().now_ms
    }

    pub fn in_flight(&self) -> usize {
        self.core.borrow().in_flight.len()
    }

    /// Set datagram drop probability in 1/1000.
    pub fn set_loss(&self, per_mille: u32) {
        self.core.borrow_mut().drop_per_mille = per_mille;
    }

    /// Set one-way latency and jitter (both ms).
    pub fn set_latency(&self, latency_ms: u64, jitter_ms: u64) {
        let mut c = self.core.borrow_mut();
        c.latency_ms = latency_ms;
        c.jitter_ms = jitter_ms;
    }
}

/// A transport handle bound to one shim endpoint; give one per session.
#[derive(Clone)]
pub struct ShimEndpoint {
    addr: SocketAddr,
    core: Rc<RefCell<Core>>,
}

impl Transport for ShimEndpoint {
    fn send(&mut self, to: SocketAddr, data: &[u8]) {
        let mut core = self.core.borrow_mut();
        if core.drop_per_mille > 0 && (core.next_rand() % 1000) < core.drop_per_mille as u64 {
            return; // lost at send time
        }
        let jitter = if core.jitter_ms > 0 {
            (core.next_rand() % (core.jitter_ms + 1)) as u64
        } else {
            0
        };
        let deliver_at = core.now_ms + core.latency_ms + jitter;
        core.in_flight.push(InFlight {
            from: self.addr,
            to,
            deliver_at,
            data: data.to_vec(),
        });
    }

    fn try_recv(&mut self) -> Option<(SocketAddr, Vec<u8>)> {
        let mut core = self.core.borrow_mut();
        core.queues.get_mut(&self.addr).and_then(|q| q.pop_front())
    }

    fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}
