//! `weapon_data` (ACTION.ELF 0x2bf150, 115 × 0x10C bytes): zero on the disc, filled at boot by the static
//! initialiser `_GLOBAL_$I$weapon_data`. We run that initialiser over the user's ELF (`ee::Machine`).
//! Spec: docs/formats/weapon-data.md.

use crate::buf::{invalid, Buf, Result};
use crate::ee::Machine;
use crate::elf::Elf;

pub const RECORD: usize = 0x10c;

#[derive(Clone, Debug)]
pub struct WeaponData {
    pub records: Vec<[u8; RECORD]>,
    /// `ammo_data` (0x2C69B8, 12-byte records) `+0x02`: the most rounds of each ammo type Bond carries
    /// (docs/systems/pickups.md).
    pub ammo_cap: Vec<i16>,
}

impl WeaponData {
    pub fn from_action_elf(data: &[u8]) -> Result<WeaponData> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        let sym = |n: &str| syms.iter().find(|s| s.name == n).map(|s| (s.value, s.size))
            .ok_or_else(|| crate::buf::FormatError::Invalid(format!("ACTION.ELF has no symbol {n}")));
        let (table, size) = sym("weapon_data")?;
        let (init, _) = sym("_GLOBAL_$I$weapon_data")?;
        if size as usize % RECORD != 0 { return invalid(format!("weapon_data size {size} is not a multiple of {RECORD:#x}")); }
        let mut m = Machine::from_elf(data)?;
        m.call(init, 0, 0, 2_000_000)?;
        let bytes = m.read(table, size as usize)?;
        let (ammo, ammo_size) = sym("ammo_data")?;
        let ammo = m.read(ammo, ammo_size as usize)?;
        let ammo_cap = ammo.chunks_exact(12).map(|r| i16::from_le_bytes([r[2], r[3]])).collect();
        Ok(WeaponData { records: bytes.chunks(RECORD).map(|c| c.try_into().unwrap()).collect(), ammo_cap })
    }

    /// A character's muzzle flash (`DroneWeap_FireWeapon`): the `+0x4C` mesh (none when 0) and the light colour
    /// `Light_Create` gets, (+0x56, +0x55, +0x54) as (r, g, b).
    pub fn muzzle_flash(&self, weapon: u8) -> Option<(u32, [u8; 3])> {
        let r = self.records.get(weapon as usize)?;
        let mesh = u32::from_le_bytes(r[0x4c..0x50].try_into().unwrap());
        (mesh != 0).then_some((mesh, [r[0x56], r[0x55], r[0x54]]))
    }

    pub fn bytes(&self) -> Vec<u8> { self.records.iter().flatten().copied().collect() }

    /// Third-person model a character holds (`+0x80`, `AnimDatumGetWeaponInfo`), and the skin datum it hangs on
    /// (`+0x50`; 0xFF = none, which the game turns into datum index 0 when drawing).
    pub fn third_person(&self, weapon: u8) -> Option<(u32, u8)> {
        let r = self.records.get(weapon as usize)?;
        let b = Buf(r);
        let mesh = b.u32(0x80).ok()?;
        (mesh != 0).then_some((mesh, r[0x50]))
    }

    fn u16_at(&self, weapon: usize, o: usize) -> u16 {
        self.records.get(weapon).map_or(0, |r| u16::from_le_bytes([r[o], r[o + 1]]))
    }

    /// `+0x00`: the record's own id (= index).
    pub fn id(&self, weapon: usize) -> u16 { self.u16_at(weapon, 0) }

    /// `+0x02`: the base weapon a variant belongs to (drops and ownership use it).
    pub fn base(&self, weapon: usize) -> u16 { self.u16_at(weapon, 2) }

    /// `+0x05`: shares the base weapon's clip slot when the ammo types agree (`Player_EquipWeapon`).
    pub fn shares_base_clip(&self, weapon: usize) -> bool { self.records.get(weapon).is_some_and(|r| r[5] != 0) }

    /// `+0x80`: the third-person model (also the pickup's model); 0 = none.
    pub fn model(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0x80..0x84].try_into().unwrap()))
    }

    /// `+0x68`: flags (0x10: a pickup fills an empty clip first, `Player_EquipAmmo`).
    pub fn flags(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0x68..0x6c].try_into().unwrap()))
    }

    /// `+0x6C`: bullet-type flags. Bit `0x40000` = the weapon's bullets play a fly-by (whiz) sound in
    /// `Bullet_DoTrails` [M drone-damage.md "Bullets": SG5 `+0x6C` = 0x40050 → bit set]. The trigger logic and RNG
    /// call order live in the (not-yet-accessible) `Bullet_DoTrails` decompile; the accessor is ported so the fly-by
    /// can be wired once that decompile is available (GAPS G-032a).
    pub fn bullet_flags(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0x6c..0x70].try_into().unwrap()))
    }

    /// `+0x6C & 0x40000`: whether this weapon's bullets emit a fly-by sound [M].
    pub fn flyby(&self, weapon: usize) -> bool { self.bullet_flags(weapon) & 0x40_000 != 0 }

    /// `+0x90`: the ammo type (index into `ammo_data` and Bond's carried rounds).
    pub fn ammo_type(&self, weapon: usize) -> usize { self.records.get(weapon).map_or(0, |r| r[0x90] as usize) }

    /// `+0x92`: the clip size (also what a dropped gun's rounds scale).
    pub fn clip_size(&self, weapon: usize) -> i16 { self.u16_at(weapon, 0x92) as i16 }

    /// `+0xDC`: the first-person model; `Player_EquipWeapon` refuses a weapon whose model is not loaded.
    pub fn first_person(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xdc..0xe0].try_into().unwrap()))
    }

    /// `Player_Init` (0x1910f8 lines 83-113): overrides `weapon_data[1]+0xDC` (the fists skin) with a
    /// level-specific value from the `PunchSkin.204` table at vaddr 0x2b9430 (4 u32 hashcodes). The static
    /// initialiser sets `weapon_data[1]+0xDC = 0x0500003d`; `Player_Init` overrides it per level [M].
    /// Also sets `weapon_data[0x37]+0x05 = 1` (shares_base_clip for the variant).
    pub fn apply_player_init_override(&mut self, level_id: u32) {
        // PunchSkin.204 [L ee_start_05.bin @ 0x2b9430]: [0x050000ab, 0x05000099, 0x0500003d, 0x050000a1].
        const PUNCH_SKIN: [u32; 4] = [0x0500_00ab, 0x0500_0099, 0x0500_003d, 0x0500_00a1];
        let idx = match level_id {
            0x0700_0001..=0x0700_0004 => 1,
            0x0700_0007..=0x0700_0008 => 2,
            0x0700_001b => 3,
            _ => 0,
        };
        if self.records.len() > 1 {
            self.records[1][0xdc..0xe0].copy_from_slice(&PUNCH_SKIN[idx].to_le_bytes());
        }
        // `weapon_data[0x37]+0x05 = 1` (shares_base_clip for the variant) [M Player_Init].
        if self.records.len() > 0x37 {
            self.records[0x37][5] = 1;
        }
    }

    /// `+0x04`: the selectable flag (1 = the weapon appears in the switch cycle, `Player_WeaponChange` 0x1a6f80
    /// line 179 checks `weapon_data[w*0x10c + 4] == 1`). Gadgets and non-weapons have 0 [M].
    pub fn selectable(&self, weapon: u8) -> bool {
        self.records.get(weapon as usize).is_some_and(|r| r[4] == 1)
    }

    /// `+0x14`: the auto-aim accuracy percentage (0..100), used by `Check_AutoAim` as
    /// `weapon_data[w]+0x14 * 0.01 * (1 - dist/range) * difficulty_mul` [M 0x1a73f8].
    pub fn autoaim_accuracy(&self, weapon: usize) -> f32 {
        self.records.get(weapon).map_or(0.0, |r| u16::from_le_bytes([r[0x14], r[0x15]]) as f32)
    }

    /// `+0x88`: the zoom factor (1 = no zoom, `Player_SetWeaponAnimObj` state 4 sets `BL+0x8D4`).
    pub fn zoom(&self, weapon: usize) -> f32 {
        self.records.get(weapon).map_or(1.0, |r| f32::from_le_bytes(r[0x88..0x8c].try_into().unwrap()))
    }

    /// `+0xA0`: the idle script hashcode (the first-person gun's idle loop).
    pub fn idle_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xa0..0xa4].try_into().unwrap()))
    }

    /// `+0xA4`: the reload script hashcode.
    pub fn reload_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xa4..0xa8].try_into().unwrap()))
    }

    /// `+0xA8`: the reload-start script hashcode (plays before the reload).
    pub fn reload_start_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xa8..0xac].try_into().unwrap()))
    }

    /// `+0xAC`: the reload-end script hashcode (the pump-finish / reload-end animation played after the
    /// shell-by-shell loop completes, or when the loop is interrupted by firing) [M Player_SetWeaponAnimObj case 6/7].
    pub fn reload_end_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xac..0xb0].try_into().unwrap()))
    }

    /// `+0xB0`: the fire script hashcode.
    pub fn fire_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xb0..0xb4].try_into().unwrap()))
    }

    /// `+0xB4`: the fire-on-last-round script hashcode.
    pub fn fire_last_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xb4..0xb8].try_into().unwrap()))
    }

    /// `+0xC4`: the holster (put-away) script hashcode (`Player_SetWeaponAnimObj` case 1, played at 1.25x)
    /// [M disassembly 0x1a36c0: `lw v1,196(s1)`; `Player_WeaponSelect` 0x1a2cc0 line 33 checks `+0xC4 == 0`].
    pub fn holster_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xc4..0xc8].try_into().unwrap()))
    }

    /// `+0xC0`: the draw script hashcode (`Player_SetWeaponAnimObj` case 3, played at 1.25x)
    /// [M disassembly 0x1a3828: `lw a1,192(s1)`].
    pub fn draw_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xc0..0xc4].try_into().unwrap()))
    }

    /// `+0xC8`: the variant-swap script hashcode (same-base weapon swap, `Player_WeaponSelect` 0x1a2cc0 line 24,
    /// played at 1.3x, state 8) [M].
    pub fn variant_swap_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xc8..0xcc].try_into().unwrap()))
    }

    /// `+0xCC`: the idle-alert flag (non-zero = the weapon has idle-alert behavior; case 0 of
    /// `Player_SetWeaponAnimObj` checks it) [M]. Not a script hashcode.
    pub fn idle_alert_flag(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xcc..0xd0].try_into().unwrap()))
    }

    /// `+0xD0`: the idle-alert raise script (played when a target appears while idle, case 0 sub-state 0) [M].
    pub fn idle_alert_raise_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xd0..0xd4].try_into().unwrap()))
    }

    /// `+0xD4`: the idle-alert lower script (case 0 sub-state 3) [M].
    pub fn idle_alert_lower_script(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xd4..0xd8].try_into().unwrap()))
    }

    /// `+0xD8`: the holster variant flag (case 1: if non-zero, `AnimScriptAdd` instead of `AnimScriptAddSpeed`)
    /// [M disassembly 0x1a36d4: `lw a1,216(s1)`].
    pub fn holster_variant_flag(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0xd8..0xdc].try_into().unwrap()))
    }

    /// `+0xE0/+0xE4/+0xE8`: the first-person gun offset (x, y, z) in view space (`Player_PositionGun`).
    pub fn view_offset(&self, weapon: usize) -> [f32; 3] {
        self.records.get(weapon).map_or([0.0; 3], |r| [
            f32::from_le_bytes(r[0xe0..0xe4].try_into().unwrap()),
            f32::from_le_bytes(r[0xe4..0xe8].try_into().unwrap()),
            f32::from_le_bytes(r[0xe8..0xec].try_into().unwrap()),
        ])
    }

    /// `+0x48`: the first-person muzzle flash mesh (`Draw_MuzzleFlash`).
    pub fn fp_flash_mesh(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0x48..0x4c].try_into().unwrap()))
    }

    /// `+0x60`: the drone fire sound id (per `docs/systems/sound.md` §7).
    pub fn fire_sound(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0x60..0x64].try_into().unwrap()))
    }

    /// `+0x38`: the weapon name label hashcode (for the HUD / pickup message).
    pub fn name_label(&self, weapon: usize) -> u32 {
        self.records.get(weapon).map_or(0, |r| u32::from_le_bytes(r[0x38..0x3c].try_into().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon_with_bullet_flags(flags: u32) -> WeaponData {
        let mut r = [0u8; RECORD];
        r[0x6c..0x70].copy_from_slice(&flags.to_le_bytes());
        WeaponData { records: vec![r], ammo_cap: Vec::new() }
    }

    /// `+0x6C & 0x40000` = the fly-by bit. Sister: a weapon without the bit (e.g. 0x50) does not fly-by.
    #[test]
    fn flyby_bit_is_0x40000() {
        // SG5: `+0x6C` = 0x40050 → bit set [M drone-damage.md].
        let sg5 = weapon_with_bullet_flags(0x40050);
        assert!(sg5.flyby(0));
        assert_eq!(sg5.bullet_flags(0), 0x40050);
        // Sister: 0x50 (no 0x40000) → no fly-by.
        let no_flyby = weapon_with_bullet_flags(0x50);
        assert!(!no_flyby.flyby(0));
        // Sister: out-of-range weapon → false (safe default).
        assert!(!sg5.flyby(99));
    }
}
