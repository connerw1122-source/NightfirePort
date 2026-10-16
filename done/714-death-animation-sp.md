## Your task: `death-animation-sp` — Bond's death has no death animation

Playtest 1015-0046 (restart after dying): "good, missing death animation".

When Bond dies in single player, the game plays the death camera / animation script (`Bond::death_script()` already
picks 0x6000151, or 0x60008e0 in zero-g) before the failed page. Trace the game's death sequence:
- `Player_HandleDeath` 0x190bb0 and `Player_ChangeSubState` case 0xd/0xe;
- the camera it uses and how long it runs before the fail screen.

Check whether the port plays that script, positions its camera, and waits the game's time before the failed page.
Fix what's missing in the sim / flow; the drawing side is the main agent's.

Test: kill Bond on 05; the death script runs for the game's tick count, then the failed page opens.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour, and say so in the report. Don't change working code to make a task look busy.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/death-animation-sp`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T18:25Z

---
## Report (done, 2026-10-16T19:39Z)

