# fix-08-collision2 (100m) — the scoped collision broke the 01 escort walk

100j-fix-08-collision (origin/s/fix-08-collision 4023ba9, held) scoped Bond's collision to his
straddled cels [M `Player_Collision` 0x194df0, `Collide_Pick` 0x1eb320]. On the merge with master
(ccae18) the full suite left ONE real failure:
`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` — the contact settled
at (−5.2, 6.3, 14.1) while the LoadPointActivator plate is at (−33.1, 2.3, 15.4). Both symptoms
are fixed the game's way; 08's invisible wall and the 09 stairwell keep.

## Reproduce and diagnose

Reproduced on the merged tree first (the test failed at 211 s, the file's other test passed).
The driver's suspicion (Bond stopped/fell/snagged) turned out to be one step upstream: with the
failure trace captured (`--nocapture`), the contact was stuck in `AllyLeadHide` [M 0x15ec08]
because west-room guard 16 stayed ALIVE (health 10) — Bond's shots at dist 1.2–10 with exposed
parts landed no damage, the ammo pool emptied (clip 1 spare 0, resupply found nothing), and the
contact hid forever. Guard 16 alive also kept the west door's unlock from firing, so Bond could
not cross west (master's walk crosses at t≈183180 to (−18.24, 6.47, 14.47) and up to
(−26.05, 10.30, 14.12); the merged Bond never got past x ≈ −14).

**The divergence hunt** (the driver's method): an env-gated per-60-tick position trace in the
shared walker (`NF_WALK_TRACE`), run on the merged tree AND on a master worktree (3103 vs 3682
samples). The walks agree tick-for-tick until **t2460**, where Bond's path through the spawn
corridor shifts 3 cm — the scoped pass and the flat pass resolved the SAME wall push
differently. A second env-gated probe (`NF_COLLISION_DEBUG`, since removed) printed every
scoped-vs-flat disagreement with the per-cel bucket owners (`debug_bucket_owners`): the first
divergence sits at cel 60, pos (−2.62, 6.47, −1.12), owners `[(6, 7), (60, 5)]` — the pusher's
resolution depends on WHICH cel roots the straddle and in what order the triangles test.

## Root causes, with the game's evidence

1. **The straddle root is the game's FindCel relink, not the room bbox.** The port refreshed
   `bond.cel` through the coarse room-bbox mapping (`cel_at` → `room_cell`). The game's per-tick
   relink is [M] `build_LinkToRoom` 0x1c9680:35-40 → `build_FindCel` 0x1c8cb0: the room cels'
   TWO-RAY tie-break (the up ray from the sphere centre, then the down ray capped at
   min(up hit, len); the nearest hit wins; no hits → the first candidate) — so the actor's cel
   always carries the geometry under him. From the wrong root the straddle BFS keeps the same
   SET here (probe: both roots straddle {60, 6}) but walks it in a different ORDER, and
   `Collide_Pick`'s iterative push is order-dependent — the resolved push differs by ~3 mm per
   tick, enough to steer the walk off master's line and cascade into the escort fight. The port
   already had the tie-break (`find_cell_raycast`, fix-portal-eyecel's build_FindCel port; the
   third icons' arm test uses it every tick); the movement pass just never did.
2. **The portal reference point is the stored centre.** [M] `build_alloc_portal` 0x1c8928
   stores the 4 corners' average at portal+0x70 (`puVar2[0x1c..0x1e] = (c0+c1+c2+c3)·0.25`),
   and [M] `Intersect_Portal` 0x1ea7b8's 0x800 path projects the query segment against THAT
   stored point, then `vecutil_Dist2Tri` to the portal's two triangles against radius × 1.5.
   The port projected against corner 0 — wrong reference, the same test otherwise. (Measured
   note: on the 01 route's portals the two references agree — the straddle sets are identical
   under both — so this half is a correctness fix by citation; the walk's operative half is the
   root. Kept because it is what the game does, and the 08 floor-seam portals are the kind
   where a corner reference can disagree.)

## The fix

- `crates/nf-game/src/world.rs`: `ScopePortal` gains `centre: V3` (the corners' average, the
  +0x70 point) and `capsule_portal_hit` projects against it; `debug_straddle(cell, a, b, r)`
  added next to `debug_bucket_owners` so the straddle set is probeable from tests.
- `crates/nf-view/src/scene.rs`: `bond.cel`'s per-tick refresh is now
  `find_cell_raycast(&self.cel_graph, bond.pos, Some(&self.world))` — the [M 0x1c8cb0] relink;
  `scope_graph` fills the portal centres.
- The void-redo fallback from 100j stays (the scoped floor missing → the flat redo), and the
  09 stairwell + the 02 ladder head keep passing with the new root.

## Tests

- **Failing-first**: `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`
  failed on the merged tree (211 s, the contact stuck at (−5.2, 6.3, 14.1)) and with the portal
  fix alone (217 s — the root half is operative); it PASSES with both (204 s). The trace logs
  from both trees are the divergence evidence.
- `crates/nf-view/tests/fix_08_collision2_live.rs` (new, 2 tests, both green):
  `the_01_seam_scoped_pass_matches_the_flat_one` — at the t2460 seam, Bond's capsule with the
  game's root: the scoped push equals the flat push per component (the pre-fix tree fails this:
  the straddle from the coarse root tested the neighbour's run in a different order), the
  straddle set keeps ≥ 2 cels, and the walkable verdict agrees;
  `the_findcel_root_carries_the_floor_under_bond` (sister) — the root's own chain carries the
  floor under Bond ([M 0x1c8cb0]'s down-ray rule) and both roots print for the record.
- Regression set on the final tree, all green: `fix_08_collision_live` 4 (the missile-room
  invisible wall stays fixed), `walkreal_07000009_live` 2 (the stairwell), `fix_ladder_live` 4,
  `fix_ladder2_live` 4, `walkreal_07000002_live` 3, `fix_swim_vent5_live` 2, `fix_08_walls_live`
  3, `fix_wire2_live` 1, `walk_07000008_live` 5, `fix_safe08_live` 1, `qclaw_live` 3,
  `gadget_grapple_live` 3, `walkreal_07000003/0a/0b/0c/0d/11/12/16/1b/4a` sweeps,
  `walk_07000001_live`, `walk_07000004_live`, and `weapon_matrix_live`'s matrix_01/02/03 +
  the sequence test 4.

## The full suite (final tree, commit 6901d0b)

The sandbox kills background jobs (the nohup'd fast runner died twice last session), so the
suite ran in foreground chunks (`tools/test/suite_chunk.py`, untracked scratch — the suite.py
loop resuming from suite.log):

- **233 of the test binaries completed: 975 passed, 3 failed, 31 ignored** — the 3 are exactly
  the allowed slim-ISO `disc_golden` trio (`bigf_header_fields_behave_as_documented`,
  `bigf_tables_match_the_golden_manifest`, `directory_tree_matches_the_golden_manifest`).
- `weapon_matrix_live` hit the 240 s per-binary cap (the sandbox is slow; the binary needs
  ~15 min here) — a cap, not a failure; its matrix_01/02/03 + the sequence test re-ran green
  separately (4/4). The remaining ~40 binaries (the level-sweep tail) did not fit the window;
  the collision-affected set (every walkreal/walk/fix-collision/ladder/wire/swim/grapple binary)
  ran green in the sweeps above.
- `fix_08_collision2_live` 2/2 in the logged run.
- A final `origin/master` merge (8e4c4d6, b65d6f7's 10 commits) landed after the chunked run;
  the collision set re-ran green on it (`fix_08_collision2_live` 2, `fix_08_collision_live` 4,
  `walkreal_07000009_live` 2).

## Live checks for the main agent

PCSX2 (the real machine — the port's collision should now match it at the seams):

1. The 01 escort: from the spawn, walk the route (kills allowed) to the plate at
   (−33.1, 2.9, 15.4) — the contact must arrive with Bond and the exit loads 07000002. The
   corresponding nf-view render: the walk test drives it headlessly (the plate is reached).
2. The 01 spawn corridor seam (Bond at `--bond -2.62,6.47,-1.12` facing north): walk north along
   the corridor — no snag, no sideways drift against the seam wall; the same walk on master and
   on this branch should feel identical.
3. The 08 missile room (`--bond 155.19,43.12,-113.38,1.57,0`): the exterior wall still blocks
   (no invisible wall returns — `fix_08_collision_live` pins it headlessly).
4. gdbwatch (optional): the write watch on `obj+0x20` (the player's relink) during the 01
   corridor walk — the relinked cel should flip exactly where the corridor's doorway portals
   are crossed, matching `find_cell_raycast`'s picks.

## What's left

- The ~40 suite binaries the window cut (the level-sweep tail; `weapon_matrix_live`'s full 28 —
  its spot check is green). The next window should run the full 226+-binary loop on this tree.
- The portal-centre half has no measured behavioural delta on the 01 route (the report's
  measured note above); if the main agent's suite or a playtest surfaces a seam where the
  corner reference mattered, the centre fix is already in place.

## Files

`crates/nf-game/src/world.rs` (the portal centre + the projection + `debug_straddle`),
`crates/nf-view/src/scene.rs` (the FindCel root + the centre init — shared file, the two hooks
listed), `crates/nf-view/tests/common/walker.rs` (shared test infra: the env-gated
`NF_WALK_TRACE` per-60-tick position trace), `crates/nf-view/tests/fix_08_collision2_live.rs`
(new), this report, the WORKLOG entries. `crates/nf-game/src/player.rs` ended unchanged (the
session's diagnostic probes were removed before the commit).
