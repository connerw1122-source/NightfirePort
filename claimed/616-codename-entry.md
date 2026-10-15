## Your task: `codename-entry`, creating / renaming / deleting a codename

"New Codename" (front end, both the NightFire codename page and Codenames → Edit Codename) opens the name-entry page
(`P_CNNAME_Handler` 0x213790, the letter grid `C_KEYBOARD_Handler` 0x213a98 on page 0x40000020's grid controls,
`Menu_IsValidCodename` 0x1f9930, the default proposal from `def_codename` 0x2e1b18). Port: the grid model (cursor,
rows 0-9 / A-Z / ← → ✓, the typed name, max length, valid-name rules and the game's error messages), creating the
profile through the existing save code (`nf_game::profile`, see docs/reports/fix-save-profile.md), Edit Codename's
~X Delete (with its confirm) and the options wheel's last row (Save Codename / rename). Keyboard: typed letters may
also go straight in (say so in the doc; the game's grid stays the reference). Tests: create → appears in the lists
and saves; invalid names refused; delete removes it.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/codename-entry`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-15T19:13Z
