The user's playtest 2026-10-15 (results id 1015-1738) said: "Even standing directly at a wall and throwing it at it, it
still bounced. Also, the numbers aren't working on it" (Satchel Charge 0x3b, 07000005, thrown at the low wall by
the start: --bond 105.97,24.87,110.12,-4.369,-0.166). The worker for 109z couldn't make it bounce headless, so the
headless setup differs from play somewhere: replay the exact window input path (the throw from the view model at
the real release frame, at a wall 1-2 m away, and while walking), find the bounce, and fix it from the stick arm
[M Bullet_CollisionHandler 0x12ce80:256-266]. The countdown digits: check the sim side feeding `phase.digits` and
the satchel list `frame_state` reads (kinds 4/5); the drawing is the main agent's, but the data must be there.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/satchel-stick2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
