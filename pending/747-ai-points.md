## Your task: `ai-points` — AI point placements (0xE9)

162 placements: `Drone_AIPoint` 0x137550. Port the record (read its params from the decompile) and wire it into the guards' route/patrol logic the way the game uses it. Test on 07000007 (34 points): guards use them as the game does.

The script census (task 709, merged) lists this gap with occurrence counts and the game handler in `docs/systems/script-census.md` ("Top 10 ready-to-queue tasks"). Port from the decompile [M], test on real levels through the real path with a sister assertion, and update `crates/nf-formats/tests/script_census.rs`'s handled sets.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/ai-points`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
