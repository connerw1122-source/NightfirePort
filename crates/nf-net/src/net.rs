//! The real transport: a non-blocking [`std::net::UdpSocket`] wrapper.
//!
//! This is the only place this crate touches the OS network stack; tests use
//! the in-memory shim in [`crate::shim`] instead. No new crates — std only.

use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use crate::MAX_PACKET;

/// Transport abstraction: send to an address, drain pending datagrams,
/// report the local bind address. Implemented by [`UdpTransport`] (real
/// UDP) and the test shim.
pub trait Transport {
    fn send(&mut self, to: SocketAddr, data: &[u8]);
    /// Non-blocking: returns one pending datagram and its origin, if any.
    fn try_recv(&mut self) -> Option<(SocketAddr, Vec<u8>)>;
    fn local_addr(&self) -> SocketAddr;
}

/// Real UDP transport. The host binds `0.0.0.0:port`; clients bind an
/// ephemeral port (`bind "127.0.0.1:0"` or the interface they need).
pub struct UdpTransport {
    sock: UdpSocket,
    scratch: Vec<u8>,
}

impl UdpTransport {
    /// Bind a UDP socket. Hosts: `"0.0.0.0:7777"`; clients: `"127.0.0.1:0"`.
    pub fn bind(addr: &str) -> std::io::Result<UdpTransport> {
        let sock = UdpSocket::bind(addr)?;
        sock.set_nonblocking(true)?;
        // Read timeout is irrelevant in non-blocking mode but set a sane
        // bound anyway; try_recv never blocks.
        sock.set_read_timeout(Some(Duration::from_millis(1)))?;
        Ok(UdpTransport { sock, scratch: vec![0u8; MAX_PACKET] })
    }
}

impl Transport for UdpTransport {
    fn send(&mut self, to: SocketAddr, data: &[u8]) {
        let _ = self.sock.send_to(data, to);
    }

    fn try_recv(&mut self) -> Option<(SocketAddr, Vec<u8>)> {
        match self.sock.recv_from(&mut self.scratch) {
            Ok((n, from)) => Some((from, self.scratch[..n].to_vec())),
            Err(_) => None,
        }
    }

    fn local_addr(&self) -> SocketAddr {
        self.sock.local_addr().unwrap_or_else(|_| {
            "0.0.0.0:0".parse().expect("static fallback SocketAddr")
        })
    }
}
