## Your task: `nfstats-model2`, land 614 (held: 1 suite failure)

614 (`s/nfstats-model`, dd301c6 — the Mission Stats rows) was HELD on 2026-10-15. On master + 614 (the menu.rs
merge kept the MP pages and the Stats page side by side) the main agent's full suite gave **1449 passed, 1 failed**:
- `nf-view/tests/fix_scores_handoff_live.rs:104` — "row 5: the no-kill run's stat", left 100, right 0.
614 itself edited that test; master also gained 111e (`s/fix-scores-handoff`, the PlrMissionStats handoff in
`crates/nf-game/src/progress.rs`) after 614 branched — the two changes meet in that row. Start from
`origin/s/nfstats-model`, merge `origin/master` first, reproduce, decide from the game (P_NFSTATS_Handler 0x209938 /
the results rows) which value row 5 must show for a no-kill run, fix the code or the test with that evidence, and
report before/after.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/nfstats-model2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-15T20:39Z
