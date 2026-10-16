# Net lockstep — the MP match sim as a pure function

Status: **determinism proven for the MP core + bots; the UDP host itself is future work** (`nf-net`,
not built here). This doc is the audit the lockstep design rests on: every nondeterminism source in
the MP tick was checked against the code, the two real ones were fixed, and the sim's purity is now
pinned by tests that hash the whole state every tick. Facts are tagged [M] code (function + address),
[D] disc, [L] EE snapshot, [S] suggested — the repo's evidence rules (`docs/fields/README.md`).

Branch: `s/net-determinism` (2026-10-15). Deliverables: `nf_game::mp::hash` (`state_hash`),
`nf_game::mp::replay` (the recorded-input harness), `crates/nf-game/tests/mp_lockstep.rs` (6 tests).

## 1. The contract

Deterministic lockstep: every peer runs the same match from the same `(settings, seed, spawn/object
registrations)` and exchanges **only per-tick inputs**; a per-tick state hash detects any desync.
Concretely, for the port's MP core:

- **Input** per tick: the human slot positions (`MpMatch::set_slot` — the pure core's stand-in for
  applying an agent's controller), the ordered cross-sim events (kills, bullet hits, respawns,
  touches — `mp::replay::MpEvent`), the KOTH on-hill mask, the paused flag.
- **Order**: positions → events → `update_objects(DT, …)` → `update(DT, …)` — the game's
  `MP_ObjectUpdate` then `MP_Update` on one frame [M `MP_Update` 0x184bd0; the established test
  driver `mp_match_rules.rs::tick`].
- **Clock**: the fixed 60 Hz tick, `DT = 1.0/60.0` as a literal. The game samples
  `psiGetTimeIn100ths`, a wall clock [M `MP_CheckForEndCondition` 0x184268:35-55]; the port's
  *Deviation* (the mp module doc) folds the constant instead — required for lockstep: every peer
  folds the identical constant in the identical order, which is what keeps the f32 timer
  accumulators (`clock`, `elapsed`, the uplink drips `score += dt*0.2` [M rules.rs]) bit-stable.
- **RNG**: ONE stream per match — the game's own generator (`Rand_Rand` 0x1e3738 / `Rand_FRand`
  0x1e3780; the port's bit-exact `Rng` in `drone.rs:383`, seeded `Rng::new(seed)`). Every draw in
  the tick — spawn picks, assassin picks, GE/blueprint respawns, bot goal/burst rolls — comes off
  that one stream in call order. No other entropy exists anywhere in `nf-game/src` (grep: no
  `rand` crate, no `getrandom`, no `SystemTime`, no `Instant` outside `prof.rs`, which never
  feeds the sim [M]). The test `one_extra_rng_draw_diverges` proves an unaccounted draw desyncs.

## 2. The audit (what was checked, what was found)

Swept `crates/nf-game/src` for: wall-clock/time reads, HashMap/HashSet iteration into state, sorts
without total order, uninitialised memory, thread/rayon/channel use, "pick any" over unordered data,
libm-dependent float paths. Results:

| # | Finding | Verdict |
|---|---------|---------|
| F1 | `bots.rs` aimed with **std** `atan2`/`cos`/`sin` (libm-dependent — ulp differences across platforms flip the `dot <= 0.9998` aim-cone gate [M 0x178c48]) | **FIXED**: now the game's own `ATAN2_APPROX` 0x1e4528 and `PS2Sinf` 0x120700 / `PS2Cosf` (`crate::trig`), bit-exact to the game [M; validated in trig.rs:336-359, diff_calls.rs]. Same convention (tangent dz/dx). Bots 9/9, mp_bots_live 2/2, mp_match_rules 14/14, lockstep 6/6 after the swap. |
| F2 | `.sqrt()`/`.powi(2)` in bot movement/aim | Safe — IEEE-754 exact ops, no libm. Rust does no fast-math/FMA contraction; no `mul_add` in the MP path [M]. |
| F3 | f32 accumulation (`clock`, `elapsed`, uplink drips, bot `next_bullet_at`) | Deterministic given the fixed `DT` fold order — pinned by the 10 000-tick identical-hash-every-tick test (any reassociation shows). |
| F4 | Two bot clocks: the driver-supplied `ctx.now` (accumulated 1/60) vs the brain's own `tick/60` (which `next_bullet_at` is stored on [M bots.rs]) — a host deriving `now` differently moved sim state | **FIXED**: the brain's canonical clock is its own tick counter; `ctx.now` is kept for API compat and **ignored** (documented on the field). |
| F5 | Collections: no HashMap/HashSet anywhere in `mp/`; spawn candidates are a `[u16; 64]` index scan (mod.rs:480), rules/bots use argmin/argmax with strict keep-first ties | Clean [M]. The HashMaps elsewhere in nf-game are key-lookup or load-time built, never iterated into sim state. |
| F6 | Sorts: none in `mp/`; elsewhere `total_cmp` or stable sorts on deterministic keys | Clean [M]. |
| F7 | Uninitialised memory: none (`unsafe`/`transmute`/`MaybeUninit` zero hits in the MP path). The game's own uninit-buffer read (spawn policy 2 with no candidates) is a documented *Deviation* — the port returns the range start (deterministic) [M mod.rs:469] | Clean. |
| F8 | Threads: none in `nf-game` (single-threaded tick; the only lock is `CollisionWorld.dynamic`'s RwLock, uncontended) | Clean [M]. |
| F9 | nf-view host: tick counts per render frame come from wall-clock dt with a dropped-ticks cap (`ticks_for_frame`, NF_MAXTICKS) | Not a sim-state risk, but **the future nf-net host must take tick counts from the network, never from frame pacing**. |

## 3. `state_hash(&MpMatch) -> u64` (`nf_game::mp::hash`)

FNV-1a 64 over a canonical field walk — settings → spawn globals → `MPGame` → stored MP objects →
scenario objects → the assassin words. Rules (module doc carries the same list):

- integers as little-endian bytes; **f32 as `to_bits()`** — never formatting, never `PartialEq`
  (−0.0 vs 0.0 and NaN payloads are distinct states; pinned by unit tests);
- `Option<T>` = 0/1 tag + contents; `Vec<T>` = `u32` len + elements;
- implemented here, **not** `std::hash::DefaultHasher` — SipHash's cross-version stability is
  explicitly not guaranteed, and peers on different builds must agree bit-for-bit;
- the FNV-1a reference vectors are pinned by `mp::hash::tests::fnv1a_reference_vectors`.

`BotBrain::state_hash() -> u64` covers the brains (they are sim state): every live field of
`BotVars` + the brain's own fields, f32 by bit pattern. The derived logs (`shots`,
`visited_pickups`, `trace`) hash as **lengths only** — they are pure functions of live state + the
shared stream, so a content divergence implies a hashed divergence, and the per-tick hash stays
O(1). **When you add a field to `MpMatch` or `BotBrain`, add it to the hash** — an outhashed field
is a desync the hash cannot see.

## 4. The replay harness (`nf_game::mp::replay`)

`MpRecord` = the complete transcript of a match: `seed` + `settings` + the level-derived
registrations (`spawn_registrations`/`object_registrations` — identical on every peer from the
shared ISO, carried so a headless replay is self-contained) + `slot_spawns` (the slot list — the
spawn **indices are derived**, re-picked through `get_spawn_point`'s shared-stream draw [M
0x185098], not recorded) + per-tick `MpTickInput { on_hill, paused, set_slots, events }`.

`run_record(&MpRecord)` drives the exact live call order on one `Rng::new(seed)` stream and returns
the per-tick `state_hash`. `apply_event` is the single event→method mapping used by live play and
replay alike, so they cannot drift. Scope note: **the record covers the MP core** — bots-off
matches are complete transcripts; in a bots-on match the brains are shared-sim (draw from the same
stream), so every peer drives them identically and they are not inputs (the two-sim soak proves
that path; an `nf-net` host wires `BotBrain::step` into its tick the same way).

## 5. The tests (`crates/nf-game/tests/mp_lockstep.rs`, `--features mp-bots`)

Run: `cargo test --offline --release -p nf-game --features mp-bots --test mp_lockstep` — headless,
no ISO, ~1.3 s. Every assertion has a sister that would fail if the property were absent:

1. `two_sims_same_inputs_hash_identically_every_tick` — two INDEPENDENT sims (own `Rng`, own
   world, own brains) fed the same scripted inputs for **10 000 ticks, bots on, all 12 scenario
   words**: combined match+brains hash equal every tick (and the run is not vacuous: kills scored,
   a bot fired, the clock folded > 100 s).
2. `one_extra_rng_draw_diverges` (sister) — one unaccounted draw desyncs: the hash sees the stream.
3. `different_seed_diverges` (sister) — the per-match seeding matters.
4. `hash_catches_a_one_bit_state_difference` — flipping ONE bit of an f32 (score), an integer
   (kills), a settings word (radar), or a bot var flips the hash; restoring restores.
5. `replay_matches_the_recording_tick_for_tick` — a live run and `run_record` over the transcript
   hash identically every tick (4 000 ticks, TDM).
6. `replay_catches_a_dropped_input` (sister) — one dropped kill diverges from that tick and stays.

## 6. What the future `nf-net` host owes this (not built here)

- `std::net::UdpSocket`, direct connect (host listens; clients join IP:port; port-forward note for
  internet play); no relay. Keyboard+mouse only. Loopback tests.
- Host loop per tick: gather peers' inputs for tick N (or wait — lockstep stall) → apply local
  input + remote inputs as the `MpTickInput` fields → step the shared sim (core events via
  `apply_event`; brains via `BotBrain::step` on the shared stream) → exchange
  `state_hash(&m) ^ brains` → mismatch = desync alarm (drop or resync-from-record).
- Tick counts come from the network, never from `ticks_for_frame` (F9).
- The port's live MP loop currently ticks only `m.update` (nf-view scene.rs:1824) — wiring
  `update_objects`/events/bots into the host tick is the pre-netcode step (their absence is why the
  current loop is trivially deterministic).

## 7. Live checks for the main agent

Nothing is required to merge this — the tests are headless and the findings are code-level. If you
want on-hardware confirmation of F1's premise (the PS2 game's own trig), a `gdbwatch.py` watch on
the bot aim during a bots-on MP match (the `ATAN2_APPROX` call site 0x1e4528) recording a few
(dx, dz) → yaw samples would let us assert the port's `atan2_approx` against real bot turns, the
way `diff_calls.rs` does for `PS2Sinf`.
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
