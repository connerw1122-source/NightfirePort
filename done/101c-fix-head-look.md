## Your task: `head-look`, heads tip sideways and look the wrong way

The user's playtest (07000007, results 1013-1758): "the models are looking kind of weird, where their heads are turning
kind of the wrong direction, wrong angle." Compared in PCSX2 at the same spots (main agent, 2026-10-13): at
`--bond 42.59,44.10,-71.71,-1.228,-0.089` on 07000007 the port's guard by the door has his head ROLLED sideways towards
the camera; the real game's guard keeps his head upright and only turns (yaw) it. At `--bond 42.86,44.10,-77.92,2.484`
the guest's head tips UP well past the camera; at `43.21,44.10,-61.98,1.309` the left guest looks up over Bond's head.
In cut-scenes Bond's own head also turns oddly (the Kimiko scene).

Find the game's head-look (the look-at bone rotation: the drone head-track code used by fix-heads / guest reactions
merged recently; `docs/systems/` for the anim/head entries): the axis order, the limits (yaw/pitch clamps), which bone,
the target point (the player's eye, not his feet or a point above), and whether it applies during a script/cut-scene
(it likely must NOT override a scene's own animation). Fix the axis/sign/clamp and the cut-scene gating. Tests: a guard
facing +x with Bond 5 units to his side at eye level: the head bone's world up-vector stays within a few degrees of
vertical (no roll) and its forward points at Bond within the clamp; during a script camera the head bone equals the
animation's own pose; sister assertions.

Also: s/fix-facial3 (task 276c) was held: merged with master it fails
`fix_cutscenes2_live::the_kimiko_scene_characters_play_their_face_tracks` and
`gpu_skin_live::gpu_palette_matches_the_cpu_pose_on_07000005_06_07`. Take over its branch: merge `origin/s/fix-facial3`
into yours, make both tests pass for the right reason, and keep its face work.

Files: `crates/nf-game/src/{anim,drone}.rs`, `crates/nf-view/src/scene.rs`.

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

**Branch:** `s/head-look`.

---
claimed-by: sandbox5 at 2026-10-14T01:13Z

---
## Report (done, 2026-10-14T02:23Z)

