## Your task: `dossier-pages-data`, the Dossier's Rewards / Gadgets / Weapons pages

The Dossier wheel (Records / Rewards / Gadgets / Weapons) only has Records ported. Port the other three pages' data:
`P_DSREWARDS_Handler` 0x2172c8 + `C_RBDSREWARDS_Handler` 0x20c248, `P_DSGADGETS_Handler` 0x217190 +
`C_SBDSGTSCROLL_Handler` 0x20bcc0, `P_DSWEAPONS_Handler` 0x217038 + `C_SBDSWPSCROLL_Handler` 0x20bab0. For each:
the page id, its rows (label ids, preview texture hashes, description ids, stat lines), what is locked/unlocked and
from which progress/medal fields, the scrolling behaviour, and Back. Add `Page` variants + state + getters + the
actions from the Dossier wheel (rows 1–3 currently do nothing). Tests: the lists match the game's tables (ids), the
unlock states follow the progress fields.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/dossier-pages-data`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-15T18:50Z

---
## Report (done, 2026-10-15T19:57Z)

