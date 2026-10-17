# level-load-time — measure and cut the level build time

Branch `s/level-load-time`, finish ID 652. Done in the 03:09-05:09 UTC window of 2026-10-16.

## What's done

**1. The probe (step 1 of the task).** `crates/nf-view/tests/load_time_probe.rs` (NF_ISO-gated,
release-only) times `nf_view::load` for every on-foot level on the disc — 27 archives, the same
definition `levels_sweep_live.rs` uses (0700xxxx.bin minus the driving missions 07000023-29 and the
shared packs 07000500/07000999) — cold (first load of the level in the process) and warm (the
second load), broken down by phase. The phases are marked inside the real load path
(`load_impl` in `src/lib.rs` and `Scene::build` in `src/scene.rs`) through a thread-local log
(`nf_view::phase_mark` / `phases_reset` / `phases_take`, ~30 marks per load, wall-clock only for
measurement — nothing feeds the Sim).

Baseline probe (pre-optimization tree, warm runs, totals over all 27 levels — top first):

| phase | total s | share |
|---|---|---|
| 07 Scene::build (total) | 17.61 | |
| — b5 characters + attachments + pools | 8.18 | 47% of build |
| — b6 materials assembly | 7.65 | 43% of build |
| 04 weapon-models merge (session mesh merge) | 6.95 | |
| b8 floor_below | 0.85 | |
| 09 sound banks + streams | 0.78 | |
| 03 level+levelbin parse, skeletons, 07F sequences | 0.70 | |
| b9 sim create | 0.53 | |
| everything else | ~1.0 | |

Whole-load warm totals, worst levels: 07000014 1.469 s, 07000015 1.299 s, 0700004b 1.245 s,
0700004a 1.221 s, 07000041 1.214 s … 07000004 0.590 s (full table in the probe's stdout; the
per-level × per-phase matrix ran 27 levels × 2 passes in ~75 s).

**2. The cuts (step 2), all load-side only, renderer untouched:**

- **Pool-slot replay** (`scene.rs` `pool_snapshot`/`pool_record`/`pool_replay`): the
  projectile (GRENADE+TRACER+PLAYER_BULLET slots per mesh), tracer (32), gas, decal (200),
  casing and debris (128) pools called `add_instance` once per pool slot with IDENTICAL
  geometry (same object, same identity matrix — only the attachment index differs). Slot 0 now
  decodes and its group pushes are recorded; slots 1..N replay them bit-exactly (indices
  re-based on the current group length; radius via max — the replay's `r_record` is exactly
  max(glist radius, all batch bounds) of the same geometry). b5c (projectile pools) went
  7.74 s → 4.69 s over the 27 levels.
- **Reserved final arrays**: the groups→scene assembly (`vertices`/`indices`/`char_verts`)
  pre-reserves the exact totals before the extend loop (it used to realloc through several
  doublings of ~500 MB of vertex/CharVertex traffic). b6 went 7.65 s → 6.20 s.
- **Parsed-foreign-archive cache** (`lib.rs` `foreign_level_cache`): `merge_weapon_models` — the
  session object table merge [M `LoadableLoad` 0x1d69e8 / the session-table mechanism documented
  at the call site] — re-parsed every earlier story archive on EVERY load (5-9 `Level::parse`
  per level). The parsed archives are now cached per archive name (immutable after parse; taken
  maps CLONE out — `LevelMap`/`MapObject` gained `#[derive(Clone)]` in nf-formats, a clone is
  bit-identical to the move it replaced), bounded FIFO of 4 entries (memory cap; the scan order
  re-visits the same archives across consecutive loads so the small window hits). 04 went
  6.95 s → 6.48 s (the per-parse cost that remains is the misses in the 4-entry window and the
  map clones).

After-probe, warm whole-load totals, worst levels (baseline → after):
07000014 1.469 → 1.236 s, 07000015 1.299 → 1.155 s, 0700004b 1.245 → 1.091 s,
0700004a 1.221 → 1.123 s, 07000041 1.214 → 1.071 s, 07000016 1.180 → 0.975 s,
07000005 0.696 → 0.448 s, 07000002 0.683 → 0.492 s, 07000004 0.590 → 0.387 s.
Build total 17.61 → 13.36 s (-24%); whole-load total ~24.5 → ~21 s over the 27 warm loads.

**3. The proof (step 3).** `crates/nf-view/tests/load_scene_digest.rs` fingerprints every built
scene fact (FNV-1a over vertex pos/uv/rgba/nrm bits, indices, every material field, CharVertices,
characters, attachments, colliders + sim drone/pickup/door/bond facts + the stats line). It was
run on the PRE-optimization tree for 5 levels — 07000002, 07000005, 0700000c, 07000014, 07000001
(the two heaviest, the archive-heavy mid-game pair, the smallest) — and those digests are pinned
as constants in `crates/nf-view/tests/load_scene_equality.rs`, which rebuilds each level through
the current path and asserts byte-identical. All 5 match after the cuts (verified three times:
after the pools+assembly, after the merge cache, after the FIFO-4 widening). One regression was
caught and reverted by this harness mid-task: the assembly's per-group bbox re-scan looked like a
no-op next to `add_instance`'s batch fold, but non-World groups merge instances with different
centres under one key, so the pass is the only sound cull radius — it is kept, with the proof in
a comment at the site.

## How it was verified

- `load_scene_equality` (the 5 pinned digests): PASS (6.3 s).
- `async_level_load_live` (the task's reusable comparison): 3/3 PASS.
- `load_time_probe` + `load_scene_digest`: PASS on the final tree.
- nf-view lib tests: 15/15 PASS.
- nf-view integration, the scene-sensitive set (every test that exercises the changed code
  paths — pools, cull radii, decals, casings, effects, portals, weapon models): frustum_cull_live
  2/2, decal_order_live 1/1, casings16_live 4/4, impacts_live 2/2, world_fx_live 4/4,
  fix_flesh_impacts_live 5/5, fix_weapon_effects_live 2/2, portal_live 4/4, fix_portal2_live 6/6,
  fix_render_artifacts_live 4/4, models_weapons_live 4/4 — 38/38 PASS; the merge-cache set
  (weapon_assets_live 6/6, fix_weapons_all_live 4/4, fix_start_loadouts_live 5/5, mp_core_live
  5/5) and the real-path smokes (walkreal_0700000c 9/9, level_reload_live 1/1) — 30/30 PASS.
- nf-formats: 11 passed / 3 failed = exactly the 3 `disc_golden` tests the slim ISO allows to fail
  (bigf header/tables/directory golden manifests).
- nf-game / nf-audio are untouched by this branch (no source change); their crates were not rerun.

## What's left (suggested next passes)

- **b5c projectile pools, 4.69 s**: the replay removed the transforms; what remains is the
  memcpy of the replicated geometry itself. Removing the replication needs the scene to store
  shared geometry per pool mesh (a renderer-visible format change — out of scope here by the
  task's scene-equality constraint).
- **b6 assembly, 6.20 s**: the extends into `vertices`/`char_verts` are one full copy of the
  final arrays; writing add_instance's output into the final arena directly (groups holding
  ranges instead of Vecs) would remove it — a larger refactor of the hot function.
- **04 merge, 6.48 s**: raise the cache window above 4 (memory-risky) or cache only the taken
  weapon-model maps keyed by (archive, map) with a full wanted-union growth policy — designed,
  not implemented (time box).
- The full `tools/test/suite.py` fast run did NOT fit the sandbox window (background jobs are
  reaped between tool calls here; the foreground cargo runs above are what completed — the
  suite's total line is not reported).
- Live checks for the main agent: none needed (load-side only, no visual change; the scene
  equality harness is the check). A `--bond` render spot-check on 07000005 spawn
  (-2.78, 8.35, -12.40, yaw ~0) before/after this branch would confirm the draw output is
  untouched by eye.

## ntfy

(delivered — see the finish log; if it had failed, the exact error output would be here.)
