# escort-01 — Mayhew's escort on 07000001 (queue 690)

Branch `s/escort-01` (from origin/master e416e5f0). Playtest 1012-1723: "mahew doesnt really
follow properly ... level transition door should be able to open without him walking all the way
to it".

## What's done

### 1. The follow distance and the catch-up — the game's own rule, replacing the stand-ins

The port's `AllyLead`/`AllyLeadWait` used a straight-line 12.0/10.0 stand-in [S]. This task ports
the game's actual machinery:

- **`DroneMove_ObjectMissionPathDistance` 0x153f50** (new `Drone::object_mission_path_distance`,
  exposed as the `mission_path_dist` field = the game's SM+0x20 scratch [M 0x15e698:60,
  0x15eda8:35, 0x15f120:31]): the signed route distance between the lead's current route node
  (+0x9f0, the port's `route.index` clamped) and the route node nearest Bond. The node scan ranks
  by squared 2D distance from obj+0x30 — the FIRST route node is always a candidate, later ones
  only while strictly closer (the 5.0 arg is only the initial cutoff) [M 0x153f50:89-127] — and
  qualifies a node with 3D ≤ 30 (DAT_002c7954 = 900.0), |dy| ≤ 4.0 (DAT_002c794c), the slope band
  (DAT_002c7948/44 — `Drone_PostLoad_Init` 0x136938:53-62 writes 4.0/30.0 and the 1.5·/2.0 +
  0.7854/0.8727 pair the port already carries), then the **boundary test only** (`AINetwork_
  BoundsTest`, gated on the boundary count DAT_002c8774) [M 0x153f50:100-121]. The port's new
  `Network::node_scan_ok` implements exactly this — no collision-world LOS: the game's scan sees
  through furniture, and `move_test`'s world-LOS deviation disqualified route nodes behind
  benches in testing. The sign: `AINetwork_GetRouteDistanceBetweenNodes` 0x156160 walks the route
  list from the lower to the higher index and returns +sum when Bond's node is AHEAD, −sum when
  BEHIND [M 0x156160:47-53].
- **AllyLead 0x15e698 case 3** in the game's own order: armed hostile within 10 → hide (the
  FindRunToPoint→GoToGoalPosition half stays the documented hide stand-in); the route (done →
  Done, MOVING → RUN anim); Bond in the π/6 cone (0x3f060a92 = 0.5236, `BondWithinAngle`
  0x154f98 — |yaw difference| ≤ arg) and within 3.0 → PlayerInWay; then the path bands: **path
  < −9.0 → AllyLeadWait** (Bond over 9 route-units behind), **path == 0.0 (no node within the
  scan of Bond) with Bond ≥ 5.0 straight-line → AllyLeadWait**. The (15,25) Drone+0x9F8 band →
  BondCombat is dead in the game: no decomp writer for +0x9F8 exists anywhere (both readers
  0x15e698:80/83; docs/fields/hitdata-spawner-ally.md §3), so `alert ≤ 15` always keeps the lead.
- **AllyLeadWait 0x15eda8 case 3**: armed hostile → hide; **path ≤ −7.0 stays**; path == 0.0 with
  Bond ≥ 5.0 straight-line stays; anything else resumes the lead — the game's −9.0 enter / −7.0
  resume hysteresis.
- **PlayerInWay 0x15ea28 case 3**: resume the lead when Bond leaves the π/3 stay band
  (0x3f860a92 = 1.0472) × 3.5 — replacing the port's distance-only stand-in.
- **BondCombat 0x15f120 case 3**: path < 0 stays; `DAT_00298f80 != 0` stays (no writer — reads 0);
  else Lead.

### 2. The waiting at doors

Already the game's model from the 663/fix-npc-doors work (route door nodes → `DSTATE_OpenDoor`
0x81, the Mayhew talk lines, the door-node pairs) — unchanged and re-verified by the tests. The
new bands add the game's OWN waiting: the escort parks 9 route-units ahead of a lagging Bond and
resumes at 7 — around corners, through doorways and down the stairwell, where the old
straight-line check never fired (that was the "doesnt really follow properly" symptom).

### 3. The exit door's condition — verdict, no port change needed

- The exit doorway (gallery_secretdoor pair #224/#225, group 10, keep-open) has **no channels and
  no Mayhew condition at all** — Bond's E opens it with Mayhew anywhere [D docs/atlas/door.md +
  ee_start_07000001; M Player_Activate 0x1a5170 case 0x1c → Door_Activate]. It never auto-closes.
- The transition fires from **Touch #180 (out 0x10, mask 0x18, filter player 0xE + skin 0x4F)**:
  `Trigger_Hit` 0x1b1fc8 requires **both Bond and the skin-0x4F contact inside the volume
  simultaneously** — a presence test re-evaluated every tick, NOT a radius, NOT an
  AllyLeadDone/route-end check [M 0x1b1fc8; L ee_start_07000001]. LOAD_LEVEL #18 (gate 0x10 →
  0x07000002) [D], `Trigger_Update` case 10 0x1b1a78; the cancel channel 0x60 (Mayhew's death)
  blocks it. `AllyLeadDone` 0x15f2e8 writes no channels — finishing the route raises nothing.
- The port already models this exactly (triggers.rs mask-0x18 filter + LOAD_LEVEL gate) — the
  playtest's "should open without him" is the door half (Bond's E — already works) plus the plate
  being a two-body AND (the game's own design, already proven). See the sister
  `the_plate_refuses_bond_alone` (green).

## Verification

- **New test `crates/nf-view/tests/escort_follow_07000001_live.rs`** (the task's headless walk):
  real-input only, Bond at the analog half-press (forward 0.55 = walking; full press only past 10
  to close a big gap), from the guard-clear meet-up through the shoji, the interior doors, the
  courtyard door (Bond E-opens it — the game's other activator, with fix_mission's closed/closing
  patience gate) and down the stairwell to the plate. **6587 in-walk band checks** (every Lead
  tick ⇒ path ≥ −9.0; every Wait tick ⇒ path ≤ −7.0 or 0.0) with **zero violations**, and the
  escort + Bond on the plate fire ch 0x10 → **level_exit 0x07000002**. GREEN (~9 s).
- `walkreal_07000001_live`: **2/2 green** (the required-green real-input walk, unchanged
  choreography).
- `nf-game --lib`: **237 passed** (includes the nav/Route unit tests).
- Crates run before the push: nf-game (lib), nf-view (the three binaries above). nf-formats/
  nf-audio untouched by this branch (no source delta), so their binaries were not re-run; the
  full-suite fast runner was started twice but the sandbox kills detached background jobs
  between tool calls, and the disk filled once (target cleaned, CARGO_INCREMENTAL=0) — the four
  affected live binaries were run in the foreground instead.

## What's left

1. **`fix_mission_live::the_07000001_exit_works_end_to_end` now deadlocks (the one red).** Its
   Bond never follows — he stands at the spawn and teleports only to E-open the courtyard door.
   Under the GAME'S OWN bands a motionless player parks the escort 9 route-units out forever
   (path < −9 with Bond's node fixed) — the game would do the same; the playtest's "follow
   properly" presumes the player walks. The fix is choreography, not logic: give that test's
   Bond the walking-speed follow the new test models (its E-door patience, the
   blocked-line/floor-gap nav closes and the plate walk reuse as-is). Proposed: replace the test
   inner loop's standing with the new test's follower block, keeping the asserts byte-identical.
2. The stall diagnostics surfaced one candidate port delta worth a live check: with Bond standing
   south of the upper-hall boundary fence strip (db 2.6, inside 5.0), the port's scan still found
   qualifying nodes 9.8 route-units back (pd −9.8 → the escort waits), where the game's scan —
   if its boundary geometry excludes those segments — would return 0.0 and the pd == 0.0 &&
   Bond-within-5.0 branch would keep/resume the lead. Live checks for the main agent:
   - At the 07000001 upper hall (Bond ~(3.4, 6.5, 3.9), Mayhew parked ~(3.7, 6.3, 6.5)):
     watch `DroneMove_ObjectMissionPathDistance` 0x153f50's return (SM+0x20) for the escort
     drone — expect either ≈ 0.0 (my far-node qualification is too permissive) or ≈ −9.8
     (matches the port; the game also waits there).
   - Watch `Drone+0x9F8` (f32) for the escort during any combat — if it ever leaves 0, the
     (15,25) BondCombat band is live and the port should model its writer.
3. The FindRunToPoint (0x15168) half of the armed-hostile branches (run to a point →
   GoToGoalPosition → Hide) remains the documented hide-only stand-in from fix-actors.

## Report notes

- Committed and pushed in two commits (WORKLOG start; the port + test + this work entry). The
  report lands as the third commit.
- No game bytes, tokens, URLs or probe dumps were committed; `git status --short` checked before
  each commit.
