//! UDP lockstep multiplayer session for 007: Nightfire (PS2 NTSC-U) port.
//!
//! This crate owns the *session* layer only: handshake, per-tick input
//! exchange, state-hash exchange, stall and timeout handling. It has no
//! dependency on the game sim (`nf-game`) and no rendering; the host app
//! wires it in (see `docs/systems/net-lockstep.md`):
//!
//! * the match seed (`u32`) received on the client is fed to
//!   `nf_game::drone::Rng::new(seed)` (the game's own RNG, bit-exact,
//!   [M] `Rand_Rand` 0x1e3738);
//! * `MatchSettings` carries the host's `MpSettings` values (field mapping
//!   documented in [`settings`]);
//! * the per-tick state hash is an opaque `u64` the caller reports via
//!   [`Session::submit_hash`] — task 050's `state_hash(&MpWorld)` wires in
//!   there once it lands (it does not exist yet, checked 2026-10-15);
//! * [`Input`] maps to `nf_game::player::Input` + the separate
//!   `trigger/reload/use_key` args of `Sim::step_bond` (mapping documented).
//!
//! Determinism rules: no wall clock anywhere — every [`Session::poll`] takes
//! a logical `now_ms` supplied by the caller (real time in the app, simulated
//! time in tests), and the bit-exact RNG is never touched by this crate.
//!
//! Wire protocol summary (little-endian, see [`proto`]):
//! 4-byte header `4E 46 <ptype> <reserved>`, then per-type payloads.
//! Star topology: clients talk only to the host; the host relays.

pub mod input;
pub mod net;
pub mod proto;
pub mod session;
pub mod settings;
pub mod shim;

pub use input::Input;
pub use net::UdpTransport;
pub use session::{Config, DropPolicy, Event, LeaveReason, RefuseReason, Session, SessionError};
pub use settings::MatchSettings;

/// Maximum agents in an MP match — the game's own limit: humans occupy slots
/// 0..4 (`MpSettings.human_count`, bots 4..8), `MP_Start` 0x183a50 inits
/// humans into slots 0..player_count-1 and bots into 4..4+bot_count-1 [M]
/// (`work/decomp` MP_Start; `crates/nf-game/src/mp/mod.rs:59,664`;
/// `docs/atlas/multiplayer.md` "MP 1-4").
pub const MAX_AGENTS: usize = 4;

/// Wire protocol version. Bumped on any incompatible packet-layout change.
pub const PROTOCOL_VERSION: u16 = 1;

/// Build id carried in the handshake so mismatched executables refuse cleanly.
/// The host app sets this (e.g. a build counter); both sides must match exactly.
pub const DEFAULT_BUILD_ID: u32 = 1;

/// Default codename, nul-padded to 32 bytes on the wire. Both sides must match.
pub const DEFAULT_CODENAME: &str = "nightfire-port";

/// Largest packet we ever build or accept (fits a 1500-byte MTU with headroom).
pub const MAX_PACKET: usize = 1400;

/// Magic u16 at the start of every packet: 'N','F'.
pub const MAGIC: u16 = 0x4E46;
