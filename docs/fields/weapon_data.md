# `weapon_data` — the full record map (0x10C bytes × 115, table at EE 0x2bf150)

Proof levels: see [README.md](README.md). Companion to `docs/formats/weapon-data.md` (the extraction
story and the muzzle-flash walkthrough — read that first). Evidence: [M] the decompile (file:line or
`mipsdis` addresses); [L] the measured table `/tmp`-dumped from `work/ee_start_05.bin` at 0x2bf150
(record 1 +0xDC = 0x050000AB is the runtime skin write; record 0 all-zero; the ELF bytes are pre-ctor —
the static initialiser `gcc2_compiled.` 0x1b3488 fills the table at boot, incl. the record-0 defaults
+0x50=0xFF, +0x92=0xFFFFFFFF, +0x68=0x20, +0x64=0xE10 and the −1 sentinel fields).

The reader inventory: 58 decompiled files reference `weapon_data`; the top consumers are
`Player_WeaponFiring` 0x1a5460 (+0x02,0x05,0x1C,0x40,0x44,0x68,0x6C,0x7C,0x90,0x91,0x92,0x94,0xA4,0xA8,0xAC,0xB4,0xDC),
`Player_SetWeaponAnimObj` 0x1a2e38 (+0x02,0x44,0x68,0x88,0xA0..0xDC), `Player_Weapon` 0x1a24c8,
`Player_EquipWeapon`/`EquipAmmo` 0x1a8a58/0x1a8d18, `Player_ReloadAmmoType` 0x1a1988,
`Player_InitAmmoWeapons` 0x1a17f8 (the full 115 walk), `Player_WeaponHasAmmo` 0x1a16c0,
`Player_MuzzleFlash`/`Draw_MuzzleFlash` 0x1a4298/0x1a3fe0, `Pickup_Handler` 0x18e650 (+0x38,+0x3C,+0x80,+0x90,+0x92),
`AnimDatumGetWeaponInfo` 0x1c7900 (+0x4C,+0x50,+0x60,+0x80), `Bullet_init_casing_ex` 0x12f678
(+0x68&0x40000, +0x90, +0xF8..0x100), `DroneWeap_FireWeapon` 0x178f68, the BOTWEAP_* mirrors
(0x12a308/0x12a138/0x12a028/0x129d28/0x12aa14/0x12aa4c), `Player_Update` 0x193600 (the regen clip sizes)
and `Camera_ApplySwing` (+0x8C via 0x194134).

## Field table

| Offset | Type | Meaning | Proof | [L] samples (w = 02 / 16 / 24 / 45 / 4A / 71) | Port |
|---|---|---|---|---|---|
| +0x00 | u16 | weapon id (= index) | every record | 2 / 0x16 / 0x24 / 0x45 / 0x4A / 0x71 | ✓ |
| +0x02 | u16 | base weapon (variants → the plain one; the clip/owned slot selector in EquipWeapon) | [M 0x1a8a58:48-55] | 2 / 0x16 / 0x24 / 0x45 / 0x4A / 0x71 | ✓ |
| +0x04 | u8 | selectable/ownable (the cycle's gate with owned + has-ammo) | [M1 `InitAmmoWeapons` walk `lbu 4(v0)` 0x1a1894; 0x1a6f80] | 1,1,1,1,1,**0** | ✓ |
| +0x05 | s8 | twin/variant record link (added to the index when the ammo types match — `Player_AmmoIndex` 0x1a9148:13-17); runtime-set rec 0x37 = 1/2 by `Player_Init` | [M1 0x1a557c `lb 5(s3)`; 20+ fns] | 1,1,1,0,0,0 | ✓ (bool only) |
| +0x06 | u8 | the weapon class; **4 = explosive** | [M1 BOTWEAP_tooCloseForWeapon 0x12aa14 `lbu 6(v0)`; 0x129580, 0x126b88] | 1,2,3,0,0,0 | not modeled |
| +0x08 | f32 | min engagement range / blast radius (the bots' range test) | [M1 0x12aa4c `lwc1 f0,8(v1)`; 0x12a538] | 0,0,0,0,0,0 | not modeled |
| +0x0C | f32 | **damage per hit** | [M1 `DroneInit_TakeXHits` 0x14d944 `lwc1 f0,12(v0)`; docs/formats/weapon-data.md] | 3.5, 2.75, 30, 100, 0, 4 | ✓ (`+0x0C` accessor) |
| +0x10 | u8 | unknown (powers of two: 8/64/4/16); no reader found | — | 8,64,64,8,4,16 | — |
| +0x14 | f32 | the auto-aim accuracy % | [M1 0x1a7630 `lwc1 f0,20(v1)`] | 100,60,0,0,0,0 | ✓ |
| +0x18 | u8 | bullets fired per trigger loop | [M1 0x1793a4 `lbu 24(s3)`] | 1×6 | not modeled |
| +0x1C | f32 | max range (m) | [M* WeaponInitBullet 0x1a4bf0, AutoAim, TaserBeam (rec 76), HUD (rec 80)] | 50, 450, 1200, 1000, 5, 300 | not modeled (the port hardcodes ranges per weapon) |
| +0x20 / +0x24 | f32 | ? (rate-like 1/x patterns: 33.33, 15.35, 8.33; 10/100) — no reader found [S: shots/sec?] | — | — | — |
| +0x28 / +0x2A | u16/u16 | a fire-count cluster (999 sentinel on rec 0x4A); no direct reader found | — | 1/1, 3/3, 1/1, 1/1, 999/1, 50/1 | — |
| +0x2C | u16 | **bullets per shot (pellets)** | [M1 0x179e74 `lhu 44(v1)` (ThrowGrenade family)] | 1, 3, 1, 1, 1, **7** | not modeled |
| +0x2E | u8 | (=1 everywhere sampled); no reader found | — | 1×6 | — |
| +0x30..0x3F | 4×u32 | the fire-mode name-label ids (0x0500xxxx / 0x0200xxxx spaces): the HUD reads +0x30 + mode·4; +0x38 = the SP name, +0x3C = the MP name (Pickup_Handler case 0) | [M1 HUD_UpdateAmmoPane 0x19a7f0, 0x18e650:265-284] | 0x0500000e… | ✓ (+0x38; the MP +0x3C variant undocumented) |
| +0x40 | u32 | **the refire interval in frames** — the player path: u32 → float, max 1.0 → BL+0x92C; the bot path: s16 × FRAME_RATE_DIV | [M1 0x1a64b0-0x1a64e8, 0x179dc0] | 8, 4, …, 26, … | **not modeled** (the port's fire cadence is script-driven) |
| +0x44 | u16 | ? (non-zero → an alternate fire/anim path; compared as a float in SetWeaponAnimObj; rec 0x45 = 13) | [M1 0x1a586c `lhu 68(s3)`] | 0,0,0,**13**,0,0 | not modeled |
| +0x48 | u32 | the first-person muzzle-flash mesh | [M1 0x1a401c `lw 72(s3)`] | 0x020001F6, 0, … | ✓ `fp_flash_mesh` |
| +0x4C | u32 | the mesh hashcode passed out by `AnimDatumGetWeaponInfo` (the muzzle flash's third-person mesh; 0 = none) | [M1 0x1c7948] | ✓ per docs/formats | ✓ |
| +0x50 | u8 | the datum id the third-person model hangs on (0xFF = none → id 0 when drawn) | [M1 0x1c7980 `lbu 80(v0)`, `xori 0xff`] | 0x40, …, 0xFF | ✓ |
| +0x51 / +0x52 | u8/u8 | the muzzle-flash scale min/max (a Rand between them) | [M1 0x1a4234] | 0x40 / 0xC0 | not modeled |
| +0x54..0x56 | u8×3 | the flash light colour — passed to `Light_Create` as (r,g,b) = **(+0x56, +0x55, +0x54)** | [M1 0x1a4370 `lbu 86(s0)`; docs/formats/weapon-data.md] | 00 8A FF = orange | ✓ (colour order correct) |
| +0x58 | f32 | the flash light intensity/scale (`Light_Create`'s radius; 7.0 on the pistols) | [M1 0x1a3fe0 family] | 7.0, 7.0, 0, 0, 5.0, 0 | not modeled (the port lights 7 fixed) |
| +0x5C | u32 | the flying projectile's drawn mesh — `Bullet_update` 0x12e7b0 disasm 0x12e8f0 `lw a1,0x5c(v0)` → `hashtable_set_object_to_entity_gfx` (the grenades/rockets/darts/Q-Worm carry one; 0xFFFFFFFF misses the hashtable = none). **doc-fixed (dev-panel-spawns)**: was "no reader found" | [M1 0x12e8f0; docs/formats/weapon-data.md:27] | 0x34→0x020002af, 0x35→0x02000726, 0x36→0x0200071f, 0x37→0x02000a6e, 0x3a→0x0200074f | ✓ `projectile_mesh` |
| +0x60 | **u16** | **the view-anim datum id** (the third out-param of `AnimDatumGetWeaponInfo` beside +0x4C/+0x80); `lhu 96(v1)`; +0x62 = 0 in all 115 records | [M1 0x1c7964] | 1068, 1541, 1054, 1519, 0, 0 | **fixed (fields-player)**: was a u32 "fire_sound" accessor — retyped `view_anim_datum()` (values were right by luck) |
| +0x64 | u32 | ? (init default 0xE10 on rec 0; 3 on recs 2/0x71); no reader found | — | 3,0,0,0,0,3 | — |
| +0x68 | u32 | the flags: 0x2/0x4/0x8 laser-pointer-present [M1 0x1a9338], 0x10 = clip weapon [M1 0x1a8d18:39-64], 0x40 = no-auto-aim/no-crosshair [M1 Player_Aiming 0x1a1d28, HUD], 0x2000 AnimObjectAimAt, 0x20000 the bot's reload path, 0x40000 the Bullet fly-by; init default 0x20 | [M*] | 0x1004, 0x80, 0x4080D0, 0x180A0, 0x12480, 0 | ✓ `flags` |
| +0x6C | u32 | the bullet flags (0x40000 = the fly-by whiz in `Bullet_DoTrails` 0x12d890) | docs/formats/weapon-data.md | 0x40050, 0x40050, 0x40040, 0x20C08, 0x880, 0x1000 | ✓ |
| +0x70 / +0x74 / +0x78 | u32 | flags-like / timer (rec 0x24 +0x78 = 60); no reader found | — | — | — |
| +0x7C | u32 | the view-entity model hash (`AnimDatumSetEntity` per draw, `Player_WeaponFiring`) | [M1 0x1a5460] | 0x02000842, 0, … | no accessor |
| +0x80 | u32 | the third-person / pickup model hash | [M*] | 0x02000B44 (SG5) | ✓ |
| +0x84 | u8 | the aimable-part index (0 = `AnimObjectAimAt` skips the weapon) | [M1 0x1bff74 `lbu 132(v0)`] | 1, 3, 3, 0, 0, 0 | not modeled |
| +0x88 | **f32** | **the zoom magnification** — `SQRT` into the BL record lane 0; `Upgrade_Weapon` 0x1a8f30 writes 8.0f/16.0f into recs 84/85 | [M* 0x1a17f8:51, 0x1a8f30] | 1, 1, 10, 1, 1, 2 | ✓ f32 |
| +0x8C | f32 | the camera recoil/swing → `Camera_ApplySwing` | [M1 0x194134 `lwc1 f12,140(a1)`] | 0, …, 0.001 | not modeled |
| +0x90 | u8 | **the ammo-type index** into `ammo_data` (34 × 12 B at 0x2c69b8 — [ammo-data.md](ammo-data.md)) | [M1 0x1a6100 `lbu 145(s3)`, Player_AmmoIndex 0x1a9148] | 1, 5, 7, 32, 20, 11 | ✓ `ammo_type` |
| +0x91 | u8 | the clip-add rounds (how many the pickup/reload moves per unit) | [M1 0x1a8d18] | 1,1,1,100,1,1 | ✓ |
| +0x92 | u16 | **the clip capacity** (the reload/clip-init cap; the regen target) | [M*] | 7, 30, 5, 100, 100, 50 | ✓ `clip_size` |
| +0x94 | u8 | the fire rumble magnitude → `Input_RumbleStart(., 5, ·)` | [M1 0x1a648c] | 20, 50, 40, 60, 0, 0 | not modeled |
| +0x98 / +0x9C | f32 | ? (0.01..1.0 patterns); no reader found | — | 0/0.05, 1.0/0.04 | — |
| +0xA0..+0xD8 | u32 ×14 | the view-anim **script hashes (0x0600xxxx)**: +0xA0 idle, +0xA4 reload, +0xA8 fire-anim (`AnimScriptAdd`), +0xAC, +0xB0 fire, +0xB4 fire-last (the random pick with flag 0x80000), +0xB8 draw-at-1.25×, +0xBC alt, +0xC0 draw, +0xC4 holster (1.25×), +0xC8 variant-swap, **+0xCC idle-alert script** (non-zero = has the idle behaviour), +0xD0/+0xD4 idle-alert raise/lower, +0xD8 holster-variant script | [M* SetWeaponAnimObj cases, WeaponFiring, 0x1a2cc0, 0x181408 rec83+0xB4] | 0x06000825… contiguous | ✓ mostly (the port models +0xB0/+0xB4/+0xC0/+0xC4/+0xC8) — **+0xCC/+0xD8 doc-fixed (fields-player)**: they are script hashes, the "flag" tests work because script-present IS the flag |
| +0xDC | u32 | the first-person skin hash (rec 1 runtime-overridden via `PunchSkin`) | [M*] | 0x05000088 (P2K) | ✓ |
| +0xE0..0xEC | vec3 ×2 | +0xE0 = the first-person gun offset (view space); **+0xEC = a second vec3, no reader found** | [M1 0x12f810-0x12f840 (casing path reads +0xF8..)] | (0.03, 0.08, −0.9) / (0.024, 0.076, −0.903) | ✓ / — |
| +0xF8..0x100 | vec3 | the shell-casing eject vector (`Bullet_init_casing_ex` 0x12f678:83) | [M1] | (−0.01, −0.03, 0) | not modeled |
| +0xDB / +0xDF | u8 | the high bytes of the +0xD8/+0xDC words (no independent meaning) | — | — | — |

Record 0 is all-zero in the measured table (the ctor still writes its defaults into it — +0x50 = 0xFF,
+0x92 = 0xFFFF etc. — but every reader guards w < 115 and the id-0 paths are the unarmed/hands case).

## Zero / readerless columns (honesty)

No reader found at all: +0x10, +0x20, +0x24, +0x28/+0x2A (semantics), +0x2E, +0x64, +0x70/0x74/0x78,
+0x98/+0x9C (meaning), +0xEC vec3, +0x5C, +0xDB. The remaining "unknown columns" of the nonzero census
are byte lanes of the f32/u32 fields above (e.g. 0x0A-0x0F = the +0x08/+0x0C floats; 0x8A-0x8F = zoom+swing).

## Port status

`crates/nf-formats/src/weapon.rs` models the record for the extractor; `nf_game::gun` consumes
ammo_type/clip_size/flags/damage/zoom. Fixed this session: +0x60 retyped u16 (`view_anim_datum`),
+0xCC/+0xD8 doc-corrected to script hashes. Missing accessors (harmless today, listed for the next
consumer): +0x7C view-model hash, +0x8C camera swing, +0x94 rumble, +0x40 refire, +0x2C pellets, +0x18
rounds-per-shot, +0x06 class, +0x08 min-range, +0x1C range, +0x84 aimable, +0x58 flash intensity, +0xF8
casing vec3, +0xB8/+0xBC hashes.

## Live checks for the main agent

1. **+0x60 is a datum id**: in PCSX2, `AnimDatumGetWeaponInfo` 0x1c7900's third out-param should equal
   `weapon_data[w]+0x60` (u16) for any weapon draw — `watch 0x2bf150 + w·0x10C + 0x60 2` on a weapon
   pickup and compare with the mesh datum chosen for the flash.
2. **The refire (+0x40)**: fire the P2K (rec 2, +0x40 = 4) — `watch BL+0x92C 4` should reset to 1.0 on the
   first shot then to max(4·k, 1.0) per shot; the port's cadence is script-driven and will differ.
3. **The clip regen caps**: `watch 0x2bf150 + 0x45·0x10C + 0x92 2` (= 100) beside BL+0x4F4 — the belt
   should stop climbing exactly at the cap.

## The census to run

`gdbwatch 0x2bf150 0x7864 read --seconds 6` on 07000005 during a fire-switch-reload pass: the expected
readers are Player_WeaponFiring/SetWeaponAnimObj/Weapon/EquipAmmo/ReloadAmmoType/WeaponHasAmmo/HUD_UpdateAmmoPane
(+ the init walks at level load). A reader outside the inventory above means a missed field.
