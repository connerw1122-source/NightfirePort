**Base:** merge `origin/master` first and check it contains `crates/nf-view/tests/briefing_plays_live.rs` (the
front-end audit + test fixes landed with it, 2026-10-16 evening). If it does not, fetch again before you start: the old
master still carried 28 tests that contradicted the decompile.

## Your task: `fe-gaps-g048`, the front end's open gaps (docs/GAPS.md G-048)

The front-end audit left these unported; each is logic/state, not drawing:
1. `Menu_ChangePageCloseIris`: difficulty to missions (and its other callers) is itself a ~10-tick delayed page change
   with input blocked; the port changes the page at once. Decompile it, list every caller, port the delay through the
   same delayed-page path `C_GONIGHTFIRE` uses (`DELAYED_PAGE_TICKS`), and update the walks.
2. `Menu_EnableSounds` 0x1f31f0: the menu sound flags are set per page open (find every call site); the port uses one
   global 0x0b. Port the per-page flags and make the menu sfx (move/select/back) obey them; menu_sfx_flow.rs tests.
3. The completed-mission results page: the game opens no page until the load finishes (Mission_Update case 4 sets the
   page id; the manager is created by the load), but the port shows results over the still-loaded mission. Port the
   order. The MP debriefing over the arena has the same doubt: decompile the MP match-end to debriefing path and port it.
4. P_PARISENUM 0x20a648: the memory-card check completes on its first tick [S]. Port what the handler waits on (the
   card state machine HandlePS2CardState 0x110b98 as far as a no-card machine goes) so the box shows for the game's time.

Docs first (docs/systems/frontend.md), each change cited [M], tests with sisters. Remove the G-048 rows you close.

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

**Branch:** `s/fe-gaps-g048`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-17T06:27Z

---
## Report (done, 2026-10-17T08:19Z)

