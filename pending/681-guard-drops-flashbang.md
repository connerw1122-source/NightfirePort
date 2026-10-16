## Your task: `guard-drops-flashbang` — guards drop the right items; the flashbang doesn't kill

Playtest 1011-1655 (07000005), still open:
- "all enemies are dropping multiple objects";
- "the flashbang item is killing enemies even if I dont throw it directly at them".

Port from the game:
1. A dying guard's drops: which items, how many, under what conditions [M DroneWeap_DropWeapon, the pickup
   creates on death, the drone's held weapon and inventory flags]. Make each guard drop exactly what the game
   drops.
2. The stun (flash) grenade (0x35): its effect is a stun/blind, not damage [M the explode/effect arm for the stun
   type, the drones' stun state, the damage path]. A flashbang must never kill: direct hit, near, or far.

Tests on 07000005: kill 5 guards of different types and assert each one's drop list. Throw a flashbang among 3
guards and assert no health loss, plus the stun state per the game's radius. Sisters: a frag still kills, and a
guard holding nothing drops nothing.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guard-drops-flashbang`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
