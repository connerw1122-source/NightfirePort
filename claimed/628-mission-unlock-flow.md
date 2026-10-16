## Your task: `mission-unlock-flow`, finishing a mission unlocks the next

Verify and complete the campaign progression end to end: completing a mission (the results page's commit,
`P_NFRESULTS_Handler` 0x208e78, the progress layer `crates/nf-game/src/progress.rs`) unlocks the next row of the
Single Player list (`sp_level[i]+0x10`, `FrontEnd::unlocked`), saves it to the active codename, and the list shows it
after a restart of the program; "Play without saving" keeps it only for the session; the driving missions
(0x0900000x) in the list — how the game unlocks/plays them and what the port does today (the PTP handover, 502).
Tests: a headless completion chain over the first three missions unlocks rows in the game's order and survives a
save/load.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mission-unlock-flow`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T00:02Z
