## Your task: `mp-backout-after-character` — local MP setup breaks after backing out

Playtest 1015-2231: "multiplayer menu breaks if you back out after setting a character in local mp".

Walk the local MP setup by actions only (menus_e2e.rs style): pick a character for player 1, back out one page,
then back to the main MP menu, then forward again. Repeat with 2 joined players. Find the state that goes wrong:
- the join slots, the cursor or the team/character prompts (`enter_mp_setup` states 2/3);
- the pair or overlay stack.

Compare with the game's page handlers for the character page and its Back case [M the MP character-select page
handler]. Fix it.

Tests: every back-out sequence above returns to a working page, and a match can still be started afterwards.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour, and say so in the report. Don't change working code to make a task look busy.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-backout-after-character`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T18:53Z

---
## Report (done, 2026-10-16T20:40Z)


---
## Report from the worker (done, 2026-10-16T20:40Z)

