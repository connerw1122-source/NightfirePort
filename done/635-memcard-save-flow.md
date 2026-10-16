## Your task: `memcard-save-flow`, when and how the game saves

Port the save flow around the profile (`nf_game::profile`, docs/reports/fix-save-profile.md): when the game saves
(after a mission's results commit, on option changes, on codename create/delete — `LS_Save` callers,
`P_PS2MEMCARDINIT_Handler` 0x218538, `C_LBERROPTIONS_Handler` 0x2185e8, `Menu_UpdateMessageBox` 0x1f7de8's
cases), the messages it shows ("Saving…", the card-full / no-card errors — which apply to a PC disk save and
which don't), and the autosave behaviour. Map the memory-card states to the port's file save honestly (no fake card
errors; document which game messages have no PC equivalent). State + messages on `FrontEnd`, the host writes the
file. Tests: each save trigger writes once with the right contents; a write failure surfaces the game's error page.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/memcard-save-flow`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T00:57Z

---
## Report (done, 2026-10-16T02:06Z)

Closed by the driver: the report is on `s/memcard-save-flow` (docs/reports/memcard-save-flow.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T02:08Z)

