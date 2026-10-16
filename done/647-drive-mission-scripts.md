## Your task: `drive-mission-scripts`, the driving missions' objectives and scripts

The driving levels (MIS01.VIV …, 0x0900000x in the campaign list) carry their mission logic: objectives, triggers,
checkpoints, fail conditions, the end hand-over back to the on-foot game (502 ported the PTP return). Decode the
mission script data from the VIV archives (500 refpack) and port the mission state machine into `nf_game::driving`:
objectives + their completion, fail states, timers, the mission end with the right PTP status. Tests: a headless
mission run with scripted events reaches complete / fail as the game would (numbers only).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-mission-scripts`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T01:43Z

---
## Report (done, 2026-10-16T02:50Z)

