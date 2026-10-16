## Your task: `spurious-complete-16` — no stray "mission complete" on 07000016

Playtest 1013-1825 (07000016): "I also saw a mission complete thing pop up for some reason in this mission, but I
still couldn't continue". Find what posts it: a script setting channel 100 or an objective-complete message
mislabelled, the type-3 banner from a sub-objective, or an inherited channel [M Mission_Update, the 16 scripts'
channel writes, the objective message types]. Reproduce headless by walking 16's route (the 16 door/corridor tests have
fixtures) and logging every mission message. Fix: the banner shows only on the game's real completion. Test: the 16
walk posts no "Mission Complete" before the end condition; at the end it does.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/spurious-complete-16`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T14:40Z
