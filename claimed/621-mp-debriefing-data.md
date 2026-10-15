## Your task: `mp-debriefing-data`, the multiplayer match's end and debriefing

After an MP match the game shows the debriefing (`P_MPDEBRIEFING_Handler` 0x212198, `C_RBMPFINISH_Handler` 0x20db48)
with per-agent scores/kills/deaths/awards, then returns to setup. Port: what ends a match (the scenario rules from
the MP settings — time/score limits; tie to what nf-game's mp code already has), the debriefing's rows and columns
(label ids, values, award rules), the ordering, and the return flow. State + getters on `FrontEnd` (coordinate with
the MP menu flow task 611 if it has landed — merge `origin/master` first). Tests: a headless match with known
results produces the right debriefing rows.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-debriefing-data`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-15T21:17Z
