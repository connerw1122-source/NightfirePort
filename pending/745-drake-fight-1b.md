## Your task: `drake-fight-1b` — the 0700001b final fight's mechanics

Playtest 1016-1355 (the user, on master 2026-10-16): "none of the mechanics are set for this fight properly. This one's going to require a lot of testing." Port the 1b boss fight's logic from the decompile and the level scripts [M + D]: Drake's AI and phases, the missile launch sequence (the missile channels 1..8, `MissileDeploy`, `SpaceMissile_Update`'s states), and the win/fail conditions. Tests: a scripted headless run reaches each phase on the game's triggers, and the mission completes or fails on the game's conditions.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drake-fight-1b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
