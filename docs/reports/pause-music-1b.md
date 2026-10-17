# pause-music-1b — the pause entry's music calls, ported; 1b's own music updater; the ScriptCam pause gate

Branch `s/pause-music-1b` (from origin/master f09f3c6c). Status: **done (code + tests in; the full
suite could not be re-run in this sandbox — see Verification)**.

## The symptom and what the game actually does

Playtest 1016-1355: "Music still plays during the pause menu" on 0700001b; earlier fixes stopped it
on other levels.

The game's pause entry (`Player_Update` 0x193600:71-77: the pause action AND `GameState+0x36==0` AND
`ScriptCam==0` → `MenuManager_Create(0x80000002, 0x4000004b, …, pause_sound=1, 3)` + `GS_PauseGame(1)`)
has exactly two audio calls [M 0x21b908:121-124]: `SFXPause` (batch op 9) and `SFXPauseAllStreams`
(op 0x16). The op-9 EE side sets the global `SFXPaused` 0x30c92c [M 0x116b70:11-13], and
`UpdateMusicalEvents` 0x114e70 **returns on it before every event but the level-start restart**
(the E[0] block runs at 0x114e70:43-54, the gate is 0x114e70:61-64) — while paused the music board
freezes and its events persist. The port had neither the `SFXPaused` freeze nor 1b's own per-frame
music updater, and its pause-menu key (Esc) lacked the game's ScriptCam gate.

## What was measured on 1b before changing anything

`crates/nf-view/tests/pause_music_1b_probe.rs` (committed; run with `--nocapture`) walks the real
path (`nf_view::load` + `Sim::step_bond` + `nf_view::sound_tick`) on 0700001b and 07000005:

- 1b starts MapMusic track 35 (MFX_13, 11 sections) at section 4 and keys **10** bank ambient items
  (ids 933, 1303, 1273, 1304 ×2, 1271, 1220, 642, 643, +1479 later) [D the probe]; 05 starts track 15
  section 0 with 2 items.
- The engine-level pause (`Sound::set_paused`) holds both levels identically: the pause edge
  pitch-0/volume-0s every live item's voices (`Driver::set_paused`, the `ES_PauseSfx` shape) and the
  mixer freezes the music drain under `Shared.paused` (the op-0x16 `GamePausedMe` shape [M
  music.md §3.7]); a paused driver update emits 0 commands; the unpause restores.

So the mixer cannot sound while the hold stands on any level; the 1b-specific gaps were upstream of
it — the music board and the pause-menu entry itself:

## The fixes (all the game's way, cited)

1. **The SFXPaused freeze in the music board** — `nf-game/src/music.rs`: `Music::sfx_paused` +
   an early return in `Music::update` after the level-start restart, exactly `UpdateMusicalEvents`
   0x114e70:61-64 (events persist; the level-start restart at :43-54 still runs while paused).
   Wired on the App's pause edge (`nf-view lib.rs` — the game's pause entry IS the SFXPaused setter)
   and re-read every `sound_tick` from the driver's pause (the game reads the global every frame).
2. **`UpdateMFX_HT_Level_SpaceStationD` 0x114d60 (1b's own updater, was unported)** —
   `Music::space_station_d`: the alert pair (`DAT_002d3518`/`DAT_002d3504` = the port's e[8]/e[3] by
   the EventList slot map DAT_002d34fc+4k ↔ e[k+1]) jumps end_calm 10 instant + `SFXFadeDown(0)` +
   `MusicWaitForEnd=1` (1b's combat list is empty on disc [D MapMusic row 20], so the scan always
   takes the end_calm arm); the intro cinema's end (`Script_KillStream` of 0x0600085c [M 0x1d9860:74]
   → `DAT_002d3528` = e[0xc]) restarts the track at section 0, not instant, while no end-wait holds
   [M 0x114d60:40-43].
3. **The ScriptCam pause gate** — the Esc arm (`nf-view lib.rs`) now refuses the pause while a
   script camera runs [M 0x193600:72 `ScriptCam == 0`; the pause-results-flows report's live check 1]:
   on 1b the load state is the intro cinema 0x0600085c (SP #147, 2000 ticks), the port used to open
   a pause the game would refuse. (`GameState+0x36` is the gate's other half — a live check below.)

## Tests

- `nf-game/src/music.rs::the_pause_freezes_the_music_board_but_the_level_start_still_restarts` —
  fail-first (pre-fix `sfx_paused` does not exist): paused, the alert/cinema events emit nothing and
  persist; the level-start restart still fires while paused; unpaused, the persisted events process.
- `nf-game/src/music.rs::space_station_d_restarts_the_music_when_the_intro_cinema_ends` — fail-first
  (pre-fix no SpaceStationD): the cinema end restarts at section 0 (not instant); the alert arm jumps
  end_calm 10 instant + fade down; no restart while the end-wait holds; **sister**: 05's board does
  nothing on 1b's events.
- `crates/nf-view/tests/pause_music_1b_probe.rs` — the 1b end-to-end pin (the task's test): the
  level music starts through the real path, the pause holds (no paused key-ons/recompute, the music
  player's feed stops), the unpause restores; the 05 sister is the second test in the same file plus
  the pre-existing `fix_pause_audio_live` pair (05's truck ambient), still green.
- The Esc-arm ScriptCam gate is pinned by decompile citation (0x193600:72); a live-scene test of the
  window key handler is not headless-reachable — left to the pause-results-flows live check.

## Verification (what ran)

- `cargo test --release -p nf-game --lib music -- --test-threads=1`: **5 passed** (2 new + 3
  pre-existing music tests).
- `cargo test --release -p nf-view --test fix_pause_audio_live --test menu_audio_flows
  --test pause_music_1b_probe --test pause_results_flows_live -- --test-threads=1`: **22 passed,
  0 failed** (2 + 9 + 2 + 9).
- `cargo check -p nf-view --lib`: clean (35 pre-existing warnings).
- **The full suite (`tools/test/suite.py`)**: launched 4× in the background; attempt 1 died on a
  missing `cargo` PATH, attempt 2 on a full disk (a stale duplicate repo from the previous session
  was deleted to free 3.8 GB), attempts 3-4 were killed between tool calls (this sandbox reaps
  background jobs — the fix-pause-audio session hit the same). The foreground per-binary runs above
  stand in for the touched crates; **the full-suite total line is not available to me** — the next
  runner should start the suite in their first minutes per docs/tools/test-suite.md.

## Left / live checks for the main agent

1. **The pause key on a real pad** — the port opens the pause from the keyboard Esc only; the pad's
   Start (mask 0x800, gamepad.rs:75) is not wired to the pause arm. gdbwatch: `Player_Update`
   0x193600:71 reads `Input_Action(..., 0x1e, 4)` — confirm the pause action id and that the port's
   Esc stand-in matches it.
2. **`GameState+0x36`** — the pause gate's second half [M 0x193600:72]. What writes it (a live
   watch during a mission and with a menu up) decides whether the Esc arm needs a further guard
   (the pause-results-flows report suggested `!sim.disabled`).
3. **The pause menu on 1b after the fixes** — start 1b, let the intro cinema play (or Esc during it:
   no pause should open now), then Esc after it: the music and the space ambience must go silent,
   resume on unpause. Also check the cinema end restarts the music at its section 0 [M 0x114d60].
4. **[S] unverified, out of pass/fail** — the movie player's own cpal stream (`movie.rs`
   `open_pcm_out`) is the one audio path `Sound::set_paused` never holds; on the playtest tree movie
   audio did not play yet (e8cc8d50 landed after the playtest), so it cannot explain the playtest
   symptom, but a briefing-movie tail riding into gameplay would play under the pause on the current
   master. If the playtest repeats on the fixed tree, hold the movie ring drain under the pause as
   the op-0x16 shape and ask for a playtest round.
5. The full suite on this tree (see Verification).

## Usage

not visible to me — the session's token/context/cost figures are not exposed to this environment
(no token, context-window or cost readout is shown anywhere in my tooling; the only concrete number
I can cite is the decompile-research subagent's own report of prompt 274439 / completion 1662 /
total 276101 tokens, read from its return message).
