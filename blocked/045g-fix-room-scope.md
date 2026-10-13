## Your task: `fix-room-scope`, switch on room-scoped collision and drawing (the 07000008 blue block)

`s/fix-safe08b` (merged into master; read `docs/reports/fix-safe08b.md`) built the game's room-scoped collision
machinery in `crates/nf-game/src/world.rs` (`ScopeGraph`: `Collide_StraddleCels` 0x1eac98's portal BFS,
`Intersect_Portal` 0x1ea7b8's per-query-type tests, `build_LinkToRoom` 0x1c9680's object→cel linking) and proved the
blue block: from the bedroom (cell 48) the straddle walk reaches 17 cells and never the exterior room, so
`Room_Outside_3`'s collision (map51 obj16) is out of every indoor query in the game. It is not switched on for the
actor capsule because of one gap:

**The floor-mapping gap.** With the scope on, Bond falls through the GuardRoom floor at the fusebox (153.15, 43.38,
−119.67, cell 40): the only collision under that spot is the shared `Room_Stairwell_T_2` mesh (instance 32), which the
port's smallest-volume containment maps to cell 59, never in the capsule's straddle set from 40. The game links it
with `build_FindCel` 0x1c8cb0's tie-break (raycast each containing cel's own object list; the nearest hit wins; see
`Scene::cel_at`'s doc). Port that tie-break into the scope build.

**Do:**
1. Port `build_FindCel`'s tie-break, switch the scope on for the actor capsule (`capsule_push`, the walk), and show:
   Bond walks the 07000008 bedroom from the door to the safe without the block, and does NOT fall at the fusebox spot.
2. Run every walk test (`walkreal_*`, `walk_*`, the escort/door tests): a scoped capsule must not open new fall-throughs
   anywhere. Any level where it does is a floor-mapping case to fix the same way.
3. The draw side: un-gate `portal_visible_cels` (NF_PORTAL) or replace it with the game's `Vision_Portal_Recurse`
   0x1e86e8 → 0x1e82e0 cel set, so the bedroom doesn't draw the exterior stone (render spot `07000008 --bond
   155.19,43.12,-113.38,-3.837,0.010`). Re-check the 07000006 incident that gated it (fix-perf-level).
   Coordinate with `130-fix-portal` (pending): say in your report if you did the draw so 130 can be trimmed.

Tests (NF_ISO, fail-first): the bedroom walk to the safe, the fusebox no-fall, a scoped query that misses the exterior
mesh from indoors, the sister (an outdoor query still hits it).

Files: `crates/nf-game/src/world.rs`, `crates/nf-view/src/scene.rs` (the cel graph, the draw), the walk tests.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  and `cargo build --release -p nf-extract`. On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/fix-room-scope`.

---
claimed-by: sandbox4 at 2026-10-13T09:44Z

---
## Report (blocked, 2026-10-13T10:53Z)

