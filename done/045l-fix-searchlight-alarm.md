## Your task: `fix-searchlight-alarm`, 07000006 searchlights and the guards' reaction time

These items are from the user's playtest of 2026-10-12 (the user's own words are quoted). Screenshots are referenced by their `--bond` spot: run `nf-view <iso> <level> --bond x,y,z,yaw,pitch` to stand there.

1. **Searchlights don't raise the alarm** on 07000006 ("no alarm"). Port the spotted → alarm channel path.
2. "guards shoot way too soon upon spotting you, at least give it half a second." Port the game's reaction delay exactly (the alertness ramp on `Drone+0x50C`, the first-shot timers; `docs/fields/drone.md`). Don't invent a delay: measure the game's.
3. "spotlights also look weird": check the beam/cone draw against the game.

Files: `crates/nf-game/src/{searchlight,drone}.rs`

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in your report. Pull `master` before you start and before every push: other workers merge into it all day.

## Measured in the real game (main agent + the user, 2026-10-12; master 9d82ea3)

`tests/traces/06-firstsight.trace` (07000006 from the normal spawn, god mode, nobody shooting back; per frame: Bond, every drone's state, position and live alertness `Drone+0x50C` as `a<value>`, plus `hit <frame> health … armour …` lines):
- On sighting, a guard's alertness jumps from ~0.1 to **1.0 in one frame** (d9 at 15.7 units: state 4 → 103 → 92 → 140; d1 at 9.6 units: state 4 → 115).
- **The first hit on Bond lands 70 frames (1.17 s, d9) and 128 frames (2.1 s, d1) after that jump**; then hits come in pairs 3 frames apart every ~40-60 frames.
- Only hits are visible (not misses), and the trace doesn't hold the visibility accumulator before the jump. If you need either, say so and the main agent extends the recorder and re-records.
- `tests/traces/06-searchlight.trace` started inside a beam (the alarm fired within ~1 s), so it shows the alarmed attack cycle (states 92 → 95 → 115/116 every ~100-130 frames across ~20 guards), not first sight.

**The port must match these timings.** Decode the state ids with `NDrone2_StateFuncs` (docs/fields/dronetype-tables.md) and replay the sighting in a headless test against the trace's numbers.

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

**Branch:** `s/fix-searchlight-alarm`.

---
claimed-by: sandbox3 at 2026-10-13T07:44Z

---
## Report (done, 2026-10-13T09:04Z)

