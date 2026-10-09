//! Collision geometry (map block 0x2E, `parsemap_block_Coll_Data_New` 0x1d1810). Spec: docs/formats/collision.md.
//!
//! Payload (after the block's id/size word): u32 version (4 or 5; 5 has 16 more header bytes), u16 box count, u16
//! triangle count, u16 vertex-block count; data at +0xC (v4) / +0x1C (v5): vertex blocks (0x40 bytes each, s16
//! values), boxes (0x30 each), triangles (8 bytes each). Read by `Intersect_RayGeom` (0x10a8c8) and
//! `ASH_RecurseBoxesRay` (0x10af30).

use crate::buf::{invalid, Buf, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct CollBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// Internal node: child box indices. Leaf: `None`.
    pub children: Option<(u16, u16)>,
    /// Leaf: triangle range and first vertex block / count.
    pub tris: std::ops::Range<u16>,
    pub vblock: u16,
    pub vblocks: u16,
    /// Leaf vertex decode: `p = s16 · scale + origin`.
    pub origin: [f32; 3],
    pub scale: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollTri {
    /// Halfword indices into the leaf's vertex blocks (3 s16 per vertex).
    pub v: [u16; 3],
    /// Halfword index of the plane (3 s16 normal · 2⁻¹⁴, then an i32 distance).
    pub plane: u16,
}

#[derive(Clone, Debug)]
pub struct Collision {
    pub version: u32,
    pub boxes: Vec<CollBox>,
    pub tris: Vec<CollTri>,
    /// The vertex-block area as s16 values.
    pub words: Vec<i16>,
    /// One surface byte per triangle, after the triangles (`parsemap_block_Coll_Data_New` puts its address at the
    /// geometry's +8; `Intersect_RayGeom` copies `[triangle]` to `HITDATA+0x50`; `& 0x3F` indexes `EffectInfo`).
    pub materials: Vec<u8>,
}

impl Collision {
    pub fn parse(payload: &[u8]) -> Result<Collision> {
        let b = Buf(payload);
        let (version, nbox, ntri, nvb) = (b.u32(0)?, b.u16(4)? as usize, b.u16(6)? as usize, b.u16(8)? as usize);
        let base = match version { 4 => 0xc, 5 => 0x1c, v => return invalid(format!("collision version {v}")) };
        let words = (0..nvb * 0x20).map(|k| b.i16(base + 2 * k)).collect::<Result<_>>()?;
        let bo = base + nvb * 0x40;
        let mut boxes = Vec::with_capacity(nbox);
        for k in 0..nbox {
            let o = bo + 0x30 * k;
            let f = |i: usize| b.f32(o + 4 * i);
            let c0 = b.u16(o + 0xc)?;
            let leaf = c0 & 0x8000 != 0;
            boxes.push(CollBox {
                min: [f(0)?, f(1)?, f(2)?],
                max: [f(4)?, f(5)?, f(6)?],
                children: (!leaf).then_some((c0, b.u16(o + 0xe)?)),
                tris: if leaf { b.u16(o + 0xe)?..b.u16(o + 0x1c)? } else { 0..0 },
                vblock: if leaf { b.u16(o + 0x1e)? } else { 0 },
                vblocks: if leaf { c0 & 0x7fff } else { 0 },
                origin: [f(8)?, f(9)?, f(10)?],
                scale: f(11)?,
            });
        }
        let to = bo + 0x30 * nbox;
        let tris = (0..ntri).map(|k| {
            let o = to + 8 * k;
            Ok(CollTri { v: [b.u16(o)?, b.u16(o + 2)?, b.u16(o + 4)?], plane: b.u16(o + 6)? })
        }).collect::<Result<_>>()?;
        let mo = to + 8 * ntri;
        let materials = b.sub(mo, ntri, "collision materials")?.bytes().to_vec();
        Ok(Collision { version, boxes, tris, words, materials })
    }

    /// World-free triangle list: for every leaf, its triangles' three vertices (`s16 · scale + origin`).
    pub fn triangles(&self) -> Vec<[[f32; 3]; 3]> { self.triangles_with_material().into_iter().map(|t| t.0).collect() }

    /// [`Collision::triangles`] with each triangle's surface byte.
    pub fn triangles_with_material(&self) -> Vec<([[f32; 3]; 3], u8)> {
        let mut out = Vec::new();
        for bx in self.boxes.iter().filter(|b| b.children.is_none()) {
            let base = bx.vblock as usize * 0x20;
            for (ti, t) in self.tris[bx.tris.start as usize..bx.tris.end as usize].iter().enumerate() {
                let m = self.materials.get(bx.tris.start as usize + ti).copied().unwrap_or(0);
                out.push((t.v.map(|i| {
                    let w = |k: usize| self.words.get(base + i as usize + k).copied().unwrap_or(0) as f32;
                    [w(0) * bx.scale + bx.origin[0], w(1) * bx.scale + bx.origin[1], w(2) * bx.scale + bx.origin[2]]
                }), m));
            }
        }
        out
    }
}
