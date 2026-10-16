## Your task: `pickups-08` — the briefcase and the rocket launcher on 07000008

Playtests 1012-2337 and 1013-1758: "cant pickup the briefcase or rocket launcher" in the 07000008 safe room; the
invisible wall there is fixed. Find why each pickup refuses:
- the placement types and pickup kinds [M Pickup_Create 0x18dda0, Pickup_Handler 0x18e650];
- the use/collect radius;
- any gate (a script channel, the safe's state, the Q-Laser cut);
- whether the briefcase is a gadget pickup (the briefcase gadget 0x...: the weapon id from the pickup record).

Fix the sim so walking or using the item collects it exactly as the game does. Headless tests on 07000008: collect
both, from Bond positions taken from the placements; check the inventory and the HUD message. Sister: out of range
does nothing.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/pickups-08`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T07:48Z
