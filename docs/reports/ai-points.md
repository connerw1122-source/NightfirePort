# ai-points — report

Task 747, branch `s/ai-points` (from origin/master bee8c129). Sandbox window ran out mid-task; the
state below is what the branch carries.

## What's done

**The record.** `Drone_AIPoint` (0x137550, placement 0xE9, 162 across the disc) is ported as
`crates/nf-game/src/aipoint.rs`. The record reads (level_tag → placement-parameter mapping
`+0x2c+4k` ↔ param k, pinned off the already-ported `Drone_AIVolume_Create` 0x1376c0 and
`Trigger_Create` 0x1b1720):

- `+0x2c` u16 = parameter 0 → the point's **group** (`AIPoint+0x06`);
- `+0x30` word = parameter 1 → the **goal accuracy** float (`AIPoint+0x14`); `cvt.s.w` when the
  word is ≥ 0, the `srl 1 / andi 1 / or / cvt.s.w / add.s ×2` form when negative
  [M disasm 0x1375d4-0x137614] (no negative words occur on the disc's 162 records);
- `+0x34` word = parameter 2 → the **owner** (`AIPoint+0x0c`), matched against `Drone+0x144`
  (itself placement parameter 0x11, `DefaultInit` 0x14b300:472);
- `+0x20` = the placement translation (the goal position), `+0x44` = the placement's y-rotation
  raw bits (`FindAlarmPoint` stores them at Drone+0x734 [M 0x153008:24]).

The embedded `AIEmitter` (+0x50) binds `AINetwork_NavPathForPosition(pos)` at creation
[M 0x15b1e8:36-42] and `Drone_PostLoad_Init` 0x136938:78-86 runs `AINetwork_EmitPath` (a Dijkstra
from the point over its path, the port's existing `Network::emit`) per point at level load —
`aipoint::build` does both, the same shape as the ported cover-node build.

**The wiring.**

- `NDrone2_ClosestAIPoint` (0x151b18) → `aipoint::closest`: a point qualifies when its group
  matches, its emitter path equals the drone's nav path (`Drone+0x954`, now set at load to
  `NavPathForPosition(drone pos)` per `NDrone2_PostLoad_Init` 0x14d890:18-22 — the port set it
  lazily before), and its distance byte at the drone's nearest node is not 0xff; among the
  eligible ones least cost wins (first on ties) and a point whose owner matches the drone's
  +0x144 blocks non-matching points once incumbent [M 0x151b18:128-181].
- `NDrone2_FindRunToPoint` (0x153168): the asked group — property 0x56 → 8, property 0x4e → 1,
  class byte 1 → 0, 2 → 9, 3 → 2, others none; a hit sets the goal to the point's position.
- `DSTATE_EnemyRunToPoint` (0x168910) ENTER: prop 0x56 → RunToAlarm 0x89; the return state
  +0x5A2 (0x56, on 11/12/13/41 unseen → 4/5); `FindRunToPoint` found → +0x5A0 = +0x5A2 and
  class 1 → GoToGoalPosition 0x63, else **HostageGoToGoalPosition 0x64 — newly ported**
  (`NDrone2_DSTATE_HostageGoToGoalPosition` 0x165a68: the run/walk anims, arrival → +0x5A2,
  anim-done → +0x5A2).
- `DSTATE_CivilianScared` (0x15d3a0) ENTER, mode-7 arm: the scared-run marker (`Drone+0xC6 = 7`,
  the port's `scared_run`) runs `FindRunToPoint` — found → return state 0x13 and 0x64, none →
  hiding [M 0x15d3a0:34-55].

**Tests** (`crates/nf-game/tests/ai_points_live.rs`, opt-in `NF_ISO`; all 4 pass):

- `aipoint_records_07000007` — 07000007's 33 main-map records, every one group 2 / accuracy 1.0 /
  owner 0 [D, `nf-extract placements 07000007.bin e9`], each bound to a nav path with a full
  dist array (one record is all-0xff: unreachable, never eligible [M 0x151b18:142]); the level's
  34th census record is the empty-params map-1 placement.
- `closest_point_selection_on_07000007` — the selection rules: the point the ask stands on (cost
  0) wins; group 0/1/8/9 asks come up empty; a pathless drone finds nothing; a path no point
  binds finds nothing [M 0x151b18].
- `scared_guest_runs_to_the_closest_aipoint` — the real router: a team-3 guest with the mode-7
  marker, SCREAM (prop 0x27) → CivilianScared → the closest group-2 point → goal = that point's
  position → arrival → CivilianHiding. Fails on the pre-change tree (the guest hid at once).
- `scared_guest_without_the_mode_hides_at_once` — the sister pin: no marker → straight to hiding
  [M 0x15d3a0:53-55].

`crates/nf-formats/tests/script_census.rs`: 0xE9 moved to the handled side (57 sim placement
types; unhandled 1537 → 1375). All 6 census tests pass.

## How it was verified

- `cargo test --offline --release -p nf-game --test ai_points_live` — 4 passed.
- `cargo test --offline --release -p nf-formats` — only the 3 `disc_golden` failures (the slim
  ISO's allowed set).
- `cargo test --offline --release -p nf-game --test cover_selection_live --test
  fix_ai_remainder_live` — pass (the sibling AI consumers the nav-path change touches).
- The fast suite (`tools/test/suite.py`) ran once on the branch at ~01:12-01:53 UTC — **before**
  the implementation landed: 1945 passed, 7 failed — the 3 allowed `disc_golden` plus
  `fix_0c_pylon_live` ×2 (searchlight/pylon view spots, pre-existing, not touched by this task)
  and 2 more FAILED lines the log truncated (grep saw 5). **The suite did not run again on the
  final tree** — the sandbox window closed first. The final tree is compiled clean
  (`cargo check -p nf-game -p nf-view --tests`) and the touched crates' tests above pass.

## What's left

- The alarm side: `NDrone2_FindAlarmPoint` (0x153008) found-arm and `NDrone2_AssignAIPath`'s
  group-8 store (0x154190:55-66, `Drone+0xa70`) — the port's `assign_path` still degrades to the
  generic path assignment (documented at the fn). 07000007 has no group-8 points and no kind-8
  paths, so nothing there is testable on the task level.
- `DSTATE_PressAlarm` (0x168ea8, state 0x8A) — the RunToAlarm found-arm's destination, unported;
  wiring the found arm without it would strand the drone in an unknown state.
- The goal-accuracy float (route+0x8c / AITarget+0x14, `SetupRouteToPosition` 0x158b10) is
  carried in the record but not modelled by the port's `Route` (arrival stays the game's 2.0,
  `DefaultInit` 0x14b300:579). 07000007's accuracy is 1.0 — the difference is arrival-edge only.
- The `+0xC6 = 7` setter: all four `DroneFunc_HandleImpact` blocks carry the level gate
  `GameState + 0xf8fffff7 < 2` (= 0x07000009/0x0700000A) [M 0x1469b8:103-127 et al.]; the exact
  message(s) that reach them is [S]. The port leaves `scared_run` settable (the test sets it);
  wiring the impact setter on 09/0A is follow-up.

## Live checks for the main agent

- `gdbwatch.py` on a guest's `Drone+0xC6` (0x07000009): expect `= 7` written from
  `DroneFunc_HandleImpact` (0x1469b8) when a gunshot/scary impact reaches it — and which msg id
  was in flight (the [S] above).
- `fieldprobe.py` poke a guest's `Drone+0x954` in 07000007 to the AI points' path pointer and
  watch `Drone+0x6F0` after a scare: expect it to become an `AIPoint` record whose +0x06 == 2.

## Usage

not visible to me
