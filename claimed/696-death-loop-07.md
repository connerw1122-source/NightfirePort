## Your task: `death-loop-07` — no replay loop after dying on 07000007

Playtest 1012-1723 (07000007): "once I died, we went into a replay loop where it kept replaying at the start of the
mission over and over again. I had to quickly close out." Reproduce headless: die on 07 (a guest-shot fail and a
health-zero death), take the fail path (the port's auto-restart and the fail menu's REPLAY/RESTART), and run 600 ticks
after the reload. Find what re-fails at once:
- a carried fail channel;
- the guest-shot flag not reset;
- the weapon-lock edge firing a guard reaction;
- `level_exit_restart` loops.

Fix it to the game's restart semantics [M Mission_Update's fail cases, the RamLoad restore, the 07 party script's
init]. Test: two consecutive deaths and restarts each land in a playable 07 that stays alive for 600 ticks.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/death-loop-07`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T14:11Z
