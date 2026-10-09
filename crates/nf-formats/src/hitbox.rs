//! `HumanCollisionBones` (ACTION.ELF 0x2c6be8) and `AstronautCollisionBones` (0x2c6d40): the boxes
//! `AnimGetCollData` (0x1c4ac8) builds on a character's skeleton for bullet hits. 20 bytes per entry: s16 bone (the
//! hit part), s16 other bone (−1 = the bone's skeleton parent), f32 unused, f32 ×3 half-extents; the list ends at a
//! negative bone id. Spec: docs/systems/drone-damage.md.

use crate::buf::{Buf, FormatError, Result};
use crate::elf::Elf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionBone { pub bone: i16, pub other: i16, pub half: [f32; 3] }

#[derive(Clone, Debug)]
pub struct CollisionBones { pub human: Vec<CollisionBone>, pub astronaut: Vec<CollisionBone> }

impl CollisionBones {
    pub fn from_action_elf(data: &[u8]) -> Result<CollisionBones> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        let segs = elf.load_segments(data)?;
        let read = |name: &str| -> Result<Vec<CollisionBone>> {
            let addr = syms.iter().find(|s| s.name == name).map(|s| s.value)
                .ok_or_else(|| FormatError::Invalid(format!("ACTION.ELF has no symbol {name}")))?;
            let bytes = segs.iter().find_map(|(va, b, _)| (addr >= *va && ((addr - va) as usize) < b.len()).then(|| &b[(addr - va) as usize..]))
                .ok_or_else(|| FormatError::Invalid(format!("{name} outside the loaded data")))?;
            let b = Buf(bytes);
            let mut out = Vec::new();
            for k in 0..64 {
                let o = k * 20;
                let bone = b.u16(o)? as i16;
                if bone < 0 { return Ok(out); }
                out.push(CollisionBone { bone, other: b.u16(o + 2)? as i16, half: [b.f32(o + 8)?, b.f32(o + 12)?, b.f32(o + 16)?] });
            }
            Err(FormatError::Invalid(format!("{name}: no terminator in 64 entries")))
        };
        Ok(CollisionBones { human: read("HumanCollisionBones")?, astronaut: read("AstronautCollisionBones")? })
    }

    /// The table `AnimGetCollData` uses for a skin.
    pub fn for_skin(&self, skin: u32) -> &[CollisionBone] { if skin == 0x0500_0078 { &self.astronaut } else { &self.human } }
}
