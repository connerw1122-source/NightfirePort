# mp-match-end — report

Branch `s/mp-match-end` (from origin/master 3bb7be9a). Task 705: a local MP match ends and
returns through the debriefing — the frag-limit and time-limit ends, the debriefing page with
its board, ranks and winner line, Confirm → the scenario page, a second match starts.

## What was there and what was missing

The MP core was already on master (402 mp-match-rules + mp-debriefing-data2): the full
`MP_CheckForEndCondition` 0x184268 latch/dispatch, the `MP_Update` 0x184bd0 end states 1–6
including the handover flag and the `MpGame::request_debriefing` exposure at the case-4
navigation [M 0x184bd0:62-78], the debriefing board builder (`MpMatch::debriefing`,
P_MPDEBRIEFING 0x212198 msg 0x4c), and the front end's debriefing page with its two arms —
Confirm → the scenario page, Extra → the arena replay [M C_MPDBG 0x217860:8-16].

**The missing piece was the host glue**: nothing consumed `request_debriefing` — a real local
match reached state 5 and the end only logged. The fix mirrors the single-player end sequence
(`Drones::mission_end` → `App::demo_results`):

- `Sim::take_mp_debrief_request` (scene.rs) — the host's poll; takes the request once and
  returns the finished match's debriefing board plus the match's settings (the port-side source
  for `Menu_RestoreMPSettings` 0x202848's backup restore — the game memcpy's the pre-match
  MPSettings and the four PlayerSetting rows back before C_MPDBG's Confirm reopens the scenario
  page).
- `App::mp_results(board, settings)` (lib.rs, beside `demo_results`) — a fresh FrontEnd
  inheriting the music statics, the flow on the runner state [M 0x1c98c8 case 1], the MP menu
  state restored from the match's settings, the debriefing page opened with the board
  (GameState+0x1C = 0x40000033 [M 0x184bd0:63]), the front-end map requested
  (`ResetMap_LevelToLoad(0x7000048)` + the fade push [M 0x184bd0:64-78]); the arena sim freezes
  (the tick loop runs only while the front end is closed — the game's per-slot GS_PausePlayer).
- `menu.rs` — `show_mp_debriefing(board)` (the page half, the board by value: the host cannot
  borrow the match that lives in the sim the front end replaces) and `restore_mp_match(&MpSettings)`
  (the arena, the scenario + wheel rows, the limits with the confirm's ×0x3c inverted, the
  weapon set, radar, friendly fire, the agents' join records, the bot slots).
- `menu.rs` — the debriefing page's **draw** (the data existed; the page drew nothing): the
  menu-def page 0x40000033's layout [L mp-debriefing.md §4] — the header (50,40) 0x151, the
  five column titles (Agent/Points/Victories/Deaths/TOTAL) at x=52 on the pinned ys
  183/217/250/284/318, the four slot panels at x 176/280/384/488 (the portrait 81×81 at y=102 —
  the row's small-table texture or the panel's placeholder 0x0300015d/c/56/6b; the rank caption
  over the portrait; the name at y=183; the values under the pinned ys), the result plate
  (176,355,408×52) + line (178,369) with the winner/draw text, the hint bar 0x2c1 and the
  NightFire logo — all binding the mp-debriefing-data2 getters (`mp_debriefing_rows` /
  `mp_debriefing_result`).

Confirm then returns to the scenario page with the match's choices live, and `mp_confirm_start`
hands the same arena over again — the second match needs nothing new.

## The tests (the real path)

`crates/nf-view/tests/mp_match_end_live.rs`, both green:

1. **The frag-limit end**: the arena (Atlantis 0x07000023 [D MP_MAP_ROWS]) loads through
   `load_mp_carry` — the exact loader the confirm's handover drives — with frag limit 1; one
   frag rides the scoring entry `MpMatch::player_killed` (the port's MP_RegisterKill; the sim's
   drone-death wiring is phase 2 and will call this same fn); the real per-tick machine
   (`step_bond` → `MP_Update`) walks states 1 → 3 → 4 → 5 and the request arrives; the board
   carries "Player 1" 1st with the single frag; the winner line binds the codename; the
   front-end map 0x07000048 is requested; Confirm lands on the scenario page (0x4000001a) with
   the arena, the frag limit and the joined agent restored; `mp_confirm_start` re-requests the
   arena with the settings — **a second match can start**.
2. **The time-limit end** (1 minute, the confirm's minimum [M the ×0x3c]): 3600 real ticks latch
   0xfe [M 0x184268:142-143], the machine runs 2 → 3 → 4 → 5, and the scoreless board's result
   line is the draw text 0x010002a0.

## Verification

- Tests run before push: mp_match_end_live 2/2, mp_debriefing_flow 6/6, mp_menu_flow 9/9,
  mp_defaults_setup 10/10, menus_e2e_mp 5/5, mp_net_lobby 9/9, menu_audio_flows 9/9,
  mp_bots_live 2/2, mp_core_live 5/5, nf-game mp_debriefing 10/10, mp_match_rules 14/14,
  mp_hud_data 4/4 — **85 green across the touched path**.
- The fast-runner suite on the final tree: started 17:34 UTC; the total line is appended below.

## What's left

- The suite total (appended below when the run lands).
- Phase 2's sim→MP wiring (a drone death in an arena calls `MpMatch::player_killed`) is a
  separate task; the scoring entry this task's tests use is exactly the fn it will call.

## Live checks for the main agent

1. **The frag-limit match end**: start a local match (frag limit 1), frag the bot — the
   debriefing page should open over the arena: the board (2 rows), the ranks, "Player 1 wins
   the match."
2. **The debriefing page's render**: header "Debriefing" top-left, the five column titles on
   the left column, the up-to-4 slot panels (portrait, rank caption over it, name, the three
   value columns + TOTAL), the result plate at the bottom with the winner line, the hint bar
   "~A Continue  ~Y Replay". The rank caption's exact y over the portrait is [S] (the def gives
   no y; drawn at the portrait's top edge) — PCSX2 m-page 0x40000033 settles it.
3. **Confirm** on the debriefing → the scenario page; **start another match** from it — the
   same arena, the same limits.
4. **The time-limit match**: set a 1-minute limit, wait it out — the debriefing opens with
   "Match was a draw" (no frags).
