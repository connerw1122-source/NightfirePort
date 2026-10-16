# pause-results-flows — the in-mission menus, end to end

Branch `s/pause-results-flows` (from master ad761677). Queue 674.

## What's done

**Nine headless end-to-end tests** on live scenes (`crates/nf-view/tests/pause_results_flows_live.rs`,
all opt-in via `NF_ISO`), driven ONLY through `FrontEnd::action()` / `tick()` plus the host glue
mirrored where the headless suite can reach it (every mirror cites its lib.rs arm):

1. `esc_opens_the_pause_page_and_the_sim_stands_still_until_it_closes` — Esc over a live 07000005
   opens the pause page on MISSION/CONTINUE with the box closed [M Player_Update 0x193600:71-78,
   the page 0x4000004b rows 0x16a/b/c]; 120 host frames freeze the sim (`drones.frame` and Bond's
   position unchanged — the port's gate lib.rs:3090 is the GS_PauseGame(1) of Game_Run
   0x1c9790:30-35); Esc/Back resumes and the sim runs again [M MenuManager_Monitor 0x21d508:24-39 +
   MenuManager_Delete 0x21c2b0].
2. `the_pause_tabs_cycle_with_left_right_and_side` — on a live 07000007 (whose rows announce at
   spawn [M 0x1827d8:41-54; 05's own first objective waits for its enable channel]): the four tabs
   cycle with Left/Right and wrap, Tab (the game's ~H Next) steps, entering OBJECTIVES re-selects
   the last live row [M 0x204408 msg 0x1d], Up/Down scroll it, a row's ✕ hint opens on Confirm and
   pops on the next key, and Esc resumes from a non-MISSION tab. The rows are fed the way the
   window populates them every frame (lib.rs:3431-3433 → `pause_objective_rows` [M 0x204408:251-271]).
3. `pause_continue_resumes_the_mission` — CONTINUE closes without a load [M 0x204408:186-192] and
   the mission resumes.
4. `pause_restart_reloads_the_base_map_cold_on_full_health` — RESTART's YES/NO opens on NO, NO
   cancels clean, YES requests the mission's base map COLD [M 0x204408:134-141], the host takes the
   request exactly once (lib.rs:1822), the cold reload comes back at 100/0 with clean channels and
   a fresh entry checkpoint [M Player_Init 0x1910f8:128-138 + Init_SwitchChannels 0x1d5080:44-46],
   and 360 ticks hold with no re-fail.
5. `pause_quit_returns_to_a_working_front_end` — QUIT's YES queues 0x4000001c + the front-end map
   [M 0x204408:142-149]; the real 07000048 archive builds; the queued page re-opens (see the fix
   below); the mission list works by actions (driving notice → dismiss → The Exchange requests, a
   repeated Confirm refused [M 0x1d3800:11-17 + the MissionSelect guard]).
6. `dying_opens_the_fail_menu_and_restart_re_serves_the_entry_checkpoint` — a real 500-damage kill
   on 07000005, ✕ past the red fade [M 0x190bb0:32], the 0x40000042 menu opens with the sim
   answering nothing (`auto_restart=false`); the front end mirrors the host's results hook
   (lib.rs:3296) onto the FAILED page [rows REPLAY/QUIT, M P_ENDMISSION 0x216618:19-27]; row 0
   requests the CURRENT level WARM and the sim's `fail_choose_restart` agrees; the host serves
   `fail_restart_save(entry)` [M Player_RamLoad 0x1957a8] and the warm reload comes back at full
   health on the entry checkpoint's clean channels — 360 ticks, no replay loop.
7. `the_fail_menu_quit_row_returns_to_the_front_end` — row 1 (value 2) queues 0x4000001c + the
   front-end map [M 0x216618:24-26]; the sim's `fail_choose_quit` agrees (cold).
8. `completing_the_mission_reaches_the_results_and_the_next_pages` — ch 100 on a live 07000005
   runs the banner → the end sequence [M 0x182c60:85-122]; the front-end leg wraps to the map with
   the results page queued [M 0x1d6360:160-163]; `results_enter` builds the fields and stamps the
   award (Alpine Escape unlocks [M 0x1ffa10:62-66]); Side opens Stats and Back returns; Confirm
   routes to NfBonus when a new reward stands, then the bonus page confirms on to the mission list
   [M 0x208e78:39-53, P_NFBONUS 0x209670:47-57], which starts the next mission.
9. `the_loading_hold_window_and_the_double_press_guards` — pins `LOADING_HOLD == 5.0` (300 ticks at
   the fixed 60 Hz; a port choice [S] — the game's cold-entry minimum is its loading clip
   [M 0x1d6360:101-113], restarts have none [M 0x1d6360:39,184]) and the host's take-once rule
   (lib.rs:1822); the window's input gates during the hold are cited in place (lib.rs:2790 keys,
   2729/2737/2750 mouse, 2998 the frame-ending poll).

**Three small fixes** (each cites the game):

- **Quit now lands on the mission list, not the placeholder page.** The quit arms leave the front
  end on a placeholder while the front-end map loads; the game's load tail runs its own
  `MenuManager_Create(GameState._32_4_, GameState._28_4_, …)` [M ResetMap_Load 0x1d6360:160-163],
  which re-opens the queued page. New `FrontEnd::reopen_queued_page()` (menu.rs) is called by the
  host's front-end-load swap branch (lib.rs:2074). A model-path edge (set in `GameFlow::load_tick`,
  consumed in `tick`) was tried first and REVERTED: it perturbed the boot flow (menus_e2e 0/5 —
  the boot's own queued page is not a no-op there); the host branch is the path the real window
  takes anyway.
- **The Esc arm rides the runner state** (`fe.flow.stack = vec![ST_RUNNER]`, lib.rs:2886) — the
  game's GameState sits on state 2 while a mission plays [M GameFlow_Main 0x1c98c8 case 2], which
  is what makes the pause arms' `ResetMap_LevelToLoad` state gate accept [M 0x1d3800:11-17].
- **`LOADING_HOLD` is pub** so the headless suite can pin the 5 s window (lib.rs:1745).

## How it was verified

- `pause_results_flows_live` 9/9, `menus_e2e` 5/5, `menus_flow` 42/42, `front_end_chain_live` 2/2,
  `fix_restart_target_live` 2/2 (`cargo test --offline --release -p nf-view`, NF_ISO set) — these
  are the crates/tests the changes touch. The fast runner (`tools/test/suite.py`) was started in
  the background three times; this sandbox reaps background jobs between tool calls (the previous
  session's WORKLOG notes the same), so the full-suite total could not be produced here — the
  per-crate runs above stand in for it. See "For the next runner" below.
- Regressions were checked against master by stashing the src changes: `menus_e2e` 5/5 both ways
  after the revert of the model edge; the interim guard on the fail rows was also reverted after
  `menus_flow::the_results_pages_split_complete_from_failed` showed it refused the FIRST row press
  (the fail arm's own queued request stands at page-open — the game's one-press rule is modeled by
  the arm's page move, M 0x216618:26).

## Findings / what's left

- **Sim-side fail menu pause depth**: the port holds Bond (GS_PausePlayer) but the drones keep
  ticking under `fail_menu_open`, where the game's `GS_PauseGame(1)` [M 0x182c60:78] freezes the
  whole object pump [M 0x1c9790:30-35]. Not changed here (the fail-choose machinery and several
  live tests depend on the current behaviour); flagged for a task with a full-suite window.
- **Results stats are `Stats::default()`** — the host's `demo_results` never wires the sim's real
  shots/hits/time (pre-existing; the results fields' numbers on screen will be zeroes).
- **The fail page's re-press re-requests the same level** (benign — the host takes it once; the
  game deletes its manager so a second press is impossible there [M 0x216618:26]).
- **Esc during a cutscene**: the game refuses pause while `ScriptCam != 0` [M 0x193600:71-78]; the
  port's Esc arm has no cutscene guard (pausing freezes the cutscene instead of refusing).

### Live checks for the main agent

1. Pause during the 07000005 intro cutscene: press Start in PCSX2 while the intro NIS plays —
   expect NO pause menu (ScriptCam gate [M 0x193600:75]). In the port, Esc during the intro would
   open the pause page over it — if confirmed, the Esc arm needs `!sim.disabled`.
2. Pause → Quit → YES: expect the MISSION LIST (0x4000001c), not the main menu — the new
   `reopen_queued_page` in the swap branch should now match the game [M 0x1d6360:160-163].
3. The loading screen's minimum: the 5 s hold is the port's own choice [S] — the game's cold
   entries play the loading clip (skippable), restarts have no minimum [M 0x1d6360:39,184].

### For the next runner

The full suite (`tools/test/suite.py`) needs a sandbox that does not reap background jobs, or a
foreground chunked run (the difficulty-effects2 session's approach: 76 binaries). The final tree
here is df10fd01 + the report/WORKLOG commit.
