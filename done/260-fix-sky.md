## Your task: `fix-sky`, the sky remainder: lightning, the rotating sky, the sky pass's GS state

The sky draws (type-0x2A placements, slot order, viewer-centred; WORKLOG 0014). Still open (GAPS G-019):

1. **Lightning variants:** sky parameter 1, the random flicker (its `Rand_*` draws in game order), the flash timing, and whether the flash also lights the level (the ambient, or a light through `Light_Create`; `050-fix-lighting` owns the light system, so call into it or leave a marked hook).
2. **The rotating sky of 0x0700001B:** the rotation rate and axis.
3. **The sky pass's GS state:** the real blend, fog and depth state of the sky draws, from the first CNT's `DIRECT` A+D writes, against the pipeline flags `gpu.rs` uses now (`pass 0` / `pass 4`).
4. **The snow and rain particles' look** if they're part of the sky system (the density, the speed and the size per level).

You have no GPU: build the data and state, and write tests on it. The main agent renders and compares with PCSX2. List the levels and spots to compare in your report.

Tests: the lightning timer and flicker sequence from a seeded RNG match the decompile's arithmetic over 600 ticks, with a sister assertion; the 0x0700001B rotation after N ticks; the GS state per sky draw against the decoded A+D writes.

Files: the sky code in `crates/nf-view/src/scene.rs` (the sky part), `crates/nf-view/src/gpu.rs` (the sky pipeline state only), a new `crates/nf-game/src/sky.rs` for the timed parts, and `docs/systems/render.md`. Update G-019.

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

**Branch:** `s/fix-sky`.

---
claimed-by: sandbox6 at 2026-10-13T15:48Z

---
## Report (done, 2026-10-13T16:48Z)

