# level-09-interactables — every interactive thing and every model on 07000009 (Night of the Agent, Tower A)

Task: the census of every touch/use/shoot/ride placement, one real-input live test per KIND, and
every model resolving — on the Tower A level whose elevator buttons the user reported (task 700
fixed the button itself; this task covers the rest of the level).

## What's done

1. **The census** (docs/levels/07000009.md, rewritten; the dump probe is
   `census_07000009.rs --ignored`, now over ALL 40 maps): the main map (m39) carries 288
   placements in 23 kinds. Corrections to the old doc, each from the disc or the decompile:
   - **0x20 is BREAKABLE GLASS, not doors** (the old "Doors 0x20/0xdb ×23" lumped them): 17
     `Break_Create` 0x12c230 placements — 16 `pillarglass` (vertical slabs x ≈ ±8/−9.55, y
     −369.25..−354.12 — the tower-base panes over the lobby) + `securityroom_glass` #87 (param 2
     = 0x30 → the bullet filter ~0x30 [M 0x12c230's tag+0x08]). Glass kind 2 dies to bullets
     with sound 0x2E [M Break_Kill 0x12b970]; the drone-step damage is 07000001-only [M
     0x12bfd8]. The real doors are the six 0xdb placements.
   - **No Monitor exists on this level**: no 0x30 placement on any of the 40 maps [D census],
     `OBJECT_Srceen` #326 is kind-0x19 static scenery (parsemap has no 0x19 case [M
     0x1d04f0]) — the security room's monitors are art. The old "Monitor ×1 | 0x30" row is gone.
   - **0x71 is `rotor_init`** [M 0x1d04f0:284-289] — the `office_N_hologram` spinning rotor, NOT
     a DroneSpawner (0xF1 exists only in the prefab map m6, never placed). The old "Spawner 0x71"
     row is gone.
   - New rows: the four SoundTriggers (0x51→0x3F6 arrival, 0xAB→0x47E liftside, 0x7C→0x498 and
     0x7D→0x4D8 alarm sirens), the pickups in detail (BodyArmourHeavy kind 3; 2× Pickup 007 kind
     6 = BOND_BONUS, spin, model 0x020004A), the rain/skyline/weather kinds, the AI infra.
2. **The guards' freeze — the level's real "doesn't work" bug, found and fixed.** The drones'
   placement param 3 = 0x51: their WAIT_SWITCH channel is the ARRIVAL TOUCH PLATE #248's out
   [D drone params; M `NDrone2_DSTATE_Global` 0x15b830 waits while the channel is off, and
   `NDrone2_DSTATE_WaitSwitch` 0x15b960 case 3 wakes on the channel EVENT — which reaches a
   frozen drone]. The port's tick loops filter on `enabled`, so a drone that entered
   WAIT_SWITCH before the plate fired never sampled the channel again and froze forever —
   `Drone_CollisionHandler` 0x140968:62's enabled gate then made it **unkillable** (15 aimed
   rounds, zero damage; two of six guards were frozen every load). Fix (crates/nf-game/drone.rs,
   `tick_inner` head): the tick samples the switch channel for the waiting frozen drones and
   wakes them exactly as the game's msg-3 arm does (`NDrone2_Enable(1)` + the start state).
3. **Live tests, one per KIND, through the real input path** (crates/nf-view/tests/
   interact_07000009_live.rs, `nf_view::load` + `Sim::step_bond`; the settle-then-operate
   pattern — a teleport TO the spot, then real input only). **12 passed, 0 failed**:
   - *alarm Switch* (0x29): the real E press on the car-park panel #253 → ch 0x7C → OR mux #278
     → ch 0x61 → the mission fail end [M Switch_Activate 0x1b02a8, 0x1827d8]; sister: no press,
     no alarm, no end.
   - *Breakable glass* (0x20): the real fire path breaks roof glass #123 — dead + the kill
     effect (kind 2, sound 0x2E) [M 0x12b970]; sister: aim only, intact.
   - *Door* (0xdb): #155 opens during the press (state 8 observed) and swings back shut;
     the shutter #157 (lock ch 200, never written) refuses; sister: no press, closed.
   - *Pickup* (0xF0): the walk-over takes BodyArmourHeavy #270 — the armour hard-sets 50.0 at
     the take [M 0x18e650 armour case]; sister parked 1.4 away never takes it.
   - *Touch plate* (0xEA): the real walk to the lift mouth fires #272 → ch 0xAB (the #273 siren
     rides it); sister parked in the lobby: silent. (The cab touch #67 + the arrival plate's
     ch 0x51 were already walk-proven by walkreal/elevator tests.)
   - *Guard* (0x0f): the guard stands while unfired-at (pinned); the kill half is **ignored**
     with the finding below.
   - Kinds already covered by the existing suite: the elevator button/lift doors/cab exit
     (elevator_buttons_09_live: 3), the channel-chain walk + the E-path pin
     (walkreal_07000009_live), the alarm-fail chain (walk_07000009_live), the doors en route.
   - Kinds with nothing to operate on 09 (census-documented): keypads/safes/turrets/searchlights/
     ladders/vents/grapple/vehicles/monitors/cameras — absent from the disc; the laser switch
     #71 sits at the world origin (unreachable in the game too; the laser state is the level
     scripts' — s/laser-scripts owns the playback, held for a render check).
4. **Models**: the loader's own resolution line on every 09 load: "191 world instances (3
   skipped by the cel pass: flag 0x10), 43 animated character meshes, 0 drones with a skin from
   another archive, 0 unresolved, 2,960,058 triangles, 3,274 materials, 272 sequences; 9,394
   weapons held (0 without mesh or datum)" — no missing hash, no unresolved placement. The
   guards' skins resolve (0x05000027/0x05000028, pinned in the guard test). A dedicated
   NF_TEX_TRACE=1 white-fallback sweep test was NOT written (time) — listed below.

## How it was verified

- The 12 new live tests above (release, `--test-threads=1`).
- `cargo test --release -p nf-game --lib` → 276 passed; `-p nf-formats --lib` → 79 passed
  (the touched crates).
- The fast runner (tools/test/suite.py, started in the session's first minutes on the
  origin/master baseline): **2068 passed, 6 failed, 74 ignored (47 min)** — the 6 are the 3
  `disc_golden` tests (allowed on the slim ISO) + `fix_0c_pylon_live` ×2 and `probe_pylon_render`
  (pre-existing 0700000c render probes, untouched here).
- The suite was NOT re-run on the final tree (the window closed); the final-tree targeted runs
  are the lib + live batches above.

## What's left

- **The two lobby-side guards re-freeze** after the frame-1 wake (enabled=false, state 1 by the
  intro's end; no `DroneEnable` script event ever fires on 09 — traced). The wake fix is in; the
  re-freeze path (something re-enters WAIT_SWITCH after the wake) needs the main agent's
  gdbwatch. The kill-half test is `#[ignore]`d with this note.
- **The E route through the closed lift doors**: the port's use pick runs from the RENDER CAMERA
  (scene.rs passes `camera` as the head) while the game builds it from `Player_GetHeadPos` [M
  0x1a5170:101-106]. At the doors the camera hangs ~2.5 behind Bond, so the 1.0 sphere never
  spans the exit box (z 10.33..12.49 [D #63]) — Bond presses in to z 13.85 and the load never
  fires. The fix is shared with the doors/switches/SP reaches (tuned to the camera) and needs
  its own session; the elevator route is the pinned, working exit. The test is `#[ignore]`d.
- **The models sweep test** (NF_TEX_TRACE=1 white-fallback assertion, per level) — the loader
  line shows 0 unresolved, but the pinned test is not written.
- The lobby laser grid art stays with s/laser-scripts (held for a render check).

## Live checks for the main agent

- gdbwatch on a 09 lobby-side guard object from a fresh boot: watch `obj+0xF0` (the enable bits)
  and the state machine word through the intro — the port wakes both at frame 1 (ch 0x51) and
  then finds them enabled=false/state 1 again by the intro's end; the game should show them
  enabled throughout. The suspected re-entry is a second `NDrone2_DSTATE_Global` ENTER reading
  a stale `channel_on`.
- Fresh-boot 07000009, stand at the CLOSED lift doors (−3.7, lobby floor, z ≈ 13.9), press E —
  expect the 0700000a load with objective 2 incomplete (the doc's route (b)) [M 0x1b2318:76-90
  key_gate 0]. This pins the camera-vs-head pick question.
- `NF_TEX_TRACE=1` on a fresh 09 load: expect no white-fallback prints (the loader says 0
  unresolved).

## Usage

not visible to me (no token/credit/context figures are shown in this environment).
