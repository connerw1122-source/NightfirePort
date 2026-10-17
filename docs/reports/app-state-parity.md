# app-state-parity — report

Branch `s/app-state-parity` (2026-10-17). Task: audit the `App` fields initialised from args or
at startup that the GAME updates per load or per menu choice, make the port update them on the
same event, and pin each with a test. Docs first: `docs/systems/app-state-parity.md` (the full
table with addresses and evidence tags).

## What's done

The audit walked the `App` initialiser (`crates/nf-view/src/lib.rs` `run()`'s `App { ... }`)
against the game's stores (`GameState` 0x2a3768, `CheatInfo` 0x2a3738, `PlayerSetting` 0x2a38c8,
`SelLang` 0x30cee0, `menu_unlock_everything` 0x30d2a7, the PTP block 0x31c090). Two of the
candidates were stale and are fixed; the rest are already event-correct or have no port event
(the table below; details and [M]/[D]/[L] citations in the systems doc).

| Field | Game source | Was it stale? | Fix |
|---|---|---|---|
| `selected_difficulty` | `GameState+0x28` (0x2a3790): boot 2 [M 0x1c7e78:25], difficulty page [M 0x216ca8:15], **MP confirm 1 [M 0x20ec00:150]**, mini-mission 1 [M 0x1ffb90:22], driving return [M 0x1c8560:36] | **YES** — the MP start never wrote it; the App kept the SP pick across a match (the `level_code` bug's class) | `mp_confirm_start` + `mp_client_start` raise `requested_difficulty = Some(1)`; the App's `menu_after` take adopts it (the same channel the difficulty page's pick rides) |
| driving-return difficulty | the PTP block +0x144 [M 0x1c8198:15-20 → 0x1c8560:35-36 writes it back] | **YES** — `DriveReturn` dropped the field | `DriveReturn.difficulty` (carried from the block's `mission_state_1`), applied in `swap_in_scene` to `Progress::difficulty` + the `Drones::difficulty` mirror |
| mission row | the list re-derives per open: highest unlocked [M P_NFMAP 0x20b608:57-64]; Records opens at `GameState+0x48` [M P_DSRECORDS 0x217248:13] | no — the port re-derives at every entry and seeds the pause manager from the mission's BaseMap | none |
| `entry_save` | the RamSave 0x2b9440 re-saved every load [M 0x1a7ef8:325] | no — `swap_in_scene` re-captures (cold included) | none |
| ptp save | the PTP block 0x31c090 | partially (the difficulty half, row 2) | `DriveReturn.difficulty` |
| `all` | no game-side writer found [M-negative] | no — port debug flag, persisting is intended [S] | none |
| cheats | `CheatInfo` 0x2a3738 + `menu_unlock_everything` 0x30d2a7; the cheats page's spinners write them [M 0x215ee0:9 / 0x2164c0:9] | no port event exists (the cheats page is unported) | none — the writers are cited in the systems doc for the page's task |
| the sim's settings | `PlayerSetting` 0x2a38c8, the invert byte read every tick [M 0x17ee88:50], toggled by the pause's controls page [M 0x204408:526-527] | no port event exists (no controls page); the sim flag exists (`PadConfig.invert_look`) and boots 0 like the game's memset [M 0x17edd8:13] | none — documented |
| the sound bank | per level [M ResetMap_Load]; the front-end flip `DAT_002e1b11` [M 0x1dceb8:94-101] | no — `build_sound` per swap + `set_primary` on the flip | none |
| the language | `SelLang` 0x30cee0, `Txt_LoadLanguage` loads `FNames[SelLang]` [M 0x1caad0:64]; the disc carries USATxt only [D] | no — boot + per-load, matching NTSC-U | none |

## How it was verified

- **Reproduce first.** The pinned test went through the real input path (MpMap confirm →
  MpSetup → the confirm page's `mp_confirm_start`) and failed on the pre-fix tree, captured by
  stashing the menu.rs fix and re-running:
  `thread 'repro_mp_confirm_leaves_difficulty_unset' panicked at crates/nf-view/tests/zz_repro_tmp.rs:16:5: left: None, right: Some(1)` / `test result: FAILED. 0 passed; 1 failed`.
  The temp repro file was deleted after the run; its assertions live on in
  `mp_defaults_setup::mp_confirm_start_writes_difficulty_1`, which passes on the fix (with the
  sister assertion `requested_level == Some(fe.mp_level)` that can fail alone).
- **The ptp round-trip** is pinned in `nf-game`'s ptp unit tests: the block built with
  `game_state_40 = 1` lands in `DriveReturn.difficulty`, and `apply` writes it into the
  caller's mirror (`assert_eq!(difficulty, 1, "the driving return restores GameState+0x28")`).
  `walk_07000004_ptp_live` additionally pins `ret.difficulty == sim.drones.difficulty` through
  the live handover.
- **Suites run** (this sandbox kills background jobs — two nohup/setsid starts died with no
  output — so the runner went to the foreground; the full run cannot finish inside one tool
  call and restarts from the first binary each chunk, so three targeted `--only` chunks over the
  touched systems stand in, on the final merged tree):
  - `mp_defaults_setup, mp_menu_flow, mp_core_live, mp_match_end_live, mp_host_match_live,
    difficulty_wiring_live, walk_07000004_ptp_live, level_end_live, fail_replay_level,
    pause_results_flows_live, menu_sp_e2e, gameflow_stack_sp_run, menus_e2e, mp_net_lobby,
    net_e2e_loopback_live` → **80 passed, 0 failed, 0 ignored**.
  - `async_level_load_live, async_load_frame_loop, mp_bots_live, mp_screens, menus_flow,
    menu_audio_flows, menu_music_flow, mp_debriefing_flow, mp_defaults_setup, unlock_popups,
    mission_unlock_flow, save_progress_roundtrip` → **112 passed, 0 failed, 1 ignored**.
  - `menus_e2e_mp, mp_menu_lists_live, codename_entry, settings_persist, memcard_save_flow,
    mp_defaults_setup, mp_match_end_live, level_carry_live, level_reload_live,
    fix_restart_target_live, fix_restart_health_live` → **78 passed, 0 failed, 0 ignored**.
  - Directly: `nf-game --lib ptp` 11 passed; `driving_mission_live` 5 passed;
    `walk_07000004_ptp_live` 4 passed; `mp_defaults_setup` 11 passed.
  - The full build (`cargo test --no-run` over the four crates) completed: **496 test binaries
    build ok** on the final tree. The only failures the full run surfaced before the window cut
    it off were the 3 `disc_golden` tests, which the slim ISO is documented to fail.

## What's left

- The cheats page (`CheatInfo`) and the controls page (`PlayerSetting` invert) are unported;
  their game-side writers are cited in the systems doc so those tasks can wire the same events.
- The mini-mission launch (`Menu_RunMiniMission` 0x1ffb90 writes difficulty 1 and loads
  0x9000001) cannot fire in the port yet (driving maps are refused from the menus).
- A full uninterrupted `suite.py` total line is owed (sandbox limitation above); the two
  targeted chunks cover every touched system.

## Live checks for the main agent

1. **MP confirm writes the difficulty.** In PCSX2, boot → main menu → Multiplayer → start any
   match; at the confirm page's start (`P_MPCONFIRM` 0x20ec00 case 0x4b), `gdbwatch.py` on
   **write** at `0x2a3790` (GameState+0x28) should record `sw` of **1** (and the preceding
   `MPSettings+0x180 = 1` store). Expected before this task: the port left its mirror at the
   difficulty page's last pick.
2. **The difficulty persists after the match.** After the debriefing → main menu, `0x2a3790`
   still reads **1** [M no other writer on that path] — the port's next SP mission (without
   visiting the difficulty page) should then run at difficulty 1 (Operative rosters).
3. **Driving return restores the difficulty.** On a driving interlude's return
   (`Boot_LoadPTPData` 0x1c8560), a write watch on `0x2a3790` should record the block's +0x144
   value (the pre-drive `GameState+0x28`); with the mini-mission special case
   (`Boot_GetPTPData` 0x1c8198:15) it is **1**.

## Usage

not visible to me
