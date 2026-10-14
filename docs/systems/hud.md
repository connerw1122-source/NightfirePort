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
- Also: Bond-moment icon (16) — not ported; context icon (18) and crouch icon (19, 0x03000173 / 0x03000174) — see the sections below.

## Context icon (health pane sprite 18) [M] (ported by fix-context-hints)

`HUD_UpdateHealthPane` 0x19b9c8:273-303, driven by `BLData+0x95F` (the third-icon context, docs/fields/bldata.md;
the Sim writes it per tick from the level's third icons — fix-wire2/fix-08-walls):

- Set (0x95F != 0xFF): the texture is the table at **0x2FC168**[ctx] = `0x03000064 + ctx` [L ELF dump; the
  decompile's 8-byte stack slots are the decompiler's — the disassembly indexes 4-byte entries
  (`0x19c104 addu v1,sp,v0` over `id*4`)]. `hashtable_set_sprite` 0x1ca3d8 fills w/h and uv with the texture's
  own size on success; a miss (0, or an id whose texture is not loaded) leaves the sprite hidden. Only three of
  the seven table textures exist on the disc [D tex-list]: 0x03000066 (winch — ctx 2, the Q-Claw grapple), 0x03000067
  (hanging man — ctx 3, the wire grab), 0x0300006A (standing figure — ctx 6, use); 0x64/65/68/69 never resolve.
- Position: the runtime x/y are written **(400, 48)** every shown frame [M disasm 0x19c124-0x19c14c] — the same
  runtime override the crouch icon uses (template (320, −300), off-screen). Layer 29, colour 0x7f7f7fff.
  `ThirdIconTimer` (0x30CCE8) = FRAME_RATE·0.5 = 30.
- Cleared (0x95F == 0xFF): the icon fades out over ThirdIconTimer, alpha = timer·255/30, colour 0x7f7f7f00|alpha
  [M 0x19c160-0x19c1d4]; hidden once it reaches 0.
- The template: `HealthSprInfo[18]` tex 0x0300006A, layer 255 (hidden), (320, −300), w/h 0 [L 0x2BD368].
- **Position caveat**: the task's PCSX2 observation described the indicator "at the bottom-left ring"; the code
  draws sprite 18 at (400, 48). The live check (below) settles the screen position from the real machine.
- The Q-Claw marker (ammo-pane sprite 7, flags 0x900, 3D-positioned) turns magenta (0xff00ff) while the context
  is 2 [M HUD_UpdateAmmoPane 0x19a7f0:330-336] — not ported (the marker itself is not drawn).

## Hint / info bar (pane 4, TXTMSG type 1) [M/L] (ported by fix-context-hints)

The TOP status bar (`MsgInfoStatusPane` **0x319810** [L]: x 128, w 480, create 0x1a10b8, update 0x19c310,
`InfoStatusSprInfo` 0x319730, 5 sprites, 4 secondary slots, slide 0x10) — the same
`HUD_UpdateStatusPane` 0x19c310 state machine as the pickup/objective/mission panes:

- `InfoStatusSprInfo` [L ee_start_05.bin]: [0] the text sprite (template (128, 16), style 0x30cdb0 = centre
  justify, colour 0x7D6D59 + black shadow, layer 29), [2] the bar (480×26 at (16, 0) — the screen's top edge,
  uv (190,0,1,56) of 0x0300007A, colour 0x7F7F7F alpha 0x8D, layer 30), [3]/[4] the 5-px caps (uv 185/191)
  riding the bar's ends [M 0x19c310:342-347]. The text's screen x = template 128 + pane 128 = 256, centred;
  the wrap width is `TextMsgFormats[1]`'s 480 [L 0x2BF110+4 = 0x1e0] — the layout is {u32 style, u16 maxWidth}
  with maxWidth at +4 (the doc's +6 was wrong; fixed from the snapshot).
- Message sources (TXTMSG type 1, life 0xB4 = 180 ticks):
  - **Hint markers** — placement kind 0x33, `Hint_Create` 0x1b3248 (class 0x43): instance {param 1 = the sound
    id (`Sound_Play(100.0, sound, 0, 0)`), param 0 = the label, param 3 = the on-channel, param 4 = the
    off-channel}; `Hint_Update` 0x1b3380 fires ONCE when the player is in the marker's obj+0xD0 chain with the
    channels satisfied (on-channel 0 = always; off-channel non-zero and ON suppresses), posts the message and
    deletes itself (obj+0xFE bit 0). 07000008 has two: 0x0200000A "Hint: Use Watch Laser" by the safe and
    0x010001E9 "Press Action to open the control panel" [D USATxt.dat entries 1822/1489].
  - **`ThirdIcon_Update` 0x196e80's type-6 hint**: while a ctx-6 icon has the player in its chain and the
    player's substate != 7 (creep), it posts label **0x1000291** "Press Action" [D USATxt.dat entry fixups[1]+0x291]
    on the icon instance's 4-tick timer (data+8: −1/tick every frame, reset to 4 per in-range write — so the
    message posts once per entry into the icon's region) [M 0x196e80:15-17,38-53].
- `Txt_BindLabel` 0x1cad60 is INDEX-based: `fixups[id>>24] + (id & 0xffffff)` = the string index into the loaded
  bank's pointer table (`Bank[i]`, i ≥ 1; 0 = the heap sentinel), NOT a byte offset — the earlier byte-offset
  reading was wrong. Fixups are per-bank string-index bases ([0, 1000, 1812, …] on USATxt.dat [L]).

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
the air pane (AirSprInfo 0x2bd490) and the crouch icon (sprite 19). fix-context-hints (WORKLOG 0100h) added the
context icon (sprite 18, the 0x2FC168 texture table) and the INFO hint bar (pane 4) with the Hint markers
(placement 0x33) and the third icons' type-6 "Press Action" as its message sources. Not yet: objective/status
message callers beyond what fix-hud ports, the Q-Claw marker's 3D draw + magenta context tint
(HUD_UpdateAmmoPane 0x19a7f0:301-350), the Bond-moment flash (sprite 16, `PlrStat_DoneBondMoment` 0x199448 →
BL+0x948), blood / scope panes beyond the sight overlay.
