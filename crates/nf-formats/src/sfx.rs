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

/// `Drone_SfxInfo` (ACTION.ELF 0x2812d0, 346 × 32 bytes): a drone voice line — sound id, facial anim script,
/// repeat delay in frames (+0x08 last play frame is runtime state, global per entry).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceLine { pub sfx: u32, pub facial: u32, pub repeat: u32 }

pub fn voice_lines(data: &[u8]) -> Result<Vec<VoiceLine>> {
    let elf = Elf::parse(data)?;
    let syms = elf.symbols(data)?;
    let (addr, size) = syms.iter().find(|s| s.name == "Drone_SfxInfo").map(|s| (s.value, s.size as usize))
        .ok_or_else(|| crate::buf::FormatError::Invalid("ACTION.ELF has no symbol Drone_SfxInfo".into()))?;
    let segs = elf.load_segments(data)?;
    let bytes = segs.iter().find_map(|(va, b, _)| (addr >= *va && (addr - va) as usize + size <= b.len()).then(|| &b[(addr - va) as usize..][..size]))
        .ok_or_else(|| crate::buf::FormatError::Invalid("Drone_SfxInfo outside the loaded data".into()))?;
    let b = Buf(bytes);
    (0..size / 32).map(|k| Ok(VoiceLine { sfx: b.u32(32 * k)?, facial: b.u32(32 * k + 4)?, repeat: b.u32(32 * k + 12)? })).collect()
}

/// `EffectInfo` (ACTION.ELF 0x3037c0, 23 × 0x88): per collision surface (`HITDATA+0x50 & 0x3F`) the bullet-impact
/// (+0x14, `Effect_Bullet`), footstep (+0x16, `Effect_Player`) and body (+0x18) sound ids; 0xFFFF = none.
#[derive(Clone, Debug, Default)]
pub struct SurfaceSounds { pub impact: Vec<u16>, pub footstep: Vec<u16>, pub body: Vec<u16> }

impl SurfaceSounds {
    pub const RECORD: usize = 0x88;
    pub fn from_action_elf(data: &[u8]) -> Result<SurfaceSounds> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        let (addr, size) = syms.iter().find(|s| s.name == "EffectInfo").map(|s| (s.value, s.size as usize))
            .ok_or_else(|| crate::buf::FormatError::Invalid("ACTION.ELF has no symbol EffectInfo".into()))?;
        if size % Self::RECORD != 0 { return invalid(format!("EffectInfo size {size:#x}")); }
        let segs = elf.load_segments(data)?;
        let bytes = segs.iter().find_map(|(va, b, _)| (addr >= *va && (addr - va) as usize + size <= b.len()).then(|| &b[(addr - va) as usize..][..size]))
            .ok_or_else(|| crate::buf::FormatError::Invalid("EffectInfo outside the loaded data".into()))?;
        let b = Buf(bytes);
        let col = |o: usize| (0..size / Self::RECORD).map(|k| b.u16(k * Self::RECORD + o)).collect::<Result<Vec<_>>>();
        Ok(SurfaceSounds { impact: col(0x14)?, footstep: col(0x16)?, body: col(0x18)? })
    }
    /// The sound of `surface` (a `HITDATA+0x50` byte) in `column`; None for 0xFFFF / out of range.
    pub fn get(column: &[u16], surface: u8) -> Option<u32> { column.get((surface & 0x3f) as usize).copied().filter(|&v| v != 0xffff).map(|v| v as u32) }
}

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
