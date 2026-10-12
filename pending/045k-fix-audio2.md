Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-audio2`, the audio remainder (GAPS G-032)

- **G-032d:** the exact SPU2 Gaussian interpolation table (currently Catmull-Rom).
- The cel/room-driven `SFXSetEnvironment` (reverb per room, now that cels and portals exist).
- The speech remainder (`docs/systems/speech.md`).
- Bullet casing sounds (the `fix-impacts` task, 110, ports the casings and owns the landing-sound trigger if it lands first; you own the sample choice), the 0x601 body-hit variant, drone foley (`NDrone2_PlaySFX`, and its RNG pitch draw in game order), sound precache, and pausing sounds during cut-scenes.
- Music events from systems ported since (objectives, exits, room triggers).

Each one with a test against the decompile's values.

Files: `crates/nf-audio/*`, `crates/nf-game/src/{sound,music}.rs`.

## New from the user's playtest of 2026-10-12

- **Voices are missing everywhere**: cut-scenes on 07000001, 07000005 (the truck ride end: "it transitioned but there were no voicelines") and 07000007. Speech is now the top item of this task (coordinate with `045j-fix-cutscenes2`, which owns the NIS side; you own the voice streams and playback).

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

**User playtest (2026-10-11, after fix-truck):** the 07000005 truck ride now ends the level, but **no voice lines play during the ride**. In PCSX2 the ride has dialogue. Find what plays it (the truck script's sound/speech events, `docs/systems/speech.md`, the script-player speech events), reproduce it headless along `fix_truck_live`'s ride (assert that the speech starts at the script's tick), and port it.

**Branch:** `s/fix-audio2`.
