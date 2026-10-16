## Your task: `music-triggers` — music trigger placements (0xFB)

15 placements: `MusicTrigger_Create` 0x1b30e0 and `MusicTrigger_Update` 0x1b31a0 (`Music_Event(id, 2)` while its countdown runs). Port them into the music system. Test: entering the volume raises the game's music event; leaving lets it expire at the game's tick. Tests are silent (NF_SILENT).

The script census (task 709, merged) lists this gap with occurrence counts and the game handler in `docs/systems/script-census.md` ("Top 10 ready-to-queue tasks"). Port from the decompile [M], test on real levels through the real path with a sister assertion, and update `crates/nf-formats/tests/script_census.rs`'s handled sets.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/music-triggers`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
