## Your task: `headless-app-harness-2`, re-land the headless app harness on the newest master (HELD at merge)

`s/headless-app-harness` (task 758) was held at the 2026-10-17 batch: it moves `RedrawRequested`'s
non-GPU half into `App::frame` and the constructor into `App::bare`, but master changed the same
`crates/nf-view/src/lib.rs` regions since the branch was cut (the window event loop: the rapid-fire input,
the movie glue/timing, the rocket and view-model work, the profile session field), so the merge conflicts
across ~370 lines. Rebase the refactor onto the newest origin/master by hand: carry every change master made
to the moved code into `frame`/`draw_frame`/`bare` (RAPID_FIRE, `sim.rapid_fire`, the movie timing, the
profile_session field, everything else in that diff). Behaviour must not change: the full suite green, and
the headless harness tests from the branch passing.

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

**Branch:** `s/headless-app-harness-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
