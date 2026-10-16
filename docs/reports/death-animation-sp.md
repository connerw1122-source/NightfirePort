# death-animation-sp — Bond's death: the death script, the frozen death camera, and the wait

Playtest 1015-0046 (restart after dying): "good, missing death animation".

## What the game does (traced)

The death sequence, end to end:

1. **The death starts** — `Player_CheckForDeath` 0x190810 on health ≤ 0 [M :34-97]: ch 0x5e,
   `Music_Event(6,1)`, `Player_ClearInertia`, `Player_WeaponNone` (the fists), the death marker
   (`obj+0xff = 0x12`), the dead-tick stamp (`obj+0xec`), **cam mode 0** (`Player_SetCamMode(BL, 0)`
   [M :64]), health 0, `obj+0xf4 = 2`, the death sound 0x89, then `Player_ChangeSubState(obj, 0xd)`
   — 0xe when ch 0x60's bit 0x100 marks a vehicle [M :89-94].
2. **The death script** — `Player_ChangeSubState` case 0xd/0xe share one arm [M 0x190630:64-73]:
   `AnimSetDeleteAll` + `AnimScriptAdd(obj, 0x6000151)`, or 0x60008e0 only when the gravity global
   is exactly 0.0 (the zero-g level; retail keeps −9.4, so 0x6000151 is the ground death —
   fix-world-gravity). The stops the sounds 0x12/0x3d/0xf0 [M 0x190630:83-85].
3. **The dead branch** — `Player_Update` 0x193600 with `obj+0xf4` 2/3 runs ONLY
   `AnimObjectUpdate` + `Player_HandleDeath` [M :316-319]. No substate arm, no `Player_MonitorAir`,
   no collision pass. **Bond's eye (`obj+0x30`) freezes where he died.** The anim system moves no
   object position: `AnimObjectUpdate` 0x1befd8, `AnimSeqTick` 0x1c28d0 and `AnimFrameResolve`
   0x1c35f8 contain no `obj+0x30` store — the collapsing skeleton never moves the object.
4. **The camera** — cam mode 0 → `Player_PositionCamera` 0x18f5d8 case 0 [M :96-134]:
   `Camera_SetToPlayer` 0x1bc048 [M :24-27] = body matrix × pitch, position from
   `Player_GetHeadPos` 0x195aa0 [M :39-51] = `obj+0x30/34/38 + BL+0x908 − BL+0x90c`; the dead arm
   zeroes BL+0x90c [M 0x18f5d8:130-132] and BL+0x908 lerps to 0 on land [M :113-128]. So the death
   camera is a **frozen first-person eye at the death spot** — it does NOT follow the body down.
   The visible "death animation" in first person is the view model posed from the body's
   collapsing bones (the drawing side).
5. **The script's run** — the clock starts at 1.0 (`AnimScriptInit` 0x1c1600 sets +0x90 = 1.0 on
   `AnimScriptAdd`) and advances 2.0 frames per dead tick [L the PCSX2 death trace: +0x90 reads
   12/32/52 at dead frames 8/18/28]; `AnimScriptEnd` 0x1c1930 stops it at the length. On 05 the
   script 0x06000151 is 297 frames with one opcode-0 window (1, 297, seq 0x040000A3 — the collapse
   pose) and op-4 effect commands at frames 5 (kind 5), 8 (kind 4), 152 (kind 4), 278 (kind 6, the
   body) [L the parsed archive; `AnimProcessScriptCmds` 0x1c1b38 case 0 starts the sequence, case 4
   fires the footstep/body effects].
6. **The wait** — `Player_HandleDeath` 0x190bb0 [M :20-36] (SP): the first dead tick enables the
   blood pane (pane 12); every later one raises ch 0x62 once the pane's state is non-zero — the red
   fade covers at 448 ticks [L viewer.h 448 × 512-wide templates] — or on action 25 (✕), gated on
   `PlrStat_OkToUpdate` (mission status < 2 [M 0x1993e0]) and `Menu_PlayerHasQuitOrRestarted`.
   `Mission_Update` 0x182c60 then walks 1 → 3 → 7 → 4 and opens the 0x40000042 fail page [M
   :21-81]. `fix_death_restart_live` already pins that flow end to end.

## What master already did right (now pinned, not changed)

The whole death **flow** was already correct on master, and `fix_death_restart_live` pinned it:
the death-start effects, the blood pane, the ✕ skip, the 448-tick fade → ch 0x62 → the fail menu →
the warm restart on the entry checkpoint. The **script playback** was already right too: `death.rs`
runs the 2.0/tick clock with the 1.0 start and the length cap, and fires the op-4 effects at the
game's frames with the surface's footstep/body sounds. This task pins that side with
`death_anim_sp_live::death_script_runs_the_games_frames_then_the_fail_page` (the script's shape
straight from the 05 archive — length 297, the seq window, the four op-4 frames/kinds — plus the
clock, the events, the 448-tick beat and the fail page), so a future regression fails a named test.

## What was missing — and the fix

**The dead branch's movement freeze.** The port kept ticking Bond's walk physics while dead
(`run()` only zeroed the input), so the dead body still integrated gravity/friction/collision:
an airborne death kept **falling** (the probe: −3.06 units over ~55 ticks on 05) and the eye slid
on slopes. The game's dead branch runs nothing — the eye is bit-frozen from the first dead tick,
and the camera (`Player_GetHeadPos` reads `obj+0x30`) freezes with it. That motion (the view
drifting away from the death spot while the red fade runs) is the flow half of the "missing death
animation" symptom.

The fix (nf-view `scene.rs`, one gate): skip `bond.tick_ctx` while `drones.death.active` — the
game's dead branch [M 0x193600:316-319]. The death tick itself, the mission and the drones' world
keep running (they run in the game too).

Verified by `death_anim_sp_live::dead_bond_holds_the_death_camera_spot`: grounded and airborne,
the eye is bit-equal across 60 dead ticks and cam mode stands at 0. **Fail-first**: with the gate
reverted the airborne arm fails (`a dead Bond in the air freezes there — the pre-fix port fell
3.06 units`, recorded on this tree before the fix).

## The drawing side (for the main agent's render task)

The sim exposes everything the renderer needs:

- The death pose: `nf_game::anim::script_pose(&sim.scripts, &sim.seqs, skel, skin,
  0x0600_0151, sim.drones.death.script_time)` — seq 0x040000A3, frame = `script_time − 1`
  (`AnimSeqTick` 0x1c28d0: seq.frame = script.time − start + 1.0); the clock stops at 297.
  `death_anim_sp_live` asserts a skin/skeleton resolves the pose.
- While dead the game does NOT tick the weapon anim object (the dead branch runs
  `AnimObjectUpdate` on the player object only), so the first-person arms freeze mid-pose. The
  port's view chain (`view_script`/`view_time`) keeps idling — left alone here (shared code); the
  render task may want to hold it at the death frame.
- Bond's own body model draws in other views only (`View_SetDrawInOtherViewsOnly` [M
  0x18f5d8:98]) — in first person the collapse is visible only through the view model's bones.

## Live checks for the main agent

1. **The frozen eye** — gdbwatch WRITE-watch on Bond's `obj+0x30/0x34/0x38` (the player object
   position) through an SP death on 05: expect **no writes after the death tick** (the game's dead
   branch integrates nothing). The port now matches bit-for-bit.
2. **The frozen view** — two `--bond x,y,z,yaw,pitch` death screenshots ~2 s apart (same death
   spot): the camera must not move between them, only the red fade grows. A moving backdrop
   between the shots means the freeze regressed.
3. **The arms freeze** — a death screenshot mid-fade: the view model holds one pose (no idle
   sway). The port's view chain still idles — if the render task wants the game's freeze, that is
   the one remaining sim-side nit.
4. **The collapse pose** (optional, visible only off-view): the body posed from
   `script_pose(0x06000151)` at `death.script_time` should match a PCSX2 screenshot of the same
   dead tick's skeleton (the collapse ends face-down by frame ~297).

## What I ran

- `cargo test --offline --release -p nf-view --test death_anim_sp_live` — 2/2 (and the fail-first
  arm of `dead_bond_holds_the_death_camera_spot` recorded the pre-fix failure before the fix:
  `a dead Bond in the air freezes there — the pre-fix port fell 3.06 units`).
- The crates my tests touch: nf-view (the sim/flow change and the live tests) and nf-game
  (the death module the tests read) — both inside the full runner below, `--test-threads=1`.
- The full suite (`tools/test/suite.py`, 436 test binaries across nf-game + nf-formats + nf-audio
  + nf-view) on this branch **merged with origin/master** (master had not moved; the tree the
  suite ran is the pushed code): **`1896 passed, 6 failed, 72 ignored (46 min)`**. The 6 failures
  are the allowed set on this sandbox: the 3 `disc_golden` tests (slim ISO) and the 3 pre-existing
  pylon render probes (`fix_0c_pylon_live` ×2, `probe_pylon_render` — no GPU here).

## What's left

- The drawing side (the collapse pose, the arms freeze, the red fade's final look) — the main
  agent's render task, inputs listed above.
- Dying on a moving platform: the game's object carry may still move a dead Bond
  (`Player_StandAtNewPosition` from the object's update); the port's carry path was not gated.
  Edge case, not modelled here.
- The view-model freeze (item 3) if the render task wants it.
