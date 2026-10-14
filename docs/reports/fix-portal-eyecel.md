# fix-portal-eyecel — the 05 trench walls: the eye cel now resolves like the game's

Branch `s/fix-portal-eyecel` (from `origin/master` 5f13369). Session 04:10–05:3x UTC 2026-10-14.

## What was wrong

130g-fix-portal-sweep added `scene::near_los_rescue` for the trench spot, but the RENDERED frame
was unchanged: `nf-view 07000005 --bond 16.67,30.04,58.60,1.755,-0.001` still drew the trench with
the near stone walls missing, and `NF_PORTAL_DEBUG=1` printed the `[pwalk]` walk from cell 14 with
no `[rescue]` line — the rescue restored 0 cels in the real draw path. The root cause (per
docs/reports/fix-portal-sweep.md) is `build_FindCel` (0x1c8cb0)'s tie-break input, and this task
replaces the heuristic with the game's own per-cel attribution.

## The root cause, pinned with evidence

- **[D decompile 0x1c8cb0]** `build_FindCel`'s tie-break casts an up ray then a mirrored down ray
  per candidate through `Collide_RayIntersect(from, to, CANDIDATE, 0, 0, &hits, 1, 0x90)` — the
  candidate cel is the cel ARGUMENT, obj lists 0, the flags char = 1 [M disasm 0x1c900c: `a3 = 0`,
  `stack0 = 0`, `t2 = 1`, `t3 = 0x90`; the custom arg registers hold obj2/hits/flags/0x90].
- **[D decompile 0x1ec690 + disasm 0x1ec784-0x1ec794]** `Collide_RayIntersect`'s post-pass
  exact-tests a static hit only when `HITDATA+0x54` equals `HITTEST+0x80` (the passed cel).
  `HITDATA+0x54` is the static-geometry cel the hit belongs to [M docs/fields/
  hitdata-spawner-ally.md:41]; `Collide_Pick` (0x1eb320) writes it from the walked chain node
  [M disasm `sw $s3, 0x54($a0)` at 0x1eb8d8/0x1eba14, `lw $s3, 0x18($s3)` — the +0x18 chain
  walk]. So **each tie-break ray sees only the candidate's OWN chain geometry**; the straddle
  collects neighbour cels' nodes but the filter rejects every one of them.
- **[L ee_start_05.bin]** The game's chains at the trench: the trench room (game room 17, bbox
  (−20.5,−8.8,27.1)..(81.6,71.9,168.2) = the port's cell 15) chain holds **58 generic nodes
  (all 0x8019), spheres clustered over the trench**; the outdoor room (game room 18 = the port's
  cell 14) chain holds 41 nodes, **none near the eye** (closest sphere ~78 units away).
- **[D probe, pre-fix]** The port's stand-in (`find_cell_raycast` through
  `segment_hit_scoped(Some(cell))`) straddles scope buckets + the `always` runs — and BOTH
  candidates' rays missed everything (the trench floor run lives in neither candidate's bucket),
  so `0x1c8cb0`'s no-hit rule returned the FIRST candidate: the outdoor room 14.

## The fix (evidence-cited)

1. **`world::attribute_scope`** (`crates/nf-game/src/world.rs`): re-keys the cel-scope buckets by
   an explicit per-instance attribution — the collision side of the game's per-cel attribution
   (`build_link_objects_to_rooms` 0x1c9150 hangs every generic's collision on the room whose
   chain holds it). The live loader (`scene.rs`, after `build_chains`) replays the parse-wide
   cell counter to map each pushed object to its placement's cell, then a room cell keeps its
   own runs, a generic's runs join its chain owner's bucket, and unchained/unresolvable cells
   fall to the `always` bucket (the actor safety net, unchanged).
2. **`world::segment_hit_cell`** (`crates/nf-game/src/world.rs`): the first static hit of a
   segment in a cell's OWN bucket — the port's reading of the owner filter above. No straddle,
   no `always` (the game leaves unlinked objects untested). Static only: the game's
   dynamic-object hits (+0x54 = 0) pass its filter, but the port's dynamic set is not cel-keyed
   and Bond's own parts would tie both candidates — a documented [S] deviation.
3. **`scene::find_cell_raycast`** now feeds the tie-break rays through `segment_hit_cell`
   (the up/down/nearest/no-hit-first rules are unchanged, [M 0x1c8cb0]).
4. **The rescue stays.** With the eye cel fixed, the 27-level sweep's RAW walk failures dropped
   on 07000005 (1382 → 31) but remain elsewhere (07000001: 138, 07000002: 850, 07000003: 372,
   07000006: 210, 07000007: 759 …) — other documented causes, which the rescue covers. Removing
   it would regress the sweep, so `near_los_rescue` ships unchanged (additive; it still restores
   0 at the trench because the walk now visits the trench rooms itself).

## Verification

- **Failing first** [D pre-fix run]: `portal_eyecel_live::trench_eye_cel_is_15_and_the_near_walls_draw`
  panicked with *"the trench eye cel must be the trench room (cell 15), got 14"* on the pre-fix
  tree; it passes after the fix: **eye cell 15, 0 near visited-chain wall draws hidden**. The
  test drives the exact `--shot` draw-list inputs (`scene::eye_cell` +
  `portal_visible_cels_game_from_cell`, the renderer's World-draw cull) and asserts every
  node-keyed generic within 45 units of the eye that the frustum keeps with clear static LOS and
  a visited chain is kept by the mask.
- The 27-level sweep (`sweep_every_level_near_los_never_culled`) and the four ground-truth spots
  (`ground_truth_spots_stay_put`) stay green: 06 courtyard eye cel 33 / rescue 0 (masonry
  verdict unchanged), 06 start 29, 07 ballroom window rescue 0 (the lit building stays hidden),
  07 clock hall eye 33 with the clock (50) + shields (45/46/269/270) visible — the rescue adds
  the same one near prop (94) as before.
- Headless `--shot` commands for the main agent (the two images must show the same walls):
  ```
  nf-view Nightfire-onfoot.iso 07000005 --bond 16.67,30.04,58.60,1.755,-0.001 --skip-cutscenes --frame 30 --shot trench_portal.png
  NF_NOPORTAL=1 nf-view Nightfire-onfoot.iso 07000005 --bond 16.67,30.04,58.60,1.755,-0.001 --skip-cutscenes --frame 30 --shot trench_noportal.png
  ```

## Suites

- On the final tree (origin/master merged): **nf-game 256 passed / 0 failed** (33 binaries, run
  twice, deterministic), **nf-view 318 passed / 0 failed** (87 binaries), the portal test set
  (portal_eyecel_live 1, portal_sweep_live 3, fix_portal6_live 4, fix_portal3_live 4) green.
- nf-audio + nf-formats (pre-reset run, same code): green except the 3 allowed `disc_golden`
  failures on the slim ISO.
- The full `tools/test/suite.py` wrapper could not complete in this sandbox: background jobs are
  killed between tool calls here, and the wrapper's single run exceeds the 10-minute tool limit
  after the mid-session sandbox reset; the same per-binary runs were executed directly instead
  (the wrapper runs the identical binaries one at a time, `--test-threads=1`).

## Files

- `crates/nf-game/src/world.rs` — `attribute_scope`, `segment_hit_cell`, `has_scope` (task file).
- `crates/nf-view/src/scene.rs` — `find_cell_raycast` reads the own-bucket query; the loader
  re-keys the scope after `build_chains` (task file; the shared-file hooks are the two additive
  blocks marked `fix-portal-eyecel` — no shared struct/constructor lines touched).
- `crates/nf-view/tests/portal_eyecel_live.rs` — the failing-first rendered-path test (new).
- `docs/WORKLOG.md` — the task entries (append-only).

## What's left

- The remaining raw-walk failures on the other levels (the sweep's pinned counts) — the eye-cel
  fix removes the wrong-eye-cel cause at the trench; the other causes the rescue covers are
  unchanged and still need their own attribution work if the rescue is ever to die.
- `segment_hit_cell` treats the tie-break as static-only; if a live check ever shows the game
  resolving an eye cel through a dynamic object's collision, the port needs cel-keyed dynamic
  sets to match.
- The sandbox reset mid-session (05:08 UTC) cost the local tree; the branch was pushed before it,
  so no work was lost — the final suites were re-run on the re-cloned tree.

## Live checks for the main agent

1. **The trench render** (`--bond 16.67,30.04,58.60,1.755,-0.001 --skip-cutscenes --frame 30`):
   the near stone trench walls draw solid (no sky through them) and match the `NF_NOPORTAL=1`
   shot; the wall-hug marker near grapple point #282 shows. With `NF_PORTAL_DEBUG=1` the run
   prints the walk FROM CELL 15 (the `[pwalk]` line) and no `[rescue]` line at the spot.
2. **The game's eye cel at the trench [L]** (already requested by fix-portal-sweep, still the
   pin): read `Vision_Portal_Recurse` 0x1e86e8's `*(int *)(viewer+4)` with Bond held at
   `16.67,30.04,58.60` — expected: the trench room's record (game room 17), i.e. the cel whose
   +0x18 chain holds the 58 trench generics walked in this task's EE dump.
3. **The ground-truth spots still render right** (`--bond`): the 06 courtyard
   `7.24,42.03,-39.31,10.161,0.428` (masonry solid), the 07 window
   `61.11,44.12,-52.58,-2.541,-0.010` (the lit building NOT drawn), the 07 clock hall
   `34.25,44.13,-70.71,1.549,-0.070` (clock + shields visible).
4. **Per-cel attribution spot check [L]** (the sweep's request, still open): `gdbcall.py` on
   `Collide_RayIntersect` (0x1ec690) during a `Camera_CheckLocation` (0x1bae90) call at the
   trench — the exact-tested hits' `+0x54` should be the trench room's record for the down ray,
   confirming the owner-filter reading this fix implements.
