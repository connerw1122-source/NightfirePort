The user's playtest 2026-10-15 (results id 1015-1738) said: the stunner's guards "get stunned
infinitely and don't complete the animation". Port the stunned drone state fully: entry, the stun animation, the
duration/re-hit rules and the exit back to the AI [M the stun arm in the drone damage path and its state handler].
Prove a stunned guard completes the animation and returns to combat after the game's timer, and that re-stunning
behaves like the game. Tests on 07000005 with the Stunner (0x4a/0x4c) on a guard.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/stun-ai-recover`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T03:10Z

---
## Report (done, 2026-10-16T04:40Z)

