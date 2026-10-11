# Player/weapon port-check

Every `+0xNNN` offset in `crates/nf-game/src/{gun,pickup,player}.rs`, `crates/nf-view/src/scene.rs`, and
`crates/nf-formats/src/weapon.rs` whose **meaning or offset disagrees** with the atlas
([player.md](player.md), [weapon-data.md](weapon-data.md)). Offsets that agree are not listed.

Tags: `[M]` confirmed in the decompile, `[L]` snapshot-verified, `[S]` suggested.

**Update (fix-player, s/fix-player):** entries 1–5 below are fixed (each with a test that failed before
the fix); 6–7 remain open gaps. Entry 3's code claim was stale — the port's `PlayerHealth`
(`bullet.rs:125`) already has `armour: f32` and `pain()` absorbs it; the live gap was the pickup path,
which never wrote it and never refused at ≥ 50.

---

## Confirmed disagreements (real bugs / stand-ins)

### 1. `weapon_data +0x14` — **type mismatch: f32, not u16** `[M+L]`

**The atlas says**: `+0x14` is `f32 autoaim_accuracy` (0..100). `Check_AutoAim`@`0x1a73f8`:148 reads
`*(float *)(weapon_data + w*0x10c + 0x14) * 0.01 * (1-d/range) * difficulty_mul`. The snapshot stores `0x42c80000`
(= 100.0) for the P2K, `0x42700000` (= 60.0) for the SG5 `[L]`.

**The Rust port reads it as u16**: `crates/nf-formats/src/weapon.rs:132`:
```rust
u16::from_le_bytes([r[0x14], r[0x15]]) as f32
```
The low 2 bytes of the f32 are `0x0000` for every weapon (the float `0x42c80000` has low bytes `0x00 0x00`), so
the port's `autoaim_accuracy` always returns **0.0** for every weapon. This is a real port bug: the auto-aim cone
is always zero-width. The decompile is the source of truth `[M]`.

**Fix**: read `+0x14` as `f32::from_le_bytes(r[0x14..0x18])`.

**FIXED** (s/fix-player): `weapon.rs::autoaim_accuracy` now reads the f32. NF_ISO test
`crates/nf-formats/tests/weapon_live.rs::autoaim_accuracy_is_the_f32_at_plus_0x14` asserts 100.0 / 60.0
against the ELF off the disc [L] and pins the old u16 read as always 0; unit tests assert it. (The sim
path, `drone.rs::auto_aim`, already read +0x14 as f32 — only the accessor was wrong.)

| file:line | offset | atlas | port | severity |
|---|---|---|---|---|
| `weapon.rs:129,132` | `+0x14` | f32 (0..100) | u16 (always 0) | **bug** — auto-aim never triggers |

---

### 2. `BLData +0x4F4` — **alias, not a separate field** `[M]`

**The atlas says**: `+0x4F4` is **not a separate field**. It is `clip[0x45]` = `BLData + 0x1B8 + 0x45·0xC = 0x4F4`
(the grenade weapon's clip). `Player_WeaponHasAmmo`@`0x1a16c0`:15 reads it only when `param_2 == 0x45` — the
compiler folded the constant index `[M]`.

**The Rust port treats it as a standalone field**: `crates/nf-game/src/gun.rs:92,96` comments:
```
Weapon 0x45 (a grenade type) checks BLData+0x4F4 against weapon_data+0x48CE
```
The port's `has_ammo` has a STANDIN that always returns true, so the alias is never actually read. But the comment
implies `+0x4F4` is a dedicated grenade-count field, which is wrong — it's just `clip[0x45]`.

**Fix**: the comment should say "checks `clip[0x45]` (= `+0x1B8 + 0x45·0xC` = `+0x4F4`) against
`weapon_data[0x45].clip_size` (= `+0x48CE` = 100)". No code change needed (the STANDIN bypasses it), but the
misleading comment should be corrected.

**FIXED** (s/fix-player): comment corrected, and the STANDIN itself removed — `gun.rs::has_ammo(0x45)`
implements the literal `100 <= clip[0x45]` comparison [M 0x1a16c0:14-15] (see entry 5).

| file:line | offset | atlas | port | severity |
|---|---|---|---|---|
| `gun.rs:92,96` | `+0x4F4` | alias of `clip[0x45]` | implied separate field | **misleading comment** (STANDIN hides it) |

---

### 3. `BLData +0x8B0` (armor) — **not in the port's Health struct** `[M]`

**The atlas says**: `+0x8B0` is `armor` (f32, max 50.0), written by `Player_HandlePain`@`0x1902c8` and
`Pickup_Handler`@`0x18e650` (sets 50.0 on armour pickup) `[M]`.

**The Rust port**: `crates/nf-game/src/pickup.rs:262` references it:
```
Type 3: armour 50 (BLData+0x8B0, only when below 50)
```
But the port's `Health` struct (in `drone.rs`) has `health` and `flash` but **no `armor` field**. The pickup code
sets it, but the damage code (`Player_HandlePain`) can't decrement it because the field doesn't exist in the
port's struct. Armor pickups have no effect on damage taken.

**Fix**: add `armor: f32` to the `Health` struct and port `Player_HandlePain`'s armor-decrement path.

**FIXED** (s/fix-player): the struct part was already done elsewhere — `PlayerHealth` (`bullet.rs:125`)
has `armour: f32` and `pain()` absorbs damage into it. The missing half was the pickup: `Inventory::take`
now takes `&mut f32` armour and implements the game's case 3 — refused while ≥ 50.0 (the pickup stays),
else hard-set 50.0 [M 0x18e650:220-225] — instead of consuming and giving nothing. Unit test
`armour_fills_to_50_and_refuses_when_full`.

| file:line | offset | atlas | port | severity |
|---|---|---|---|---|
| `pickup.rs:262` | `+0x8B0` | f32 armor (in BLData) | referenced but no field in `Health` struct | **gap** — armor has no gameplay effect |

---

### 4. `BLData +0x1BC` — **STANDIN (not ported)** `[M]`

**The atlas says**: `+0x1BC` (per-weapon entry `+0x8`) is tail padding / unused `[M]`.

**The Rust port**: `crates/nf-game/src/gun.rs:112`:
```
The per-player +0x1BC variant-slot adjustment is a STANDIN (not ported — the Inventory has no +0x1BC field)
```
The port correctly identifies it as unported, but the comment calls it a "variant-slot adjustment" — the atlas
shows it's just tail padding. The STANDIN is harmless (the field is unused), but the comment mischaracterizes it.

**FIXED** (s/fix-player): comment corrected to "tail padding / unused" (no code change).

| file:line | offset | atlas | port | severity |
|---|---|---|---|---|
| `gun.rs:112` | `+0x1BC` | tail padding | "variant-slot adjustment" (STANDIN) | **misleading comment** (harmless) |

---

### 5. `weapon_data +0x48CE` — **STANDIN (always true)** `[M]`

**The atlas says**: `+0x48CE` = `weapon_data[0x45].clip_size` (u16 = 100) `[M]`. `Player_WeaponHasAmmo` checks
`(i16)clip_size <= BLData+0x4F4` (the grenade carried count) `[M]`.

**The Rust port**: `crates/nf-game/src/gun.rs:96`:
```
weapon_data._18638_2_ (offset 0x48CE) <= BLData+0x4F4. STANDIN: always true if owned.
```
The STANDIN bypasses the check — Bond can always throw grenades regardless of carried count. This is a known gap
(G-029 in `docs/GAPS.md`).

**FIXED** (s/fix-player): `gun.rs::has_ammo(0x45)` returns `100i16 <= clip[0x45]` — the game's literal
comparison (the comparison direction is as the decompiler prints it: `(short)clip_size <= *(short*)(BL+0x4F4)`)
[M 0x1a16c0:14-15]. Unit test `grenade_belt_weapon_0x45_follows_the_carried_count`.

| file:line | offset | atlas | port | severity |
|---|---|---|---|---|
| `gun.rs:92,96` | `+0x48CE` | `weapon_data[0x45].clip_size` | STANDIN (always true) | **gap** — grenade count check bypassed |

---

### 6. `M_CONTROL +0x60` (aim flag) — **not referenced by the port** `[M]`

**The atlas says**: the aim flag is at `M_CONTROL+0x60` (u16 bitmask; bit 0x1 = aiming), where
`M_CONTROL = *(obj_tag+0xDC)` `[M]`. Read by `Player_Aiming`@`0x1a1d28`:31, `Player_Zoom`@`0x1a1a70`:18,
`Player_HandleJump`@`0x1916b0`:67,103,182 `[M]`.

**The Rust port**: does not reference `M_CONTROL+0x60` anywhere. The port uses its own `Sim::aiming` bool
(set by the right-mouse input handler) instead of reading the game's aim flag. This works for the single-player
mouse path, but means the port doesn't model the full aim-flag bitmask (bit 0x8 = jump pending, bit 0x20 = touched
above).

| file:line | offset | atlas | port | severity |
|---|---|---|---|---|
| (none) | `M_CONTROL+0x60` | u16 aim flags (bit 0x1/0x8/0x20) | not referenced (port uses `Sim::aiming`) | **gap** — aim-flag bits 0x8/0x20 not modelled |

---

### 7. `PlayerSetting` — **only `+0x01` referenced** `[M]`

**The atlas says**: `PlayerSetting` is a 1376-byte (4 players × 0x158) struct at `0x002a38c8` with per-player
control settings `[M]`.

**The Rust port**: only `PlayerSetting+1` (the auto-aim option) is referenced, at `crates/nf-view/src/scene.rs:181`:
```
the auto-aim option (PlayerSetting+1 = 1 in the game [L]; off by default with a mouse, D-001)
```
The rest of the PlayerSetting struct (the per-player control settings `+0x02`..`+0x0A`) is not ported. This is a
known gap (the port's input system is its own design, not a port of the game's control settings).

| file:line | offset | atlas | port | severity |
|---|---|---|---|---|
| `scene.rs:181` | `PlayerSetting+0x01` | autoAim (u8) | referenced correctly | OK |
| (none) | `PlayerSetting+0x02`..`+0x0A` | per-player control settings | not ported | **gap** — input system is own design |

---

## Offsets that agree (no action needed)

The following offsets in the Rust port match the atlas exactly (correct offset + correct type + correct meaning).
Listed here so the audit is complete — these are NOT disagreements.

### BLData (all agree)

`+0x20`, `+0x28`, `+0x50`, `+0x110`, `+0x118`, `+0x11C`, `+0x120`, `+0x124`, `+0x170`, `+0x1B4`, `+0x1B8`, `+0x1BA`,
`+0x8A8`, `+0x8D0`, `+0x8D4`, `+0x8DC`, `+0x8E8`, `+0x908`, `+0x90C`, `+0x924`, `+0x928`, `+0x92C`, `+0x930`,
`+0x932`, `+0x938`, `+0x940`, `+0x94A`, `+0x94C`, `+0x952`, `+0x959`, `+0x960`.

### weapon_data (all agree)

`+0x00`, `+0x02`, `+0x04`, `+0x05`, `+0x40`, `+0x48`, `+0x4C`, `+0x50`, `+0x54`..`+0x56`, `+0x58`, `+0x60`,
`+0x68`, `+0x6C`, `+0x80`, `+0x88`, `+0x90`, `+0x91`, `+0x92`, `+0xA0`, `+0xA4`, `+0xA8`, `+0xAC`, `+0xB0`,
`+0xB4`, `+0xC0`, `+0xC4`, `+0xC8`, `+0xCC`, `+0xD0`, `+0xD4`, `+0xD8`, `+0xDC`, `+0xE0`, `+0xE4`, `+0xE8`.

### obj_tag (all agree)

`+0x30` (position), `+0x54` (yaw), `+0x90` (matrix), `+0xD4` (→ BLData+0x740), `+0xDC` (→ M_CONTROL), `+0xE0`
(→ BLData), `+0xF0` (flags), `+0xF6` (subState), `+0xFE` (deleted).

### ammo_data (all agree)

`+0x02` (cap, s16), record stride 12 bytes.

---

## Summary

| # | offset | struct | issue | severity |
|---|---|---|---|---|
| 1 | `+0x14` | weapon_data | type is f32, port reads u16 (always 0) | **bug** — auto-aim broken |
| 2 | `+0x4F4` | BLData | alias of `clip[0x45]`, comment implies separate field | misleading comment (STANDIN hides) |
| 3 | `+0x8B0` | BLData | armor field not in port's `Health` struct | **gap** — armor has no effect |
| 4 | `+0x1BC` | BLData | tail padding, comment says "variant-slot adjustment" | misleading comment (harmless) |
| 5 | `+0x48CE` | weapon_data | STANDIN always true (grenade count check bypassed) | **gap** — G-029 |
| 6 | `+0x60` | M_CONTROL | aim flag not referenced (port uses own bool) | **gap** — bits 0x8/0x20 not modelled |
| 7 | `+0x02`..`+0x0A` | PlayerSetting | per-player control settings not ported | **gap** — input system is own design |

**Real bugs**: 1 (the `+0x14` type mismatch breaks auto-aim).
**Gaps (stand-ins)**: 3, 5, 6, 7.
**Misleading comments**: 2, 4 (no code change needed, but the comments should be corrected to match the atlas).
