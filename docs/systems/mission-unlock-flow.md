# The mission-unlock flow (completing a mission unlocks the next SP row)

**Port status: VERIFIED + COMPLETED (mission-unlock-flow, 2026-10-16).** The chain — results-page
commit → award → codename save → restart persistence, plus the driving rows' handover — with the
game addresses it implements. Facts are tagged [M] code (function + address + decompile line),
[D] disc, [L] EE snapshot (address + value), [S] suggested.

## The campaign table (`sp_level` 0x2df2e0)

12 records × 0x18 bytes: +0x00 ring-preview texture, +0x04 name label, +0x08 description label
(Txt ids), +0x0c level hash, +0x10 unlocked byte, +0x14 locked-description label [M `C_SBNFMAP`
0x20b8d8:18-21 reads +0xc/+0x10; `Menu_UpdateWheel` 0x1f8388 binds +0x08 when unlocked and +0x14
when not]. The records, byte-dumped from the snapshot ([L] ee_start_05.bin, file offset = EE
address 0x2df2e0 — a post-unlock session, so its +0x10 bytes read 1 everywhere) and the hashes
plus the BOOT +0x10 bytes from the shipped ACTION.ELF .data at file offset 0x1d91e0 ([D] rows
0-1 = 1, rows 2-11 = 0):

| row | +0x04 name | +0x08 desc | +0x0c level hash | boot +0x10 | kind |
|---|---|---|---|---|---|
| 0 | 0x03000001 | 0x03000013 | 0x09000001 | 1 | driving |
| 1 | 0x03000002 | 0x03000014 | 0x07000005 | 1 | on-foot |
| 2 | 0x03000003 | 0x03000015 | 0x09000005 | 0 | driving |
| 3 | 0x03000004 | 0x03000016 | 0x09000006 | 0 | driving |
| 4 | 0x03000005 | 0x03000017 | 0x07000001 | 0 | on-foot |
| 5 | 0x03000006 | 0x03000018 | 0x07000009 | 0 | on-foot |
| 6 | 0x03000007 | 0x03000019 | 0x0700000c | 0 | on-foot |
| 7 | 0x03000009 | 0x0300001b | 0x07000011 | 0 | on-foot |
| 8 | 0x0300000c | 0x0300001e | 0x09000002 | 0 | driving |
| 9 | 0x0300000e | 0x03000020 | 0x09000003 | 0 | driving |
| 10 | 0x03000034 | 0x03000035 | 0x07000014 | 0 | on-foot |
| 11 | 0x03000011 | 0x03000023 | 0x0700001b | 0 | on-foot |

(Labels are [L ee_start_05]; the +0x14 locked-description label is 0x01000009 on rows 2-11 and 0
on rows 0-1 [L].) A fresh program therefore offers exactly rows 0 and 1 — the first (driving) and
first on-foot mission.

## The award chain (completing a mission)

1. The mission monitor sets `Mission_Status() = 6` before the front end loads [M `Mission_Update`
   0x182c60:85-122]; the results page opens on the front-end map 0x0700048 (the outro rides the
   briefing machinery, phase 2 sets GameState+0x28 = 0x40000036 [M 0x1d6360]).
2. `P_NFRESULTS_Handler` 0x208e78 case 0x4c runs `PlrStat_GetScore(0)` — commit mode [M :77,
   0x1984b0:301-315] — builds the page's fields, compares `last` against the entry's thresholds
   for the medal [M :104-146].
3. **The award**: `just_completed = Menu_SetLevelBonus(GameState+0x72, medal, 0)` [M :169].
   `GameState+0x72` is `Mission_BaseMapHCode` (the just-played hash).
4. `Menu_SetLevelBonus` 0x1ffa10: the medal loop sets the reward-counter bits (only for
   1 ≤ medal < 5; a no-medal run awards no bonus bits, raising `new_reward` when a bit was clear
   [M :70-79 → `PlrStarts_ProcessRewardCounter` 0x199210]); then it finds the row whose +0xc
   equals the hash and sets **`sp_level[i]+0x10 = 1` AND `sp_level[i+1]+0x10 = 1`** [M :62-66].
   The scan is by hash, so it covers the driving rows unchanged. Hash 0x0700001B instead sets
   game complete (`GameState[0x52] = 1`) [M :68].
5. **The save**: the results flow's option box (case 0x50) leads to
   `Menu_UpdateMessageBox(param_1, 0xff, 0)` with `ls = 1` [M 0x208e78 case 0x50] — box mode 1
   writes the block chain under the codename (`LS_MakeMission` 0x1cd720's MSSN block carries the
   32-bit nightfire status = the unlock word; see docs/systems/progress.md §save). A failed card
   pops the retry box (Txt 0x1000206) [M case 0x50 tail]. The fail menu has no save arm.

## The status word (`sp_level[i]+0x10` ⇄ the MSSN value)

- `Menu_GetNightfireStatus` 0x202360: **bit i of the returned word = `sp_level[i]+0x10`**, i =
  0..11 [M the OR loop].
- `Menu_SetNightfireStatus` 0x2023a0: **a 0 value loads as 0x3** (the first two rows), then bit i
  writes `sp_level[i]+0x10` [M the `uVar4 = param_1 != 0 ? param_1 : 3` head]. This is the LOAD
  path's default; the BOOT state comes from the .data bytes themselves ([D] above).
- The results save therefore persists the unlocks as the MSSN status; the same word is what the
  codename load unpacks (`Menu_UpdateMessageBox` case 0 → `LS_Load` 0x1cdfc0 → `LS_LoadMission`
  0x1cd880 → `Menu_SetNightfireStatus` [M the port's mapping in nf-game `progress.rs`]).

## Session vs saved ("Play without saving")

The codename page's row 1 pops the confirm box [M `C_SBNFCN` 0x20b390:19-21 —
`Menu_CreateOptionBox(0x10000bb, 9, 1, 1)` + `Menu_MapDefaultCodename(0, sel, 0xff)`]; the confirm
continues to the difficulty page, the same continuation a saved codename's load takes [M
0x20b390's ≥2 arm → `Menu_UpdateMessageBox(.., 0xff, 0)` mode 0 = load, then the difficulty
page]. With no codename in play there is no card write anywhere in the flow (the save needs the
codename buffer `DAT_002e1ae8`, filled only by a row pick or the name entry [M 0x20b390:31-34]) —
so the awards land in `sp_level+0x10` **in memory only** and a program restart shows the boot
state again (rows 0+1, [D]). The results page's award code is the same code either way; only the
save arm is gated.

## The driving rows (0x0900000x)

- **Unlock**: the same `Menu_SetLevelBonus` hash scan sets a driving row's +0x10 exactly like an
  on-foot row's [M 0x1ffa10:62-66] — completing the row-1 on-foot mission unlocks the row-2
  driving mission, and so on. There is no special driving unlock path.
- **Play**: `C_SBNFMAP` 0x20b8d8 case 0x4b sets `GameState+8 = sp_level[i]+0xc` for any unlocked
  row, then splits [M :29-40]: an on-foot hash runs `ResetMap_LevelToLoad(hash,0,0)` + the 60-frame
  fade push (`GameFlow_PushState(0x42a00000, 7, 0xff)` — 0x42a00000 = 60.0f) + the menu manager
  delete; **a driving hash only sets `level_chosen = 1`** — `P_NFMAP` 0x20b608:15-28 then walks the
  iris and calls `ResetMap_LevelToLoad(hash)`.
- **The handover**: `ResetMap_LevelToLoad` 0x1d3800 tests `Menu_IsDrivingLevel` 0x202750 —
  exactly {0x09000001, 0x09000002, 0x09000003, 0x09000005, 0x09000006} [M the `>0x9000000 &&
  (<0x9000004 || (0x9000004 < && <0x9000007))` band] — and for a driving hash pushes
  **GameFlow state 9** and stores the hash at GameState+8, skipping the story-code wrap (the 0x71
  switch is inside the on-foot arm) [M :104-108]. `GameFlow_Main` 0x1c98c8 case 9 fills the PTP
  block (`Boot_GetPTPData` 0x1c8198: PTP+0x14c = hash, PTP+0x974 = hash − 0x9000000) and launches
  DRIVING.ELF (`psiLaunchDriving` 0x120af0) [M :197-222]. DRIVING.ELF contains no menu/sp_level
  code at all; it exits through BASE (SLUS_205.79) back into ACTION.ELF, whose boot
  (`GameFlow_Main` case 1 → `Boot_LoadPTPData` 0x1c8560) restores GameState+0x72 from PTP+0x14c
  and calls `ResetMap_LevelToLoad(0x7000048,0,0)` — the front-end hub — where a status-6 return
  reopens the results page and the award fires for the driving hash [M; see docs/systems/progress.md
  §hand-off].
- **The port today**: the flow models the state-9 push (`FrontEnd`'s GameFlow, ST_LAUNCH_DRIVING =
  9, from `reset_map_level_to_load`'s driving branch) and stops there — the port has no DRIVING
  program, so the state holds (states {1,3,4,5,9} skip the draw [M 0x1c98c8]) and the host reports
  the attempt through `FrontEnd::launching_driving()`. The confirm sets `requested_level` only for
  on-foot rows (a 0x09 hash has no level archive on the ISO — the old path failed at the archive
  name). The PTP block itself, the drive side's exit record and the return leg ARE modelled for the
  in-mission interludes (nf-game/src/ptp.rs; scene.rs's `ptp_handover`; `DriveReturn::apply`).
- **"502"**: unresolved — `rg "\b502\b|0x1f6|0x502"` over all three decomp programs has zero hits,
  and the port's own 502 matches are unrelated (an entity count, a wrap width, a model hash, a
  float). If it came from a driver note it is not a game constant this flow touches.

## The port's map (what the draw code calls)

| Game | Port |
|---|---|
| `sp_level` 0x2df2e0 rows/`+0x10` | `Progress.nightfire_status` (bit i) — the boot value is the disc's 0x3 [D 0x1d91e0]; `SP_LEVEL_HASHES` is the row order |
| `Menu_Get/SetNightfireStatus` | `Progress::nightfire_status` / `set_nightfire_status` (0 → 0x3) |
| `Menu_SetLevelBonus` 0x1ffa10 | `Progress::set_level_bonus` (row i and i+1; 0x0700001B → game_complete) |
| `FrontEnd::unlocked` | the view's mirror of the status word (`results_enter` / `load_codename` / `save_profile` pack-unpack it) |
| the results commit | `FrontEnd::results_enter` (get_score commit + set_level_bonus + mirror + the profile save when a codename is in play) |
| the SP list gate | `FrontEnd::unlocked_row` (+ `unlock_all` = `menu_unlock_everything`), `first_unlocked`, `step` (locked rows skipped) |
| the driving handover | `GameFlow::reset_map_level_to_load`'s state-9 branch, `FrontEnd::launching_driving()`, `nf_game::ptp::is_driving_level` |
| the session's single sp_level | the host seeds each fresh sim's `progress.nightfire_status` from the live front end at the scene swap (the game keeps ONE global; the port splits it across the sim and the view) [S the port's glue] |

Tests: `crates/nf-view/tests/mission_unlock_flow.rs` — the headless completion chain over the
first three rows (unlocks in the game's order, no regression of earlier rows, MSSN round-trip
through save/load), the play-without-saving session rule, and the driving-row confirm's state-9
handover with the `Menu_IsDrivingLevel` gate.

## Live checks for the main agent

1. gdbwatch 0x2df2f0 (sp_level[0]+0x10): complete the first on-foot mission in PCSX2 — expect a
   byte write 1 from `Menu_SetLevelBonus` (0x1ffa10) while the results page is up, and the SP list
   afterwards showing row 2.
2. gdbwatch 0x2df300 (sp_level[1]+0x10) on "Play without saving": expect the same write on
   completion, NO `LS_Save`/card activity, and after a soft reset the byte back at 1 (the boot
   value) with row 2 locked again.
3. fieldprobe the SP list after loading a codename whose MSSN = 0x1f (rows 0-4): the list should
   offer through row 4 and show rows 5+ as locked (Txt 0x01000009).
