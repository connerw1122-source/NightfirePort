## Your task: `mp-match-end` — a local MP match ends and returns through the debriefing

By the real path: start a local MP match from the menus (frag limit 1, or a 1-minute time limit), play it headless
until the end condition fires [M MP_CheckForEndCondition 0x184268 / MP_Update states, 402 + 621b on master], and
assert the host's handover to the front end:
- the debriefing page with its board, the ranks and the winner line;
- Confirm returns to the scenario page;
- a second match can start.

The host glue may be missing (the single-player end sequence was missing until 2026-10-15): if the match end only
logs, port the handover the way the SP mission end now works (`Drones::mission_end` → the host). Tests for the
frag-limit and the time-limit ends.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-match-end`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
