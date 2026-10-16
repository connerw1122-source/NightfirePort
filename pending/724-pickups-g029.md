## Your task: `pickups-g029` — GAPS G-029: pickups auto-switch, placed/script pickups, respawn

Close the open half of GAPS G-029 from the decompile [M Pickup_Create 0x18dda0, Pickup_Update 0x18e148 and the take
path]:
- auto-switching to a better weapon on pickup (the game's rule and its option gate);
- the level-placed vs script-spawned pickup differences;
- respawning pickups (multiplayer and any single-player cases).

Tests for each rule on a real level. Update GAPS.md with what is closed.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/pickups-g029`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
