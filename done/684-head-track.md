## Your task: `head-track` — guards' heads turn the right way

Playtests 1013-1758 and 1015-0046: "how all the models are looking kind of weird, where their heads are turning
kind of the wrong direction, wrong angle", and "how weird their head looks when they're turning". Port and verify
the head/neck tracking:
- the target choice;
- the yaw/pitch limits and rates;
- the bone(s) it drives and the composition order with the body animation [M the head-track code in the drone
  update / `AnimProcess` (fix-hat-details / 'Guards turn their heads' touched it), the limit constants].

Compare against a PCSX2-free source: the decompile's constants plus a frame-stepped headless trace. For 3 guard
poses (Bond left, right and above), assert the head bone's world yaw/pitch per tick against values computed from
the game's formula, and that the limits clamp. Report any convention flip found (left/right, the axis order).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/head-track`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T10:25Z

---
## Report (done, 2026-10-16T11:34Z)

