# UDP lockstep session (`crates/nf-net`)

The MP session layer for online play: a deterministic-lockstep session over
UDP where every machine runs the same match from the same seed and settings
and exchanges only per-tick inputs; a per-tick state hash detects desyncs.
This is a port feature — the PS2 game has no network mode — so wire choices
here are [S] unless they carry a game fact. Task 050 (determinism /
`state_hash(&MpWorld)`) has NOT landed; this session takes the per-tick hash
as an opaque `u64` from the caller (`Session::submit_hash`) and 050's
`state_hash(&MpWorld) -> u64` wires in there. [S] branch state 2026-10-15:
`origin/s/net-determinism` contains only a WORKLOG entry, and no
`state_hash` exists anywhere in `crates/`.

## Why a separate crate

`crates/nf-net` has zero dependencies (std only, `std::net::UdpSocket`) and
no rendering, so its loopback tests build and run in milliseconds and the
host app can drive a match without the game sim. `nf_game::net` would have
made every session test pay the full `nf-game` build.

## API

```rust
Session::new_host(cfg, transport, settings: MatchSettings, seed: u32)
Session::new_client(cfg, transport, host: SocketAddr)
session.poll(now_ms) -> Vec<Event>          // pump + timers; now_ms is LOGICAL
session.local_input(tick, input)            // app samples controls; ignored pre-slot
session.ready_inputs(tick) -> Option<[Input; 4]>   // run the sim tick only on Some
session.submit_hash(tick, hash: u64)        // 050 wires in here
session.start_match(first_tick)             // host only: sends GO
session.leave()
```

`poll(now_ms)` takes the time from the caller on purpose: the determinism
rule forbids wall clocks in the sim, so tests drive logical milliseconds and
the app passes real ones. `ready_inputs` must be called sequentially
(start_tick, +1, ...) — that sequence produces `Event::Stalled` /
`Event::Resumed`. The app samples local controls once per 60 Hz frame and
schedules them `input_delay` ticks ahead (config, default 3, task range
2..=4); while stalled it keeps sampling — inputs for future ticks are
sampled from present controls, never invented.

## Topology and reliability

Star topology: clients talk only to the host; the host relays every agent's
inputs to each client (an INPUT packet to one client carries all slots but
that client's own). All packets fit a 1500-byte MTU (`MAX_PACKET` 1400).

* **Input delay**: tunable `Config::input_delay` (2..=4 recommended), sent in
  WELCOME so both sides agree.
* **Redundancy**: every INPUT packet carries the full unacked window; when it
  fits the packet cap, the last `Config::redundancy` (default 4) already
  acked inputs ride along too. When the unacked span overflows the cap
  (32 ticks client→host, 8 ticks host→client) the window is sent
  OLDEST-FIRST with the END truncated — the peer's ack can always advance,
  so no tick is ever silently skipped (a wedge where old ticks were never
  re-sent was found and fixed by this rule; see tests).
* **Acks**: each INPUT carries `ack_tick` (highest contiguous tick received
  from that peer) + a 32-tick bitmap. The receiver's ack scan starts from
  its own sliding `our_ack`, NOT from the start tick — pruning old received
  ticks must not be able to reset an ack to zero (second real bug found by
  the 2000-tick test).
* **Stall**: a missing HUMAN input stops the sim (`ready_inputs == None`,
  `Event::Stalled { tick, missing }`). The session never fabricates an input
  to cover a missing packet.
* **Hash exchange**: every `hash_every` ticks (default 30), plus the previous
  hash piggybacked. Mismatch at a tick both sides ran → `Event::Desync`;
  the session keeps running (the app decides). A hash packet's
  `prev_tick == tick` marks "no previous hash yet" and the receiver skips it.

## Handshake

1. Client → `HELLO { protocol_version, build_id, codename }` (retransmitted
   every `hello_retry_ms` until answered).
2. Host validates each field; mismatch → `REFUSE { Version | Build |
   Codename }`, full (already 3 clients) → `REFUSE { Full }`. Mismatched
   executables therefore refuse cleanly before any match state is shared.
3. Host → `WELCOME { proto, build, codename, slot, input_delay, hash_every,
   seed: u32, settings: MatchSettings }`. The client takes slot 1..=3 (host
   is slot 0; 4 agents = the game's MP limit, humans 0..4 / bots 4..8 per
   `MP_Start` 0x183a50 [M], `docs/atlas/multiplayer.md`).
4. Host `start_match()` → `GO { first_tick }` (retransmitted until the
   client's first INPUT proves it started). Clients emit `MatchStarted`.
5. `SLOTMAP { human_slots bitmask }` broadcasts join/drop changes.

`seed` is the u32 the app feeds to the game's own `Rng::new(seed)`
(`crates/nf-game/src/drone.rs:394`, `Rand_Rand` 0x1e3738 bit-exact [M]) so
every machine seeds the shared stream identically. `MatchSettings`
(`settings.rs`, 176-byte fixed layout) carries the `MpSettings` values the
sim reads — field-by-field mapping to the game's hex offsets is documented
in that module (no wire encoding existed before; the only prior save code is
`LS_MakeMPSettings` 0x1ce508's 13-byte `MPSG` profile block, which carries
unrelated fields) [M]+[S].

## Disconnects: NEUTRAL continuation (the game's own behaviour)

On a client timeout (`Config::timeout_ms`, no packets) or graceful LEAVE the
host marks the slot non-human, relays `LEAVE` + `SLOTMAP`, and — per the
default `DropPolicy::Neutral` — feeds that agent the game's disconnected-pad
input every tick: `NEUTRAL` = buttons 0, sticks centred. That is exactly
what the PS2 engine does for a dead pad: `psiInput_PollDevices` 0x10ca78
writes buttons 0 (+0x120/+0x122) and analog 0x7f (+0x128..0x12b) for any pad
not in state 99, the match runs on, and the agent keeps living/dying under
the normal rules (`Player_HandleDeath` 0x190bb0 → `MP_ReSpawn` 0x185368) [M].

Rejected alternatives, with reasons:
* **Bot takeover** has no game precedent mid-match: bots are fixed into
  slots 4..8 at `MP_Start` 0x183a50 and configured only pre-match
  (`Menu_PrepareBots` 0x1f94a0, front-end only); nothing mutates the roster
  during play. `DropPolicy::Neutral` is therefore closer to the game than a
  bot-fill, and a reconnect resumes real input exactly like the pad
  state machine re-initialising on replug (`ps2_handle_pad` 0x10c738) [M].
* **Match end** on the console is only the explicit pause-menu QUIT
  (`C_GCPAUSE_Handler` 0x204408 → debriefing 0x40000033), a user action —
  not something the game infers from absence. It exists as
  `DropPolicy::EndMatch` for host apps that want it.

Timeouts are reported, never guessed around: the host emits
`Event::ClientLeft { slot, reason }`, a client that stops hearing from the
host emits `Event::HostTimeout`.

## Wire format

4-byte header `4E 46 <type> <rsv>`, little-endian payloads, full table in
`proto.rs` (HELLO/WELCOME/REFUSE/INPUT/HASH/KEEPALIVE/LEAVE/SLOTMAP/GO).
`Input` is 24 bytes: the four `f32`s (`forward/strafe/turn/look`, matching
`nf_game::player::Input` player.rs:96-125) travel as raw bit patterns for
bit-exactness, then two flag bytes (jump/crouch/aiming/zoom/scope/fire/
mode_switch/trigger, reload/use_key), `weapon_cycle`/`gadget_cycle` i8, and
`switch_to` (0xFF = None). The sim's separate `trigger/reload/use_key` args
of `Sim::step_bond` (scene.rs:1818) are folded into the flag bytes [M].

## Tests (crates/nf-net, all loopback, no second machine)

`tests/lockstep.rs` drives a deterministic shim network (seeded SplitMix64
loss/latency/jitter, logical ms — `shim.rs`):

* `handshake_delivers_settings_seed_and_go` — settings + seed reach clients;
  SLOTMAP; GO at tick 0.
* `version_and_build_mismatch_refused` — wrong proto / build → `Refused`,
  host never joins them.
* `four_agents_2000_ticks_lossy_stay_in_sync` — host + 3 clients, 5% drop +
  30 ms latency + 20 ms jitter, 2000 ticks, identical hash chains and no
  desync.
* `one_flipped_input_byte_is_caught_as_desync` — sister test: one flipped
  bit in one agent's input at tick 900 must surface as `Desync` at/after
  that tick.
* `stall_never_guesses_then_resumes` — dead link: sim freezes (no tick may
  run while an input is missing), `Resumed` on recovery, catch-up.
* `client_timeout_reported_then_neutral_continuation` — dead client →
  `ClientLeft { Timeout }`, match continues on NEUTRAL (the game's
  pad-disconnect shape).
* `endmatch_policy_ends_match_instead` — `DropPolicy::EndMatch` →
  `MatchEnded`.
* `real_udp_loopback_sync` — real `std::net::UdpSocket`s on 127.0.0.1, 300
  ticks, hashes agree.

## Live checks for the main agent

None needed — the PS2 game has no network mode, so there is nothing to
observe in PCSX2. The game-side facts this doc cites (pad-disconnect shape,
bot slot rules, quit path) were verified from the decompile and can be
spot-checked there if desired.
