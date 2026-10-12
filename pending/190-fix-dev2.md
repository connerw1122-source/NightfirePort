Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-dev2`, every weapon gets a first-person model (the dev panel)

The dev panel (`crates/nf-view/src/panel.rs`, http://127.0.0.1:7007 with `--dev`) can give any weapon, but most show "no 1st-person model here": each level archive only ships the first-person skins of its own weapons. The user wants every weapon to have its first-person model on every level.
1. At startup, scan every level archive on the disc (FILES.BIN) for first-person weapon skins (`weapon_data+0xDC`, per weapon, after `Player_Init`'s overrides) and their sleeve meshes, and build an index: skin hash → archive.
2. When a level loads with `--dev`, load the missing first-person skins (and their textures, skeletons and sequences) from the other archives and add a view model for every weapon, the same way `Scene::build` builds them. GPU skinning (`Scene::skin_layout`) must include them.
3. The panel's catalog then shows a model for every weapon. Test: on 07000005, every selectable weapon has a view model.

This is dev mode only. Normal play must be unchanged: no extra archives loaded without `--dev`.

Files: `crates/nf-view/src/{lib,scene,panel}.rs`, and the archive helpers in `crates/nf-formats`.

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

**Branch:** `s/fix-dev2`.
