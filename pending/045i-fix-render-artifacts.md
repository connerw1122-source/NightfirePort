## Your task: `fix-render-artifacts`, the render artifacts in the user's screenshots

These items are from the user's playtest of 2026-10-12 (the user's own words are quoted). Screenshots are referenced by their `--bond` spot: run `nf-view <iso> <level> --bond x,y,z,yaw,pitch` to stand there.

Stand at each spot with `nf-view --bond … --shot`, find the object in the scene's draw list, and fix it against the game's draw path (no GPU in the sandbox: assert the draw state in tests; the main agent renders):
1. **07000012, a pale-blue untextured cross of quads** floating in the corridor (`07000012 --bond -3.36,1.13,35.13,4.424,-0.124`). Probably a corona/flare mesh drawn without its texture or blend.
2. **07000012, the sun is a "T" shape with a red vertical line under it** (`07000012 --bond 26.81,1.13,33.17,6.287,-0.097`): the sky sun sprite/flare.
3. **0700000c, a white untextured floor patch** on the roof (`0700000c --bond 14.22,16.73,-154.43,9.919,-0.349`).
4. **07000008, dark slabs fill the missile-closet doorway** (`07000008 --bond 152.35,43.13,-120.76,1.261,-0.126`). Is it geometry the game hides (a closed/open pair, a 0x1000 cage, a portal), or drawn wrong?
5. **07000007, two identical guards in the same pose** in the corridor (`07000007 --bond 34.25,44.13,-70.71,1.549,-0.070`, frame 1790): duplicates (compare `045j`'s Drake-scene duplicates).

Files: `crates/nf-view/src/{scene,gpu}.rs`, `crates/nf-formats/src/level.rs`

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in your report. Pull `master` before you start and before every push: other workers merge into it all day.

## New from the user's second round (2026-10-12 evening)

6. **0700000c: a pale blue-white crane shape on the ground** (`0700000c --bond 14.22,16.29,-147.02,0.973,-0.300`), next to the white floor patch of item 3 (`0700000c --bond 14.22,16.72,-154.43,3.505,-0.279`). **Lead (main agent):** these look like baked shadow decals drawn *brightening* the floor instead of darkening it. models-world measured the per-object ALPHA_1 shapes; check the subtractive one, 0x7F00000042 = (Cd − Cs)·FIX… (A=2, B=0, C=0, D=1), and any other the port maps to an additive or normal blend, against the GS formula (ps2tek). Item 1's pale-blue cross on 07000012 may be the same bug. Fix the blend once and recheck all three spots.
7. **07000007: the party guests have a green-grey colour cast** (`07000007 --bond 68.33,44.12,-43.74,-1.111,-0.154`): dull and greenish next to the warm room. The user's check "characters lit by the room" failed. fix-lighting2 (merged) gave characters the level radiators plus the muzzle light; find what tints them green (the radiator colour order, the TAmbient tint, `obj+0x100` ambient, or the VU1 byte scale) against `docs/systems/render.md` and the VU1 program, and say what the real game shows.

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

**Branch:** `s/fix-render-artifacts`.
