## Your task: `fix-bitexact`, bit-exact matrices: `PS2Sinf3` and the matrix palette

Object and bone matrices are close, but not bit-exact with the game: the port's world matrices are up to 4.6e-5 off (GAPS G-024), and object matrices use the host's `sin` instead of the game's approximation (G-017). That is small, but it adds up in collision and line-of-sight decisions, so seeded runs can drift from the real game. Docs: `docs/formats/animation.md` ("Arithmetic") and `docs/formats/placement.md`.

1. **`PS2Sinf3`:** port the game's sine and cosine approximation exactly (the coefficients and the range reduction, in f32 with the PS2's FPU behaviour: no denormals, round-toward-zero, and the clamp on overflow). Use it everywhere the game does (`View_RotTransMatrix`, `control_create_object` 0x1344e0, and every other caller you find).
2. **`psiBuildMatrixPalette`'s VU0 composition and `Quat_QuatToMat`:** reproduce the VU0 operation order and the PS2 rounding (VU0 rounds differently from IEEE: truncation, no denormals), so the world matrices match the game bit for bit.
3. **A PS2 float helper** in nf-game (multiply, add, multiply-add with the EE FPU and VU0 rounding), with unit tests on known corner cases from the PS2 documentation or measured from PCSX2 snapshots ([L]).

Tests: against the EE snapshots in `work/ee_start_*.bin` (`nf-extract pose-check` compares the port's palette with the game's), every object matrix and every bone matrix at the snapshot frame is bit-identical, with a sister assertion that the host `sin` path is not. All existing trace tests must still pass. If a trace's outcome changes, the new one must be closer to the real game's trace, and you say so.

Files: a new `crates/nf-game/src/ps2math.rs`, the matrix code in `crates/nf-formats/src/placement.rs` and the animation module, and the docs above. Update G-017 and G-024.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-bitexact`.
