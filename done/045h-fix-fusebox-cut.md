## Your task: `fix-fusebox-cut`, fusebox wires are cut by hits (the laser), not by pressing E

**The user (2026-10-12):** on 07000008 the guard-room fusebox has to be burned with the laser gadget. The port opens it
with two E presses, a stand-in introduced by fix-objects2 and kept by fix-party-mission to drive its chain test.

**The game:** `FuseBox_Update` 0x1adfe8 walks the fusebox's wire objects; for each it filters the wire's hit list
(`obj+0xD0`) with `Collide_FilterBullets(list, 0xfffd)` [M 0x1adfe8:~88]. A wire with a remaining hit dies
(`Control_NextLOD`, `Effect_RemoveSObjs`); a group-1 wire's death hurts the player only when +0x56 == 0 and clears
+0x5C [M :99-102, see docs/fields/fusebox.md]. So a wire is cut by a projectile hit. Find which projectile classes
pass the 0xfffd mask (read `Collide_FilterBullets` and the bullet class word), and confirm the laser watch / Q-Laser
beam's projectile is one of them (fix-gadgets' beam ray, `crates/nf-game/src/gadget.rs`; the watch's bullets carry
the weapon id). Say whether ordinary gunfire also cuts wires.

**Do:** port the hit-driven wire cut through the real bullet/beam path, remove the E stand-in (or prove from the
decompile that `Player_Activate` 0x1a5170 can cut a fusebox, then keep it), and re-drive
`fix_party_mission_live::the_07000008_safe_chain_works_in_play` and the 07000014 sabotage test through the real cut.

Tests (NF_ISO, fail-first): the laser on 07000008's fusebox #219 cuts its wires → ch 137; the sister: E presses do
nothing (unless the decompile says otherwise); the 07000014 sabotage completes with the real cut.

Files: `crates/nf-game/src/fusebox.rs`, `gadget.rs`, the bullet hit path in `drone.rs`/`bullet.rs`,
`crates/nf-view/tests/fix_party_mission_live.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

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

**Branch:** `s/fix-fusebox-cut`.

---
claimed-by: sandbox5 at 2026-10-13T05:54Z

---
## Report (done, 2026-10-13T07:34Z)

