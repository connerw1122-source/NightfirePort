## Your task: `fix-scope-zoom2`, scope zoom: the sniper mode-press test fails on the merge

Base: `s/fix-scope-zoom` — merge origin/master FIRST. HELD (bisected: master + fix-samurai-fire + weapon-matrix5
passes this test; adding this branch fails it): `fire_modes3_live::sniper_mode_press_resets_zoom_and_swaps_damage`
"the scope zoomed in (zoom 1)" — after this branch the sniper's zoom stays 1.0 in the test's window. Decide from the
game which is right: your zoom model (the aim-start sqrt(mag), the zoom keys, the scoped gate) vs the test's
expectation (fire_modes3_live.rs ~line 452, written for the old zoom); master also has fix-weapon-anims' aim raise and
fix-aim-raise2 may land first (the aim bit set at the raise's end — the zoom should start then). Fix the code or the
test with the game's numbers; keep fix_scope_zoom_live green. Run the full `tools/test/suite.py`; total in the report.

Files: `crates/nf-view/src/{scene,hud}.rs`, `crates/nf-view/tests/fire_modes3_live.rs` as justified.

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

**Branch:** `s/fix-scope-zoom2`.

---
claimed-by: sandbox6 at 2026-10-14T23:49Z
