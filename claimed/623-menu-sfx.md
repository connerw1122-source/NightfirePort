## Your task: `menu-sfx`, the front end's and pause page's sounds

The menus make sounds in the real game (cursor move, select, back, value change, page open/close, the error buzz on a
locked row). Port them: find the calls in the menu engine (`Menu_InputAction` 0x1f1f40, `Manager_SendMessage`
0x21c858, the wheel/list/spinner handlers, `C_GCPAUSE_Handler` 0x204408) — which SFX id / bank each plays and when —
and play them through the port's sound stack (nf-audio; the menu runs in the front end level 07000048 and over a paused
mission, check which bank is loaded there and how the game reaches the sounds). Hook points: `FrontEnd::action()`
returns whether the input was handled — add the sound event list the host plays. Tests: each action on each page emits
the game's sound id (numbers only); tests stay silent (NF_SILENT).

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/menu-sfx`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-15T21:28Z
