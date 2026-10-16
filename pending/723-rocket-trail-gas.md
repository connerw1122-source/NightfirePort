## Your task: `rocket-trail-gas` — the rocket's smoke: count, size, life

In the 08 side-by-side against PCSX2, the port fills the corridor with grey haze during a rocket's flight. The PS2
corridor stays mostly clear: a thin trail, no haze.

Audit the gas puffs a rocket spawns, against the game:
- `Bullet_DoTrails` 0x12d890 (the trail smoke 0x0200_001f — spawn interval, start/end scale, rgba, life);
- the muzzle smoke in `Player_MuzzleFlash` 0x1a4298 (skipped for `+0x68 & 0x2000` records);
- any launch puff.

Compare the port's `gas.init` calls and the gas update (scale growth, fade) for these kinds [M Gas_Init / the gas
update per kind]. Fix any mismatch in the sim.

Tests: one 0x2d shot on 08 spawns the game's number of puffs with the game's per-puff data and lifetimes.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour and say so in the report.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/rocket-trail-gas`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
