## Your task: `drive-track-mesh`, the driving missions' track geometry

503 documented the CRP track format (docs/formats/crp.md; the VIV archives via 500's refpack). Decode the track's
geometry into meshes the port can use: vertices/indices/UVs/texture refs per section, plus the collision/road
surface data if separable. A `nf_formats` module + an `nf-extract` command (e.g. `crp-mesh <iso> <viv> <out>`
writing OBJ for local inspection, never committed). Cross-check: counts and bounds per track match what the format
headers declare; a golden of per-section counts + SHA-1s of the decoded index/vertex buffers (numbers only). No
renderer work (the main agent wires drawing).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-track-mesh`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-15T19:51Z

---
## Report (done, 2026-10-15T21:14Z)

