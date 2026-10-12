# The runtime HUD sprite and the HUD pane list

Proof levels: see [README.md](README.md). Live session: 2026-10-12, PCSX2, 07000005 at start (P2K drawn, "7-41 Semi").

## How to reach them live

`PLAYERS` 0x2D88E0 → player obj → `obj+0xE0` = BLData → **`BL+0x7E0` = the HUD block** [M HUD_Init 0x199e88:20;
L-read: obj 016cefe0, BL 016cf0f0, HUD 016c8630].

**The pane list** (`HUD_Update` 0x19a088): 22 entries of 0x20 bytes from `HUD+0` (entry k at `HUD + k·0x20`).

| Entry offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x08 | — | the `HUDPANE` the update function gets (`HUD + k·0x20 + 8`) | [M1 0x19a088:39] |
| +0x10 | ptr | the pane's sprite-pointer array | [M1 0x19a088:26] [L-read: ammo pane 016c85f0] |
| +0x14 | fn | the pane's update function (0 = none) | [M1 0x19a088:38] [L-read: k=0 → 0x19a7f0 `HUD_UpdateAmmoPane`] |
| +0x20 | u16 | sprite count | [M1 0x19a088:24] [L-read: ammo 8] |
| +0x24 | u8 | enabled: 0 → every sprite's `+0x38 = 0xFF`; else `+0x38` = the template sprite's layer (`SprInfo[i]+8`) | [M1 0x19a088:22-55] [L-read: ammo 1] |

## The runtime sprite

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x10 | u32 | colour, `0xRRGGBBAA`: **low byte = alpha** (0x80 = opaque on the GS scale; 0xFF also drawn opaque) | [M1 HUD_UpdateHealthPane 0x19b9c8:323 `uVar13 \| 0x7f7f7f00`] [L-read 0x7d6d59ff] **[L-verified]**: `poke 0x01fd7c48 4 0x7d6d5920 --shot` → "Semi" fades to a ghost, nothing else changes |
| +0x18 | ptr | the text buffer (`Sprite_SetText`) | [M1 0x19a7f0] [L-read: "7-41", "Semi", "Wolfram PP7"] |
| +0x20 | u16 | the text sprite's draw flag/length: 0 = no text drawn | [M1 0x19a7f0:178-189] **[L-verified]**: the switch timer → the ammo text's `+0x20` = 0 and "7-41" disappears |
| +0x24 / +0x26 | s16 | screen x / y (written at run time; overrides the template's position: the crouch icon gets 0x30, 0x30) | [M1 0x19b9c8:309-310, 0x19a7f0:255-262] |
| +0x2C / +0x2E | s16 | width / height | [M1 0x19a7f0:263-266] |
| +0x30 / +0x32 / +0x34 / +0x36 | s16 | u / v / uw / vh (texels) | [M1 0x19a7f0:264-269] |
| +0x38 | u8 | **the draw layer; 0xFF = not drawn.** **Live census** (gdbwatch, 4 s of play on 07000005, readwrite): `Sprite_BuildList+0x94` (reader, 325 hits: the sprite renderer's draw list), `HUD_Update+0xc4` (66, writer), `HUD_UpdateAmmoPane+0x43c` (65, writer); no other instruction touched it. **[M*] + [L-verified]**. Not alpha (a worker read it as alpha: the inverted weapon labels of 2026-10-11) | [M1 0x19a7f0:218-226, 0x19a088:31,51] [L-read: "Semi" 0x1c shown, "Wolfram PP7" 0xff hidden, matching the screen] **[L-verified]**: driving the switch timer makes the game set the name's `+0x38` to 0x1c and the fire mode's to 0xff, and the screen shows "Wolfram PP7" alone |

## The inputs that drive the ammo pane

| Field | Meaning | Proof |
|---|---|---|
| `BL+0x95E` (u8) | the weapon-switch timer: set to `FRAME_RATE` (60) when the drawn weapon's record `+2` word changes, then −1 per frame. While > 0: name shown, fire mode, ammo text and rounds strip hidden | [M1 0x19a7f0:213-226] **[L-verified]**: `BL+0x95E = 60` pinned → "Wolfram PP7" alone on screen; at 0 → "7-41 Semi" |

Template sprites (`*SprInfo`, 0x2C bytes, static): layer at +8, x/y at +0xC/+0xE, w/h +0x10/+0x12, uv +0x14..+0x1A,
label +0x1C, texture +0x24 [`nf_formats::hud::SpriteInfo`]. Exception: `PickupStatusSprInfo`'s three bar sprites are
copied from 0x2FC460 with a 0x30 stride [L ee_start_05.bin].

## Corrections and extensions (fields-player, 2026-10)

- **The template `label` (+0x1C) has three cases**, not two [M `Sprite_Create2` 0x1de3d8:111-134; L data]:
  −1 = a texture sprite; **0 = a text sprite whose string is bound at run time** (`Txt_BindLabel(0)` → heap
  sentinel; the ammo pane's three text sprites are 0); 0x02xxxxxx = a static label (e.g.
  `ObjectiveStatusSprInfo[5]` = 0x02000052).
- **Runtime sprite fields the first pass conflated** [M 0x1de3d8; S 0x1de5bc-5d8]: rt+0x1C = the text STYLE
  pointer (a 16-byte `{u32 colour, 0, u32 colour2, 0}` record; the ammo text's is 0x30CC78); **rt+0x20 = the
  text LENGTH** (strlen at create — the "+0x20 draw flag/length" row above is this); **rt+0x22 = the template
  flags u16** (copied from template +0x0A); rt+0x28/0x2A = the home x/y (template +0xC/+0xE when label ≠ −1).
- **Template +0x20 = the text-style pointer, +0x28 (s8) = the viewer link index** — 5 (the overlay viewer)
  for every entry of every table [L census]. The port's decoder reads neither (no functional impact).
- **flags 0x1000 = clip-to-screen** (skipped by `HUD_CalcWidthHeight` 0x19a598:29 — the ammo pane's strip
  sprites carry it, which is why the pane's h computes to 38) [M]; 0x800 on the crosshair/blip sprites.
- **The BSS status tables** (`MissionStatus`/`ObjectiveStatus`/`InfoStatus`/`PickupStatus`SprInfo at
  0x319580/0x319608/0x319730/0x319830) are boot-filled by `__static_initialization_and_destruction_0`
  0x1a0058: entry 0 built on the stack, the rest byte-copied from **0x30-stride source records at
  0x2FC2E0 (mission, 2) / 0x2FC340 (objective, 3) / 0x2FC3D0 (info, 3) / 0x2FC460 (pickup, 3)** — the
  sprite.md exception above is one quarter of a 0x2FC2E0..0x2FC4F0 region [M 0x1a0058:77-104; L dump].
- **`TextMsgFormats` 0x2BF108** (56 B) = 7 × {u32 stylePtr, u16 maxWidth}: the status CREATE tags' widths
  are TextMsgFormats[type]·maxWidth (480/480/256) [L; M 0x19c310 family].
- **The full pane maps** (HUDPANE_tag, HUDPANECREATE_tag, all 26 SprInfo tables with entry counts and
  consumers) are in [hud-panes.md](hud-panes.md).
