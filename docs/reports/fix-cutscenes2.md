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
