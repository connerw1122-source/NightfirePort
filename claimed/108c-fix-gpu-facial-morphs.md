## Your task: `gpu-facial-morphs`, faces: the facial morphs on the GPU skinning path

fix-cutscene-faces / 100i2 ported the facial morph weights (`AnimFacialBlend` 0x1be648, the MORPH_DATA targets), but
their reports say the weights are applied only on the CPU skinning path; the GPU skinning path (the default since
282-fix-gpu-skinning) draws still faces. The main agent confirmed: nf-view renders of the 07000007 Kimiko scene at
frames 600 and 1400 are byte-identical with and without the faces fix. Apply the morph targets on the GPU path (upload
the per-character weight vector and the morph deltas; blend before skinning, as the game's order), so faces move in
play and in renders. Tests: the GPU palette/vertex output equals the CPU path's for a face mid-blend (numbers); render
the Kimiko close-up at frames 600/700/800 and confirm the mouth/eyes change.

Files: `crates/nf-view/src/gpu.rs`, the skinning shader, `crates/nf-view/src/scene.rs` (the upload).

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

**Branch:** `s/gpu-facial-morphs`.

---
claimed-by: sandbox1 at 2026-10-15T00:16Z
