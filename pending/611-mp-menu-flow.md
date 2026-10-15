## Your task: `mp-menu-flow`, the multiplayer front-end flow and its data (no drawing)

Main menu → Multiplayer opens the game's MP setup pages; the port has none of them. The main agent draws the pages;
you port **the page flow, the lists and their state** into the front-end model (`crates/nf-view/src/menu.rs`
`FrontEnd`, `Page`, `action()`), headless-testable.

Handlers (ACTION.ELF; decomp export `work/decomp/ACTION.ELF/`; menu-def pages in 07000048.bin kind-8 entry
08000002, tag format in menu.rs's header; the language bank via `nf_formats::hud::Lang`):
- `C_GOMULTIPLAYER_Handler` 0x216a78 → `P_MPJOIN_Handler` 0x20ca00 (Join Game: four agent panels — "Press ✕ to join",
  Choose Codename with ◀ ▶, "Press ✕ when ready"; agents 2–4 show the insert-controller text), `C_RBMPSTART_Handler`
  0x20d048, `C_RBMPCNAME_Handler` 0x20d448.
- `P_MPSCENARIO_Handler` 0x217348 / `C_SBMPSCEN_Handler` 0x20c600 (Select Scenario wheel: Quick Game, Arena, Team Arena,
  Capture The Flag, Uplink, Top Agent, Demolition, …; per-row preview texture, description, locked state "This
  scenario is locked.").
- `P_MPMAP_Handler` 0x217468 / `C_SBMPMAP_Handler` 0x217538 (Select Map wheel: per-scenario map list, preview,
  description, locks).
- `P_MPSETUP_Handler` 0x20d7a8, `C_RBMPSETUP_Handler` 0x20de58, `P_MPOPTIONS_Handler` 0x217678, `C_SBMPOPTIONS_Handler`
  0x20e818, `P_MPRULES_Handler` 0x20f790, `P_MPENVIROMODS_Handler` 0x210030, `P_MPPLAYERMODS_Handler` 0x2104d0,
  `P_MPBOTS_Handler` 0x217790 / `C_SBBOTS_Handler` 0x210a08 / `P_MPBOTCHOOSE_Handler` 0x210cd8 /
  `P_MPBOTSETUP_Handler` 0x211270, `P_MPCONFIRM_Handler` 0x20ec00, `P_MPDEBRIEFING_Handler` 0x212198.

Deliver:
1. `docs/systems/mp-menus.md`: the page graph (page ids, which action goes where), each list's rows from the game's own
   tables (label ids, preview texture hashes, description ids, unlock conditions, the defaults), and what the final
   confirm hands to the game (the scenario/map/rules/bots values the MP level load reads — tie it to what the port's
   mp code in nf-game already consumes).
2. `Page` variants + `FrontEnd` state + `action()` handling for the flow, with getters the draw code can use (rows,
   preview hash, description id, locked flag per row). Keyboard only: one local agent (agent 1); agents 2–4 stay in
   their "insert controller" state. Starting the match should reach the existing MP level load if the port has one;
   otherwise stop at the confirm page and record the gap.
3. Tests: the flow from Main → Multiplayer → Join → Scenario → Map → Setup → back out; the lists match the tables
   (numbers only, ISO-gated); each with a sister assertion that can fail.

Not in scope: drawing (the main agent does), split-screen/multiple pads (gamepad work is lowest priority).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-menu-flow`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
