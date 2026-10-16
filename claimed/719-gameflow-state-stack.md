## Your task: `gameflow-state-stack` — port the real GameFlow state stack

GAPS: "Game-flow state stack: GameFlow_Main is a stand-in, Push/PopState not started". Port `GameFlow_Main` and the
`GameFlow_PushState` / `PopState` stack from ACTION.ELF [M]:
- the state ids;
- which call pushes and pops (level load, movie, front end, pause, the fade hold);
- what each state's tick does.

Then replace the port's stand-in `flow` state machine where it differs, without breaking the menus. The menu e2e
tests and mission_unlock_flow are the guard rails.

Tests: the stack contents across a full SP run (title → briefing → level → outro → results → next level) match
the game's sequence of pushes and pops.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/gameflow-state-stack`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T19:29Z
