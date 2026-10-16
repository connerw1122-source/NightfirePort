## Your task: `scorpion-modes-12` — the Scorpion's heat-seeker mode

Playtest 1012-1723 (07000012): "the scorpion launcher heat seeker mode doesn't work". Port the Scorpion's fire modes:
- the mode switch (B / the fire-mode action [M Player_Weapon 0x1a24c8]) between the dumb rocket and the
  heat-seeker (weapon records 0x2e/0x2f [L weapon_data]);
- the homing behaviour: `Bullet_homing` 0x12f358 for `+0x6C & 0x20000` — target acquisition, turn rate, re-target
  [M].

Tests on 07000012 with a guard target: a heat-seeker fired off-axis curves into the guard; a dumb rocket flies
straight. Check the mode label value too (the HUD reads it). Sister: homing never locks onto Bond himself.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/scorpion-modes-12`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
