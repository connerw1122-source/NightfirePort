**Base:** merge `origin/master` first and check it contains `crates/nf-view/tests/briefing_plays_live.rs` (the
front-end audit + test fixes landed with it, 2026-10-16 evening). If it does not, fetch again before you start: the old
master still carried 28 tests that contradicted the decompile.

## Your task: `headless-app-harness`, run the window's App logic in tests

Three playtest bugs on 2026-10-16 lived in `crates/nf-view/src/lib.rs`'s `App` (the window host), which no test runs:
the state-6 briefing popped before it showed (fixed by moving the glue into `nf_view::mission_movie_glue`), Esc in a
menu-started mission quit the game (`App::level_code` was only set from the command line), and the online session
went unpolled through the loading screen. Make the App testable without a window or GPU:
1. Split `App`'s per-frame logic (RedrawRequested: `movie_tick`, `menu_music_tick`, the async load, the net poll, the
   front-end clock, the sim tick gate) and the key handling (`WindowEvent::KeyboardInput` arms: Esc/pause, menu keys)
   into methods that take no winit/wgpu objects; the window event loop calls them. A `HeadlessApp` (pub, tests only or
   behind a cfg) builds an App with `gfx: None`, `sound` silent, and exposes `frame(dt)` and `key(KeyCode)`.
2. Tests (NF_ISO-gated, silent): (a) a fresh boot walked through the real menus to The Exchange: the briefing state
   holds while the movie plays, the level loads after it, `level_code` is 0x07000005; (b) Esc in that mission opens the
   pause page (fe page Pause), Esc again resumes, the process never requests exit; (c) Pause > Quit returns to the
   mission list [M 0x204408:141-152]; (d) a level load keeps an online session polled (a fake 6 s load must not drop the
   peer). Each with a sister that fails on the old behaviour.
No drawing changes; the render path stays as is.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/headless-app-harness`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-17T06:27Z

---
## Report (done, 2026-10-17T08:06Z)

