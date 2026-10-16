## Your task: `cover-nodes` — the AI cover node placements (0xE5 corner, 0xE6 low)

968 placements on every mission level that the port doesn't spawn: `Drone_CoverCornerNode` 0x136ff0 and `Drone_CoverLowNode` 0x137340. Check how the port's existing cover system (docs/systems/drone-cover.md, drone.rs `covers`) finds cover today, and bind the game's own nodes the way the game does. Test on 07000006: a guard under fire takes the game's cover node instead of standing in the open.

The script census (task 709, merged) lists this gap with occurrence counts and the game handler in `docs/systems/script-census.md` ("Top 10 ready-to-queue tasks"). Port from the decompile [M], test on real levels through the real path with a sister assertion, and update `crates/nf-formats/tests/script_census.rs`'s handled sets.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/cover-nodes`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
