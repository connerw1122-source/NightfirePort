The user's playtest 2026-10-15 (results id 1015-1738) said: "those trees are, for some reason, bulletproof, like the
leaves" (07000005, the small trees near the start). Find how the game's bullet trace treats foliage: the surface
flags / collision class of leaf polygons and whether bullets pass through [M the trace's material/flag filter in
Bullet_CollisionHandler 0x12ce80 and the collide trace]. Make the port's trace match. Test on 07000005: a bullet
through a tree's leaves continues (or stops) exactly as the game does, with the hit surface flags cited.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/foliage-bullets`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
