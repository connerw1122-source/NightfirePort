## Your task: `fix-wire2`, crossing the cable on 07000005, and the fusebox dev scenario

These items are from the user's playtest of 2026-10-12 (the user's own words are quoted). Screenshots are referenced by their `--bond` spot: run `nf-view <iso> <level> --bond x,y,z,yaw,pitch` to stand there.

1. **07000005, the cable:** "pressing e didnt do anything so I lept at it over the ledge and it kind of grabbed it. I then couldnt really move forward so I kept jumping 2 more times to the other side." fix-wire (merged) ported the hang; reproduce the user's sequence and fix the grab (E) and the hand-over-hand move.
2. **The dev scenario for 07000014's fusebox** "spawns above the vent I'm supposed to be in. So likely need to spawn in there crouched." Fix the scenario so Bond starts crouched inside the vent.

Files: `crates/nf-game/src/wire.rs`, `crates/nf-view/src/dev.rs`

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in your report. Pull `master` before you start and before every push: other workers merge into it all day.

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

**Branch:** `s/fix-wire2`.
