## Your task: `mp-match-rules`, the multiplayer scenarios' rules in the sim

The MP menu flow (611) picks a scenario (Arena, Team Arena, Capture The Flag, Uplink, Top Agent, Demolition, …),
map and rules (time/score limits, environment/player modifiers, bots). Port each scenario's rules in nf-game's mp
code: scoring, win conditions, flag/uplink/objective logic, respawn rules, the modifiers — from the game's mp code
(find the handlers via the scenario ids in docs/systems/mp-menus.md). Tests: headless matches per scenario reach the
game's win condition with the right scores (numbers only).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-match-rules`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
