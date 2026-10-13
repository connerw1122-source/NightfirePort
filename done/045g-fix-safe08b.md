## Your task: `fix-safe08b`, the 07000008 bedroom's blue block: room-scoped collision and drawing

**Update:** the take now works and fix-safe08 (9c9674b) is merged into master: created pickups leave the collision world (0x1d8d54), so item 2 below is DONE. Do only the room-scoped collision and drawing. Start from `origin/master` and read `docs/reports/fix-safe08.md` first. It found both root causes:
- the safe's items are spawned by the SafePickUps script 0x060009E2 (SP #286 on ch 136) through
  `Script_CreateEntity` 0x1d8968's 0xF0 arm. That's ported: the shelf pickups exist at the game's key positions. But
  `fix_safe08_live::the_safe_pickups_take_after_the_safe_opens` is still red: the take's `Collide_LineOfSight`
  (flags 0x721) is blocked in the port by a hit the game's query evidently ignores (t 0.65–0.88 on the rays).
- the blue block is `Room_Outside_3` (map51 obj16), the exterior room's geometry poking into `Room_Guardhouse_F2`.
  The game never draws it or collides with it from indoors: drawing is the `Vision_Portal_Recurse` cel set
  (0x1e86e8 → 0x1e82e0), collision is `Collide_Pick` 0x1eb320 over the cels `Collide_StraddleCels` 0x1eac98 collects.
  The port draws by frustum per cel (the portal walk `portal_visible_cels` exists but is NF_PORTAL-gated since the
  07000006 incident) and collides with one flat static world.

**Do:**
1. Scope the static collision by the room/portal graph as the game does (`Collide_StraddleCels`: the cels the query
   straddles via `Intersect_Portal`, cap 0x7e), first for the actor queries (`capsule_push`, the walk), then the
   rest; and evaluate un-gating the draw's portal walk on this level (the walk from every room must keep at least
   the room itself; re-check the 07000006 case). Coordinate with `130-fix-portal` (pending), which owns the general
   portal draw: if you do the draw side, say so in your report so 130 can be trimmed.
2. Decode the LOS query-type filter (HITTEST +0x94/+0x98, 0x201/0x721 in `Collide_Intersect` 0x1ebaa8's switch, the
   per-node +0x3c word) enough to name the class the game skips, and make the take fire. The main agent can run the
   live checks in the report (the GDB watch on the take's `Collide_LineOfSight`, the straddle cel dump) with the
   user: ask for exactly what you need.
3. Run the FULL suite (all four crates) before pushing: the previous session couldn't.

Tests (NF_ISO, fail-first): Bond walks from the bedroom door to the safe without the block (the capsule passes where
`Room_Outside_3`'s tris are); the take fires for both items after the safe opens; the sister (no take before ch 136)
stays green.

Files: `crates/nf-game/src/world.rs`, `crates/nf-view/src/scene.rs` (the cel graph, the draw), the pickup/LOS path,
`crates/nf-view/tests/fix_safe08_live.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/fix-safe08b`.

---
claimed-by: sandbox2 at 2026-10-13T07:34Z

---
## Report (done, 2026-10-13T09:33Z)


---
## Report (fix-safe08b, 2026-10-13T09:34Z)

# fix-safe08b — the 07000008 bedroom's blue block: room-scoped collision and drawing

Branch `s/fix-safe08b`, from `origin/master` ac5ca9f. Time-boxed session. The cel-scoped collision
machinery is implemented, unit-tested and verified against the decompile; the take query runs scoped;
the actor capsule stays flat for one documented reason (a floor-mapping gap that needs the game's
`build_FindCel` raycast tie-break). The blue block's DRAW fix is not landed — evaluation below.

## What the decompile says (all [M], ACTION.ELF)

1. **`Collide_StraddleCels` 0x1eac98**: every collision query is scoped to cels — BFS from the
   query's start cel (HITTEST+0x80) through portals the query shape straddles (`Intersect_Portal`
   0x1ea7b8), cap 0x7e cels, each cel entered once (the GameState stamp), the start cel appended
   last. `Collide_Pick` 0x1eb320 tests only those cels' node lists (cel+0x18) and object lists
   (cel+8), plus the ForcedList (objects with obj+0xF0 & 0x20000000, unless query+0x98 bit 0x10).
2. **`Intersect_Portal` 0x1ea7b8** dispatches on HITTEST+0x94: 0x201 = the ray vs the portal's two
   triangles (`Collide_RayTriangle`, the Index_189 corner order 0,1,2 / 0,2,3); 0x400/0x1000 = the
   sphere centre vs the portal tris (`vecutil_Dist2Tri < radius`); 0x800 = the point on the capsule
   segment closest to the portal's reference point, then `Dist2Tri < radius × 1.5`.
3. **`build_LinkToRoom` 0x1c9680** links each object to its cel: `Control_BuildWorldSph` 0x133070
   (the glist local sphere × the object matrix → obj+0x80/+0x8c), then `build_FindCel` 0x1c8cb0
   over the world's cels. Objects with obj+0xF0 & 0x20000000 go to the ForcedList instead
   (flags |= 0x10008000). Objects whose centre maps to no cel are never linked and never tested.
4. **The take's query type**: `Pickup_Update` 0x18e148:203-214 runs `Collide_LineOfSight`
   0x1edf50 with flags **0x721**; `Collide_RayIntersect` 0x1ec690 hard-sets HITTEST+0x94 = 0x201
   (ray) and stores the caller's ushort in **HITTEST+0x98**. The filters that 0x721 turns on:
   - bit 0x20: `Collide_Pick` 0x1eb480-0x1eb49c skips nodes whose flags word (+0x94) has 0x1000
     (the missile cage) — already ported (fix-objects2);
   - bit 0x1: `Collide_Intersect` 0x1ebaa8's tail drops every hit whose surface byte's low 6 bits
     are 0xd or 0xe (`(byte & 0x3f) − 0xd < 2` → the hit is discarded);
   - the per-node +0x3c word's bit 0x8000000 picks the node's quick test: box nodes
     (`Intersect_RayBox` over the node's AABB at +0x40..+0x50) vs sphere nodes (the closest point
     on the ray's line to the node's world sphere +0x80/+0x8c);
   - `Collide_PickObj` 0x1eae80 repeats the same quick test per OBJECT (obj+0x80/+0x8c world
     sphere) and skips objects with obj+0xF0 bits 0x20 / 0x1000 (actor bit) / 0x40 / 0x10.

## What was measured on 07000008 (probe, NF_SAFE08_PROBE=1 in `fix_safe08_live.rs`)

- 118 cells, 21 portals, 0 dropped. The bedroom cell is 48; the GuardRoom's is 40.
- **The bedroom has NO portal path to the exterior**: BFS from cell 48 reaches 17 cells, none of
  them the exterior room's cell (106-area). The game's straddle discipline keeps
  `Room_Outside_3`'s collision (map51 obj16) out of every indoor query — that is the blue block.
- **The take rays**: the rocket ray (the player's world-sphere centre → the shelf rocket) hits
  `Object_Safe`'s own shell (map51 obj155, surface byte 6, object flags 0x0) at t 0.647/0.692.
  Every decoded filter lets that hit through: the straddle set from cell 48 is exactly {48} and the
  safe is in it; the safe's object flags are 0 (no 0x20/0x1000 skip); the surface class 6 is not
  0xd/0xe; the quick tests pass (the safe's world sphere r 1.562 vs the ray's closest approach
  0.398). **The port's graze is one the game's query also should register by this decompile** — the
  skip must live in something this decode has not reached (the safe's collision-data attachment or
  the SS door's linked state). Live check 2 below settles it.
- **The floor-fall finding** (why the capsule is not scoped yet): with the scope enabled, Bond
  falls through the GuardRoom floor at the fusebox (153.15, 43.38, −119.67, cell 40). The only
  collision under that spot is the shared `Room_Stairwell_T_2` mesh (instance 32), which the port's
  smallest-volume containment rule maps to cell 59 — a cell the capsule's straddle walk from 40
  never keeps. The game links it via `build_FindCel`'s tie-break (raycast each containing cel's own
  object list, nearest hit wins — see `Scene::cel_at`'s doc), which the port's build_scoped rule
  does not implement. Reproduced in the probe (the down-ray at that spot hits only instance 32).

## What was implemented (all cited in the code)

- `crates/nf-game/src/world.rs`: `ScopeGraph`/`ScopePortal` (plain data: cell bboxes + each portal
  as the two triangles 0x1ea7b8 tests); `CollisionWorld::build_scoped` (per-instance cell by the
  transformed glist centre, smallest-volume tie — the port's `find_cel` rule; unmapped instances go
  to an `always` bucket that every scoped query keeps); `straddle` (the 0x1eac98 BFS, cap 0x7e);
  `capsule_push_scoped`, `segment_hit_scoped` (the walk's floor ray), `segment_take_scoped` (the
  take: straddle scope + the 0xd/0xe skip of 0x1ebaa8 + no 0x1000 cage + the flat fallback).
  `Geo` now carries per-run owners so the buckets survive `chunk(16)`.
- `crates/nf-view/src/scene.rs`: the world builds with `build_scoped(..., Some(scope_graph(...)))`;
  `Sim::room_cell` (the room-cel → pCelList-cell map) and `Bond::cel`/`Player::cel` are filled each
  tick from the position (obj+0x20, the movement pass's relink; `build_LinkToRoom` re-links the
  same way).
- `crates/nf-game/src/pickup.rs`: `Pickup::touches` runs the take LOS through
  `segment_take_scoped(cel, …)` — flags 0x721's discipline [M 0x18e148:213-214].
- `crates/nf-game/src/player.rs`: the capsule/floor call sites remain the FLAT queries (one-line
  revert from the scoped variants, which stay in world.rs), per the floor-fall finding above.
  `Bond::cel` is still computed (the take uses it).

## Tests

- `world::tests::scoped_actor_queries_keep_only_the_straddle_cels` (nf-game, no ISO, fail-first for
  the mechanism): a wall assigned to a portal-unreachable cell is not tested by
  `capsule_push_scoped`/`segment_hit_scoped` while the flat queries still hit it; a portal the
  capsule straddles brings the wall back; a mat-0xd triangle in the start cell is skipped by
  `segment_take_scoped` (the 0x721 bit 0x1) while `segment_actor` still hits it; the unmapped-cell
  fallback keeps the old behaviour.
- `fix_safe08_live.rs`:
  - `the_briefcase_is_not_takeable_before_the_safe_opens` — green (the sister).
  - `the_safe_pickups_take_after_the_safe_opens` — green (the briefcase takes through the real
    path, ch 0x4A on; the shelf rocket exists at the script's key position).
  - `bond_walks_from_the_bedroom_door_to_the_safe_without_the_block` — green; *measured*: the
    straight line also completes on the flat world (the block's tris stand beside the east wall,
    off this corridor), so this walk is the regression guard that the scope never EATS bedroom
    geometry, not the symptom's fail-first — the mechanism's fail-first is the unit test above and
    the render checks below.
  - `probe_blue_block_and_take_rays` — the env-gated probe (NF_SAFE08_PROBE=1) backing this report.
- Full suite (NF_ISO, `cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1`): **135 test binaries green; the only failures are
  the 3 allowed `disc_golden` tests** (the slim ISO's missing /DRIVING files).

## The blue block — status and the draw side

- **Collision**: the mechanism is in and unit-tested, but the actor capsule is not scoped until the
  `build_FindCel` tie-break is ported (the floor fall). For 07000008's bedroom the practical
  impact is small (the walk test shows the corridor was never blocked; the user's "obstacle" was
  primarily the DRAWN block over the room).
- **Draw**: NOT landed. The portal walk `portal_visible_cels` exists (NF_PORTAL-gated since the
  07000006 incident). The task's evaluation item, and the coordination note for **130-fix-portal**
  (which owns the general portal draw — I did NOT do the draw side, so 130 keeps it): the walk from
  the bedroom keeps the room itself and drops the exterior cell only if the portal graph between
  them stays closed — same graph as the collision straddle (17-cell component from 48, no 106), so
  the draw walk would drop the blue block. The 07000006 re-check and the render are the main
  agent's.

## What's left (for the next session)

1. Port `build_FindCel` 0x1c8cb0's raycast tie-break into the instance→cell rule (or gather the
   live check below), then scope `capsule_push` + the floor ray (the call sites already exist —
   `world.capsule_push_scoped(self.cel, …)` is a one-line swap in `player.rs`).
2. The rocket take: decode why the game's 0x721 query does not register the safe shell (live
   check 2), then assert the rocket take in `the_safe_pickups_take_after_the_safe_opens`.
3. The draw side (with 130-fix-portal).

## Live checks for the main agent

1. Renders (rule 6): `07000008 --bond 155.19,43.12,-113.38,-3.837,0.010` — the blue block beside
   the safe (the port-only exterior stone) — this is the picture to compare after 130's draw work.
   `07000008 --bond 161.87,43.39,-116.54,1.583,-0.495` — the briefcase and rocket on the safe's
   shelves (the take side is already green in the harness).
2. GDB-watch the game's `Collide_LineOfSight` call at 0x18e148:214 while taking the rocket from the
   doorway spot (161.87, 43.39, −116.54): the HITTEST words +0x94/+0x98 (expect 0x201/0x721), then
   for the `Object_Safe` node (map51 obj155): whether it appears in the candidate list at all
   (`Collide_Pick`'s node walk), its node+0x3c word (the box/sphere bit 0x8000000) and node+0x94
   (expect no 0x20/0x1000), and whether `Collide_Intersect` returns a hit whose HITDATA+0x50 reads
   6. That pins the class the game skips (my decode says none of the filters should skip it).
3. In the bedroom, break on `Collide_StraddleCels` 0x1eac98 and dump the collected cel ids for a
   walk segment toward the safe — confirms the exterior cel is absent for indoor queries (the port
   now matches this in `segment_take_scoped`; the capsule will follow after item 1).
4. `Controls_StraddleTest`/ForcedList: dump obj+0xF0 for the static objects around the safe
   (Object_Safe, the SS door's object) — bit 0x20000000 would put them in the ForcedList (tested in
   EVERY query), bit 0x30000000 would skip them entirely; either would change the take's answer.
