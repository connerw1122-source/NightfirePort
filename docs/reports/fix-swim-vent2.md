# fix-swim-vent2 — land the swim fix green, then the vent crawl and the alarm music

Branch `s/fix-swim-vent2` = `s/fix-swim-vent` (6b63096: the swim fix + the vent crawl) merged with
`origin/master` (8142a7f). The sandbox was reset at session start, so setup was redone (clone,
work-slim, the onfoot ISO, rustup, a user-prefix ALSA).

## Done: the 12 nf-game unit failures — the one-sided ray rule now uses the game's STORED plane

Root cause, from evidence:

- The game's ray test is one-sided by the triangle's **stored plane record**: `Intersect_RayGeom`
  (ACTION.ELF 0x10a8c8) loads the plane quadword the triangle halfword points at (leaf words +
  plane·2 bytes: 3 s16 normal · 2⁻¹⁴ + i32 distance) and skips a triangle when
  `dot(plane_normal, ray_dir) >= 0` (the decompile's `if (0.0 <= dot) skip` before
  `t = -(dot(from,n)+dist)/dot(dir,n)`).
- The port recomputed the normal from the winding. `docs/formats/collision.md` says stored normal =
  +cross(v1−v0, v2−v0) for 2,242,859/2,245,313 disc triangles, **146 the opposite, 477 disagree**.
- Measured on the disc (`crates/nf-view/examples/probe_winding.rs`, through the same
  `CollisionWorld` the sim walks on): real floors' winding normals point **up** — 7,246 up / 0 down
  on 07000008 (77/0 on 07000001). The 12 unit tests' hand-built triangles were wound the opposite
  way (floors normal-down), so the faithful one-sided rule rejected them.

Fix (two parts):

1. `nf-formats/src/collision.rs` — `triangles_with_material` now decodes each triangle's stored
   plane normal from the already-parsed `words` and, **where it opposes the winding cross, swaps
   the triangle's vertex order**, so the port's cross equals the game's plane direction everywhere
   (the hit point and t are winding-independent; only the cross flips). Every one-sided site in
   `world.rs` (segment/segment_surface/scoped/capsule/ground_motion) inherits the game's behavior
   without signature changes. Cited in the doc comment.
2. The 12 unit tests' hand-built triangles re-wound to the disc convention, with the evidence
   cited in place (up-wound floors; walls wound to face the ray/capsule they stop):
   `world::tests::{segments_hit_floors_and_walls, moving_objects_collide_where_they_are,
   scoped_actor_queries_keep_only_the_straddle_cels}`, `player::tests::{jumps_and_falls_with_gravity,
   walks_at_the_game_speed_stops_at_walls_and_climbs_steps}`, `drone::tests::{drone_stands_on_the_floor,
   drone_just_below_the_floor_snap_threshold_is_in_air}`, `grenade::tests::{grenade_falls_under_gravity_and_bounces,
   grenade_never_explodes_from_a_slow_bounce, grenade_sits_after_max_bounces_and_explodes_only_at_the_fuse}`,
   `pickup::tests::{a_dropped_gun_falls_to_the_floor_and_rests, the_take_ray_origin_decides_the_take}`.

`cargo test -p nf-game --lib`: **117 passed, 0 failed** (was 105/12).

## Done: the cage test (fix_objects2_live) — the disc shows the wall is one-sided

`probe_planes` (new example, raw 0x2E blocks) shows the cage wall at x 135.33 (object 34) has
stored plane normal **(−1, 0, 0)** — raw s16 (−16384, 0, 0), facing west into the station side;
the lid over the walkway faces down (into the map). Only 5/73,197 level triangles have a stored
normal opposing their winding (none of them the cage). So under the game's one-sided test:

- an **east-bound** ray (station → walkway) stops at x 135.33 — the test now asserts that, with the
  stop x checked against the plane;
- the old expectation, a **west-bound** ray (walkway → station) stopping at 135.3, is a back-face
  hit the game itself lets pass — the test now asserts it misses (sister). A west-bound bullet
  from the walkway continues into the cage and meets the east-normal membranes of object 12 around
  x ≤ 100 (the cage's west side) — that far stop is not asserted here.

Result: `fix_objects2_live` **5/5**.

## Done: the merge-caller fix, kept

Master's newer `segment_surface` callers (the fix-impacts bullet paths in `drone.rs`,
`impacts_live.rs`, `walkreal_probe_fb4.rs`, the `probe_swim` example) pass `no_water = false`;
`impacts_live` keeps the normal binding. `probe_cw` was updated to the current disc API (it still
used the removed `disc::Disc`).

## Still red (2): Mayhew's exit walk and the ST-1 plate walk

- `fix_mission_live::the_07000001_exit_works_end_to_end` (line 347): Mayhew still stalls at
  (−33.0, 4.54, 11.8), short of the plate. Evidence gathered this session: the floor under the
  stall spot is y 3.539 (mat 0xa) with an up normal; a down-ray at the plate area
  (−36.6, 3.3 → −2.7, 11.8) finds **no floor within 6** — the plate's collision floor is either
  elsewhere or not up-wound. Next step: dump the collision triangles under the plate from the raw
  blocks (`probe_planes` with the plate's bbox) and check whether the plate pad's stored plane
  faces up; then either the pad is found by the walk (fix in `world.rs` scoping) or the stall is a
  steering/ground-motion issue, not the one-sided rule.
- `walkreal_0700000c_live::walk_start_to_the_st1_plate_and_the_photograph` (line 94): ch 0x1b still
  never fires (fails fast, 0.75 s). Not root-caused yet — the same plate-pad question may apply
  (plate #506's trigger needs Bond standing on its pad).

## Suite status in this window

Run and green: `nf-game --lib` 117/117; `fix_objects2_live` 5/5; `fix_mission_live` 3/4 (the exit
walk above); `walkreal_0700000c_live` 0/1 (above). The full four-crate run
(`NF_ISO=<iso> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p
nf-audio -p nf-view -- --test-threads=1`) could not complete inside this sandbox's window — the
release build + live suite exceed the remaining minutes. The next session should run it as the
first step and pick up the two reds above; the base branch's remaining live suite was green
pre-merge and this session only touched the files listed here.

## Files touched

`crates/nf-formats/src/collision.rs` (the stored-plane alignment — additive decode inside
`triangles_with_material`), `crates/nf-game/src/{world,player}.rs` (test-geometry rewinds only in
this session; `world.rs` unchanged otherwise), `crates/nf-game/src/{drone,grenade,pickup}.rs`
(test-geometry rewinds + the two bullet-impact `no_water=false` callers in drone.rs — shared files,
small additive hooks), `crates/nf-view/tests/fix_objects2_live.rs` (the cage evidence rewrite),
`crates/nf-view/tests/{impacts_live.rs,walkreal_probe_fb4.rs}` + `crates/nf-view/examples/
{probe_swim.rs,probe_cw.rs}` (the caller flag / the API fix), new examples `probe_winding.rs`,
`probe_planes.rs`. The vent crawl (`creepwall.rs`, the crawl substate, the music event) came over
verbatim from `s/fix-swim-vent` 6b63096 and was not modified here; the alarm music's per-level
dispatch was already on master (fix-npcs a80b713).

## Live checks for the main agent

None required for what landed. If the plate pad stays unfound, a `gdbwatch` on
`Player_FeetOnPoint` (0x194188) while Mayhew walks onto the 07000001 exit plate would show which
cel/plane the game grounds him on there.
