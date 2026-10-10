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
