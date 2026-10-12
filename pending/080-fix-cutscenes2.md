Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-cutscenes2`, cut-scenes now play; make them right (user playtest after 6faffb4)

`fix-cutscenes` made the loadable-pack scenes play. The user reports, for every cut-scene they watched (07000001's intro, 07000007's party scenes, 07000008's opening):
1. **No voice audio:** none of the characters' lines play. Port NIS speech: the script's sound/stream commands for dialogue, the voice streams (`STREAMS.LUT`/`BIN`), and whatever the pack scripts use for speech.
2. **Mouths don't move:** facial animation is missing. The pack scripts carry separate face tracks (`fix-drone2` found 22-channel face sequences beside the body tracks), and `AnimScriptPeekSfx`'s facial path is unported. Port lip-sync and face tracks, and play both tracks together, not just the body.
3. **07000007, the gun hand-back scene:** it's skipped very quickly, some models are already standing in the room before it starts and shouldn't be, and the mission state doesn't change afterwards. Check the pack scene's entity visibility before its start (objects created hidden until the script shows them), its length and skip handling, and the channels and mission flags it should set at the end.
4. **07000007, the Drake fly-through: every model is duplicated.** Both the live drones and the script's animated characters are drawn. The game hides one set during the scene (the script takes over the drone's object, or the drone is hidden while the NIS entity draws). Port that rule.
5. **07000001 intro: the models are missing ambient lighting.** Make sure pack-script characters get the same `obj+0x100` ambient and TAmbient tint as live characters (fix-render2 fixed it for archive script Anims, but pack entities may take another path).

Files: `crates/nf-game/src/nis.rs`, `crates/nf-audio`/`sound.rs` for speech, and the script-character parts of `crates/nf-view/src/scene.rs`.

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

**Branch:** `s/fix-cutscenes2`.
