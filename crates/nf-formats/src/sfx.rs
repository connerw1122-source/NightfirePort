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
pub struct SurfaceSounds {
    pub impact: Vec<u16>, pub footstep: Vec<u16>, pub body: Vec<u16>,
    /// The non-sound `EffectInfo` columns (decals, ricochet, gas puffs, debris), same record stride.
    pub effects: Vec<EffectRecord>,
    /// `ammo_data` (fix-impacts): the per-ammo-type casing mesh, parsed from the same ACTION.ELF
    /// (default empty when the symbol is absent so the sound-only parse never fails).
    pub ammo: AmmoData,
}

/// One `EffectInfo` gas-puff entry (+0x20 + k·0x1c, k = 0..3) — `Effect_Bullet` 0x1ce8a8 passes it to
/// `Gas_Init` 0x1aacc8 [M disasm 0x1ce964..0x1cea90: t0=+0 mesh, t1=+4 type, t2=+8 rgba start, t3=+0xc rgba end,
/// f12=+0x10 scale start, f13=+0x14 scale end, u16 +0x18 life].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EffectGas {
    pub mesh: u32,
    pub kind: u32,
    /// Packed rgba start / end (MSB = r … LSB = a, the port's gas.rs convention).
    pub rgba: [u32; 2],
    pub scale: [f32; 2],
    pub life: u16,
    /// Entry+0x1a: 0 = puff at the hit position; ≠0 = hit + normal·0.05, and the puff is skipped
    /// when |normal.y| > 0.3 (walls only).
    pub wall_offset: u8,
    /// Entry+0x1b: read only when wall_offset == 0 — the velocity is the normal-derived euler vector.
    pub orient_normal: u8,
}

/// One `EffectInfo` record minus the sound columns (those stay in `SurfaceSounds`' own fields).
/// Offsets from the decompile: `Effect_Bullet` 0x1ce8a8, `Effect_Create` 0x1ce688,
/// `Debris_Create` 0x1aaa00, `Effect_RicochetProb` 0x1cf290 and the ricochet reflect in
/// `Bullet_CollisionHandler` 0x12ce80 (which reads +0x1c raw bits as the reflection scalar).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EffectRecord {
    /// +0x04: bullet-hole decal mesh hash; 0 = none (`Effect_Bullet`).
    pub decal: u32,
    /// +0x1a: `Effect_RicochetProb`'s N (0 = never; the roll is `Rand_Rand(N) & mask`).
    pub ricochet_n: u16,
    /// +0x1c: the ricochet reflection scalar, read as raw f32 bits by `Bullet_CollisionHandler`
    /// (0x0000003f on most materials — a denormal ≈ 0, so the reflection is dir − dot·normal).
    pub restitution_bits: u32,
    pub gas: [EffectGas; 3],
    /// +0x74: debris mesh hash; 0 = none.
    pub debris_mesh: u32,
    /// +0x78: debris speed (vel = 0.5·speed·normal + FRandHalf(speed)·basis, × 1/60).
    pub debris_speed: f32,
    /// +0x7c: debris spin scale (`Rand_FRandHalf_Vec`).
    pub debris_spin: f32,
    /// +0x81: debris life base (life = (b>>1) + Rand_Rand(b>>1)).
    pub debris_life: u8,
    /// +0x82: debris enabled (`Effect_Create` gate).
    pub debris_enable: u8,
    /// +0x83: debris count draw (count = Rand_Rand(b) + 1).
    pub debris_count: u8,
    /// +0x84: the impact-emitter def hash — `Effect_Create` 0x1ce688's flags & 0x20 arm passes it
    /// to `Emitter_Create` 0x17ae00 (`hashtable_getitem`; a miss → no object). 0 = none.
    pub emitter: u32,
}

/// `ammo_data` (ACTION.ELF 0x2c69b8, 34 × 0xc): per ammo type the casing mesh at +4
/// (`Bullet_init_casing` 0x12fde0: `ammo_data[weapon_data[w+0x90]] + 4`).
#[derive(Clone, Debug, Default)]
pub struct AmmoData { pub casing_mesh: Vec<u32> }

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
        let n = size / Self::RECORD;
        let effects = (0..n).map(|k| {
            let o = k * Self::RECORD;
            let gas = (0..3).map(|g| {
                let e = o + 0x20 + g * 0x1c;
                Ok(EffectGas {
                    mesh: b.u32(e)?, kind: b.u32(e + 4)?,
                    rgba: [b.u32(e + 8)?, b.u32(e + 0xc)?],
                    scale: [b.f32(e + 0x10)?, b.f32(e + 0x14)?],
                    life: b.u16(e + 0x18)?,
                    wall_offset: b.u8(e + 0x1a)?,
                    orient_normal: b.u8(e + 0x1b)?,
                })
            }).collect::<Result<Vec<_>>>()?;
            Ok(EffectRecord {
                decal: b.u32(o + 4)?,
                ricochet_n: b.u16(o + 0x1a)?,
                restitution_bits: b.u32(o + 0x1c)?,
                gas: [gas[0], gas[1], gas[2]],
                debris_mesh: b.u32(o + 0x74)?,
                debris_speed: b.f32(o + 0x78)?,
                debris_spin: b.f32(o + 0x7c)?,
                debris_life: b.u8(o + 0x81)?,
                debris_enable: b.u8(o + 0x82)?,
                debris_count: b.u8(o + 0x83)?,
                emitter: b.u32(o + 0x84)?,
            })
        }).collect::<Result<Vec<_>>>()?;
        Ok(SurfaceSounds { impact: col(0x14)?, footstep: col(0x16)?, body: col(0x18)?, effects, ammo: AmmoData::from_action_elf(data).unwrap_or_default() })
    }
    /// The sound of `surface` (a `HITDATA+0x50` byte) in `column`; None for 0xFFFF / out of range.
    pub fn get(column: &[u16], surface: u8) -> Option<u32> { column.get((surface & 0x3f) as usize).copied().filter(|&v| v != 0xffff).map(|v| v as u32) }
}

impl AmmoData {
    pub const RECORD: usize = 0xc;
    pub fn from_action_elf(data: &[u8]) -> Result<AmmoData> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        let (addr, size) = syms.iter().find(|s| s.name == "ammo_data").map(|s| (s.value, s.size as usize))
            .ok_or_else(|| crate::buf::FormatError::Invalid("ACTION.ELF has no symbol ammo_data".into()))?;
        if size % Self::RECORD != 0 { return invalid(format!("ammo_data size {size:#x}")); }
        let segs = elf.load_segments(data)?;
        let bytes = segs.iter().find_map(|(va, b, _)| (addr >= *va && (addr - va) as usize + size <= b.len()).then(|| &b[(addr - va) as usize..][..size]))
            .ok_or_else(|| crate::buf::FormatError::Invalid("ammo_data outside the loaded data".into()))?;
        let b = Buf(bytes);
        let casing_mesh = (0..size / Self::RECORD).map(|k| b.u32(k * Self::RECORD + 4)).collect::<Result<Vec<_>>>()?;
        Ok(AmmoData { casing_mesh })
    }
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
