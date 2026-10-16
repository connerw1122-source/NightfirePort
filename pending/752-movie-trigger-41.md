## Your task: `movie-trigger-41` — the movie-player trigger (action 0xD) and NIS events 0x08 and 0x0a

Two small, fully specified gaps:
- **The trigger:** `Trigger_MoviePlayer` 0x1b2a08 and `Trigger_Update` case 0xd. The input channel clears, `GameState+0x16 = out`, and GameFlow pushes states 6/7 (movie + fade). One record, in 07000041. Wire it into the port's `resolve_exit_code` path like LOAD_LEVEL.
- **NIS events in script 0600005b (07000001):**
  - 0x08: `Drone_CoderCreate` 0x13a108, which spawns the coder drone (skin 0x0500001b, anim 0x060000a6).
  - 0x0a: `Break_DoBreak` 0x12c168 at the stream object.
  - Event 0x09 is task 733's.

Tests for each.

The script census (task 709, merged) lists this gap with occurrence counts and the game handler in `docs/systems/script-census.md` ("Top 10 ready-to-queue tasks"). Port from the decompile [M], test on real levels through the real path with a sister assertion, and update `crates/nf-formats/tests/script_census.rs`'s handled sets.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/movie-trigger-41`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
