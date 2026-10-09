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
        Ok(WeaponData { records: bytes.chunks(RECORD).map(|c| c.try_into().unwrap()).collect() })
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
}
