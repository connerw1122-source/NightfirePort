# fix-ai-remainder — report

Branch `s/fix-ai-remainder` (from origin/master 78d73e0). Sandbox window 12:47–14:44 UTC; the
session covered three of the eight task items in full and scoped the rest.

## Done

### 1. SpecialDeath scripts (G-026) — already on master, verified
`NDrone2_DefaultInit`'s placement param 13 → `Drone+0xBD4` and the SpecialDeath 0x46 state machine
(0x163c68: the at-spot test within 1.0 in 2D and 90° facing, the script as anim state 0x40 → 0xBF →
Dead, the free root motion, `SetDeathChannel` 0x13e660) were already ported in `drone.rs
handle_death` with the NF_ISO test `tower_guards_fall_out_of_the_towers` (the 07000005 snipers end
9.5 / 8.1 lower). No code change needed; GAPS G-026 now records it as done.

### 2. Head tracking and the aim bone (G-026) — ported
Evidence: `psiBuildMatrixPalette` 0x111498 calls `AnimObjectHeadTrack` 0x1bf618 on bone 5 while the
0x30 track flags are set and `AnimObjectAimAt` 0x1bfe58 on bone 2 while 0x180; `NDrone2_DoTracking`
0x1479d8 / `NDrone2_DoHeadTracking` 0x1472c8 drive them; `AnimObjectSetHeadTracking` 0x1c6748 sets
maxdist² 9.0, speed, flags 0x10|0x40, mode 5.
- `Drone::head_yaw/head_pitch/tracking/track_recheck`: track the opponent from the FIRST sight pass
  (the port gates on `los` +0x228 & 2, not the reaction-gated `seen` — the pre-attack window);
  re-decided on `+0x358` every (Rand(5)+2) ticks with the same-target early return before the draw;
  one Rand(10) when tracking stops; clamps yaw ±1.1 (0 beyond 2.6), pitch [-1.0, +0.6]; speed 0.14
  when alerted (+0x50c > 0.66) else 0.07; while the aim flag is up the head eases back (HeadTrack's
  bVar5 == 2 path) — the torso aim owns the pose.
- `Drone::aim_pitch`: `AnimObjectAimAt`'s second angle (mode-0 clamps ±0.9, 0x3f666666/0xbf666666),
  smoothed at SetAiming's 0.2 (0x1c6878) — before this the spine only yawned.
- `object_pose`: the head rotation applies to bone 5 ALONE (the game rewrites just that quaternion),
  the aim pitch to bone 2 and below; composed R_yaw · R_pitch premultiplied (the game's
  q_yaw ⊗ q_pitch ⊗ q_bone), pivots at the bones. *Approximations* (documented in the code): the
  port rotates about the model up / the aim direction's horizontal right instead of the animated
  bone axes; the tracked point stands in Bond's eye (the game tracks the target's `sAnimObject+0x94`
  bone).
- Tests (`crates/nf-game/tests/fix_ai_remainder_live.rs`, NF_ISO, seed 2, the real `Drones::tick`):
  `head_tracks_bond_until_the_aim_takes_over` (max pre-attack |head_yaw| > 0.05; the posed bone-5
  forward delta equals head_yaw and points at Bond; after ATTACK the torso aim engages and the head
  eases back) and `the_aim_pitch_follows_bond_across_heights` (Bond at y 30.5/28.5 → aim_pitch
  > 0.15 up; the downward case is not placeable in that yard — the floor). Both fail on master
  (the fields and the pose rotation did not exist).

### 3. Corpse fading in the renderer (G-026) — ported
`NDrone2_FadeOut` 0x172dd8 / `DroneFunc_DeadDrone` 0x14a298's sim side was already ported (Fade 0x48
0.6/tick, FadeFast 8.0, removal at 0); the renderer never consumed it. `Scene::posed_vertices` now
multiplies the plain character vertices' alpha byte by the drone's alpha (obj+0x106 — pickups,
attachments and searchlight/door parts are their own objects and stay untouched). Test
`crates/nf-view/tests/fix_ai_remainder_live.rs`: a guard in the Fade state with the decayed alpha
draws scaled vertices (compared against the skin's own per-vertex alpha), a full-alpha guard stays
at the skin alpha. Sister assertion included.

## How it was verified
- `cargo test --offline --release -p nf-game --test fix_ai_remainder_live -- --test-threads=1` — 2 passed.
- `cargo test --offline --release -p nf-view --test fix_ai_remainder_live -- --test-threads=1` — 1 passed.
- `cargo check --offline -p nf-game -p nf-view` clean (pre-existing warnings only).
- **The full 4-crate suite did not fit the sandbox window** (it exceeds the tool timeout on this
  machine per earlier WORKLOG notes). The touched crates' slice is green; no other code was touched.

## Reproduce-before-fix notes
- Head tracking: reproduced directly (the pre-attack window with Bond visible produced no head
  rotation on master — the fields/rotation did not exist; the test's sim-side and pose-side
  assertions both pin the new behaviour).
- Corpse fade end-to-end: **not reproduced through Bond's gun** — exactly what I tried: Bond
  teleported to the 07000005 sight spot 6 units from the guard at (61.34, 99.50), aim re-tracked
  every tick, pulsed fire (the P2K is one shot per press, `Player_WeaponFiring` 0x1a5460), 66 shots
  over 600 ticks — the guard fought (states 0x73/0x74 step-aims with the crosshair on him) but his
  health never moved (10.00 throughout), so the headless scene's player→drone bullet damage path
  never lands in this setup. The fix is therefore tested at the state the death chain ends in (the
  Fade state's own decay machine ran through `Drones::tick`), not through a live kill. Follow-up:
  check the scene-level bullet→drone pick (the same path the playtests exercise) before trusting
  headless gunfights.

## Live checks for the main agent
1. Render `--bond 67.31,26.5,99.47,-0.9,0` on 07000005 before the guard notices: his head should
   turn toward Bond as Bond approaches (the anim pose ± the tracked yaw, ease-in at 0.07/s of the
   gap). Pre-fix: heads locked to the animation.
2. Render a guard aiming up/down a staircase (e.g. the truck courtyard slope): the spine should
   pitch (mode-0 ±0.9). Pre-fix: flat aim.
3. Kill a guard and watch 5 s after the fall: the body fades over ~7 s (0.6/tick) and disappears;
   a killed-from-view corpse beyond 3 corpses fades fast. Pre-fix: the body popped out after the
   300-tick timer.
4. gdbwatch on `obj+0x106` of a dying guard (address = the drone obj + 0x106): expect 255 until the
   Dead timer, then −0.6/tick (−8/tick FadeFast). And the head-track Rand churn: watch the shared
   RNG state SEED_X/SEED_Y (0x30d0a0/0x30d0a4) while a guard eases his head back during aim — the
   decompile reads like one Rand(10) per tick while `sAnimObject+0x58` keeps 0x20; the port draws
   once per stop. If live shows the churn, the port's stream shifts again.

## Not done (in the task's order)
4. Bond's 16 hit boxes (guards' bullets still test the cylinder — `bullet.rs::PlayerVolume`, the
   documented deviation; the game tests `HumanCollisionBones` 0x2c6be8 on Bond's skeleton via
   `Collide_Jointy` 0x1ec180) with the per-location damage multipliers (the drone-side multipliers
   are ported; Bond's own per-part table needs his skeleton in the viewer).
5. Reload animation/clip/sound for every drone weapon (only the sniper 0x98/0x52 and the
   undercover paths exist).
6. Weapon swaps by animation events (G-023: `DroneAnim_EventFunc` 0x13b750 datum 0x28/100) and
   dropped weapons matching the held one.
7. Missing drone skins (G-021, 33 placements): `CommonAnimFile` (LoaderLoad type 4) +
   `AnimSleeveGetEntity`.
8. Drone modes 9 / 0x12 / 0x1E in the alarm and the `Drone_InitComms` cover-node order (G-026).
Also open from item 2: the ally-glance half of `DoHeadTrackingBehaviour` 0x147578 (idle guards
glancing at nearby peers within 30 units, the 1-in-4 re-pick Rand(4)) — the opponent-track half is
in.
