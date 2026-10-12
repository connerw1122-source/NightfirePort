Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-mission`, the mission layer: objectives, mission flow, level ends

Several level ends and NPC behaviours wait on mission scripting the port lacks. For example, the 07000001 exit needs a skin-0x4F NPC who waits on channel 0xB, which nothing on the disc writes (WORKLOG `fix-level`).
1. Find the game's mission/objective system in the decompile: objective lists, completion flags, `PlrStat_*`, the mission-complete screen, failure conditions (killing civilians, alarms), and the code that writes channels like 0xB.
2. Port it: objectives per level, completing them, the HUD objective notice, mission success leading to the level end, and mission failure leading to a restart.
3. Make the 07000001 exit work end to end (the NPC leads Bond to the exit plate, and the level loads), and document every level's objectives in `docs/systems/levels.md`.

Files: a new `crates/nf-game/src/mission.rs`, hooks in `triggers.rs`/`nis.rs`, HUD notices in `crates/nf-view/src/hud.rs`.

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

**Branch:** `s/fix-mission`.
