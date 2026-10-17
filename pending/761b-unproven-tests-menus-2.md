## Your task: `unproven-tests-menus-2`, finish unproven-tests-menus (HELD at merge)

`s/unproven-tests-menus` (task 754) was held at the 2026-10-17 batch: merged onto master with the batch
(laser-scripts, start-slot-0x2d, movie-trigger-41, menu-def-runtime) it breaks six menu tests that pass
without it (bisected to its merge alone):
`menus_e2e_mp::every_options_subpage_moves_its_values_and_rides_them`,
`menu_sp_e2e::a_completed_mission_routes_the_results_save_and_the_next_row`,
`menus_e2e::back_walks_the_single_player_chain_to_main`,
`menus_mouse_parity::right_click_is_back_and_the_wheel_is_up_and_down`,
`menus_mouse_parity::the_wheels_hover_scroll_and_click_like_the_arrows_and_enter`,
`menudef_script_runtime::same_time_keyframes_replace_unless_flag_0x20`.
Merge `origin/s/unproven-tests-menus` with the newest origin/master (which now carries menu-def-runtime), find
which change breaks the navigation (back, the wheel, the options sub-pages, the results route), decide from
the decompile which side is right [M], and make all six pass alongside your changes. Fix wrong tests only
with decompile evidence; never loosen a proven one.

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

**Branch:** `s/unproven-tests-menus-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
