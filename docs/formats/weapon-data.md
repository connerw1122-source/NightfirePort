# `weapon_data` (ACTION.ELF 0x2bf150)

115 records × 0x10C bytes (symbol size 30,820; `AnimDatumGetWeaponInfo` checks weapon < 0x73). The bytes in the ELF
are all zero: the table is filled at boot by the C++ static initialiser `_GLOBAL_$I$weapon_data` (0x1b9900), which
calls `gcc2_compiled.` (0x1b3488, 25,720 bytes: constant stores, unaligned 64-bit copies, 20 `memset` calls, a few
single-precision multiplies). *Measured*.

## How we get it

`nf_formats::ee::Machine` runs that initialiser over the user's ACTION.ELF (PT_LOAD image, small stack, `memset`
done natively, FPU single ops rounded toward zero). It implements only the instructions the initialiser uses and
errors on anything else. Code: `crates/nf-formats/src/ee.rs`, `weapon.rs`; `nf-extract weapons <iso> [live.bin]`.

**Cross-check** (*measured*, WORKLOG 0017): PCSX2 memory at 0x2bf150 read over PINE in level 0x07000001 equals our
table in 30,819 of 30,820 bytes. The odd byte is record 1 +0xDC: a skin hashcode the game writes there at run time
(0x05000099 in 07000001, 0x050000AB in 07000005). Golden: SHA-1 `2fd48d6d…` (opt-in test).

## Fields used so far

| Offset | Type | Meaning | Evidence |
|---|---|---|---|
| +0x00 | u16 | weapon id (= index) | every record |
| +0x02 | u16 | base weapon (variants point at the plain one, e.g. 0x17 → 0x16) | records 0x15–0x2B |
| +0x4C | u32 | hashcode passed out by `AnimDatumGetWeaponInfo` (often 0x02000261) | decomp |
| +0x50 | u8 | datum id the third-person model hangs on (0xFF = none → id 0 when drawn) | `AnimObjectDraw` |
| +0x6C | u32 | bullet-type flags; bit `0x40000` = the weapon's bullets play a fly-by (whiz) sound in `Bullet_DoTrails` | drone-damage.md "Bullets" (SG5 `0x40050`); accessor `bullet_flags`/`flyby` |
| +0x7C | u32 | first-person (Bond) model hashcode | `Player_WeaponFiring` |
| +0x80 | u32 | third-person model hashcode (`…_3rd_LOD0` map objects in the level archives) | `AnimObjectDraw` |

80 records have a third-person model. 07000005's guards: 0x16 → `02000b44` `Sg5commando_nosight_3rd_LOD0` on datum 8;
0x24 → `020008ac` `L96a1_white_3rd_LOD0` on datum 16.

## Muzzle flash (`DroneWeap_FireWeapon` 0x178f68)

Per shot: `AnimDatumGetWeaponInfo` → datum id (+0x50) and flash mesh (+0x4C, e.g. `02000261` `Pistol_Muz`; 0 = none,
as for the sniper rifle 0x24); `AnimGetBoneWorldTrans(obj, id, 1, …)` looks up datum **id + 1** (the function adds
its third argument to the id) — the muzzle datum (8 → 9 at x −0.77, 16 → 17 at x −1.18 on bone 35). Direction = the
datum's −x axis. `Light_Create(radius 7.0, 1.0, position, +0x56, +0x55, +0x54)` (0x16: 255, 138, 0 — orange); an effect
object (type 0x40, `+0xFF` = 0x10) at the position with rotation = spherical angles of the direction
(`Vec_Cartesian_2_Spherical`: atan2(y, |xz|), atan2(x, z), length) + `Rand_FRand(2π)` / `Rand_FRandHalf(0.2) + π/2` /
`Rand_FRandHalf(0.2)`, alpha 0xFF, life 2 frames. The player's own flash is `Draw_MuzzleFlash` (+0x48 mesh, scale
0.9–1.1, alpha between +0x51 and +0x52).

nf-view draws the mesh additively (`F` makes armed drones fire: one shot every 8 ticks, staggered — a viewer rate, not
the weapon's; no dynamic light yet; the GS blend is *suggested* until the mesh's A+D state is decoded, U-021).

## Drawing (`AnimObjectDraw` 0x1c4490)

For objects with anim flag bit 1 and datums: `AnimDatumSetEntity(obj, datum id, weapon_data[w].+0x80, 1)` every
draw, w = `sAnimObject` weapon byte (`anim+0x62`, set by `NDrone2_DefaultInit` from parameter 9). Each datum with an
entity is drawn with `QuatTransToMat(datum q, datum t)` concatenated with the bone's world matrix (`Concat(A, B)`:
B = B·A, row vectors) through `psiDrawObjectMatrix`.
