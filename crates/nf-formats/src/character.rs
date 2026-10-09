//! Characters: skeletons (level `.bin` type 6, `SKELnnnn`) and skins (type 3, hashcode prefix 05), which name the
//! skinned meshes a character is drawn with. Spec: docs/formats/character.md.
//!
//! Read in exactly the order `AnimSkeletonProcess` (ACTION.ELF 0x1bd328) and `AnimProcessSkinData` (0x1bd500) do:
//! ```text
//! skeleton: +0 u16 skeleton id (index into pSkeletons), +2 u8 bone count, …
//! skin:     +0 u32 hashcode, +4 f32[3], +0x10 u8 main_count, +0x11 u8 extra_count, +0x12 u8, +0x13 u8 tail_count,
//!           +0x14 u8 skeleton id, then bone_count bytes,
//!           if extra_count: extra_count bytes, align 4, extra_count × u32 hashcode (−1 = none),
//!           if main_count:  align 4, main_count × u32 hashcode (−1 = none; byte 2 bit 0x10 → "sleeve" substitute),
//!           then per-bone matrices and tail_count × 0x24 bytes (not needed for a bind-pose view).
//! ```

use crate::buf::{invalid, Buf, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skeleton { pub id: u16, pub bones: u8 }

pub fn parse_skeleton(data: &[u8]) -> Result<Skeleton> {
    let b = Buf(data);
    Ok(Skeleton { id: b.u16(0)?, bones: b.u8(2)? })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Skin {
    pub hashcode: u32,
    pub scale: [f32; 3],
    pub skeleton: u8,
    /// Meshes the character is drawn with (`skin+4` list). `None` = −1.
    pub main: Vec<Option<u32>>,
    /// The second list (`skin+8`), with its per-entry bytes.
    pub extra: Vec<Option<u32>>,
}

impl Skin {
    /// A main entry whose hashcode byte 2 has bit 0x10 is replaced at load by `AnimSleeveGetEntity(0)`.
    pub fn is_sleeve(hash: u32) -> bool { (hash >> 16) & 0x10 != 0 }
}

/// `bone_count(skeleton id)` must give the bone count of that skeleton (from the archive's SKEL entries).
pub fn parse_skin(data: &[u8], bone_count: impl Fn(u8) -> Option<u8>) -> Result<Skin> {
    let b = Buf(data);
    let (main_count, extra_count, skeleton) = (b.u8(0x10)? as usize, b.u8(0x11)? as usize, b.u8(0x14)?);
    let Some(bones) = bone_count(skeleton) else { return invalid(format!("skin uses skeleton {skeleton}, which is not loaded")) };
    let mut p = 0x15 + bones as usize;
    let list = |p: &mut usize, n: usize| -> Result<Vec<Option<u32>>> {
        *p = p.next_multiple_of(4);
        let v = (0..n).map(|k| b.u32(*p + 4 * k).map(|h| (h != u32::MAX).then_some(h))).collect::<Result<_>>()?;
        *p += 4 * n;
        Ok(v)
    };
    let mut extra = Vec::new();
    if extra_count != 0 {
        p += extra_count;
        extra = list(&mut p, extra_count)?;
    }
    let main = if main_count != 0 { list(&mut p, main_count)? } else { Vec::new() };
    Ok(Skin { hashcode: b.u32(0)?, scale: [b.f32(4)?, b.f32(8)?, b.f32(12)?], skeleton, main, extra })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_synthetic_skin_in_loader_order() {
        // 3 bones, no extra list, one main mesh: 0x15 + 3 = 0x18, already aligned.
        let mut d = vec![0u8; 0x1c];
        d[0..4].copy_from_slice(&0x0500_0030u32.to_le_bytes());
        d[4..8].copy_from_slice(&1.0f32.to_le_bytes());
        d[0x10] = 1;
        d[0x14] = 7;
        d[0x18..0x1c].copy_from_slice(&0x0200_040du32.to_le_bytes());
        let s = parse_skin(&d, |id| (id == 7).then_some(3)).unwrap();
        assert_eq!((s.skeleton, s.main.clone(), s.extra.len()), (7, vec![Some(0x0200_040d)], 0));
        assert!(parse_skin(&d, |_| None).is_err(), "skeleton must be known");
        // With 4 bones the list starts at align4(0x19) = 0x1c: past the data.
        assert!(parse_skin(&d, |_| Some(4)).is_err());
    }

    #[test]
    fn skeleton_header() {
        assert_eq!(parse_skeleton(&[2, 0, 22, 0]).unwrap(), Skeleton { id: 2, bones: 22 });
        assert!(Skin::is_sleeve(0x0210_0001) && !Skin::is_sleeve(0x0200_040d));
    }
}
