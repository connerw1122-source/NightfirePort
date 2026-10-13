## Your task: `gpu-skinning`, GPU skinning, the pose cache and 120 fps rendering

GAPS G-043: on 07000006, `posed_vertices` costs 13.2 ms/frame (76 characters, 379k vertices) and `animate` 5.8 ms. In order:
1. **GPU skinning:** `CharVertex` already carries the bind pose + weights + two bone slots. Move the bone matrices to a storage
   buffer and skin in the vertex shader. The existing `gpu_palette_matches_the_cpu_pose_*` test is the oracle: the GPU path
   must give the same positions (worst error ≤ the current 1.5e-5).
2. **The tick-keyed pose cache** (no re-pose when no sim tick ran).
3. **120 fps:** logic stays 60 Hz; render at the display rate, interpolating the camera and poses between the last two sim
   states by the fractional tick. `--fps N` cap; no gameplay change at any frame rate (a test that 60 and 144 Hz render
   loops produce identical sim states).

You may have no GPU: build it so the CPU path stays as the fallback and the oracle test runs where an adapter exists. The
main agent renders and compares.

Files: `crates/nf-view/src/{gpu,scene,lib}.rs` and the shaders.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/gpu-skinning`.
