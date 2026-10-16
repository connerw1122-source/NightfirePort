## Your task: `scorpion-rocket-speed` — the AT-600 Scorpion's rockets are too slow

Playtest 1016-1355 (the user, on master 2026-10-16): "Scorpion rockets are way too slow" (multiplayer). The Sentinel's speed was measured live (×4.04 on 0x2c/0x2d). Check the Scorpion records 0x2E/0x2F the same way: the init speed, the 0x200 quarter, the per-tick flight step [M Bullet_init 0x12c5a0, Bullet_update 0x12e7b0] and any homing. Set them to the game's numbers, with evidence. Test: the Scorpion's per-tick step matches the game's.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/scorpion-rocket-speed`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
