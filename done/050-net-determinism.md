## Your task: `net-determinism`, prove the MP sim is lockstep-safe

Before any netcode: lockstep needs the MP match simulation (nf-game's mp code — 400 mp-core, 401/s mp-bots, 402
match rules, 611 menu flow's MpSettings) to be a pure function of (settings, seed, per-tick inputs). Audit and fix:
every source of nondeterminism in the MP tick — host time / wall clock reads, HashMap iteration order, uninitialised
or address-dependent ordering, thread timing, float paths that differ by build — and the RNG seeding (one seeded
stream per match, the game's own `Rand_*`). Deliver `state_hash(&MpWorld) -> u64` (stable across runs and machines:
hash integers and the f32 bit patterns of the sim state, not floats via formatting), a recorded-input replay harness,
and tests: two independent sims fed the same inputs for 10 000 ticks (bots on, every scenario) hash identically every
tick; a sister test proves the hash catches a one-bit state difference. Document in docs/systems/net-lockstep.md.

**Context — online multiplayer (the user, 2026-10-15: "I want online support where we can host a mp server and other
clients can join").** The PS2 game has no online mode; this is a port feature. The plan (main agent): **deterministic
lockstep over UDP** — every machine runs the same MP match from the same seed and settings and exchanges only each
agent's per-tick input; a per-tick state hash detects desyncs. Direct connect: the host listens on a UDP port, clients
join by IP:port (internet play needs the host to forward the port; no relay/server infrastructure). **No new crate
downloads** — use `std::net::UdpSocket` (ask in the report if something truly needs a crate; don't add one). Keep the
networking in a new `crates/nf-net` crate (or `nf_game::net` if a crate is overkill — say which and why), with no
rendering. Keyboard + mouse only (no pads). Tests run on loopback (127.0.0.1) and must not need a second machine.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/net-determinism`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-15T22:38Z

---
## Report (done, 2026-10-15T23:47Z)


---
## Report from the worker (done, 2026-10-15T23:47Z)

# net-determinism — the MP sim is lockstep-safe (branch `s/net-determinism`)

## What's done

**The audit.** Every nondeterminism source in the MP tick was checked in code: wall-clock/time
reads, HashMap/HashSet iteration into state, sorts without total order, uninitialised memory,
threads/channels, "pick any" over unordered data, and libm-dependent float paths. Full table with
the [M] citations in `docs/systems/net-lockstep.md` (F1–F9). The core verdict: the MP match core
(`mp/mod.rs`, `mp/rules.rs`) was already a clean pure function of (settings, registrations, rng,
dt, events) — fixed `DT = 1.0/60.0` literal, no clock reads, no unordered iteration, no threads,
one seeded RNG stream. Two REAL finds were in the bots, both fixed:

1. **F1 — std libm trig in the bot aim** [M bots.rs]: `dz.atan2(dx)` + `yaw.cos()/yaw.sin()` fed
   the aim-cone gate `dot <= 0.9998` [M 0x178c48]. std trig differs by ulps across libm
   versions/platforms → two peers pick different fire ticks → desync. **Fixed**: the game's own
   `ATAN2_APPROX` 0x1e4528 and `PS2Sinf` 0x120700 / `PS2Cosf` (`crate::trig`, bit-exact to the
   game, validated in trig.rs:336-359 and `diff_calls.rs`); same convention (tangent dz/dx).
2. **F4 — two bot clocks**: the brains read the driver-supplied `ctx.now` (accumulated 1/60) for
   the sight latch / regen / fire-timing comparisons while `next_bullet_at` was stored on the
   brain's own `tick/60` [M bots.rs] — a host deriving `now` differently moved sim state. **Fixed**:
   the brain's canonical clock is its own 60 Hz tick counter; `ctx.now` is kept for API
   compatibility and ignored (documented on the field). The regen compare is now exact in ticks.

**The RNG.** One `Rng::new(seed)` stream per match — the game's own bit-exact generator
(`Rand_Rand` 0x1e3738 / `Rand_FRand` 0x1e3780; `drone.rs:383`). Every MP/bot draw comes off it in
call order (all 16 call sites listed in the doc); no other entropy exists anywhere in `nf-game`
(grep: no rand crate, no getrandom, no SystemTime/Instant outside prof.rs, which never feeds the
sim). Tests pin it: an extra out-of-band draw desyncs the hash; a different seed diverges.

**Deliverables** (all on the branch, pushed):

- `nf_game::mp::hash` — `state_hash(&MpMatch) -> u64`: FNV-1a 64 over a canonical walk of every
  sim field, **f32 hashed as `to_bits()`** (never formatting/PartialEq — −0.0 and NaN payloads are
  distinct states), integers as LE bytes, `Option`/`Vec` tagged. Own hasher, not
  `std::hash::DefaultHasher` (SipHash cross-version stability is not guaranteed). Reference FNV
  vectors pinned. Plus `BotBrain::state_hash()` (brains are sim state; derived logs hash as
  lengths — content divergence implies a hashed divergence, so the per-tick hash stays O(1)).
- `nf_game::mp::replay` — the recorded-input harness: `MpRecord` = seed + settings +
  level-derived registrations + `slot_spawns` (slot list only — the spawn indices are DERIVED,
  re-picked through `get_spawn_point`'s shared-stream draw [M 0x185098]) + per-tick inputs
  (positions, ordered events, on-hill mask, paused). `run_record` drives the exact live call
  order; `apply_event` is the single event→method mapping used by both live play and replay.
- `docs/systems/net-lockstep.md` — the contract, the audit, the design, and what the future
  `nf-net` host owes (std-only UdpSocket, direct connect, tick counts from the network — never
  from `ticks_for_frame`; the port's live loop currently ticks only `m.update`, so wiring
  `update_objects`/events/bots into the host tick is the pre-netcode step).
- 6 tests in `crates/nf-game/tests/mp_lockstep.rs` (headless, no ISO, ~1.3 s; every assertion has
  a failing sister): the **10 000-tick two-sim bots-on soak over all 12 scenario words** with the
  combined match+brains hash equal every tick; `hash_catches_a_one_bit_state_difference` (one bit
  of an f32 / an int / a settings word / a bot var each flips the hash, restoring restores); the
  replay round-trip (4 000 ticks, transcript-only, identical hashes every tick) with the
  dropped-input sister; the extra-RNG-draw and different-seed sisters.

## How it was verified

Ran (all `--offline --release --test-threads=1`, `NF_ISO` set, foreground — background jobs are
killed between tool calls in this sandbox, so the background fast-runner pass cannot complete
here; foreground chunks stand in, as in the last windows):

- **nf-game: 371 passed / 0 failed / 2 ignored** (the full crate, includes mp_match_rules 14/14,
  mp_arenas_live, all the bots/drone/world suites).
- **`mp_lockstep` with `--features mp-bots`: 6/6** (the feature is not part of the standard suite
  command, so it ran separately — bots compile behind it).
- **nf-audio: 13/0. nf-formats: 92/3** — the 3 failures are the permitted `disc_golden` trio on
  the slim ISO.
- **nf-view targeted determinism set: 28/0** — mp_core_live 5, mp_bots_live 2 (the ISO rigs,
  green after both fixes), mp_menu_flow 9, mp_menu_lists_live 5, replay_live 3,
  fps_determinism_live 4.
- The full nf-view pass (wgpu GPU probes + the long walks) does not fit the window — same flag as
  the previous windows' reports; the GPU probes are the known environmental failures.

Crates touched: `nf-game` only (mp/mod.rs module registry, mp/hash.rs, mp/replay.rs, mp/bots.rs,
tests/mp_lockstep.rs) + docs. No game bytes, tokens, URLs or credentials committed; staged by name.

## What's left (for the next window / the nf-net task)

1. The `nf-net` crate itself: UdpSocket host/join, the per-tick input exchange, the per-tick hash
   compare, desync handling (§6 of the doc is the design sketch). No crate downloads needed.
2. Wire the full tick (events + `update_objects` + `BotBrain::step`) into the live host
   (`nf-view/src/scene.rs` ticks only `m.update` today) — the pre-netcode step; the replay
   harness's call order is the recipe.
3. Optional live check for the main agent (in the doc §7): gdbwatch samples of the game's own
   `ATAN2_APPROX` during bot aim, to assert `atan2_approx` against on-hardware bot turns.
4. `ctx.now` is now ignored — a follow-up could drop the field and update the two rigs that set
   it (kept for API compatibility this round).

## ntfy

(sent — see the finish steps; if it failed, the exact error would be here under "ntfy failed".)

---
## Report from the worker (done, 2026-10-15T23:48Z)

# net-determinism — the MP sim is lockstep-safe (branch `s/net-determinism`)

## What's done

**The audit.** Every nondeterminism source in the MP tick was checked in code: wall-clock/time
reads, HashMap/HashSet iteration into state, sorts without total order, uninitialised memory,
threads/channels, "pick any" over unordered data, and libm-dependent float paths. Full table with
the [M] citations in `docs/systems/net-lockstep.md` (F1–F9). The core verdict: the MP match core
(`mp/mod.rs`, `mp/rules.rs`) was already a clean pure function of (settings, registrations, rng,
dt, events) — fixed `DT = 1.0/60.0` literal, no clock reads, no unordered iteration, no threads,
one seeded RNG stream. Two REAL finds were in the bots, both fixed:

1. **F1 — std libm trig in the bot aim** [M bots.rs]: `dz.atan2(dx)` + `yaw.cos()/yaw.sin()` fed
   the aim-cone gate `dot <= 0.9998` [M 0x178c48]. std trig differs by ulps across libm
   versions/platforms → two peers pick different fire ticks → desync. **Fixed**: the game's own
   `ATAN2_APPROX` 0x1e4528 and `PS2Sinf` 0x120700 / `PS2Cosf` (`crate::trig`, bit-exact to the
   game, validated in trig.rs:336-359 and `diff_calls.rs`); same convention (tangent dz/dx).
2. **F4 — two bot clocks**: the brains read the driver-supplied `ctx.now` (accumulated 1/60) for
   the sight latch / regen / fire-timing comparisons while `next_bullet_at` was stored on the
   brain's own `tick/60` [M bots.rs] — a host deriving `now` differently moved sim state. **Fixed**:
   the brain's canonical clock is its own 60 Hz tick counter; `ctx.now` is kept for API
   compatibility and ignored (documented on the field). The regen compare is now exact in ticks.

**The RNG.** One `Rng::new(seed)` stream per match — the game's own bit-exact generator
(`Rand_Rand` 0x1e3738 / `Rand_FRand` 0x1e3780; `drone.rs:383`). Every MP/bot draw comes off it in
call order (all 16 call sites listed in the doc); no other entropy exists anywhere in `nf-game`
(grep: no rand crate, no getrandom, no SystemTime/Instant outside prof.rs, which never feeds the
sim). Tests pin it: an extra out-of-band draw desyncs the hash; a different seed diverges.

**Deliverables** (all on the branch, pushed):

- `nf_game::mp::hash` — `state_hash(&MpMatch) -> u64`: FNV-1a 64 over a canonical walk of every
  sim field, **f32 hashed as `to_bits()`** (never formatting/PartialEq — −0.0 and NaN payloads are
  distinct states), integers as LE bytes, `Option`/`Vec` tagged. Own hasher, not
  `std::hash::DefaultHasher` (SipHash cross-version stability is not guaranteed). Reference FNV
  vectors pinned. Plus `BotBrain::state_hash()` (brains are sim state; derived logs hash as
  lengths — content divergence implies a hashed divergence, so the per-tick hash stays O(1)).
- `nf_game::mp::replay` — the recorded-input harness: `MpRecord` = seed + settings +
  level-derived registrations + `slot_spawns` (slot list only — the spawn indices are DERIVED,
  re-picked through `get_spawn_point`'s shared-stream draw [M 0x185098]) + per-tick inputs
  (positions, ordered events, on-hill mask, paused). `run_record` drives the exact live call
  order; `apply_event` is the single event→method mapping used by both live play and replay.
- `docs/systems/net-lockstep.md` — the contract, the audit, the design, and what the future
  `nf-net` host owes (std-only UdpSocket, direct connect, tick counts from the network — never
  from `ticks_for_frame`; the port's live loop currently ticks only `m.update`, so wiring
  `update_objects`/events/bots into the host tick is the pre-netcode step).
- 6 tests in `crates/nf-game/tests/mp_lockstep.rs` (headless, no ISO, ~1.3 s; every assertion has
  a failing sister): the **10 000-tick two-sim bots-on soak over all 12 scenario words** with the
  combined match+brains hash equal every tick; `hash_catches_a_one_bit_state_difference` (one bit
  of an f32 / an int / a settings word / a bot var each flips the hash, restoring restores); the
  replay round-trip (4 000 ticks, transcript-only, identical hashes every tick) with the
  dropped-input sister; the extra-RNG-draw and different-seed sisters.

## How it was verified

Ran (all `--offline --release --test-threads=1`, `NF_ISO` set, foreground — background jobs are
killed between tool calls in this sandbox, so the background fast-runner pass cannot complete
here; foreground chunks stand in, as in the last windows):

- **nf-game: 371 passed / 0 failed / 2 ignored** (the full crate, includes mp_match_rules 14/14,
  mp_arenas_live, all the bots/drone/world suites).
- **`mp_lockstep` with `--features mp-bots`: 6/6** (the feature is not part of the standard suite
  command, so it ran separately — bots compile behind it).
- **nf-audio: 13/0. nf-formats: 92/3** — the 3 failures are the permitted `disc_golden` trio on
  the slim ISO.
- **nf-view targeted determinism set: 28/0** — mp_core_live 5, mp_bots_live 2 (the ISO rigs,
  green after both fixes), mp_menu_flow 9, mp_menu_lists_live 5, replay_live 3,
  fps_determinism_live 4.
- The full nf-view pass (wgpu GPU probes + the long walks) does not fit the window — same flag as
  the previous windows' reports; the GPU probes are the known environmental failures.

Crates touched: `nf-game` only (mp/mod.rs module registry, mp/hash.rs, mp/replay.rs, mp/bots.rs,
tests/mp_lockstep.rs) + docs. No game bytes, tokens, URLs or credentials committed; staged by name.

## What's left (for the next window / the nf-net task)

1. The `nf-net` crate itself: UdpSocket host/join, the per-tick input exchange, the per-tick hash
   compare, desync handling (§6 of the doc is the design sketch). No crate downloads needed.
2. Wire the full tick (events + `update_objects` + `BotBrain::step`) into the live host
   (`nf-view/src/scene.rs` ticks only `m.update` today) — the pre-netcode step; the replay
   harness's call order is the recipe.
3. Optional live check for the main agent (in the doc §7): gdbwatch samples of the game's own
   `ATAN2_APPROX` during bot aim, to assert `atan2_approx` against on-hardware bot turns.
4. `ctx.now` is now ignored — a follow-up could drop the field and update the two rigs that set
   it (kept for API compatibility this round).

## ntfy failed

All three fallbacks failed — ntfy.sh's daily message quota is exhausted (the service returned the
same 429 to every attempt, ~23:50 UTC 2026-10-15):

1. curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase
   your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. python3 urllib: `HTTPError 429: 'Too Many Requests'`
3. wget: exit code 8 (server error), no body.

The message that would have been sent: "net-determinism: done — MP sim proven lockstep-safe:
audit fixed bots' libm trig (ATAN2_APPROX) + the ctx.now seam; state_hash(&MpMatch) FNV-1a over
f32 bits; replay harness; 10k-tick two-sim bots-on soak over all 12 scenarios green (nf-game
371/0)". The queue's `done/050-net-determinism.md` entry carries this report; the branch is
pushed through `0c492a2f` + the report fix below.

---
## Report from the worker (done, 2026-10-15T23:52Z)

# net-determinism — the MP sim is lockstep-safe (branch `s/net-determinism`)

## What's done

**The audit.** Every nondeterminism source in the MP tick was checked in code: wall-clock/time
reads, HashMap/HashSet iteration into state, sorts without total order, uninitialised memory,
threads/channels, "pick any" over unordered data, and libm-dependent float paths. Full table with
the [M] citations in `docs/systems/net-lockstep.md` (F1–F9). The core verdict: the MP match core
(`mp/mod.rs`, `mp/rules.rs`) was already a clean pure function of (settings, registrations, rng,
dt, events) — fixed `DT = 1.0/60.0` literal, no clock reads, no unordered iteration, no threads,
one seeded RNG stream. Two REAL finds were in the bots, both fixed:

1. **F1 — std libm trig in the bot aim** [M bots.rs]: `dz.atan2(dx)` + `yaw.cos()/yaw.sin()` fed
   the aim-cone gate `dot <= 0.9998` [M 0x178c48]. std trig differs by ulps across libm
   versions/platforms → two peers pick different fire ticks → desync. **Fixed**: the game's own
   `ATAN2_APPROX` 0x1e4528 and `PS2Sinf` 0x120700 / `PS2Cosf` (`crate::trig`, bit-exact to the
   game, validated in trig.rs:336-359 and `diff_calls.rs`); same convention (tangent dz/dx).
2. **F4 — two bot clocks**: the brains read the driver-supplied `ctx.now` (accumulated 1/60) for
   the sight latch / regen / fire-timing comparisons while `next_bullet_at` was stored on the
   brain's own `tick/60` [M bots.rs] — a host deriving `now` differently moved sim state. **Fixed**:
   the brain's canonical clock is its own 60 Hz tick counter; `ctx.now` is kept for API
   compatibility and ignored (documented on the field). The regen compare is now exact in ticks.

**The RNG.** One `Rng::new(seed)` stream per match — the game's own bit-exact generator
(`Rand_Rand` 0x1e3738 / `Rand_FRand` 0x1e3780; `drone.rs:383`). Every MP/bot draw comes off it in
call order (all 16 call sites listed in the doc); no other entropy exists anywhere in `nf-game`
(grep: no rand crate, no getrandom, no SystemTime/Instant outside prof.rs, which never feeds the
sim). Tests pin it: an extra out-of-band draw desyncs the hash; a different seed diverges.

**Deliverables** (all on the branch, pushed):

- `nf_game::mp::hash` — `state_hash(&MpMatch) -> u64`: FNV-1a 64 over a canonical walk of every
  sim field, **f32 hashed as `to_bits()`** (never formatting/PartialEq — −0.0 and NaN payloads are
  distinct states), integers as LE bytes, `Option`/`Vec` tagged. Own hasher, not
  `std::hash::DefaultHasher` (SipHash cross-version stability is not guaranteed). Reference FNV
  vectors pinned. Plus `BotBrain::state_hash()` (brains are sim state; derived logs hash as
  lengths — content divergence implies a hashed divergence, so the per-tick hash stays O(1)).
- `nf_game::mp::replay` — the recorded-input harness: `MpRecord` = seed + settings +
  level-derived registrations + `slot_spawns` (slot list only — the spawn indices are DERIVED,
  re-picked through `get_spawn_point`'s shared-stream draw [M 0x185098]) + per-tick inputs
  (positions, ordered events, on-hill mask, paused). `run_record` drives the exact live call
  order; `apply_event` is the single event→method mapping used by both live play and replay.
- `docs/systems/net-lockstep.md` — the contract, the audit, the design, and what the future
  `nf-net` host owes (std-only UdpSocket, direct connect, tick counts from the network — never
  from `ticks_for_frame`; the port's live loop currently ticks only `m.update`, so wiring
  `update_objects`/events/bots into the host tick is the pre-netcode step).
- 6 tests in `crates/nf-game/tests/mp_lockstep.rs` (headless, no ISO, ~1.3 s; every assertion has
  a failing sister): the **10 000-tick two-sim bots-on soak over all 12 scenario words** with the
  combined match+brains hash equal every tick; `hash_catches_a_one_bit_state_difference` (one bit
  of an f32 / an int / a settings word / a bot var each flips the hash, restoring restores); the
  replay round-trip (4 000 ticks, transcript-only, identical hashes every tick) with the
  dropped-input sister; the extra-RNG-draw and different-seed sisters.

## How it was verified

**Final tree = branch merged with origin/master (24b1f4dd, merge 2115381f), re-run and pushed:**

- **nf-game: 378 passed / 0 failed / 2 ignored** (the merged crate — the 7 new stats tests from
  master green too).
- **`mp_lockstep` with `--features mp-bots`: 6/6** (the feature is not part of the standard suite
  command, so it ran separately — bots compile behind it).
- **nf-view targeted determinism set: 28/0** — mp_core_live 5, mp_bots_live 2 (the ISO rigs,
  green after both fixes), mp_menu_flow 9, mp_menu_lists_live 5, replay_live 3,
  fps_determinism_live 4.

Pre-merge (same session, same command shape): nf-game 371/0, nf-audio 13/0, nf-formats 92/3 (the
3 permitted `disc_golden` on the slim ISO), nf-view targeted 28/0.

All runs `--offline --release --test-threads=1`, `NF_ISO` set, **foreground** — background jobs
are killed between tool calls in this sandbox, so the background fast-runner pass cannot complete
here (the same limitation the last windows reported); foreground chunks stand in, and the full
nf-view pass (wgpu GPU probes + the long walks) does not fit the window — the GPU probes are the
known environmental failures.

Crates touched: `nf-game` only (mp/mod.rs module registry, mp/hash.rs, mp/replay.rs, mp/bots.rs,
tests/mp_lockstep.rs) + docs. No game bytes, tokens, URLs or credentials committed; staged by name.

## What's left (for the next window / the nf-net task)

1. The `nf-net` crate itself: UdpSocket host/join, the per-tick input exchange, the per-tick hash
   compare, desync handling (§6 of the doc is the design sketch). No crate downloads needed.
2. Wire the full tick (events + `update_objects` + `BotBrain::step`) into the live host
   (`nf-view/src/scene.rs` ticks only `m.update` today) — the pre-netcode step; the replay
   harness's call order is the recipe.
3. Optional live check for the main agent (in the doc §7): gdbwatch samples of the game's own
   `ATAN2_APPROX` during bot aim, to assert `atan2_approx` against on-hardware bot turns.
4. `ctx.now` is now ignored — a follow-up could drop the field and update the two rigs that set
   it (kept for API compatibility this round).

## ntfy failed

All three fallbacks failed — ntfy.sh's daily message quota is exhausted (the service returned the
same 429 to every attempt, ~23:50 UTC 2026-10-15):

1. curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase
   your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. python3 urllib: `HTTPError 429: 'Too Many Requests'`
3. wget: exit code 8 (server error), no body.

The message that would have been sent: "net-determinism: done — MP sim proven lockstep-safe:
audit fixed bots' libm trig (ATAN2_APPROX) + the ctx.now seam; state_hash(&MpMatch) FNV-1a over
f32 bits; replay harness; 10k-tick two-sim bots-on soak over all 12 scenarios green (nf-game
371/0)". The queue's `done/050-net-determinism.md` entry carries this report; the branch is
pushed through `0c492a2f` + the report fix below.
