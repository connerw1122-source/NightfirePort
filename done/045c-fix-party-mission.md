## Your task: `fix-party-mission`, the 07000007 party and 07000008 mission flow

These items are from the user's playtest of 2026-10-12 (the user's own words are quoted). Screenshots are referenced by their `--bond` spot: run `nf-view <iso> <level> --bond x,y,z,yaw,pitch` to stand there.

1. **Guards at the start of 07000007 shoot Bond.** "At start of this mission, enemies, the guards, are shooting at me. They shouldn't be shooting at me at all. It's only the ones in the beginning." Bond is a guest: find the game's gate (the team/alert state the party guards start in, the weapon-lock objective, and what turns them hostile) and port it.
2. **Punching a guard must fail the mission at once** ("If you punch a guard, it should be an automatic mission fail"), and **shooting a guest doesn't fail it** (the `guests07` check: "It doesn't work"). Find both fail writers (props 0x53/0x54 and `DroneFunc_OnInitDeath` 0x145bd0 are mapped in `docs/fields/hitdata-spawner-ally.md`; the hit/knock-out path for punches) and make both fail through the mission layer.
3. **Punching a guest makes all the other guests T-pose.** Reproduce it (punch a guest on 07000007) and find why the others lose their animation.
4. **The mission music keeps playing over the cut-scenes**, and on 07000008 "the cutscene music still hasn't finished" while playing. Port the music change at NIS start and end.
5. **Objective banners** "don't actually say the proper objective. It just says new objective", and they show **during** cut-scenes ("It needs to wait until after the cutscene's finished"). Port the banner's text lookup (the objective record's label) and its hold-off while a NIS runs. On 07000001 "no banner came up"; the first one the user saw was on 07000002.
6. **Guards aren't in the proper place after the Kimiko cut-scene.** Port the post-NIS repositioning (the script's final poses or the teleports it writes).
7. **07000008:** "after you end the cutscene … There's a giant pillar just blocking" the missile closet (spot `07000008 --bond 152.35,43.13,-120.76,1.261,-0.126`: dark slabs fill the doorway); "the gondola can't be activated"; "Still can't pick up the rocket launcher". fix-objects2 (merged) ported the gondola chain headless; find why it fails in play.

The cut-scene side of the same playthrough (the Kimiko scene skipped at once, duplicated models in the Drake scene, no voices) belongs to `045j-fix-cutscenes2`; coordinate through `master`.
Tests: each item through the real input path on 07000007/08, fail-first.

Files: `crates/nf-game/src/{mission,drone,ally}.rs`, `crates/nf-game/src/music.rs`, the HUD banner in `crates/nf-view/src/hud.rs`

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

**Branch:** `s/fix-party-mission`.

---
claimed-by: sandbox2 at 2026-10-13T01:09Z

---
## Report (done, 2026-10-13T05:22Z)

Closed by the driver session: the report is on `s/fix-party-mission` (docs/reports/fix-party-mission.md); the worker did not run finish.sh.
