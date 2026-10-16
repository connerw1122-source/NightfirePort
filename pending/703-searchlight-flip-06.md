## Your task: `searchlight-flip-06` — a searchlight beam that flips on its side (07000006)

Playtest 1013-1825 (07000006): "Sometimes a spotlight will cover a piece of ground and it will like flip on its side,
which looks really weird". Trace each 06 searchlight's beam matrix per tick over its sweep (`searchlight.rs`, the
parts' matrices). Find a sign flip or gimbal discontinuity: the `align2dir` up-vector fallback when the beam points
straight down, an atan2 wrap, or the roll [M Searchlight_Update 0x1cb900, Mat_Align2Dir with its fallback vector].
Fix it to the game's math. Test: every searchlight's beam basis is continuous tick to tick over 1800 ticks (no
column flips sign while the direction moves smoothly). Sister: a synthetic straight-down beam uses the game's
fallback.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/searchlight-flip-06`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
