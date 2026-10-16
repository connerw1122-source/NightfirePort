## Your task: `drone-abseil-elevator` — AbseilHang and ElevatorJumper drones

The decompile census lists `NDrone2_AbseilHang` and the ElevatorJumper states as not cited in the port. Find which
levels place drones that use them [D the placements / DCVars whose setup reaches those states]. Then port both:
- the hang / descend / land sequence and its anims and timings;
- the elevator jump, whatever it does in the game [M].

Tests on a level that uses them: the drone runs the sequence and lands at the game's position and tick. Without
those states the drone just stands there.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drone-abseil-elevator`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T19:40Z
