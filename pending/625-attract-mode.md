## Your task: `attract-mode`, what the title does when left alone

`P_ATTRACT_Handler` 0x20a1f0 and the title's no-input counter (`Menu_ResetNoInputCount`, `FrontEnd::no_input_frames`
already counts): after the timeout the game plays an attract sequence (a demo movie or a recorded play?). Port the
timeout value, what plays (movie codes / demo data), how input interrupts it, and the return to the title. Flow +
state on `FrontEnd`, the host starts the movie through the existing movie player. Tests: the timeout fires at the
game's frame count; any input returns to the title.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/attract-mode`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
