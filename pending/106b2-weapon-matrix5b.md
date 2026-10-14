## Your task: `weapon-matrix5b`, weapon matrix 5: rebuild on master's matrix4b

Base: `s/weapon-matrix5` — merge origin/master FIRST (master now has weapon-matrix4b = matrix3 + matrix4 + the in-suite
sample, and fix-samurai-fire's `Gun::has_ammo(…, clip_slot)` signature). HELD: on master + this branch alone,
`weapon_matrix_live::upgraded_runner_checks_the_new_behaviours_on_07000005` fails at weapon_matrix_live.rs:227 "0x54:
the watch's twin chain must operate (action 12, or the OR'd fire press)" — your gap-1 fix (the watch pair's mode press
ORs the fire edge) doesn't hold on master's harness. Rebuild your three harness fixes on matrix4b's runner, keep the
in-suite binary under 1200 s, and re-run the sweep; report the red list as numbered findings. Run the full suite.

Files: `crates/nf-view/src/matrix.rs`, `crates/nf-view/tests/weapon_matrix_live.rs`.

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

**Branch:** `s/weapon-matrix5b`.
