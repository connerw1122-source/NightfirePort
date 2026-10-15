## Your task: `drive-car-models`, the driving missions' car models

From the driving VIV archives (500 refpack, 501 the DRIVING.ELF map in docs/atlas/driving-elf.md), find and decode
the player's and the AI cars' models (format, LODs, textures, wheel attachment points). A `nf_formats` module + an
`nf-extract` command dumping OBJ locally (never committed). Golden: per-model vertex/triangle counts + buffer
SHA-1s. Document the format in docs/formats/. No renderer work.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-car-models`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-15T19:59Z
