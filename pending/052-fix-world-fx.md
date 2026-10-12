## Your task: `fix-world-fx`, the presentation object types the port still skips

From `fix-levels-sweep`'s table (`docs/systems/levels.md` §7). These are visual or audio, but they're part of what makes
a level look like the game:

1. **0xe3 Flicker** (flickering lights), **0x4e Light** (level dynamic lights: coordinate with `050-fix-lighting`,
   which owns the light system, and only add the placement creation and the per-type update if it hasn't),
2. **0xf2 Emitter** (`Emitter_CreatePlist`, particles: smoke, steam, sparks), **0x21 Ripples** (water ripples),
3. **0xf3 EnvTree** (`Env_Tree_Create`), **0xd4 Cloud** / **0xd5 OneSided**, **0x4a Apocalypse**, **0xf4 MoviePlayer**
   (in-level video screens; the decoder is `220-fix-movies`'s: wire the placement to it, or leave a hook),
4. **0x38 SpaceLaser / 0x39 SpaceMissile** on 0700001b: check with `045-fix-finale-blockers`'s report; if they block
   the mission, they're gameplay, so port them fully.

For each: document it in `docs/atlas/<type>.md`, port the sim side (timers, spawn rates, RNG draws in game order), and
expose render data the way the snow does (`Scene` returns this frame's sprites or instances). You have no GPU: the
main agent checks the pixels, so list the spots to render in your report (`--bond x,y,z,yaw,pitch`).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in
your report. Put new behaviour in new files. Pull `master` before you start and before every push: other workers
merge into it all day.

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

**Branch:** `s/fix-world-fx`.
