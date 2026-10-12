# `ammo_data` — the ammo-type table (34 × 12 bytes at EE 0x2c69b8)

Proof levels: see [README.md](README.md). Evidence: [M] the decompile (file:line or `mipsdis` addresses);
[L] `work/ee_start_05.bin` at 0x2c69b8 (byte-identical to the ELF's data — the table is pure constant
data; every reference in ACTION.ELF is a read). It follows `weapon_data` (0x2bf150 + 30,820 = 0x2c69b4)
with 4 bytes of unrelated padding. Port: `crates/nf-formats/src/weapon.rs` parses it (`ammo_cap` = +2,
`ammo_type()` = weapon_data+0x90, `ammo_name_label()` = +8).

## Record layout (34 records × 12 bytes; indices 0..32 live — `Player_InitAmmoWeapons` copies 0x21 = 33; record 33 all-zero [L])

| Offset | Type | Meaning | Proof | [L] samples |
|---|---|---|---|---|
| +0x0 | u16 | **unused in ACTION.ELF** — no load anywhere (all 27 refs checked); the values (20/25/24/15/10/…/1) suggest rounds-per-pickup-box, but the pickup amount actually comes from PICKUPINFO+0x26 [M Pickup_Handler 0x18e650:207,211] | repo-wide rg | rec1 = 20, rec9 = 7 |
| +0x2 | s16 | **max carried rounds** — also the reserve's seed under the all-weapons cheat (DAT_002a373c, set by cheat handler C_CHCHWEAP 0x215dc8:15) | [M1 Player_EquipAmmo 0x1a8d18:37,74 (the clamp), Player_InitAmmoWeapons 0x1a17f8:16-29] | rec1 = 70, rec2 = 90, rec14 = 4 |
| +0x4 | u32 | **the shell-casing gfx object id** (packed id, 0 = none), passed to `hashtable_set_object_to_entity_gfx` | [M1 Bullet_init_casing 0x12fde0:15, Bullet_init_casing_ex 0x12f678:83] | rec1 = 0x0200043F, rec3 = 0x02000445, rec9 = 0x02000548 |
| +0x8 | u32 | **the ammo name text label id** (packed `group<<16\|index`, group 0x0500) for the "picked up X" message | [M1 Pickup_Handler 0x18e650:265-284 (`Txt_BindLabel`)] | rec1 = 0x05000025, rec2 = 0x05000026, rec19 = 0x05000075 |

## The BL+0x170 index mapping

- The player's `BL+0x170` = s16 reserve[33], indexed by ammo type (= the record index) [M1 0x1a8d18:37,69-78].
- Seed: `Player_InitAmmoWeapons` 0x1a17f8:16-29 copies `ammo_data[i].max → BL+0x170+i·2` for i in 0..32
  **only when the all-weapons cheat is on**; otherwise the reserve starts at 0 (pickups fill it).
- Spend/receive: `Player_EquipAmmo(BL, weapon, n)`: type = `weapon_data[w]+0x90`; reserve += n then clamp
  to `ammo_data[type].max` [M1 0x1a8d18:72-78]. Clip weapons (`weapon_data+0x68 & 0x10`) fill the empty
  clip first up to `weapon_data+0x92` [M1 :39-64]. `Player_ReloadAmmoType` 0x1a1988:23-38 moves
  reserve→clip the same way. The bots mirror it exactly (Drone reserve BL+0x698, clip slots Drone BL+0x144
  [M1 BOTWEAP_EquipAmmo 0x12a308]).
- Twin weapons share a clip slot when `weapon_data+0x5 != 0` and both use the same ammo type [M1 0x1a9148:13-17].

## The weapon → ammo map (from the measured `weapon_data` +0x90 bytes [L])

| ammo | max | casing | label | weapons using it |
|---|---|---|---|---|
| 0 | 0 | 0 | 0 | 0,1,70-73,80-88,93-96,98-105,107,112,114 (unarmed/melee/dummy) |
| 1 | 70 | 0x0200043F | 0x05000025 | 2,3,4,5 |
| 2 | 90 | 0x0200043F | 0x05000026 | 6-13,18-21 |
| 3 | 36 | 0x02000445 | 0x0500001F | 14,15 |
| 4 | 50 | 0 | 0x05000016 | — (no weapon points here) |
| 5 | 90 | 0x02000447 | 0x05000022 | 22-26 |
| 6 | 100 | 0x02000446 | 0x05000023 | — |
| 7 | 30 | 0x02000444 | 0x0500001D | 30,32,34,36,38,40 |
| 8 | 30 | 0x02000444 | 0x0500001E | 31,33,35,37,39,41 |
| 9 | 24 | 0x02000548 | 0x05000017 | 28,29 |
| 10 | 32 | 0x02000443 | 0x05000018 | — |
| 11 | 35 | 0x02000445 | 0x05000021 | 16,97,113 |
| 12 | 6 | 0 | 0x0500001C | 27 |
| 13 | 12 | 0 | 0x05000020 | 42,43 |
| 14 | 4 | 0 | 0x05000027 | 44-49,109 (grenades) |
| 15 | 21 | 0 | 0x05000019 | 17 |
| 16 | 20 | 0 | 0x0500001A | — |
| 17 | 20 | 0 | 0x0500001B | — |
| 18 | 10 | 0 | 0x05000024 | 67,68 |
| 19 | 9 | 0 | 0x05000075 | 66 |
| 20 | 0 | 0 | 0 | 74-77 |
| 21 | 9 | 0 | 0x05000024 | 65 |
| 22 | 4 | 0 | 0x0500003E | 55-57 |
| 23 | 9 | 0 | 0x0500003F | 53 |
| 24 | 9 | 0 | 0x0500003C | 54 |
| 25 | 9 | 0 | 0x05000031 | 52 |
| 26 | 0 | 0 | 0 | 78,79 |
| 27 | 0 | 0 | 0x05000040 | 82 |
| 28 | 4 | 0 | 0x05000049 | 58 |
| 29 | 4 | 0 | 0x0500004C | 59-64,108 |
| 30 | 0 | 0 | 0x0500004B | 89-92 |
| 31 | 0 | 0 | 0 | 50,51,106,110,111 |
| 32 | 0 | 0 | 0 | 69 (the 0x45 special) |
| 33 | 0 | 0 | 0 | — (allocated, unused) |

Records 4/6/10/16/17 are reachable by no +0x90 byte — only their caps/labels would ever be consumed [M1,L].

## Port status

Parsed (+2 cap, +8 label, the +0x90 link) [M weapon.rs:14-18,33-37,93-100]. **Missing: the +4 casing gfx
id** (the `Bullet_init_casing` path — the port has no shell casings). The reserve/clip/clamp arithmetic is
modelled in `Inventory`/`gun.rs`; the cheat seed (DAT_002a373c) is not.

## Live checks for the main agent

1. **The clamp**: pick up ammo over the cap on 07000005 (`watch 0x016cf0f0+0x170+2·1 2` = reserve[1]) —
   it must stop at 70 (rec 1's max) regardless of further pickups.
2. **The casing id**: fire the P2K (ammo 1) — the eject mesh `hashtable_set_object_to_entity_gfx` receives
   0x0200043F; the port has no casings, so this is the main agent's check when casings get ported.

## The census to run

`gdbwatch 0x2c69b8 0x198 read --seconds 6` on 07000005 during a shoot-reload-pickup pass: the expected
readers are Player_EquipAmmo (0x1a8d18), Player_EquipWeapon (0x1a8a58), Player_InitAmmoWeapons (0x1a17f8),
Pickup_Handler (0x18e650), Bullet_init_casing_ex (0x12f678), Bullet_init_casing (0x12fde0) and the
BOTWEAP mirrors. Anything else means a missed reader.
