## Your task: `swim-vent2`, land the swim fix green, then the vent crawl and the alarm music

Read `docs/reports/fix-swim-vent.md` on `s/fix-swim-vent` (e34a30c) first. Its finding is right and valuable: the
water bed is material 0xE in the water cels, the game's player queries are cel-scoped, so the walking Bond never
stands on it. The main agent merged it into master. That needed one fix on top: master's newer callers of
`segment_surface` (the fix-impacts bullet impact in drone.rs, impacts_live.rs, walkreal_probe_fb4.rs, the probe_swim
example) got the new flag, `false`. The full suite then went **691 passed, 15 failed**, so it was backed out:

- **12 nf-game unit tests** (these fail on your branch alone too: run `cargo test -p nf-game --lib`):
  `world::tests::{segments_hit_floors_and_walls (world.rs:950: None where a hit is expected),
  moving_objects_collide_where_they_are, scoped_actor_queries_keep_only_the_straddle_cels}`,
  `player::tests::{jumps_and_falls_with_gravity, walks_at_the_game_speed_stops_at_walls_and_climbs_steps}`,
  `drone::tests::{drone_stands_on_the_floor, drone_just_below_the_floor_snap_threshold_is_in_air}`,
  `grenade::tests::{grenade_falls_under_gravity_and_bounces, grenade_never_explodes_from_a_slow_bounce,
  grenade_sits_after_max_bounces_and_explodes_only_at_the_fuse}`,
  `pickup::tests::{a_dropped_gun_falls_to_the_floor_and_rests, the_take_ray_origin_decides_the_take}`.
  The one-sided ray rule: the unit tests' hand-built triangles are wound the other way, or the stored-normal
  convention is the opposite of what the rule assumes. Decide which from docs/formats/collision.md and the disc
  (do the real floors' stored normals point up?), not by flipping the tests.
- `fix_mission_live::the_07000001_exit_works_end_to_end` (fix_mission_live.rs:347): Mayhew stops at
  (−33.0, 4.54, 11.8), 3.6 short of the exit plate and 2.2 above it.
- `fix_objects2_live::the_missile_cage_still_stops_bullets_but_never_the_capsule` (fix_objects2_live.rs:117): the
  07000008 cage wall no longer stops a bullet/LOS segment crossing x 135.3. One-sided bullets/LOS let a shot pass
  through the back face, so check what the game's `Collide_Pick` does for the 0x1000 cage.
- `walkreal_0700000c_live::walk_start_to_the_st1_plate_and_the_photograph` (walkreal_0700000c_live.rs:94): the
  ST-1 plate #506 never fires ch 0x1b.

Your report said nf-game had 68 passing. The lib alone has 117 tests, so run the whole suite, not a subset.
Prefer the faithful long form your report named (the per-cel query scoping) over global rules if that's what keeps
all of the above green. The material-0xE drop is fine for the player's feet and capsule.

Then, if time allows: the vent crawl (creepwall 0x3F / class 0x2B → `Player_CollCreepWall` 0x196758 → substate 7
`Player_Creep` 0x1969d0) and the alarm music's per-level dispatch (`UpdateMFX_01CastleB` 0x113060 for 07000006),
as in the original task 140.

Files: `crates/nf-game/src/{world,player}.rs` and the callers your branch touched, `crates/nf-view/tests/fix_ladder2_live.rs`,
a new creepwall module, `crates/nf-game/src/music.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/swim-vent2` (start from `s/fix-swim-vent`, then merge `origin/master` into it).

---
claimed-by: sandbox5 at 2026-10-13T13:25Z

---
## Report (done, 2026-10-13T14:21Z)

