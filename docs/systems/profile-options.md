# Profile options — the Codenames pages' data and their effects

The front end's Codenames → Edit Codename → "Edit \<name\>" wheel leads to the option pages. This
document is the data layer: per page, the rows (control ids and label ids), the value choices, the
profile byte each row reads/writes (in `PlayerSetting[slot]` 0x2a38c8 / `MPSettings[slot]`
0x2a47a0 / the GSET globals), the boot defaults, and the runtime function that reads each byte.
The menu-def page records live in 07000048.bin kind-8 entry 08000002 (pages 0x4000001d wheel,
0x4000002d Advanced, 0x4000002e MP, 0x40000031 AV); `crates/nf-view/src/menu.rs` documents the tag
format and carries the row tables (`ADV_ROWS` / `MP_ROWS` / `AV_ROWS`) the main agent draws.

Port side: the save model is `crates/nf-game/src/profile.rs` (the PLRS/MPSG/GSET/CHET blocks,
bit-exact with the game's own chain [L InBufData 0x3416e0]); the runtime snapshot is
`profile::Options` (`Options::of(&Profile)`), carried by `Drones::options` and wired per effect
below. Tests: `crates/nf-game/tests/profile_options_live.rs` + `crates/nf-view/tests/
profile_volumes_live.rs`.

## The "Edit \<name\>" wheel — P_CNMENU_Handler 0x213288 / C_SBCNOPTIONS_Handler 0x213528

`C_SBCNOPTIONS_Handler` drives the wheel (`Menu_UpdateWheel(…, 0x2dfc28, 0x100000ff, 0x100001a5,
0x100001ec, 0x10000109, …)` [M 0x213528:23-25]); the confirm switch jumps by wheel position [M
0x213528:28-60]:

| # | Row | Page hash | Page |
|---|---|---|---|
| 0 | Secret Unlocks | 0x40000020 | the secret-codes page (the port stores the field only) |
| 1 | Controller Setup | 0x40000022 | the control-style page (`controller_style` +0x00) |
| 2 | Driving Controller | 0x4000003e | the driving style (`ptp_option` +0x10, the PTP hand-off [M Boot_GetPTPData 0x1c8198]) |
| 3 | Advanced Options | 0x4000002d | `P_CNOPTIONS_Handler` (below) |
| 4 | Multiplayer Options | 0x4000002e | `P_CNMPOPTIONS_Handler` (below) |
| 5 | AV Options | 0x40000031 | `P_CNAVOPTIONS_Handler` (below) |
| 6 | Codenames (rename) | — | `Menu_UpdateDefaultCodename` + the name entry (slot < 2; the deeper slots take the else arm) [M 0x213528:60-75] |

## Advanced Options — P_CNOPTIONS_Handler 0x213f08 (page 0x4000002d)

On open (msg 0x4b) the handler fetches each row's current value with `Menu_Send(ctrl, 0x35)` into
`PlayerSetting[slot]+offset` statics; on refresh (0x4c) it fills each list with the value items
[label id, value] [M 0x213f08:11-71]. Rows in page order:

| Ctrl | Row (title label) | Value items | Byte (`PlayerSetting+`) | Default | Runtime reader |
|---|---|---|---|---|---|
| 0x1000011A | Vibration (0x196) | 0x1B7 On=1 / 0x1B6 Off=0 | +0x09 rumble | 1 | `Input_RumbleStart` 0x17f6b8:9 / `Input_RumbleStop` 0x17f748:9 |
| 0x1000011B | Auto Aim (0x197) | 0x1B7 On=1 / 0x1B6 Off=0 | +0x01 auto_aim | 1 | `Player_AutoAim` 0x1a7c98:41 (when `MPSettings+0x180 == 0`) |
| 0x10000124 | Crosshairs (0x29c) | 0x1B7 On=1 / 0x1B6 Off=0 | +0x08 crosshair | 1 | `HUD_UpdateCrossHair` 0x19e6d0:96 |
| 0x10000125 | Crouch (0x032) | 0x2A2 Toggle=1 / 0x2A3 Hold=0 | +0x04 crouch_toggle | 1 | `Player_SSCrouch` 0x1924c0:85 picks the action mode (toggle 1 / hold 4) |
| 0x10000126 | Manual Aim (0x02a) | 0x2A2 Toggle=1 / 0x2A3 Hold=0 | +0x03 manual_aim_toggle | 0 (Hold) | `Player_Weapon` 0x1a24c8:45-52: byte 0 (Hold) clears level+0x60 bit 0 and re-sets it while action 0x13 is held; byte 1 (Toggle) flips on the press |
| 0x100001CB | Weapon Auto Switch (0x399) | 0x1B7 On=1 / 0x1B6 Off=0 | +0x0A auto_equip_best | 1 | `Player_EquipWeapon` 0x1a8a58's tail: the flag ∧ `Player_IsBetterWeapon` writes `anim+0x63` |
| 0x10000226 | Flashing Objects (0x5b7) | 0x1B7 On=2-slot / 0x1B6 Off | +0x0C flashing_objects (save packs 2 bits; bit 1 reserved) | 1 | `View_SetupRenderModes_Tweak` 0x1e5888:24: def-flag-0x80000 objects within 4 u flash while the byte is non-zero |
| 0x1000023B | HUD Always On (0x5bd) | **inverted**: 0x1B7 On=**0** / 0x1B6 Off=**1** | +0x0B hud_auto_hide (0 = always on) | 0 | `Player_Update` 0x193600:414-425: byte 0 pins the pane timer `BL+0x8BC` at 1.0 every tick; byte 1 decays it by 2·0.0020833334·FRAME_RATE_MUL per tick (≈4 s to fade); `HUD_UpdateHealthPane` 0x19b9c8:118-122 scales the panes off the timer and pops it back to 1.0 |

Defaults: `Input_Init` 0x17edd8 (memset 0, then +0x0C = 1 and the bits above); the "New Codename"
template [L 0x2e1b18] matches it except `controller_style` = 7.

## Multiplayer Options — P_CNMPOPTIONS_Handler 0x2144d8 (page 0x4000002e)

| Ctrl | Row (label) | Value items | Byte | Default | Runtime reader |
|---|---|---|---|---|---|
| 0x10000128 | Radar (0x1b3) | 0x1B7 On=1 / 0x1B6 Off=0 | `MPSettings+0x28` radar | 1 | `MP_GetRadarObjects` 0x189840:22 / `HUD_RadarUpdate` 0x19f6e8:102 |
| 0x1000011C | Health Handicap (0x296) | −75, −50, −25, 0, +25, +50, +75, +100 (the label literals at 0x30d390..0x30d3c8) | `MPSettings+0x2c` health (32 raw bits, signed) | 0 | `Player_Init` 0x1910f8:129 / `MP_ReSpawn` 0x185368:72 / `HUD_UpdateMPHealthPane` 0x19b630:69 add it to 100.0 |
| 0x10000129 | Auto Aim (0x197) | 0x1B7 On=1 / 0x1B6 Off=0 | `PlayerSetting+0x02` auto_aim_alt [M the handler's `DAT_002a38ca`] | 0 | `Player_AutoAim` 0x1a7c98:44 (the MP context arm) |

Note: the page reads its working copies from `MPSettings+0x40/+0x44` [M 0x2144d8:13-14] — the
display statics the menu system seeds; the SAVE path is `LS_MakeMPSettings` 0x1ce508's
`slot·0x30 + 0x28/+0x2c` [M], which is what the port models.

## AV Options — P_CNAVOPTIONS_Handler 0x214818 (page 0x40000031)

| Ctrl | Row (label) | Value items | Stored where | Default | Runtime reader |
|---|---|---|---|---|---|
| 0x10000135 | Music Volume (0x194) | slider 0..20 (msg 0x27/0x2e/0x40; the exit writes ×5) | GSET music_volume (7 bits) | 70 (0x46 [L template]) | `SFXMusicSetVolume` 0x116cf8 → `MusicMasterVolume` 0x30c958 |
| 0x10000134 | Effects Volume (0x193) | slider 0..20 (×5) | GSET sfx_volume (7 bits) | 100 (0x64) | `SFXSetVolume` 0x116d98 → `SfxMasterVolume` 0x30c95c |
| 0x10000136 | Subtitles (0x2b5) | 0x1B7 On=1 / 0x1B6 Off=0 | GSET bit 22 → `GameState+0x60` 0x2a37c8 | 0 | `Sound_DoSubtitle` 0x1dde58:21 gates on it |
| 0x1000019A | Multiplayer Split Screen (0x2b8) | 0x2BA=0 / 0x2B9=1 | GSET draw_info (32 bits, the `DrawInfo` global 0x2a37c0) | 2 [L template] | the render's split-screen layout (`DrawInfo` reads) |
| 0x10000198 | Speaker (0x195) | 0x393 Mono=0 / 0x394 Stereo=1 / 0x395 Pro Logic=4 | GSET sfx_mode (32 bits) | 0 [L template] / restore sets 1 | `SFXSetMode` 0x117078 |
| 0x10000223 | Widescreen | 0x1B7 On=1 / 0x1B6 Off=0 | GSET bit 55 → `GameState+0x5C` 0x2a37c4 | 0 | `C_CHCHWS_Handler` 0x216168:9 copies it to `switch_CONFIG_WIDESCREEN`; the render's aspect |
| — | Screen Adjust | the drag surface | GSET screen_x/screen_y (2×32 bits, via `psiGetScreenPos`/`psiSetScreenPos` 0x30d8e8/0x30d8ec) | 0,0 | `psiSetScreenPos` |
| — | Restore Defaults | action | music 0x0E·5=70, effects 0x14·5=100, subtitles per `LanguageToLoad`, DrawInfo=0, sfx_mode=1, `psiRestoreScreenPos`, widescreen 0 [M 0x214818:96-110] | — | — |
| — | Credits | action (0x10000138) | — | — | opens the credits subscreen |
| — | Die Another Day Trailer | action (0x10000222) | — | — | plays the trailer movie |

The AV page defers its writes to the exit-confirm (msg 0x50: `SFXMusicSetVolume(slider·5)`,
`SFXSetVolume(slider·5)` [M 0x214818:70-75]); backing out (0x4d) restores the opening values [M
0x214818:79-84].

## Codename entry — P_CNNAME_Handler 0x213790 / C_KEYBOARD_Handler 0x213a98

The letter grid page (page 0x40000021): the grid is the keyboard's `Menu_UpdateWheel`-driven
cursor over the def's letter items; accept validates through `Menu_IsValidCodename` 0x1f9930
(trim, uppercase, empty = 0x3bc, a default-codename substring = 0x2c0, length < 0x21) and
`Menu_UpdateMessageBox` case 1 writes the chain under the codename [M]. The port's model:
`FrontEnd` + `Profile::save_to_dir` (the file name IS the codename [M 0x1f7de8]) — see
`fix_save_profile_live.rs`. No per-option data lives on this page.

## The port's wiring (what reads what)

| Option | Where the port reads it |
|---|---|
| auto_aim (+0x01) | `scene.rs`'s shot-direction block seeds `Sim::auto_aim` from `drones.options.auto_aim` each tick; the lock itself is `Drones::auto_aim` (flag-independent — the caller gates it, like the game's call site under `Player_AutoAim`) |
| crosshair (+0x08) | `hud.rs`'s reticle draw skips when off (`HUD_UpdateCrossHair` 0x19e6d0:96's gate) |
| crouch_toggle (+0x04) | `LadderCtx::crouch_toggle` (scene.rs plumbs `drones.options.crouch_toggle`); `Bond::physics_tick`: toggle flips on the press edge (`Bond::crouch_prev`), hold crouches while held and stands on the release |
| manual_aim_toggle (+0x03) | `scene.rs`'s input path: hold rides the button (the old behaviour), toggle latches on the press edge (`Sim::aim_prev/aim_latch`) |
| auto_equip_best (+0x0A) | `Drones`' pickup take pass gates `weapon_switch_request` on it |
| hud_auto_hide (+0x0B) | `Drones::hud_fade` (BL+0x8BC): pinned at 1.0 or decayed per tick in `Drones::tick`; reset to 1.0 at both player-damage sites |
| flashing_objects (+0x0C) | stored on `Options`; the render-side flash (0x1e5888) is a GAPS hook below |
| rumble (+0x09) | stored only (the port drives no force-feedback — per the task's scope) |
| music/sfx volume | `nf_view::sound_tick` copies `drones.options.{music,sfx}_volume` into the `nf_audio` driver's `music_master`/`master` every frame |
| subtitles / widescreen / sfx_mode / draw_info | stored on `Options` (the port's subtitle draw and render aspect are the view's; GAPS hooks below) |
| radar / health_handicap / mp_auto_aim | stored + round-tripped (MP systems are out of the port's SP path) |

## GAPS (systems the port does not have yet — hooks left)

- **Flashing Objects render tweak** (0x1e5888): the port's render has no per-object flash mode;
  the flag is on `Options` for the renderer to read.
- **HUD pane alpha**: the panes' fade scale (`HUD_UpdateHealthPane` 0x19b9c8:118) is the draw
  side; `Drones::hud_fade` carries the timer for it.
- **Split-screen / widescreen / speaker mode**: render + audio-presentation concerns; the values
  round-trip on `Options`.
- **Secret Unlocks codes** (the 0x40000020 page): the unlock fields live in the progress layer's
  BNUS/MSSN blocks; the page itself is the main agent's draw work.
- **Vibration**: stored only.
- **MP systems** (radar, handicap at spawn, the MP auto-aim context `MPSettings+0x180`): the port
  has no MP match path for them yet (`mp::MpMatch` exists for the bots); the fields round-trip.
- The MP options page's working-copy statics (`MPSettings+0x40/+0x44` [M 0x2144d8]) vs the saved
  `slot·0x30+0x28/+0x2c`: the page's seed path (which slot the codename editor targets) is [S]
  "the active codename's slot" — a live check would settle it.

## Live checks for the main agent

1. **HUD Always On vs Off**: on any level, set +0x0B = 1 (Off) via the page and stand still ~4 s:
   the panes fade out; taking damage pops them back. gdbwatch `BL+0x8BC` (expected: 1.0 → decays
   0.004167/tick → 0; a hit writes 1.0).
2. **Crouch Toggle vs Hold**: with +0x04 = 1 a tap keeps Bond crouched; with +0x04 = 0 he stands
   the moment the button lifts (`Player_SSCrouch` 0x1924c0:85's action-mode pick).
3. **Manual Aim Toggle**: with +0x03 = 1, tapping the aim button latches free-aim (the crosshair
   window stays); with 0 it follows the button (`Player_Weapon` 0x1a24c8:45-52's level+0x60 bit 0).
4. **MP page slot**: open Multiplayer Options on a second codename and gdbwatch the write the
   Radar row makes — `MPSettings+0x40` (0x2a47e0) or `slot·0x30+0x28`: settles the [S] above.

## Tests

- `crates/nf-game/tests/profile_options_live.rs` (5, all pass on the NF_ISO image):
  `round_trips_every_field`, `options_snapshot_matches_the_profile`,
  `crouch_toggle_flips_on_the_press_edge_and_hold_tracks_the_button` (through `Bond::tick_ctx`),
  `hud_fade_pins_and_decays_with_the_option` (through `Drones::tick`),
  `auto_aim_bridge_follows_the_profile`.
- `crates/nf-view/tests/profile_volumes_live.rs` (1): `volume_sliders_reach_the_driver` (through
  `nf_view::sound_tick`, `output: None` — silent).
- The pre-existing `profile.rs` unit tests (the game's own chain decode [L InBufData], the block
  sizes, `round_trip_and_files`) still pass after the field renames.
