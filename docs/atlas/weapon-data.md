# weapon_data atlas

The master weapon-definition table. 115 records × 0x10C (268) bytes = 30 820 bytes. Read by every player/drone/pickup
function that needs a weapon's model, clip, fire rate, scripts, or flags.

Tags: `[M]` decompile, `[L]` `work/ee_start_05.bin` snapshot, `[S]` suggested.

---

## Allocation site `[M]`

| symbol | addr | size | meaning | tag |
|---|---|---|---|---|
| `weapon_data` | `0x002bf150` | 30 820 (115 × 0x10C) | the weapon definition table | `[M]` symtab |
| `ammo_data` | `0x002c69b8` | 408 (34 × 12) | the ammo type table | `[M]` symtab |
| `BestWeapon` | `0x002bede0` | 36 (2 × 18) | the auto-switch priority table | `[M]` symtab |
| `PunchSkin.204` | `0x002b9430` | 16 (4 × 4) | the per-level fists-skin override table | `[M]` symtab |
| `_GLOBAL_$I$weapon_data` | `0x001b9900` | 32 B | the static initialiser | `[M]` symtab |

The bytes on the disc are zero. The table is filled at boot by the C++ static initialiser `_GLOBAL_$I$weapon_data`
(32 B), which calls `gcc2_compiled_(1, 0xffff)` (`0x1b3488`, 25 720 B: constant stores, unaligned 64-bit copies,
20 `memset` calls). The Rust port runs that initialiser over the user's ELF via `nf_formats::ee::Machine`
(`crates/nf-formats/src/weapon.rs:20` `WeaponData::from_action_elf`) `[M]`.

## Record size

0x10C (268) bytes per record. 30820 / 0x10C = 115.0 records exactly `[L]`. Confirmed by:

- `Player_InitWeapon`@`0x1a7ef8`:321 — `weapon_data + w * 0x10c + 2` `[M]`
- `Player_EquipWeapon`@`0x1a8a58`:18 — `iVar10 * 0x10c` `[M]`
- `Player_WeaponFiring`@`0x1a5460`:88 — `*(char *)(anim+0x62) * 0x10c` `[M]`
- `AnimDatumGetWeaponInfo`@`0x1c7900`:14 — bounds check `< 0x73` ⇒ 115 records (0x00..0x72) `[M]`

---

## Field table

Every field the port reads (cross-checked against the decompile) plus others verified from the snapshot.

| offset | type | name | meaning | writers (fn@addr) | readers (fn@addr) | tag |
|---:|:---:|---|---|---|---|:---:|
| +0x00 | u16 | `id` | record's own id (= index) | static init | every reader | `[M]` |
| +0x02 | u16 | `base` | base weapon a variant points at (e.g. 0x17 → 0x16 SG5) | static init | `Player_EquipWeapon`@`0x1a8a58`:30,33; `Player_WeaponFiring`@`0x1a5460`:116; `Player_AmmoInGun`@`0x1a9190`:14; `Player_WeaponChange`@`0x1a6f80`:70 | `[M]` |
| +0x04 | u8 | `selectable` | 1 = appears in the switch cycle | static init | `Player_WeaponChange`@`0x1a6f80`:179: `weapon_data[w*0x10c+4] == 1` | `[M]` |
| +0x05 | u8 | `shares_base_clip` | non-zero = shares the base weapon's clip slot when ammo types agree | `Player_Init`@`0x1910f8`:106 | `Player_EquipWeapon`@`0x1a8a58`:44; `Player_AmmoInGun`@`0x1a9190`:13 | `[M]` |
| +0x08 | f32 | `explosion_radius` | grenade explosion radius | static init | `Bullet_*` (WORKLOG 0080) | `[M]` |
| +0x0C | f32 | `explosion_damage_mul` | damage multiplier for `Explode_Propagate` | static init | `Bullet_*` | `[M]` |
| +0x14 | **f32** | `autoaim_accuracy` | 0..100 weight; `Check_AutoAim` does `*(f32*)(w+0x14) * 0.01 * (1-d/range) * difficulty_mul` | static init | `Check_AutoAim`@`0x1a73f8`:148 | `[M]` |
| +0x1C | f32 | `range` | laser/grenade range | static init | `Player_WeaponFiring`@`0x1a5460`:543,582 | `[M]` |
| +0x20 | f32 | `bullet_speed` | projectile speed | static init | `Bullet_init`@`0x12c5a0` | `[M]` |
| +0x28 | u16 × 4 | `shots_per_pull[4]` | shots per trigger pull, per fire-mode (0=semi,1=auto,2=burst,3=alt) | static init | `Player_WeaponFiring`@`0x1a5460`:413,458,620 | `[M]` |
| +0x38 | u32 | `name_label` | `Txt_BindLabel` hashcode for the HUD/pickup message; 0xFFFFFFFF = no name | static init | `Pickup_Handler`@`0x18e650`:246; `HUD_UpdateAmmoPane`@`0x19a7f0`:229 | `[M]` |
| +0x40 | u32 | `fire_rate` | refire interval in ticks (cast to float: `(float)*(u32*)(w+0x40)`) | static init | `Player_WeaponFiring`@`0x1a5460`:661 | `[M]` |
| +0x44 | u16 | `fire_anim_len` | fire-script length (frames); bullet spawns when anim frame ≥ this | static init | `Player_WeaponFiring`@`0x1a5460`:191,201 | `[M]` |
| +0x48 | u32 | `fp_flash_mesh` | first-person muzzle-flash mesh hashcode | static init | `Player_MuzzleFlash`@`0x1a4298`:66 | `[M]` |
| +0x4C | u32 | `muzzle_mesh` | third-person muzzle-flash mesh (`0x02000261` `Pistol_Muz` for P2K; 0 = none) | static init | `AnimDatumGetWeaponInfo`@`0x1c7900`:17 | `[M]` |
| +0x50 | u8 | `datum_id` | anim datum the third-person model hangs on (0xFF = none) | static init | `AnimDatumGetWeaponInfo`@`0x1c7900`:22 | `[M]` |
| +0x54..+0x56 | u8 × 3 | `muzzle_rgb` | muzzle-flash light colour (b, g, r) | static init | `Player_MuzzleFlash`@`0x1a4298`:89 | `[M]` |
| +0x58 | u32 | `fp_flash_radius` | `Light_Create` radius for the first-person muzzle flash | static init | `Player_MuzzleFlash`@`0x1a4298`:88 | `[M]` |
| +0x60 | u16 | `fire_sound` | drone fire-sound id | static init | `AnimDatumGetWeaponInfo`@`0x1c7900`:20 | `[M]` |
| **+0x68** | u32 | `flags` | weapon flags (see bit table below) | static init | `Player_WeaponFiring` (10×), `Player_SetWeaponAnimObj`, `Player_EquipAmmo` | `[M]` |
| **+0x6C** | u32 | `bullet_flags` | bullet-type flags (see bit table below) | static init | `Player_WeaponFiring`@`0x1a5460`:666,671; `Player_MuzzleFlash`@`0x1a4298`:93,105 | `[M]` |
| +0x70 | u32 | `bullet_flags_2` | second bullet-flags word (gravity/explode/bounce) | static init | `Bullet_init`, `Bullet_CollisionHandler` | `[M]` |
| +0x7C | u32 | `fp_flashlight_model` | first-person flashlight model | static init | `Player_WeaponFiring`@`0x1a5460`:106 | `[M]` |
| **+0x80** | u32 | `tp_model` | third-person (and pickup) model hashcode; 0 = none | static init | `AnimDatumGetWeaponInfo`@`0x1c7900`:14; `Pickup_CreateSimple`@`0x18f348`; `Pickup_CreateFromSet`@`0x18f1b0` | `[M]` |
| **+0x88** | f32 | `zoom` | zoom factor (1 = no zoom; sniper 10–16×). Loaded into `BLData+0x8D8` on draw | static init | `Player_SetWeaponAnimObj`@`0x1a2e38`:328 | `[M]` |
| **+0x90** | u8 | `ammo_type` | index into `ammo_data` and `BLData+0x170`; 0 = melee/unlimited | static init | `Player_EquipWeapon`@`0x1a8a58`:45,59,64; `Player_WeaponFiring`@`0x1a5460`:354; `Player_WeaponHasAmmo`@`0x1a16c0`:20 | `[M]` |
| +0x91 | u8 | `ammo_per_shot` | rounds consumed per shot | static init | `Player_WeaponFiring`@`0x1a5460`:445,585,655 | `[M]` |
| +0x92 | u16 | `clip_size` | max rounds in a clip | static init | `Player_EquipWeapon`@`0x1a8a58`:48,55; `Player_WeaponFiring`@`0x1a5460`:198,356 | `[M]` |
| +0x94 | u8 | `rumble` | rumble intensity for the fire anim | static init | `Player_WeaponFiring`@`0x1a5460`:658 | `[M]` |
| +0xA0 | u32 | `idle_script` | first-person idle loop hashcode | static init | `Player_SetWeaponAnimObj` cases 0,4,7,8,10 | `[M]` |
| **+0xA4** | u32 | `reload_script` | shell-loop reload hashcode (case 5→6; case 6 re-queues) | static init | `Player_SetWeaponAnimObj` case 5:339, case 6:346; `Player_WeaponFiring`@`0x1a5460`:418,471 | `[M]` |
| **+0xA8** | u32 | `reload_start_script` | reload-start hashcode (state 5; 0 = jump to 6) | static init | `Player_WeaponFiring`@`0x1a5460`:417,463 | `[M]` |
| **+0xAC** | u32 | `reload_end_script` | reload-end hashcode (state 6→7) | static init | `Player_SetWeaponAnimObj` case 6:357; `Player_WeaponFiring`@`0x1a5460`:492 | `[M]` |
| +0xB0 | u32 | `fire_script` | fire hashcode (case 9; bullet spawns at frame `+0x44`) | static init | `Player_SetWeaponAnimObj` case 9 | `[M]` |
| +0xB4 | u32 | `fire_last_script` | fire-on-last-round hashcode (case 9 → 0xB/0xC) | static init | `Player_WeaponFiring`@`0x1a5460`:481,677; `Player_SetWeaponAnimObj` case 0xB | `[M]` |
| **+0xC0** | u32 | `draw_script` | draw hashcode (case 3, 1.25×; case 10 reverse-draw) | static init | `Player_SetWeaponAnimObj` case 3:274,288; case 10:487 | `[M]` |
| **+0xC4** | u32 | `holster_script` | holster hashcode (case 1/0xD, 1.25×) | static init | `Player_SetWeaponAnimObj` case 1:206; `Player_WeaponSelect`@`0x1a2cc0`:33 | `[M]` |
| **+0xC8** | u32 | `variant_swap_script` | same-base weapon-swap hashcode (state 8, 1.3×) | static init | `Player_WeaponSelect`@`0x1a2cc0`:24 | `[M]` |
| **+0xCC** | u32 | `idle_alert_flag` | non-zero = weapon has idle-alert behaviour (only non-zero tested) | static init | `Player_SetWeaponAnimObj` case 0:79 | `[M]` |
| **+0xD0** | u32 | `idle_alert_raise_script` | raise hashcode when a target appears | static init | `Player_SetWeaponAnimObj` case 0:80,88 | `[M]` |
| **+0xD4** | u32 | `idle_alert_lower_script` | lower hashcode (case 0 sub-state 3, 1.25×) | static init | `Player_SetWeaponAnimObj` case 0:170,177 | `[M]` |
| **+0xD8** | u32 | `holster_variant_flag` | non-zero = holster at 1.0× (only non-zero tested) | static init | `Player_SetWeaponAnimObj` case 1:212 | `[M]` |
| **+0xDC** | u32 | `fp_model` | first-person model hashcode; `Player_EquipWeapon` refuses if 0 | `Player_Init`@`0x1910f8`:105 (PunchSkin override, record 1) | `Player_EquipWeapon`@`0x1a8a58`:20; `Player_SetWeaponAnimObj` case 8:390 | `[M]` |
| +0xE0 | f32 | `view_off_x` | SP idle gun x in view space | static init | `Player_SetWeaponAnimObj`@`0x1a2e38`:569 | `[M]` |
| +0xE4 | f32 | `view_off_y` | SP idle gun y | static init | `Player_SetWeaponAnimObj`@`0x1a2e38`:570 | `[M]` |
| +0xE8 | f32 | `view_off_z` | SP idle gun z | static init | `Player_SetWeaponAnimObj`@`0x1a2e38`:571 | `[M]` |
| +0xEC..+0xF4 | f32 × 3 | `view_aim_xyz` | SP aim gun position (interpolated from +0xE0..+0xE8) | static init | `Player_SetWeaponAnimObj`@`0x1a2e38`:579 | `[M]` |
| +0xF8..+0x100 | f32 × 3 | `view_off_mp_xyz` | MP gun position (SP base +0xC; MP skips the aim blend) | static init | `Player_SetWeaponAnimObj`@`0x1a2e38`:569 | `[M]` |

### `+0x68` flag bits

| bit | meaning | reader | tag |
|---:|---|---|:---:|
| 0x2 | can fire while surfaced (underwater) | `Player_WeaponFiring`@`0x1a5460`:504 | `[M]` |
| 0x10 | fill empty clip first (pickup) | `Player_EquipAmmo`@`0x1a8d18`:39 | `[M]` |
| 0x40 | scoped (enters aim mode on select) | `Player_WeaponFiring`@`0x1a5460`:404; `Player_Zoom` | `[M]` |
| 0x100 | one-at-a-time reload (shell loop) | `Player_SetWeaponAnimObj` case 6:354 | `[M]` |
| 0x200 | rapid reload (skip the 1.35× speed) | `Player_WeaponFiring`@`0x1a5460`:416,474 | `[M]` |
| 0x800 | has first-person flashlight | `Player_WeaponFiring`@`0x1a5460`:99,106 | `[M]` |
| 0x1000 | reload-anim starts from frame 6 | `Player_WeaponFiring`@`0x1a5460`:476 | `[M]` |
| 0x2000 | gas/smoke puff on muzzle flash | `Player_MuzzleFlash`@`0x1a4298`:92 | `[M]` |
| 0x8000 | aim-mode weapon | `Player_WeaponFiring`@`0x1a5460`:633 | `[M]` |
| 0x10000 | fire-on-trigger hold | `Player_WeaponFiring`@`0x1a5460`:429 | `[M]` |
| 0x40000 | alternate fire scripts (+0xB0/+0xB4 by the `BLData+0x958` toggle) and alternate muzzle datum (bone 4) | `Player_SetFiringAnim`@`0x1a3c70`:40-49; `Player_MuzzleFlash`@`0x1a4298`:70 | `[M]` |
| 0x80000 | random fire-script pick: one `Rand(100)` per shot, > 50 plays +0xB4 (the fists' left/right punch) | `Player_SetFiringAnim`@`0x1a3c70`:34-36 | `[M]` |

### `+0x6C` flag bits

| bit | meaning | reader | tag |
|---:|---|---|:---:|
| 0x8 | gravity-affected bullet (grenades/arcs) | `Bullet_update` (WORKLOG 0080) | `[M]` |
| 0x80 | taser beam | `Player_MuzzleFlash`@`0x1a4298`:134 | `[M]` |
| 0x100 | laser beam | `Player_MuzzleFlash`@`0x1a4298`:131 | `[M]` |
| 0x400 | fire-on-last-round variant | `Player_WeaponFiring`@`0x1a5460`:673 | `[M]` |
| 0x20000 | scan ground for matching ammo pickup | `Player_WeaponHasAmmo`@`0x1a16c0`:29 | `[M]` |
| 0x40000 | bullet fly-by (whiz) sound | `Bullet_DoTrails` (WORKLOG 0076) | `[M]` |

---

## Player_Init per-level overrides (PunchSkin)

`Player_Init`@`0x1910f8` (lines 83–113) writes two `weapon_data` bytes per level — the only runtime writes to the
table apart from the static initialiser:

```c
switch (GameState._12_4_) {            // the level id
  case 0x07000001..0x07000004: iVar8 = 1; break;   // PunchSkin[1]
  case 0x07000007..0x07000008: iVar8 = 2; break;   // PunchSkin[2]
  case 0x0700001b:             iVar8 = 3; break;   // PunchSkin[3]
  default:                     iVar8 = 0; break;   // PunchSkin[0]
}
weapon_data._488_4_ = *(u32*)(PunchSkin_204 + iVar8*4);   // record 1 +0xDC (fists skin)
weapon_data[0x3999] = 1;                                  // record 0x37 +0x05 (shares_base_clip)
```

- `weapon_data._488_4_` = Ghidra decimal: 488 = 0x1E8 = `1*0x10C + 0xDC` ⇒ **record 1 +0xDC** (the fists first-person
  skin). The static initialiser sets it to `0x0500003d`; `Player_Init` overrides it per level `[M]`.
- `weapon_data[0x3999]` = byte 0x3999 = `0x37*0x10C + 0x05` ⇒ **record 0x37 +0x05** (the `shares_base_clip` byte
  for weapon 0x37, the mine). Set to 1 on every level `[M]`.

`PunchSkin.204` table @ `0x002b9430` (4 × u32), read from the snapshot `[L]`:

| idx | level | value | meaning |
|---:|---|---|---|
| 0 | default (incl. 0x07000005, 0x07000006, 0x07000009+) | `0x050000AB` | fists skin |
| 1 | 0x07000001..0x07000004 | `0x05000099` | fists skin |
| 2 | 0x07000007..0x07000008 | `0x0500003D` | fists skin |
| 3 | 0x0700001B | `0x050000A1` | fists skin |

For MP the override is `weapon_data._488_4_ = 0x050000B0; weapon_data[0x3999] = 2;` instead. The Rust port mirrors
this in `WeaponData::apply_player_init_override` (`crates/nf-formats/src/weapon.rs:105`) `[M]`.

---

## The `+0x48CE` grenade-count check

`Player_WeaponHasAmmo`@`0x1a16c0`:15 reads `weapon_data._18638_2_` (= byte 0x48CE in the array, read as i16) for
weapon 0x45. Decomposition: `0x48CE = 0x10C * 0x45 + 0x92` ⇒ **record 0x45 +0x92** (clip_size, u16) `[M]`.
Value @ `0x002c3a1e` = `0x0064` = **100** `[L]`. `Player_WeaponHasAmmo` does
`(i16)weapon_data[0x45].clip_size <= BLData+0x4F4` — Bond can throw grenades while his carried count (at
`BLData+0x4F4`, which is `clip[0x45]`) is ≥ 100 `[M 0x1a16c0:14-15]`.

---

## Weapon id table

115 records (0x00..0x72); 71 populated weapons (0x01..0x47), 21 populated gadgets (0x4A..0x5E); the rest are zero.
`selectable` = `weapon_data[w]+0x04 == 1`. Names are looked up via `Txt_BindLabel(+0x38)`; the hashcode→string
mapping is not in the codebase, so only hashcodes are listed, with names pinned by WORKLOG / docs/port-map.
All values from `ee_start_05.bin` @ 0x2bf150 `[L]`.

### Weapons (0x01..0x47)

| id | base | sel | name hash | name (if known) | ammo_type | clip | flags68 (notable) | zoom | tag |
|---:|---:|:---:|---:|---|---:|---:|---|---:|:---:|
| 0x01 | 0x01 | ✓ | 0xFFFFFFFF | Fists (melee, no HUD name) | 0 | 1 | 0x880A2 | 1.0 | `[L+S]` |
| 0x02 | 0x02 | ✓ | 0x05000038 | **P2K** (handgun) | 1 | 7 | 0x1004 | 1.0 | `[L+M]` |
| 0x03 | 0x02 |   | 0x05000038 | P2K variant (silenced?) | 1 | 7 | 0x1884 | 1.0 | `[L]` |
| 0x04 | 0x04 | ✓ | 0x05000038 | P2K variant 2 | 1 | 7 | 0x1004 | 1.0 | `[L]` |
| 0x05 | 0x04 |   | 0x05000038 | P2K variant 2 (silenced?) | 1 | 7 | 0x1884 | 1.0 | `[L]` |
| 0x06 | 0x06 | ✓ | 0x05000036 | (default-equip on most levels) | 2 | 16 | 0x100C | 1.0 | `[L]` |
| 0x07 | 0x06 |   | 0x05000036 | variant of 0x06 | 2 | 16 | 0x188C | 1.0 | `[L]` |
| 0x08 | 0x08 | ✓ | 0x05000036 | variant of 0x06 family | 2 | 16 | 0x100C | 1.0 | `[L]` |
| 0x09 | 0x08 |   | 0x05000036 | variant of 0x06 family | 2 | 16 | 0x188C | 1.0 | `[L]` |
| 0x0A | 0x0A | ✓ | 0x05000033 | — | 2 | 18 | 0x0 | 1.0 | `[L]` |
| 0x0B | 0x0A |   | 0x05000033 | variant of 0x0A | 2 | 18 | 0x0 | 1.0 | `[L]` |
| 0x0C | 0x0C | ✓ | 0x05000073 | — | 2 | 18 | 0x0 | 1.0 | `[L]` |
| 0x0D | 0x0D | ✓ | 0x05000033 | — (0x40000 alt-muzzle) | 2 | 36 | 0x40000 | 1.0 | `[L]` |
| 0x0E | 0x0E | ✓ | 0x0500002D | — | 3 | 9 | 0x1000 | 1.0 | `[L]` |
| 0x0F | 0x0E |   | 0x0500002D | variant of 0x0E | 3 | 9 | 0x1008 | 1.0 | `[L]` |
| 0x10 | 0x10 | ✓ | 0x0500002E | — | 11 | 7 | 0x1000 | 1.0 | `[L]` |
| 0x11 | 0x11 | ✓ | 0x0500002C | — (scoped, aim-mode) | 15 | 3 | 0x180C0 | 4.0 | `[L]` |
| 0x12 | 0x12 | ✓ | 0x0500003A | — | 2 | 32 | 0x0 | 1.0 | `[L]` |
| 0x13 | 0x12 |   | 0x0500003A | variant of 0x12 | 2 | 32 | 0x0 | 1.0 | `[L]` |
| 0x14 | 0x14 | ✓ | 0x05000034 | — | 2 | 21 | 0x880 | 1.0 | `[L]` |
| 0x15 | 0x14 |   | 0x05000034 | variant of 0x14 | 2 | 21 | 0x0 | 1.0 | `[L]` |
| 0x16 | 0x16 | ✓ | 0x0500003B | **SG5 Commando** | 5 | 30 | 0x80 | 1.0 | `[L+M]` |
| 0x17 | 0x16 |   | 0x0500003B | SG5 variant | 5 | 30 | 0x88 | 1.0 | `[L]` |
| 0x18 | 0x19 |   | 0x0500003B | variant of 0x19 family | 5 | 30 | 0x8 | 3.0 | `[L]` |
| 0x19 | 0x19 | ✓ | 0x0500003B | — (zoom 3.0) | 5 | 30 | 0x0 | 3.0 | `[L]` |
| 0x1A | 0x1A | ✓ | 0x05000035 | — (scoped, zoom 6.0) | 5 | 30 | 0x50 | 6.0 | `[L]` |
| 0x1B | 0x1A |   | 0x05000035 | variant of 0x1A | 12 | 6 | 0x18050 | 6.0 | `[L]` |
| 0x1C | 0x1C | ✓ | 0x0500003D | **SPAS-12** (shotgun) | 9 | 8 | 0x8100 | 1.0 | `[L+M]` |
| 0x1D | 0x1C |   | 0x0500003D | SPAS-12 variant | 9 | 8 | 0x100 | 1.0 | `[L]` |
| 0x1E | 0x1E | ✓ | 0x05000029 | — (scoped sniper, zoom 10.0) | 7 | 5 | 0x408050 | 10.0 | `[L]` |
| 0x1F | 0x1E |   | 0x05000029 | variant of 0x1E | 8 | 5 | 0x408050 | 10.0 | `[L]` |
| 0x20 | 0x20 | ✓ | 0x05000029 | — (scoped, zoom 16.0) | 7 | 5 | 0x408050 | 16.0 | `[L]` |
| 0x21 | 0x20 |   | 0x05000029 | variant of 0x20 | 8 | 5 | 0x408050 | 16.0 | `[L]` |
| 0x22 | 0x22 | ✓ | 0x05000029 | — (scoped, zoom 16.0) | 7 | 10 | 0x408050 | 16.0 | `[L]` |
| 0x23 | 0x22 |   | 0x05000029 | variant of 0x22 | 8 | 10 | 0x408050 | 16.0 | `[L]` |
| 0x24 | 0x24 | ✓ | 0x0500002A | **Sniper / L96A1** (white, scoped) | 7 | 5 | 0x4080D0 | 10.0 | `[L+M]` |
| 0x25 | 0x24 |   | 0x0500002A | L96A1 variant | 8 | 5 | 0x4080D0 | 10.0 | `[L]` |
| 0x26 | 0x26 | ✓ | 0x0500002A | — (scoped, zoom 16.0) | 7 | 5 | 0x4080D0 | 16.0 | `[L]` |
| 0x27 | 0x26 |   | 0x0500002A | variant of 0x26 | 8 | 5 | 0x4080D0 | 16.0 | `[L]` |
| 0x28 | 0x28 | ✓ | 0x0500002A | — (scoped, zoom 16.0) | 7 | 10 | 0x4080D0 | 16.0 | `[L]` |
| 0x29 | 0x28 |   | 0x0500002A | variant of 0x28 | 8 | 10 | 0x4080D0 | 16.0 | `[L]` |
| 0x2A | 0x2A | ✓ | 0x05000032 | — (0x10000 fire-on-trigger) | 13 | 6 | 0x10000 | 2.0 | `[L]` |
| 0x2B | 0x2A |   | 0x05000032 | variant of 0x2A | 13 | 6 | 0x10000 | 2.0 | `[L]` |
| 0x2C | 0x2C | ✓ | 0x05000039 | — | 14 | 4 | 0x4000 | 1.0 | `[L]` |
| 0x2D | 0x2C |   | 0x05000039 | variant of 0x2C | 14 | 4 | 0x14000 | 1.0 | `[L]` |
| 0x2E | 0x2E | ✓ | 0x05000048 | — | 14 | 4 | 0x14000 | 1.0 | `[L]` |
| 0x2F | 0x2E |   | 0x05000048 | variant of 0x2E | 14 | 4 | 0x14000 | 1.0 | `[L]` |
| 0x30 | 0x30 | ✓ | 0x0500004A | — | 14 | 4 | 0x14002 | 1.0 | `[L]` |
| 0x31 | 0x30 |   | 0x0500004A | variant of 0x30 | 14 | 4 | 0x14002 | 1.0 | `[L]` |
| 0x32 | 0x33 |   | 0x05000053 | variant of 0x33 | 31 | 100 | 0x12040 | 3.5 | `[L]` |
| 0x33 | 0x33 | ✓ | 0x05000053 | — (scoped, zoom 3.5) | 31 | 100 | 0x12040 | 3.5 | `[L]` |
| 0x34 | 0x34 | ✓ | 0x05000031 | — | 25 | 1 | 0x14080 | 1.0 | `[L]` |
| 0x35 | 0x35 | ✓ | 0x0500003F | **Frag Grenade** | 23 | 1 | 0x14080 | 1.0 | `[L+M]` |
| 0x36 | 0x36 | ✓ | 0x0500003C | **Stun Grenade** | 24 | 1 | 0x14080 | 1.0 | `[L+M]` |
| 0x37 | 0x37 | ✓ | 0x0500003E | **Remote Mine** | 22 | 1 | 0x14090 | 1.0 | `[L+M]` |
| 0x38 | 0x37 |   | 0x0500003E | mine variant | 22 | 500 | 0x20010 | 1.0 | `[L]` |
| 0x39 | 0x37 |   | 0x0500003E | mine variant | 22 | 500 | 0x20010 | 1.0 | `[L]` |
| 0x3A | 0x3A | ✓ | 0x05000049 | — | 28 | 1 | 0x14080 | 1.0 | `[L]` |
| 0x3B | 0x3B | ✓ | 0x0500004C | — | 29 | 1 | 0x10080 | 1.0 | `[L]` |
| 0x3C..0x40 | 0x3B |   | 0x0500004C | 5 variants of 0x3B | 29 | 1 | 0x10080 | 1.0 | `[L]` |
| 0x41 | 0x41 | ✓ | 0x05000052 | — | 21 | 1 | 0x1A080 | 1.0 | `[L]` |
| 0x42 | 0x42 | ✓ | 0x05000050 | — (aim-mode 0x8000; BestWeapon[0]) | 19 | 1 | 0x8000 | 1.0 | `[L]` |
| 0x43 | 0x43 | ✓ | 0x05000041 | — | 18 | 5 | 0x12080 | 1.0 | `[L]` |
| 0x44 | 0x44 | ✓ | 0x05000041 | variant of 0x43 | 18 | 5 | 0x12080 | 1.0 | `[L]` |
| 0x45 | 0x45 | ✓ | 0x05000071 | **grenade-type** (clip 100 = the +0x48CE cap) | 32 | 100 | 0x180A0 | 1.0 | `[L+M]` |
| 0x46 | 0x46 | ✓ | 0x0500000A | — (unlimited: shots_per_pull[0]=999) | 0 | 16 | 0x10020 | 1.0 | `[L]` |
| 0x47 | 0x47 |   | 0xFFFFFFFF | **Hands** (no weapon) | 0 | 0 | 0x20 | 1.0 | `[L+M]` |

### Gadgets (0x4A..0x5E)

| id | base | sel | name hash | name (if known) | ammo_type | clip | flags68 | tag |
|---:|---:|:---:|---:|---|---:|---:|---|:---:|
| 0x4A | 0x4A | ✓ | 0x05000042 | **Q-Claw** (grapple) | 20 | 100 | 0x12480 | `[L+M]` |
| 0x4B | 0x4A |   | 0x05000042 | Q-Claw variant | 20 | 1 | 0x8082 | `[L]` |
| 0x4C | 0x4C | ✓ | 0x05000042 | Q-Claw variant 2 | 20 | 100 | 0x12480 | `[L]` |
| 0x4D | 0x4C |   | 0x05000042 | Q-Claw variant | 20 | 1 | 0x8082 | `[L]` |
| 0x4E | 0x4E | ✓ | 0x05000043 | **Q-Decryptor** | 26 | 500 | 0x12482 | `[L+M]` |
| 0x4F | 0x4F | ✓ | 0x05000043 | Q-Decryptor variant | 26 | 500 | 0x12482 | `[L]` |
| 0x50 | 0x50 | ✓ | 0x05000044 | **Q-Laser** (watch laser) | 0 | 0 | 0x124A0 | `[L+M]` |
| 0x51 | 0x51 | ✓ | 0x05000044 | Q-Laser variant | 0 | 0 | 0x124A0 | `[L]` |
| 0x52 | 0x52 | ✓ | 0x05000040 | — | 27 | 1 | 0x20A0 | `[L]` |
| 0x53 | 0x53 | ✓ | 0x05000040 | variant of 0x52 | 0 | 1 | 0x20A0 | `[L]` |
| 0x54 | 0x54 | ✓ | 0x05000045 | **Laser Watch** (scoped, zoom 8.0) | 0 | 1 | 0x20E0 | `[L+M]` |
| 0x55 | 0x54 |   | 0x05000045 | Laser Watch variant | 0 | 1 | 0x20E0 | `[L]` |
| 0x56 | 0x56 | ✓ | 0x0500004E | — | 0 | 0 | 0x1820A0 | `[L]` |
| 0x57 | 0x57 | ✓ | 0x0500004E | variant of 0x56 | 0 | 0 | 0x1820A0 | `[L]` |
| 0x58 | 0x58 | ✓ | 0x0500004F | — | 0 | 0 | 0xA0A0 | `[L]` |
| 0x59 | 0x59 | ✓ | 0x0500004B | — | 30 | 1 | 0x100B0 | `[L]` |
| 0x5A | 0x59 |   | 0x0500004B | variant of 0x59 | 30 | 500 | 0x20030 | `[L]` |
| 0x5B | 0x5B | ✓ | 0x0500004B | variant of 0x59 family | 30 | 1 | 0x100B0 | `[L]` |
| 0x5C | 0x5B |   | 0x0500004B | variant of 0x5B | 30 | 500 | 0x20030 | `[L]` |
| 0x5D | 0x5D |   | 0xFFFFFFFF | — (not selectable; zoom 10.0) | 0 | 1 | 0x20A0 | `[L]` |
| 0x5E | 0x5E |   | 0xFFFFFFFF | — (not selectable; zoom 10.0) | 0 | 1 | 0x20A0 | `[L]` |

**Counts**: 40 selectable weapons (incl. fists 0x01; excl. hands 0x47) and 14 selectable gadgets `[L]`. The
`BestWeapon` table (0x002bede0, 18 × u16) — used by `Player_GetBestWeapon` for auto-switch on pickup — is
`[0x42, 0x32, 0x1a, 0x18, 0x1c, 0x16, 0x15, 0x08, 0x10, 0x0e, 0x0d, 0x0c, 0x12, 0x06, 0x04, 0x02, 0x0a, 0x01]` `[L]`,
matching the Rust `BEST_WEAPON` constant in `crates/nf-game/src/gun.rs:412` `[M]`.

---

## Snapshot check `[L]`

Read from `work/ee_start_05.bin` at `weapon_data` = `0x002bf150`.

### Weapon 0x02 (P2K)

Record at `0x002bf150 + 2*0x10C = 0x002bf368`.

| offset | address | type | value | decoded | tag |
|---:|---:|---|---|---|---|
| +0x02 | `0x002bf36a` | u16 | `0x0002` | base = 2 (P2K is its own base) | `[L]` |
| +0x14 | `0x002bf37c` | f32 | `0x42c80000` | **100.0** (auto-aim accuracy %) | `[L]` |
| +0x40 | `0x002bf3a8` | u32 | `0x00000008` | fire rate = 8 ticks | `[L]` |
| +0x68 | `0x002bf3d0` | u32 | `0x00001004` | flags = 0x1000 \| 0x4 | `[L]` |
| +0x6C | `0x002bf3d4` | u32 | `0x00040050` | bullet_flags = 0x40000 (fly-by) \| 0x50 | `[L]` |
| +0x80 | `0x002bf3e8` | u32 | `0x020008b3` | third-person model hashcode | `[L]` |
| +0x88 | `0x002bf3f0` | f32 | `0x3f800000` | zoom = 1.0 (no zoom) | `[L]` |
| +0x90 | `0x002bf3f8` | u8 | `0x01` | ammo_type = 1 | `[L]` |
| +0x92 | `0x002bf3fa` | u16 | `0x0007` | clip_size = 7 | `[L]` |
| +0xA4 | `0x002bf40c` | u32 | `0x06000827` | reload_script | `[L]` |
| +0xA8 | `0x002bf410` | u32 | `0x00000000` | reload_start_script = none | `[L]` |
| +0xAC | `0x002bf414` | u32 | `0x00000000` | reload_end_script = none | `[L]` |
| +0xB0 | `0x002bf418` | u32 | `0x06000823` | fire_script | `[L]` |
| +0xC0 | `0x002bf428` | u32 | `0x0600082b` | draw_script | `[L]` |
| +0xC4 | `0x002bf42c` | u32 | `0x0600082a` | holster_script | `[L]` |
| +0xC8 | `0x002bf430` | u32 | `0x06000829` | variant_swap_script | `[L]` |
| +0xDC | `0x002bf444` | u32 | `0x05000088` | fp_model | `[L]` |
| +0xE0 | `0x002bf448` | f32 | `0x3ce7d644` | view_off_x = 0.0283 | `[L]` |
| +0xE4 | `0x002bf44c` | f32 | `0x3d9aef1e` | view_off_y = 0.0755 | `[L]` |
| +0xE8 | `0x002bf450` | f32 | `0xbf671ea0` | view_off_z = −0.9026 | `[L]` |

### Weapon 0x16 (SG5 Commando)

Record at `0x002bf150 + 0x16*0x10C = 0x002c0858`.

| offset | address | type | value | decoded | tag |
|---:|---:|---|---|---|---|
| +0x02 | `0x002c085a` | u16 | `0x0016` | base = 0x16 (SG5 is its own base) | `[L]` |
| +0x68 | `0x002c08c0` | u32 | `0x00000080` | flags = 0x80 (alt muzzle datum) | `[L]` |
| +0x6C | `0x002c08c4` | u32 | `0x00040050` | bullet_flags = 0x40000 (fly-by) \| 0x50 | `[L]` |
| +0x80 | `0x002c08d8` | u32 | `0x02000b44` | tp_model (SG5 3rd-person) | `[L]` |
| +0x88 | `0x002c08e0` | f32 | `0x3f800000` | zoom = 1.0 | `[L]` |
| +0x90 | `0x002c08e8` | u8 | `0x05` | ammo_type = 5 | `[L]` |
| +0xDC | `0x002c0934` | u32 | `0x050000c0` | fp_model | `[L]` |

### PunchSkin override (verified live)

- `weapon_data[1]+0xDC` @ `0x002bf338` = `0x050000AB` — matches `PunchSkin.204[0]` (default for level 0x07000005) `[L]`.
- `weapon_data[0x37]+0x05` @ `0x002c2ae9` = `0x01` — set by `Player_Init` on every level `[L]`.
