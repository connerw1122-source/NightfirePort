The user's playtest 2026-10-15 (results id 1015-1738) said: "Even standing directly at a wall and throwing it at it, it
still bounced. Also, the numbers aren't working on it" (Satchel Charge 0x3b, 07000005, thrown at the low wall by
the start: --bond 105.97,24.87,110.12,-4.369,-0.166). The worker for 109z couldn't make it bounce headless, so the
headless setup differs from play somewhere: replay the exact window input path (the throw from the view model at
the real release frame, at a wall 1-2 m away, and while walking), find the bounce, and fix it from the stick arm
[M Bullet_CollisionHandler 0x12ce80:256-266]. The countdown digits: check the sim side feeding `phase.digits` and
the satchel list `frame_state` reads (kinds 4/5); the drawing is the main agent's, but the data must be there.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/satchel-stick2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T03:13Z

---
## Report (done, 2026-10-16T05:11Z)

Closed by the driver: the report is on `s/satchel-stick2` (docs/reports/satchel-stick2.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T05:29Z)

# satchel-stick2 — the satchel's wall stick and the countdown digits

Branch `s/satchel-stick2` (merged with origin/master 6bab6409). Task: playtest 1015-1738 — "Even standing
directly at a wall and throwing it at it, it still bounced. Also, the numbers aren't working on it" (Satchel
Charge 0x3b, 07000005, the low wall by the start, `--bond 105.97,24.87,110.12,-4.369,-0.166`). The 109z worker
could not bounce it headless, so the headless setup differs from play; find the bounce and fix it from the
stick arm [M Bullet_CollisionHandler 0x12ce80:256-266], and make the countdown digits' data correct.

## What's wrong (found, and fixed)

**1. The world hittest: the port runs a move-length ray where the game runs two forms.**
`Bullet_update` 0x12e7b0:425-465 builds the world HITTEST over the PRE-move position with the probe radius
`max(0.25, obj+0x8C)` (0x3e800000 floor; only the MGL family 0x2c..0x31/0x62/0x6d zeroes it, :427-429):

- move < radius·0.5 → **version 0x101**: a one-sided sphere of that radius AT THE CURRENT position —
  `Collide_SphereIntersect` 0x1ec940 → `Intersect_PointGeom` 0x1e92d8 (signed plane distance of the centre;
  faces in front by more than the radius skip; `Dist2Tri` 0x1dfca8's closest point wins; faces the centre is
  BEHIND skip — the best gate inits to 0.0 [M 0x1e92d8:103-109] and Bullet_update's flag build writes only
  0x20 into the HITTEST flag word [M 0x12e7b0:391], never bit 0x8);
- move ≥ radius·0.5 → **version 0x201**: a ray of `max(1.0, move)` [M :443-465] — up to 1 u of look-ahead.

The satchel's throw speed is 0.0667 u/tick [L ee_start_05.bin weapon_data 0x3b +0x20], so its whole 1-2 m
approach runs in the 0x101 sphere form: **the game parks the satchel the moment a face comes within 0.25 u**.
The port ran a ray of exactly the move (0.067 u) — a throw that passes just over the wall top, grazes along a
face, or is released right at the wall never CROSSES a triangle, so nothing catches it and the satchel flies
on (which reads as "it still bounced / didn't stick"). The headless tests all aimed dead-centre at a face 1+
u away, where the tiny ray always crosses — that is the play-vs-headless difference the 109z report could not
find.

**The fix** (`crates/nf-game/src/world.rs` + `grenade.rs`): `CollisionWorld::sphere_surface(centre, radius)`
— the 0x101 form transcript (one-sided, closest-point-on-triangle, min distance wins, the same
static+missile+dynamic triangle set `segment_surface` walks) — and `Grenade::update` now picks the game's two
forms for the sticky family this arm serves (0x37/0x3a/0x3b): sphere at the pre-move position when slow, the
`max(1.0, move)` ray when fast. The stick/bounce arms are unchanged (the stick parks at the hit point
+ normal·0.04 [M 0x12ce80:383-392]; for the sphere form the hit point IS Dist2Tri's closest point).
The hat 0x45 keeps its verified path; the bouncing family (0x34…) keeps the old move-length ray — the game
gives it the same two forms, but a 1 u look-ahead would shift every pinned frag bounce point [S backlog row].

**2. The digits: the sim always had them; the VIEW gate hid them until the stick.** `Bullet_DoTrails`' 0x3b
tail spawns the two digit objects EVERY tick in every state < 2 [M 0x12d890:418-553] — the numbers ride the
satchel from the throw. The sim writes `StuckGadget.digits` in flight too (the tail runs before the stuck
early-out, grenade.rs:264-282), but `frame_state`'s kinds-4/5 filter required `g.stuck` (scene.rs:6317) — a
satchel that hadn't stuck yet drew nothing ("the numbers aren't working" is downstream of the same
no-stick). Fixed: the filter now matches the game (`state < 2` only). In flight the phase's normal is the
spawn's [0,1,0] placeholder, so flight digits stand upright on the satchel [S — the game aligns them to the
bullet's own matrix; the main agent renders].

**3. The stick no longer blanks the countdown.** The stick arm replaced `StuckGadget`, resetting
`digits: None, last_qs: 0` — but the countdown latch (BU+0x100) and the digit objects live in BU, which the
stick does not touch [M 0x12ce80:255-266 writes only obj+0xF4/0x90/0x30]. The old reset opened a one-tick
digit gap exactly when the satchel parked (the kinds-4/5 slots went FAR on the stick tick). Fixed: the new
phase carries `last_qs`/`digits` from the flight phase.

## How it was verified

- **Failing-first sisters** (`nf-game/src/grenade.rs` mod tests, run against the pre-fix logic by reverting
  only the fix — both FAILED, then PASS with it): `satchel_flying_along_a_wall_within_the_probe_radius_sticks_
  at_once` (0.2 u off a wall, moving along it — the old ray NEVER sticks, the game sticks on the first
  update, parked at closest-point+0.04, wall's +z normal) and `satchel_descending_onto_the_floor_within_the_
  probe_radius_sticks_early` (0.2 u above the floor → parked 0.04 above it, +y normal, within 3 ticks).
- Real-path live tests (`crates/nf-view/tests/fix_satchel_stick_play_live.rs`, all through
  `load_weapons` → the panel give → `switch_to` → the fire pipeline → `step_bond`, the same release-frame
  path the window runs): the 5 existing 109z tests stay green; new: `satchel_pitch_fan_all_sticks_on_the_low_
  wall` (pitches −0.02…−0.28 at the user's spot — every throw sticks on the wall's top face, normal ≈ +y,
  none flies off), `satchel_walk_up_throw_parks_on_the_walls_near_side` (fire held + forward held through the
  windup — Bond advances ~4.5 u, the release lands 0.43 u from the face, the park is on the near side),
  `satchel_digits_are_present_in_flight` (digits Some while flying + `frame_state`'s kinds-4/5 carry non-FAR
  matrices in flight). 9/9 green.
- The stick family around it: fix_satchel_tripbomb_live 5/5, fix_grenade_release_live 4/4,
  fix_grenade_throw_live 4/4, fix_grenade_types_live 3/3 (+1 ignored), fix_remote_mine_play 3/3,
  nf-game lib 236/236 — all on the merged tree.
- Full suite (`tools/test/suite.py`, merged tree 87cb71f2): the 2-hour window did not fit all binaries —
  **89 completed, 88 passed, 1 failed** (`probe_pylon_render` — the sandbox's missing Vulkan ICD, the known
  environmental failure from previous sessions), 0 TIMEOUT/CRASH. The rebuild alone took ~6 min; the next
  window should run the suite to completion.

## What's left / candidates

1. The bouncing family (frag 0x34…) still uses the move-length ray — the game's 0x201 form gives it a 1 u
   look-ahead [M 0x12e7b0:443-465]. Applying it shifts every pinned frag bounce point; it needs its own
   pass over the grenade/impact tests.
2. `obj+0x8C` (the probe radius's other term) is not modelled — the port's bullets carry no obj, so the 0.25
   floor is the whole radius [S: `Bullet_init` 0x12c5a0 sets obj+0x8C from the record/mesh; a live read of a
   stuck satchel's obj+0x8C would pin it].
3. The pierce/roll arm (`Collide_Intersect`'s surface-byte 0x40/0x80 fold + the `Rand_Rand(0x3f)` roll,
   0x12ce80:149-158 + 0x1ebaa8:296-300) is still unported — a 0x40/0x80-flagged face pierces in the game
   where the port sticks. The spawn-wall's faces show no 0x40/0x80 in the tests' hits, so this is not the
   playtest's bounce, but it belongs to the same arm.
4. The digit pool instances inherit the main map's first placement's room (scene.rs:5334) while the game
   links the digit objects to the satchel's cel (`control_link_object_to_cel`); the Effect layer draws
   unculled in the port, so this should be invisible, but the render check below settles it (backlog L-583).

## Live checks for the main agent

1. **Rebuild + replay the user's exact repro** (07000005, `--give 0x3b,0x3a`, stand at
   `105.97,24.87,110.12`, yaw −4.369 pitch −0.166, throw at the low wall; also hug the wall and throw; also
   walk while throwing): expected — every satchel parks ON the wall/its top within ~0.5 s, the two-digit
   countdown visible from the throw (upright while flying, face-aligned once parked), beeps ~4/s.
2. `--bond 105.97,24.87,110.12,-4.369,-0.166` after a throw — the satchel parked on the wall's top face with
   its countdown digits; `--bond` hugging the wall closer (e.g. `106.5,24.87,110.1,-4.369,-0.3`) — the
   satchel parks on the near face, digits visible.
3. If a bounce is still visible in play: `gdbwatch.py` on the stuck bullet's obj+0xF4 (expect the write 3 at
   the stick, 0x12d1fc block) and obj+0x30/0x34 — that distinguishes a sim-side non-stick (the sphere form
   missing the face) from a render-side disagreement.
