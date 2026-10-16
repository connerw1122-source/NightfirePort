## Your task: `npc-doors` — NPCs open doors

Reported over several playtests (1012-1723 and 1013-1825, levels 07000001, 07000006 and 07000007): "npcs cant open
doors", "NPC still can't open doors". The escort (Mayhew on 07000001) stalls at doors, and guards and guests don't
path through them.
- Port the drone side of door use: how a drone's path or route reaches a door, the trigger or activation call it
  makes (the `Door_Activate` caller list from the drone code), the wait-for-open behaviour, and how locked or
  scripted doors refuse [M the NDrone route/door code].
- Tests: 07000001 Mayhew's escort route through its door; a 07000006 guard chasing Bond through a closed door; a
  locked door that must stay shut for drones.
- Sister assertions prove a closed door still blocks a drone that doesn't open it.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/npc-doors`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T05:17Z

---
## Report (done, 2026-10-16T06:57Z)

