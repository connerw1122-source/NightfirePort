## Your task: `sg5-semi-laser` — the SG5's semi-auto mode turns its laser sight on

Playtest 1015-1738: "the SG5, when you turn on semi-auto, it's supposed to have a laser ... turn on, and it doesn't".
Find the SG5's mode-dependent laser [M the weapon record's mode table, the fire-mode switch in Player_Weapon
0x1a24c8, any laser-sight datum or attachment toggled per mode, the aim-laser beam (cf. the laser-aim sphere query
0xa0a in the foliage work)]. Port the sim state:
- which mode lights it;
- the beam's start (the datum) and end (the aim trace hit).

Expose it as a getter the draw code reads (the main agent draws the beam). Tests: switch the SG5 (0x1c? check the
id) to semi and assert the laser state and its end point on a wall; full-auto → off.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/sg5-semi-laser`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
