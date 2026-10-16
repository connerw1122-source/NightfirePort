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
