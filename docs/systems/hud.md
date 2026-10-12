# HUD (single player, on foot)

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp, [L] snapshot `work/ee_start_05.bin`, [S] suggested. Source: a
research pass (subagent, 2026-10-09); "checked" = read here (the tables are read from ACTION.ELF by
`nf_formats::hud::HudTables` and asserted against the snapshot in `hud_tables_match_the_snapshot`).

## Framework [M]

- Screen 512 × 448 (viewer 0) shown at 4:3; the HUD sprites belong to the overlay viewer 5. Margins
  (`HUD_ValidateXY` 0x19a250): 16 px from the right / bottom edges.
- HUDINFO = `*(BL+0x7E0)`: [0] crosshair, then 22 panes (0 ammo, 1 health, 2 mission status, 3 objective, 4 info,
  5 air, 6 / 18 / 19 scope, 7 night vision, 8 lens flare, 9 Redeemer, 10 car, 11 camera, 12 blood, 15 X-ray,
  16 security camera, 17 OICW, 20 space, 21 pickup); enabled at level start: 0, 1, 2, 3, 4, 8, 21 [L].
- `SpriteInfo` (0x2C bytes, `Sprite_Create2` 0x1de3d8) [checked]: colour 0xRRGGBBAA, shadow colour, layer, flags,
  x / y (pane-relative), w / h, u / v / du / dv, label, texture hash. Drawing (`View_DrawSprites` 0x1e5290): GS
  modulate (0x80 = 1.0), alpha drawn as A >> 1; flags 0x100 additive, 0x200 subtractive, 0x800 centred, 0x40 / 0x80
  flip U / V, 0x20 drop shadow (text: colour 2 at (+1, +1)). Sorted by layer, higher drawn first (`CompSpr` 0x1d3320).

## Crosshair [checked]

`HUD_UpdateCrossHair` 0x19e6d0: sheet 0x03000026, `HUDCrossCoords` 0x2bec48 (1 aim 29 × 29, 2 normal 11 × 11,
3 friendly), centred at (256 (1 + cx), 224 (1 − cy)), 0xFF0000FF, layer 49.

## Health / armour pane 1 (`HUD_UpdateHealthPane` 0x19b9c8) [M; tables checked]

- Origin (16, 337) [L]; `HealthSprInfo` 0x2bd050 (24 sprites), texture 0x0300007A.
- Armour sprites 0–7 (armour `BL+0x8B0`, max 50): hidden when armour · 0.16 = 0; else alpha 1.0 for sprite 0,
  0.75 below armour · 0.16, 0.35 above.
- Health sprites 8–14 (`BL+0x894` clamped 0–100, h = health · 0.07): colour 0x52A88B (h ≥ 4), 0xC6984E (2–4),
  0x9D1912 (< 2); alpha 0.55 below h, 0.15 above; sprite 8 blinks (30-tick period) when h < 0.5. Ring 15: the
  health colour at alpha 140.
- Fade F = min(`BL+0x8BC` · 4, 1): held at 1 while `PlayerSetting+0xB` = 0 (snapshot) [S: an auto-hide option].
- fix-hud: ported — the damage-direction bits (`pain_from`), the air pane (AirSprInfo), and the crouch icon (sprite 19: 0x03000173 crouched, 0x03000174 held 60 ticks after standing, fade over the last 30 [M 0x19b9c8:305-328]).
- Damage glow sprites 20–23: full screen, additive, textures 0x0300018D (20, 21 flipped V) / 0x0300018E (22, 23
  flipped U), red at alpha `BL+0x968`, per bit of `BL+0x967` (direction from `Player_DealWithObjHit`; 0xC both
  sides from `Player_HandlePain`).
- Also: Bond-moment icon (16), context icon (18), crouch icon (19, 0x03000173 / 0x03000174) — not ported.

## Ammo pane 0 (`HUD_UpdateAmmoPane` 0x19a7f0) [M; tables checked]

- Origin (496, 394) [L]; `AmmoSprInfo` 0x2bce58.
- Sprite 0: `"%d-%d"` (clip-spare) in font 2, right-aligned at (496, 412), 0x7D6D59, black shadow; percentage
  `"%d"` for ammo types 0x14 / 0x1A / 0x1F; blank with weapon flag 0x20.
- Sprites 1 / 2 at (496, 432): fire-mode label (`weapon+0x30 + mode·4`) / weapon name (`weapon+0x38`), the name shown
  for 60 ticks after a weapon change (`BL+0x95E`) — need the text labels (not ported).
- Sprites 5 / 6: the rounds strip / the empty strip (0x4040407F) from texture 0x03000128 by `BulletImg.189`
  (0x2bea38: per ammo type u, v, w, pixels per round; ×16 width for clips > 87): bottom at y 394, x = 496 − w.

## Fonts [M; font 2 checked]

`FontTable` 0x2dd8b0; font 2 = `FONTSerpLight` (texture 0x03000003, 512 × 64): first / last char 33 / 255, 149
glyphs (0x18 bytes: s16 u, v, w, h, y offset, pre-gap, post-advance, kern count; u32 kern list; u16 code), line
gap 5, space 3. Pen (`__Font_DrawText` 0x1ee598): + kern (`Font_GetKernAdjust` 0x1ee298: the previous glyph's 6-byte
pairs, next char at +2, signed adjustment at +4), + pre-gap (not for the first), quad at y + yoff − (glyph 0's
h + yoff), + w + post-advance. `FE 2` right-aligns, `FE 3` centres.

## Pickup / message panes [M] (fixed by fix-hud)

`Text_AddMsg` (0x1b0c30), pane 21 (pickups, 180 ticks): bar 256 × 26 at (128, 364) from 0x0300007A, text centred at
(256, 380) in font 2; states hidden → open → text → fade (last 15 ticks, alpha = time·17) → close (16 px / tick;
the open is one frame — this doc's "16 px/tick open" was wrong). Strings come from the `*.dat` language tables:
`Txt_LoadLanguage` 0x1caad0 loads `FNames[SelLang]` (the NTSC-U default is `USATxt.dat`; FNames[0] "DUTxt.dat" is
Dutch); layout: u32 dataBytes + data (padded `4 - (dataBytes & 3)`, 4 even when aligned), u32 NumEntries,
u32 offsets[NumEntries−1] (entry i ≥ 1 at data[offsets[i−1]]), u32 NumFixups + fixups[]; `Txt_BindLabel` 0x1cad60
= fixups[id>>24] + (id & 0xffffff). Pickup_Handler 0x18e650: 0x200000D "Picked up %s", 0x200000E "Picked up %dx %s",
0x200000F armour, 0x2000010 key, 0x200004A bonus; sounds 0xF5/0x141/0x14C. fix-hud ports the queue over the port's
`drones.taken` events (Sim::hud_tick per tick).

## Port

nf-view `hud.rs` (WORKLOG 0046): crosshair, health / armour segments and ring, damage glow, ammo text and strips, from
the ELF tables and the level's textures; layer-sorted runs, additive where flagged. fix-hud (WORKLOG 0090) added the
fire-mode / weapon-name labels, the pickup message bar, the damage-direction glow (BL+0x967 bits → sprites 20-23),
the air pane (AirSprInfo 0x2bd490) and the crouch icon (sprite 19). Not yet: objective/status message callers
(Mission_MonitorObjectives 0x1827d8 is unported), the gadget icon area (no HUD_Update* function identified yet),
blood / scope panes beyond the sight overlay.
