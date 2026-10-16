## Your task: `ladder-02` — ladders on 07000002 (the bunker and the water)

Playtests 1011-1732 and 1014-1725 report several ladder problems:
- "You can't move your camera while on the ladders";
- "When you get on the ladder, it unequips your weapon. And when you get off the ladder, it doesn't re-equip";
- "I'm infinitely latching onto the ladder, and I can't get off of it" (climbing off into the water).

Port the ladder state machine's look limits, the weapon holster and redraw, and the dismount at the bottom into
water [M Ladder_* / the player's ladder state in Player_Update, the dismount checks]. Headless tests on 02's water
ladder and the bunker ladder: look while climbing (within the game's clamp), dismount at the top and at the
bottom (into the water, with no re-latch), and the weapon returns after the dismount.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/ladder-02`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T12:39Z

---
## Report (done, 2026-10-16T13:36Z)

Closed by the driver: the report is on `s/ladder-02` (docs/reports/ladder-02.md); the worker didn't run finish.sh.
