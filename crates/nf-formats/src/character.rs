//! Characters: skeletons (level `.bin` type 6, `SKELnnnn`) and skins (type 3, hashcode prefix 05).
//! Spec: docs/formats/animation.md, docs/formats/character.md.
//!
//! Read in exactly the order `AnimSkeletonProcess` (ACTION.ELF 0x1bd328) and `AnimProcessSkinData` (0x1bd500) do;
//! the full skin layout matches the file size (padded to 32) in all 1,217 skins on the disc (WORKLOG 0015).

use crate::buf::{invalid, Buf, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct Skeleton {
    pub id: u16,
    pub bones: u8,
    /// Bit i: bone i has translation channels in sequences.
    pub translation_mask: [u32; 3],
    /// Fixed local translation of bones without translation channels.
    pub offsets: Vec<[f32; 3]>,
}

impl Skeleton {
    pub fn has_translation(&self, bone: usize) -> bool { self.translation_mask[bone / 32] >> (bone % 32) & 1 != 0 }
    pub fn translated_bones(&self) -> usize { self.translation_mask.iter().map(|m| m.count_ones() as usize).sum() }
}

pub fn parse_skeleton(data: &[u8]) -> Result<Skeleton> {
    let b = Buf(data);
    let bones = b.u8(2)?;
    let offsets = (0..bones as usize).map(|i| Ok([b.f32(0x10 + 12 * i)?, b.f32(0x14 + 12 * i)?, b.f32(0x18 + 12 * i)?])).collect::<Result<_>>()?;
    Ok(Skeleton { id: b.u16(0)?, bones, translation_mask: [b.u32(4)?, b.u32(8)?, b.u32(12)?], offsets })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Skin {
    pub hashcode: u32,
    /// Multiplies animated and skeleton translations.
    pub scale: [f32; 3],
    pub skeleton: u8,
    /// Per bone: bit 7 = animated, low 7 bits = parent (0x7F = root); parents precede children.
    pub bones: Vec<u8>,
    /// Meshes the character is drawn with (`skin+4` list). `None` = −1.
    pub main: Vec<Option<u32>>,
    /// Rigid parts: (bone, hashcode) (`skin+8` list).
    pub extra: Vec<(u8, Option<u32>)>,
    /// Per bone (only when there are skinned meshes): inverse-bind translation and quaternion.
    pub inverse_bind: Vec<([f32; 3], [f32; 4])>,
    /// Facial morph count (disc `+0x12` → runtime `AnimSkin+0x2e` [M `AnimProcessSkinData`
    /// 0x1bd500]). `AnimObjectNew` 0x1be910 mallocs this many f32 for the object's morph weight
    /// array (animData+0xb0, zeroed); 22 on every dialogue skin measured — the same count as the
    /// cut-scene scripts' face tracks. fix-facial.
    pub facial: u8,
    /// Attachment points (`AnimDatum*`): weapons and props hang here.
    pub datums: Vec<Datum>,
}

/// 36 bytes: `u32 id, i32 bone (−1 = none), f32 t[3], f32 q[4]` (`AnimObjectDraw` 0x1c4490 builds
/// `QuatTransToMat(q, t)` and concatenates the bone's world matrix).
#[derive(Clone, Debug, PartialEq)]
pub struct Datum { pub id: u32, pub bone: i32, pub t: [f32; 3], pub q: [f32; 4] }

impl Skin {
    /// A main entry whose hashcode byte 2 has bit 0x10 is replaced at load by `AnimSleeveGetEntity(0)`.
    pub fn is_sleeve(hash: u32) -> bool { (hash >> 16) & 0x10 != 0 }
    pub fn parent(&self, bone: usize) -> Option<usize> { match self.bones[bone] & 0x7f { 0x7f => None, p => Some(p as usize) } }
    pub fn animated(&self, bone: usize) -> bool { self.bones[bone] & 0x80 != 0 }
}

/// `bone_count(skeleton id)` must give the bone count of that skeleton (from the archive's SKEL entries).
pub fn parse_skin(data: &[u8], bone_count: impl Fn(u8) -> Option<u8>) -> Result<Skin> {
    let b = Buf(data);
    let (main_count, extra_count, skeleton) = (b.u8(0x10)? as usize, b.u8(0x11)? as usize, b.u8(0x14)?);
    let Some(nb) = bone_count(skeleton) else { return invalid(format!("skin uses skeleton {skeleton}, which is not loaded")) };
    let bones = b.sub(0x15, nb as usize, "skin bones")?.bytes().to_vec();
    let mut p = 0x15 + nb as usize;
    let hashes = |p: &mut usize, n: usize| -> Result<Vec<Option<u32>>> {
        *p = p.next_multiple_of(4);
        let v = (0..n).map(|k| b.u32(*p + 4 * k).map(|h| (h != u32::MAX).then_some(h))).collect::<Result<_>>()?;
        *p += 4 * n;
        Ok(v)
    };
    let mut extra = Vec::new();
    if extra_count != 0 {
        let extra_bones = b.sub(p, extra_count, "skin part bones")?.bytes().to_vec();
        p += extra_count;
        extra = extra_bones.into_iter().zip(hashes(&mut p, extra_count)?).collect();
    }
    let mut main = Vec::new();
    let mut inverse_bind = Vec::new();
    if main_count != 0 {
        main = hashes(&mut p, main_count)?;
        for i in 0..nb as usize {
            let o = p + 0x1c * i;
            let f = |k: usize| b.f32(o + 4 * k);
            inverse_bind.push(([f(0)?, f(1)?, f(2)?], [f(3)?, f(4)?, f(5)?, f(6)?]));
        }
        p += 0x1c * nb as usize;
    }
    let datums = (0..b.u8(0x13)? as usize).map(|k| {
        let o = p + 36 * k;
        let f = |i: usize| b.f32(o + 8 + 4 * i);
        Ok(Datum { id: b.u32(o)?, bone: b.u32(o + 4)? as i32, t: [f(0)?, f(1)?, f(2)?], q: [f(3)?, f(4)?, f(5)?, f(6)?] })
    }).collect::<Result<_>>()?;
    Ok(Skin { hashcode: b.u32(0)?, scale: [b.f32(4)?, b.f32(8)?, b.f32(12)?], skeleton, bones, main, extra, inverse_bind, facial: b.u8(0x12)?, datums })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_synthetic_skin_in_loader_order() {
        // 3 bones, no parts, one main mesh at align4(0x18) = 0x18, then 3 × 0x1C inverse-bind records.
        let mut d = vec![0u8; 0x1c + 3 * 0x1c];
        d[0..4].copy_from_slice(&0x0500_0030u32.to_le_bytes());
        d[4..8].copy_from_slice(&1.0f32.to_le_bytes());
        d[0x10] = 1;
        d[0x14] = 7;
        d[0x15..0x18].copy_from_slice(&[0xff, 0x80, 0x81]);
        d[0x18..0x1c].copy_from_slice(&0x0200_040du32.to_le_bytes());
        d[0x1c + 0x1c..0x1c + 0x20].copy_from_slice(&(-0.8f32).to_le_bytes());
        let s = parse_skin(&d, |id| (id == 7).then_some(3)).unwrap();
        assert_eq!((s.skeleton, s.main.clone(), s.extra.len()), (7, vec![Some(0x0200_040d)], 0));
        assert_eq!((s.parent(0), s.parent(1), s.parent(2), s.animated(0), s.animated(1)), (None, Some(0), Some(1), true, true));
        assert_eq!(s.inverse_bind.len(), 3);
        assert_eq!(s.inverse_bind[1].0[0], -0.8);
        assert!(parse_skin(&d, |_| None).is_err(), "skeleton must be known");
        assert!(parse_skin(&d, |_| Some(4)).is_err(), "4 bones push the records past the data");
    }

    #[test]
    fn skeleton_header_and_offsets() {
        let mut d = vec![0u8; 0x10 + 24];
        d[0] = 2;
        d[2] = 2;
        d[4] = 0b10;
        d[0x1c..0x20].copy_from_slice(&1.5f32.to_le_bytes());
        let s = parse_skeleton(&d).unwrap();
        assert_eq!((s.id, s.bones, s.has_translation(0), s.has_translation(1), s.translated_bones()), (2, 2, false, true, 1));
        assert_eq!(s.offsets[1], [1.5, 0.0, 0.0]);
        assert!(Skin::is_sleeve(0x0210_0001) && !Skin::is_sleeve(0x0200_040d));
    }
}
