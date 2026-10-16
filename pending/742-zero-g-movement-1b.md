## Your task: `zero-g-movement-1b` — Bond's zero-G movement on 0700001b

Playtest 1016-1355 (the user, on master 2026-10-16): "we're in space ... there's a different movement system that you're missing". Port Bond's zero-G substate: `Player_ZeroG` 0x192ab0 (dispatch cases 8/9), the thruster controls, inertia, the auto-stabilisers ('Press Action to activate spacesuit auto-stabilizers') and the camera rules [M]. The level already starts in zero-g gravity (p3 == 2). Tests: movement integrates the game's way (drift with inertia; the stabiliser damps it); sister: on-foot levels are unchanged.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/zero-g-movement-1b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
