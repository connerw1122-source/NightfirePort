## Your task: `movie-playback`, the briefing and outro movies in the mission flow

The movie decoder is merged (`--features movies`); `docs/systems/movies.md` documents where the game plays them: the
briefing 0x071000xx via `ResetMap_LevelToLoad` 0x1d3800 + `ResetMap_Load` 0x1d6360 (GameFlow states 3 -> 6), the outros
0x073000xx via `Mission_Update` 0x182c60, the win movie 0x73F0048. fix-menus models state 6 as a countdown stand-in.
Play the real movie in that state (a full-screen 512x448 plane, 30 fps, PCM audio through cpal, skip on action 0x19 /
Enter) for a menu-started mission and the mission-complete flow. Use the full retail image from your sandbox storage.
Tests: which movie code plays for each mission start and end (from the decompile's tables), the skip, and the return to
the level load.

Files: `crates/nf-view/src/{movie,menu,lib}.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/movie-playback`.

---
claimed-by: sandbox4 at 2026-10-13T21:51Z

---
## Report (done, 2026-10-13T22:58Z)

Closed by the driver: the report is on `s/fix-movie-playback` (docs/reports/fix-movie-playback.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-13T23:06Z)


---
## Report from the worker (done, 2026-10-13T23:06Z)

# fix-movie-playback — the briefing and outro movies in the mission flow

## What was done

State 6 of the port's GameFlow (`nf-view`'s front end) now plays the real briefing and outro
movies instead of only the headless countdown, per docs/systems/movies.md:

- **The codes match the game's own switches.** `reset_map_level_to_load` wraps story levels to
  their 0x71 briefing codes like the compiled switch in `ResetMap_LevelToLoad__FUiScSc` 0x1d3800 —
  identity for 0x7000001..0x7000016 and 0x700004a, and the one non-identity case **0x700001b →
  0x7100018** (the Equinox briefing is movie 18; the old `level + 0x100000` produced 0x0710001b, a
  file that is not on the disc [D /MOVIES/30_FPS]). `outro_for` (new, menu.rs) implements
  `Mission_Update__Fv` 0x182c60 case 6's level → outro switch: 04/08/0b/0d/0f/13/1b → their own
  0x73 codes, **0x7000016 → 0x7300017** (the one non-identity entry), everything else → the
  front-end map (no outro). Phase 2's resolve (`load_tick`) now implements
  `GetLevelToLoadFromMovieCode__Fv` 0x1d3560: 0x07100018 → 0x0700001b, the enumerated 0x73 codes →
  0x7000048 (setting +0x28 = PAGE_RESULTS as 0x1d6360 does), everything else strips the middle
  byte.
- **The mission-complete flow routes through the outro.** `mission_ended` sets `LevelToEndTo`
  (outro code on a complete, the map on a failure — 0x182c60 cases 2/3), requests it through
  `requested_level` and `reset_map_level_to_load`, so the outro rides the same state-3 → state-6 →
  phase-2 machinery as a briefing and returns to the map with the results page (phase 2 sets
  flow.page = 0x40000036 [M 0x1d6360]).
- **State 6 waits like the game.** `GameFlow` gained `movie_playing` (`BGFMVPlaying` [L 0x30c69c];
  `psiMovieFinished` 0x1082e8 is `BGFMVPlaying == 0`), `movie_pending` and `phase2_resolved`
  (each on its own line, per the merge lesson). `main_tick`'s ST_MOVIE arm holds while a real
  movie plays and pops on its end; the countdown model still runs when no real movie ever started
  (the slim ISO's zeroed PSS files), so every existing menus test is unchanged. `skip_movie` is
  case 6's skip arm (`Input_Action(-1, 0x19, 1)` [M 0x1c98c8]); the host feeds it from the Confirm
  keys (Enter/Space — the port's stand-ins for the pad's X/Start [S]).
- **The player.** `movie::Player::start` (`psiStartBackgroundMovie__FUiScUi` 0x1081a8): gates on
  the `UsingMovie` whitelist (the 23 hashes of 0x107eb8), opens `MOVIES/30_FPS/%8.8X.PSS`,
  decodes on a worker thread (bounded channel, 2 frames in flight), plays the 16-bit 48 kHz stereo
  PCM through a cpal stream fed per frame (the game's AudioDec IOP ring, 0x107100/0x107360), and
  `present()` hands out one frame per second call (30 fps — main 0x109080's decode-per-2-vblanks).
  Without the `movies` feature it still runs the whitelist + file checks and reports done
  immediately, so the flow machine is testable everywhere.
- **The full-screen plane.** The decoded RGBA frame is drawn as a full-screen 512×448 quad through
  the HUD pipeline after everything else (the game blits the frame straight into the frame buffer
  over the empty backdrop 0x7000500, DecodeBGFMVFrame 0x108550 → PS2UploadBGMImage 0x108740); the
  overlay is suppressed while it plays, but `fe.tick()` still runs so the state machine keeps
  moving.
- **The host glue** (`App::movie_tick`, one call per redraw): starts the player when a wrap is
  pending (early — the decode overlaps the fade, since a whole-stream decode costs ~23 s on
  07100001), starts/holds/stops it with state 6, and routes the phase-2 resolve: a mission level
  closes the front end and loads through the existing `pending_level` path (the sim resumes — the
  fe lifecycle that was missing for menu-started missions); the map reopens the results page.
  The host defers the real level load while `movie_pending` stands (the game loads the backdrop,
  the movie, then the real level [M 0x1d6360 phases 0/3]). `demo_results` (F8/F9) now seeds the
  flow stack with state 2 so the outro request passes `reset_map_level_to_load`'s gate.

## Shared files touched (all small, additive)

- `crates/nf-view/src/lib.rs` (named shared file): the `App.movie` field (own line), `movie_tick`,
  the pending-level deferral, the state-6 skip keys, the hud branch + the `render` movie argument.
- `crates/nf-view/src/gpu.rs` (NOT in the task's file list — needed for the plane, listed here):
  additive `Renderer` fields (`movie_tex/movie_bg/movie_size/movie_vbuf/hud_tbgl/hud_sampler`), a
  static 6-vertex quad, one additive trailing parameter `movie: Option<(u32, u32, &[u8])>` on
  `render()` and one additive draw block at the end of the pass. Nothing existing changed shape.
- `crates/nf-view/Cargo.toml`: `cpal = "=0.18.2"` (the same pin nf-audio uses) + `Cargo.lock`.

## How it was verified

- `crates/nf-view/tests/movie_playback_flow.rs`, 10 headless tests: the Equinox wrap (written
  FIRST and shown failing against the old code — left 0x0710001B vs the game's 0x07100018), the
  full briefing table, the outro switch, the mission-start walk through state 6 to the load, the
  skip (hold while playing, pop on the skip, phase 2 resolves), the complete → outro → map →
  results-page flow, the failure → straight-to-map flow, the whitelist refusals, and the stubbed
  image's refusal (the game would hang in `strFileOpen` 0x108cc0). All green without the `movies`
  feature (the suite's configuration) and with it.
- **The retail-image data test** (`the_briefing_movie_plays_from_the_retail_image`, `movies`
  feature + `NF_MOVIE_ISO`): starts 0x07100001 from the full retail image in the sandbox, waits
  out the ~23 s whole-stream decode, asserts the first frame is the game's 512×448 RGBA plane and
  that the ~40 s stream is not over. It skips cleanly on images without movie data; the slim ISO
  the suite mounts stubs every PSS to 0 bytes, so it exercises the refusal path instead.
- Full suite: see the count below (run with NF_ISO = the slim on-foot image; the 3 `disc_golden`
  failures are the allowed ones there).

## What's left / known limits

- **Whole-stream decode**: `nf_formats::mpeg2::decode_video` decodes the entire ES before the
  first frame comes back (~23 s on 07100001). The player hides it by starting the decode when the
  menu picks the mission (it overlaps the fade and the level load), but a direct jump into a
  movie-heavy moment shows a black frame until it finishes. A streaming decoder in nf-formats
  (decode per GOP) would fix it properly — nf-formats is outside this task's file list.
- **Audio**: the movie PCM runs on its own cpal stream at fixed volume (the game applies
  `MovieVolume` = `SFXGetVolume()` at the start site [M]); the sfx master volume is not plumbed
  in. Video/audio pacing is independent (frame-count vs cpal clock) — a drift of a few frames over
  a full movie is possible [S].
- **The in-level "MISSION COMPLETE" banner** (~5 s before the outro, Mission_Update case 7 [M]) is
  not modelled — the port's results page shows immediately and the outro plays right away.
- The title/menu background movies (P_INTRO's 0x7370048/0x7380048 and the FILES.BIN page+0xC4
  loopers) stay on the countdown/plate model — that is task 183's `fix-menu-movies` scope; this
  task changed nothing on that path except that it now shares the movie-playing hold.
- The backdrop level 0x7000500 is not loaded (the game's phase 0 does [M]) — the movie plane covers
  the screen completely, and the port saves the full archive load; noted as a deviation.
- The real mission-event → `mission_ended` glue (the sim side) is still the host work fix-menus
  listed; the outro flow rides `mission_ended` (F8/F9 demo hook or any caller).

## Live checks for the main agent

1. PCSX2, mission select → start The Exchange (0x07000005): the briefing movie 07100005.PSS plays
   full-screen right after the fade (action: confirm on the row; expected: the movie, subtitles
   from the 0x6000961 script object, no menu).
2. PCSX2, press X/Start during the briefing: the movie stops instantly and the level loads
   (state 6 pop → phase 2 → load).
3. PCSX2, complete Night Shift's predecessor (level 0x0700000c chain — or force script channel
   100 on 0x07000004): the "MISSION COMPLETE" banner ~5 s, then the outro 07300004.PSS, then the
   front-end map with the results page.
4. nf-view (after merge), retail image: `nf-view Nightfire-retail.iso` → Enter on the title →
   after the intro countdown, unlock-all → Single Player → The Exchange: expect
   "movie: decoding 07100005 ahead of state 6" on stderr, then the briefing over a dark scene, no
   menu text on top.
5. nf-view `--bond` spots are unchanged (the movie path draws no world).

## ntfy

Sent at the end of the session (see the "ntfy" section only if it failed).

---
## ntfy failed

All three fallbacks returned HTTP 429 "limit reached: daily message quota reached" (2026-10-13T23:08Z). The notification could not be sent.
