# fix-08-collision — the invisible wall's collision in 07000008's missile room

Task 100j. The 2026-10-13 night playtest (07000008 at `--bond 155.19,43.12,-113.38,1.57,0`, portal
culling ON): "the wall is still there but invisible". fix-08-walls (101h) already pinned the
visual half: the exterior `Room_Outside_3` (graph cell 15) wall slabs cull indoors. This task is
the collision half — the same slabs' triangles still blocked Bond inside `Room_Guardhouse_F2`
(graph cell 48) because the port's movement pass tested one shared world.

## Root cause, with the game's evidence

The port's `Bond::physics_tick`/`physics_tick_climb` called the FLAT
`CollisionWorld::capsule_push`/`segment_hit` — every static triangle of the level, one shared
world. The game scopes the whole pass to the actor's own cel:

- [M] `Player_Collision` 0x194df0, the function's first store: `*(BLData+0x7c0) = *(obj+0x20)` —
  the actor's relinked cel pointer goes into the BLData+0x740 HITTEST's start-cel field
  (HITTEST+0x80). The capsule points are the same HITTEST's +0x760/+0x770.
- [M] `Collide_Pick` 0x1eb320: `Collide_StraddleCels` 0x1eac98 returns the straddle cel array; the
  walk reads each straddled cel's OWN object list head (`*(cel + 0x18)`) and tests only those
  nodes. Nothing outside the straddle set is ever tested.
- [M] `Player_CollisionHandler` 0x1949a8 runs the floor test (`Player_FeetOnPoint(param_1,
  BLData+0x740, …)`) off the same HITTEST — the floor ray shares the per-cel scope.
- [D] The slabs: cell 15's instance (`Room_Outside_3`, map51 pl17) carries 64 collision triangles
  inside the guardhouse box (bands x 155.0..165.9, y 36.75..57.4, z −125.0..−118.0 — the west face
  at z −118.0); the missile room (cell 48) spans x 154.5..163.1, z −122.33..−112.83.

So a triangle hanging on another room's cel never blocks Bond indoors — culling and collision
share the per-cel attribution. The port already had the machinery
(`capsule_push_scoped`/`segment_hit_scoped`, the straddle walk, and fix-portal-eyecel's
`attribute_scope` chain-owner buckets) — the player just never used it.

## The fix

`crates/nf-game/src/player.rs`: the walk pass and the climb pass now run
`capsule_push_scoped(self.cel, …)` + `segment_hit_scoped(self.cel, …)` — `self.cel` is obj+0x20,
the Sim's per-tick relink (`cel_at(pos)` → `room_cell`, the `build_LinkToRoom` 0x1c9680 order).
`None` keeps the flat fallback.

**The void-redo deviation [S].** The port's cel buckets are coarser than the game's cels, and the
game never lands over a void: `build_FindCel` 0x1c8cb0 relinks obj+0x20 by tie-break rays against
each candidate's own chain geometry, so the actor's cel always carries the geometry under him.
Over 07000002's ladder head the scoped pass sees neither a push nor a floor (the garden-floor rim
sits in a bucket the straddle walk doesn't keep) and Bond fell down the shaft he had just climbed
(fix_ladder_live caught it). When the scoped pass finds neither push nor floor, the tick is redone
against the flat world — the stand-in for that relink. On the ground the scoped pass always finds
the room's own floor, so the redo never fires there: across the missile-room slab band the floor
stays in cell 48's bucket while the exterior wall stays out of it (`debug_bucket_owners` probe:
the north-wall band runs live in cells 15/40/48, the room floor in 15/48).

**Kept as-is:** `bond.cel`'s resolution stays the pre-task `cel_at` + `room_cell` mapping. A
try with the raycast `find_cell_raycast` (build_FindCel's tie-break) resolved the user's spot to
cell 45 (the floor below's chain holds the room floor there) whose straddle set doesn't keep cell
48's buckets — the room's own north wall stopped blocking. The room-cell mapping keeps Bond's
straddle rooted at the room he stands in; the deeper FindCel port stays a follow-up (see Left).

`crates/nf-game/src/world.rs`: added `debug_bucket_owners(lo, hi)` — per-cell (and `always`, cell
= −1) counts of chunked runs meeting a box, so the per-cel attribution is probeable without
reaching into the private `ScopeData`.

## Reproduce before fix (shown failing)

`crates/nf-view/tests/fix_08_collision_live.rs` (opt-in NF_ISO), through the real input path
(`nf_view::load` + `Sim::step_bond`):

- `the_missile_room_walk_across_the_slab_band_meets_no_wall` — Bond at the user's spot, walking
  north (yaw π). At master the walk stopped at the slab face (crossing tick z −117.70, y 44.03 —
  the flat pass squeezing him UP the invisible wall) — FAILED. Post-fix the crossing rides the
  level floor (z −118.16 at tick 66, max y 43.748 — no climb), and the room's real north wall
  still stops the walk (z −121.78).
- Sisters: `the_missile_rooms_real_west_wall_still_blocks`, `the_missile_rooms_real_east_wall_still_blocks`
  (the per-cel attribution must not open the building), and
  `the_scoped_capsule_disagrees_with_the_flat_one_at_the_slab_face` — the mechanism pin: the flat
  push at the slab face shoves the capsule back (z +1.1), the scoped push with Bond's cel finds
  nothing, and the scoped push into the room's own west wall still fires (0.55).

Regression set on the fixed tree, all green: fix_08_collision_live 4, fix_08_walls_live 4,
fix_safe08_live 4 (the GuardRoom fusebox floor holds), fix_ladder_live 3+1 ignored (the bunker
exit climb works again), fix_ladder2_live 5, fix_swim_vent5_live 2, fix_wire2_live 3.

## Suite

Full `tools/test/suite.py` on the final tree (this commit): see the total line below (slim ISO —
the 3 `disc_golden` tests and `pss_movies_live` are the allowed failures).

- Baseline reference: fix-08-walls' merged-tree run (2026-10-14) — 932 passed, 3 failed (the
  allowed disc_golden trio), 27 ignored, 19 min.
- This tree: **(filled from suite_final.out)**.

## Files

- `crates/nf-game/src/player.rs` — the scoped movement pass + the void redo (task file).
- `crates/nf-game/src/world.rs` — `debug_bucket_owners` (task file).
- `crates/nf-view/src/scene.rs` — SHARED file: two comment blocks only (the bond.cel note at the
  relink site; no behaviour change — the find_cell_raycast try was reverted).
- `crates/nf-view/tests/fix_08_collision_live.rs` — the new test file.

## Live checks for the main agent

PCSX2, 07000008, the safe area (the 1013-1758 playtest spot):

1. Spot: stand at the missile room's south part near (155.2, 43.1, −113.4), walk north into the
   room's west half across z −118 (the invisible wall's old line). Expected in the game: no wall
   — Bond walks to the room's north wall (z ≈ −122.3) and it stops him. In nf-view (post-merge,
   `--bond 155.19,43.12,-113.38,1.57,0` then walk north): same — the old stop-and-squeeze-up at
   z ≈ −117.5 is gone (pre-fix the flat pass shoved Bond up the face, y 43.75 → 44.17).
2. Spot: the room's real walls. Walk west into x ≈ 154.5, east into x ≈ 163.1, north into
   z ≈ −122.3 — each stops Bond (sister assertions pin all three in the port).
3. Spot: the bunker-exit ladder on 07000002 (`--bond` the level start, walk into the ladder,
   climb). Expected: the climb tops out and Bond walks away on the garden floor (the void redo's
   case). If he falls down the shaft, the redo's condition (no push AND no floor) needs a look.
4. gdbwatch (optional, the attribution's ground truth): watch a word at the missile room cel's
   object list head (cel+0x18 of the runtime cel the game gives Bond at (155.2, 43.1, −113.4))
   during a north walk — expect the walked nodes to be the guardhouse's own, no `Room_Outside_3`
   nodes; and `Collide_Pick` 0x1eb320's straddle array (`HITTEST+0x80` = obj+0x20) to start at
   the missile room's cel.

nf-view render spots (`--bond x,y,z,yaw,pitch`):

- `155.19,43.12,-113.38,1.57,0` — the user's spot; the room looks normal, no exterior wall inside.
- `155.5,43.75,-119.5,1.57,0` — mid-slab-band, facing east: the room's furniture draws, the
  exterior slabs stay culled (fix-08-walls' pin, unchanged by this task).
- `155.5,43.75,-121.5,3.14,0` — at the room's north wall facing it: the wall is drawn (it is the
  room's own mesh, cell 48's bucket) and Bond's collision stops there.

## What's left

- The per-cel attribution's coarse edges: the buckets hang instances on ONE cell (the game's
  `build_link_objects_to_rooms` 0x1c9150 rule via fix-portal-eyecel's `attribute_scope`); shared
  meshes whose rim another room stands on (the 07000002 ladder head) still need the flat redo.
  Porting the game's exact FindCel relink for obj+0x20 (the raycast tie-break at the actor's
  position, with the chain-owner-filtered rays) would shrink the redo's surface; the first try
  regressed the missile room's north wall (cell 45 vs 48 — the room floor's chain lives below), so
  it needs the attribution and the resolver to land together.
- `ground_motion` (moving platforms) and the drone/guard movement passes still query the flat
  world; the game's per-cel rule covers them too (same `Collide_Update` path), but no playtest
  symptom points there yet.
