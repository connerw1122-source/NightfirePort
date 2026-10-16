## Your task: `sim-tick-cost` — profile and cut the Sim's per-tick cost without changing results

On 07000005, the window's 60 Hz sim ticks cost about 6.7 ms per frame (main agent probe, 2026-10-15). An earlier
measurement (fix-perf-level) had about 4.1 ms per tick on 07000006. The window caps catch-up at 4 ticks, so a slow
tick turns into dropped game time on slower PCs.

1. Add a headless tick-cost probe test (NF_ISO, release-only): load each single-player level, run 600 ticks of a
   fixed input script, and record the per-tick median, p95 and max, broken down by the Sim's major phases (drones/AI,
   bullets, collision, triggers/scripts, sound batch, pickups, doors). Table the results per level in the report.
2. Find the top hotspots (for example linear scans per drone per tick, repeated cel lookups, allocations in the
   tick) and fix them.
3. **Determinism is the bar:** the per-tick state hash (the net-determinism work, 050, if it has landed; otherwise
   hash the drone/bond/bullet state yourself) must be identical before and after for every level over the 600
   ticks. Report before/after timings.
No behaviour changes. If a fix would change any result, don't make it: write it up in the report instead.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/sim-tick-cost`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T02:06Z
