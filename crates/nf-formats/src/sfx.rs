//! `SFXOutputData` (ACTION.ELF 0x2c94a0, 0x60D × 0x18): per sound effect the AI hearing data `Sound_Play3D` copies into
//! its list — `+4` inner radius, `+8` outer radius, `+0xC` weight (0 = silent to drones), `+0x14` looped (ignored for
//! hearing). Spec: docs/systems/drone-alerts.md "Hearing".

use crate::buf::{invalid, Buf, Result};
use crate::elf::Elf;

pub const RECORD: usize = 0x18;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SfxHearing { pub id: u32, pub inner: f32, pub outer: f32, pub weight: f32, pub looped: bool, pub cull: bool }

#[derive(Clone, Debug)]
pub struct SfxTable { pub entries: Vec<SfxHearing> }

impl SfxTable {
    pub fn from_action_elf(data: &[u8]) -> Result<SfxTable> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        let (addr, size) = syms.iter().find(|s| s.name == "SFXOutputData").map(|s| (s.value, s.size as usize))
            .ok_or_else(|| crate::buf::FormatError::Invalid("ACTION.ELF has no symbol SFXOutputData".into()))?;
        if size % RECORD != 0 { return invalid(format!("SFXOutputData size {size:#x}")); }
        let segs = elf.load_segments(data)?;
        let bytes = segs.iter().find_map(|(va, b, _)| (addr >= *va && (addr - va) as usize + size <= b.len()).then(|| &b[(addr - va) as usize..][..size]))
            .ok_or_else(|| crate::buf::FormatError::Invalid("SFXOutputData outside the loaded data".into()))?;
        let b = Buf(bytes);
        let entries = (0..size / RECORD).map(|k| {
            let o = k * RECORD;
            Ok(SfxHearing { id: k as u32, inner: b.f32(o + 4)?, outer: b.f32(o + 8)?, weight: b.f32(o + 0xc)?, looped: b.u8(o + 0x14)? != 0, cull: b.u8(o + 0x15)? != 0 })
        }).collect::<Result<_>>()?;
        Ok(SfxTable { entries })
    }

    /// The hearing data of a sound as `Sound_Play3D` enters it (low 20 bits of the id): ids ≥ 0x60D get inner 10,
    /// outer 20 and weight 0.
    pub fn get(&self, id: u32) -> SfxHearing {
        self.entries.get((id & 0xfffff) as usize).copied().map(|h| SfxHearing { id, ..h }).unwrap_or(SfxHearing { id, inner: 10.0, outer: 20.0, weight: 0.0, looped: false, cull: false })
    }
}
