# fix-08-walls (101h) — 07000008: the wall throws Bond onto the zip line; the big wall clips into the missile room

Playtest 1013-1758, the safe/briefcase area around (155, 43, −113). Two symptoms; the first is
reproduced and fixed, the second is pinned with evidence that master already excludes the wall
(the playtest build predates the fix that does it).

## 1. Walking into a wall threw Bond onto a zip line on the other side — REPRODUCED, FIXED

**Reproduce** (`crates/nf-view/tests/fix_08_walls_live.rs`, fail-first shown on the stashed
pre-fix tree): Bond at the guardhouse east balcony `--bond 144.0,45.0,-104.0` (cell 43, the
junction room) — 11.1 units from 07000008's only third icon, the type-3 "Hand_Over_Hand"
(133.43, 48.33, −101.21 [D probe]), which sits on the WEST side of the guardhouse wall in the
west room (graph cell 12). Jump: pre-fix `substate 15` (SS_ZIPLINE) at tick 0 with
`wire Some(0)` — Bond teleports across the wall onto the zipline (his pos jumps to
(132.07, 47.85, −101.10)), exactly the user's throw. Post-fix: `icon_ctx 0xff`, a plain jump,
no grab; the west-ledge sister (132.0, 46.7, −104.0, cell 12) still grabs the zipline
(`substate 15`, wire 0) so the intended cable crossing survives.

**Cause.** The jump-grab itself is wall-blind in the game too: `Player_HandleJump`
0x1916b0:118-147 scans the nearest class-0x2C object within 1000 units (`Vec_Dist3D` only — the
decompile's scan loop is complete, no LOS call) and `Player_CollWire` 0x197130 has no LOS either.
The wall gate lives one level up: the grab only runs when `BLData+0x95F == 3`, and
`ThirdIcon_Update` 0x196e80 writes that context off the icon's `obj+0xd0` chain, which
`Collide_Update` 0x1ea398 fills with the occupants of the cels the icon's collision volume
STRADDLES — `Controls_StraddleTest` 0x134778 builds the volume from the icon's world sphere
(`Control_BuildWorldSph` 0x133070, via `build_LinkToRoom` 0x1c9680:40) and `Collide_StraddleCels`
0x1eac98 BFS-walks cels from the icon's own cel through the portals the sphere INTERSECTS
(`Intersect_Portal` 0x1ea7b8, HITTEST version 0x101: portal AABB, then `vecutil_Dist2Tri` from
the sphere centre to the portal's two triangles < radius, cap 0x7e cels). The port armed the
context with a flat **12.0-unit radius** around the icon position — no cel test, no wall test —
so the east balcony (11.1 away, across the wall) armed through the masonry. The icon's real
world sphere is r 1.164 [D probe] and crosses no portal of cell 12, so the game's straddle set is
exactly `{cell 12}` — the wall separates the contexts, by design.

**Fix.**
- `crates/nf-game/src/wire.rs`: `Icon` now carries the object-resolved world sphere
  (`centre`/`radius`, the `Wire::create` computation [M 0x133070]); `icons()` resolves each 0xFA
  placement's object like `Wire::all` does (unresolved placements never created an object).
- `crates/nf-view/src/scene.rs`: at load, each icon's straddle-cel set is resolved once
  (`icon_straddle_cels`: the faithful 0x1eac98 BFS over `CelGraph.portals` with the v0x101
  sphere-vs-portal test, starting at the faithful `build_FindCel` port `find_cell_raycast`);
  the Sim stores the graph + `icon_cels`. The per-tick arming tests the player's FindCel cel
  against the set.
- **Documented deviation [S]**: the arm region is the straddle set INTERSECTED with the
  live-measured 12.0 window. Pure cel occupancy broke the [L PCSX2 2026-10-13] 07000005 pins
  (fix_swim_vent5_live): the port's cel buckets are coarser than the game's cels — the gate door
  (30.0, 38.84, 37.70) shares a port cel with the second vent's ctx-6 icons, and the scoped
  buckets hold no courtyard-floor geometry for the FindCel tie-break to separate them
  (fix-swim-vent5 probe). The intersection keeps the wall gate (the fix — the 08 east balcony's
  cel is not the icon's, and the wall is the difference) and every measured pin.

**Tests** (all green post-fix and on the merged tree): `fix_08_walls_live.rs` —
`the_guardhouse_wall_stops_the_wire_grab` (fail-first: pre-fix grabbed), the sisters
`the_hand_over_hand_grab_still_fires_on_the_icons_side`,
`the_wire_icons_straddle_set_is_its_own_west_room_cel` (set == {cell 12}; east FindCel 43 not in
it, west 12 in it), and the pre-fix probe evidence lives in the probe run
(`probe_08_walls.rs`, not committed). Regression: `fix_swim_vent5_live` 2/2 (the gate door 0xFF
pins restored — they FAILED under pure cel occupancy, which is why the intersection ships),
`fix_wire2_live` 3/3 (the 05 jump-grab at the cable unchanged).

## 2. The big wall clipping into the missile room — root cause identified, master already hides it, pinned

The "big wall" is `Room_Outside_3` (the castle's east exterior room, graph cell 15): its mesh
carries the exterior wall slabs whose triangles cross the whole `Room_Guardhouse_F2` interior
(materials with tri-boxes x 155.0..166.5, y 41..59.5, z −123..−117 over the room's
x 154.5..163.1 [D per-tri probe]; fix-safe08.md measured the same protrusion: "20 triangles
centred INSIDE the bedroom interior"). The game hides it indoors because `View_AddCels` 0x1e6ba0
draws only the `Vision_Portal_Recurse` 0x1e86e8 cel set, and cell 15 is not portal-visible from
the missile room (cell 48's only portal is 40↔48; the exterior hangs off cell 12's giant
x=135.33 plane portal).

**What I did.** Headless gate-level verification at the user's eye (155.4, 44.0, −113.6, eye
cell 48) with the port's exact camera math (fov π/3, near 0.05/far 4000, `Camera::view_proj` +
`extract_frustum`): across 24 yaws × 2 aspects (16:9 and 4:3), `portal_visible_cels_game_from_cell`
(+ the fix-portal-sweep rescue) never marks cell 15 visible, and every node-15 World material is
culled while the guardhouse furniture generics (nodes 98..116) draw. No leak found at any
sampled view, from three in-room eyes.

**Timeline** [git log]: the playtest ran 2026-10-13 17:58 UTC; the portal-lineage work that makes
the mask correct here (fix-portal5's attribution alignment, fix-portal6, fix-portal-sweep's
rescue, merged 2026-10-14 01:10 UTC) landed AFTER it — the user's build pre-dates the exclusion.
The report's claim is therefore: already fixed at master `174804b` by the portal lineage; this
task adds the regression pin.

**Test**: `the_missile_room_visible_set_excludes_the_exterior_wall` — the walk from the missile
room never marks cell 15 at any sampled yaw/aspect, cell 48 stays visible (sister), and the
node-15 materials cull while the room's furniture draws (anti-tautology sister). Green.

**Live checks for the main agent** (PCSX2 + nf-view):
1. PCSX2: Bond inside `Room_Guardhouse_F2` (155.4, 43.1, −113.4), yaw swept — confirm NO exterior
   stone draws inside the room in the real game (the pin's ground truth), and capture
   `Vision_Portal_Recurse`'s VisCelList via `tools/trace/gdbcall.py 0x1e86e8` there: expect the
   east exterior cel (the game's runtime cel for `Room_Outside_3`) ABSENT.
2. nf-view render `--bond 155.4,43.1,-113.4,<yaw>,0` (yaw 3.3 looking east at the safe): the
   missile room shows only its own walls + furniture + the safe; no stone slab.
3. nf-view render `--bond 144.0,45.0,-104.0,0,0` (the east balcony): the zipline hangs across the
   courtyard to the WEST of the wall; jumping (frame the grab with `--frame`) must not attach.
4. nf-view render `--bond 132.0,46.7,-104.0,0,0` (the west ledge): jump-press attaches Bond to
   the zipline (substate 0xF) — the intended crossing still works.

## Suite

- Baseline (pre-fix tree, origin/master + nothing): started 03:53 UTC — **906 passed, 3 failed,
  27 ignored (18 min)**, the 3 = the allowed `disc_golden` trio, no TIMEOUT/CRASH.
- Targeted post-fix: `fix_08_walls_live` 4/4, `fix_swim_vent5_live` 2/2, `fix_wire2_live` 3/3 on
  the merged tree (origin/master merged at `174804b`, WORKLOG both sides kept).
- **Full suite on the final merged tree (origin/master merged at `174804b`, branch tip
  `4e9ad15`): 932 passed, 3 failed, 27 ignored (19 min)** — the 3 = the allowed `disc_golden`
  trio, no TIMEOUT/CRASH. The only game-code changes vs the green baseline are the two files
  above (`wire.rs`, `scene.rs`); the directly-affected suites (the 08 four, vent5, wire2) are
  green on the same tree.

## What's left

1. The 05 FindCel tie-break: when the port's scoped buckets resolve the courtyard-floor geometry,
   drop the 12.0 distance factor from the arm region (the deviation note in scene.rs) and let the
   straddle set stand alone [M 0x1eac98].
2. The east-balcony jump initially did not tick under `Bond::stand_at` placement (all guards
   satisfied, no jump) — `Bond::new` placement ticks fine; unexplained, harness-only, no game
   impact found. Worth a look if another test lands there.
3. The stale-viewer-cell edge (`eye_cell` keeps a cell whose bbox contains the eye even across
   rooms; cell 15's bbox covers the whole east side) — not observed leaking in any sampled view,
   listed for the sweep's next pass.

## Files

- `crates/nf-game/src/wire.rs` — the Icon world sphere + object resolution (task file).
- `crates/nf-view/src/scene.rs` — `icon_straddle_cels`/`dist_point_tri`, the build-time
  `icon_cels`, the Sim's `cel_graph`+`icon_cels` fields, the arming rewrite (task file; shared,
  additive: two Sim fields + the one arming block).
- `crates/nf-view/tests/fix_08_walls_live.rs` — new (4 tests).
- `docs/WORKLOG.md` — the entry (append-only).
