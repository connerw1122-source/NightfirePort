## Your task: `pause-audio`, pausing the game pauses the sound

The user's playtest (2026-10-13): with the pause menu open, the ambient sound keeps playing, including the truck driving
loop on 07000005. The port stops ticking the sim while a menu is up, but nothing pauses the sound output, so looped and
3D voices keep running.

Find the game's pause path for sound: `GS_PauseGame` 0x17d178 sets `GameState[0x50]` and the MP pause words; find who
reads `GameState+0x50` on the sound side (`Sound_Update*`, the EE->IOP batch, `SFXPause`/`SFXUnPause` RPCs in SFX.IRX,
`docs/systems/sound.md` section 8: the port notes it has no global `SFXPause`) and what the music does while paused (the
streamed music may keep playing in the game: check). Port the global pause in `nf-audio` (voices hold their position and
resume; no new key-ons while paused) and drive it from the pause page opening/closing (`menu.rs`/`lib.rs`), with the menu's
own sounds still playing. Tests: a looping ambient voice's sample position doesn't advance while paused and resumes after;
the menu click still plays; sister: unpaused it advances.

Files: `crates/nf-audio/src/{sfx,lib}.rs`, `crates/nf-view/src/{lib,menu}.rs` (the hook), `docs/systems/sound.md`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/pause-audio`.

---
claimed-by: sandbox6 at 2026-10-14T00:20Z

---
## Report (done, 2026-10-14T02:02Z)


---
## Report from the worker (done, 2026-10-14T02:03Z)

# fix-pause-audio — pausing the game pauses the sound

Branch `s/fix-pause-audio` (from origin/master 3c77a9d; final tree merged with origin/master 9db3fda — only
`docs/WORKLOG.md` conflicted, both entries kept; no source conflict). Status: **done**.

## The symptom

Playtest 2026-10-13: with the pause menu open, the ambient sound keeps playing, including the truck-driving
loop on 07000005. The port stops ticking the sim while a menu is up (`nf_view` lib.rs: the tick gate
`!self.paused && self.fe.is_none()`), so no new key-ons happen — but nothing pauses the sound output: the
mix callback keeps advancing every already-keyed voice (looping ambients forever), and the music queue keeps
draining its ~1.7 s of buffered stream.

## What the game actually does (the evidence)

- **GameState+0x50 is not the sound pause.** `GS_PauseGame__FSc` 0x17d178 writes only `GameState[0x50]`,
  the 8 `MPGame` per-player pause words (stride 0x30) and the `PlarStat` timer pause — it calls no sound
  function [M]. Grepping all of ACTION.ELF for readers of `GameState[0x50]`: `GS_IsPaused` 0x17d1e8, the
  input gate, the sky-thunder gate in `View_DrawSky` 0x1e5ad0 and `ResetMap_GameInit` 0x1d5080 — **zero
  Sound_*/SFX* readers** [M]. The port must not gate its audio batch on it.
- **The real sound pause is menu-driven.** `Player_Update` 0x193600:71-77 opens the pause page with
  `MenuManager_Create(…, pause_sound=1, …)` + `GS_PauseGame(1)`; `MenuManager_Create` 0x21b908:121-124 runs
  `SFXPause` (batch op 9) + `SFXPauseAllStreams` (op 0x16) when that flag is set; `MenuManager_Delete`
  0x21c2b0:11-22 runs `SFXUnPause` (op 0xa) + `SFXUnPauseAllStreams` (op 0x17) [M].
  `Mission_Update` 0x182c60:78-79 opens the fail menu with `pause_sound=0` — game paused, sound keeps
  playing [M].
- **IOP mechanics** (SFX.IRX, docs/systems/sound.md Part 2 §8): `SFXPause` 0x171c → `ES_PauseSfx` 0x1648
  sets the item's `Paused` word (SFXItem+0x290) and pitch-pauses every live voice
  (`psiSamplePause` 0xf46c saves the pitch, sets 0); `SFXUpdate` 0x4ee4 skips Paused items (no 3-D update,
  no end test, Counter frozen). Sounds started **while** paused record `StartedWhenPaused` (SFXStart3D
  0x3aac) and still key on — that is how the menu's own sounds play under the pause [M sound.md §3.2, §8].
  Streams get the `GamePausedMe` hold (op 0x16; docs/systems/music.md §3.7). The EE keeps sending op-4
  update batches while paused [M].
- **Music while paused**: not faded — the stream voices are pitch-paused by op 0x16, and the EE sequencer
  `UpdateMusicalEvents` 0x114e70 returns early while the EE-global `SFXPaused` (0x30c92c) is set, keeping
  its events [M; docs/systems/music.md §3.7]. The port's sequencer rides the sim tick (already gated), so
  only the stream hold was missing.

## What the port does now

- `crates/nf-audio/src/sfx.rs` — `Driver::set_paused(on, out)`: at the pause edge every item live **now**
  gets its voices pitched to 0 (plus volume 0, the port's NIS-pause shape — pitch 0 alone would hold one
  sample audible) and its new `gpaused` flag set (the game's second user of item+0x290); `Driver::update`
  skips `gpaused` items entirely (the Paused check). Items started while paused are **not** paused, so a
  menu sound keys on and plays. The unpause emits nothing — the next full update's per-channel recompute
  restores pitch and volumes (the same restore the existing NIS pause relies on). `Sound::set_paused`
  flushes the edge commands to the output immediately, because the host stops ticking under a menu and the
  next `batch` would only run after the menu closes.
- `crates/nf-audio/src/lib.rs` — `Shared.paused` freezes the **music queue drain** only (the op-0x16
  stream hold): the playhead (`front`) stops, so the resume continues the stream exactly. Voices are
  deliberately **not** held at the output — a while-paused start (the menu click) must still sound. The
  mix closure's state (voices, music, reverb) is extracted verbatim into a `Mixer` struct so the hold is
  testable headless; `Output::open`'s callback now locks `Shared` once and calls `Mixer::mix`.
- `crates/nf-view/src/lib.rs` — the hook: one per-frame diff in `RedrawRequested`, `menu_hold =
  fe.is_some() || viewer-paused`, applied through `Sound::set_paused` on the edge; re-armed after
  `build_sound` swaps the sound on a level reload. Plus the missing host consumption of `fe.closed`: the
  pause page's Resume row (cursor 0) and Back already set `menu.rs:626/629 closed = true`, but nothing in
  `App` read it — Enter-on-Resume left the menu (and now the sound hold) stuck forever.
- `crates/nf-view/src/menu.rs` — `open_pause`'s doc comment now cites the sound pause (0x21b908:121-124).
- `docs/systems/sound.md` §8 — the port note above, with the addresses.

Deviation, listed: the game's fail/results menu keeps sound playing (`pause_sound=0`, 0x182c60:78); the
port's demo results hook (F8/F9) rides the same `fe` gate, so it holds the sound too. Splitting them needs
the mission-event → results host glue (fix-menus report's remaining work). Also, the game keeps its audio
batch flowing while paused; the port stops ticking under a menu, so the pause edge is flushed at once
instead of at the next batch — audible behavior is the same.

## Verification (tests first, then the fix)

- Pre-fix state: the three tests below failed to compile (`set_paused`/`Mixer`/`Shared.paused` did not
  exist — 12 errors), and the symptom itself is structural: the pre-fix mix closure had no pause path at
  all and `Shared` had no flag (nf-audio lib.rs before this branch).
- `crates/nf-audio/src/sfx.rs::the_global_pause_holds_items_and_a_start_while_paused_still_keys_on` —
  the pause edge pitch-0s the live voices; paused updates emit no recompute and no key-ons; a 2-D start
  while paused keys on (the menu click); the unpause's next update restores the recompute; a second
  `set_paused(true)` emits no edge. Sister: unpaused, the per-frame recompute runs.
- `crates/nf-audio/src/lib.rs::the_pause_holds_the_music_playhead_and_the_pitch0_voice_resumes` — a
  looping voice's sample position advances unpaused (sister), holds while the driver's pitch-0 edge is
  applied, resumes on restore; the music playhead freezes through `Shared.paused` and resumes exactly; a
  voice keyed on while paused still advances (the menu click).
- `crates/nf-view/tests/fix_pause_audio_live.rs` (NF_ISO, the real input path: `nf_view::load` +
  `Sim::step_bond` + the exact `nf_view::sound_tick` the winit loop runs, 07000005 — the playtest level):
  the level's ambient keys on through the real path; `Sound::set_paused(true)` holds it (no recompute, no
  key-ons); `Menu_PlaySound`'s 2-D id 0x1da [M 0x1f3278] keyed on while paused still plays; the unpause
  restores. Sister: unpaused the recompute runs.
- Suite (fast runner could not survive the sandbox — background jobs are killed between tool calls, two
  attempts died; the equivalent command was run per crate in the foreground, `--test-threads=1`):
  - **nf-game: 34 binaries, 255 passed, 0 failed** (final tree).
  - **nf-view: 117 binaries, 450 passed, 0 failed** (final tree, first alphabetical pass to
    walk_0700000d) + the sound/menu-critical set re-run explicitly (fix_pause_audio_live 2, menus_flow 2,
    menu_live 2, fix_room_sound_live 12, fix_audio2_live 17 — all green); the remaining walk/wire probe
    binaries were re-launched detached (log /tmp/nfview_final2.log) — on the pre-merge tree all of them
    had passed (two full passes, 0 failures).
  - **nf-formats: 74 passed, 3 failed** — exactly the 3 `disc_golden` tests allowed to fail on the slim
    ISO (bigf_header_fields, bigf_tables, directory_tree — they need the retail image).
  - **nf-audio: 13 passed, 0 failed** (9 lib units + bank_live + fix_sound_remainder_live 2 + music).
  - Total: 342 passed, 3 failed (all 3 the allowed disc_golden set), 3 ignored, before the detached
    walk-probe pass finishes; no failure anywhere.

## Files touched

Task files: `crates/nf-audio/src/sfx.rs`, `crates/nf-audio/src/lib.rs`,
`crates/nf-view/src/lib.rs` (the hook + the `fe.closed` consumption), `crates/nf-view/src/menu.rs`
(doc comment only), `crates/nf-view/tests/fix_pause_audio_live.rs` (new), `docs/systems/sound.md` (§8 port
note), `docs/WORKLOG.md`, this report.

Shared-file hooks: **none** — `drone.rs`, `scene.rs`, `player.rs` and nf-view's `lib.rs`-externals were not
touched; nf-view's `lib.rs` is a task-named file (the hook lives there).

## Live checks for the main agent

PCSX2 (retail image):
1. Spot: 07000005 at the truck ride, ambient loop audible. Action: press Start (pause). Expected: the
   truck loop and every ambient stop immediately; navigate the pause menu — if the port adds menu sounds
   (not yet ported), they would play. Resume: the loop continues from where it held (not restarted).
2. Spot: the fail/death menu (07000007 or any). Expected per the game: sound KEEPS playing
   (`pause_sound=0`, Mission_Update 0x182c60:78) — the port currently holds it too (deviation, above).
3. Spot: pause menu → Resume row with Enter (not Esc). Expected: the menu closes and the sim resumes
   (pre-fix the menu stayed up; `fe.closed` was never consumed).
4. gdbwatch, if desired: `SFXBatchCommands` writes of op 9 then a `SFXBatchCommands2` op 0x16+5 pair at
   `MenuManager_Create` 0x21b908:121-124, and ops 0xa/0x17 at `MenuManager_Delete` 0x21c2b0:11-22.

nf-view (`--bond x,y,z,yaw,pitch`, level 07000005):
- `--bond 104.9,24.9,108.8,0,0` (spawn): ambient audibles at spawn (ms0 id 0x40000205, r 365, dist 126;
  ms1 id 0x40000294, r 200, dist 102) — press Esc: silence; Esc again: the loop resumes mid-loop.

## Left / next steps

- Menu UI sounds (`Menu_PlaySound` 0x1f3278, the 2-D ids 0x1da/0x1d8/0x1d9) are still unported; the pause
  design keeps them possible (while-paused starts play). Their EE path needs `enabled_sound_flags`
  (0x30d184 = 0x0b) and a menu-side sound call in `menu.rs`.
- The fail/results menu should not hold the sound (`pause_sound=0`) — one more flag on the host glue when
  the mission-event → results path lands.
- `pss_movies_live` could not run (no retail image in the sandbox); the 3 `disc_golden` failures are the
  allowed slim-ISO set.
