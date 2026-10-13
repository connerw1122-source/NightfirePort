## Your task: `exact-math-swap`, switch the sim to the bit-exact sine and matrix path

GAPS G-017 / G-024: `anim::sinf` and the EE-FPU `Quat_QuatToMat` are bit-exact against the recorded real-game calls
(`tests/diff_calls.rs`), but switching the sim to them flips a damage outcome in the 07000001 contact-escort walk (guard 5
dies a round early; repro in docs/reports/verify-bitexact.md). The port should run the game's exact math.

1. Root-cause the flip: run both paths side by side, find the first tick and value where they diverge, and decide which one
   the game agrees with (the `ai_traces_live` harness on 01-doors/01-raptor is the reference).
2. Swap `placement::object_matrix` and the palette path to the exact models. Re-pin any seed/trace test the swap moves, with
   the evidence (a trace match improving, not just a passing seed).

Files: `crates/nf-game/src/{anim,placement,ps2math}.rs`, the tests that move.

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

**Branch:** `s/exact-math-swap`.

---
claimed-by: sandbox2 at 2026-10-13T21:20Z
