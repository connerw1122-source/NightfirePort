## Your task: `sensor-volumes` — sensor volumes (0x28)

62 placements: `Sensor_Create` 0x1af010 and its update. Trip volumes that anything (not only the player) can set, writing switch channels. Test: a drone walking through sets the sensor's out channel; sister: an empty volume doesn't.

The script census (task 709, merged) lists this gap with occurrence counts and the game handler in `docs/systems/script-census.md` ("Top 10 ready-to-queue tasks"). Port from the decompile [M], test on real levels through the real path with a sister assertion, and update `crates/nf-formats/tests/script_census.rs`'s handled sets.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/sensor-volumes`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
