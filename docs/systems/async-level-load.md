# Async level load (the animated loading screen)

The six LOADING dots must animate while a level builds. The game does this on one thread: the
main thread blocks inside the load while the **vblank interrupt** animates the dots. The port's
old model blocked the **window thread** in the load and drew the screen once, so the dots froze
on their first state. The port now splits the game's single-threaded load the same way it splits
the interrupt: the build runs on a worker thread and the window thread redraws the screen with
the dot count advanced every frame, swapping the scene in the frame it is ready.

## The game's load and its animation (evidence)

- `ResetMap_Load` 0x1d6360, state 3 [M 001d6360_ResetMap_Load__Fv.c:117-170]: draws the screen
  (`ResetMap_GenLoadScreen` 0x1d5de0, **skipped for the front end 0x07000048**), then runs the
  whole blocking build **on the calling thread** — `Mem_Init`, `hashtable_initialise`,
  `Sound_Kill`/`Sound_Init`, `ResetMap_GameInit`, the `LoaderLoad` 0x1d4a00 type-0 loop, then the
  type-1 loop, `build_MakeCelOverlapBox`, `build_link_objects_to_rooms`, `AIPath_BindNodes`,
  `Mem_Shrink`, `Player_Start` 0x190fd0, `MP_Start`, `Drone_PostLoad_Init`, `Cameras_PostLoad`,
  `psiPreGameRun` 0x11e2e8, `MenuManager_Create` (0x07000048 only) — and **only then**
  `psiEndLoadingScreen` 0x10ba28, which just clears `PS2LSActive = 0` [M 0010ba28].
- The dot animation runs while `PS2LSActive == 2`: `psiStartLoadingScreen` 0x10b9f8 sets it to 1
  [M 0010b9f8], `psiAllocateLoadingBlobs` 0x11f270 upgrades 1 → 2 [M 0011f270:10-20].
- `VBlankInt` 0x124088 [M 00124088_VBlankInt__Fi.c:18-37]: while `PS2LSActive == 2`,
  `BlobTime` counts down from 10; on underflow it resets to 10 and steps
  `BlobToDraw += BlobDir` — a step **every 11 vblanks** — with the ping-pong `BlobDir = 1` at
  `BlobToDraw == 0` and `BlobDir = -1` at `BlobToDraw == 6` (0→6→0 forever). The step index
  selects one of `LoadingBlobs[]`, DMA'd to the screen that vblank. The port's
  `loading::dots_lit(ticks)` reproduces exactly this arithmetic (`ticks / 11`, ping-pong 0→6→0).
- `CreateLoadingBlob` 0x11bf70 / `CacheLoadingBlobs` 0x11f2e8: the dot is 07000500.bin's
  16×16 texture 0x03000175 [M].

So during the load the game's main loop does nothing else — no menu manager ticks, no input —
while the interrupt alone moves the picture. The port's async model mirrors this split: while
the worker builds, the window thread's redraw runs **only** the animated screen (no movie glue,
no sim ticks, no input handling), and the swap frame falls through to normal play.

## The port's model

`crates/nf-view/src/async_load.rs` — `Job<T>`:
- `Job::spawn(run)`: the build on a named `level-load` worker thread; the result crosses on an
  `mpsc` channel as `Option<T>` — `None` = the worker panicked (`catch_unwind`; the default
  hook still prints). `Job::poll()` returns `None` (still running), `Some(None)` (worker death),
  or `Some(Some(value))`; after a terminal result the job is drained. A `Disconnected` channel
  without a queued value (the thread never started) also surfaces as `Some(None)` — never an
  infinite pending state.
- `Job::elapsed_ticks()`: the load's elapsed wall-clock in 60 Hz vblank units — the game counts
  the dots in vblanks, and 11 vblanks at 60 Hz is the cadence the window's redraws stand in for
  (an exact vblank count is not available to a winit loop; on a 60 Hz display the redraws are
  the vblanks) [S: the cadence mapping; the 11-vblank step itself is M above].
- A compile-time `Send` assertion pins that `scene::Scene` crosses the thread boundary (it is
  plain data: Vecs, HashMaps, the Sim).

`crates/nf-view/src/lib.rs`:
- `App::async_load: Option<LevelLoad>` (job + the transition's level code + the drive-ptp
  return captured from the old sim).
- `App::start_async_load(next)` — the transition's first half, called from `RedrawRequested`
  when `pending_level` fires and no movie is pending (the movie-wrapped hold is unchanged):
  captures the carry-over exactly as the synchronous path did (the cold/restart/warm arms —
  `level_exit_cold`, `fail_restart_save` on the ENTRY checkpoint, the `Player_RamSave`
  capture), draws the screen's first frame (0 dots), and spawns the worker running
  `load_with_carry` / `load_with_carry_dev` (`--dev`).
- `App::poll_async_load() — one frame of the running load: pending → redraw the screen with
  `loading::dots_lit(job.elapsed_ticks())` and request the next redraw, `false` (the caller's
  redraw ends here); ready → `swap_in_scene` + `true`.
- `App::swap_in_scene(scene, ptp_return)` — the main-thread half, in the synchronous path's
  exact order: `build_sound` (the audio stack is built here, not on the worker — it consumes
  the scene's banks and opens the output), `audio_paused = false`, the camera from the level's
  start placement, `self.scene = scene`, `anim_time = 0.0`, the drive-ptp return applied
  (`Boot_LoadPTPData` 0x1c8560's mirror), the renderer rebuild — **GPU upload stays on the main
  thread**, which owns the wgpu device/queue — and the ENTRY checkpoint re-capture
  (`entry_checkpoint_save`). `prev_lock_ch` resets on every terminal branch (Ok / Err / panic),
  as the sync path did after its match.

Behaviour kept identical: warm/cold saves and the restart checkpoint (the save snapshot is taken
from the old sim before the spawn, which stays in `self.scene` until the swap — the worker and
the screen's font/picture reads see the same state the sync path saw); movie-wrapped loads (the
`fe.flow.movie_pending` hold re-issues through `movie_tick` before any spawn); the
`pending_weapon_assets` give-reload (unchanged, synchronous: it has no loading screen, and an
async give would let the sim keep ticking mid-give — a behaviour change); the startup load in
`run()` (before the window exists); `dt`/tick behaviour across the load (the pending frames do
not touch `self.last`, so the swap frame's `dt` spans the load and hits the same 0.1 s cap the
sync path hit).

Determinism: the worker runs the same `load_impl` with the same inputs — no RNG draws added,
no wall-clock inside the Sim (the loading screen's hint seed is the pre-existing `SystemTime`
pick, outside the Sim).

## Tests

- `crates/nf-view/src/async_load.rs` `#[cfg(test)]` (headless): value delivery + tick advance
  across a 120 ms fake load; a panicking worker surfaces as `Some(None)` and the job drains;
  the `inline` seam polls like a finished worker.
- `crates/nf-view/tests/async_load_frame_loop.rs` (headless): the frame-loop contract — at the
  16 ms redraw cadence against a 1 s fake load, ≥ 20 pending frames tick through and the dot
  count passes several states of the 0→6→0 sweep (the old model showed one state); a fast load
  arrives intact and drains.
- `crates/nf-view/tests/async_level_load_live.rs` (NF_ISO): the worker's scene equals the sync
  load's scene — cold and warm (a captured `RamSave` rides the channel) — on level 07000002,
  compared on level/bond position/cels/vertices/characters/colliders/sequences/stats; a missing
  archive fails through the worker as an `Err(String)` like the sync path.
- Existing level-load tests (`level_carry_live`, `level_reload_live`, `walkreal_*`) cover the
  sync path, which is untouched.

## Live checks for the main agent

1. **The dots animate across a level transition.** Play 07000001 → exit through the mission's
   level-exit trigger (warm load into 07000002): the LOADING screen must show the six dots
   stepping roughly every 0.18 s, filling 0→6 and emptying back — not a frozen first frame.
   (Old model: dots static; new model: animated. `BlobToDraw`'s exact cadence is 11 vblanks
   [M 0x124088].)
2. **Cold restart.** Pause menu → Restart on any mission (the cold arm to BaseMap): the dots
   must animate the same way, and the level must come back playable with health 100.
3. **Fail → Restart.** Die, choose Restart from the fail menu (the warm ENTRY-checkpoint arm):
   the dots animate; the reload lands at the entry checkpoint with the health floor — the
   restart behaviour must be unchanged by the threading.
4. **Movie-wrapped load.** From the front end, launch a mission whose briefing movie plays
   (state 6): the movie plays out, then the level loads with animated dots — no double load,
   no stuck black screen.
5. **Weapon give hitch (unchanged).** Dev give of a weapon the level archive lacks
   (`pending_weapon_assets`): still a synchronous hitch, then the weapon draws — this path was
   deliberately not threaded.
6. **Load failure path.** (Optional, hard to trigger live:) a bad archive logs
   `level: failed to load …` and keeps the old scene rather than freezing.
