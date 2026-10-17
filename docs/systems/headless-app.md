# The headless App harness (`headless-app-harness`)

The window host `App` (`crates/nf-view/src/lib.rs`) owned the three 2026-10-16 playtest
bugs — the state-6 briefing popped before it showed, Esc in a menu-started mission quit
the game (`level_code` was only ever set from the command line), and the online session
went unpolled through the loading screen. None of that code was reachable from a test:
it lived inline in `ApplicationHandler::window_event`'s `RedrawRequested` and
`KeyboardInput` arms, which need a window, a GPU and a winit event loop.

## The split

`App`'s per-frame work is now two methods, called by the window's
`WindowEvent::RedrawRequested` arm in this order:

1. **`App::frame(&mut self, dt: f32)`** — everything that needs no window or GPU:
   - `movie_tick` (the ONE movie glue: the state-6 briefing/outro via
     `mission_movie_glue`, the intro run's players, the page aux loop, the phase-2
     resolve [M `psiStartBackgroundMovie` 0x1081a8, `psiMovieFinished` 0x1082e8,
     ResetMap_Load 0x1d6360 LoadState 2]);
   - `menu_music_tick` (the page-change transitions + the driver drain [M
     0x21c858:345-346, 0x20a408]);
   - the pending-weapon-asset warm reload;
   - the level-load schedule (`pending_level` → `start_async_load`) and the async poll
     (`poll_async_load`, the `LOADING_HOLD` window);
   - the net poll and its `CoreEvent` handovers — **including through the loading
     screen** (the async-load early return polls the session first);
   - the sound-hold diff (`audio_paused`, the pause page's `SFXPause` pair [M
     0x21b908:121-124, 0x21c2b0:11-22]);
   - the 60 Hz sim tick gate (`!paused && fe.is_none()`, the lockstep gate, the
     `ticks_for_frame` scheduler [G-043]);
   - the mission-end / MP-debriefing handovers;
   - the front end's own 60 Hz clock (`fe_clock`, at most 8 `fe.tick()`s per frame [M
     `MenuManager_Update` 0x21c3d8's vblank pacing]) — moved here from the draw half,
     which consumed the same post-tick state.
2. **`App::draw_frame(&mut self, dt: f32)`** — the GPU half, byte-for-byte the old
   drawing path: interpolation palettes, the surface, the HUD/menu/movie overlay, the
   render, present, the fps cap. No game logic.

The key handling is **`App::app_key(&mut self, k: KeyCode, state: ElementState,
repeat: bool) -> bool`** — the whole old `KeyboardInput` arm: the cutscene-skip keys,
the F12 shot, the front-end typing pages, `menu_input`, the dev-menu keys, and the
mission Esc arm (pause open [S the game's Start pause; the gates M Player_Update
0x193600:72]). It returns `true` only where the old code called `el.exit()` — the bare
viewer's un-grabbed Esc — and the window event loop turns that into `el.exit()`. No
winit/wgpu objects beyond the plain `KeyCode`/`ElementState` data.

Two small extractions serve both callers:

- **`App::bare(iso, scene, cam, opts, choice, anim_time) -> App`** — every field at its
  neutral value (`gfx: None`, `sound: None` = silent, no dev/panel/net). `run()` and the
  harness build the SAME skeleton and apply their knobs on top, so a headless frame runs
  the exact per-frame path the window runs.
- **`App::movie_plane_up(&self) -> bool`** — the movie-plane decision (intro / state-6 /
  aux / attract), shared with the harness so it pumps `movie::present()` on exactly the
  frames the window's draw would [M the draw half's `movie_up` reads].

## The harness

`mod headless_app_harness` (cfg(test), `crates/nf-view/src/lib.rs`) defines
**`HeadlessApp`**: an `App` with `gfx: None` and a silent sound stack,
`level_code`/front end set the way `run()` does, exposing `frame(dt)` (frame + the movie
pump), `key(KeyCode)` (one edge-triggered press), `frames(n)`, `pump(wall_secs, pred)`
and `exit_requested`. The load hold (`Job::elapsed_ticks`) and the movie clocks
(`shown_for`) are wall-clock, so their waits are real seconds.

The tests (NF_ISO-gated, silent; run `cargo test --release -p nf-view --lib --
--test-threads=1`):

| test | pins | evidence |
|------|------|----------|
| `boot_to_the_exchange_briefing_holds_then_the_level_loads` | fresh boot walked through the real menus (keys only) to The Exchange; the pick wraps to the briefing 0x07100005; state 6 holds while the movie plays; the level loads after the skip; `level_code == 0x07000005` | [M 0x1d3800's 0x71 wrap, 0x1c98c8 case 6, 0x1d3560, ResetMap_Load phase 2]; the state-6 hold assertions run only with `--features movies` on a movie-bearing image — the on-foot ISO's PSS files are zero-length stubs [D `/MOVIES/30_FPS`], where the port's headless countdown stands in [S] |
| `esc_in_a_mission_pauses_then_resumes_and_never_quits` | Esc opens the pause page, Esc resumes, `exit_requested` stays false | [S the Start pause; M the game's gates 0x193600:72; the old bug: `level_code` never set from the menu path] |
| `pause_quit_returns_to_the_mission_list` | QUIT → YES loads the front-end map and reopens the mission list | [M C_GCPAUSE_Handler 0x204408:141-152; the load's queued-page create 0x1d6360:160-163] |
| `a_level_load_keeps_the_online_session_polled` | a fake 6 s load (real loopback host+client) does not drop the peer | [M nf-net `Config.timeout_ms` 5000; the frame's async-load poll] |

## Sisters (fail on the old behaviour)

- The Esc tests' `exit_requested` assertions: the old `el.exit()` arm fires when the
  host does not know the mission — exactly the pre-fix shape.
- The state-6 hold count (movies build): the pre-a2cdd771 shape popped state 6 on its
  first tick (briefing_plays_live's bisect).
- Demonstrated: with the async-load net poll suppressed (`if false && …`), the 6 s load
  test FAILS at the session assertions (the client times the host out / the host drops
  the client — the 5 s timeout); restored, it passes.
