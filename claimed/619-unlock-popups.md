## Your task: `unlock-popups`, the medal / bonus / cheat unlock pages after a mission

After the results page the game can show unlock pages: `P_NFBONUS_Handler` 0x209670 (bonus earned), `P_CHEATMEDAL_Handler`
0x2086b0 (a cheat/medal unlock), `P_WINGAME_Handler` 0x215100 (the end of the campaign). Port when each appears (the
medal tiers, the 007 Bonus, which unlocks — missions, MP maps/characters/scenarios, cheats — and from which progress
fields), what it shows (label ids, textures), the order, and where it returns. State + flow on `FrontEnd` after
`results_enter`, getters for the draw code. Tests: a gold/platinum finish on a level with an unlock shows the right
page with the right ids; none when nothing unlocks.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/unlock-popups`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-15T19:27Z
