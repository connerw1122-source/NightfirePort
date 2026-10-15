## Your task: `subtitles-data`, cut-scene and movie subtitles

AV Options has a Subtitles On/Off row (profile byte read back by `P_CNAVOPTIONS_Handler` 0x214818 via control
0x10000136 → DAT_002a37c8). Port what it switches: where the game stores subtitle lines for the in-engine cut-scenes
(NIS / script players — docs/systems/script-player.md) and the FMV briefings, their timing (start/end ticks or frames),
the label ids, and who draws them (the function, the font, the position/colour it sets). Deliver the data model and a
per-tick query (`active_subtitle(t) -> Option<label>`) for both kinds, wired to the profile flag; the main agent
draws them. Tests: a known cut-scene yields the right line at the right tick; Off yields none.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/subtitles-data`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
