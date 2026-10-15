## Your task: `mp-pickups-respawn`, multiplayer pickups, weapon sets and respawns

The MP options choose a weapon set (MPSettings+0x1B4, 11 sets), respawn rule (+0x1C0 Near/Far/Random), vehicles,
fixed gun emplacements, grapple (docs/systems/mp-menus.md, 611/402). Port how the game applies them in a match: which
pickups spawn per weapon set (the tables), their respawn timers, the spawn-point choice per rule, armour/health
pickups, the gun emplacements / vehicles / grapple toggles, in nf-game's mp code. Tests: per weapon set, the map's
pickup list matches the game's tables; the respawn rule picks the game's point (numbers only).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-pickups-respawn`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
