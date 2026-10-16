# The HUD pane structs and the `*SprInfo` tables

Proof levels: see [README.md](README.md). Builds on [sprite.md](sprite.md) (the runtime sprite and the
pane list walk). Data sources: the Ghidra decompile `work/decomp/ACTION.ELF` [M file:line]; the EE
snapshot `work/ee_start_05.bin` (file offset = EE address) [L]; the ELF file itself (LOAD: vaddr −
0x106100 = file offset) cross-checked against the snapshot for every table below [L-xcheck].

## HUDPANE_tag (0x20 bytes, the per-pane runtime record at `HUD+8 + k·0x20`)

`HUD_Init` 0x199e88 mallocs 0x2C8 = 8 + 22·0x20 at `*(BL+0x7E0)`, creates the crosshair sprite from
`CrossHair` 0x2BCDF8 into HUD+0, then walks `PaneList`/`MPPaneList` (22 entries) [M 0x199e88:18-63].

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | ptr | → its `HUDPANECREATE_tag` (re-read each frame: `HUD_Update` re-applies the template layer from CREATE+0x10's table [M 0x19a088:51-53]; the status pane compares it against `&MsgMissionStatusPane`/`&MsgObjectiveStatusPane` [M 0x19c310:53,57]; the radar's blip cap is CREATE+0x16 [M 0x19f6e8:108,153]). [L] pane[0]+0 = 0x2bcfb8 = `AmmoPane` | 
| +0x04 | ptr | the secondary u32 array (count = CREATE+0x16), malloc'd with the sprite array, zeroed [M 0x199b70:25-39]. Per pane: status panes = message state slots (below); radar = 24 runtime blips [M 0x19f4e8:45-59]; redeemer = 3D reticle objs [M 0x19dec0:55-63]; camera = 1 slot [M 0x1a12d8:19-29]; the ammo pane's 4 slots have no reader found (honesty) |
| +0x08 | ptr | the sprite-pointer array (`malloc((count+count2)·4)`: sprites [0..count), secondary at base+count·4) [M 0x199b70:25-27] |
| +0x0C | fn | the update fn (copied from CREATE+0x0C), called `(BL, pane, obj)` when non-0 [M 0x199b70:42, 0x19a088:38-41] |
| +0x10 / +0x12 | s16 | pane x / y (from CREATE+0x00/+0x02 after `HUD_ValidateXY`). The ammo pane reuses +0x12 as the bullet-strip bottom anchor [M 0x19a7f0:260,263,287,291] | 
| +0x14 / +0x16 | s16 | pane w / h (from CREATE+0x04/+0x06 after `HUD_CalcWidthHeight` 0x19a598:20-63 — max(x+w)−min(x) over sprites with flag 0x1000 clear) |
| +0x18 | u16 | sprite count = CREATE+0x14 (the update fns gate per-sprite work on it: >1, ≥3, >5, >6, >7 — how 0x19a7f0 serves both the 8-sprite SP and 3-sprite MP tables [M 0x19a7f0:172,199,211,247,275,301]) |
| +0x1A | u16 | written 0 by `HUD_Enable` (`sh a3,34(v0)`, all callers pass 0); no reader found |
| +0x1C | u8 | enabled: 0 → `HUD_Update` sets every sprite's +0x38 = 0xFF; 1 → +0x38 = the template layer [M 0x19a088:22-55]; early-outs the ammo/camera/status updates [M 0x19a7f0:103, 0x1a12d8:14, 0x19c310:30]; written by `HUD_Enable` 0x1a0e60, init 1 [M 0x199e88:55] |
| +0x1D/+0x1E | u8 | zeroed at init; no reader found [M 0x199e88:56] |
| +0x1F | u8 | the "created" flag (1 = the create fn ran; 0 = a NULL PaneList slot); `HUD_Enable` refuses when 0 [M 0x199e88:45-46,54; S 0x1a0e80-84] |

`HUD_Reset` 0x1a0d38 = 13 × `HUD_Enable(HUD, k, 0, 0)` over panes {6,7,5,9,10,11,12,15,16,17,18,19,20} [M 0x1a0d38:6-18].

## HUDPANECREATE_tag (0x1C = 28 bytes; the `*Pane` / `Msg*Pane` statics)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 / +0x02 | s16 | the pane x / y | [M 0x199b70:44-45] |
| +0x04 / +0x06 | s16 | w / h — **overwritten by `HUD_CalcWidthHeight`** before `HUD_CreateDefault` [M 0x1a0ed8:7, 0x19a598:20-63] |
| +0x08 | u32 | the create fn; 0 = the PaneList slot is unused [M 0x199e88:50,59] |
| +0x0C | u32 | the update fn → HUDPANE+0x0C [M 0x199b70:29,42] |
| +0x10 | u32 | → the `*SprInfo` template array [M 0x199b70:60, 0x19a598:26] |
| +0x14 | u16 | the sprite count [M 0x199b70:23] |
| +0x16 | u16 | the secondary-slot count [M 0x199b70:25-39] |
| +0x18 | u16 | `HUD_ValidateXY` alignment: 0x08 = clamp to the right edge, 0x10 = clamp to the bottom, 0x80/0x100 = add the viewer's screen x/y (split-screen) [M 0x199b70:54-56, 0x19a250:30-114] |
| +0x1A | u16 | 0 in all 40 decoded CREATE tags [L data] |

Worked example [L data + M]: `AmmoPane` = {512, 448, w→0, h→38, 0x1a0ed8, 0x19a7f0, `AmmoSprInfo`, 8, 4,
0x18} → ValidateXY x = (512−16)−0 = **496**, y = (448−16)−38 = **394** — exactly the port's
`AMMO_AT (496,394)` [nf-view/src/hud.rs:27]. HealthPane {16,337, flags 0} → `HEALTH_AT (16,337)` ✓.

## The pane lists ([L] decoded from 0x2be5b8 `PaneList` / 0x2be9e0 `MPPaneList`)

SP k → CREATE (update fn, SprInfo table, n·n2, flags): 0 AmmoPane (0x19a7f0, AmmoSprInfo, 8·4, 0x18) ·
1 HealthPane (0x19b9c8, HealthSprInfo, 24·0) · 2 MsgMissionStatusPane (0x19c310, MissionStatus, 3·4, 0x10) ·
3 MsgObjectiveStatusPane (0x19c310, Objective, 6·4, 0x10) · 4 MsgInfoStatusPane (0x19c310, Info, 5·4, 0x10) ·
5 AirPane (0x1a11c8, AirSprInfo, 3·0) · 6 SightPane (0x1a12d0 — an 8-byte no-op [M], SightSprInfo, 3·0, 0x180) ·
7 NightSightPane (0x19d218, NightSight, 6·0, 0x180) · 8 LensFlarePane (0x19d770, LensFlare, 7·0) ·
9 RedeemerPane (0x19e178, Redeemer, 11·4, 0x180) · 10 RCCarPane (0x1a15a0, RCCar, 6·0, 0x180) ·
11 CameraPane (0x1a12d8, Camera, 9·1) · 12 BloodPane (0x19a6a8, Blood, 2·0, 0x180) · 13,14 NULL ·
15 XrayPane (0x1a1410, Xray, 5·0) · 16 SecCamPane (0x1a1538, SecCam, 7·0) · 17 OICWPane (0x19cef8, OICW, 7·0, 0x180) ·
18 RoninPane (0x1a12d0, Ronin, 6·0, 0x180) · 19 LaserPane (0x1a12d0, Laser, 6·0, 0x180) ·
20 SpacePane (0x19d3b8, Space, 11·0) · 21 MsgPickupStatusPane (0x19c310, Pickup, 5·4, 0x18).
MP list: 0 MPAmmoPane (AmmoSprInfo consumer with MPAmmoSprInfo, 3·4, 0x18) · 1 MPHealthPane (0x19b630,
MPHealth, 8·0, 0x180) · 4 MPMsgInfoStatusPane (0x19ca88, MPInfoStatus, 1·4, 0x180) · 6 Sight · 9 Redeemer ·
10 RCCar · 12 Blood · 13 MPScorePane 0x2be9c0 (0x19f0a0, MPScoreInfo, 7·0, 0x180) · 14 RadarPane (0x19f6e8,
RadarSprInfo, **1·24**) · 17 OICW · 18 Ronin · 19 Laser; rest NULL.

## The SprInfo tables — 26 tables, every one a clean 0x2C-stride array

| Table | Addr | Size | Entries | Consumers | Port extracts? |
|---|---|---|---|---|---|
| CrossHair | 0x2BCDF8 | 44 | 1 | HUD_Init → HUD+0; HUD_UpdateCrossHair 0x19e6d0 | hardcoded |
| AmmoSprInfo | 0x2BCE58 | 352 | 8 | pane 0 → 0x19a7f0 | ✓ |
| BloodSprInfo | 0x2BCFD8 | 88 | 2 | pane 12 → 0x19a6a8 | – |
| HealthSprInfo | 0x2BD050 | 1056 | **24** | pane 1 → 0x19b9c8 | ✓ |
| AirSprInfo | 0x2BD490 | 132 | 3 | pane 5 → 0x1a11c8 | ✓ |
| SightSprInfo | 0x2BD558 | 132 | 3 | pane 6 (no-op) | tex hardcoded |
| CameraSprInfo | 0x2BD600 | 396 | 9 | pane 11 → 0x1a12d8 | – |
| NightSightSprInfo | 0x2BD7B0 | 264 | 6 | pane 7 → 0x19d218 | – |
| LensFlareSprInfo | 0x2BD8D8 | 308 | 7 | pane 8 → 0x19d770 | – |
| RedeemerSprInfo | 0x2BDA30 | 484 | 11 | pane 9 → 0x19e178 | – |
| RCCarSprInfo | 0x2BDC38 | 264 | 6 | pane 10 → 0x1a15a0 | – |
| RadarSprInfo | 0x2BDD60 | 44 | 1 | MP pane 14 → 0x19f6e8 (+24 blips) | – |
| XraySprInfo | 0x2BDDB0 | 220 | 5 | pane 15 → 0x1a1410 | – |
| SecCamSprInfo | 0x2BDEB0 | 308 | 7 | pane 16 → 0x1a1538 | – |
| OICWSprInfo | 0x2BE008 | 308 | 7 | pane 17 → 0x19cef8 | – |
| RoninSprInfo | 0x2BE160 | 264 | 6 | pane 18 → 0x1a12d0 | – |
| LaserSprInfo | 0x2BE288 | 264 | 6 | pane 19 → 0x1a12d0 | – |
| SpaceSprInfo | 0x2BE3B0 | 484 | 11 | pane 20 → 0x19d3b8 | ✓ (fix-space-pane: `nf_view::hud::space_pane`; `MissileDeploy` not modelled) |
| MPAmmoSprInfo | 0x2BE610 | 132 | 3 | MP pane 0 → 0x19a7f0 | – |
| MPHealthSprInfo | 0x2BE6B8 | 352 | 8 | MP pane 1 → 0x19b630 | – |
| MPInfoStatusSprInfo | 0x2BE838 | 44 | 1 | MP pane 4 → 0x19ca88 | – |
| MPScoreInfo | 0x2BE888 | 308 | 7 | MP pane 13 → 0x19f0a0 | – |
| MissionStatusSprInfo (BSS) | 0x319580 | 132 | 3 | pane 2 → 0x19c310 | – |
| ObjectiveStatusSprInfo (BSS) | 0x319608 | 264 | 6 | pane 3 → 0x19c310 | – |
| InfoStatusSprInfo (BSS) | 0x319730 | 220 | 5 | pane 4 → 0x19c310 | – |
| PickupStatusSprInfo (BSS) | 0x319830 | 220 | 5 | pane 21 → 0x19c310 | – |

Every symtab size = entries·0x2C exactly (1056 = 24·44 — the "not divisible" worry was wrong); every
CREATE tag's n matches its table; the BSS extents sit exactly before the following `Msg*Pane` [L adjacency].

The runtime mapping is `Sprite_Create2` 0x1de3d8 [M]: +0x00→rt+0x10 colour, +0x04→rt+0x14 colour2,
**+0x08 (u8)→rt+0x38 layer**, +0x0A (u16)→rt+0x22 flags, +0x0C/+0x0E→rt+0x24/26 x/y, +0x10/+0x12→rt+0x2C/2E
w/h (0 = the texture's size), +0x14..+0x1A→rt+0x30..36 uv (uw/vh 0 → texW−1/texH−1), +0x1C label
(−1 = texture sprite; 0 = run-time text; 0x02xxxxxx = static label), +0x20 the text-style pointer,
+0x24 the texture hash, **+0x28 (s8) the viewer link index — 5 (the overlay viewer) for every entry of
every table** [L census]. Decoded samples (identical to the port decoder's output):

```
HealthSprInfo[0] col=7d6d59ff col2=7f7f7fff layer=19 x=32 y=30 w=32 h=32 uv=(5,219,32,32) label=-1 tex=0300007a
AmmoSprInfo[0]   col=7d6d59ff col2=000000ff layer=28 flags=1020 label=0 style=0x30cc78   (ammo-count text)
AmmoSprInfo[4..6] tex 03000002/03000128 strips, flags 0x1000 (clip-to-screen → excluded from CalcWidthHeight)
AmmoSprInfo[7]   layer=31 flags=0900 tex=0300004c (the Q-claw marker, positioned 3D→screen by 0x19a7f0:301-350)
CrossHair[0]     col=ff0000ff layer=49(0x31) flags=0800 tex=03000026
```

**The BSS status tables** are boot-filled by `__static_initialization_and_destruction_0` 0x1a0058 [M
0x1a0058:77-104]: entry 0 built on the stack, the rest byte-copied from **0x30-stride source records at
0x2FC2E0 (mission, 2) / 0x2FC340 (objective, 3) / 0x2FC3D0 (info, 3) / 0x2FC460 (pickup, 3)** — the same
field layout plus 4 zero bytes at +0x2C. [L] the BSS dump matches the reconstruction exactly.

**`TextMsgFormats` 0x2BF108** (56 B) = 7 × {u32 stylePtr, u16 maxWidth}: the status CREATE tags' widths are
TextMsgFormats[type]·maxWidth (480/480/256) [L data; M the 0x19c310 family]. The status panes' secondary
array: [0] = the TXT_MSG out-param, [1] = the state machine 0..2 [M 0x19c310:61,102,110,310,313], [2] =
the TXTMSG_TYPE (3 mission / 2 objective / 1 info / 6 pickup) [S 0x1a1040-0x1a1140], [3] = 0x20 mission /
0x10 objective-info-pickup — writer proven, reader not identified (honesty).

## AmmoPane (0x2BCFB8) / MPAmmoPane (0x2BE698), 28 B each

They are the SP/MP ammo pane's **HUDPANECREATE_tag**s (hence exactly 0x1C), pointed at by PaneList[0] /
MPPaneList[0] [L data]; `HUD_CreateAmmoPane` 0x1a0ed8 = `HUD_CalcWidthHeight` + `HUD_CreateDefault` only.
SP {512, 448, 0, 38, 0x1a0ed8, 0x19a7f0, 0x2BCE58, 8, 4, 0x18}; MP {512, 448, 0, 26, 0x1a0ed8, 0x19a7f0,
0x2BE610, 3, 4, 0x18}. The ammo pane's runtime side reads pane+0x1C (enabled), +0x08 (sprite array:
0/1/2/5/6/7 = count text, fire mode, weapon name, filled strip, strip background, Q marker, gated by
pane+0x18's count), +0x12 (the strip bottom anchor) and `BulletImg.189` f32[4] = {u, v, width,
pixels-per-round} [M 0x19a7f0:103,116-123,172,199,211,247,254-269,275,287,301].

## Port status (nf-formats/src/hud.rs → nf-view/src/hud.rs)

- The decoder's field map is correct (verified against `Sprite_Create2`; layer-as-i16 harmless — the u8 at
  +8 with byte +9 always 0). `HudTables` extracts Health/Ammo/Air + `BulletImg.189` + the font; the other
  23 tables are unconsumed (the crosshair/scope are hardcoded — consistent with the pane design).
- **Flags 0x1000 (clip-to-screen) and 0x800 are not handled** by the port's flag mapping (0x100/0x200
  blend, 0x40/0x80 UV flip, 0x20 right-align); the ammo layout depends on 0x1000's CalcWidthHeight
  exclusion — worth a comment/test if the pane system is ever generalised.
- Not decoded: template +0x20 (style) and +0x28 (viewer link) — no functional impact today.

## Live checks for the main agent

1. **The pane count gate**: `watch 0x016c8638+0x18 2 readwrite` (pane[0]'s count = 8) during a weapon
   switch — `HUD_UpdateAmmoPane`'s per-sprite gates (>1, ≥3, >5, >6, >7) should be the only readers.
2. **The 0x1000 exclusion**: poke an ammo strip sprite's template flags to clear 0x1000 (AmmoSprInfo[4]+0xA
   at 0x2bce58+4·0x2C+0xA) before a pane rebuild — the pane's h should grow by the strip's height and the
   ammo strip should jump to a clamped position (proves `HUD_CalcWidthHeight` reads CREATE, not the runtime).

## The census to run

`gdbwatch 0x016c8630 0x2c8 readwrite --seconds 6` on 07000005 while switching weapons and taking damage:
the expected accessors are HUD_Update (0x19a088), HUD_UpdateAmmoPane (0x19a7f0), HUD_UpdateHealthPane
(0x19b9c8) and the crosshair path (0x19e6d0). A reader outside {0x19a088, 0x19a7f0, 0x19b9c8, 0x19e6d0,
0x1a0e60, 0x1a0d38, 0x1afcc8, 0x191a70, 0x195928, 0x190bb0, 0x1ac2c8} means a missed accessor.
