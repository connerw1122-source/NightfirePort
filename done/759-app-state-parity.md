**Base:** merge `origin/master` first and check it contains `crates/nf-view/tests/briefing_plays_live.rs` (the
front-end audit + test fixes landed with it, 2026-10-16 evening). If it does not, fetch again before you start: the old
master still carried 28 tests that contradicted the decompile.

## Your task: `app-state-parity`, App fields that only the command line sets

`App::level_code` was initialised from the command-line level and never updated by a load, so every menu-started
mission read as the front end (fixed 2026-10-16: `poll_async_load` sets it). Audit the rest of `App` (crates/nf-view/
src/lib.rs, the struct and `run()`'s initialiser) for the same class of bug: every field initialised from args or at
startup that the GAME updates per load or per menu choice (difficulty / `selected_difficulty`, the mission row,
ptp/entry save, `all`, cheats, the sim's settings, the sound bank, the language). For each: cite where the game keeps it
(GameState offsets, [M] addresses), make the port update it on the same event, and pin it with a test (use the
headless harness if `headless-app-harness` has landed; otherwise a focused unit on the extracted update function).
Report a table: field, game source, was it stale, fix.

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

**Branch:** `s/app-state-parity`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-17T06:44Z

---
## Report (done, 2026-10-17T07:48Z)

