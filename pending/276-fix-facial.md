## Your task: `facial`, facial animation: morphs and the facial sequences

GAPS G-020 / G-030: the skins carry a facial count (skin +0x12) and a morph index in the skinning word; the game plays facial
sequences (`AnimFacialUpdate` / `AnimFacialBlend`) during dialogue. The port's faces are frozen, which the user notices in
cut-scenes ("models in that cutscene aren't accurate").

1. Decode the morph data (where the morph targets live, how the index in the skinning word selects them) in
   `docs/formats/character.md`.
2. Port the facial update/blend and drive it from the cut-scene scripts' face tracks (fix-ai-traces2 found the two-window
   scripts: a body track plus a 22-channel face track) and from speech.
3. Tests: the morph weights over a known dialogue line's first 60 ticks match the decompile's arithmetic; a face vertex moves.

Files: `crates/nf-formats/src/{character,anim}.rs`, `crates/nf-game/src/anim.rs`, the skinning upload in nf-view.

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

**Branch:** `s/facial`.
