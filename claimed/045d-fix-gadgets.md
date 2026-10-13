Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-gadgets`, Bond's gadgets (currently 0 of 8 ported, removed from selection)

Read `docs/port-map/gadgets.md` first.

Port, one at a time, each with a test, and re-enable it in the G-key cycle only once it works:
1. Q-Claw grapple (`Player_SSGrapple`, substate 2; grapple points are placement 0x3A, already parsed);
2. Q-Decryptor (`Player_SSScanMode`);
3. Q-Laser (`Player_LaserPointer`);
4. the laser watch and any others the 07000005–07000007 levels give Bond.

Include each gadget's HUD, view model, sounds and the world interaction (grapple targets, locks, cuttable objects).

Files: a new `crates/nf-game/src/gadget.rs`, `player.rs` substates (pull often: `fix-ladder2` and `fix-wire` also edit it), the gadget view models in `scene.rs`.

## New from the user's playtest of 2026-10-12 (highest priority: "need to prioritize getting every gadget and weapon to work properly")

- **Every gadget must be selectable and usable.** The user could not find or equip: the **Q-Worm** on 0700000a ("cant find the gadget"), the **camera** on 0700000c ("I also can't equip the camera gadget. Don't know how to. Don't think it's possible"), the **briefcase** gadget picked up on 07000012 ("it wouldn't let me switch to it, not sure how to"). Port the game's gadget selection (the inventory and the select input) so every gadget the levels give Bond can be selected, then make each one work.
- **The laser watch** must melt the hinges of 0700000c's roof hatch (`045h-fix-level-0c` owns the hatch side).
- Say in your report which key selects gadgets, so the user knows.

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

**Branch:** `s/fix-gadgets`.

---
claimed-by: sandbox1 at 2026-10-13T01:13Z
