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

---
claimed-by: sandbox5 at 2026-10-13T07:36Z

---
## Report (done, 2026-10-13T09:13Z)

# fix-audio2 — the audio remainder (GAPS G-032)

Branch `s/fix-audio2`, 2026-10-13. The session's budget went to the playtest's top item — the missing
cut-scene voices — plus the 0x601 body-hit variant. Everything else in G-032 is listed under "What's left".

## Done: the cut-scene voices (the 2026-10-12 playtest's "voices are missing everywhere")

**Symptom.** The user's play: cut-scenes on 07000001, 07000005 (the truck ride end: "it transitioned but there
were no voicelines") and 07000007 have no voice lines; PCSX2 has dialogue in all of them.

**Investigation** (probe kept at `crates/nf-view/tests/audio_probe.rs`): I dumped every script's op-0x16
`Script_SoundStart` commands and their bank resolutions on the three levels. The cut-scene scripts' own sounds are
bank samples (footsteps, ambience) plus id 0x172 (the NIS music stream, already routed to the music trigger by
fix-party-mission). The truck-ride-end script 0x060008C8 has no sound commands at all. The voices were found
elsewhere: **op-1 commands inside the cut-scene characters' animation scripts**, each pointing at a STREAMS.BIN
entry through the bank effect's negative FileRef:

- 07000005: the ride dialogue script 0x06000994 (ScriptPlayer placement 298, gate channel 0x43) starts anims
  0x06000995/96/97; 0x06000996 fires sfx 223 (LUT[249]) at frame 769 and 224 (LUT[250]) at 934; 0x06000997 fires
  227 (LUT[252]) at 838 and 171 (LUT[253]) at 986.
- 07000007: the Drake fly-through 0x06000072 → anim 0x060001FD fires 367/366/327 (LUT[16]/[17]/[18]) at frames
  650/1347/2110; the hand-back 0x06000073's anims 0x06000225/060004F1/060004F2 fire twelve lines (LUT[254..265]).
- 07000001: the intro 0x06000087 (ScriptPlayer 277, mode 1 — plays from the first update) → anims
  0x0600055C/55D/55F fire 488/489/490/491 (LUT[320..323]) and bank sample 598.

**Root cause** [M `AnimProcessScriptCmds` 0x1c1b38 case 1 + `Script_Interp` 0x1d9a08 case 2 + `AnimScriptTick`
0x1c2630]: the game's `Script_Interp` case 2 overrides the anim script's time with the NIS stream's frame, zeroes
its advance, and runs `AnimObjectUpdate` → `AnimScriptTick` → `AnimProcessScriptCmds(old, new)` — an op-1 command
fires when its frame lies in (old, new] (old = 0 on the first tick, so the start frame's commands run then), and
plays `Sound_Play3D(sfx, obj+0x30, 100, −1, −1, pitch Rand(500)−250, ref 0, custom 0)`. The port's anim engine only
consumed op-0 (sequences) — `docs/systems/anim-engine.md`'s "Port" section listed nothing else — so every
cut-scene voice was silent while the same op-1 path worked for drones (`Drones::script_commands`).

**Fix** (fail-first, `fix_audio2_live` 4/4):
- `nis.rs`: `World::anim_ops1(script, from, to)` (the op-1 ids in the interval, implemented on `Data` from the
  anim scripts; the weapon-id remaps are skipped — they only apply to anim objects with a weapon parent, which a
  script anim object never has [M 0x1c1b38 case 1]); `Obj::Anim` gained `ticked` (first-interp flag);
  `Script_Interp` case 2 emits `Request::AnimSound { id, at }` per firing command.
- `drone.rs`: the request drains through `Self::script_obj_pos` (the anim object's discrete-0 root as the 3-D
  position) → `Sounds::play3d` + the hearing list, and consumes the shared RNG's `Rand(500) − 250` pitch draw per
  fire — one draw per fired op-1, in command order, as in the game; the pitch value itself is not forwarded (the
  EE sound layer stores and never reads it [M crates/nf-game/src/sound.rs header]). *Deviation*: the game's
  per-command fired bit (script+0x7c) is not modelled — the interval test reproduces its effect for one-shot
  forward scripts (the only kind the cut-scene anims use); the SFXPreCache warm-up (0x1c1b38 case 1's ≤ 0x33-frame
  window for flag-0x10000 scripts) is not ported — it only hides CD latency, which the port does not model
  (docs/systems/speech.md "the stream's CD latency is not modelled").

**Tests** (`crates/nf-view/tests/fix_audio2_live.rs`, all shown failing on the pre-fix tree at the voice
assertions): the truck-ride dialogue through ScriptPlayer #298's real gate channel (0x43) — the four ride lines
start at their frames and none before frame 769; the party fly-through through the real TouchOnce trigger volume
(fix-cutscenes' path) — 367/366/327; 07000001's intro — 489/488/490 and bank sample 598, none before frame 615.

## Done: the 0x601 head-hit body sound (G-032)

`NDrone2_DoHitEffects` 0x1458e0:102-109 [M]: a body hit with damage > 0.5 and `weapon_data+0x70 & 0x180 == 0`
plays **0x601** when HITDATA+0x52 (the hit part, 5 = head — docs/fields/hitdata-spawner-ally.md +0x52) is the
head, else **0x413**. The port always played 0x413 (its comment said the 0x601 variant was "not modelled").
Fix: `WeaponData::hit_flags` (+0x70) accessor in nf-formats and the part-5 branch in the player-bullet drone-hit
path. `body_hit_sounds_head_601_body_413` (fail-first at the 0x601 assertion) drives real aimed shots; the test
re-aims every shot because Bond drifts for a few ticks after a teleport, and it walks the drones until a shot
registers on the boxes (level geometry blocks some stands). Sister assertion: a torso shot starts 0x413 and never
0x601. Not ported: the player-side pair 0x600/0x412 (`Player_DealWithObjHit` 0x194458:229-236) — Bond's hit volume
is the cylinder stand-in with no part to branch on; a faithful port needs the 16-box player collision.

## How it was verified

- Both fixes shown failing first on the pre-fix tree (git stash of the two source files), passing after.
- Full suite (`NF_ISO=... cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view
  -- --test-threads=1`): green except the 3 allowed `disc_golden` failures (the slim-ISO /DRIVING files).
- The branch was rebased on master mid-session (fix-npc-doors2 moved `script_obj_pos` to an associated function —
  followed); WORKLOG conflict resolved by re-appending my entry.

## What's left (G-032, for the next session)

- **G-032d** the SPU2 Gaussian interpolation table (currently Catmull-Rom 4-tap in `nf-audio/src/lib.rs`; the
  nocash PSX-SPX table needs a web fetch + a bit-exactness test against it).
- `SFXSetEnvironment` (cel/room-driven reverb; depends on the room graph being wired into the audio path).
- Speech remainder per docs/systems/speech.md "Port status": the facial path on levels with facial scripts,
  HitTalkExtra (type 0x10), the level-specific sets, drone foley via PlaySFX (doors 0x27, kicks 0x28).
- Bullet casing sounds (sample choice owned here; the trigger belongs to fix-impacts), sound precache semantics,
  pausing sounds during cut-scenes (`SFXParameters` `pause_in_nis`, byte +0x3F — the field is already parsed by
  `Bank::parse`/`Effect`), music events from systems ported since (objectives, exits, room triggers), the op-1
  per-weapon id remaps for the view model (the swaps listed in anim-engine.md "Commands").
- The drone-side op-1 handler (`Drones::script_commands`) still does not consume the game's `Rand(500)` pitch
  draw — adding it shifts every gameplay RNG stream and needs a deliberate suite-wide pass.
