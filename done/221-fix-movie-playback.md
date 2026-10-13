## Your task: `movie-playback`, the briefing and outro movies in the mission flow

The movie decoder is merged (`--features movies`); `docs/systems/movies.md` documents where the game plays them: the
briefing 0x071000xx via `ResetMap_LevelToLoad` 0x1d3800 + `ResetMap_Load` 0x1d6360 (GameFlow states 3 -> 6), the outros
0x073000xx via `Mission_Update` 0x182c60, the win movie 0x73F0048. fix-menus models state 6 as a countdown stand-in.
Play the real movie in that state (a full-screen 512x448 plane, 30 fps, PCM audio through cpal, skip on action 0x19 /
Enter) for a menu-started mission and the mission-complete flow. Use the full retail image from your sandbox storage.
Tests: which movie code plays for each mission start and end (from the decompile's tables), the skip, and the return to
the level load.

Files: `crates/nf-view/src/{movie,menu,lib}.rs`.

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

**Branch:** `s/movie-playback`.

---
claimed-by: sandbox4 at 2026-10-13T21:51Z

---
## Report (done, 2026-10-13T22:58Z)

Closed by the driver: the report is on `s/fix-movie-playback` (docs/reports/fix-movie-playback.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-13T23:06Z)

