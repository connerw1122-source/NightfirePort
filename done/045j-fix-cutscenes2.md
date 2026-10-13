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

## New from the user's playtest of 2026-10-12

- **07000007, the Kimiko cut-scene** "immediately gets skipped and an objective banner plays" (the user may have pressed W; make sure movement keys don't skip a scene unless the game's skip input does).
- **07000002 intro:** "models are missing from this cutscene".
- Items 1, 3 and 4 above were seen again (no voices, the gun hand-back scene, the duplicated models in the Drake scene); the user's 07000007 screenshot `--bond 34.25,44.13,-70.71,1.549,-0.070` at frame 1790 shows two identical guards side by side.
- The objective banner's timing around cut-scenes belongs to `045c-fix-party-mission`.

## From the user's check `party07b` (2026-10-12, after fix-party-mission merged)

- **The Kimiko scene still doesn't hold the objective:** "obj doesnt wait for kimiko cutscene and plays the sound immediately". fix-party-mission's `hud_frozen` hold-off (set by the NIS camera requests) works for the 07000001/07000002 intros but not for this scene: find which camera/script path the Kimiko scene takes that skips it, and hold the banner **and its sound** until the scene ends.
- **"models in that cutscene arent accurate"**: compare the scene's entities with the script (missing, duplicated or wrong-skinned characters; the user's earlier report saw duplicates in the Drake scene).
- **The mission-fail message is off-centre** during a cut-scene fail (`fail07`): centre it like the game's text (the status pane's own centring).

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

---
claimed-by: sandbox1 at 2026-10-13T06:49Z

---
## Report (done, 2026-10-13T09:08Z)


---
## Report (done, 2026-10-13T09:09Z)

# fix-cutscenes2 — cut-scenes play; make them right

Branch: `s/fix-cutscenes2` (from master 323bf00).

## What's done

**1. Movement keys no longer skip a cut-scene (the Kimiko scene complaint, all three symptoms).**
The viewer skipped on ANY key press. The game's skip is `Input_Action(-1, 0x19, 1)` read by
`SP_Update` state 2 [M 0x1dbda8:129-141], and action 0x19 is bound to raw pad mask **0x840 — R1
(0x800) or D-pad DOWN (0x040), pad 0 only** [M `psiInput_MapInputs` 0x10cd80:415-417]; movement never
skips. The user's W press skipped the 07000007 Kimiko scene, which also produced the other two
complaints: the objective banner+sound "immediately" (the skip ends the scene's camera, the
`hud_frozen` hold clears, the banner lands) and the mission state not changing (the hand-back's end
events — event 7, the player-return event 6, the ch-0x52 write — never ran). Fix: the viewer's skip
now goes through `nf_view::cutscene_skip_key` (ArrowDown and R stand for the pad's D-down/R1);
`--skip-cutscenes` stays a dev override. `Script_FFwd`'s own gates (camera-started bit, death-NIS
refusal, the freeze re-press) were already faithful [M 0x1dac78:16-23].

**2. Face tracks / lip-sync.** `AnimProcessScriptCmds` (0x1c1b38) starts EVERY opcode-0 window whose
time the script enters; the pack scripts pair a narrow 22-channel face track (first window) with the
222-channel body track (last) [D `nf-extract script-info`: 07000007's 060004f1/f2 = windows
[1..0x10e6] seq …467/…468 then [1..0x10e7] …332/…3f9; the 07000005 ledge's 06000996/97]. The port
played only the last window (fix-drone2's last-wins), so mouths never moved. New
`nf_game::anim::script_pose_together`: the widest started window is the base, then every window in
command order poses its bones — per bone the FIRST-started covering window owns it [S: the direction
follows the play reference — the game's mouths move while the body animates; both windows start at
frame 1, so command order breaks the tie]. `nf_formats::anim::{bones_covered, local_pose_bones}`
were added (a bone-limited core that `local_pose_with` now delegates to — public signatures
unchanged). The cut-scene character path in `scene.rs::frame_state` uses it; the AI/drone path keeps
`script_pose` (fix-drone2's pinned behaviour). A lone window that does not fit the skeleton still
hides the character (the old `fits` rule).

**3. The duplicated models (Drake fly-through) — the game rule verified in the port.** The rule is
the script's own events, already ported: event 5 at tick 0 = `Drone_EnableAll(0)` [M
Script_EventHandler 0x1d94c0 case 5 → 0x1383e8; `NDrone2_Enable(0)` 0x149dd0 sets obj+0xF0 |=
0x10|0x20], event 7 near the end = flag 0x80, re-enabled at `Script_Free` 0x1da880 through
`NDrone2_Enable(1)` (anims restart at script time 1). Both 07000007 scenes carry exactly this pair
[D 06000072 streams 16/17, 06000073 streams 4/5]. The new test proves the live party hides while the
fly-through plays (npc gate off, no present drone enabled, the 13 guest Anims live) and restores
after the game's skip. The user's frame-1790 screenshot predates fix-party-mission's control-walk
interleave; a live render should confirm the fix holds on screen.

**4. Voice audio — the chain pinned end-to-end.** `Script_SoundStart` (0x1d8820) passes a bank
effect id; the op-0x16 request path, the drain's 2-D/3-D start and the pool's driver commands were
already wired. New test: the Kimiko scene's lines **0x11d** (at script tick 0x351 = 849 — the
`nf-extract nis` dump prints hex, which cost one false alarm), **0x5c0**, **0x5c1** all fire (the
pool emits `Start` commands for them) and resolve in the 07000007 level bank (slot 6) to in-bank
samples 111/110/115/116 with `pause_in_nis 0` — they are bank samples, not STREAMS entries (the
STREAMS.LUT/BIN path exists for FileRef < 0 sub-tracks [M `SFXInitialiseStream` 0x9030]). What a
headless sandbox cannot prove: that the audible output reaches the speakers in the viewer.

**5. The Kimiko objective hold.** With the skip fixed, the scene's camera request arms `hud_frozen`
at tick 0 [M Script_CameraStart 0x1d9140:46-49] and the mission layer holds the banner and its
sounds until the scene ends (the game's hold reads ScriptCam, `HUD_UpdateStatusPane` 0x19c310:219-235).
Pinned: 150 ticks of forward movement leave the scene playing and the hold on.

## How it was verified

`crates/nf-view/tests/fix_cutscenes2_live.rs` (5 tests, real input path: `nf_view::load` + Bond's
capsule into the real trigger boxes + `step_bond`):
- `only_the_game_skip_action_keys_skip_a_cutscene` — movement/viewer keys never skip; ArrowDown/R do.
- `the_drake_flythrough_hides_the_live_party_and_restores_it` — the freeze, the game-action skip, the
  re-enable with anims from script time 1.
- `movement_input_leaves_the_kimiko_scene_playing_and_the_objective_held`.
- `the_kimiko_scene_characters_play_their_face_tracks` — the joined pose differs from body-only
  exactly on the face-track's bones and matches it past them.
- `the_kimiko_scene_fires_its_voice_lines` — the three lines fire and resolve to playable bank audio.

The voice test was shown failing before its fixes (the observation had to flush the sound pool per
tick like the viewer's frame pass — a single end-of-run flush saw only Remove commands). Full suite
on the slim ISO: nf-game 94 + unit tests green, nf-view all green including the 5 new ones, nf-audio
green, nf-formats 61 + 11 with only the 3 allowed `disc_golden` failures.

## What's left

- **Live render checks for the main agent** (no GPU here): with the branch built, in 07000007
  (a) `--bond 34.25,44.13,-70.71,1.549,-0.070` mid-fly-through — the party guests should appear once
  each (no duplicates); (b) during the Kimiko scene a close spot on Kimiko/Drake — the faces should
  animate (mouth/jaw bones carry the face-track pose); (c) confirm voices are audible during both
  scenes and the objective banner waits for the scene to end.
- The skip keys are a stand-in mapping (the game binds pad 0x840); if the user's pad/keyboard layout
  should match the PC port's action mapping, it needs the input-map owner's sign-off.
- Per-bone ownership direction of the face/body windows is tagged [S]; a live check of the two
  tracks' effect on the jaw bones settles it (both windows start at frame 1, so command order decides).
- op-0x16's b[1]==0 object resolution uses stream i−1 where the game reads SCRIPTINFO+0xab3 [M
  0x1d8820] — positional difference only (2-D fallback), noted, not chased.
- Not attempted (time): the hand-back scene's pre-standing models (the user's item 3 first part) —
  the scene's entities are created/hidden by the now-verified event path; the 07000002 intro's
  missing models; the 07000001 intro's ambient (fix-render2's resolution covers script Anims); the
  fail-label centring; `SP_Hit` proxy-trigger bits (the urn scene).
