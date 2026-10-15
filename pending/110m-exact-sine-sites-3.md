## Your task: `exact-sine-sites-3`, the player-motion trig sites (G-046, third group)

G-046 (docs/GAPS.md; survey in docs/reports/fix-exact-sine-flip.md): the player sites where the game runs the exact
sine and the port still uses host trig:
- player.rs ~:1030, the movement axes (`View_RotTransMatrix` 0x1e5168:33-36 / `RotMatrixX` 0x1e46a0);
- player.rs ~:431, the ladder grab snap (`Player_CollLadder` 0x196118:33-40, direct PS2Sinf);
- player.rs ~:827 + creepwall.rs ~:34, the creep (`Player_Creep` 0x1969d0:122-125; `Vec_Spherical_2_Cartesian`);
- player.rs ~:633/~:670, the wire hand (`Player_CollWire` 0x197130 / `Player_Wire` 0x197658, direct PS2Sinf2).

For each site: match the decompile's arithmetic order exactly (the EE FPU is not fused). Write a fail-first test that
compares bit-for-bit with the game's formula where host and exact differ, and show it failing on the old code. These
are ulp-level changes; the movement must not change perceptibly. Add a test that walks Bond 600 ticks with the old and
new math side by side and asserts the end positions agree within 1e-3. Report the largest divergence. Keep the
full suite green.

Line numbers are from the survey; grep, because they drift. Files: `player.rs` (those sites), `creepwall.rs`, the new
tests, `docs/GAPS.md` (G-046).

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


**Branch:** `s/exact-sine-sites-3`.
