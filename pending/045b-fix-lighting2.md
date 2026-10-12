## Your task: `fix-lighting2`, finish fix-lighting so it can merge: the per-draw light selection and the colour scale

`s/fix-lighting` did the hard part: the VU1 microcode is read (the point-light formula, G-014 closed), and it added the
sim light list (`crates/nf-game/src/light.rs`: `Light_Create` with its RNG draw, `Light_Update`, the selection, the
level's ambient radiators and 0x4E lights, the guards' muzzle lights, the smoke). The main agent **did not merge it**
because the render regressed. Start from that branch: check out `s/fix-lighting`, merge current `master` into it
(master moved on), and continue on a new branch `s/fix-lighting2`.

**What the main agent found** (rendering 07000005, the P2K shot at frame 30, `nf-view <iso> 07000005 --frame 30 --pull 30 --shot`):
1. **The muzzle light never reaches the world draws.** Each draw uploads its 2 closest lights (`pick2` over
   `closest(sphere, flags)`). The port has no per-object display flags (`obj+0xF0`) and passes **0x80 for every
   draw** [S], so the level's permanent **kind-0 ambient radiators** (`Light_SetAmbientRadiators`) qualify everywhere,
   rank closer, and fill both slots. The muzzle light (kind 1) is crowded out. Experiment: excluding kind 0 from the
   world draws brings the glow back. The glow the user had approved in play is gone on this branch.
2. **With it back, it saturates.** The gun and hand go solid yellow and the parapet orange. The shader adds
   `lc.rgb · term / 128` onto `c`, but `c` is still the vertex colour in byte/255 units at that point (it's multiplied by
   255/128 only afterwards). Check the unit chain end to end, and how the game avoids blowing out the view model at
   under one unit from the muzzle (a separate light setup for the view model, its own flags, or a clamp in the VU program).

**What to do:**
- Find the real per-object flags: `obj+0xF0`'s 0x80 / 0x100 bits per object class (world cels in `View_DrawCelGlist`
  0x1e6538, objects in `View_DrawObjList` 0x1e69a0, characters, the first-person view model), and which kinds of
  light each class really gets. Map them per draw in the port (`Material` can carry the source object's class).
- Fix the colour units so the shader's sum is the game's (byte scale, 0x80 = 1.0), with a test that pins a known vertex's
  lit colour to the formula's number.
- Write the live checks for the main agent (see `docs/fields/README.md` and the probe `tools/trace/fieldprobe.py`): which
  light-list address to read in PCSX2 at a P2K shot on 07000005, what `obj+0xF0` should read for the parapet's cel, and so on.
  The main agent will run them on the real game and compare its render with a PCSX2 screenshot.
- Keep the render-independent tests, and add one that fails on this branch as-is: on 07000005 at the shot, the world
  draw nearest the muzzle must select the muzzle light.

Files: `crates/nf-game/src/light.rs`, `crates/nf-view/src/{gpu,scene}.rs`, `crates/nf-view/tests/lighting_live.rs`.

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

**Branch:** `s/fix-lighting2` (from `s/fix-lighting` + master).
