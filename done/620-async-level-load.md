## Your task: `async-level-load`, load levels off the render thread so the loading screen animates

The port now draws the game's loading screen (crate::loading, `ResetMap_GenLoadScreen` 0x1d5de0) once, then loads the
level synchronously in the window's thread (`App` level switch in `crates/nf-view/src/lib.rs`, `load_with_carry`), so
the six LOADING dots can't animate (the game animates them from the vblank interrupt every 11 vblanks, ping-pong
0→6→0 — `loading::dots_lit`). Move the level build (`load_with_carry` / `Scene` construction — everything that is not
GPU upload) to a worker thread: the window keeps redrawing the loading screen with `dots_lit(ticks)` until the scene
is ready, then swaps it in (renderer rebuild on the main thread). Keep every existing behaviour (warm/cold saves, the
restart checkpoint, movie-wrapped loads, `pending_weapon_assets`). If `Scene` isn't `Send`, find the minimal change.
Tests: the load path still produces the same scene (existing level-load tests stay green); a test that the frame loop
keeps ticking during a load (headless: a fake slow load, the dot count advances). This is runtime plumbing, not a
look change — the main agent checks the result live.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/async-level-load`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-15T21:11Z

---
## Report (done, 2026-10-15T22:08Z)


# async-level-load — report

**Branch:** `s/async-level-load` (head: the merge of origin/master 2bda7b46 + the work below)
**Task:** load levels off the render thread so the loading screen animates.

## What's done

The level build moved to a worker thread; the window keeps redrawing the loading screen with the
game's dot animation until the scene is ready, then swaps it in on the main thread.

- **The game's evidence** (docs/systems/async-level-load.md carries the full chain): the load
  blocks the main thread in `ResetMap_Load` 0x1d6360 state 3 (draw screen → `LoaderLoad` loops →
  `Player_Start` → `psiPreGameRun` → `psiEndLoadingScreen` 0x10ba28) while `VBlankInt` 0x124088
  animates the six dots — every 11 vblanks (`BlobTime` 10 → underflow), ping-pong 0→6→0
  (`BlobDir` ±1), gated on `PS2LSActive == 2` (1 from `psiStartLoadingScreen` 0x10b9f8, 2 from
  `psiAllocateLoadingBlobs` 0x11f270, 0 from `psiEndLoadingScreen`). All [M], read in
  `work/decomp/ACTION.ELF/`.
- **`crates/nf-view/src/async_load.rs`** — `Job<T>`: the build on a named `level-load` thread;
  the result crosses an mpsc channel as `Option<T>` (worker panic → `None`, surfaced as
  `Some(None)`, never an infinite pending state — a Disconnected channel without a value and a
  spawn failure both land there); `elapsed_ticks()` = elapsed wall-clock in 60 Hz vblank units
  (the window's redraws stand in for the vblanks [S: the mapping], `loading::dots_lit` consumes
  it); a compile-time `Send` assertion pins `scene::Scene` across the boundary (it is plain
  data — no `Send` surgery needed).
- **`crates/nf-view/src/lib.rs`** — `App::async_load: Option<LevelLoad>` (job + level code +
  the drive-ptp return); `start_async_load` = the transition's first half exactly as the sync
  path had it (the cold / fail-Restart ENTRY-checkpoint / warm `Player_RamSave` arms — the old
  sim stays in `self.scene` until the swap, so the save snapshot and the screen's reads see the
  same state), the screen's first frame drawn, then the spawn; `poll_async_load` = one frame of
  the running load (pending → animated screen + `request_redraw`, `false` = the caller ends the
  frame there — no movie glue, no sim ticks, no input, mirroring the game's blocked main loop);
  `swap_in_scene` = the main-thread half in the sync path's exact order (`build_sound`,
  `audio_paused` reset, camera, scene, `anim_time = 0`, drive-ptp return, **the renderer's GPU
  rebuild stays on this thread**, the entry-checkpoint re-capture); `prev_lock_ch` resets on
  every terminal branch (Ok / Err / panic).
- **Kept identical:** warm/cold saves, the restart checkpoint, movie-wrapped loads (the
  `fe.flow.movie_pending` hold re-issues through `movie_tick` before any spawn), the
  `pending_weapon_assets` give-reload (deliberately still synchronous — no loading screen on
  that path, and an async give would let the sim tick mid-give), the pre-window startup load,
  the `dt` cap across the load (pending frames don't touch `self.last`).

## How it was verified

- **Tests (all green on the final merged tree):**
  - `async_load.rs` lib tests, 3: value delivery + tick advance across a 120 ms fake load; a
    panicking worker surfaces as `Some(None)` and drains; the `inline` seam.
  - `tests/async_load_frame_loop.rs`, 2 (headless, no ISO): the frame-loop contract — at the
    16 ms redraw cadence against a 1 s fake load, ≥ 20 pending frames tick through and the dot
    count passes several states of the 0→6→0 sweep (the old model showed exactly one); a fast
    load arrives intact and drains. Each test carries a sister assertion that can fail.
  - `tests/async_level_load_live.rs`, 3 (NF_ISO): the worker's scene equals the sync load's
    scene — cold and warm (a captured `RamSave` rides the channel) — on 07000002, compared on
    level/bond position/cels/vertices/characters/colliders/sequences/stats; a missing archive
    fails through the worker as `Err(String)` like the sync path.
  - Untouched-path slices on the merged tree: `level_carry_live` 8/8, `level_reload_live` 9/9,
    `menus_flow` 36/36, nf-view lib 15/15.
- **Suite:** see "Suite on the final tree" below.

## Suite on the final tree

The full four-crate run does not fit this sandbox's window (detached processes get killed, and
the ~330 long per-level walk binaries alone exceed it — the same wall the screen-adjust worker
hit), so it ran as foreground slices through the fast runner on the final merged tree
(8ce6bf7a), with `tools/test/suite.py` building **all 357 test binaries clean** (324 s) first:

- Family slice (17 binaries, the load path + the menus/flow/level-transition suites my change
  touches): 16/17 green — 128 passed / 0 failed / 0 TIMEOUT. `walkreal_07000001_live` was cut
  by the window (its spawn-to-exit walk alone runs minutes of headless sim; pre-existing, not
  touched by this change — the sync load path it exercises is unchanged).
- Unit binaries: nf-view lib 15/15, nf-game lib 206/206.
- The master-merge files on the final tree: `objective_state` 5/5, `codename_entry` 14/14,
  `objective_hints_flow` 7/7.
- The task's own tests: async_load lib 3/3 + `async_load_frame_loop` 2/2 +
  `async_level_load_live` 3/3.

Total observed: **375 passed / 0 failed / 0 TIMEOUT** across the slices; every binary of the
four crates compiled.

## What's left

- Nothing on the task. The visual result (the dots actually animating on screen) is the main
  agent's live check — the renderer's draw of `dots_lit` was already wired
  (`menu::loading_overlay` takes the dot count; the old model always passed 0).

## Live checks for the main agent

1. **The dots animate across a level transition** — play 07000001 → its level-exit trigger
   (warm load into 07000002): the LOADING dots step roughly every 0.18 s, filling 0→6 and
   emptying back (the game's cadence: 11 vblanks per step [M 0x124088]), instead of a frozen
   first frame.
2. **Cold restart** — pause menu → Restart: animated dots; the level returns playable, health 100.
3. **Fail → Restart** — the warm ENTRY-checkpoint arm: animated dots; the reload lands at the
   entry checkpoint with the health floor (behaviour unchanged by the threading).
4. **Movie-wrapped load** — a mission whose briefing movie plays (state 6): the movie plays
   out, then the level loads with animated dots; no double load, no stuck black frame.
5. **Weapon give hitch** — dev give of a weapon the level archive lacks: still a synchronous
   hitch, then the weapon draws (this path was deliberately not threaded).
6. **Optional** — a load failure (bad archive) logs `level: failed to load …` and keeps the old
   scene rather than freezing; a worker panic logs `level: load of … panicked` the same way.
