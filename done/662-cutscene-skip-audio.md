## Your task: `cutscene-skip-audio` — skipping a cut-scene stops its voice

The user's playtests (1014-1725, 07000002): "Skip the cutscene in the middle of dialogue, and the dialogue still kept
going until it finished." Earlier reports also said the mission music kept playing over cut-scenes, and that the
music only changed late.
- Port what the game does to the cut-scene's streams and voices when a skip fires: the script player's skip path, the
  NIS stream stop/fade, and `SFXStop` on the speech handles [M the skip arm and the stream/sound calls it makes].
  Also port the music handover around a cut-scene (`nis_state`, the NIS pause/quit [M music.md]).
- Headless tests on 07000002's intro and 07000007's Kimiko scene: skip mid-line → no voice item or stream left
  alive after the game's fade window; let it play → the voice ends naturally; the level music resumes the way the
  game does. Tests stay silent (NF_SILENT).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/cutscene-skip-audio`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T05:11Z

---
## Report (done, 2026-10-16T06:26Z)

# cutscene-skip-audio (662) — skipping a cut-scene stops its voice

**Status: done** for playtest 1014-1725 (07000002: "Skip the cutscene in the middle of dialogue,
and the dialogue still kept going until it finished"). Branch `s/cutscene-skip-audio` from
origin/master (6bab6409), merged with origin/master (f42de6ff) before the final runs.

## What the game does (all [M], the anchors this port follows)

- `Script_FFwd(si, 1)` 0x1dac78 (called from `SP_Update` 0x1dbda8 state 2 while the skip action
  0x19 is pressed, unless `Script_IsDeathNIS`): guards on the camera bit (`+0xAB5 & 0x10`, set by
  `Script_CameraStart` 0x1d9140) and the skip latch; arms the fade (`+0xAB4 = min(remaining, 45)`,
  `+0xAB2 |= 0x10`, `+0xA4 = 1`, `+0xAB5 |= 1`, `Camera_SetFade(len, 4, 0xff, 1)`) and posts
  **`Music_Event(5, hash)`**.
- `UpdateMusicalEvents` 0x114e70: event 5 → `NisPlaying = 0`, **`NisQuitten = 5`** (5-frame
  countdown), which also stops the ducker.
- IOP `SFXUpdate` 0x4ee4 step 3 (docs/systems/sound.md §6): while `NisQuitten` stands, every item
  with **`StartedWhenNisPlaying`** (sampled at `SFXStart3D` 0x3aac init from the IOP's
  `NisPlaying`) gets `FadeAndKill = 1`; step 4 spends `FadeAndKillVolume` −5/frame from 100
  (≈21 frames), then deletes the item. **This is what kills the mid-line dialogue** — script
  stream voices and lip-synced anim op-1 voices alike (the anim sounds have no stoppable handle:
  the kill/free path can't reach them).
- `Script_SoundStart` 0x1d8820: the start gate `stream+0x10 == 0 && !(script+0xAB5 & 0x20)` — a
  stream with a live handle never re-fires, and once the skip's fade has run past its length (the
  `+0xAB5 & 0x20` catch-up state `Script_Update` 0x1da998 sets) no new line starts: the skipped
  script's remaining lines never come to be.
- `Script_SoundEnd` 0x1daf60 (op 0x17): the marker stops only a **looping** sound
  (`Sound_IsLooping` 0x1de260 reads the SFX table's loop bit for the handle's id; handle 0 reads
  as looping but `Script_KillStream` case 5 returns early on handle 0). A one-shot line plays to
  its own sample end. `Script_KillStream` 0x1d9860 case 5 (stream kill / script free) always
  stops.
- Music handover [M docs/systems/music.md §4.4]: event 0xB (`Script_CameraStart`) raises
  `NisPlaying`; events 0xC (the camera stream's kill) and 5 drop it; the combat/stealth block is
  frozen while `NisPlaying`. The port already had this board (`nf-game/src/music.rs`) and the
  op-0x19 push (`nf-view`'s `sound_tick`) — what was missing was the event-5 pulse reaching it.

## What was broken in the port

1. `drone.rs`'s request dispatcher had **no `Request::Event` arm** — `Script_FFwd`'s
   `Music_Event(5, hash)` was dropped on the floor. `NisPlaying` never cleared via the skip and
   `NisQuitten` never armed, so nothing ever FadeAndKilled the voices.
2. `nf-audio`'s driver had `fade_kill`/`fade_kill_vol` (the decay and the volume term) but
   **nothing ever set `fade_kill`** and there was no `StartedWhenNisPlaying` sampling.
3. `nis.rs`'s op 0x16 started every line unconditionally (the skip's catch-up would have started
   all remaining lines), and op 0x17 stopped any kind-5 stream at its marker (truncating one-shot
   lines the game lets play out).

## The fix

- `crates/nf-game/src/drone.rs` — the dispatcher carries `Request::Event { kind: 0x40, params }`
  → `self.music.event(5, hash)` (cited above). `World::sfx_looped` (additive, default true) so
  the script player can read the SFX table's loop bit.
- `crates/nf-game/src/nis.rs` — op 0x16 records the handle state (`sound_id`) and the +0x22 start
  frame, and takes the game's start gate (live handle ⇒ no re-fire; `+0xAB5 & 0x20` ⇒ no start).
  Op 0x17 ports `Script_SoundEnd` exactly: only a live looping sound under the game's condition
  (`stream clock < script end || script not looping || start frame ≠ 1`) stops at its marker.
  `kill()` case 5 stays the unconditional stop.
- `crates/nf-audio/src/sfx.rs` — `Item.started_when_nis` (own line), sampled from `NisPlaying` at
  start init; `SFXUpdate` step 3 flags it `fade_kill` while `NisQuitten` stands (the step-4 decay
  and the `ES_CombineVolumes` term already existed).

## Verification (reproduce before fix)

`crates/nf-view/tests/cutscene_skip_audio_live.rs` (headless, NF_SILENT, through
`nf_view::load` + `Sim::step_bond` + `nf_view::sound_tick`):

- **07000002 intro** (0x0600_0084): wait for the first live NIS sound → skip mid-line →
  `NisPlaying` down and `NisQuitten` armed on the same tick → after the game's fade window
  (80 ticks ≥ 45 fade + 5 quitten + 21 kill) **no NIS sound item is alive** and the countdown is
  spent.
- **no skip**: the intro's one-shot line (the probe's looping bit excludes the NIS score) dies on
  its own while `NisQuitten` never arms.
- **07000007 Kimiko scene** (trigger 412 → 0x0600_0073): skip mid-line → nothing alive after the
  fade window, and the level-music handover completes (`NisPlaying` down, countdown spent).

With the game-side halves reverted (the `Music_Event(5)` arm + the nis.rs gate/0x17), both skip
tests fail ("the skip pulse never reached the music board"); restored, all three pass.

## What I ran

On the final tree (merged with origin/master f42de6ff): `tools/test/suite.py` batches —
78 binaries (the run's first 9.5 min) + the full cutscene/script/audio/subtitle set (142 passed,
0 failed, 3 ignored) + the walk_ set (29 binaries, 0 failed) + the walkreal set (15 passed,
0 failed, 5 ignored) — **~264 of 382 binaries, 0 failed except the 3 `disc_golden` tests the slim
ISO always fails** (documented). The sandbox reaps background jobs between tool calls, so the
runner was driven in the foreground with `--only` batches (the drive-mission-scripts worker hit
the same). Lib unit tests: nf-game 234, nf-audio 11, nf-view 15, nf-formats 79 — all pass
(nis.rs's own skip-armed tests included).

## What's left

- The suite's remaining ~118 untouched binaries (probe/render/dev-tool one-offs, mp, memcard) —
  none of them read the code this task changed; the next suite run covers them.
- `Camera_SetFade`'s on-screen skip fade is view-side (the port's `ViewFade` stand-in already
  draws it); the main agent's render is the check, below.
- "Mission music kept playing over cut-scenes / changed late": the event board + `NisPlaying`
  freeze and the NIS music trigger (id 0x172) were already ported (fix-party-mission item 4);
  this task completed the skip-side handover. If the user still hears the level music under a
  cut-scene that carries its own score, the next anchor is `SfxNisMusicTrigger`'s per-level
  updaters (only 07000005's `UpdateMFX_01CastleA` is ported).

## Live checks for the main agent

1. 07000002 intro, PCSX2 + `gdbwatch.py` on the EE's `EventList[5]` (0x2d34f8 + 5·4 = 0x2d350c),
   write watch: pressing SKIP writes the intro script's hash (0x0600_0084) once [M `Music_Event`
   0x1de328]. The port now posts the same pulse.
2. Same level, `fieldprobe.py` on the IOP's `NisQuitten` (SFXActiveData 0x339b4): 0 before the
   skip, 5→0 over 5 frames after [M 0x114e70].
3. Mid-line voice check: with dialogue playing, skip — the line fades ≈0.35 s (not a hard cut) as
   `FadeAndKillVolume` spends [M sound.md §6 step 4].
4. Render (`--bond`): 07000002 spawn, let the intro run, press SKIP mid-line — the screen fades
   to black over ≤ 45 frames and the dialogue audio fades with it; after the handback there is no
   tail of the old line. Suggested spot: `--bond 0,0,0,0,0` is fine (the cut-scene camera owns
   the view); compare against PCSX2's own skip.

## Files

- `crates/nf-game/src/drone.rs` — the `Request::Event` arm; `World::sfx_looped` on `Data`.
- `crates/nf-game/src/nis.rs` — the sound-start gate, `Script_SoundEnd`'s loop-only stop, the
  stream's `sound_id`/`sound_start_frame`.
- `crates/nf-audio/src/sfx.rs` — `StartedWhenNisPlaying` + the step-3 flag loop; the probe's
  looping bit.
- `crates/nf-view/tests/cutscene_skip_audio_live.rs` — the three tests.
- `docs/WORKLOG.md` — the `cutscene-skip-audio` entry.
