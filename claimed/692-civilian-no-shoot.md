## Your task: `civilian-no-shoot` — the no-shoot crosshair blocks the trigger

Playtest 1011-1419 (07000002): "The no shoot icon was there, but I was still able to shoot at him". The game shows
the red no-shoot crosshair over civilians AND refuses to fire [M the crosshair's target class check in the player's
weapon update and the trigger gate]. Port the trigger refusal: which targets, which weapons, and whether a held
trigger fires once the aim leaves the civilian. Tests on 07000002: aim at a civilian and fire — no shot, no ammo
used; aim off — fires. Sister: a guard right next to the civilian can be shot.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/civilian-no-shoot`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T13:19Z
