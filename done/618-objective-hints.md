## Your task: `objective-hints`, the pause OBJECTIVES tab's ✕ Hint

On the pause page's OBJECTIVES tab the hint bar reads "✕ Hint ✚ Scroll ✚ Next"; ✕ on an objective shows its hint
(the objective record's `+0x08` hint label, docs/systems/mission.md). Port it from `C_GCPAUSE_Handler` 0x204408 (the
OBJECTIVES case) and the objective records: which row is selected, what ✕ shows (a message box? label ids, timing),
whether hints are limited or affect scoring (the Mission Stats / 007 Bonus), and Back. State + getters on
`FrontEnd` (`pause_objectives` already lists the visible rows); the host side passes the hint labels. Tests: ✕ on
row N yields that objective's hint label; a hidden objective has none.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/objective-hints`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-15T19:20Z

---
## Report (done, 2026-10-15T20:39Z)

Closed by the driver: the report is on `s/objective-hints` (docs/reports/objective-hints.md); the worker didn't run finish.sh.
