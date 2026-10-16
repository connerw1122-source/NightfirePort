## Your task: `guards-0c` — the 0700000c guards' AI and walking

Playtests 1012-1723 and 1012-2105 (0700000c): "All of the AI in this level is bugged as hell. The walking animation is
kind of fucked too." List every 0c drone at start and over 1800 ticks: class, state transitions, route, the anim
sequence and frame per tick. Compare against the game's route/patrol data and state machine [M NDrone2 states, the
level's AI network]. Find:
- guards stuck in a state;
- walk anims that don't match their speed (foot sliding — the anim rate vs the move speed);
- route nodes they never reach.

Fix the sim side with evidence; table before/after. Tests: each patrol guard completes its route loop within the
game's period, with the walk anim rate tied to its speed.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guards-0c`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T14:25Z

---
## Report (done, 2026-10-16T15:22Z)

