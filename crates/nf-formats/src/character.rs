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

    /// `AnimProcessSkinData` 0x1bd500:159-166 — every MAIN-list entry whose hash byte 2 carries bit
    /// 0x10 is replaced, before the celglist lookup, by `AnimSleeveGetEntity(0)` 0x1c6590: the
    /// first present entity of the fixed 8-entry `SleeveEnts` list @0x2c6ed8 [L ee_snapshot,
    /// byte-exact], tried from index 0 with at most 8 lookups; none present → the game stores 0
    /// (nothing drawn for that entry, modelled here as `None`). Non-sleeve entries are untouched.
    /// `present` mirrors the session hashtable (`hashtable_getitem`): true when the entity hash
    /// resolves. Direct entries never go through this — the game only reroutes the 0x10-flagged
    /// ones (`if ((*pbVar2 & 0x10) != 0) iVar4 = AnimSleeveGetEntity__FUi(0);`).
    pub fn substitute_sleeves(&mut self, present: impl Fn(u32) -> bool) {
        for slot in self.main.iter_mut() {
            if !slot.map_or(false, Self::is_sleeve) { continue; }
            *slot = SLEEVE_ENTS.iter().copied().find(|&h| present(h)); // none present → None (the game's 0)
        }
    }
}

/// `SleeveEnts.324` @0x2c6ed8, the priority list `AnimSleeveGetEntity` 0x1c6590 walks (8 × u32
/// LE read at the file offset) [L ee_snapshot; disasm 0x1c65a0 `addiu $s3,$v0,0x6ed8`]. The same
/// set the view-model build calls SLEEVES (`crates/nf-view/src/scene.rs`, `lib.rs`).
pub const SLEEVE_ENTS: [u32; 8] = [
    0x0200_09d0, 0x0200_0900, 0x0200_01f7, 0x0200_0929, 0x0200_09e6, 0x0200_09ff, 0x0200_0a14, 0x0200_0a13,
];

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

    /// `AnimProcessSkinData` 0x1bd500:159-166 via `AnimSleeveGetEntity` 0x1c6590: only the
    /// 0x10-flagged main entries are rerouted, to the first present `SleeveEnts` entity; a direct
    /// entry — even one the session lacks — stays as parsed, and a fully missing sleeve list
    /// drops the entry to `None` (the game's celglist 0).
    #[test]
    fn sleeve_substitution_reroutes_only_flagged_entries() {
        let mut s = Skin {
            hashcode: 0x0500_0030, scale: [1.0; 3], skeleton: 7, bones: vec![0xff],
            main: vec![Some(0x0210_0001), None, Some(0x0200_040d), Some(0x0210_00ff)],
            extra: Vec::new(), inverse_bind: Vec::new(), facial: 0, datums: Vec::new(),
        };
        // The first SleeveEnts entry is present: every 0x10-flagged entry becomes it.
        s.substitute_sleeves(|h| h == SLEEVE_ENTS[0]);
        assert_eq!(s.main, vec![Some(SLEEVE_ENTS[0]), None, Some(0x0200_040d), Some(SLEEVE_ENTS[0])]);
        // A session where only the third list entry is loaded: the walk skips the first two.
        let mut s2 = s.clone();
        let mut s2c = s2.clone();
        s2c.substitute_sleeves(|h| h == SLEEVE_ENTS[2]);
        assert_eq!(s2c.main[0], Some(SLEEVE_ENTS[2]));
        assert_eq!(s2c.main[3], Some(SLEEVE_ENTS[2]));
        // Nothing present at all: the flagged entries drop to None (the game's hashtable 0).
        s2.substitute_sleeves(|_| false);
        assert_eq!(s2.main, vec![None, None, Some(0x0200_040d), None]);
        // The substitute list is the game's byte-exact SleeveEnts @0x2c6ed8.
        assert_eq!(SLEEVE_ENTS[0], 0x0200_09d0);
        assert_eq!(SLEEVE_ENTS[7], 0x0200_0a13);
    }
}
