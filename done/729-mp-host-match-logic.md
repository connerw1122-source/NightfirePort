## Your task: `mp-host-match-logic` — the live MP match runs the match logic

Audit gap ④: in a live match the host never runs the match logic. `update_objects`, `player_killed` and respawn are
only exercised by tests. Wire them into the running match loop (local and online host) the way `MP_Update`
drives them [M].

If task 704's `s/mp-bots-in-match` branch exists, rebase onto it rather than redoing its wiring.

Tests: a local deathmatch in the real app path. A kill scores, the victim respawns at the policy's point, and the
frag limit ends the match into the debriefing.

The uncited-gameplay audit (task 710, merged) has this gap's description, caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that section first and meet its criteria.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-host-match-logic`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T20:59Z

---
## Report (done, 2026-10-16T22:43Z)

