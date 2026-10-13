# walkreal-a — the castle and Mayhew missions walked with the player's own movement

Status: PARTIAL (parked). The driver reassigned this session to `fix-gadgets` mid-task. Levels 01–04
are walked end to end with real input (tests in the tree, all green); 05 is scaffolded and parked;
06–08 were not started. Everything below is committed on `s/walkreal-a`.

## Per level

| level | walkable end to end | notes |
|---|---|---|
| 07000001 | yes (already at session start) | the contact escort still stalls at the stairwell head (documented in the test); Bond walks onto the plate himself |
| 07000002 | yes | 19950 ticks, no falls: spawn → ladder #114 → the garden → the south hall → the west wing stair lane → the bunker door → the doorway sensor → the bunker room → the Load Point (refuses without ch 0x4C, loads 07000003 with it) |
| 07000003 | yes | 14353 ticks, no falls: the spawn reveal (ch 0x4d) → the mid courtyard → the east lawn → the Load Point (refuses without ch 0x52, loads 07000004 with it) |
| 07000004 | yes | the intro/story scenes settle (skipped via the game's own skip action), the story places Bond beside the target, the walk shoots him: ch 0x54 → the OUT copy to ch 0x64. 227 ticks after the settle |
| 07000005 | scaffolded, `#[ignore]` | the truck route wedges at the gatehouse ledge (the hand-off volume wants Bond ON the floor ~1.1 up; the walk cannot mount it yet — the ramp lane at the ch-0x25 plate (58.7, 27.1, 110.7) is the next lead). The cable route's west anchors are untested. fix_truck_live's scripted ride (teleport start) stays green |
| 07000006–08 | not started | — |

## Stuck points found and fixed (each with the failing test first)

- **The wire grab fired on walk-in** (`walkreal_07000002_live` failed: Bond swallowed by the
  courtyard wire, substate 6, inside the secret bunker). The game grabs a wire on the ACTION KEY
  only — `Player_HandleJump` 0x1916b0:126-147 (the action edge, the icon context, the nearest
  class-0x2C object); the walking pick pass cannot hit a wire at all (`Collide_PickObj` 0x1eae80's
  world-sphere branch runs only for HITTEST versions 0x200/0x201 and a wire has no glist). The port
  now grabs on the use-key edge + the nearest wire, cel-gated (`Collide_Pick` 0x1eb320 tests only
  the straddled cels' object lists; wires own one cel per `parsemap_create_dynamic_objects`
  0x1d04f0 — the port assigns it spatially [S]). `--bond -47.3,7.46,-24.3` (the bunker): before,
  Bond hangs on the cable; after, he walks.
- **Unported AI states swallowed the player's bullets** (`walkreal_07000004_live`: the seated target
  took 35 point-blank rounds, health never moved, impacts registered). `Drone_SM_RouteMsgDCV` routes
  an unhandled message to GLOBAL and `DroneFunc_HandleImpact` 0x1469b8 IS the base DSTATE — its case
  8 (0x1469b8:134-166) runs `NDrone2_BulletImpact` with the resume 0 unconditionally. The port's
  `impact_args` had no GLOBAL arm, so every state the port has no DSTATE for (the target's 0xb2)
  dropped the bullet. Fixed: the GLOBAL arm (0, false).
- **The seated target had no hit boxes** (same test). `script_pose`'s skeleton filter
  (`seq.skeleton == skin.skeleton`) rejected the target's pose: skin 0x0500001e (skeleton 8) plays
  0x0400028a whose record byte is skeleton 0 — a shared human track. The pose machinery
  (`local_pose_with`) reads the channels by the SKEL's translation mask, never by `seq.skeleton`.
  Relaxed with the channel-count guard [S — the game's use of the seq's +0x3c byte is unverified].
  `--bond -3.87,5.44,30.3` facing the target: the chest/head boxes now sit where the pose is.
- **The guard-clear loop shot from behind walls** (`walkreal_07000001_live` rounds 5–11: guards 3/15
  took no damage for 900+ ticks at 3.2 — the eye→torso ray hit a wall at t 0.18). The approach gate
  is now LOS, not just distance.
- **The walker**: cut-scene skip through the game's own skip action (`Scripts::skip_request` =
  `Input_Action(-1, 0x19, 1)` [M nis.rs; fix-dev T1]); WireRide pulses E; the stuck-3 mitigation
  jumps WITH forward (the gatehouse ledge).

## Experiments tried and reverted (documented at the toucher construction in drone.rs)

The game-accurate trigger toucher (Player_Collision 0x194df0's capsule) fired the floor plates the
stand-in misses (07000002's bunker 0x80; 07000005's ch-0x45 reveal at feet+0.47) but broke two
shipped behaviours: 07000005's truck chain stalled (the hand-off ch 0x5C never fired — no mask-8
trigger writes it; a class-5 object contact is the likely game path, trigger-port-check #11) and the
standing ride touched the ch-0x70 park plate the game only fires crouched (fix_truck's trace — the
fix_truck_live sister caught it). The union of both capsules broke the same sister. The stand-in
stays; its known misses are documented.

## Left for the next session

- 05: the gatehouse ramp route (or a walker ledge-hop), then the truck ride from a walked start;
  the cable route's west anchors; 06 (the truck spawn side, the wire-cut), 07 (the weapon lock),
  08 (the stairs invisible wall — coordinate with 060-fix-objects2 if merged).
- The 02 stealth route (the plates via the z −26 gap at x −59 — the cell map is in the session log)
  as a bonus pass.
- The full suite ran green on the affected binaries (fix_truck, walk_05, walkreal 01–04); the
  background full-suite runs were OOM-killed twice in this sandbox — the suite should be run once
  on a bigger box before the merge.

## Live checks for the main agent

- 07000002 bunker: walk Bond to (−47.3, 7.46, −24.3) — confirm the real game does NOT grab him onto
  the courtyard cable while he walks the bunker (the port now agrees; the old port auto-grabbed).
- 07000004: shoot the seated target once — confirm the real game's damage numbers vs the port's
  (the GLOBAL impact path is new).
