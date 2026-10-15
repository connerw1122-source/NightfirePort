## Your task: `drive-ai-traffic`, the driving missions' AI cars

From DRIVING.ELF (docs/atlas/driving-elf.md): the enemy/traffic cars' AI — path following on the track's AI
splines (find them in the CRP/VIV data), speed control, attacks (the game's chase/shooting behaviours), and their
spawn/despawn per mission (MIS01.VIV's mission scripts). Port the data (the splines, the per-mission spawn lists)
and the AI step in `nf_game::driving` beside 506's physics (merge `origin/master` first; coordinate through the
data model if 506 hasn't landed). Tests: a headless AI car follows its spline at the game's speed numbers.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-ai-traffic`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-15T20:37Z
