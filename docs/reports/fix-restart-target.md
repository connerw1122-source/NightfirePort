# fix-restart-target — the fail/death menu's Restart reloads the CURRENT level, warm, on the entry checkpoint

## What was wrong

The user, failing on 07000007: "rather than go to the start of the level it goes to the very
beginning map, 05". The port's `Drones::fail_choose_restart` queued
`level_exit = mission.base_map` + `level_exit_cold` — a cold reload of the mission's first level.
The user is right: the real game's fail screen Restart re-plays the level you are on.

**The cause: the fail menu's handler was misattributed.** fix-death-restart implemented the
0x40000042 fail/death menu's Restart from `C_GCPAUSE_Handler` 0x204408:139
(`ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)` — BaseMap, cold). But 0x204408 is the PAUSE
menu 0x10000028's handler: `Handler_HandleMessage` 0x2029e0 routes menu id **0x40000042 to
`P_ENDMISSION_Handler` 0x216618** [M 002029e0:498-500] and 0x10000028 to `C_GCPAUSE_Handler`
[M :217-219]. The two Restarts differ:

| menu | handler | Restart |
|---|---|---|
| 0x40000042 fail/death (opened by `Mission_Update` case 4, MissionState 2 fail / 3 dead [M 0x182c60:77-81]) | `P_ENDMISSION_Handler` 0x216618 | row 0: `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` + `Player_RamLoad(Bond, 1)` — the CURRENT level, WARM, entry checkpoint [M 0x216618:19-21]; row 1: BaseMap cold [M :15-17]; row 2: quit → 0x7000048 [M :24-27] |
| 0x10000028 pause | `C_GCPAUSE_Handler` 0x204408 | `ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)` — BaseMap, cold [M :139] |

## Why the warm restart does not loop (the checkpoint answer)

The task asked whether a per-level checkpoint exists. **Yes — the RamSave buffer.**

- `Player_RamSave` (0x195670) runs at the previous level's exit (`Trigger_Activate` 0x1b2318:115)
  and at script checkpoints (`Script_EventHandler` case 0x12, 0x1d94c0:194-199). It saves Bond's
  loadout blocks, `PlrMissionStats`, and **all four switch-channel arrays** — `switch_channels`/
  `_hold`/`_prev` are 0x26fc90/0x26fd90/0x26fe90 and `switch_channels_time` 0x270190 [M symtab;
  saved at 0x195670:12-15, restored at 0x1957a8:17-20].
- `Player_RamLoad` (0x1957a8) is gated on `IsWarmReset`; the fail menu sets it to 1, so the
  checkpoint comes back — with ch 99/100/0x62 **clean**, because they were saved before this level
  ever failed. The dying state is never captured.
- That is why the warm restart cannot re-fail: the replay loop fix-death-restart feared ("the saved
  ch 99 re-fails the mission") only happens if you carry a capture taken AT the fail — which the
  game never does. The port's old warm path did exactly that, which is why it had gone cold.
- The boost arg 1 floors the health at the difficulty's `ContinueHealthBoost*` — **50.0 on all
  three difficulties** [L ee_start_05.bin 0x30cc34-0x30cc3c]. `Mission_Init(level, 1)` re-derives
  `BaseMap`/`ThisOrderNum`, resets the current+future parts' objective states + condition channels
  [M 0x182270:68-129]. `Player_InitWeapon` re-runs 07000007's entry case: fixed party loadout,
  armour 0 [M 0x1a7ef8:176-186]. A restart of a cold-loaded level (no checkpoint) restores the BSS
  zeros and still floors the health at 50.0.

## The fix

- `crates/nf-game/src/death.rs` — `fail_choose_restart` now targets `Drones::level_current`
  (GameState+0xC, latched where `Mission_Init` runs, drone.rs) with `level_exit_cold = false` and
  the new `level_exit_restart = true`; module + fn docs rewritten to P_ENDMISSION.
- `crates/nf-game/src/drone.rs` (shared, additive) — `level_exit_restart: bool` and
  `level_current: u32` fields (own lines), the level latch at the mission-init site.
- `crates/nf-view/src/lib.rs` (shared, additive) — `App::entry_save: Option<RamSave>`: the save each
  level was LOADED with (the game's persistent buffer); the reload handler re-serves it on a restart
  with the 50.0 floor (zeroed at the live shapes when the level was cold-loaded), keeps capturing
  fresh at trigger exits, and clears it on cold loads.
- `crates/nf-view/src/menu.rs` — the Results page rows moved to the game's arm order (row 0 = restart
  level, warm; row 1 = restart mission, BaseMap cold; row 2 = quit); the old order sent arm 1 from
  row 0. Labels are [S] (the disc's menu data was not parsed).
- `docs/systems/mission.md` — §4's Restart event row and §8 rewritten (handler routing, the three
  arms, the checkpoint semantics); the outdated cold-restart paragraphs corrected.

Shared-file touches (all additive, listed per the task): `drone.rs` (2 fields + 1 latch line),
`lib.rs` (1 field + the reload branch), `menu.rs` (Results rows). No other shared files touched.

## Verification (NF_ISO; slim ISO, 0 disc_golden failures counted below)

- Reproduced first: `fix_restart_target_live.rs::fail_restart_reloads_the_current_level_warm` FAILED
  before the fix (`left: Some(0x07000005)` cold — the user's symptom), PASSES after: the punched-guest
  fail path (ch 0x60 → ch 99 → MissionState 2 → the menu → auto-Restart) targets `Some(0x07000007)`,
  warm, with `level_exit_restart`.
- `restart_restores_the_entry_checkpoint_with_the_health_floor`: the full reload — the entry
  checkpoint's health/armour (floor 50.0), the fixed party loadout, armour 0, ch 99/0x62 clean, the
  mission playing, and **no re-fail over 360 ticks** (the loop check).
- `fix_death_restart_live.rs` re-pointed at the corrected model (8 tests green): the in-sim fail
  restart, the dying-carry loop mechanism (still demonstrated — why the game re-serves the entry
  buffer), the cold-loaded level's zero-buffer + 50.0-floor restart vs a clean load, the death
  sequence stages, the no-skip fade, the menu API, the 07000011 fall (now the warm arm).
- `menus_flow.rs` (9 tests green): the Results rows assert the corrected arms.

## What's left

- The fe Results-page restart (menu flow → `requested_level`) loads the right level but does not set
  the sim's `level_exit_restart` flags — the in-sim path (auto_restart / `fail_choose_restart`,
  what play uses on death/fail) does. The fe path needs the intent carried through `pending_level`.
- The exact on-screen row labels ("Restart level" etc.) are [S]; the disc's menu resource data was
  not parsed.
- Script checkpoints (case 0x12 mid-level `Player_RamSave`) are not ported as checkpoint events; the
  port's entry checkpoint is the level-load save only.
- The `Mission_BaseMapHCode` multiplayer arm (current level) is untested (no MP run).

## Live checks for the main agent

1. PCSX2, level 07000007 (reach it through 05→06→07 so the carry exists): punch a guest (or fail any
   objective) → the "Mission Failed" banner ~5 s → the menu → pick the first row. **Expected: the
   level reloads (07000007), Bond at the level start with the 07 loadout (P2K etc.), health = the
   level-entry health but never below 50, armour 0.** Address anchors: the Restart row is
   `P_ENDMISSION_Handler` 0x216618 arm `__Menu_Send`-returns-0; watch `IsWarmReset` (0x30cb98-adjacent
   BSS — the symbol table lists it) = 1 and `GameState+0xC` (0x2a37a8-adjacent GameState block) =
   0x07000007 across the load.
2. Same spot, then open the PAUSE menu (Start) and pick Restart: **expected: the mission's first map
   (07000005) cold** — the two menus behave differently; the port now matches both.
3. nf-view renders (no GPU here): `--bond` on 07000007 near a guest, fail it, screenshot the fail
   menu (row order/labels) and the post-restart level start.
