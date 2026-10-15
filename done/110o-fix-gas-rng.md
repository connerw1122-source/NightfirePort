## Your task: `fix-gas-rng`, the gas puffs' RNG consumption (G-033)

G-033 (docs/GAPS.md): `Gas_Init` 0x1aacc8 draws 6 `Rand_*` values per puff, and the port's gas system
(`crates/nf-game/src/gas.rs`) doesn't consume them from the game's RNG. Every puff (muzzle smoke, searchlight smoke,
impact puffs) therefore leaves the sim's RNG stream out of step with the game's, and every later AI roll shifts
(`Rand_Rand` is ported bit-exact, WORKLOG 0025).

Do:
1. Read `Gas_Init` 0x1aacc8 and list each Rand call: which variant, which argument, its order, and what it seeds
   (spin, scale, lifetime, drift…). Document this in `docs/systems/gas.md` (create it if it's missing).
2. Make the port's puff creation consume the same calls in the same order from the sim's game RNG (not the fill RNG),
   and use the drawn values for the fields the game sets.
3. Test (NF_ISO): a muzzle-smoke puff on 07000005 advances the sim RNG by exactly the game's count. Where a PCSX2 trace
   exists (work/*trace*), check the sequence after one shot. Sister: no puff, no advance. The full suite must stay
   green; RNG-order changes move AI fights, so investigate any flip with a side-by-side replay.

Files: `crates/nf-game/src/gas.rs`, `docs/systems/gas.md`, the new test.

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


**Branch:** `s/fix-gas-rng`.

---
claimed-by: sandbox7 at 2026-10-15T12:06Z

---
## Report (done, 2026-10-15T13:58Z)

Closed by the driver: the report is on `s/fix-gas-rng` (docs/reports/fix-gas-rng.md); the worker didn't run finish.sh.
