## Your task: `frame-cost-guard` — headless per-frame cost budgets so the lag can't come back

The user's playtest on 2026-10-15 was "very laggy": 07000005 gameplay ran at about 52 ms a frame. The main agent
measured `Scene::colour_mults` at 34 ms of that. It walked every character vertex (`char_verts`), and the effect
draw pools (decal/casing/debris/gas/projectile slots) push 8561 attachments onto 73 characters. The fix (master,
"fix the in-game lag") computes once per pickup and per attachment; `skin_palette` is now about 1.4 ms on 07000005.

Do two things:
1. **Budget tests:** add NF_ISO-gated tests that time the per-frame CPU work headless on every single-player
   level archive: `skin_palette`, `frame_state`, `colour_mults`, `posed_vertices` if it is still on any per-frame
   path, the HUD/menu overlay builders, and `cel_visibility` if it can run without a GPU. Take the median of N
   runs after warm-up. Assert generous budgets (for example 5 ms for the palette on a release build), and print
   the measured numbers so the report can table them. Keep the tests release-only (`#[cfg(not(debug_assertions))]`
   or skip when `cfg!(debug_assertions)`) so debug runs don't false-fail.
2. **Audit:** find any other per-frame path whose cost scales with the vertex count or the pool sizes rather than
   with the live objects (look at every loop over `char_verts`, `vertices`, `attachments` and `groups` reachable
   from the window's RedrawRequested path in `crates/nf-view/src/lib.rs`). Fix the ones that cost measurable time,
   with before/after numbers. Results must be bit-identical: prove it by comparing the palette/colour output
   before and after on 3 levels.
Sister assertion: a deliberately slow closure must fail the budget helper.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/frame-cost-guard`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T02:01Z

---
## Report (done, 2026-10-16T03:13Z)

Closed by the driver: the report is on `s/frame-cost-guard` (docs/reports/frame-cost-guard.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T03:45Z)


---
## Report from the worker (done, 2026-10-16T03:47Z)

# frame-cost-guard — headless per-frame cost budgets + the audit

Branch `s/frame-cost-guard` (merged with origin/master 4bdf7678). Task: pin the per-frame CPU work the
`RedrawRequested` handler runs with NF_ISO-gated budget tests on every single-player level archive, and audit
for any remaining loop whose cost scales with the vertex count or the effect-pool sizes rather than with the
live objects — the class of bug that made the 2026-10-15 playtest "very laggy" (52 ms frames on 07000005, 34 ms
of it `Scene::colour_mults` walking every character vertex; master's c49f8b69 fixed that one).
Per-level phases are the ones the game window's `RedrawRequested` handler runs (lib.rs:2602 on this
branch):

## What's done

**1. The budget tests** (`crates/nf-view/tests/frame_cost_live.rs`, all `#[cfg(not(debug_assertions))]` so
debug runs don't false-fail, all NF_ISO-gated):

- `frame_budgets_hold_on_every_single_player_archive` — all 34 single-player archives: the 27 on-foot
  FILES.BIN entries (`levels_sweep_live.rs:26`'s enumeration) plus the 7 driving mission archives 07000023-29
  (`docs/systems/levels.md:91` [D]; the front-end's driving rows 0900000x resolve to these). Per level: load,
  120 `step_bond` ticks to the live state, then median-of-7 after 2 warm-ups (median-of-5 for the heavy
  `posed_vertices`) of each phase the window's redraw runs (lib.rs:2460 chain):
  - `Scene::frame_state` (scene.rs:5994) — budget 8 ms,
  - `Scene::colour_mults` (scene.rs:6802) — budget 8 ms,
  - `Scene::skin_palette` (scene.rs:6678) with the real room mask — budget 8 ms,
  - the `Renderer::cel_visibility` composition (gpu.rs:567) replicated headless with the pub scene pieces
    (`extract_frustum` + `depth0_game` copy + `eye_cell` + `portal_visible_cels_game_from_cell_doors` +
    `near_los_rescue` + `portal_visible_cels_from_cell_doors` + the `room_to_cell` map — the exact
    `cutscene_models_live.rs::render_census` recipe; the method itself needs a wgpu device, its body is pure
    CPU) — budget 3 ms,
  - `hud::overlay` (hud.rs:293) — budget 3 ms,
  - `Scene::posed_vertices` — timed, printed, **no budget**: it is OFF the window's per-frame path (its only
    caller is the `NF_BENCH` block, lib.rs:1462; the render path is `skin_palette` + `update_skin`), and its
    cost is level-dependent by design (measured 0.4 ms on 07000041, 716 ms on the 74-character driving maps).
    The existing render3_live counter test guards its per-key ambient lookups.
- `menu_overlay_budgets_hold_on_every_page` — `menu::menu_overlay` (menu.rs:3980) over the six front-end pages
  with menus_flow.rs's synthetic font/textures — budget 3 ms per page.
- `colour_attach_map_covers_every_slot_on_the_lag_levels` — the invariant that kept the deleted fallback below
  dead (see fix 1): on 07000005/06/07 `colour_mults`' `attach_mult` covers every index `0..attachments.len()`.
  If a future change makes the fallback reachable again, this fails instead of the palette silently changing.
- `a_deliberately_slow_closure_fails_the_budget_helper` — the sister assertion (no ISO, no release gate): a
  25 ms closure against a 5 ms budget panics (caught and asserted), a sub-microsecond closure passes. Measured:
  slow 25.062 ms → panicked; fast 0.000 ms → passed.

**2. The audit + fixes** (bit-identical, before/after numbers below):

- **Fix 1 — `skin_palette`'s per-attachment fallback loop was dead code** (was scene.rs:6789-6796, 8561 hash
  lookups per ticked frame on 07000005, 12724 on the biggest levels): `colour_mults` inserts EVERY index
  `0..attachments.len()` into `attach_mult` whenever the sim is Some (scene.rs:6849's `for ai in
  0..self.attachments.len()`), and the fallback only ran under `has` — `!mults.is_empty()`, which implies the
  sim is Some — so `!attach_mult.contains_key(&a)` was always false and the body never executed. Deleted, with
  the proof in the comment and the invariant test above pinning it.
- **Fix 2 — `colour_mults`' two HashMaps are now preallocated** to their exact loop bounds
  (`with_capacity(pickups.len())` / `with_capacity(attachments.len())`): one entry per pickup and per slot is
  inserted unconditionally, so the maps no longer rehash-grow to 12k+ entries every ticked frame.
- Nothing else that scales with pools/vertices was fixable bit-identically headless — see "Left / candidates"
  for the rest, with reasons.

**3. Bit-identity proof.** The test prints FNV-1a hashes of `skin_palette`'s palette and colour table and of
`colour_mults`' three outputs at t=120 and t=121 per level. Before-fix run (agent-ctx/frame_cost_before.out,
34 levels × 2 ticks = 68 hash lines) vs after-fix run (frame_cost_after.out): **all 68 lines identical** — not
just 3 levels, every archive.

## Measured numbers (median ms, release, this sandbox's 2 cores)

`frame_state` is untouched by the fixes (its max is 07000014 at 3.0 ms). `colour_mults` and `skin_palette`
before → after:

| level | chars | attach | colour_mults | skin_palette |
|---|---|---|---|---|
| 07000005 (the playtest level) | 73 | 8561 | 0.468 → **0.169** | 1.806 → **1.322** |
| 07000007 | 135 | 8446 | 0.452 → 0.164 | 3.817 → **3.243** |
| 07000014 (worst palette) | 134 | 12168 | 0.551 → 0.251 | 4.051 → **3.580** |
| 07000048 (front-end map) | 0 | 11421 | 0.520 → 0.226 | 0.748 → **0.450** |
| 07000023 (driving) | 74 | 12645 | 0.546 → 0.258 | 1.215 → **0.886** |

Across all 34 archives: `colour_mults` 0.45-0.59 → 0.16-0.32 ms (−52…−65 %), `skin_palette` −0.3…−0.5 ms per
ticked frame. The other phases are already far under budget everywhere: `hud_overlay` ≤ 0.033 ms,
`cel_vis` ≤ 0.75 ms (worst 07000012), `menu_overlay` ≤ 0.014 ms. The full 34-row table is in the test output
(`[cost]`/`[budget]`/`[hash]` lines); the committed budgets (8/8/8/3/3 ms, 3 ms per menu page) leave ≥2.2×
headroom on the worst level.

## How it was verified

- The budget test end-to-end on the pre-fix tree (34/34 pass), then the fixes, then the same test on the
  post-fix tree (34/34 pass), then the merge with origin/master 4bdf7678 and the same test re-run on the
  merged tree (172 s, pass; the 68 hash lines diffed against the pre-fix baseline — still identical).
- Targeted regression set on the merged tree: gpu_skin_live 3/3, cutscene_models_live 4/4, fix_portal_live 6/6,
  render3_live 3/3, portal_eyecel_live 1/1, door_portal_live 2/2, fix_portal3_live 4/4 (+2 ignored),
  character_lighting_live 4/4, fix_hud_live 6/6, menus_flow 37/37, nf-game lib 212/212.
- Full suite (tools/test/suite.py): the 2-hour window did not fit all 372 binaries (two runs; the walkreal_*
  binaries alone take ~9.5 min each). The final merged-tree run completed **144 of 372 binaries, 143 passed,
  1 failed** — the pre-existing `walkreal_07000001_live` AI-nav failure below; 0 TIMEOUT/CRASH. The binaries
  ran on the merged tree (125abbd7's build; the two later commits are comment/doc-only, codegen-identical).
  The remaining ~228 binaries did not fit the window — the next window should run the suite to completion.
- One failure in the suite runs is pre-existing and unrelated: `walkreal_07000001_live::
  walk_from_the_spawn_to_the_exit_with_the_contact` ("the contact must walk his route to the
  LoadPointActivator plate", walkreal_07000001_live.rs:218) failed identically on the pre-fix tree (before
  the scene.rs fixes existed) and on the merged tree — an AI-nav assertion; this task touches only the render
  palette path and adds a new test file.

## Left / candidates (documented, not fixed — each with the reason)

1. **Uniform staging per frame scales with ALL materials** (gpu.rs:660-686, `Renderer::render`): `staged =
   vec![0f32; 64×(n_draws+1)]` (~685 KB zeroed on 07000005's 2675 draws), `staged.clone()` ×2, the view-proj
   written into every slot (identical in each), and `lights.pick2` per draw (a Vec alloc + sort ≤30 per draw)
   for culled draws too — plus 2×685 KB `write_buffer`s where every slot's vp is the same. ~0.5-1.5 ms CPU +
   ~1.4 MB uploads per frame by the audit's estimate. NOT fixed: `render` cannot run in this sandbox (no
   wgpu adapter), so no before/after numbers are provable here, and a staging restructure changes what the GPU
   consumes — it needs the main agent's `--shot` render comparison before merging.
2. **`lerp_palette` blends the full 7.3k-12.3k-entry palette every RENDERED frame** (the blend/upload block
   lib.rs:2969-2989 + `lerp_palette` at lib.rs:97), even entries bit-identical between ticks (the FAR pool
   slots, idle bones — most of the 8561 attachment slots). A per-entry `if x == y { return *y }` fast path
   would skip most of the work but is NOT bit-identical: `blend_pose(x, x, t)` round-trips through
   `rot_quat`/`slerp`/`quat_rows`, which differs from `*y` at ULP level — visually identical, but it breaks
   the strict hash proof and touches the fix-interp-ghosting path, so it needs a visual check first.
3. **`upload_skin` runs every rendered frame even when no tick ran and interp is off** (the
   `upload_skin` calls at lib.rs:2985/2987) — re-uploading ~0.9 MB of unchanged bytes. The skip
   (`!ticked && !interp`) is safe but untestable headless; bundle it with 2's visual check.
4. **`frame_state` poses hidden characters before checking `hidden`** (posed collected at scene.rs:5998-6053,
   `hidden` first used at 6372 inside the attach closure): the 12+ spawner-held-back characters get a full
   drone pose they don't need. NOT fixed: hidden owners' attachments collapse to `collapse(&datum)` where
   `datum` depends on the live pose — skipping the pose would change those collapse translations (not
   bit-identical).
5. `view_model_entries` scans all attachments per ticked interp frame for the view model's few (lib.rs:213) —
   tens of µs, cache-per-weapon-switch if it ever profiles hot.

## Live checks for the main agent

1. `--shot` on 07000005 spawn (and one interior, e.g. 07000001's start) before/after this branch: the skin
   palette render should be pixel-identical (my proof is headless-hash; a render sanity check is one command).
2. Candidate 1 (uniform staging): with a GPU, time `NF_BENCH=200`'s upload line before/after any restructure,
   and `--shot --bond <the playtest spots>` to confirm nothing vanishes (the staging feeds every draw's
   lights).
3. Candidates 2+3 (`lerp_palette` fast path + idle-upload skip): apply on a GPU box, then `--shot` at a
   mid-animation spot and an idle spot — the palette blend is the interp path, so look for ghosting/smear
   regressions (fix-interp-ghosting's spots) and confirm the fps-determinism headless tests still pass.
