## Your task: `frontend-music`, the music the menus play

Port which music the front end plays (title, main menu, the list pages, the results page, multiplayer setup) and over
the pause page: the track ids, when they start/stop/crossfade (the page changes, the movie backdrops' own audio vs
the music — docs/systems/music.md has the in-game system), and how a mission end hands over to the front end's music.
Wire it into the host so the menus play it (silent under NF_SILENT). Tests: the page → track mapping and the
transitions (numbers only).

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/frontend-music`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-15T21:47Z

---
## Report (done, 2026-10-16T00:55Z)

