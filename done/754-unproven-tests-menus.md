**Base:** merge `origin/master` first and check it contains `crates/nf-view/tests/briefing_plays_live.rs` (the
front-end audit + test fixes landed with it, 2026-10-16 evening). If it does not, fetch again before you start: the old
master still carried 28 tests that contradicted the decompile.

## Your task: `unproven-tests-menus`, settle the front end's unproven assertions

For every row below: find the evidence (a decompile address that says it, disc bytes, or a fresh-boot PCSX2
measurement documented in the report) and tag the assertion, OR mark it `// [S] unverified` and take it out of
pass/fail. A row whose decompile reading contradicts the test: fix the test, cite the address.

Rows (file:line as of the 2026-10-16 audit; lines may have moved, find them by the asserted behaviour):
- menus_flow.rs ~40-42: "the driver's observed page order [L PCSX2 2026-10-13]" is not a fresh boot; re-derive it from the page handlers.
- menu_movies.rs ~205: ESTHERO_TICKS = 1756x2 is only compared with itself; add a disc sister that counts 0x073B0048's pictures (like the INTRO_MOVIES test).
- menus_flow.rs ~63-73: the confirm box's triangle cancels / cross continues [L driver transcript]; decode Menu_UpdateOptionBox.
- menus_flow.rs ~260-263, menu_sp_e2e.rs ~309-310, menus_mouse_parity.rs ~305: Main Up/Down wraps, Main cursor 2 on return; the control type of 0x100000ed (Scroll_Init's wrap flag) and P_MAIN's 0x4c.
- menus_flow.rs ~264-297, menu_sp_e2e.rs ~244-245: the Back targets (Main to title, Dossier to MissionSelect, MissionSelect to Difficulty).
- menus_flow.rs ~279-281, menus_mouse_parity.rs ~164/611, menu_sp_e2e.rs ~331-361: the difficulty +1 offset and all 3 tiers confirmable on a fresh profile; read the table at ACTION.ELF .data 0x2df4c0 [D].
- menus_e2e_mp.rs ~374, mp_menu_flow.rs ~76-77: Scenario Back to Main on cursor 1.
- menus_e2e_mp.rs ~244-303: left/right wraps at list ends; the list control's left/right code (0x2104d0, 0x210030).
- menus_e2e_mp.rs ~162, mp_menu_flow.rs ~22-27: Quick Game always Skyrail; the game uses Rand_Random()%7, so label the port's choice [S].
- mp_debriefing_flow.rs ~109: C_MPDBG 0x5d = Extra, but 0x5d is Side elsewhere; settle what action 0x20 is on the pad (docs/UNKNOWNS U-011).
- front_end_chain_live.rs ~171: Results Confirm to 0x4000001c holds only when c2c==0, just_completed==0 and no DoneBetter box (0x208e78:29-46); test the other branches.
- unlock_popups.rs ~92-122: the cheat-medal row's value and wrap; P_CHEATMEDAL 0x2086b0:19-21 switches on the 0x35 VALUE, so read the menu-def values.
- menu_sp_e2e.rs ~536-548: fail REPLAY = warm restart; read the REPLAY row's menu-def value (0 = checkpoint warm, 1 = base map cold).
- pause_results_flows_live.rs ~122-130: Esc closes from any tab; MenuManager_Monitor 0x21d508:35-38 needs manager+0x7b==7.
- pause_results_flows_live.rs ~217-220: after NO, one Confirm re-arms RESTART; state_ignore_167 (0x204408:165-170) swallows one 0x4b.
- pause_results_flows_live.rs ~508: LOADING_HOLD 5 s; the game has no minimum (0x1d6360:170), so label it a port choice.
- menu_audio_flows.rs ~269-283 and lib.rs's 1 s skip gate on state-6 briefings: GameFlow case 6 skips at once (0x1c98c8:146-150); keep the port's gate but tag it [S].

Report a table: row, evidence found, what changed (tagged / [S] / fixed).

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

**Branch:** `s/unproven-tests-menus`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-17T05:19Z

---
## Report (done, 2026-10-17T07:07Z)


---
## Report from the worker (done, 2026-10-17T07:08Z)

Closed by the driver: the report is on `s/unproven-tests-menus` (docs/reports/unproven-tests-menus.md); the worker didn't run finish.sh.
