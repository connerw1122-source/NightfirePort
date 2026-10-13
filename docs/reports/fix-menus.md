# fix-menus — the front end

Branch `s/fix-menus`. Everything is in English; facts are tagged [M] code (function + address), [D] disc, [L] EE snapshot, [S] suggested.

## What's done

The game's front end is ported into `crates/nf-view`:

- **The GameFlow state machine** (`crates/nf-view/src/menu.rs`, `GameFlow`) — the GameState stack from
  ACTION.ELF: `GameFlow_GetState` 0x1c9e80, `GameFlow_PopState` 0x1c9eb0, `GameFlow_PushState` 0x1c9f38
  (including its conversions: a push of 7 starts a fade and becomes state 8; 0xb becomes the frame-wait
  0xc), and `GameFlow_Main` 0x1c98c8's per-state bodies with its draw-skip rule (states 1/3/4/5/9 skip
  Game_Draw). Boot follows the game: `bootup_bootup` 0x1c7e78 opens page 0x40000034, and
  `Reset_MapLoadSettings` 0x1d6958 requests the front-end map 0x07000048, which `ResetMap_Load` 0x1d6360
  loads (its movie-wrap phase for 0x0710xxxx codes and the pop-3/push-4 tail are both modelled; state 4
  = `Locks_Init` 0x1b0548 and pops itself). GameState fields modelled with their game offsets:
  +0x08 level to load, +0x10 movie code, +0x12 current level, +0x24 load-pending, +0x28 page, +0x40
  skip-draw. Nothing on NTSC-U ever pushes state 10 (every `GameFlow_PushState` call site is
  enumerated in the decompile) — state 2 is both the front-end runner and the in-game runner.
- **The menu screens**, with the game's own page ids. The page table lives in ACTION.ELF `.data`
  (16-byte entries {handler, size, flags 0x12, 1, name offset}; id = 0x40000000 | index — found by
  probing for `P_START_Handler` 0x20a408 and confirmed: index 0x34 = P_START, and `bootup` opens
  0x40000034, `Mission_Update` 0x182c60:74 opens 0x40000036 = P_NFRESULTS):
  - **Title** (P_START 0x40000034 / handler 0x20a408): flashing "PRESS START" on the game's 240-frame
    cycle, the legal line (label 0x375), and the attract timer (2700 input-less frames = 0xa8c [M
    `Menu_GetNoInputCount` use]; the attract demo itself is not ported — see Left).
  - **Main menu** (P_MAIN 0x4000002e / 0x20a7b0): Nightfire / Multiplayer / Options [S row order —
    the disc page data wires the buttons; the C_GONIGHTFIRE 0x2169b8 / C_GOMULTIPLAYER 0x216a78 /
    C_GOCODENAMES 0x216b38 pages exist]. Nightfire → mission select; Multiplayer is a stub (reported).
  - **Mission select** (P_NFSELECT 0x40000039 / 0x20aec0): the 9 campaign missions [M `MissionData`
    0x2a4350, docs/formats/mission-data.md].
  - **Difficulty** (P_NFDFCTY 0x40000047 / 0x216bf8): Agent / Operative / 007 [S names; the `difficulty`
    global is 0x2df4c0 [M symtab]]. Confirming starts the mission the game does: a cold load mapped to
    the 0x0710xxxx intro wrapper [M `ResetMap_LevelToLoad` 0x1d3800's switch], the movie state plays
    first [M 0x1d6360], then GameState+0x12 = the level.
  - **Pause** (P_PAUSE 0x4000000c / 0x203c28): resume / restart / options / quit-to-menu. The actions
    are the game's own, from `C_GCPAUSE_Handler` 0x204408: restart = `ResetMap_LevelToLoad(BaseMap,
    0, 1)` (the mission's FIRST map, cold, no intro), quit = a **two-step confirm** (the game arms
    `state_quit_166` and swaps the row text — label 0x200 is literally "QUIT: ARE YOU SURE?" [D
    USATxt.dat]) then `GameState+0x28 = 0x4000001c` + `ResetMap_LevelToLoad(0x7000048, 0, 0)`.
  - **Results** (P_NFRESULTS 0x40000036, reached as Mission_Update 0x182c60:74 sets the page and pushes
    a 60-frame fade): MISSION COMPLETE / MISSION FAILED with a stats line, and the three choices of
    `P_ENDMISSION_Handler` 0x216618: restart (BaseMap), continue (current level, warm, `Player_RamLoad`
    in the game — the port re-loads the level), quit to menu (0x4000001c + 0x7000048).
- **Draw side** (`menu_overlay` in menu.rs): per page, one `gpu::HudFrame` built from the game's own
  font (FONTSerpLight, texture 0x03000003 [M docs/systems/hud.md]) on the game's 512 × 448 screen, a
  dim backdrop plate, the page head, the row list with the cursor highlighted, and the pause quit
  confirm line. Strings come from USATxt.dat (`Lang`, the game's own language file) when available.
- **Host wiring** (`crates/nf-view/src/lib.rs`): plain `nf-view <iso>` now boots to the title screen
  on the 07000048 map (the game's own front-end level); `nf-view <iso> <level>` keeps the direct
  launch unchanged. Mouse + keyboard: arrows/WASD move, Enter/Space/click confirm, Esc back; Esc in a
  mission opens the pause page (the sim is not ticked while a menu is up); a menu-picked level loads
  through the existing `pending_level` reload path. F8/F9 show the mission-complete/failed results
  page for the loaded mission (demo hook — see Live checks).

## How it was verified

- 9 new headless tests in `crates/nf-view/tests/menus_flow.rs`, one per transition: boot → title
  through the real load path; title → main → mission select → difficulty → launch (with the 0x07100005
  wrap, the movie state, and the back-map); pause resume/restart/two-step quit; results
  restart/continue/quit with the game's page ids and fade state; cursor wrap + back navigation; the
  attract timer; the overlay producing font quads for every page. The NF_ISO ones parse the disc's
  USATxt.dat and check the game's own strings.
- Disc evidence found along the way (all in the tests' output): 0x200 = "QUIT: ARE YOU SURE?", 0x201 /
  0x1000323 / 0x202 = button-prompt legends, and P_PAUSE 0x203c28's eight list labels 0x82, 0x91, 0xa0,
  0xaf, 0xbe, 0xcd are **"NightFire", "Moonraker", "Octopussy", "Goldfinger", "Dr. No", "Thunderball"**
  — the multiplayer scenario names, not pause rows. That cost me one wrong first draft of the pause
  rows (8 rows from those ids); the shipped model is the 4-row one with C_GCPAUSE's actions.
- Full suite: 131 + 124 test binaries green (nf-game, nf-formats, nf-audio, nf-view), the only failures
  the 3 allowed `disc_golden` tests on the slim ISO. This branch touches only `crates/nf-view`
  (`menu.rs`, `lib.rs`) plus the new test and docs — nf-game/nf-formats/nf-audio are untouched.

## What's left (for the next message)

1. **The disc's page data** (the front-end layouts): the menus are data-driven in the game —
   `MenuManager_Load` 0x21de88 / `MenuManager_Create` 0x21b908 load pages of widgets (M_BUTTON,
   M_LABEL, M_COMBO, …) with scripts. The port's row sets for main/options/pause are [S] where the
   handler code doesn't pin them; parsing the page files from the disc would make every label and
   layout exact and unlock the real backgrounds.
2. **The menu background FMV** (`menu_movie`, SFX 0x1d7/0x470 [M 0x20a408]) — the menus play a
   background movie; the port draws a dim plate instead.
3. **Attract mode** — the 2700-frame timer is ported; the demo playback it switches to is not.
4. **Mission-event → results glue**: the game reaches P_NFRESULTS from `Mission_Update`'s complete/fail
   events; the port has F8/F9 demo hooks. The nf-view loop should watch the sim's mission status and
   call `FrontEnd::mission_ended` with real stats (shots/hits/time).
5. **The options page** is a stub (screen adjust / controls / back); P_CNOPTIONS 0x213f08,
   P_CNAVOPTIONS 0x4000006a, P_SCREENADJUST 0x4000006c and the difficulty/look inversions in
   `PlayerSetting` are not wired.
6. **Continue-after-results** needs the RamSave carry (`Player_RamLoad` 0x1d…) into the next mission
   part (`GameState+0x12`'s successor via MissionData).
7. The title's legal line label 0x375 does not resolve in our USATxt.dat parse (empty) — check the
   label banks (`Txt_BindLabel(id, bank)`'s second arg is always 0 in these calls; maybe 0x375 lives
   in another language file or the legal text is a sprite).

## Live checks for the main agent

Please run in PCSX2 and compare with `nf-view <iso>` (windowed) / `--shot`:

1. **Pause rows' real labels**: the disc page data holds the single-player pause texts. Watch
   `Txt_BindLabel` 0x1caad0 (or breakpoint `__Menu_SendEx` 0x1f2…) with the pause menu open and log
   which label ids the eight controls (0x10000028's controls 6, 8, 9, 0xb, 0xc, 0xd, 0xe, 0x10) get.
   Expect four visible rows and their texts (Resume / Restart / Options / Quit is my [S] guess).
2. **Main menu rows**: same for P_MAIN 0x20a7b0's page (id 0x4000002e) — which buttons exist besides
   Nightfire/Multiplayer (Codenames? Options? Mem card?) and in which order.
3. **Difficulty choices**: confirm the three choices and their label ids on P_NFDFCTY 0x40000047
   (expect Agent / Operative / 007 texts).
4. **F8/F9 vs the real results page**: open the real P_NFRESULTS (0x40000036) after a mission and
   screenshot it; compare with my results layout (head + stats line + 3 rows). The stats the game
   shows (time, accuracy, saves?) come from the stats pane 0x100001a4 — the record fields are in
   `PlarStat_*` 0x199… if the next task needs them.
5. `nf-view <iso>` should show the title screen with a flashing PRESS START and the legal line at the
   bottom (mine shows neither text if 0x375 doesn't resolve — see Left #7).

## ntfy

Delivered 13:36 UTC (curl, first method, response id 2xr9p6sFXsnY).

(sent after this report is pushed; see the queue finish log if it failed)
