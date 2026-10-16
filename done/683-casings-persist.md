## Your task: `casings-persist` — what spent casings do after landing (and an optional keep setting)

Playtest 1015-1738: "Do you think you can keep the bullet casings as like physical objects ... Maybe like keeps a
certain amount." First port exactly what the GAME does after a casing lands: its rest state, its lifetime, the
fade, the 32-casing pool's recycling order [M Bullet_InitCasingLists 0x12fd48, the casing update / deletion,
`nf_game::casing`]. Then add a PC-port option (default = the game's behaviour), "Keep shell casings: N" (0, 32,
64, 128), which keeps the newest N resting casings, still recycling oldest-first. Deterministic: no wall-clock.
Tests: fire 40 rounds and assert the game's count and lifetime by default; with keep = 64 the resting casings are
held and the pool recycles oldest-first. Sister: casings in flight still collide and bounce the same. Data/sim
only (the casings already draw).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/casings-persist`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T10:14Z

---
## Report (done, 2026-10-16T11:17Z)

Closed by the driver: the report is on `s/casings-persist` (docs/reports/casings-persist.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T12:09Z)

