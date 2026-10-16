## Your task: `script-census` — what level scripts and events the port doesn't handle yet

The user asked: "Are there things like scripts and whatnot that we haven't found yet?" Measure it. For EVERY level
archive (all 0x07 levels on the disc, the front end included), enumerate:
- every script (the script players / NIS scripts / SimpleScripts) and every script command/opcode/event type it
  contains, with occurrence counts;
- every trigger placement type and its action codes;
- every placement type the level parse sees.

Then classify each opcode, event, trigger action and placement type as handled (cite the port's handler) or
unhandled (cite the game's handler [M] and what it does). Output: docs/systems/script-census.md with a per-level
table and a global "unhandled, ranked by occurrences" list, plus a test that pins the handled set (numbers only),
so a regression or a newly handled opcode is visible. Don't implement anything big here. The census is the
deliverable, with the top 10 unhandled items written up as ready-to-queue task descriptions in the report.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/script-census`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
