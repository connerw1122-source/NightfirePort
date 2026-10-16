The user's playtest 2026-10-15 (results id 1015-1738) said: after a guided (Sentinel) rocket finishes, "I can still
hear a sound being played from where I was standing when I shot the rocket, and it doesn't stop, and it can stack up
the more times I shoot rockets". Find the looping sound started at launch (the rocket's flight loop / the launch
sound's handle) that is never stopped when the rocket dies or the guided camera ends, and stop it the way the game
does [M SFXStop / the object's sound handle on delete]. Also check every other projectile/object loop for the same
leak (grenades rolling, the MGL, thrown mines). Headless test: fire 3 guided rockets, let them die, and assert no
leftover live looping items in the driver.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/rocket-sound-leak`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T03:10Z
