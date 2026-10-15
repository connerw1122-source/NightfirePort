## Your task: `mp-hud-data`, what the multiplayer HUD shows

In an MP match the HUD shows per-agent info (score / frags, the match timer, the scenario's objective status —
flags held, uplink progress, the Top Agent target, the radar's blips). Port the data model the HUD reads (from the
game's MP HUD code — find it from the in-match draw calls and nf-game's mp code), as a per-frame snapshot struct
with getters (scores sorted the way the game sorts, timer mm:ss, objective markers in world space, radar blips
relative to the viewer). The main agent draws it. Tests: a headless match's snapshot matches the sim state.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-hud-data`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
