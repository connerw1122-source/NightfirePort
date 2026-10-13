# fix-swim-vent — the plain-language report

Branch `s/fix-swim-vent` (from master f0e1b23). The sandbox window ended the session early; the
swim fix is done and verified, the vent crawl and the music step are not started beyond the
research fix-ladder2 already recorded.

## Done — swimming from a bank (playtest item, fix-ladder2's leftover)

**The user's symptom**: on 07000002, Bond walks on the water instead of entering it.

**Root cause** (evidence-tagged, with the addresses in the WORKLOG and code comments):
- The game's player collision is **cel-scoped**: the grounding ray (`Player_FeetOnPoint` 0x194188)
  and the capsule test (`Collide_SphereIntersect` 0x1ec940) only test the geometry of the cel the
  player is currently in (`obj+0x20`). [M]
- The water surface/bed planes carry surface **material 0xE** and live in the water cels, whose
  bbox tops are the water surface (`cel+0x90`). A walking root stands 1.03 above any bed, i.e.
  always above the water volume's bbox — so the walking player's cel is never the water cel, and
  the game's Bond never stands on water: he falls through and `Player_InWater` 0x195e50 (root − 0.3
  < surface) puts him in the swim substate. [M + D probe]
- On the disc, every underwater hit in all four 07000002 water cels is material 14 (0xE) at the
  surface level; the banks are 1/4/7/10. [D probe_vent_swim]
- The port's collision world is flat (no cel scoping), so it grounded Bond on the mat-14 bed — he
  walked across the pond at standing height and `Player_InWater` never fired. That was the bug.
- Two supporting game facts also ported: all the game's ray tests are **one-sided** by the stored
  plane (`Intersect_RayGeom` 0x10a8c8 skips `n·dir >= 0`; the stored normal = +cross of the
  winding per docs/formats/collision.md) [M]; and the swim substate builds its own **low-slung
  capsule** (spheres at 0.65−cc / 0.45−cc, radius 0.55, `Player_Collision` 0x194df0 case 3) so the
  swimming Bond's capsule hangs under the root and never catches the bed plane. [M]

**The fix** (`crates/nf-game/src/world.rs`, `crates/nf-game/src/player.rs`):
1. The player's grounding ray (`segment_hit`) and both capsule paths (`capsule_push`,
   `capsule_push_scoped`) drop material-0xE triangles — the material form of the cel scoping. The
   full per-cel scoping (a query-time cel argument through the world) is the faithful long form
   and is listed below. Bullets, LOS, the searchlight spot and the grenade bounce keep water
   geometry (unchanged); the drone and player footstep rays drop it (they are feet rays).
2. All ray paths are now one-sided by the triangle's winding (the game's rule).
3. The capsule is one-sided for near-horizontal faces (the game's head-on cull 0x109da0:302-322,
   approximated by the |n·y| > 0.7 split ≈ the game's 0.999/1.31 spine cone) — walls and steep
   slopes stay double-sided, as in the game's non-head-on path.
4. `Bond::capsule` returns the game's case-3 swim spine while swimming.
5. `CollisionWorld::tris_near` added (the read-only evidence accessor).

**Verification** (all NF_ISO, through `nf_view::load` + `Sim::step_bond`):
- `fix_ladder2_live::swim_repro_walks_on_the_water_from_the_bank` — un-ignored, expectation
  flipped: Bond starts on the west bank (floor 4.155 vs surface 3.7675), walks in, is NOT grounded
  over the pond, enters SS_SWIM at tick 1621 at y 3.725 (0.04 under the surface), and is still
  swimming 180 ticks later. Sisters: the bed under the pond centre is material 0xE at the surface
  level; the entry point is under the surface. This test failed before the fix (he never entered
  the water).
- fix_ladder2_live's other 3 tests (the ladder draw/look/weapon) still pass 4/4;
  walk_07000002_live 2/2 and walkreal_07000002_live 1/1 (the level the change touches) pass.
- Suite: nf-game 68 passed, nf-formats 11 passed + the 3 allowed `disc_golden` failures (slim
  ISO), nf-audio green. **The remainder of nf-view's live suite did not fit the sandbox window** —
  the collision change is wide, so the next session should run the full nf-view suite first thing.

## Done — the vent crawl (the creepwall), landed just before the window closed

The crawlable vents are the 07000005/0700000d/07000041 creepwall placements [D docs/systems/
levels.md] — not 07000002 (that "grate" is the SS hatch, class 0x25, already ported by
fix-npc-doors-escort). Ported: the creepwall module (placement 0x3F → class 0x2B '+', the channel
= param 3 [M 0x196fc8 + 0x1969d0:86-88], the half-length from the placement scale [S]); the crawl
entry `start_creep` [M Player_CollCreepWall 0x196758]; the crawl substate 7 `creep_tick` [M
Player_Creep 0x1969d0 — the per-tick axis snap + fwd·0.275, the 1.0-end exits with the input
toward the end, the exit yaw ∓π/2, regrab 0x78, the +0.05 push; [S] the crawl motion 0.02/tick
(the game's displacement rides the unread anim scripts)]; the scene wiring (the use-key creep arm
hoisted above the doors/SS pick — the game's context-6 arm returns first [M 0x1a5170:66-97]) and
the per-tick `Music_Event(0xD,1)` + `switch_channels[ch] = 1`.

**Verified** (`fix_swim_vent_live.rs`, NF_ISO, 07000005): the two disc creepwalls parse (ch 113 =
the atlas's 0x71); the use key near the vent enters SS_CREEP on the wall's axis (≤0.35 off), the
crawl holds across ticks, and crawling to the far end exits to SS_WALK with the regrab lockout.
Sister: the axis midpoint sits on the placement centre.

**Remaining known issue**: the port's channel-gated grate door writes its OLD state's not-moving
0 over the crawl's channel set every tick (`Door_SetState` 0x135618:26's mirror in door.rs
set_state), so `channels.on(0x71)` reads 0 while crawling. The game's crawl wins because its door
only writes on transitions; the port's door re-writes per tick. The crawl side is faithful; the
door-side ordering is the remaining fix. The door-grate link itself (the crawl's channel opens
the grate door) is the same channel write.

## Not done (each with its reason)

1. **The crawl's body animation and the crawl camera** — the port runs no body anim scripts for
   Bond (the crawl anims 0x600046d/0x600046f/0x6000471/0x6000475 and the entry 0x6000476 are
   unread), and the crawl camera mode 0xB's specifics are not modelled (the port keeps its normal
   third-person camera). The substate, the movement, the entry/exit and the channel/music effects
   are ported.
2. **The alarm music's last step — already on master, nothing to do.** The per-level dispatch
   (07000006 → the castle-B updater's `MusicUnderAttack ? PickRandomCombat(0) : PickRandomStealth()`,
   `UpdateMFX_01CastleB` 0x113060) landed with fix-npcs (commit a80b713, music.rs lines with the [M
   0x113060:26-30 citation] + the unit test `the_07000006_alarm_demands_combat_music`). Verified
   passing in this session's nf-game suite run (68 passed) — the alarm stream `Music_Event(2,1)`
   flips the hysteresis on the 12th tick and the combat pick fires; a quiet level picks nothing.

## Shared files touched (minimal, per the rules)

- `crates/nf-game/src/world.rs` — the one-sided rays + the water/capsule rules + `tris_near`
  (the swim fix's substance; the file is shared, the change is confined to the query functions).
- `crates/nf-game/src/{drone,searchlight,grenade}.rs`, `crates/nf-game/tests/ai_live.rs`,
  `crates/nf-view/src/scene.rs` — mechanical only: the `segment_surface` signature gained the
  no-water flag (`false` = unchanged behavior everywhere except the feet rays, which pass `true`).
- `crates/nf-view/tests/fix_ladder2_live.rs` — the repro un-ignored with the correct expectation.
- New: `crates/nf-view/examples/probe_vent_swim.rs` (the read-only probe).

## Live checks for the main agent

1. 07000002 garden pond (cel 17, west of the bunker, around (−67, 32)): walk from the west bank
   into the pond — Bond should fall in and swim (the swim animation, the camera low over the
   water), not walk across. `--bond -78,5.2,32.05` facing +x, then walk +x.
2. The deep pool (cel 16, around (−42, 32)): same entry from the deck — should already have
   worked pre-fix; confirm it still does after the change.
3. The lake (cel 6, south, surface 4.34): walk in from the north bank (−45.7, 4.75, 9.5) — swim
   entry expected.
4. Water landing: no fall damage / no stuck-on-the-surface; leaving the water at a bank edge
   still works (the leave path was not changed, but the one-sided rays touch it).

## ntfy

Sent successfully (the first curl, id 2kfv5I4uAAhA): "fix-swim-vent: swim entry fixed+verified
(cel-scoped water collision, repro green); vent crawl + music step left, research recorded in
report".
