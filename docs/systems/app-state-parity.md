# App state parity — the fields the game updates per load or per menu choice

Task `app-state-parity` (2026-10-17). `App::level_code` was the found instance of this bug class
(initialised from the command-line level, never updated by a load — fixed 2026-10-16 in
`poll_async_load`). This audit walks the rest of the `App` initialiser
(`crates/nf-view/src/lib.rs`, `run()`'s `App { ... }`) and asks for every field: where does the
GAME keep it, which event updates it, and does the port update it on the same event?

The game's stores live in ACTION.ELF's globals: `GameState` 0x2a3768 (0x58 bytes, memset at boot
[M bootup 0x1c7e78:14]), plus the manager-outliving statics (`CheatInfo` 0x2a3738,
`PlayerSetting` 0x2a38c8, `SelLang` 0x30cee0, `menu_unlock_everything` 0x30d2a7, the PTP block
0x31c090). Evidence tags: [M] decompile function:line, [D] disc bytes, [L] EE snapshot/live, [S]
suggested.

## The audit table

| Field (port) | Game source | Game writers | Was it stale? | Fix |
|---|---|---|---|---|
| `selected_difficulty` | `GameState+0x28` (0x2a3790) [M 0x1c7e78:25 boot = 2] | difficulty page confirm [M C_SBNFDFCTY 0x216ca8:15]; **MP confirm = 1** [M P_MPCONFIRM 0x20ec00:150]; mini-mission = 1 [M Menu_RunMiniMission 0x1ffb90:22]; driving return [M Boot_LoadPTPData 0x1c8560:36] | **YES — the MP start never wrote it** (the App kept the SP pick across a match) | `mp_confirm_start`/`mp_client_start` raise `requested_difficulty = Some(1)`; the App adopts it in `menu_after` like the difficulty page's pick |
| driving-return difficulty | the PTP block +0x144 ← `DAT_0031c1d4` [M 0x1c8198:18-20, mirrored 0x1c8560:35-36] | `Boot_LoadPTPData` writes `GameState+0x28` from the block on EVERY driving return | **YES — `DriveReturn` dropped the field; the new sim re-derived it from `selected_difficulty`** (equal today only because the App never changes it; the base-special mini-mission case pins 1 [M 0x1c8198:15]) | `DriveReturn.difficulty` carried from the block's +0x144 (`mission_state_1`), applied to `Progress::difficulty` + `Drones::difficulty` in `swap_in_scene` |
| mission row | the front end's row lives in the page's list control, re-derived at every open: the mission list opens on the HIGHEST unlocked row [M P_NFMAP 0x20b608:57-64]; the Dossier's Records list opens at `GameState+0x48` (the last completed mission's BaseMap [M P_DSRECORDS 0x217248:13; writers: boot 0 [M 0x1c7e78:31], mission-select confirm 0 [M C_SBNFMAP 0x20b8d8:30], mission end = BaseMap [M Mission_Update 0x182c60:69], driving return [M 0x1c8560:23]) | per open (0x4c/0x1e messages) | no — the port re-derives the mission-list cursor at every entry (`highest_unlocked`), and the pause's fresh manager is seeded from the mission's own BaseMap (`sim.drones.mission.base_map` → `fe.mission`, lib.rs `run()` Esc arm); the results page reads `ended_level`/`flow.current_level`, not the row | none |
| `entry_save` | the ENTRY checkpoint = the RamSave buffer 0x2b9440, re-saved on EVERY load [M Player_InitWeapon 0x1a7ef8:325's unconditional `Player_RamSave`] | every load | no — `swap_in_scene` re-captures after every load (cold included) and `run()` captures the command-line load | none |
| ptp save | the PTP hand-off block 0x31c090 (0xa50) [M symtab]; `Boot_GetPTPData` fills it leaving for DRIVING, `Boot_LoadPTPData` mirrors it back | the two boot helpers | partially — the status/stats rode `DriveReturn`; the difficulty did not (see row 2) | `DriveReturn.difficulty` |
| `all` (`--all`) | no game-side store found: the game's draw set is fixed at load (the cels' own flags); nothing in the decompile writes a draw-all toggle per load or menu choice [M-negative: whole-decompile grep for the 0x8000-cel gate's writers] | — | no — a port debug flag; persisting across reloads is the port's intent [S] | none |
| cheats | `CheatInfo` 0x2a3738 (0x30, boot memset [M 0x1c7e78:15]) + `menu_unlock_everything` 0x30d2a7; written by the cheats page's spinners [M C_CHCHHEALTH 0x215ee0:9, C_CHCHUNLOCK 0x2164c0:9], cleared when the default codename is mapped [M Menu_MapDefaultCodename 0x1f7fa8:76], carried in/out of the PTP block [M 0x1c8198:36, 0x1c8560:50/53], saved with the profile [M LS_MakeCheats 0x1ce260] | the cheats page (unported) | no port event exists: the port has no cheats page (the game's own toggles are unreachable); `UNLOCK_ALL` is the boot-time `--locked` stand-in [S]. Nothing to update until the page lands — the writers are listed here for it | none (documented) |
| the sim's settings | `PlayerSetting` 0x2a38c8 — 0x158-byte blocks per player; +0x0 is the look-invert, read EVERY tick [M Input_Update 0x17ee88:50]; toggled live by the pause's controls page [M C_GCPAUSE 0x204408:526-527] and the Dossier's [M C_RBCONTROL 0x217908:15]; loaded from the card [M LS_LoadPlrSettings 0x1cd5b8] | the two controls pages | no port event exists: the port has no controls page yet; the sim-side flag exists (`PadConfig.invert_look`, nf-game input.rs) and boots 0 exactly like the game's memset [M Input_Init 0x17edd8:13]. The AV volumes DO reach the driver live (`apply_av_volumes` at boot and the pages' confirm [M LS_LoadGlobalSettings 0x1cdbc8:69-70]) | none (documented) |
| the sound bank | the level's bank loads with the level [M ResetMap_Load's SFXLoadSoundBank path]; the front end alternates SB_0/SB_1 through `DAT_002e1b11` [M Sound_Ready 0x1dceb8:94-101] | every load | no — `build_sound` runs on every scene swap (`swap_in_scene`, and the warm reload path) and the front-end flip re-points the driver's primary (`poll_async_load`'s FRONTEND_LEVEL arm) | none |
| the language | `SelLang` 0x30cee0 [M symtab]; `Txt_LoadLanguage` 0x1caad0 loads `FNames[SelLang]` [M :64]; writers: `Txt_SetLanguage` [M 0x1cad38] from the language page (not on this disc's flow) and the driving return [M 0x1c8560:88-89]; the disc carries USATxt only [D FILES.BIN] | `Txt_SetLanguage` | no — the port loads USATxt once at boot (`App.lang`) and per load into the sim (`load_impl`), which matches the NTSC-U disc's fixed language | none |

## The fixes

1. **The MP confirm's difficulty write** (`crates/nf-view/src/menu.rs`): the game's confirm arm
   sets `GameState+0x28 = 1` before the level request [M P_MPCONFIRM 0x20ec00:150 — the write sits
   between `MPSettings._384_4_ = 1` and `GameState._8_4_ = MPSettings._424_4_`]. The port's
   `mp_confirm_start` (and the client's `mp_client_start` handover, which mirrors the same tail
   [S]) now raise `requested_difficulty = Some(1)`; the App's existing `menu_after` take adopts it
   into `selected_difficulty`, so every following load — including the next SP mission, which the
   game would also run at 1 until the difficulty page or a driving return rewrites it — reads the
   same value the game keeps.
2. **The driving return's difficulty restore** (`crates/nf-game/src/ptp.rs`,
   `crates/nf-view/src/lib.rs`): `DriveReturn` gains `difficulty` (carried from the PTP block's
   +0x144, which `Boot_GetPTPData` filled from `GameState+0x28` [M 0x1c8198:18-20] and
   `Boot_LoadPTPData` writes back [M 0x1c8560:35-36]); `swap_in_scene` applies it to the new sim's
   `Progress::difficulty` and the `Drones::difficulty` mirror.

## Tests

- `crates/nf-view/tests/mp_defaults_setup.rs`: after `mp_confirm_start`, the front end asks for
  difficulty 1 (sister: the level request is also set — each can fail alone).
- `crates/nf-game/src/ptp.rs` unit tests: `drive_exit` carries the difficulty through, and the
  handover's block value lands in the return (sister assertions on the unchanged fields).

## Open items (not fixed here)

- The cheats page (`CheatInfo`) and the controls page (`PlayerSetting` invert) are unported; their
  game-side writers are cited in the table so the pages' tasks can wire the same events.
- The mini-mission launch (`Menu_RunMiniMission` 0x1ffb90, from P_PARISENUM) writes difficulty 1
  and loads the driving map 0x9000001; the port refuses driving maps from the menus, so the event
  cannot fire yet.
