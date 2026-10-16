## Your task: `mp-map-objects` — multiplayer maps' moving objects and vehicles

Playtest 1016-1355 (the user, on master 2026-10-16): "on the multiplayer maps, like Gondola, it's missing the actual gondolas. It has these things called remote tanks and like helicopters that you should be able to pilot, and you can't." Census every MP map's dynamic objects [D placements]: the gondolas, the remote-control tanks and helicopters, and any other movers/usables. Port their movement and the pilot/usable mechanics (the remote-vehicle control) from the decompile [M]. The drawing side is the main agent's; the sim and the controls are yours. Tests per map: the gondolas move on their routes; a player can take control of a remote tank/helicopter and drive it.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-map-objects`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
