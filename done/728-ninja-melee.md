## Your task: `ninja-melee` — the Ninja melee chain

Audit gap ③: the Ninja drones' melee chain, DSTATE 0xab-0xb3 [M], isn't ported. Port:
- the approach, the attack selection, each strike's anim and damage;
- the block/dodge states;
- the transitions.

Do it on the level that places Ninjas [D].

Tests: a Ninja engages Bond in melee with the game's sequence and damage; his states follow the game's transitions
over a scripted fight.

The uncited-gameplay audit (task 710, merged) has this gap's description, caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that section first and meet its criteria.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/ninja-melee`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T20:41Z

---
## Report (done, 2026-10-16T21:31Z)

