## Your task: `driver-air2`, redo the 06 driver fix without changing the 05 cover timing

`s/fix-driver-air` (109j, NOT merged) changed the sitting hip and the soft settle in `crates/nf-game/src/drone.rs`
(`TruckDriverInit`'s 0.2 clamp, the 0x140968 tail's correction ×0.125, a new `hip_anim` field). Start from that
branch (`git merge origin/s/fix-driver-air`) and read its report `docs/reports/fix-driver-air.md`.

The main agent's merge suite caught a regression, reproduced alone (passes on master + 109h + 109b, fails with 109j):

```
cargo test --release -p nf-game --test ai_live guard_takes_cover_like_the_game
panicked at crates\nf-game\tests\ai_live.rs:249:5: UnderCoverAim lasted 90 (live 37)
```

That test pins the 07000005 gate guard's cover cycle against a live PCSX2 trace (UnderCoverInit 31 ticks,
UnderCoverAim 37). A fix for the 06 truck driver must not touch an ordinary guard's cover timing, so the change leaks
outside the driver: the settle or the hip path now runs for every drone (or shifts the RNG draws the seeded test
relies on). Gate the driver-only parts the way the game does (`TruckDriverInit` sets them only on the driver), check
the 0x140968 settle's callers in the decompile for which drones really run the new tail, and if the game's settle
genuinely applies to every drone, prove it from the decompile and show why the live trace still says 37. Both
`fix_driver_air_live` and `ai_live` must pass, and the full suite must be green.

Files: `crates/nf-game/src/drone.rs`, `crates/nf-view/tests/fix_driver_air_live.rs`.


**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/driver-air2`.

---
claimed-by: sandbox4 at 2026-10-15T05:13Z
