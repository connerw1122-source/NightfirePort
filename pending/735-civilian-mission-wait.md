## Your task: `civilian-mission-wait` — CivilianMissionWait (state 0x17)

Audit runner-up: mission civilians must pause at wait-channel route nodes (CivilianMission msg 0x22 [M 0x15d860 case 0x22]; `CivilianMissionWait` 0x15dbb0). The port has no msg 0x22, so they walk straight through. Port it.

The uncited-gameplay audit (task 710, merged) has this item's caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that entry first, port from the decompile [M], and add a test that fails without the change, plus a sister.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/civilian-mission-wait`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
