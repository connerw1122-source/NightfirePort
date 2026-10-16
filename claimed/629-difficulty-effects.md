## Your task: `difficulty-effects`, Operative / Agent / 00 Agent

Audit everything the selected difficulty changes in the game (`GameState+0x40` from the difficulty table 0x2df4c0
+0x0c; find every read): enemy damage/accuracy/reaction/health, Bond's damage taken, armour, ammo, hints on/off, the
score's difficulty bonus, objectives per difficulty, continue health boosts (`ContinueHealthBoost*`). List each with
its address and the port's status; implement the missing ones in the sim. Tests per effect (numbers only) with
sister assertions.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/difficulty-effects`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T00:03Z
