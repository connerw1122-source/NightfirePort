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

