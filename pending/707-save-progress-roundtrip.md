## Your task: `save-progress-roundtrip` — progress survives a restart

By the real path:
- create a codename;
- complete two missions (the results commit + the save prompt Yes — 635's card flow);
- drop the FrontEnd entirely (as an app restart would);
- build a fresh FrontEnd on the same profile dir and load the codename.

Assert the mission list shows the same unlocked rows (including the driving pass-through), the medals and best
scores, the dossier rewards, and the MP rewards (the Explosive Scenery gate). Also "Play without saving" leaves no
file. Sister: a corrupted file reports Load failed (the card message) and nothing loads. Use a temp profile dir,
never the real one.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/save-progress-roundtrip`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
