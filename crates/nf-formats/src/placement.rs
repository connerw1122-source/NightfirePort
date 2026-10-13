//! Map static data (block 0x1A): object placements. Spec: docs/formats/placement.md.
//!
//! Record (*measured*: `parsemap_block_map_data_static` 0x1d1998, all 23,329 records on the disc):
//! ```text
//! +0x00 u16 object index in this map (used when hashcode = 0xFFFFFFFF)
//! +0x04 u32 hashcode (else: an entity hashcode resolved through the hash table, i.e. another map's object)
//! +0x08 u32 flags   (0x8000: create a world cel now; others are created by the dynamic-object code)
//! +0x0C f32[3] translation → cel+0x60       +0x18 f32[3] rotation angles → cel+0x70 (radians, `RotMatrix`)
//! +0x24..+0x47 not read by this function
//! +0x48 u32 n, then n × { u32 index, u32 value } parameters (passed to `parseentity_fixup_entity`)
//! size = 0x4C + 8n
//! ```

use crate::buf::{invalid, Buf, Result};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Placement {
    pub index: u16,
    pub hashcode: u32,
    pub flags: u32,
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub params: Vec<(u32, u32)>,
    /// The words at +0x24..+0x47, raw (`parsemap_create_dynamic_objects` passes +0x34..+0x3C to some creators as a
    /// scale vector, e.g. `Drone_AIVolume_Create`).
    pub extra: [u32; 9],
}

impl Placement {
    pub const CREATE_CEL: u32 = 0x8000;
    pub fn by_hash(&self) -> bool { self.hashcode != u32::MAX }
    pub fn creates_cel(&self) -> bool { self.flags & Self::CREATE_CEL != 0 }
    /// +0x34..+0x3C as floats.
    pub fn scale(&self) -> [f32; 3] { [4, 5, 6].map(|k| f32::from_bits(self.extra[k])) }
}

/// Parses a 0x1A payload; it must end exactly at the last record (true for all 2,230 maps).
pub fn parse(payload: &[u8]) -> Result<Vec<Placement>> {
    let b = Buf(payload);
    let mut out = Vec::new();
    let mut p = 0usize;
    while p < payload.len() {
        let n = b.u32(p + 0x48)? as usize;
        if n > 1024 { return invalid(format!("placement at {p:#x}: {n} parameters")); }
        let f = |o: usize| b.f32(p + o);
        let mut params = Vec::with_capacity(n);
        for k in 0..n { params.push((b.u32(p + 0x4c + 8 * k)?, b.u32(p + 0x50 + 8 * k)?)); }
        out.push(Placement {
            index: b.u16(p)?, hashcode: b.u32(p + 4)?, flags: b.u32(p + 8)?,
            translation: [f(0x0c)?, f(0x10)?, f(0x14)?], rotation: [f(0x18)?, f(0x1c)?, f(0x20)?], params,
            extra: { let mut e = [0u32; 9]; for (k, w) in e.iter_mut().enumerate() { *w = b.u32(p + 0x24 + 4 * k)?; } e },
        });
        p += 0x4c + 8 * n;
    }
    if p != payload.len() { return invalid("placement records overrun the block"); }
    Ok(out)
}

/// `RotMatrix` (ACTION.ELF 0x1e2368) followed by `Mat_SetTrans`: a row-vector matrix (`v' = v·M`, translation in
/// row 3), entries exactly as the decomp computes them. The game takes sines from `PS2Sinf3` (its own
/// approximation) and multiplies on the EE FPU; `nf_game::ps2math` + `anim::sinf` implement that bit-exactly and
/// the diff harness proves the pieces (PS2Sinf 40/40 recorded calls) — but switching this function to them flips
/// a damage outcome in the 07000001 contact-escort walk (the escort stalls; see docs/reports/verify-bitexact.md),
/// so the platform sine stays until that divergence is root-caused (G-017: the port is proven, the call site is
/// the open item).
pub fn object_matrix(rot: [f32; 3], trans: [f32; 3]) -> [[f32; 4]; 4] {
    let (sx, cx) = rot[0].sin_cos();
    let (sy, cy) = rot[1].sin_cos();
    let (sz, cz) = rot[2].sin_cos();
    [
        [cy * cz, sx * sy * cz + cx * sz, cx * -sy * cz + sx * sz, 0.0],
        [cy * -sz, cx * cz - sx * sy * sz, sx * cz - cx * -sy * sz, 0.0],
        [sy, -sx * cy, cx * cy, 0.0],
        [trans[0], trans[1], trans[2], 1.0],
    ]
}

pub fn transform(m: &[[f32; 4]; 4], v: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|c| v[0] * m[0][c] + v[1] * m[1][c] + v[2] * m[2][c] + m[3][c])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(index: u16, hash: u32, flags: u32, t: [f32; 3], r: [f32; 3], params: &[(u32, u32)]) -> Vec<u8> {
        let mut v = vec![0u8; 0x4c];
        v[0..2].copy_from_slice(&index.to_le_bytes());
        v[4..8].copy_from_slice(&hash.to_le_bytes());
        v[8..12].copy_from_slice(&flags.to_le_bytes());
        for k in 0..3 {
            v[0x0c + 4 * k..0x10 + 4 * k].copy_from_slice(&t[k].to_le_bytes());
            v[0x18 + 4 * k..0x1c + 4 * k].copy_from_slice(&r[k].to_le_bytes());
        }
        v[0x48..0x4c].copy_from_slice(&(params.len() as u32).to_le_bytes());
        for (i, x) in params { v.extend(i.to_le_bytes()); v.extend(x.to_le_bytes()); }
        v
    }

    #[test]
    fn parses_records_with_parameters() {
        let d = [record(3, u32::MAX, 0xc047, [1.0, 2.0, 3.0], [0.0; 3], &[(0, 7), (5, 9)]), record(0, 0x0200_0644, 0x4e, [0.0; 3], [0.0, 1.5, 0.0], &[])].concat();
        let p = parse(&d).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].index, p[0].creates_cel(), p[0].by_hash(), p[0].params.clone()), (3, true, false, vec![(0, 7), (5, 9)]));
        assert_eq!((p[1].hashcode, p[1].creates_cel(), p[1].rotation[1]), (0x0200_0644, false, 1.5));
        assert!(parse(&d[..d.len() - 1]).is_err());
    }

    #[test]
    fn matrix_matches_the_decomp_for_single_axes() {
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-6);
        let h = std::f32::consts::FRAC_PI_2;
        // Identity and translation.
        assert_eq!(transform(&object_matrix([0.0; 3], [1.0, 2.0, 3.0]), [1.0, 0.0, 0.0]), [2.0, 2.0, 3.0]);
        // Entries from RotMatrix with only y = π/2: m[2][0] = sin y = 1, m[0][2] = −sin y.
        let m = object_matrix([0.0, h, 0.0], [0.0; 3]);
        assert!(close(transform(&m, [1.0, 0.0, 0.0]), [0.0, 0.0, -1.0]));
        assert!(close(transform(&m, [0.0, 0.0, 1.0]), [1.0, 0.0, 0.0]));
        // Only z = π/2: x → (cz, sz, ..) = y.
        let m = object_matrix([0.0, 0.0, h], [0.0; 3]);
        assert!(close(transform(&m, [1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]));
    }
}
