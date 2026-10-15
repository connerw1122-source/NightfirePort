## Your task: `exact-sine-sites-2`, swap the game-exact trig in missile steering and the drone aim pose

G-046 (docs/GAPS.md; survey in docs/reports/fix-exact-sine-flip.md), second group:

1. `crates/nf-game/src/drone.rs` ~:5325-5326 — the missile steering: `RotMatrixZYX` 0x1e25a8 / `RotMatrix` 0x1e2368 with
   the exact sine, and the port's `asin` becomes the game's `atan2f(y, sqrt(x² + z²))`
   (`vecutil_cartesian_to_spherical_acc` 0x1e3a70:10). Use the port's existing bit-exact atan2 if there is one (grep
   `atan2_approx` / `trig.rs`); if the game's atan2f is a polynomial, port it bit-exact with a golden test
   (`work/atan_golden.txt` / `atan_samples.tsv` may already hold recorded samples).
2. drone.rs ~:926 and ~:2413-2420 — the head-track / aim pose (`AnimObjectHeadTrack` 0x1bf618:323-328 →
   `Quat_AxisAngleToQuat` 0x1e3f20).

For each site: match the decompile's arithmetic order exactly. Write a fail-first test comparing bit-for-bit with the
game's formula on inputs where host and exact differ, and show it failing on the old code first. The full suite must
stay green: AI fights have ulp-wide margins (fix-exact-math-swap's history in docs/reports), so if a fight test flips,
find out why with a side-by-side replay before you call it done.

Files: `drone.rs` (those sites), `trig.rs` if needed, the new tests, `docs/GAPS.md` (G-046).

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


**Branch:** `s/exact-sine-sites-2`.
