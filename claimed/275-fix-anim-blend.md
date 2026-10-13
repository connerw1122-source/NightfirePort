## Your task: `anim-blend`, animation frame blending and speed modes (characters walk smoothly)

The user on 0c: "The walking animation is kind of fucked too." The port snaps sequences to integer frames; the game blends
(GAPS G-020: `AnimFrameSet` f/f+1 slerp; G-030: speed modes 1–3 with distance tables and synced scripts; the far-object
blend path; the `+0x538` blend timing).

1. Port `AnimFrameSet`'s fractional-frame interpolation (slerp of the rotation keys, lerp of translations) and the
   transition blend between sequences (the blend weight and its rate).
2. Port speed modes 1–3 (distance-driven playback so feet don't slide).
3. Tests: a posed bone at frame 10.5 equals the game's interpolation of 10 and 11 (bit-exact where the ps2math models apply;
   `pose-check` against an EE snapshot if you can produce one), a walking guard's foot stays planted over a stride (the
   foot bone's world position drifts < 0.05 while it's the stance foot).

Files: `crates/nf-game/src/anim.rs`, `crates/nf-formats/src/anim.rs` (read side only), docs/systems/anim-engine.md.

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

**Branch:** `s/anim-blend`.

---
claimed-by: sandbox3 at 2026-10-13T18:19Z
