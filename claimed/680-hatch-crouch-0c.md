## Your task: `hatch-crouch-0c` — dropping through the 0700000c hatch and crouching through

Playtests 1012-1723, 1013-1758 and 1014-1725 (0700000c): after the hatch opens, "he still can't get through because
he can't crouch through" — Bond is blocked and soft-locked under the hatch. Find the collision involved: the
opening's size versus Bond's crouched capsule height [M Player_Collide / the crouch height in the player tables],
the hatch object's collision after it opens, and any crawl-space trigger.
- Port the crouched movement and collision so the passage is traversable exactly as in the game.
- Headless test: from the hatch top, open it, drop, crouch-walk the passage to the far room, and assert Bond's
  position.
- Sister: standing (not crouched) is blocked at the same spot.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/hatch-crouch-0c`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T08:46Z
