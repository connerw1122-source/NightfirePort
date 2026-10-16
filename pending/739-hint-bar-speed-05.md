## Your task: `hint-bar-speed-05` — the 05 hint bar runs far too fast

Playtest 1016-1355 (the user, on master 2026-10-16): "the hints are being displayed at 1,000 miles an hour" (07000005). Find the hint bar's timing in the game: how long each hint shows, the scroll or fade rate, and what the per-tick step is [M the hint pane / context-hint update in the HUD code]. Then find why the port runs it fast: a per-frame step instead of per tick at uncapped fps, or a wrong constant. Test: a hint shows for the game's tick count at 60 Hz, and the same at 240 rendered frames per second (frame-rate independent).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/hint-bar-speed-05`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
