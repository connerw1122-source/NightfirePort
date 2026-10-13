## Your task: `interp-ghosting`, the gun and arm smear while moving (frame interpolation)

The user's playtest (2026-10-13, 07000005): "ghosting ... when I move around ... with my arm and gun". It shows only in
motion, never in a screenshot (a screenshot is one ticked frame). It came with the 120 fps interpolation
(fix-gpu-skinning / G-043): between two sim ticks `lib.rs` draws `lerp_cam(cam_prev, cam_cur, alpha)` and uploads
`lerp_palette(pal_prev, pal_cur, alpha)`. The camera is interpolated by angle (short-arc yaw), but the palette is a
component-wise lerp of world-space matrices. The view model (the hands and gun) is posed in world space relative to
each tick's camera, so mid-turn its lerped matrices don't match the lerped camera: the gun swims and smears against the
view. A component-wise matrix lerp of a rotation also isn't a rotation (it shears/shrinks mid-way).
`NF_NOINTERP=1` hides it (the playtest launcher sets it for now).

Fix it properly: (1) interpolate the view model in CAMERA space (pose it from the interpolated camera with the
interpolated local pose, or store its palette camera-relative and re-attach to `draw_cam`); (2) interpolate world
matrices as rotation (slerp of the rotation part) + lerp of the translation, not component-wise, with the existing
4-unit snap for teleports/cuts. Tests: at alpha 0.5 between two ticks with a 30° camera turn, the view model's
camera-space matrix equals the average of its two tick matrices (no swim); an interpolated world matrix stays orthonormal
(sister: the old component lerp fails both). Keep the fps-determinism proof green.

Files: `crates/nf-view/src/{lib,scene}.rs` (the interpolation and the view-model palette).

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

**Branch:** `s/interp-ghosting`.
