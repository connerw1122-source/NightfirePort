## Your task: `objective-banner-timing` — objective banners wait for cut-scenes

Playtests 1012-1723 and 1012-2337 (07000007): "The objective banner keeps playing during the cutscene. It needs to wait
until after the cutscene's finished", and "obj doesnt wait for kimiko cutscene and plays the sound immediately".
Port the game's gate:
- how `Text_AddMsg` / the objective-message queue hold while an NIS / script camera runs [M the message update's
  NIS check, `Mission_ObjectiveState`];
- when the "new objective" sound plays.

Tests on 07000007's Kimiko scene: the banner and its sound appear only after the scene hands back. Sister: outside a
cut-scene the banner shows immediately.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/objective-banner-timing`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
