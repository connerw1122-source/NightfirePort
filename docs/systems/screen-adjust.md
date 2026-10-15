# Screen Adjust (AV Options → Screen Adjust) and the AV options' runtime effects

The picture-offset page and the AV options page (Codenames → Edit Codename → AV Options) — the
model ported into `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page::ScreenAdjust`, `action()`), with
the values and conversions the draw code needs. Evidence tags: [M] code (function + address), [D]
disc, [L] EE snapshot, [S] suggested. All handler line numbers are the decompiled files under
`work/decomp/ACTION.ELF/`.

## 1. The page flow

- AV Options is menu-def page `0x40000031` [L 07000048.bin kind-8 entry 08000002 walk 2026-10-15;
  `P_CNAVOPTIONS_Handler` 0x214818]. Its rows (control, label id, y — the page data's own values):

  | ctrl | label | text [D USATxt.dat] | y | kind |
  |---|---|---|---|---|
  | 0x10000135 | 0x194 | Music Volume | 86 | slider 0..0x14 (d3 0x64 = 100 shown), value control (349,86,216,16) |
  | 0x10000134 | 0x193 | Effects Volume | 114 | slider 0..0x14, value control (350,114,219,16) |
  | 0x10000136 | 0x2b5 | Subtitles | 142 | On/Off (0x1b7/0x1b6) |
  | 0x1000019a | 0x2b8 | Multiplayer Split Screen | 170 | 0x2ba Horizontal / 0x2b9 Vertical |
  | 0x10000198 | 0x195 | Speaker | 198 | 0x393 Mono / 0x394 Stereo / 0x395 Surround |
  | 0x10000223 | 0x5b8 | Widescreen | 227 | On/Off |
  | 0x10000137 | 0x2b6 | Screen Adjust | 256 | action → page 0x40000047 |
  | 0x100001a3 | 0x37e | Restore Defaults | 284 | action |
  | 0x10000138 | 0x198 | Credits | 312 | action |
  | 0x10000222 | 0x550 | Die Another Day Trailer | 340 | action |

  The page title is 0x2bb "AV Options" and the hint 0x217 "~A Confirm  ~V Scroll  ~H Change  ~B Back"
  [L page data; texts bound through `Lang::bind`]. Note the port's old literal "Pro Logic" for 0x395
  is wrong — the disc string is "Surround".

- **Screen Adjust** (`P_CNAVOPTIONS_Handler` 0x214818 msg 0x4b, control 0x10000137 → lines 117-126):
  sets `going_to_av_subscreen` (0x30d536), then `Manager_SendMessage(manager, 0x44, 0x40000047, 0)`.
  Manager msg 0x44's arg bit 0 pushes the current page (`(arg ^ 1) & 1` gates `Stack_Push` [M
  `Manager_SendMessage` 0x21c858 case 0x44]) — so 0x40000047 opens ON TOP of the AV page.

- The other action rows: 0x10000138 (Credits) and 0x10000222 (Trailer) only set
  `going_to_av_subscreen` [M 0x214818:91-94, 112-115]; 0x100001a3 (Restore Defaults) is §4.

## 2. The Screen Adjust page (0x40000047)

Page data [L menu-def walk 2026-10-15]: root 0x40000047 (0,0,640,480); two slider controls both
named `0x100001eb`, distinguished by their d1 field — **d1 0 = the vertical bar** (304,176,32,128),
**d1 1 = the horizontal bar** (256,224,128,32), both d3 0xa with initial value msg 0x2e (0xa
horizontal, 0x0 vertical); a fullscreen picture control with texture **0x3000074** (the reference
frame the bars move); the hint label **0x67b "~A Confirm  ~B Cancel"**; decorative corner brackets
(texture 0x3000042) around an outer (51,41)-(590,440) and an inner (115,105)-(526,376) rectangle.

Handler [M `P_SCREENADJUST_Handler` 0x217a58, size 176]:

- msg **0x4c** (page enter): `psiGetScreenPos(&PS2OldX, &PS2OldY)` — the current offset is saved to
  PS2OldX 0x30d578 / PS2OldY 0x30d57c [M symtab].
- msg **0x4b** (✕ Confirm): `Manager_SendMessage(manager, 0x5f, 1, 0)` — close. Manager msg 0x5f's
  second arg ≠ 0 **skips** the close notification block [M 0x21c858 case 0x5f: `if (param_3 == 0)`
  guards the msg 0x6b sends], so the adjustment **persists**.
- msg **0x6b** (page closed via the 0x5f(0) path — the hint's Cancel): `psiSetScreenPos(PS2OldX,
  PS2OldY)` — **restores** the page-enter snapshot [M 0x217a58 case 0x6b].

Slider behaviour [M `C_SBSCREEN_Handler` 0x217b08, size 204]:

- msg **0x51** (a control was created — `MenuManager_Create` 0x21b908 sends 0x51 for every CONTROL
  tag [M :188-193, 275-280]): sends msg 0x27 (−1, 1) back to the control — the slider's range is
  **−1..+1**, re-centring on 0 (spring). [S: the exact spring cadence — one 0x49 per ◀/▶ press — is
  the reading below; list a live check if the drawn step differs.]
- msg **0x49** (the scroll control's value changed — `Scroll_Update` 0x221b20:240 sends 0x49 to the
  page handler with the control as param 2): reads the slider's value (msg 0x40) and its d1
  (`*(int*)(control + 0x20)`), then `psiAdjustScreenPos(0, value)` for d1 0 (vertical) or
  `psiAdjustScreenPos(value, 0)` for d1 1 (horizontal).

## 3. The offset variables and what they change on screen

- Variables [M symtab]: `PS2ScreenXPos` 0x30c75c, `PS2ScreenYPos` 0x30c760,
  `PS2UpdateScreenPos` 0x30c764. Boot value 0/0 (.sbss); `psiRestoreScreenPos` 0x10b940 resets X/Y
  to 0 and raises the update flag; `psiSetScreenPos` 0x10b910 / `psiGetScreenPos` 0x10b928 are the
  plain setters/getters.

- `psiAdjustScreenPos(dx, dy)` 0x10b890 [M]:
  ```
  PS2ScreenYPos += dy * 4;
  PS2ScreenXPos += dx * 12;
  X clamped to [-0x1b0, 0x1b0] (±432); Y clamped to [-0x30, 0x30] (±48);
  PS2UpdateScreenPos = 1;
  ```
  So one slider step moves the picture **12 X-units** or **4 Y-units**; the ranges are **X ±432**,
  **Y ±48**.

- The application: `VBlankInt` 0x124088:13-17 [M] — when `PS2UpdateScreenPos` is set,
  ```
  REG_GS_DISPLAY2 = (PS2ScreenXPos + 0x27c) | (PS2ScreenYPos + 0x32) << 12 | 0x1bf9ff02000000;
  ```
  (then the flag clears — the write happens once per change, in vblank). The constant decodes
  against the game's own `sceGsSetDefDispEnv` 0x226370 [M] for NTSC 512×448 interlaced
  (`sceGsSetDefDBuff(…,0,0x200,0x1c0,…)`, `CreateDB` 0x10b000:21): MAGH = 5 (`(512+0x9ff)/512 = 5`,
  written as `(5-1)<<23`), DW = 5·512−1 = 2559 (bits 32+), DH = 448·2−1 = 895 (bits 44+), DX base
  0x27c = 636, DY base 0x32 = 50 — exactly the constant `0x1bf9ff02000000 | 50<<12` [M].

- Units: the visible line is 5·512 = 2560 DX units wide (MAGH 5), so **X is in 1/5 framebuffer
  pixel units**: one X step (12) = 2.4/512 of the picture width; the full X range (±432) =
  ±86.4/512 ≈ ±0.169 picture widths. **Y is in field lines** (448 per frame): one Y step (4) =
  4/448 of the picture height; the full Y range (±48) ≈ ±0.107 picture heights.

- Conversions the port exposes (`FrontEnd`):
  - 640×480 menu space (what the menu overlay authors in, mapped 640→512 / 480→448): **(X/4,
    Y·15/14)** — exact inverses of the overlay's 0.8 / 14·15 scaling.
  - fractions of the visible picture: **(X/2560, Y/448)**.
  - The SIGN convention (whether a positive X shifts the drawn picture left or right on the TV —
    DISPLAY's DX selects where the read starts) is not decidable from the decompile alone; the
    renderer applies the fraction and the direction is confirmed by the live check in
    docs/reports/screen-adjust-model.md. [S]

- PAL note: `sceGsSetDefDispEnv`'s PAL bases are DX 0x290 / DY 0x48 [M 0x226370:83-96], but
  `VBlankInt` writes the NTSC bases unconditionally; on this NTSC-U disc `PS2Region` is 0 and
  `CreateDB` takes the NTSC arm [M 0x10b000:17-24]. Not modelled.

## 4. The AV page's commit / cancel / Restore Defaults

`P_CNAVOPTIONS_Handler` 0x214818 [M]:

- Page enter (msg 0x4c, lines 19-66): snapshots `SFXGetVolume()/5` and `SFXMusicGetVolume()/5` into
  `start_sfx_vol` / `start_music_vol` (0x30d538-ish statics) and the screen offset into
  `screen_adjust_x/y` 0x30d8e8/0x30d8ec (`psiGetScreenPos`); sets the two sliders' range to 0..0x14
  (msg 0x27) and their values (volume/5), and pushes the current option values into the rows (msg
  0x1e). Only re-entered from page 0x4000001d (the Edit wheel) runs the full init (`param_6 ==
  0x4000001d` gate, line 21).
- Per-frame (msg 0x50, lines 70-76): music/sfx volume := slider value × 5 — **the sliders adjust
  the live mix while the page is up**.
- Row ✕ (msg 0x4b, lines 87-136): Screen Adjust / Credits / Trailer open their subscreens;
  **Restore Defaults** (0x100001a3, lines 96-110) does the full reset list (below); **any other
  row's ✕ falls through to the commit** (lines 127-136): reads every row back into the option
  globals (music/sfx ×5, subtitles `DAT_002a37c8`, split `DrawInfo`, speaker `SFXSetMode`,
  widescreen `DAT_002a37c4`), re-snapshots the screen offset, sets `cn_modified_flag`, and closes
  the page (`Manager_SendMessage(0x5f, 0, 0)`).
- Back (msg 0x4d, lines 79-85): if `going_to_av_subscreen` is clear (no subscreen was opened),
  **restores** the enter snapshots — volumes and `psiSetScreenPos(screen_adjust_x, screen_adjust_y)`.
  (Re-entering the page — coming back from Screen Adjust — re-runs 0x4c, so the snapshot includes
  the adjustments just made.)

**Restore Defaults' full reset list** [M 0x214818:96-110]:

| what | reset to |
|---|---|
| music volume slider (0x10000135) | 0xe = 14 → volume **70** |
| effects volume slider (0x10000134) | 0x14 = 20 → volume **100** |
| subtitles (0x10000136) | `LanguageToLoad < 0 || (3 < LanguageToLoad && != 6)` → **On**, else **Off** (NTSC-U's language 0 → **Off**) |
| split screen (0x1000019a / `DrawInfo`) | **0 = Horizontal** |
| speaker (0x10000198) | **1 = Stereo** |
| widescreen (0x10000223) | **Off** |
| picture offset | **(0, 0)** (`psiRestoreScreenPos` 0x10b940) |

(Note: this is the restore list, not the fresh-profile defaults — the Default profile captured on
PCSX2 has music 80 [L default_opts' PCSX2 m32/m38/m40 note in menu.rs].)

## 5. How the values persist (profile)

`LS_MakeGlobalSettings` 0x1cd9d0 [M] pushes, in order: music volume (7 bits), sfx volume (7 bits),
language (7 bits), subtitles `DAT_002a37c8` (1 bit), SFX mode (32 bits), widescreen `DAT_002a37c4`
(1 bit), `DrawInfo` (32 bits), then `psiGetScreenPos(&x, &y)` and **screen X (32 bits), screen Y
(32 bits)**, then 219 × 32 zero bits and the literal 4 as 16 bits. `LS_LoadGlobalSettings`
0x1cdbc8 [M] pulls the same order and calls `psiSetScreenPos(x, y)` — the offsets ride the GLOBAL
settings blob (one per memory card, not per codename). The boot path `Boot_LoadPTPData` 0x1c8560
also `psiSetScreenPos(DAT_0031cac0, DAT_0031cac4)` [M].

## 6. The AV values' runtime effects in the game

- **Widescreen** `DAT_002a37c4` (DrawInfo+4, 0x2a37c4): `Camera_CalcViewAngles` 0x1baf70:28-32 [M]
  writes the view aspect at `viewer+0x114` — **0x3fe38e39 = 16/9 when set, 0x3faaaaab = 4/3 when
  clear** — and with two active viewers multiplies it by 0.5 (Vertical split, `DrawInfo == 1`) or
  2.0 (Horizontal) [M :33-42]. The port models the aspect as `Sim.view_aspect` (used by the gun
  tilt, the one place the port consumed viewer+0x114) and `FrontEnd::view_aspect()` for the
  renderer's projection. `C_CHCHWS_Handler` 0x216168 [M] mirrors the flag into the PC config's
  `switch_CONFIG_WIDESCREEN` when the checkbox toggles.
- **Split screen** `DrawInfo+0` (0x2a37c0, 0 = Horizontal, 1 = Vertical): `Camera_CreateCameras`
  0x1bb2b8:44-51 [M] with 2 viewers makes the viewports columns (width 256) for Vertical and rows
  (height 224) otherwise; `HUD_ValidateXY` 0x19a250:35 and `HUD_FixMP` 0x19b110:33 take
  `DrawInfo == 1` layout paths; `MP_Start` 0x183a50:188 branches on `DrawInfo != 0`. Documented
  only — split-screen play is out of scope.
- **Subtitles** `DAT_002a37c8` (DrawInfo+8, 0x2a37c8): `Sound_DoSubtitle` 0x1dde58:21 [M] shows a
  voice line's text only when the flag is set OR the line id has bit 0x80000000 (forced, e.g.
  foreign speech); `Text_AddMsg` 0x1b0c30:148 gates the type-4 message class the same way;
  `ResetMap_Load` 0x1d6360:40 [M] binds movie subtitle tracks (0x6000963/0x600096b/…) for the
  0x71000xx movie-wrap maps when set; `Menu_WillRunMiniMission` 0x202450 [M] force-enables it for
  the mini-mission. Boot default from the console data (`Boot_LoadPTPData` 0x1c8560:79).
- **Speaker mode** (0/1/4): `SFXSetMode` 0x117078 [M] queues batch command 0x1c with the mode (the
  output config of the audio engine); restored by `SFXSetMode` on load. The port's mixer is
  stereo-only — documented, not implemented.

## 7. The port's mapping (crates/nf-view/src/menu.rs)

- `Page::ScreenAdjust` = 0x40000047; two "rows" (the sliders): row 0 vertical, row 1 horizontal —
  Up/Down move the cursor, Left/Right nudge the focused axis one step.
- `FrontEnd`: `screen_x` / `screen_y` (PS2ScreenXPos/YPos), `screen_saved_x/y` (the Screen Adjust
  page's enter snapshot, PS2OldX/Y), `screen_av_saved_x/y` (the AV page's snapshot,
  screen_adjust_x/y) — each field on its own line [WORKLOG lesson 4].
- Methods: `screen_adjust(dx, dy)` (psiAdjustScreenPos verbatim), `screen_adjust_open()` (msg 0x4c
  snapshot), `screen_adjust_cancel()` (msg 0x6b restore), `screen_restore_defaults()`
  (psiRestoreScreenPos), `screen_offset_640()` and `screen_offset_frac()` for the draw code,
  `view_aspect()` (16/9 vs 4/3 from the widescreen option).
- Flow: CnAv row ✕ on 0x1000_0137 pushes ScreenAdjust (snapshot on open); ✕ there keeps and pops
  (the AV page re-snapshots, as the game's re-enter does); ○ there restores and pops; CnAv Back
  restores its own snapshot (values + offset); CnAv row ✕ on 0x1000_01a3 runs the §4 reset list
  against `opts` + the offset; committing CnAv (row ✕ on any other row) re-snapshots the offset.
