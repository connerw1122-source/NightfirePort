Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-explosions`, explosions and destructibles

1. Port `Explode_Create` and the explosion entity: lifetime, the visual (script 0x6000052: flash, fireball, debris), camera shake, sound, and damage falloff through `Explode_Propagate`'s 12 per-type cases (guard-ai port map: currently 2 of 12).
2. Explosive objects: barrels and anything breakable that explodes, from the placement types the levels use.
3. Grenade and rocket impacts use it.

A test per item through the real Sim path: a grenade explodes and its entity, the damage values and the sound id match the decompile.

Files: a new `crates/nf-game/src/explode.rs`, `drone.rs`'s `Explode_Propagate`, effects drawing in `crates/nf-view/src/scene.rs`.

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

**Grenade follow-ups (user retest after `fix-player3`):**
- The thrown grenade kills a guard on impact, bounces back and damages Bond, and there's a bounce sound but **no explosion sound or effect**. Check the impact behaviour per grenade type (the frag fuse versus contact), the explosion sound id and the listener, and port the explosion visual (script 0x6000052, `Explode_Create`).
- **The flashbang (stun grenade) kills enemies**, even when it isn't thrown at them. A stun grenade must stun, not deal lethal damage: per-type damage (`weapon_data`, the 0x36 flags), and the stun receiving states (`StunGrenadeImpact`/`Loop`/`Recover`).
**Branch:** `s/fix-explosions`.
