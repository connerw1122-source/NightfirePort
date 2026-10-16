## Your task: `cutscene-walk-stutter` — the Kimiko scene's walking guest stutters

Playtest 1015-1738 (07000007, the Kimiko cut-scene): "the one in the red dress, stuttered a little bit while she
was walking towards the other two". Headless, run that cut-scene and trace the walking actor's root position and
pose frame per tick. Find the discontinuity:
- a script key interpolation step [M Script_GetInterp 0x1da158];
- an animation frame repeat or skip (the frame rate multiplier, the anim window hand-off);
- or a root-motion double application.

Fix the sim side to the game's evidence and assert the root moves monotonically with bounded per-tick deltas over
the whole walk. Sister: a deliberately doubled frame trips the check. Report anything that's render-side
interpolation (the main agent's).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/cutscene-walk-stutter`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T10:52Z

---
## Report (done, 2026-10-16T12:39Z)

