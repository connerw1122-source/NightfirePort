# mission-unlock-flow — report

Branch `s/mission-unlock-flow`. Task: verify and complete the campaign progression end to end —
a completed mission (the results page's commit, `P_NFRESULTS_Handler` 0x208e78, the progress
layer `crates/nf-game/src/progress.rs`) unlocks the next Single Player row (`sp_level[i]+0x10`,
`FrontEnd::unlocked`), saves to the active codename, and the list shows it after a restart;
"Play without saving" keeps it session-only; the driving rows (0x0900000x) documented.

**The full evidence doc is `docs/systems/mission-unlock-flow.md`** (addresses + [M]/[D]/[L]/[S]
tags throughout). This report is the short version.

## What was already in place (verified, not rewritten)

The results commit (`FrontEnd::results_enter` = 0x208e78:77 `PlrStat_GetScore(0)` + :169
`Menu_SetLevelBonus`), the award rule (`Menu_SetLevelBonus` 0x1ffa10:62-66 sets row **i and i+1**,
hash-agnostic, so driving rows unlock like any other; 0x0700001b → game complete), the status
word (`Menu_GetNightfireStatus` 0x202360: bit i = sp_level[i]+0x10; `Menu_SetNightfireStatus`
0x2023a0: load 0 → 0x3), the codename save/load (`save_profile` packs the bits into the MSSN
block; `load_codename` unpacks; the 0→0x3 rule), the SP list gate (`unlocked_row` with
`menu_unlock_everything`), and the results-page save after the award. All re-checked against the
decompile this session (0x208e78 case 0x4c + case 0x50's save box, 0x202360/0x2023a0, 0x202750,
0x1d3800, C_SBNFMAP 0x20b8d8 case 0x4b).

## Gaps found and fixed

1. **The boot unlock word was 0, the disc says 0x3.** `Progress::default()` had
   `nightfire_status = 0`; the shipped ACTION.ELF `.data` bytes for `sp_level` hold rows 0 and
   1's +0x10 = 1, rows 2-11 = 0 [D, file offset 0x1d91e0 — byte-dumped this session]. With the
   old default, the first award's mirror (`results_enter` mirrors the progress bits into
   `FrontEnd::unlocked`) DROPPED row 0, and the secret codes' XOR cheats landed on the wrong
   base. Fixed the default to 0x3 with the [D] citation; updated the two `secret_codes` unit
   tests that had encoded the wrong boot value (their expectations now match the game: POWDER at
   a fresh boot reads 0x3|0x4, not 0x4). Two prior test files patched around this per-test with
   `set_nightfire_status(0x3)` — those keep passing unchanged.
2. **Unlock bits regressed between missions.** The port splits the game's ONE `sp_level` global
   across the view (`FrontEnd::unlocked`) and the per-level sim (`Progress::nightfire_status`,
   recreated at every level load); nothing carried the session's bits into a fresh sim, so a
   second completed mission's results mirror reset the list to boot + that award. Fixed with the
   host seed in `swap_in_scene` (`crates/nf-view/src/lib.rs`): each fresh sim's status is seeded
   from the live front end via the new `FrontEnd::packed_unlocks()` (the 0x202360 packing). [S]
   the glue — the game needs none because the store never splits.
3. **The SP confirm on a driving row attempted a bogus level load.** The confirm posted
   `requested_level` for every row; a 0x090000xx hash has no level archive on the ISO [D] and the
   load failed ("level: failed to load 09000001.bin"). The game splits the confirm [M C_SBNFMAP
   0x20b8d8:29-40]: on-foot → `ResetMap_LevelToLoad` + the 60-frame fade + menu delete; driving →
   `level_chosen` only, then `ResetMap_LevelToLoad` pushes **GameFlow state 9** (the DRIVING.ELF
   handover) [M 0x1d3800:104-108, gated by `Menu_IsDrivingLevel` 0x202750 = exactly
   {0x0900001,2,3,5,6}]. The port now models the split: `reset_map_level_to_load` has the driving
   branch (push ST_LAUNCH_DRIVING = 9, GameState+8 = hash, no story-code wrap — the wrap switch
   is inside the on-foot arm only), the confirm sets `requested_level` only for on-foot rows and
   logs the handover otherwise, and `FrontEnd::launching_driving()` + `nf_game::ptp::is_driving_level`
   are the getters (the flow holds at 9 — states {1,3,4,5,9} skip the draw — since the port has no
   DRIVING program; the PTP block, drive-exit record and return leg were already modelled for the
   in-mission interludes in nf-game/src/ptp.rs).
4. **Doc fixes**: the stale comment claiming driving hashes are "none in the campaign rows"
   (menu.rs) — five are [L]; the SP-confirm comment now cites the real split.

## The driving rows / PTP handover / 502

- Rows 0, 2, 3, 8, 9 are the driving hashes 0x09000001/05/06/02/03 [L ee_start_05 0x2df2e0 dump].
  Unlock: the same 0x1ffa10 hash scan. Play: C_SBNFMAP confirm → level_chosen → P_NFMAP iris →
  ResetMap_LevelToLoad → state 9 → `Boot_GetPTPData` 0x1c8198 (PTP+0x14c = hash, PTP+0x974 =
  hash−0x9000000) → `psiLaunchDriving` 0x120af0; DRIVING.ELF has no menu/sp_level code at all and
  returns via BASE → ACTION's boot `Boot_LoadPTPData` 0x1c8560 → `ResetMap_LevelToLoad(0x7000048)`
  → the results page reopens on a status-6 return and the award fires there.
- **502 is unresolved**: `rg "\b502\b|0x1f6|0x502"` over all three decomp programs = zero hits;
  the port's own matches are unrelated (an entity count, a wrap width, a model hash, a float). It
  is not a game constant this flow touches.

## Verification (what I ran)

- `cargo test --offline --release -p nf-view --test mission_unlock_flow` — **4/4**: the required
  headless completion chain (rows 1, 2, 3 complete → the unlock word goes 0x3 → 0x7 → 0xf → 0x1f
  with no regression of earlier rows — the test asserts the exact masks and that row 5 stays
  locked), the MSSN save → fresh program → load round trip (0x1f survives), play-without-saving
  (bits accumulate, no card write, fresh program = boot 0x3), the SP confirm split (driving →
  state 9 + no load request; on-foot → the loader + request), and the 0x202750 gate set. Each
  assertion has a can-fail sister.
- `cargo test -p nf-game --lib` — **212/212** (the 2 secret_codes updates above).
- Neighbours my changes touch: nf-view `menus_flow` 37/37, `codename_entry` 14/14,
  `fix_save_profile_live` 4/4, `campaign_flow_live` 5/5 (ISO-gated ones with NF_ISO set).
- The background fast suite (`tools/test/suite.py`) died twice in this sandbox: first run
  `FileNotFoundError: 'cargo'` (PATH), restarted with the PATH fixed it produced no total line
  before the window closed — the full-suite total line is NOT available; the next window should
  run it on this tree. nf-formats/nf-audio untouched by this task (no re-run needed beyond the
  suite).

## Left / notes for the main agent

- The SP confirm's fade push + menu-manager delete (0x20b8d8:35-39) are still not modelled — the
  menu comment cited them but no code pushed them; I left the merged behavior alone since it
  needs a render pass to verify safely. Game order: ResetMap pushes 3, then fade 7 on top.
- The difficulty choice is likewise never seeded into the sim (`Drones::difficulty` stays the
  boot 2) — pre-existing, belongs to the difficulty flow's owner.
- "New Codename" (name entry) is still reported as not ported (codename-entry task owns it).

## Live checks for the main agent

1. gdbwatch 0x2df2f0 (`sp_level[0]+0x10`) and 0x2df308 (`sp_level[2]+0x10`): complete the first
   on-foot mission (07000005) — expect byte writes 1 from `Menu_SetLevelBonus` 0x1ffa10 while the
   results page is up, then the SP list offering row 2.
2. gdbwatch 0x2df300 on "Play without saving" + complete a mission: expect the award write, NO
   LS_Save/card activity, and after a soft reset the list back at rows 0+1.
3. fieldprobe the SP list after loading a codename whose MSSN = 0x1f: rows through 4 offered,
   rows 5+ locked (Txt 0x01000009 descriptions).
4. Confirm row 0 of the SP list (0x09000001): the real game walks the iris then switches
   programs; the port should hold the last menu frame (draw skipped) and log
   `sp list: driving level 09000001 … not ported`.
