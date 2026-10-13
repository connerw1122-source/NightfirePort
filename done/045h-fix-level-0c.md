## Your task: `fix-level-0c`, make 0700000c playable

These items are from the user's playtest of 2026-10-12 (the user's own words are quoted). Screenshots are referenced by their `--bond` spot: run `nf-view <iso> <level> --bond x,y,z,yaw,pitch` to stand there.

The user: "This mission's all sorts of bugged" and "not playable".
1. **The way down is a hatch that doesn't work**, and crouching through it "you're blocked off, which soft locks you". Find the hatch's object and its collision.
2. **The hatch behind you across the roof** opens with the laser watch melting its hinges; it doesn't work. Port the hinge-cut interaction (the laser itself belongs to `045d-fix-gadgets`; you own the hatch side and its channel).
3. **The camera gadget can't be equipped** (the jetpack photograph objective). Coordinate with `045d-fix-gadgets`.
4. "All of the AI in this level is bugged as hell. The walking animation is kind of fucked too." Reproduce on 0c, compare the guards' states and animations with the game (`docs/fields/drone*.md`), and fix.
5. **A white untextured floor patch** on the roof (spot `0700000c --bond 14.22,16.73,-154.43,9.919,-0.349`). Find the object and why it draws untextured (`045i-fix-render-artifacts` owns the draw if it's a renderer bug).
6. The guards that used to be invisible here (models-characters) need the level playable before the user can check them.

Files: `crates/nf-game/src/*` for the hatches and AI, `crates/nf-view/src/scene.rs` for the draw

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

**Branch:** `s/fix-level-0c`.

---
claimed-by: sandbox4 at 2026-10-13T05:27Z

---
## Report (done, 2026-10-13T06:20Z)

