The user's playtest 2026-10-15 (results id 1015-1738) said: bullet hits on people show "No red puffs". Port the
flesh-hit effect spawn: the material/effect lookup for a character hit [M Effect_Create / Effect_Bullet on the
flesh path, the EffectInfo row for flesh, its gas/decal/debris entries] so a guard hit spawns the game's own
blood puff into the existing gas/effect pools (data only — the pools already draw). Test: shoot a guard on
07000005 and assert the puff with the right mesh/colour/life is in the pool; sister: a world hit doesn't spawn
it.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/flesh-hit-effects`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T03:57Z

---
## Report (done, 2026-10-16T05:18Z)

Closed by the driver: the report is on `s/flesh-hit-effects` (docs/reports/flesh-hit-effects.md); the worker didn't run finish.sh.
