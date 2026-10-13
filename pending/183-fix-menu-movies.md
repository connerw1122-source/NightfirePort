## Your task: `menu-movies`, the front end's background movies and the intro

The movie decoder is merged (`--features movies`, `crates/nf-view/src/movie.rs`, `docs/systems/movies.md`); the menus are
built from the disc's page data (fix-menus2/3). In the real game the main menu's backdrop is a LOOPING FMV (the PAGE
record's aux code 0x07330048; the list pages 0x07350048), and Start on the title plays the intro movie before the main menu
(P_INTRO 0x209e88: 0x07370048/0x07380048) [docs/reports/fix-menus3.md]. Wire them:
1. With the `movies` feature on, the main menu and list pages draw their looping background movie under the page
   controls; without the feature (or on the slim ISO, which zeroes the PSS files) the current dark plate stays.
2. Title -> Start plays the intro movie, skippable (action 0x19 / Enter), then the main menu.
3. The movie audio goes through the existing cpal output.
Use the full retail image from your sandbox storage for the tests (the slim ISO stubs every movie). Tests: the page ->
movie code mapping from the page data, and a decoded frame of the menu loop (SHA-1, numbers only). List
`--shot --page main` commands for the main agent.

Files: `crates/nf-view/src/{menu,movie,lib}.rs`.

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

**Branch:** `s/menu-movies`.
