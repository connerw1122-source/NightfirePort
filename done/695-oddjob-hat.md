## Your task: `oddjob-hat` — Oddjob's hat: damage, wall sound, how many fly

Playtests 1013-2313 and 1014-1725: "oddjobs hat should be a oneshot kill but it acts like a grenade"; "it makes a weird
sound when it hits a wall"; "if you throw enough of them, you stop seeing them flying". Port from the game:
- the hat's hit damage (one-shot?) [M its weapon record and the projectile hit path];
- the wall-hit sound id;
- the projectile pool cap / recycling.

Tests: a hat hit on a guard kills him in one hit if the game says so; the wall hit plays the game's sound; 10 rapid
throws keep the game's count of live hats (the oldest recycled the game's way).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/oddjob-hat`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T14:02Z

---
## Report (done, 2026-10-16T15:31Z)

