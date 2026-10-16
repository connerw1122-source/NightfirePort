# menu-e2e-singleplayer — report

Branch `s/menu-e2e-singleplayer` (merged origin/master ad761677 before the final runs).

## What's done

`crates/nf-view/tests/menu_sp_e2e.rs` — 9 end-to-end tests, every single-player path from boot
to a level request, driven **only** through `action()`, `tick()` and the host-side calls the
window makes. No `page`/`cursor`/state-field writes to get anywhere (the playtest's rule — the
earlier flow tests set `fe.page = ...` directly, so the real paths were never exercised).

| test | covers |
|---|---|
| `boot_reaches_main_through_the_intro_both_ways` | title → intro **skipped** (second Enter) and **played out** → Main → NightFire; the intro swallows other input |
| `back_walks_every_single_player_step_to_the_title` | Dossier (□) open/Back; MissionSelect → Difficulty → Codename → Main → **title**, each Back landing on the previous page, forward again after |
| `select_codename_play_without_saving_asks_and_back_cancels` | the confirm box: opens, △ cancels, re-arms, Yes → Difficulty (Agent, the middle row) |
| `select_codename_new_codename_validates_creates_saves_and_reloads` | New Codename → the name grid (the game's "BOND" proposal, ✓ focused), the **empty-name refusal** box 0x3bc through real Delete/✓ presses, typing `MONEYP` by d-pad walks, ✓ → the create **saves to the card** → lands on the wheel; Back to Main → the card's codename is listed (rows 2+n) → loads → Difficulty → MissionSelect |
| `each_difficulty_reaches_the_mission_list` | the wheel clamps (never wraps) [M 0x2221b20:186-210], all three rows (Operative/Agent/00 Agent) confirm into the mission list, Back re-opens the wheel on the chosen row |
| `every_unlocked_row_requests_its_campaign_level` | CAMPAIGN == `SP_LEVEL_HASHES` row by row; fresh profile: row 0 = the driving notice (no request, flow stays), dismiss, The Exchange requests 0x07000005; a second Enter while the request stands is refused; `--unlock-all`: **all 12 rows** — every on-foot row asserts the exact `requested_level` against CAMPAIGN, every driving row shows the notice (Confirm and Back both dismiss) |
| `a_completed_mission_routes_the_results_save_and_the_next_row` | a saved codename (created through the real branch) → the Exchange → `mission_ended(Complete)` → the outro-less wrap to the map → `results_enter` (improved, medal, `new_reward`) → the **save prompt** → Yes → ASK_OVERWRITE → the write → the **NfBonus hop** → `bonus_enter` → Confirm → the mission list opens on the **highest unlocked row** (4, through the two chained driving rows) → Double Cross (0x07000001) starts → the written file carries rows 0-2 → **reloading the codename brings the unlocks back** |
| `a_failed_mission_restarts_and_quits_from_the_results_page` | the fail menu (2 rows), △ lands on QUIT, **REPLAY = the current level, warm** (the playtest's "restart from the results page"), **QUIT = the front end** with +0x28 = 0x4000001c |
| `the_final_mission_routes_to_the_win_page_and_credits` | Equinox (0x0700001b) via `--unlock-all` → `mission_ended(Complete)` → **the real outro wrap 0x0730001b** → phase-2 resolve → results (`just_completed`) → the routing → the win page (no controls) → `win_movie_finished()` (host) → **Credits** |

## The driving discipline

Host-side calls used (each one is a call the window or the game's own flow makes):

- `set_profile_dir` — the card wiring before the menus run;
- `fe.unlock_all = true` — the host's `--unlock-all` dev flag [M menu.rs 0x30d2a7 note];
- `fe.requested_level.take()` / `fe.flow.phase2_resolved.take()` — the host consumes each per
  leg [M lib.rs 2175, 2326];
- `fe.flow.stack = vec![ST_RUNNER]; fe.flow.current_level = <level>` before `mission_ended` —
  exactly the host's own prep [M lib.rs demo_results 2076-2077];
- `mission_ended` / `results_enter` / `bonus_enter` / `sync_profile_progress` (host order:
  commit, then save) / `win_movie_finished`;
- `card_answer(BTN_YES)` — the memcard dialog's button (the window renders the box and feeds
  the press; the same call the established memcard/codename/campaign tests use);
- `fe.tick()` pumps `main_tick` + `load_tick` + `card_tick` + the attract clock, the way the
  window's frame loop does.

The name grid is driven purely by d-pad `action()` moves over `CN_GRID` (a BFS walker that only
steps onto real cells — the rows have different lengths) + Confirm presses; the window has no
other input path for that page (lib.rs never calls `cn_type_char`).

## Findings

1. **The fail-menu REPLAY sequencing (potential port bug, needs the main agent).** After
   `mission_ended(Failed)` the flow arms the front-end load immediately [M 0x182c60:45-57 per
   movie_playback_flow]. Headless, that load lands on the next ticks and overwrites
   `flow.current_level` (GameState+0x12) with 0x07000048 — a tick-delayed REPLAY then requests
   **0x07000048** (the front-end map) instead of the mission. On the disc the mission-failed
   screen shows while 0x7000048 streams in, so a player answers before the load lands; the test
   feeds its raw actions inside that same window and pins REPLAY = the mission level, warm [M
   P_ENDMISSION 0x216618:19-21 — the contract front_end_chain_live pins]. When the real (non-F8)
   mission-end host glue lands, REPLAY must keep reading the mission's level — see the live
   check below.
2. **The phase-2 resolve is a per-leg consume.** The briefing wrap a mission pick arms resolves
   instantly headless (the movie countdown is 0 after the intro), leaving
   `flow.phase2_resolved` set; the host takes it per leg [M lib.rs 2326]. The e2e walker mirrors
   that (an unconsumed stale resolve made my first results walk load 0x07000005 again). No
   product change needed — the host already does the right thing.

## Live checks for the main agent

1. **P_ENDMISSION REPLAY's level source.** In PCSX2, fail 0x07000005 (or any mission) to the
   mission-failed screen (page 0x40000042). Watch **GameState+0x12** (the current level, the
   field `ResetMap_LevelToLoad` writes at load phase 3, [M 0x1d6360 tail]) from the fail screen
   up to the answer. Expected: it holds **0x07000005** while the screen stands (the 0x7000048
   load lands after the answer, or REPLAY cancels it). Then press REPLAY and confirm the
   mission restarts warm. If +0x12 becomes 0x07000048 before the answer, check what
   P_ENDMISSION_Handler 0x216618:19-21 actually reads (`gdbwatch.py` on 0x12's load/store during
   the fail screen).
2. **The driving notice render** (menu overlay, any boot reaches it): fresh profile → NightFire
   → Play without saving → Agent → confirm **Paris Prelude** (row 0). The port's notice box
   (the game's option-box look) should draw centered over the mission list with the three-line
   "Paris Prelude is a driving mission. / Driving isn't in the PC port yet - / the next mission
   is unlocked." Confirm (X) dismisses it and the list is interactive again (Down moves to The
   Exchange, which then starts).

## What's left

- `mouse_move`/`mouse_click` are permitted input but not exercised here — the playtest symptoms
  were keyboard-driven, and the wheel/option pages' mouse handling already has menus_flow
  coverage.
- The save prompt's **No** arm, the card Retry/error dialogs and the no-improvement no-prompt
  case keep their unit coverage (memcard_save_flow) and were not re-walked e2e.
- The Stats page's host glue (`stats_enter` is never called by the App — menu.rs:4200 expects
  it) is untouched host work, noted here.

## What I ran

- `menu_sp_e2e`: **9 passed / 0 failed** on the branch-point tree and again on the merged tree
  (origin/master ad761677) after the merge.
- The full fast suite (`tools/test/suite.py`) on the final merged tree — see the WORKLOG entry
  for the total line.
