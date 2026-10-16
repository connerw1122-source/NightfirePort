## Your task: `guard-barks-first-alert` — guard barks and FirstAlertState

Audit gap ⑨: the guard voice barks and `FirstAlertState` 0x39 aren't ported. Port:
- **The barks:** which line plays on spotting Bond, losing him, taking cover, reloading and dying; the
  voice-cooldown and selection rules; the sound ids [M + D: the voice banks].
- **FirstAlertState:** the guard's first-alert reaction.

Tests on 07000005: spotting Bond plays the game's bark once, within the cooldown rules. A guard's first alert goes
through state 0x39 with the game's timing. Tests are silent (NF_SILENT).

The uncited-gameplay audit (task 710, merged) has this gap's description, caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that section first and meet its criteria.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guard-barks-first-alert`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T21:32Z

---
## Report (done, 2026-10-16T22:28Z)

Closed by the driver: the report is on `s/guard-barks-first-alert` (docs/reports/guard-barks-first-alert.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T22:42Z)

