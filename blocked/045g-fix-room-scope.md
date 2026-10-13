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


---
## Report (blocked, fixed attachment)

# fix-room-scope — the actor capsule on the room-scoped collision: measured, and where it stops

Branch `s/fix-room-scope`, from `origin/master` 4c91a95. Time-boxed session. The task's core move —
port `build_FindCel` 0x1c8cb0's tie-break into the scope build and switch the capsule on — was
implemented, measured on the real level, and **stopped at a structural finding**: the game's static
collision never goes through object linking at all, so the tie-break over the port's folded instance
world produces mappings that contradict the game's own take behavior. The capsule stays FLAT (no
regression landed); the evidence chain and the next session's fail-firsts are in the branch.

## What was implemented and measured

1. **The faithful `build_FindCel` port** (commit 2 on this branch, since reverted in build_scoped;
   the verbatim implementation survives in the probe test): every ROOM cel (the placement flag
   word's 0x40000 classes — `parseentity_fixup_entity` 0x1d00c0 writes cel+0x3c = 0x804c023/0x804c047
   for 0xc023/0xc047) whose bbox contains the world-sphere centre is a candidate (cap 0x40); one
   candidate returns directly; several → two vertical rays from the centre (up 256 first, then down
   clamped to the up hit) through each candidate's own linked geometry; the nearest hit wins, no
   hits = the first candidate [M 0x1c8cb0 disassembly — `fStack_bc = pfVar7[1] + fVar10`, then
   `fStack_ac = pfVar7[1] − fVar10`, `HITDATA+0xc < fVar11` init 256.0]. Candidates ordered by cel
   list order; the rays see only instances linked before the current one (the game links at
   creation, in placement order — every entity creator calls `Control_CreateObjEx` 0x1343c8 →
   `build_LinkToRoom` 0x1c9680 → `build_FindCel`). `Collide_RayTriangle` 0x1edfe0 was checked for
   face culling (none — both faces hit).
   Unit-tested green while it stood: `build_find_cel_tie_break_rays_decide_not_smallest_volume`.

2. **The measurement on 07000008** (the probe: `NF_ROOMSCOPE_PROBE=1 cargo test -p nf-view --test
   fix_room_scope_live probe_room_scope_links -- --nocapture`, kept in the branch):
   - instance 32 (`Room_Stairwell_T_2`, map51 obj65), world centre (149.03, 40.49, −116.00):
     candidates **[15, 40]** — cell 59 is NOT a candidate (59 was never in the game's candidate set;
     the old smallest-volume rule invented it). Rays: cell 15's list hits 68.0 below (the exterior
     mesh instance 7 is linked to cel 15 first and its collision spans the castle base), cell 40's
     list is empty → **the law maps instance 32 to cel 15**, not 40.
   - the capsule's shaped straddle set from cell 40 (the 0x1eac98 walk + the 0x1ea7b8 0x800 shape
     test) keeps **only [40]** — the fusebox capsule straddles no portal.
   - so Bond still falls at the fusebox with the faithful law — the [D] playtest says the game does
     not fall there.

3. **The contradiction, resolved**: the game's static meshes do not go through `build_LinkToRoom`.
   Their collision is the map's own **Coll_Data node layer**: `parsemap_block_Coll_Data_New`
   0x1d1810 allocates the node list and writes it to **`pCurrCelList+8`** — the cel that is current
   when the block parses, i.e. the parse order assigns static collision to cels, not
   `build_FindCel`. Every `build_LinkToRoom` caller in the decompile is an entity creator (doors,
   pickups, ladders, the player: `Player_Init` 0x1910f8, `Player_StandAtNewPosition` 0x190d88,
   `Door_Create` 0x136100, …) [M]. `build_FindCel`'s raycast tie-break is the law for **objects**
   only. The port folds static meshes and objects into ONE instance world
   (`CollisionWorld::build_scoped`), where the object-link law cannot reproduce the game's placement:
   applied verbatim it moves the safe's own runs out of cell 48's bucket and **breaks the take**
   (`the_safe_pickups_take_after_the_safe_opens` failed; reverted — it is green again).

4. The smallest-volume containment stays in `build_scoped` (now documented with the finding).
   `ScopeGraph.rooms` and the tie-break code were removed from the shared file to keep the diff
   minimal; the law lives in the probe test where the next session needs it.

## What landed

- `crates/nf-view/tests/fix_room_scope_live.rs` (new):
  - `the_scoped_query_misses_the_exterior_mesh_from_indoors` — the straddle discipline on REAL
    level data: the flat world hits `Room_Outside_3`'s collision (map51 obj16) from the bedroom,
    the scoped query from cell 48 misses it, and SOME cell's scoped query still hits it (the mesh
    is not orphaned) [M Collide_StraddleCels 0x1eac98 / Collide_Pick 0x1eb320]. **Passing.**
  - `bond_does_not_fall_at_the_fusebox_with_the_scoped_capsule` — the end-to-end guard: Bond ticked
    at the fusebox through the real input path keeps standing (flat capsule today). **Passing.**
  - `the_scoped_floor_ray_hits_the_guardroom_floor_at_the_fusebox` + `the_scoped_capsule_stands_on_
    the_guardroom_floor_at_the_fusebox` — **the next session's fail-firsts, committed IGNORED** with
    the measured reason in the attribute: they fail until the Coll_Data node layer exists, then
    they gate the capsule switch.
  - `probe_room_scope_links` — the env-gated probe carrying the verbatim tie-break law + the
    straddle-set printer (the evidence for everything above).
- `crates/nf-game/src/world.rs`: `build_scoped`'s doc now states the measured finding with the
  addresses. No behavior change (the smallest-volume rule stays).
- `docs/WORKLOG.md`: the fix-room-scope entry.

## Full suite

`NF_ISO=$NF_ISO cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view
-- --test-threads=1`: the only failures are the 3 allowed `disc_golden` tests (the slim ISO's
missing /DRIVING files). fix_safe08_live: 5/5 green (the take intact).

## What's left (the next session's path)

1. **Port the Coll_Data node layer** — the map block handler `parsemap_block_Coll_Data_New`
   0x1d1810 (block types 4/5: vert/tri counts at FileNextBlock+8/+0xa, the 0x40-byte planes at
   +0xc, the +0.01 bbox inflation loop) attaches a collision node list to `pCurrCelList+8`; the
   parse's current-cel cursor decides the cel. `nf-formats/src/level.rs` must expose the block
   stream in parse order (placements + Coll_Data interleaved); `world.rs` gets per-cell node
   buckets beside the instance buckets. Then the fusebox floor lives wherever the parse put it.
2. Flip the two ignored fail-firsts, switch the capsule on (`capsule_push_scoped(self.cel, …)` +
   `segment_hit_scoped(self.cel, …)` at `player.rs` 483/487/884/886 — the one-line swaps), run the
   whole walkreal/walk suite (a scoped capsule must not open new fall-throughs).
3. The draw side was NOT touched this session: `portal_visible_cels` stays NF_PORTAL-gated, and
   130-fix-portal keeps the general portal draw. One design note for 130: fix-cel-order's
   `Material::node` already carries the room placement's own `CelGraph::cells` index and
   `portal_cel_chain` already runs every frame for the draw order — gating room-class World draws
   (`node != u32::MAX`) by chain membership is the game's `View_AddCels` set and needs no new walk;
   the 07000006 incident needs its eye-spot re-check (fix-safe08b's headless eval could not
   reproduce it from real eye positions).

## Live checks for the main agent

1. In the real game at the GuardRoom fusebox (153.15, 43.38, −119.67, cell 40): break on
   `Collide_StraddleCels` 0x1eac98 during a walk tick and dump the collected cel ids — confirms
   whether the game's straddle set from cell 40 is really [40] (the port matches) or reaches
   further (then the port's portal walk is wrong, not the mapping).
2. In the same spot, dump the game's `Collide_Pick` candidate walk: whether the hit triangle under
   the fusebox belongs to a Coll_Data NODE (cel+8 list) or an OBJECT (the stairwell object) — pins
   the Coll_Data-layer theory before the next session ports it.
3. Renders (rule 6): `07000008 --bond 155.19,43.12,-113.38,-3.837,0.010` — unchanged since
   fix-safe08b (the blue block still draws; the collision is already scoped out of the take).

## Status

`blocked` for the task's capsule switch: the gap is not the tie-break (it was ported and measured)
but the missing Coll_Data node layer under the port's folded static world — porting it is a
formats + world change larger than this session's window. The branch carries the probe, the
fail-firsts, and the evidence to resume directly.
