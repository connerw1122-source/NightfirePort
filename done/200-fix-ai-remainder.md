## Your task: `fix-ai-remainder`, the guard AI items still open in GAPS G-026 / G-023 / G-021

`015-fix-npcs` owns the playtest bugs (C1–C3, the torso twist, the 07000006 stationary guards, the searchlight alarm). `110-fix-impacts` owns impact effects and ricochets, and `050-fix-lighting` owns the muzzle light. Don't touch those. Yours:

1. **SpecialDeath scripts:** guards with a scripted death (falls from ledges and balconies, death animations chosen by the placement). Find the placement field and the dispatch in `docs/systems/drone-damage.md` / the decompile (`NDrone2_*Death*`, `SpecialDeath`), and port them.
2. **Head tracking and the aim bone:** guards look at and aim towards Bond in the pose (the head and spine bones). Port the look-at the game applies after the animation sample, with its clamps. Coordinate with the torso-twist fix in `fix-npcs`: if master already has a look-at, extend it rather than adding a second one.
3. **Corpse fading:** the renderer's alpha fade-out of dead guards (the timer and the alpha ramp), and the removal at the end.
4. **Bond's 16 hit boxes:** guards' bullets currently hit Bond as one shape. Port the per-bone hit boxes and the per-location damage multipliers.
5. **Reload animation and the other drone weapons:** drones reloading (clip count, animation and sound) for every weapon guards carry, not just the default.
6. **Weapon swaps by animation events** (G-023: `DroneAnim_EventFunc`, datum 0x28/100), and dropped weapons matching the held one.
7. **Missing drone skins** (G-021, 33 placements): load the common animation file (`CommonAnimFile`, LoaderLoad type 4) and the sleeve substitutes (`AnimSleeveGetEntity`), so those guards draw and animate.
8. **Drone modes 9 / 0x12 / 0x1E in the alarm**, and the cover-node order in `Drone_InitComms` (G-026 notes).

Do them in this order; each one is separate. Every `Rand_*` call goes in the game's order.

Files: the drone modules in `crates/nf-game/src/`, `crates/nf-view/src/scene.rs` (the pose and fade hooks, minimal), and the docs in `docs/systems/drone-*.md`. Update G-021, G-023 and G-026 in `docs/GAPS.md`.

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

**Branch:** `s/fix-ai-remainder`.

---
claimed-by: sandbox2 at 2026-10-13T12:41Z

---
## Report (done, 2026-10-13T13:26Z)

