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
