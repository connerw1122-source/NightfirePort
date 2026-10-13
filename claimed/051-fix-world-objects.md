## Your task: `fix-world-objects`, the gameplay object types the port still draws as static geometry

`fix-levels-sweep` (merged; `docs/systems/levels.md` §7) listed the placement types every level still skips. These
change gameplay, so port them (the creators are in `parsemap_create_dynamic_objects` 0x1d04f0):

1. **0xd2 Shooter** (`Shooter_Create`, turrets; 07000001, 07000015, 0700004a): their aim, fire, damage, the channel that
   arms them, and whether they can be destroyed.
2. **0xd8 RB** (`RB_Create`, rigid bodies; 01, 03, 04, 06, 0d, …): crates and barrels that fall, tumble and get pushed.
   Port the integration the game uses (its own step, gravity, collision response) exactly, not a generic physics engine.
3. **0x3b DynamicObject** (`DynamicObject_Create`; 03, 05, 07, 0c, 0d, 14, 1b, 41): find what they are per level and
   port their update.
4. **0x41 InverterTrigger** (06, 14), **0xe7 ThirdCam** (05, 08, 0c: the third-person camera volumes, which also give
   the wire crossing its camera), **0xda Car** (where present).

For each type: document it in `docs/atlas/<type>.md` (the creator, the instance struct, the update, the
placement-parameter mapping with tags), port it, and test it on a level that has it (NF_ISO, with sister
assertions). Skip anything another task already owns: grapple points (`150-fix-gadgets`), creep walls
(`140-fix-swim-vent`), SimpleScripts (`120-fix-explosions`), coronas/lights (`050-fix-lighting`).

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

**Branch:** `s/fix-world-objects`.

---
claimed-by: sandbox5 at 2026-10-13T09:16Z
