## Your task: `turn-stutter`, the world stutters while turning

User (05, 08, 15): "While I'm turning, I see this kind of jitter in the world, kind of like a stutter when I'm
rotating... It doesn't happen to my view model or my hand or gun... Not as smooth as it used to be." and "I'm getting
like frame lag... this is not smooth anymore." and "it seems a bit stuttery... this isn't just in this level". This is
new since the 1013 playtests (the user marked the interpolation/smoothness checks ok then). Find the regression: the
camera yaw path vs the interpolated render (the view model is smooth, the world isn't - the world camera may read the
raw tick yaw while the view model reads the interpolated one, or the fps cap / frame pacing changed), frame-time
spikes (nf_game::prof), and bisect master since 2026-10-13 23:00 if needed. Fix it; report frame times while turning
before/after.

Files: `crates/nf-view/src/lib.rs` (the frame loop/camera), as found.

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

**Branch:** `s/turn-stutter`.
