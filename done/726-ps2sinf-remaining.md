## Your task: `ps2sinf-remaining` — GAPS G-046: the last host-trig call sites

GAPS G-046: the remaining sim call sites still use the host's sin/cos/atan where the game uses its own `PS2Sinf` /
trig (bit-exact proven, 40/40 [verify ledger]).
1. List every remaining `f32::sin/cos/tan/atan2` in nf-game, and in nf-view's sim side, that stands in for a game
   trig call.
2. For each one, find the game's function at that site [M].
3. Swap it to the port's bit-exact version.

Keep `walk*`/`walkreal*` and the AI traces green. Report every site that changed a trace, with before/after.
Update GAPS.md.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/ps2sinf-remaining`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T20:26Z

---
## Report (done, 2026-10-16T21:37Z)

