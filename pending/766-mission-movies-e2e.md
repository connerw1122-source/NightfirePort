**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `mission-movies-e2e`, every mission's briefing and outro through the window's own path

The briefing for The Exchange stopped playing twice on 2026-10-16 (the user: "cutscene didnt play when I
started the exchange, it went straight to the loading screen then to game"). The second time, the
gameflow-state-stack merge's 84-frame fade ran ahead of ST_MAPLOAD and the decode-ahead only started in
ST_MAPLOAD (fixed on master by starting it whenever a briefing is pending). Only one mission was tested.

Extend `crates/nf-view/tests/briefing_plays_live.rs` (build with `--features movies`; the suite now does)
to every single-player mission row: from the front end through `nf_view::mission_movie_glue` with the
window's worst-case clock (up to 8 front-end ticks per redraw), assert the mission's briefing movie starts,
state 6 holds while it plays, and the level loads after it. Then do the same for the outro: complete or
fail the mission through the results path and assert the outro movie plays where the game plays one
(the case-6 level -> outro-code switch [M 0x182e60-0x182eb4]). Missions whose code is not in `USING_MOVIE`
must take the game's no-movie path; list which those are. Fix every row that fails.

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

**Branch:** `s/mission-movies-e2e`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
