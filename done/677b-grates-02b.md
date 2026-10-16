## Your task: `grates-02b` — land 677 without breaking the 0c vent latches

`s/grates-02` (677) was HELD on 2026-10-16. With it merged into master,
`fix_level_0c_live::burning_the_four_hinges_opens_the_display_vent` and
`fix_level_0c_live::the_warehouse_vent_latch_opens_its_hatch` FAIL. Without it they pass, as do every 0c hatch/vent
test. Your `ss.rs` (SimpleScript) change for the 02 grates alters a path the 0c hatches share. Find the shared arm
and make the change specific to what the game's evidence says differs (the grate's object class or flags vs the
hatch's) [M]. Both the 02 grates and the 0c latches must work. Start from origin/master, merge your branch,
reproduce, fix, and run the FULL suite on the final tree.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/grates-02b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T11:17Z

---
## Report (done, 2026-10-16T12:14Z)

Closed by the driver: the report is on `s/grates-02b` (docs/reports/grates-02b.md); the worker didn't run finish.sh.
