## Your task: `escort-01` — Mayhew's escort on 07000001

Playtest 1012-1723: "mahew doesnt really follow properly ... level transition door should be able to open without
him walking all the way to it". 663 fixed NPCs opening doors (the route-follow step-back). Now port the escort
itself:
- the follow distance and the catch-up;
- the waiting at doors;
- the exit door's condition (does the game require him present, or within a radius?) [M the escort drone's
  state/script, the exit trigger's channel conditions].

Headless walk from the meet-up to the exit with Bond moving at walking speed: Mayhew keeps within the game's
distance, and the exit opens under the game's condition. Keep `walkreal_07000001_live` green.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/escort-01`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T12:14Z

---
## Report (done, 2026-10-16T14:07Z)

