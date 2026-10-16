## Your task: `drake-cutscene-06` — the Drake cut-scene on 07000006: lag and object glitches

The user's playtest 1015-2142: "drake cutscene on o6 has some lag and glitchiness with objects".
1. **Lag:** headless, run the 07000006 Drake cut-scene (the script camera's NIS) and profile the per-frame CPU cost:
   the sim tick, the script players, `frame_state` / `skin_palette`, the script entities. 649 (frame-cost-guard)
   added budget tests, so extend them to this cut-scene. Find what spikes, fix it with results unchanged, and give
   before/after numbers.
2. **Object glitches:** list every script entity and character the cut-scene drives, with each one's matrix per
   frame. Flag the ones that jump, pop or teleport between frames (a discontinuity larger than the motion around it),
   and compare against the game's script data [M the script interp `Script_GetInterp` 0x1da158, the entity
   create/delete ops]. Fix the sim-side causes: a wrong key interp, a missed delete, an entity drawn before its
   create. Report any that look like a draw-only problem without changing the drawing (the main agent's).
Tests: the cut-scene runs to completion headless, and the flagged entities' matrices are continuous.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drake-cutscene-06`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T05:57Z

---
## Report (done, 2026-10-16T07:09Z)

Closed by the driver: the report is on `s/drake-cutscene-06` (docs/reports/drake-cutscene-06.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T07:47Z)

