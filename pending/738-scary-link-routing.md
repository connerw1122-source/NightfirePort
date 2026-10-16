## Your task: `scary-link-routing` — A* honours the scary-link flags

Audit runner-up: `AINetwork_SetLinksFlagInCircle` 0x15b548 flags route links inside danger radii (grenades and the like). The port scans the radii directly (drone.rs ~9370), but its A* ignores the link flags, so guards path straight through grenade radii. Make the route search honour the flags the game's way.

The uncited-gameplay audit (task 710, merged) has this item's caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that entry first, port from the decompile [M], and add a test that fails without the change, plus a sister.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/scary-link-routing`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
