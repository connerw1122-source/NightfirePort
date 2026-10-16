## Your task: `astronaut-guards-1b` — zero-G astronaut guards on 0700001b

Audit gap ⑦: the space station's astronaut guards (zero-G) don't run their own AI. Port their movement, combat and
death in zero gravity from the decompile [M]: drift, thrusters, the zero-g death anim.

Tests on 0700001b: an astronaut guard moves and fights the game's way in zero-G and dies with the game's
zero-g death.

The uncited-gameplay audit (task 710, merged) has this gap's description, caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that section first and meet its criteria.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/astronaut-guards-1b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T21:14Z
