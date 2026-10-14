# fix-save-profile — the codename profile save

Branch `s/fix-save-profile`. The task: port the game's profile save — what the memory-card save
holds (unlocked missions, the options, the controller style, the cheats, the Dossier's Records
data) with the layout from the game's own save code — as one file per codename next to the
port's config, with the hooks for creating a codename, saving after a mission, loading on the
codename page and the Dossier Records reading it.

## The game's model (the evidence)

- **The block table**: `SaveIFFBlocks` 0x2c74d8 = 6 records {mask, id, make, load} — PLRS
  (mask 1, make `LS_MakePlrSettings` 0x1cd418), MSSN (2, 0x1cd720), MPSG (4, 0x1ce508), GSET
  (8, 0x1cd9d0), CHET (0x10, 0x1ce260), BNUS (0x20, 0x1ce3b0) [M the static init
  0x1cdd50; **L** the table bytes in `work/ee_start_07000001.bin` @ 0x2c74d8; NumSaveBlocks
  0x30cf10 = 6]. `LS_GetSaveStuff` 0x1cd0d8 concatenates the blocks in this order into a
  per-slot 0x1000 buffer (`DstData`/`InBufData` + slot·0x1000 [M 0x1ce0c0/0x1cdfc0]).
- **The file name IS the codename**: `Menu_UpdateMessageBox` 0x1f7de8 passes the codename
  buffer 0x2e1ae8 to `LS_Load`/`LS_Save`; the name buffer is filled by
  `Menu_MapDefaultCodename` 0x1f7fa8's `Txt_BindLabel(def_codename[i])` and by the keyboard.
  **L** the snapshot's buffer holds "BOND". The port stores `<profiles dir>/<codename>.profile`.
- **The game's own chain** — the loaded profile sits in `InBufData` 0x3416e0 in every ee_start
  snapshot (**L**, 1022 bytes): PLRS 11 B, MSSN 68 B, MPSG 13 B, GSET 905 B, CHET 9 B, BNUS
  16 B. Each block is `{u32 id; u32 size; payload}` with size counting the 8-byte header
  [M the make functions write `(bits+7)>>3` at buf+4]. The bit packer is the progress layer's
  LSB-first `BIN_PushBits` 0x1e8b88.
- **PLRS** (19 bits → 3 payload bytes), push order [M 0x1cd418; the decode of the game's own
  chain matches `Input_Init` 0x17edd8's defaults field for field]:
  `PlayerSetting[slot]` (0x2a38c8, stride 0x158, 4 slots [M symtab size 1376]): +0x00 exists
  (1 b; [M P_PAUSE_Handler 0x203c28:80 tests it]), +0x0e controller style (4 b; [M
  `Input_ChangeControllerStyle` 0x17f5f0 writes it, `Input_Update` 0x17ee88 reads it as a
  short]), +0x10 PTP option (4 b; [M Boot_GetPTPData 0x1c8198:81 only]), +0x01 auto aim (1 b;
  [M `Player_AutoAim` 0x1a7c98:41]), +0x02 its variant (1 b; [M 0x1a7c98:44]), +0x03 manual
  weapon action (1 b; [M `Player_Weapon` 0x1a24c8:45]), +0x0a auto-equip best (1 b; [M
  `Player_EquipWeapon` 0x1a8a58:76]), +0x04 crouch toggle (1 b; [M `Player_SSCrouch`
  0x1924c0:85]), +0x09 rumble (1 b; [M `Input_RumbleStart` 0x17f6b8:9]), +0x08 crosshair (1 b;
  [M `HUD_UpdateCrossHair` 0x19e6d0:96]), +0x0c a 2-bit enum (init default 1 [M 0x17edd8:14]),
  +0x0b a 1-bit gate on Player_Update's decaying timer (1 b; [M 0x193600:414]).
- **MPSG** (33 bits): `MPSettings[slot]` (0x2a47a0, stride 0x30 [M 0x1ce508]): +0x28 radar
  (1 b; [M `MP_GetRadarObjects` 0x189840:22]), +0x2c extra spawn health (32 b; [M
  `Player_Init` 0x1910f8:129 adds it to 100.0]).
- **GSET** (7175 bits → 905 B) [M 0x1cd9d0]: music volume 7 b, sfx volume 7 b, language 7 b,
  subtitles 1 b (`GameState+0x60` 0x2a37c8 — [M `Sound_DoSubtitle` 0x1dde58:21]), SFX mode
  32 b, widescreen 1 b (`GameState+0x5c` 0x2a37c4 — [M `C_CHCHWS_Handler` 0x216168:9 copies it
  to `switch_CONFIG_WIDESCREEN`]), `DrawInfo` 0x2a37c0 32 b, screen pos x/y 2×32 b
  ([M psiGetScreenPos/psiSetScreenPos]), 219 zero dwords, then the constant **4** as 16 b.
- **CHET** (3 bits): `CheatInfo` 0x2a3738 bit 0, 0x2a373c (read by `Player_InitAmmoWeapons`
  0x1a17f8:20 [S ammo cheat]), 0x2a3740 (read by `Player_RoundToFire` 0x1a18e0:24 [S fire
  cheat]). `Menu_MapDefaultCodename` resets all three on any map [M 0x1f7fa8].
- **The "New Codename" template**: def_codename 0x2e1b18 = 2 records of 0x38 B, +0x00 = the
  label id (0x29b "New Codename" row, 0x211 [M `Menu_EnumSaves` 0x1f7b78 reads both;
  `Menu_PutCodenamesIntoControl` 0x1f79c0 overwrites ROW 1 with Txt 0x10002ff "Play without
  saving" on this page]) [**L** the template bytes: nightfire 0 (→ 0x3 [M
  `Menu_SetNightfireStatus` 0x2023a0]), bonus 0, music 70, sfx 100, the option bytes
  (style 7, auto aim 1, crouch toggle 1, rumble 1, crosshair 1, auto equip 1, aux 1),
  MP radar 1, draw_info 2]. `Menu_MapDefaultCodename` maps it onto a slot per a mask.
- **The codename validator**: `Menu_IsValidCodename` 0x1f9930 — trims spaces, uppercases and
  writes the uppercased name back [M the strupr + `strcpy(param_1, acStack_a0)`], rejects the
  empty name (0x3bc), names that contain a default codename's text (0x2c0, strstr over the two
  labels), and the 32-byte copy bounds the length.
- **Difficulty is NOT in the save**: none of the six make functions reads `GameState+0x40`
  0x2a37a8 [M all six checked]; the task brief's "difficulty per mission" item resolves to the
  MSSN per-mission best (which embeds the difficulty multiplier at the time, [M
  `PlrStat_GetScore` 0x1984b0:294-300]) — the difficulty itself boots at 2 every session
  [M 0x1c7e78:25]. The port matches (nothing to save).
- **The Records page** reads per level: the best score (`ScoringTable+0x30`), the moments
  value, the bonus count [M `C_RBDSRECORDS` 0x20bf00:26-28 sprintf's `PlrStats_GetLevelTotals`
  +0x30 / the moments / `Menu_GetLevelBonuses`].

## What's done

1. **`crates/nf-game/src/profile.rs`** (new): `PlayerOptions` / `MpOptions` / `GlobalOptions` /
   `Cheats` with the exact push/pull orders above; `Profile { name, plr, mpsg, gset, chet,
   progress }` — `encode()` emits the six blocks in `SaveIFFBlocks` order + the CRC-32 trailer
   (the same file convention as `Progress::encode_save`; the LS-layer buffer itself is the bare
   chain [L InBufData]), `decode()` verifies the CRC and applies every block;
   `default_codename()` = the template; `check_codename()` = the validator; `save_to_dir` /
   `load_file` / `list_dir` / `path_in` = the port's memory card (one file per codename next to
   the port's config — the profiles dir the host points `FrontEnd::set_profile_dir` at).
2. **The menu hooks** (`crates/nf-view/src/menu.rs`): `FrontEnd.profile_dir/profiles/
   active_profile/profile`; the codename page grows the card's rows after the two defaults
   [M `Menu_EnumSaves` 0x1f7b78:24-38] with the names as labels; `create_codename` (validate →
   template → file → active), `load_codename` (the Confirm on a saved row: load, apply the
   nightfire status as the unlock mask [M 0x2023a0], continue to the difficulty page [M
   C_SBNFCN 0x20b390]), `save_profile` + the `mission_ended(Complete)` hook (the results save
   [M P_NFRESULTS → 0x1f7de8 case 1]; the fail menu has no save arm [M 0x216618]),
   `sync_profile_progress` (the host merges the sim's tally/medals before the save),
   `record_row` and the Records overlay now drawing the active profile's best/moments/bonus
   instead of the hardcoded 0.
3. **progress.rs**: `make_mssn`/`load_mssn`/`make_buns`/`load_buns` became `pub(crate)` and
   `Progress` gained `#[derive(Clone)]` (both additive — the profile composes the MSSN/BNUS
   blocks instead of duplicating them).

## How it was verified

- **Module tests** (`profile.rs`): `decodes_the_games_own_chain` — the game's own 1022-byte
  chain from `ee_snapshot.bin` InBufData decodes to PLRS = `Input_Init`'s defaults (exists 0,
  style 0, ptp 0, auto aim 1, alt 0, reload 0, equip 1, crouch 1, rumble 1, crosshair 1, aux 1,
  decay 0), MPSG radar 1 / health 0, GSET music 70 / sfx 100 / language 6 / screen 0,0 /
  version 4, CHET all 0, BNUS 0x0400_0000_0000_0040, MSSN status 0xfff [**L** every value];
  `block_sizes_match_the_game` (11/68/13/905/9/16 → 1022); the round trip + the file API; the
  CRC/truncation rejection; the validator's rules. **The pre-fix state**: only MSSN/BUNS were
  modelled (`Progress::decode_save` silently dropped PLRS/MPSG/GSET/CHET — the options had no
  port representation), and there were no codename files at all; the new tests fail to compile
  against that tree (no `profile` module, no `FrontEnd.profile_dir`).
- **`crates/nf-view/tests/fix_save_profile_live.rs`** (NF_ISO, the real flow): create → the
  file with the game's block sizes + the template values; the game's own chain loads through
  the codename page's real action path (Down Down Confirm → the difficulty page, the unlock
  mask, the Records values == the decoded profile's); completing a mission through
  `nf_view::load("07000005.bin")` + `Sim::step_bond` + `mission_ended(Complete)` unlocks row 1
  and the file survives a fresh session's load; sisters: the FAILED path writes nothing and
  unlocks nothing, a corrupt file is refused and leaves no active profile. 4/4 pass
  (13:17 UTC, on the merged tree).
- **Suite**: the rule-5 command on the final tree (origin/master c606b5f merged, --test-threads=1,
  run in foreground chunks — the background runner was killed between tool calls three times in
  this sandbox even under setsid/nohup, so `suite.py` could not survive; the same per-binary
  coverage was reproduced by hand): **1003 passed, 3 failed** — the failures are exactly the 3
  allowed `disc_golden` tests on the slim ISO; `weapon_matrix_live` (the pre-existing >600 s
  case the WORKLOG already flags) was skipped by name — the suite records it TIMEOUT as before.
  Environment note for the next worker: this sandbox ships no Vulkan ICD, so every render test
  panics with `wgpu adapter NotFound`; `apt-get download mesa-vulkan-drivers` + extracting it
  into a user prefix and pointing `VK_DRIVER_FILES` at a path-patched `lvp_icd.json` (the
  alsa-prefix pattern, kept at `../vk-prefix/`) fixes all of them — fix_0c_pylon_live then
  passes 2/2.

## What's left (honest list)

1. **The keyboard page** (name entry) is not ported — `action()`'s row-0 Confirm stays a no-op;
   `create_codename(name, reserved)` is the hook the keyboard will call. The two reserved
   labels (def_codename 0x29b/0x211) should resolve through the language bank at the call site
   (the game strstr's the RESOLVED strings [M 0x1f9930]).
2. **MemCard_ASK_OVERWRITE / ASK_DELETE flows** — the port refuses an existing codename
   (`CodenameError::Exists`); the game pops the overwrite box [M 0x10ee38]. The host decides.
3. **The 0xa0-byte custom button table** (`PlayerSetting+0x14`, cleared by
   `Input_ChangeControllerStyle` 0x17f5f0) is NOT part of PLRS — the game's save loses custom
   remaps too [M the 19-bit push]. Nothing to port; noted so nobody "fixes" it.
4. **The MC directory furniture** (icon/header sectors) stays out — as fix-progress decided;
   the port's file is the block chain + CRC.
5. **`record_row`'s third value** counts the medal tiers 1..=4 via `has_medal`; the game's
   `Menu_GetLevelBonuses` (not decompiled here) may count the raw bonus bits instead [S] —
   a render/live check will settle it.

## Live checks for the main agent

In PCSX2 (NTSC-U):
1. Codename page (front end → NightFire → Select Codename): create a new codename named
   "TESTONE", then immediately save after the first mission completes. Expected: the memory
   card holds a file named after the codename; gdbwatch on `InBufData` 0x3416e0 during the save
   should show the chain PLRS/MSSN/MPSG/GSET/CHET/BNUS with sizes 11/68/13/905/9/16 + the
   codename as the file name.
2. Records (Dossier → Records) on a profile with a completed mission: the three value rows
   (best score / moments / bonuses) vs the port's `record_row` for the same profile; whether
   the third row counts medal tiers or raw bonus bits (my [S] above).
3. The controller-style byte: change the control style on the options page, save, reload the
   profile, and check `PlayerSetting[slot]+0x0e` (0x2a38d6+slot·0x158) changed to the picked
   style — and whether a fresh "New Codename" really boots with style 7 (the template says 7
   [L], the snapshot's runtime profile says 0).

In nf-view (`--shot` renders):
- `--page 0x40000025` (codename) with `profile_dir` set to a dir holding 2 profiles: 4 wheel
  rows — "New Codename", "Play without saving", then the two file names as rows 2/3.
- `--page 0x4000003a` (Records) with an active profile whose mission 1 has a best score: the
  value column at (375,169/202/235) shows the numbers, not "0".
