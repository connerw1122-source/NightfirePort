## Your task: `spu-gaussian-reverb` — GAPS G-032: the SPU2's Gaussian interpolation and room reverb

GAPS G-032: "sound: exact SPU2 Gaussian table, room reverb, music events". Port in nf-audio:
- **The SPU2's 4-tap Gaussian interpolation**, with the real 512-entry table. Source it, and cross-check against an
  independent emulator's table (PCSX2 and others are reference only; record the source).
- **The reverb the game configures per room / level:** the SPU2 reverb registers the game writes [M the SFX.IRX
  reverb setup calls and the level's room reverb data].

Golden tests (numbers only): the resampler on a synthetic VAG block matches the table's expected output, and each
level's reverb preset decodes to the register set the game writes. Keep tests silent (NF_SILENT).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/spu-gaussian-reverb`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
