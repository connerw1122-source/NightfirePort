## Your task: `nfstats-model`, the Mission Stats page's numbers

After a mission the results page's ○ Stats opens **Mission Stats** (`P_NFSTATS_Handler` 0x209938): a table
Category / Current / Target / TOTAL with rows Bond Moves, Opponents (Dispatched / Subdued / Surrendered), Stealth
Rating, Accuracy Rating, Health Remaining, Time, 007 Bonus, then Subtotal, Difficulty Bonus, TOTAL (PCSX2 capture on
07000005: e.g. Time 01:19 / 09:00 / 170,887; Stealth 100% / ---- / 25,000; Accuracy 100% / 100% / 50,000; Health
100% / 100% / 50,000; Subtotal = TOTAL 295,887 at Agent). Port the computation exactly: where each Current comes
from (PlrMissionStats / the progress layer, `crates/nf-game/src/progress.rs`), each Target (the level's table), each
TOTAL (the scoring formulas, the difficulty bonus), the number formatting (`SeparateNumber`, mm:ss, "----", %), and
the page flow (○ from results, △ back). Expose a `stats_rows()` getter returning the rows as display strings + ids.
Test: drive a headless mission end with known stats and assert the rows; the PCSX2 example above as a golden
(numbers only).

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/nfstats-model`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-15T18:37Z
