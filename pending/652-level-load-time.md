## Your task: `level-load-time` — measure and cut the level build time

`nf_view::load` builds a scene from the level archive. Since 620 (async-level-load) that runs on a worker thread
while the loading screen animates, but the wait is still the user's: the 07000005 load reports 2.9M triangles,
2675 materials and 8561 attachments.

1. Add an NF_ISO release-only probe test that times `nf_view::load` for every single-player level, cold and warm,
   broken down by phase: archive read/decompress, mesh/material decode, the character and attachment build, the
   effect pools, the cel/portal graph, the Sim create, and whatever else is big. Table it in the report.
2. Cut the top costs without changing the built scene. Candidates:
   - redundant decodes of the same mesh per pool slot (the decal/casing/debris/gas/projectile pools call
     `add_instance` once per slot with identical geometry);
   - repeated `placements.first().cloned()` / `level.by_hash` lookups in hot loops;
   - per-material work that could be done once.
3. **Proof:** the scene built after your change must equal the one before on at least 4 levels. Compare vertices,
   groups, characters, attachments, colliders, cels and stats; the async-level-load live test already has a
   comparison you can reuse. Report the before/after times.
Do not change the renderer or the draw output; this is load-side only.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/level-load-time`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
