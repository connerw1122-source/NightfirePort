## Your task: `stun-recovery` — stunned guards stay stunned forever

Playtest 1015-1738 (07000005, the stunner): "guards get stunned infinitely and don't complete the animation".

Port the stunned guard's whole timeline from the decompile:
- the stun state and its timer;
- the animation it plays and how the state ends;
- what a re-stun during the stun does (restart, extend, or ignore);
- the recovery back to normal AI.

[M the drone's stun/taser state handler, the hit path for the stunner weapons 0x4A/0x4C.]

Test: one stunner hit on a 05 guard; he recovers at the game's tick and returns to his AI. Holding the beam on him
follows the game's re-stun rule, not an endless loop.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour, and say so in the report. Don't change working code to make a task look busy.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/stun-recovery`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
