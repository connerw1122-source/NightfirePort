# mp-core — report

Branch `s/mp-core`, cut from `origin/master` 1288f0d6. The task: multiplayer phase 1 (atlas §6) —
the match core on one arena: `MP_Init`/`MP_Start`/`MP_Update`, `MPSettings` 0x2a47a0 / `MPGame`
0x2a4980, the spawn-point registration and selection, the MPObject registration, and
`nf-view <iso> 07000023 --mp` loading Arena 1 with Bond at a policy-chosen spawn and the match clock
running. Bots are phase 2+; none of this phase drives them.

## What's done

**`crates/nf-game/src/mp/mod.rs`** (new) — the match core, every field with its game offset:

- `MpSettings` (0x2a47a0) and `MpGame` (0x2a4980) as Rust structs — the carried fields carry their
  offsets in doc comments; `MpSettings::deathmatch(level_code)` is the phase-1 config (scenario 0x1,
  humans 1, bots 0) [S — the menu defaults are not read in phase 1].
- `MpMatch::init` = `MP_Init` 0x183438: the zeroing, `MPGame+0x404 = (f32)MPSettings+0x416`,
  `MPSettings+0x396 = (scenario >> 29) & 1` (the team game), `+0x400 = (scenario >> 30) & 1`, the
  per-slot `+0x20 = 0xfffe` / `+0x28 = 0xffff`, the 0x10-mode death-limit copy, the demo/prot
  60.0f default limit [M 0x183438:10-65].
- `register_spawn_point` = `MP_RegisterSpawnPoint` 0x184f20: the team-2 rejection [51-53], the
  FFA/team range split (team games and Assassin split the 64-entry table [0,0x20)/[0x20,0x40)
  [54-67]), the first-free-slot scan, the floor snap — `build_PointOnFloor` 0x1c95a8 with reach 3.0
  (a straight-down ray; on a hit the pos = the hit point) — and the **+1.6 eye** [79-96], the facing
  vec4, the used flag and the per-team counts.
- `get_spawn_point` = `MP_GetSpawnPoint` 0x185098: the candidate gate (the nearest-enemy squared
  distance > 2.0, the enemies read from the `MPGame` slots' obj positions, self excluded [116-135]);
  the two rolling trackers are an **argmax** (`uStack_d8`) and an **argmin** (`uStack_d4`) [136-154];
  the policies at `MPSettings+0x448`: **0 → the argmin** (the least-far candidate), **1 → the
  argmax** (the farthest), **2 → `cands[Rand_Rand(n)]`** [177-180], anything else → the range start
  [181-183], with the −1 fallbacks [165-176]. The atlas's "farthest/2nd-farthest/random" gloss does
  not survive the arithmetic — the keep-old/take-new updates are a running min and max, so policy 0
  picks the least-far candidate. Ported as the decompile says; a live check below settles the UI
  mapping.
- `register_mp_object` = `MP_RegisterMPObject` 0x185990 (stored, not driven): the type dispatch
  (0/1 CTF, 2 uplink, 3 demolition, 4/5 blueprint, 6/7 GoldenEye, 8 protection with the game's
  +0x34 ↔ +0x40 config swap, 9 KOTH), each type gated on its scenario word and capped at 8; the
  0x60 place records keep the matrix, the 0x1c config and the cel pointer. *Deviation*: the
  immediate-create path (`MP_CreateObject`) stores the record without building an object (phase 1
  drives no MP objects).
- `start` = `MP_Start` 0x183a50 (the phase's part): the master gate, `MPGame+0x420 = −1.0f` [206],
  the started flag [69,207]. The mode-object creation draws consume nothing for Deathmatch
  [77-113].
- `update` = `MP_Update` 0x184bd0: the per-slot timer decrement [25-35], state 0 →
  `check_for_end_condition` = `MP_CheckForEndCondition` 0x184268's clock: the elapsed `+0x412`
  always, the round clock `+0x400` when not paused [44-46,55,140], the time-up latch
  (`switch_channels[0xfe]`) at the limit, the demo/prot state-6 entry [57-60]. *Deviation* (the
  module doc): the game samples `psiGetTimeIn100ths` (a wall clock); the port accumulates the fixed
  1/60 tick — the determinism rule forbids wall-clock time in Sim, and the game's own state-3/6
  timers use `REC_FRAME_RATE` the same way.
- **No new RNG**: the port's `Rng::rand` already is `Rand_Rand` 0x1e3738 exactly (y then x steps,
  `((y & 0xffff) × n) >> 16`).

**The placement wiring** — corrected against the caller: `parsemap_block_map_data_dynamic` 0x1d1b68
passes the dispatch the placement record, the **0xb0 params block** (values written by key:
block+0x2c+key·4), the type word, the CEL record and cel+0x50 — so the 0x25 **team** and the 0x26
**MP object type** are the placement's **param 0** [M `parsemap_create_dynamic_objects` 0x1d04f0
case 0x25/0x26 reads `param_2+0x2c`], not a celglist field as the atlas glossed; the doc's §1 line
is corrected in `docs/atlas/multiplayer.md` (the status block).

**`crates/nf-view/src/lib.rs`** — the `--mp` flag (minimal): `load_mp` wraps the existing
`load_impl` (one new `mp: bool` parameter through the 7 call sites); when set, the sim block runs
the registrations from the main map's placements, `start`, the first human's pick
(`MP_GetSpawnPoint(team, NULL)` BEFORE the player exists — the game's own order [0x183a50:120-123]),
stands Bond at the point (`Bond::stand_at`, yaw = the facing's rotation[1] — the same convention
`level_starts` pins for the campaign starts) and stores the match on the sim.

**`crates/nf-view/src/scene.rs`** (shared file — the only hook beyond the named files): one additive
`pub mp: Option<nf_game::mp::MpMatch>` field on `Sim` (own line, `None` on every campaign level) and
one `m.update(1/60, false)` call at the top of `step_bond` [M the `MP_Update` per-frame slot]. Also
`crates/nf-game/src/lib.rs`: the one-line `pub mod mp;`.

## The deliverable, verified

`nf_view::load_mp(<iso>, "07000023.bin", false)` (the `--mp` path) prints:
`mp: arena 07000023 — 32 spawn points registered, Bond at spawn 0 (−16.15, 2.04, 3.38) yaw 1.588` —
Arena 1 loads, the policy (0) picks spawn 0 (its range-start fallback with no other players — the
game's own arithmetic), and the clock runs per tick thereafter. The interactive render needs the
main agent's GPU (the sandbox has none — the `wgpu` adapter tests panic here), spots below.

## Tests (`crates/nf-view/tests/mp_core_live.rs`, 5/5 green; NF_ISO opt-in)

1. `spawn_tables_match_the_placements_for_all_8_arenas` — all 8 arenas: every 0x25 placement
   registers in placement order, x/z exact, y = the floor snap + 1.6 (re-derived per point), the
   team counts sum. The registered count = the placement count minus the **team-2 rejects** — the
   measurement found several arenas' 0x25 placements carrying param-0 = 2, which the game rejects
   outright [M 0x184f20:51-53] (the mp_arenas_live pin counts placements, not registrations).
   Sister: the counts differ across arenas, so a constant pin fails.
2. `spawn_choice_matches_the_decompile_arithmetic` — the candidate gate, both trackers, all three
   policies (policy 2's expectation recomputed from the LCG by hand), the me-exclusion, the −1
   fallbacks, the unknown-policy fallback. Sister: the team-game split and the team-2 rejection.
3. `team_games_split_the_spawn_table` — the `(scenario >> 29) & 1` derivation; team 1 lands at
   0x20 in a team game, at 0 in FFA.
4. `mp_object_registration_gates_on_the_scenario` — 07000023's 27 type-0x26 placements store
   nothing under Deathmatch (every type's gate fails), exactly the param-0 ∈ {0,1} subset under CTF;
   the stored matrix keeps the placement translation. Sister: the count re-derives from the
   placements.
5. `the_mp_load_spawns_bond_and_runs_the_clock` — the end-to-end `load_mp` through the real
   `Sim::step_bond`: Bond stands at a registered spawn point, the real-world floor snap + 1.6
   re-verified, 60 ticks = 1.0 s on `MPGame+0x400` (bit-deterministic — a fresh fold matches),
   `+0x412` accumulates the same delta, and the time-up latch fires once the (shortened) limit
   passes.

Fail-first: the deliverable is new code — on the pre-module tree the tests fail at compile time
(no `nf_game::mp`); the branch history shows the tests landing with the module (the WORKLOG notes
the order). Regression sweep on the final tree: nf-game lib 160/160, `mp_arenas_live`,
`ai_traces_live` 4/4, `fix_06_driver_live` 2/2, `fix_ai_remainder_live`, `fix_08_collision_live`
4/4, `facial3_live` 3/3, `casings16_live` 2/2 — the scene.rs/lib.rs hooks are additive and
`mp: None` on every campaign path.

## Full suite

The chunked runner covered the blast radius (above). The full-suite total: run per the card before
the finish (see the WORKLOG for the line; on the slim ISO only the 3 `disc_golden` tests may fail).

## What's left (phase 2+)

1. `MP_Pickup_Process` / `BOT_update` in state 0 (no pickups driven, no bots) and the end-state
   wiring (states 1-6, `MP_SortOutWhoWon`), the frag-limit latch consumer.
2. The MPObject driving (the per-mode updaters), `MP_CreateObject`'s real object build.
3. Bots (phase 4 of the plan), split screen, the MP HUD panes, the radar.
4. The time-limit units ([S] minutes — the HUD math `(limit − clock) × 100` centiseconds suggests
   the menu value may be seconds or converted earlier; a live check settles it).

## Live checks for the main agent

1. **The spawn policy mapping** (the atlas gloss vs the arithmetic): boot a 4-human deathmatch with
   the spawn option cycled through its three values; watch `MP_GetSpawnPoint` 0x185098's return with
   gdbwatch — expected per the decompile: option 0 → the least-far-from-enemies candidate, option 1
   → the farthest, option 2 → random. If the game's UI order differs, the port's policy constants
   re-map (one line).
2. **The clock units**: watch `MPGame+0x400` (r per frame) with the menu time limit set to 1 —
   expected: the clock reaches the limit in 1 real minute if the limit is minutes-as-copied (the
   port's current reading) or 60× faster if the menu stores seconds; also `Timer_Seconds2String`'s
   input `((limit − clock) × 100)` on the HUD.
3. **The team-2 spawn placements**: watch `SpawnPoints` @0x2a7360 fill at an arena load — expected:
   the count = the 0x25 placements minus the param-0 = 2 ones (the same rejection the port models).
4. **nf-view `--bond` renders** (07000023 `--mp`): Bond standing at his spawn — `--bond
   -16.15,2.04,3.38` (at his head, yaw 1.588): the arena floor, Bond upright, no fall-through;
   `--bond -18,4.5,8` yaw −0.6 pitch −0.3: Bond at the spawn point from a pull-back, the arena
   geometry around. Expected: the standing pose on the floor by the spawn marker, no T-pose, the
   HUD's normal campaign panes (the MP panes are phase 2+).
