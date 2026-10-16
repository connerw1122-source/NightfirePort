## Your task: `tase-scream-broadcast` — the tase scream alerts nearby guards

Audit runner-up: entering the tase state broadcasts `NDrone2_DroneAlertToPosition` 0x14a498 with prop 0x32. The port screams only on bullets and punches (drone.rs handle_taser is cited as partial). Port the tase arm.

The uncited-gameplay audit (task 710, merged) has this item's caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that entry first, port from the decompile [M], and add a test that fails without the change, plus a sister.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/tase-scream-broadcast`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T22:32Z

---
## Report (done, 2026-10-16T23:42Z)

Closed by the driver: the report is on `s/tase-scream-broadcast` (docs/reports/tase-scream-broadcast.md); the worker didn't run finish.sh.
