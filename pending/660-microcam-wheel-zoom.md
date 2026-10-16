The user's playtest 2026-10-15 (results id 1015-1738) said: "It'd be cool if we could add some zoom to the micro
camera and have the scroll wheel do the zoom while you're right-clicking to aim in. And if you're not
right-clicking, of course, weapon switch." Implement for every zoomable weapon/gadget: while right-click (aim) is
held, the mouse wheel steps the zoom between the weapon's own min/max zoom (the game's per-weapon zoom range and
step [M the zoom input arm in Player_Weapon / weapon_data zoom fields]; for the Micro-Camera, its own range). When
right-click is not held, the wheel keeps switching weapons. Headless tests through the window's input path.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/microcam-wheel-zoom`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
