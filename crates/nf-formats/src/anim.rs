//! Animation sequences (type 4, prefix 04) and skeletal posing / skinning. Spec: docs/formats/animation.md.
//!
//! Channel decoding mirrors `AnimFrameCopy` (ACTION.ELF 0x1c5668); the quaternion → matrix formula is
//! `Quat_QuatTransToMat` (0x1e3fa8); bone composition follows `psiBuildMatrixPalette` (0x111498). Matrices are
//! row-vector (`v' = v·M`, translation in row 3), as everywhere in the game.

use crate::buf::{invalid, Buf, Result};
use crate::character::{Skeleton, Skin};

pub type Mat4 = [[f32; 4]; 4];

/// One polynomial segment: valid for integer `t = frame − 1` up to and including `end` (cumulative frame count).
#[derive(Clone, Debug, PartialEq)]
pub struct Segment { pub end: u32, pub c: [f32; 6] }

#[derive(Clone, Debug, PartialEq)]
pub struct Sequence {
    pub hashcode: u32,
    pub flags: u8,
    pub frames: u16,
    pub skeleton: u8,
    pub channels: Vec<Vec<Segment>>,
}

impl Sequence {
    /// Parses header and every channel; each channel must consume exactly its byte length and its segments must
    /// reach `frames − 1` (true for all 18,244 sequences on the disc).
    pub fn parse(data: &[u8]) -> Result<Sequence> {
        let b = Buf(data);
        let (frames, count, skeleton) = (b.u16(0x38)?, b.u16(0x3a)? as usize, b.u8(0x3c)?);
        let last = (frames as u32).saturating_sub(1);
        let mut channels = Vec::with_capacity(count);
        let mut p = 0x40usize;
        for ch in 0..count {
            let mut w = b.u32(p)?;
            let blen = (w & 0xffff) as usize;
            let mut q = p + 4;
            let (mut use_hi, mut cum) = (true, 0u32);
            let mut segs = Vec::new();
            loop {
                let e = if use_hi { w >> 16 } else { w = b.u32(q)?; q += 4; w & 0xffff };
                use_hi = !use_hi;
                cum += e >> 6;
                let mut c = [0f32; 6];
                for (k, slot) in c.iter_mut().enumerate() {
                    if e >> k & 1 != 0 { *slot = b.f32(q)?; q += 4; }
                }
                segs.push(Segment { end: cum, c });
                if cum >= last { break; }
                if q - p > blen { return invalid(format!("channel {ch}: segments overrun its byte length")); }
            }
            if q - p != blen { return invalid(format!("channel {ch}: {} bytes used, header says {blen}", q - p)); }
            channels.push(segs);
            p += blen;
        }
        Ok(Sequence { hashcode: b.u32(0)?, flags: b.u8(0x0c)?, frames, skeleton, channels })
    }

    /// Channel value at integer frame `frame` (1-based), as `AnimFrameCopy`: the first segment whose cumulative end
    /// reaches `t = frame − 1`, evaluated at absolute `t`.
    pub fn value(&self, channel: usize, frame: u32) -> f32 {
        let t = frame.saturating_sub(1);
        let segs = &self.channels[channel];
        let s = segs.iter().find(|s| t <= s.end).unwrap_or(segs.last().unwrap());
        let tf = t as f32;
        let mut p = 1.0f32;
        let mut v = 0.0f32;
        for c in s.c { v += c * p; p *= tf; }
        v
    }
}

/// `Quat_QuatTransToMat`: (x, y, z, w) and a translation → row-vector matrix.
pub fn quat_trans_to_mat(q: [f32; 4], t: [f32; 3]) -> Mat4 {
    let [x, y, z, w] = q;
    let (y2, z2, xx2, xw2) = (y + y, z + z, (x + x) * x, (x + x) * w);
    [
        [1.0 - (y2 * y + z2 * z), y2 * x + z2 * w, z2 * x - y2 * w, 0.0],
        [y2 * x - z2 * w, 1.0 - (xx2 + z2 * z), z2 * y + xw2, 0.0],
        [z2 * x + y2 * w, z2 * y - xw2, 1.0 - (xx2 + y2 * y), 0.0],
        [t[0], t[1], t[2], 1.0],
    ]
}

pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [[0f32; 4]; 4];
    for i in 0..4 { for j in 0..4 { r[i][j] = (0..4).map(|k| a[i][k] * b[k][j]).sum(); } }
    r
}

pub fn transform(m: &Mat4, v: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|c| v[0] * m[0][c] + v[1] * m[1][c] + v[2] * m[2][c] + m[3][c])
}

/// Local (translation, quaternion) per bone at an integer frame: per bone tx, ty, tz (only if the skeleton mask bit
/// is set; × skin scale) then qx, qy, qz with w rebuilt from the qz ≥ 2 sign trick. Bones without translation
/// channels use the skeleton offset × skin scale.
pub fn local_pose(seq: &Sequence, skel: &Skeleton, skin: &Skin, frame: u32) -> Vec<([f32; 3], [f32; 4])> {
    let mut ch = 0;
    let mut out = Vec::with_capacity(skel.bones as usize);
    for bone in 0..skel.bones as usize {
        let t = if skel.has_translation(bone) {
            let t = [0, 1, 2].map(|k| seq.value(ch + k, frame) * skin.scale[k]);
            ch += 3;
            t
        } else {
            [0, 1, 2].map(|k| skel.offsets[bone][k] * skin.scale[k])
        };
        let (x, y, mut z) = (seq.value(ch, frame), seq.value(ch + 1, frame), seq.value(ch + 2, frame));
        ch += 3;
        let negative = z >= 2.0;
        if negative { z -= 4.0; }
        let ww = 1.0 - (x * x + y * y + z * z);
        let w = if ww > 0.0 { if negative { -ww.sqrt() } else { ww.sqrt() } } else { 0.0 };
        out.push((t, [x, y, z, w]));
    }
    out
}

/// World matrices: root = local; child = local · parent (row vectors: a child's rotation applied before its parent's,
/// translation `t·R_parent + T_parent`), as `psiBuildMatrixPalette`. Bones the skin does not animate keep their bind
/// world matrix (the inverse of the inverse bind).
pub fn world_pose(local: &[([f32; 3], [f32; 4])], skin: &Skin) -> Vec<Mat4> {
    let mut world: Vec<Mat4> = Vec::with_capacity(local.len());
    for (bone, (t, q)) in local.iter().enumerate() {
        let m = if skin.animated(bone) || skin.inverse_bind.is_empty() {
            let l = quat_trans_to_mat(*q, *t);
            match skin.parent(bone) { Some(p) => mul(&l, &world[p]), None => l }
        } else {
            let (ib, _) = skin.inverse_bind[bone];
            quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [-ib[0], -ib[1], -ib[2]])
        };
        world.push(m);
    }
    world
}

/// Skinning matrices: world rotation, translation `inv_bind_t · R + world_t` (identity in bind pose).
pub fn skin_matrices(world: &[Mat4], skin: &Skin) -> Vec<Mat4> {
    world.iter().enumerate().map(|(bone, m)| {
        let ib = skin.inverse_bind.get(bone).map(|b| b.0).unwrap_or([0.0; 3]);
        let mut s = *m;
        s[3] = [0, 1, 2].map(|c| ib[0] * m[0][c] + ib[1] * m[1][c] + ib[2] * m[2][c] + m[3][c]).into_iter().chain([1.0]).collect::<Vec<_>>().try_into().unwrap();
        s
    }).collect()
}

/// `p' = w·(p·S[bone(slot0)]) + (1−w)·(p·S[bone(slot1)])`.
pub fn skin_vertex(pos: [f32; 3], weight: f32, slots: [u16; 2], slot_bones: &[u8], skin: &[Mat4]) -> [f32; 3] {
    let m = |s: u16| slot_bones.get(s as usize).and_then(|&b| skin.get(b as usize));
    match (m(slots[0]), m(slots[1])) {
        (Some(a), Some(b)) => {
            let (pa, pb) = (transform(a, pos), transform(b, pos));
            [0, 1, 2].map(|k| weight * pa[k] + (1.0 - weight) * pb[k])
        }
        (Some(a), None) => transform(a, pos),
        _ => pos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(entries: &[(u32, &[f32])]) -> Vec<u8> {
        // AnimFrameCopy order: word0 = blen | e0<<16, e0 floats; then [e1 | e2<<16], e1 floats, e2 floats; …
        let enc = |frames: u32, cs: &[f32]| ((frames << 6) | ((1u32 << cs.len()) - 1)) as u16;
        let mut body: Vec<u8> = Vec::new();
        let mut words: Vec<(usize, u16)> = Vec::new();
        let mut i = 0;
        while i < entries.len() {
            if i == 0 {
                words.push((0, enc(entries[0].0, entries[0].1)));
                for f in entries[0].1 { body.extend(f.to_le_bytes()); }
                i += 1;
            } else {
                let lo = enc(entries[i].0, entries[i].1);
                let hi = entries.get(i + 1).map(|e| enc(e.0, e.1)).unwrap_or(0);
                body.extend(((hi as u32) << 16 | lo as u32).to_le_bytes());
                for f in entries[i].1 { body.extend(f.to_le_bytes()); }
                if let Some(e) = entries.get(i + 1) { for f in e.1 { body.extend(f.to_le_bytes()); } }
                i += 2;
            }
        }
        let blen = 4 + body.len();
        let mut out = ((words[0].1 as u32) << 16 | blen as u32).to_le_bytes().to_vec();
        out.extend(body);
        out
    }

    fn sequence(frames: u16, channels: &[Vec<u8>]) -> Vec<u8> {
        let mut d = vec![0u8; 0x40];
        d[0x38..0x3a].copy_from_slice(&frames.to_le_bytes());
        d[0x3a..0x3c].copy_from_slice(&(channels.len() as u16).to_le_bytes());
        for c in channels { d.extend(c); }
        d
    }

    #[test]
    fn decodes_paired_entries_and_evaluates_absolute_t() {
        // frames 1..=5 → t 0..=4; segment A covers t ≤ 2 (c0 + c1 t), B covers t ≤ 4 (constant 9), C pads nothing.
        let ch = channel(&[(2, &[1.0, 0.5]), (2, &[9.0])]);
        let seq = Sequence::parse(&sequence(5, &[ch])).unwrap();
        assert_eq!(seq.channels[0].len(), 2);
        assert_eq!((seq.value(0, 1), seq.value(0, 3), seq.value(0, 4), seq.value(0, 5)), (1.0, 2.0, 9.0, 9.0));
        let three = channel(&[(1, &[1.0]), (1, &[2.0]), (2, &[3.0])]);
        let s3 = Sequence::parse(&sequence(5, &[three])).unwrap();
        assert_eq!((s3.value(0, 1), s3.value(0, 2), s3.value(0, 3), s3.value(0, 5)), (1.0, 1.0, 2.0, 3.0));
    }

    #[test]
    fn rejects_byte_length_mismatch() {
        let mut ch = channel(&[(4, &[1.0])]);
        ch[0] += 4; // claim 4 more bytes than the segments use
        ch.extend([0; 4]);
        assert!(Sequence::parse(&sequence(5, &[ch])).is_err());
    }

    #[test]
    fn quaternion_matrix_and_skinning_identity() {
        let id = quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [1.0, 2.0, 3.0]);
        assert_eq!(transform(&id, [1.0, 0.0, 0.0]), [2.0, 2.0, 3.0]);
        // 90° about z: x → y (rows are rotated axes).
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let rz = quat_trans_to_mat([0.0, 0.0, s, s], [0.0; 3]);
        let v = transform(&rz, [1.0, 0.0, 0.0]);
        assert!((v[0]).abs() < 1e-6 && (v[1] - 1.0).abs() < 1e-6);
        // Blend half-way between two translations.
        let a = quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]);
        let b = quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [2.0, 0.0, 0.0]);
        assert_eq!(skin_vertex([1.0, 0.0, 0.0], 0.5, [0, 1], &[0, 1], &[a, b]), [2.0, 0.0, 0.0]);
    }
}
