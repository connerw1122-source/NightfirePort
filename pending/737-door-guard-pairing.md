## Your task: `door-guard-pairing` — door guards and their partners

Audit runner-up: `NDrone2_DoorGuardHandler` 0x1416d8 and `SetDronePartner` 0x141360. CivilianDoorGuard 0x31 (0x160648) exists as a const with no arms (WORKLOG 3969). Port the pairing and the handler on the levels that place door guards [D].

The uncited-gameplay audit (task 710, merged) has this item's caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that entry first, port from the decompile [M], and add a test that fails without the change, plus a sister.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/door-guard-pairing`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
