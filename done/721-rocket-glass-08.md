## Your task: `rocket-glass-08` — the 08 corridor rocket stops ~5 units short

A side-by-side recording against PCSX2 (2026-10-16, the main agent) fired the unguided launcher (0x2d) from the
07000008 level start straight down the corridor:
- PS2: about 72 frames from launch to explosion. At the measured 0.3367 units/frame [L] that is about 24 units.
- Port: 56 ticks, 19.25 units (the bullet starts at the halfway origin, see rocket-origin in the merge log). It
  explodes at the far window.

Find what the port's rocket hits at 19.25 units and whether the game's bullet passes it, using the bullet
collision's pass/flag rules for that surface [M `Bullet_CollisionHandler` 0x12ce80, the HITTEST flags on the
segment query; D the surface's material/flags in 08's collision]. Glass that shatters and lets the round through,
or a pass-flagged pane, are the likely answers; the decompile decides. Fix it.

Test: the 08 start shot explodes where the game's does, ~24 units, ±1. Sister: a shot at a plain wall still stops
on its face.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour and say so in the report.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/rocket-glass-08`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T19:40Z

---
## Report (done, 2026-10-16T22:39Z)

