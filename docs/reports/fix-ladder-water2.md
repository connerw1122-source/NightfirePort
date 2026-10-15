# fix-ladder-water2 — the walk-on grab regression in 109i's new pick

Task 109i2. Branch `s/fix-ladder-water2` (from `origin/master` 2cbfd7e3, merged `origin/s/fix-ladder-water` = 109i, then `origin/master` again before each push).

The main agent's merge suite caught (reproduced alone: both pass on master and on master + 109h + 109b; both fail with 109i):

```
cargo test --release -p nf-view --test fix_ladder2_live
test camera_looks_during_the_climb ... FAILED
test weapon_saved_on_grab_restored_on_dismount_and_hands_cannot_fire ... FAILED
panicked at crates/nf-view/tests/fix_ladder2_live.rs:50:5: walking into the ladder never attached
```

## 1. The diagnosis (the per-tick evidence the task asked for)

The failing tests' Bond **never walked**. 07000002's level-start NIS arms on the very first step, and `step_bond` zeroes the input and skips the whole player tick while `disabled` (`scene.rs`: `if !self.disabled { … self.bond.tick_ctx(…) }`; the input zero at `let (trigger, reload) = if self.disabled { input = Default::default(); … }`). The old `grab_nearest_ladder` placed Bond 1.2 in front of the ladder and held forward for 900 ticks — all 900 frozen (`dis true` from t0, position bit-identical every tick, `local[1]` pinned at 0.021 = the single pre-NIS ramp tick).

The nearest ladder to the 07000002 start (centre −26.869, 2.542, 42.225, r 2.567, yaw π) leaves the frozen Bond at **frontal +1.130**, lateral +0.060 (the per-tick `pick_diag` print, kept in the test):

- **master's pick**: the bare radial column `min(r,1).max(0.35) + 0.55` = 1.55 ≥ 1.130 → it **grabbed the frozen Bond standing 1.13 off the plane** — a false pass the game never grants: the game's `Player_CollLadder` 0x196118 fires from the walking HITTEST only after the glist mesh refine (`Collide_Update` 0x1ea398:84-93 → `Collide_Intersect` 0x1ebaa8), and a walk capsule parked 1.13 from the rung plane touches nothing.
- **109i's pick**: the mesh-scale stage 2 `|frontal| ≤ 0.55` → 1.130 > 0.55 → never attaches. The "regression" is the pick becoming game-correct; the test's walk was vacuous.

Nothing solid blocks the approach (the ladder mesh is not in the port's world collision — `Level::instances(true)` skips the 0x22 placements exactly as the game's static path does [M 0x1d1998/0x1d1b68]), so a REAL walk reaches the plane — which the fixed test then shows.

## 2. The fixes

### 2.1 The tests walk for real (`fix_ladder2_live.rs::grab_nearest_ladder`)

The NIS is skipped the way the game lets the player — carry the skip action (`sim.drones.scripts.skip_request = true`) while `disabled` until control returns (the `fix_ladder_live::load` / `fix_ladder_water_live::step` pattern), and keep carrying it on every walk tick (the level's scripts grab Bond into cut-scenes within seconds of any step). After the skip the walk-on grab fires at the game's contact: the diagnostic shows frontal +1.130 → +0.282 over 11 walk ticks (the walk ramp 0.021 → 0.099), and **t12 lands sub 1 (SS_CLIMB) the tick the walk capsule's frontal crosses the plane reach**, snapped to the game's own +0.5 offset (front +0.500, lateral −0.000) [M `Player_CollLadder` 0x196118:36-44].

### 2.2 The walk pick is the game's WALK arm (`ladder.rs` + `player.rs`)

[M `Player_Collision` 0x194df0's capsule switch, case 0/6 = substate 0 (walk)] writes the radius slot `+0x7CC = 0x3e8ccccd` = **0.275** and builds the segment **A = pos + up·0.55** (0x3f0ccccd), **B = pos + up·(+0x7CC − cc)** — the B base read from the same radius slot it just wrote. The pre-109i2 port passed the swim arm's 0.55 (`RADIUS`) for every non-swim substate: the walk grab reached 0.55 past the rung plane and **re-grabbed a dry stander parked at the dismount offset 0.5** — the game's walk capsule at 0.5 off the plane touches nothing. Now `ladder_grab` selects the arm by substate exactly as the switch does:

- substate 0, not crouched → `Ladder::capsule_hits`: the walk arm, radius `WALK_PICK_RADIUS = 0.275` [M 0x194df0 case 0/6]; stage 2 `|frontal| ≤ 0.275`, `|lateral| ≤ 0.4 + 0.275`.
- the crouched walk (the port folds the game's substate 4 into substate 0 + the `crouched` flag) and any other walk-tick substate → `Ladder::capsule_hits_default`: the switch's **default arm** [M 0x194df0 case default] radius 0.55, A = up·0.275, B = up·(0.55 − cc). The 102h yard walker grabs crouched at 0.38 off the plane exactly as before (walkreal_0700000c green).
- swim (3) → `Ladder::capsule_hits_swim` unchanged [M 0x194df0 case 3] — the 109i dismount-into-water escape is intact.

All three forms share `pick_hit` (stage 1: the object's world sphere inflated ×1.5 plus the capsule radius against the closest point on the arm's segment [M `Collide_PickObj` 0x1eae80:152-168, mode word BLData+0x7D4 = 0x800]; stage 2: the mesh-scale column [S — the stand-in for the unported cel scoping + glist refine]).

### 2.3 `script: false` in the two Msg literals

`fix_objective_banner_live.rs` and `fix_objective_messages_live.rs` build `nf_view::scene::Msg` literals; 109i added the `script` field (the op-0x1e subtitle flag that `Text_FlushAllSubtitles` 0x1b15c8 deletes — fix-ladder-water). These tests post objective/fail lines, not subtitles, so both literals get `script: false`. This also unblocks the suite build, which failed on these two files before the fix.

## 3. Fail-first

- The two live tests fail on the merged 109i tree (the regression as reported): 900 ticks of frozen stand at frontal 1.130 → "walking into the ladder never attached". Shown before the fix (session log: `dis true` every tick, stage 2 front +1.130 (≤0.55) ✗).
- The dry-stander flip: the pre-fix unit asserted the walk form **reaches** at 0.5 (`swim_pick_is_the_game_swim_capsule…`: `assert!(l.capsule_hits([10.0, 3.0, 20.5], 0.55))` — "at 0.5 BOTH forms still reach"); the re-anchored units assert the walk form must **not** (`!l.capsule_hits([10.0, 3.0, 20.5], WALK_PICK_RADIUS, cc)` — the dry stander never re-grabs) while the swim form still does. Same spot, hit→miss, from the decompile constant [M 0x194df0 case 0/6].

## 4. Verification

- `nf-game --lib`: **161 passed / 0 failed** (the re-anchored ladder units: the walk-on at the plane, the 0.4-off no-grab, the lateral span, the deep walk segment (B = 0.275 − cc), the thin-ladder sister, the 102h dismount rejects at 0.372/2.9, the dry-stander pin, the crouch-grab sister at the default arm).
- The affected live suites (all through `nf_view::load` + `Sim::step_bond`): `fix_ladder2_live` **4/4**, `fix_ladder_water_live` **4/4** (109i's own: the pond dismount escape, the swim grab, the voice stop, the dialogue flush), `fix_ladder_live` **3/3** (+1 ignored probe), `fix_swim_live` **4/4**, `fix_swim_vent_live` **2/2**, `walkreal_0700000c_live` **1/1** (the 102h yard walk, crouched), `fix_objective_banner_live` **3/3**, `fix_objective_messages_live` **5/5**.
- The full fast suite on the final tree (origin/master merged): see the total line below.

Suite (final tree, `tools/test/suite.py`): **(pending — filled at the finish commit)**

## 5. Files

- `crates/nf-game/src/ladder.rs` — the three pick arms over the shared `pick_hit`; the docs; the re-anchored units.
- `crates/nf-game/src/player.rs` — `WALK_PICK_RADIUS = 0.275` [M 0x194df0 case 0/6] + the arm selection in `ladder_grab` (task-named file).
- `crates/nf-view/tests/fix_ladder2_live.rs` — the NIS skip + the per-tick pick diagnostics (the task's "print the sphere test, the refine result and the substate per tick" requirement; the file is outside the task's list but holds the failing tests).
- `crates/nf-view/tests/fix_objective_banner_live.rs`, `fix_objective_messages_live.rs` — `script: false` (task-named).
- `docs/WORKLOG.md` (the entry), this report.

Shared files (`drone.rs`, `scene.rs`, `lib.rs`): **not touched**. No `git add -A`; every commit staged by name.

## 6. What's left / notes for the next pass

- The stage-2 stand-ins (`|frontal| ≤ radius`, `|lateral| ≤ 0.4 + radius`) remain [S] models of the unported cel scoping + glist refine. The live checks below can measure the game's true walk-on reach to tighten them.
- **Flagged, not changed (out of scope)**: the port's WORLD-collision capsule (`Bond::capsule()`, walls/floors) models the standing stance with the DEFAULT arm's segment (A 0.275, B 0.55 − cc) and the crouched stance with the swim arm's pair (0.65 − cc, 0.45 − cc). The game's switch is: substate 0 → (A 0.55, B 0.275 − cc, r 0.275), substate 4 → (A 0.275, B 0.55 − cc, r 0.55), substate 3 → (0.65 − cc, 0.45 − cc, r 0.55) [M 0x194df0]. The world capsule is pinned by the whole walkreal suite (walls, floors, doors); retuning it is its own task with its own walkreal evidence.

## 7. Live checks for the main agent

PCSX2 (07000002 unless noted):

1. **The walk-on grab's reach** — `tools/trace/gdbwatch.py` on the substate write `obj+0xF6` (the `Player_ChangeSubState(1)` store inside `Player_CollLadder` 0x196118:44) while Bond walks into the free-standing ladder near the garden start (approach line x ≈ −26.93, z 41.1 → 42.2, facing +z). Expected: the write fires when Bond's xz is ≈ ≤ 0.3–0.35 from the ladder axis (the walk capsule's contact with the rungs), NOT from 1.0+ away; the grab then snaps him to axis + fwd·0.5 [M 0x196118:36-44].
2. **The dry-stander no-re-grab** — climb any ladder a little, action-dismount at the base, keep the stick neutral ≥ 2 s (the regrab 60 [M `Player_Climb` 0x196310] expires): expected — NO re-latch while standing ~0.5 off the plane (the pre-109i2 port re-grabbed here; the game's walk capsule doesn't touch the rungs). Watch the same `obj+0xF6` write.
3. **The crouched grab** (0700000c) — crouch-walk into the yard ladder: expected — the grab fires (the game's substate-4 default arm, radius 0.55), matching the port's `capsule_hits_default`.
4. **The 109i escape stays** — the pond dismount with the stick held back: expected — no re-latch (unchanged from 109i; guard against this task's walk-arm change leaking into the swim arm).

nf-view (`--bond x,y,z,yaw,pitch`):

- A. `--bond -26.93,1.6,41.6,0,0` — Bond a step in front of the garden ladder: walking forward should latch the climb (the walk-on grab at contact).
- B. The hut ladder base after a dismount (the 109i report's spot): standing still — the climb substate must not re-engage.
