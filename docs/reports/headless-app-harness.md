# headless-app-harness — report

Branch `s/headless-app-harness` (from `origin/master`, which contains
`crates/nf-view/tests/briefing_plays_live.rs` — the required base check).

## What's done

The window host `App` (`crates/nf-view/src/lib.rs`) is testable without a window or GPU:

- **`App::frame(&mut self, dt: f32)`** — the whole non-draw half of the old inline
  `RedrawRequested` body: `movie_tick`, `menu_music_tick`, the pending-weapon reload,
  the level-load schedule + `poll_async_load` (with the net poll through the loading
  screen), the sound-hold diff, the 60 Hz sim tick gate (the lockstep gate, the
  `ticks_for_frame` scheduler), the mission-end / MP-debrief handovers — plus the
  front end's 60 Hz clock (`fe_clock`, ≤ 8 `fe.tick()`s a frame), moved here from the
  draw half, which consumed the same post-tick state. During an async load the frame
  returns early exactly as before, so the fe clock does not run behind the loading
  screen (the same rule the key-swallow follows).
- **`App::draw_frame(&mut self, dt: f32)`** — the GPU half, the old drawing path
  unchanged (no drawing changes; the render path stays as is).
- **`App::app_key(&mut self, k: KeyCode, state: ElementState, repeat: bool) -> bool`**
  — the whole `KeyboardInput` arm (cutscene skips, F12, the front-end typing pages,
  `menu_input`, the dev-menu keys, the mission Esc/pause arm). It returns `true` only
  where the old code called `el.exit()` (the bare viewer's un-grabbed Esc); the window
  event loop turns that into `el.exit()`. No winit/wgpu objects beyond the plain
  `KeyCode`/`ElementState` data.
- **`App::bare(iso, scene, cam, opts, choice, anim_time)`** — the neutral App skeleton
  (`gfx: None`, silent `sound: None`, no dev/panel/net); `run()` builds it and applies
  its CLI knobs, the harness builds the same skeleton — a headless frame runs the exact
  per-frame path the window runs.
- **`App::movie_plane_up()`** — the movie-plane decision, shared so the harness pumps
  `movie::present()` on exactly the frames the window's draw would.
- **`HeadlessApp`** (cfg(test) in lib.rs): `frame(dt)`, `key(KeyCode)`,
  `frames(n)`, `pump(wall_secs, pred)`, `exit_requested`; `boot()` (the front-end map +
  boot intro + strings, the window's UNLOCK_ALL default) and `mission(iso, level)` (the
  command-line shape).

One deliberate behaviour note [S]: a **minimized** window now ticks the front-end clock
(the fe tick moved from the draw half into `frame`, which runs before the minimized
early return). The game's menu timers run off the vblank regardless of present state,
so this matches the game better; nothing else changed.

## The tests (NF_ISO-gated, silent; `cargo test --release -p nf-view --lib -- --test-threads=1`)

1. **boot → The Exchange** (`boot_to_the_exchange_briefing_holds_then_the_level_loads`):
   a fresh boot walked through the real menus with keys only (title → the intro run →
   Main → Night Fire → codename/play-without-saving → difficulty → mission list →
   The Exchange), the pick wraps to the briefing `0x07100005` [M 0x1d3800's 0x71 wrap],
   state 6 holds while the movie plays [M 0x1c98c8 case 6], the skip resolves phase 2
   [M 0x1d3560], the level loads, `level_code == 0x07000005` [M ResetMap_Load phase 2,
   GameState+0xc]. The state-6 hold count runs only with `--features movies` **and** a
   movie-bearing image: the on-foot ISO's `/MOVIES/30_FPS/*.PSS` are zero-length stubs
   [D — `Player::start` → `NoMovieData`], where the port's headless countdown pops
   state 6 on its first tick [S the documented stand-in; `briefing_plays_live` pins the
   hold itself on a retail image].
2. **Esc → pause → Esc → resume** (`esc_in_a_mission_pauses_then_resumes_and_never_quits`):
   the pause page opens, the second Esc resumes, the sim keeps ticking, and
   `exit_requested` stays false throughout — the old behaviour quit the game because
   `level_code` was only ever set from the command line.
3. **Pause > QUIT → YES** (`pause_quit_returns_to_the_mission_list`): the front-end map
   loads, the queued page reopens on the mission list [M C_GCPAUSE_Handler
   0x204408:141-152 — +0x1c = 0x4000001c + ResetMap_LevelToLoad(0x7000048) + the fade;
   the load's queued-page create 0x1d6360:160-163], and the running level is the
   front-end map.
4. **A 6 s load keeps the online session polled**
   (`a_level_load_keeps_the_online_session_polled`): a real loopback host+client
   (`mpnet::SessionProxy`, the same cores the lobby pages wrap) joins, then a fake load
   (worker sleeps 6 s, past nf-net's `timeout_ms` 5000 [M nf-net session.rs Config])
   runs on the App; every load frame polls the session; after the swap the peer is
   still adopted, both cores open, no ClientLeft / HostTimeout / Desync anywhere.

Each carries its sister: the `exit_requested` assertions fail on the old `el.exit()`
shape; the state-6 hold count fails on the pre-a2cdd771 pop-on-first-tick shape; and
**demonstrated**: with the async-load net poll suppressed (`if false && …`), test 4
fails at the session assertions (the 5 s timeout fires); restored, it passes.

## How it was verified

- `cargo check --offline -p nf-view --tests` clean.
- `NF_SILENT=1 NF_ISO=<on-foot iso> cargo test --offline --release -p nf-view --lib --
  --test-threads=1` → **22 passed, 0 failed** (the 4 new + all pre-existing lib tests).
- Sister demo (above): poll suppressed → test 4 FAILED at `the session was dropped…`;
  poll restored → 22/22 again.
- The other three crates are untouched by this diff (lib.rs only + docs), so their
  suites are unchanged by it.

**The fast full suite did not run in this sandbox**: background processes are reaped
between tool calls here (the suite died three ways: plain nohup, setsid, and no tmux
binary — the pause-music-1b/zoe-gondola-08 precedent), and a foreground full run exceeds
the 10-minute tool cap. What ran instead: the full `nf-view` lib binary above, which is
the only crate the diff touches.

## What's left

- Nothing task-blocking. Follow-ups the main agent may want:
  - the minimized-window fe-tick note above, if the main agent wants the old
    draw-half timing kept exactly;
  - more flows can now ride `HeadlessApp` (the MP confirm handover, the fail menu's
    Restart) without their lib.rs mirrors — `pause_results_flows_live` could be
    migrated later;
  - a movie-bearing retail image run of the movies-gated assertions (they are inert on
    the slim on-foot ISO).

## Live checks for the main agent

None needed — this task is host glue, not a game-data field. The one thing a render
check could confirm (the same state the tests already pin): Esc in a menu-started
mission opens the pause page (fe page Pause) instead of quitting.

## Usage

Token / context / cost figures for this session: not visible to me — the environment
shows no token, credit or context-window counters anywhere I can read (no status output
from the driver, no usage fields in tool results). Written as required rather than
estimated.
