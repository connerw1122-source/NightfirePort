//! PS2_GFX map blocks (0x2D): glist boxes and their DMA/VIF packets, decoded to triangle-strip batches.
//! Spec: docs/formats/ps2gfx.md.
//!
//! The packet is replayed through a small VIF1 → VU1-memory model (UNPACK with STCYCL, generation-tagged qwords).
//! At every `MSCNT`/`MSCAL`/`MSCALF` the batch is read back from VU memory at the layout the doc describes (GIF tag at
//! B, vertex k at B+1+3k / B+2+3k / B+3+3k, UV at B+0xC7+k), and every qword read must have been written since the
//! previous kick — so the layout is checked on every batch instead of assumed.

use crate::buf::{invalid, Buf, Result};
use crate::vif::{parse_vif, VifPacket, MSCAL, MSCALF, MSCNT, STCYCL};

pub const BOX_SIZE: usize = 0x38;
pub const UV_OFFSET: usize = 0xc7;
const VU1_QWORDS: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct GlistBox {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub children: [i32; 2],
    pub packet: u32,
    pub texture_refs: u32,
    pub ret_offset: u32,
    pub unknown_2c: u32,
    pub vertex_count: u32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub rgba: [u8; 4],
    pub normal: [i8; 3],
    /// The signed V4-8 stream's w: −1 = no kick (does not close a triangle), 0 = kick.
    pub w: i32,
    /// Skinned only: weight of the first matrix (source qword 0 w) and the two matrix slots (qword 1 words 0/1, low
    /// 15 bits); slots index `GfxObject::slot_bones`. Static vertices: 1.0 and [0, 0].
    pub weight: f32,
    pub slots: [u16; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    /// Map texture index from the latest texture REF, if any.
    pub texture: Option<u32>,
    pub prim: u16,
    pub nloop: u16,
    pub eop: bool,
    /// From a skinned object: positions are the bind-pose source positions, `w` is the raw packed skinning word.
    pub skinned: bool,
    pub verts: Vec<Vertex>,
    /// The GS register state in effect at the kick: ALPHA_1 (0x42), TEST_1 (0x47), ZBUF_1 (0x4e). The GS applies
    /// DIRECT A+D writes in DMA order, so a leaf with two ALPHA_1 writes has different blend modes per batch group.
    /// The port snapshots these at kick time so each batch carries its own state [M G-018 fix].
    pub alpha: u64,
    pub test: u64,
    pub zbuf: u64,
}

/// Skinned vertices: bit 15 of the packed skinning word (source qword 1, word 3) marks "no kick": set on all 51,664
/// strip-start vertices (positions 0–1) on the disc and on 25 % of the others. The upper half-word is the morph index
/// `SkinIt` reads (−1 = none). WORKLOG 0013.
pub const SKIN_NO_KICK: i32 = 0x8000;

impl Batch {
    pub fn kicks(&self, i: usize) -> bool {
        let w = self.verts[i].w;
        if self.skinned { w & SKIN_NO_KICK == 0 } else { w != -1 }
    }

    /// Strip triangles: vertex i ≥ 2 closes (i−2, i−1, i) if it kicks; winding alternates with i's parity.
    pub fn triangles(&self) -> Vec<[u16; 3]> {
        (2..self.verts.len()).filter(|&i| self.kicks(i))
            .map(|i| { let i = i as u16; if i % 2 == 0 { [i - 2, i - 1, i] } else { [i - 1, i - 2, i] } })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Leaf {
    pub box_index: usize,
    pub batches: Vec<Batch>,
    /// Kicks whose header was sent without vertex data (the first CNT of every packet).
    pub setup_kicks: u32,
    /// GS registers the packet's `DIRECT` A+D writes set (register, value) — in 07000005 only ALPHA_1 (0x42), TEST_1
    /// (0x47) and ZBUF_1 (0x4E) (docs/systems/render.md). Empty: the leaf inherits the previous leaf's state.
    pub gs: Vec<(u8, u64, usize)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GfxObject {
    pub boxes: Vec<GlistBox>,
    pub skin_info: u32,
    pub footer_unknown: u32,
    /// Decoded leaves (skinned objects: bind pose).
    pub leaves: Vec<Leaf>,
    /// Skinned objects: matrix slot → skeleton bone (the 0xFF-terminated list at skin-info word 3).
    pub slot_bones: Vec<u8>,
}

impl GfxObject {
    pub fn is_skinned(&self) -> bool { self.skin_info != 0 }

    /// `block` is the whole 0x2D block including its 4-byte block header (offsets in the data are relative to it).
    pub fn parse(block: &[u8]) -> Result<GfxObject> {
        let b = Buf(block);
        let footer = b.u32(4)? as usize;
        if footer == 0x10 { return Ok(GfxObject { boxes: Vec::new(), skin_info: 0, footer_unknown: 0, leaves: Vec::new(), slot_bones: Vec::new() }); }
        let footer = footer & !3;
        let (count, box_off, skin_info, footer_unknown) = (b.u32(footer)? as usize, b.u32(footer + 4)? as usize & !3, b.u32(footer + 8)?, b.u32(footer + 12)?);
        if count > 1 << 16 { return invalid(format!("PS2_GFX: {count} boxes")); }
        let mut boxes = Vec::with_capacity(count);
        for i in 0..count {
            let o = box_off + i * BOX_SIZE;
            let f = |k: usize| b.f32(o + 4 * k);
            boxes.push(GlistBox {
                a: [f(0)?, f(1)?, f(2)?], b: [f(3)?, f(4)?, f(5)?],
                children: [b.i32(o + 0x18)?, b.i32(o + 0x1c)?],
                packet: b.u32(o + 0x20)?, texture_refs: b.u32(o + 0x24)?, ret_offset: b.u32(o + 0x28)?,
                unknown_2c: b.u32(o + 0x2c)?, vertex_count: b.u32(o + 0x30)?, flags: b.u32(o + 0x34)?,
            });
        }
        // Skinned objects: skin info word 0 = offset of the source vertex buffer (2 qwords per vertex: position with
        // w = 1.0, then packed skinning data). In bind pose (identity bones) `SkinIt`'s output equals its input, and the
        // REF tags' addresses are byte offsets into that buffer (`CacheSkin` adds the buffer base), WORKLOG 0013.
        let skin_src = if skin_info != 0 { Some(b.u32(skin_info as usize & !3)? as usize & !3) } else { None };
        let mut leaves = Vec::new();
        for (i, bx) in boxes.iter().enumerate().filter(|(_, bx)| bx.packet != 0) {
            let leaf = decode_packet(block, bx.packet as usize & !3, skin_src)
                .map_err(|e| crate::buf::FormatError::Invalid(format!("box {i}: {e}")))?;
            leaves.push(Leaf { box_index: i, ..leaf });
        }
        let mut slot_bones = Vec::new();
        if skin_info != 0 {
            let mut q = b.u32((skin_info as usize & !3) + 12)? as usize & !3;
            while slot_bones.len() < 256 {
                match b.u8(q)? { 0xff => break, x => slot_bones.push(x) }
                q += 1;
            }
        }
        Ok(GfxObject { boxes, skin_info, footer_unknown, leaves, slot_bones })
    }
}

struct Vu {
    mem: Vec<[u32; 4]>,
    gen: Vec<u32>,
    cur: u32,
    cl: u32,
    wl: u32,
    header: Option<usize>,
    /// The GS register state: ALPHA_1 (0x42), TEST_1 (0x47), ZBUF_1 (0x4e). Updated by DIRECT A+D writes,
    /// snapshotted at kick time into each Batch [M G-018 fix].
    alpha: u64,
    test: u64,
    zbuf: u64,
}

impl Vu {
    /// `inline`: the unpack came from a CNT payload (only those can carry the batch's GIF-tag header; skin REFs never).
    fn unpack(&mut self, p: &VifPacket, inline: bool) -> Result<()> {
        if p.imm & 0x8000 != 0 { return invalid("UNPACK with FLG (TOPS-relative) not supported"); }
        if self.wl > self.cl { return invalid("STCYCL fill mode (WL > CL) not supported"); }
        let (comps, bits) = (p.vn() as usize + 1, 32usize >> p.vl());
        if bits == 16 { return invalid("16-bit UNPACK not expected in PS2_GFX"); }
        let n = p.count() as usize;
        if inline && p.count() == 1 && comps == 4 && bits == 32 { self.header = Some(p.addr() as usize); }
        let d = Buf(p.data);
        for k in 0..n {
            let q = p.addr() as usize + (k / self.wl as usize) * self.cl as usize + k % self.wl as usize;
            if q >= VU1_QWORDS { return invalid(format!("UNPACK writes VU1 qword {q:#x}")); }
            for c in 0..comps {
                self.mem[q][c] = match bits {
                    32 => d.u32(4 * (k * comps + c))?,
                    _ => { let v = d.u8(k * comps + c)?; if p.usn() { v as u32 } else { v as i8 as i32 as u32 } }
                };
            }
            self.gen[q] = self.cur;
        }
        Ok(())
    }

    fn fresh(&self, q: usize) -> Result<[u32; 4]> {
        if q >= VU1_QWORDS || self.gen[q] != self.cur { return invalid(format!("batch reads VU1 qword {q:#x} not written for this batch")); }
        Ok(self.mem[q])
    }

    /// Ends the current batch; returns it, or `None` for a header-only setup kick.
    fn kick(&mut self, texture: Option<u32>, skinned: bool) -> Result<Option<Batch>> {
        let Some(base) = self.header.take() else { return invalid("kick without a GIF tag header") };
        let tag = self.fresh(base)?;
        let lo = tag[0] as u64 | (tag[1] as u64) << 32;
        let (nloop, eop, prim, flg, nreg) = ((lo & 0x7fff) as u16, lo >> 15 & 1 != 0, (lo >> 47 & 0x7ff) as u16, lo >> 58 & 3, lo >> 60);
        let any_vertex = self.gen.get(base + 1).is_some_and(|g| *g == self.cur);
        let out = if !any_vertex {
            None
        } else {
            if flg != 0 || nreg != 3 || tag[2] & 0xfff != 0x412 {
                return invalid(format!("unexpected GIF tag {:08x} {:08x} {:08x} {:08x}", tag[0], tag[1], tag[2], tag[3]));
            }
            let mut verts = Vec::with_capacity(nloop as usize);
            for k in 0..nloop as usize {
                let (xyz, rgba, nw, uv) = (self.fresh(base + 1 + 3 * k)?, self.fresh(base + 2 + 3 * k)?, self.fresh(base + 3 + 3 * k)?, self.fresh(base + UV_OFFSET + k)?);
                let w = nw[3] as i32;
                if !skinned && w != 0 && w != -1 { return invalid(format!("vertex {k}: signed V4-8 w = {w}")); }
                verts.push(Vertex {
                    pos: [f32::from_bits(xyz[0]), f32::from_bits(xyz[1]), f32::from_bits(xyz[2])],
                    uv: [f32::from_bits(uv[0]), f32::from_bits(uv[1])],
                    rgba: [rgba[0] as u8, rgba[1] as u8, rgba[2] as u8, rgba[3] as u8],
                    // Skinned: this slot holds the source's packed skinning qword (normals come from SkinIt at run time).
                    normal: if skinned { [0; 3] } else { [nw[0] as i32 as i8, nw[1] as i32 as i8, nw[2] as i32 as i8] },
                    w,
                    weight: if skinned { f32::from_bits(xyz[3]) } else { 1.0 },
                    slots: if skinned { [(nw[0] & 0x7fff) as u16, (nw[1] & 0x7fff) as u16] } else { [0, 0] },
                });
            }
            // Nothing past the last vertex may be fresh: the unpack counts must equal NLOOP.
            if self.gen.get(base + 1 + 3 * nloop as usize).is_some_and(|g| *g == self.cur) && base + 1 + 3 * (nloop as usize) < base + UV_OFFSET {
                return invalid(format!("more vertex data than NLOOP {nloop}"));
            }
            Some(Batch { texture, prim, nloop, eop, skinned, verts, alpha: self.alpha, test: self.test, zbuf: self.zbuf })
        };
        self.cur += 1;
        Ok(out)
    }
}

/// Replays one leaf packet (offset `at` within the block). `skin_src`: offset of a skinned object's source vertex
/// buffer; REF tags with VIF codes then read `qwc` qwords at `skin_src + addr` (addr signed, may be −16).
pub fn decode_packet(block: &[u8], at: usize, skin_src: Option<usize>) -> Result<Leaf> {
    let b = Buf(block);
    let mut vu = Vu { mem: vec![[0; 4]; VU1_QWORDS], gen: vec![0; VU1_QWORDS], cur: 1, cl: 1, wl: 1, header: None, alpha: 0x44, test: 0x5001b, zbuf: 0x3100_0000 };
    let mut texture = None;
    let (mut batches, mut setup_kicks, mut gs) = (Vec::new(), 0u32, Vec::new());
    let mut p = at;
    for _ in 0..1 << 16 {
        let lo = b.u32(p)? as u64 | (b.u32(p + 4)? as u64) << 32;
        let (qwc, id, addr) = ((lo & 0xffff) as usize, (lo >> 28 & 7) as u8, (lo >> 32) as u32);
        let vif = &block[p + 8..p + 16];
        match id {
            1 => {
                let stream = b.sub(p + 8, 8 + 16 * qwc, "CNT payload")?.bytes();
                for pk in parse_vif(stream)? {
                    if pk.is_unpack() { vu.unpack(&pk, true)?; continue; }
                    match pk.cmd {
                        STCYCL => { vu.cl = (pk.imm & 0xff) as u32; vu.wl = (pk.imm >> 8) as u32; }
                        MSCNT | MSCAL | MSCALF => match vu.kick(texture, skin_src.is_some())? { Some(batch) => batches.push(batch), None => setup_kicks += 1 },
                        // A GIF packet straight to the GS: tag (NLOOP, FLG, NREG, REGS); in PACKED A+D (register 0xE) each
                        // qword is a 64-bit value and a register number.
                        crate::vif::DIRECT | crate::vif::DIRECTHL => {
                            let d = pk.data;
                            if d.len() >= 16 {
                                let tag = u64::from_le_bytes(d[0..8].try_into().unwrap());
                                let regs = u64::from_le_bytes(d[8..16].try_into().unwrap());
                                let (nloop, flg, nreg) = ((tag & 0x7fff) as usize, (tag >> 58) & 3, ((tag >> 60) & 0xf) as usize);
                                if flg == 0 && nreg == 1 && regs & 0xf == 0xe {
                                    for k in 0..nloop {
                                        let o = 16 + 16 * k;
                                        if o + 16 > d.len() { break; }
                                        let reg = d[o + 8];
                                        let val = u64::from_le_bytes(d[o..o + 8].try_into().unwrap());
                                        gs.push((reg, val, batches.len()));
                                        // Track the GS state that affects blend/test/depth so each batch can snapshot it [M G-018].
                                        match reg { 0x42 => vu.alpha = val, 0x47 => vu.test = val, 0x4e => vu.zbuf = val, _ => {} }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                p += 16 + 16 * qwc;
            }
            3 if vif.iter().all(|&x| x == 0) => {
                texture = Some(addr);
                p += 16;
            }
            3 => {
                let Some(src) = skin_src else { return invalid(format!("REF at {p:#x} carries VIF codes but the object has no skin info")) };
                let start = src as i64 + addr as i32 as i64;
                if start < 0 { return invalid(format!("skin REF at {p:#x} reads before the block")); }
                let data = b.sub(start as usize, 16 * qwc, "skin REF data")?.bytes();
                let stream = [vif, data].concat();
                for pk in parse_vif(&stream)? {
                    if pk.is_unpack() { vu.unpack(&pk, false)?; continue; }
                    match pk.cmd {
                        STCYCL => { vu.cl = (pk.imm & 0xff) as u32; vu.wl = (pk.imm >> 8) as u32; }
                        0 => {}
                        c => return invalid(format!("skin REF at {p:#x}: unexpected VIF command {c:#x}")),
                    }
                }
                p += 16;
            }
            6 => return Ok(Leaf { box_index: 0, batches, setup_kicks, gs }),
            _ => return invalid(format!("DMA tag id {id} at {p:#x}")),
        }
    }
    invalid("packet has no RET within 65,536 tags")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(v: &[u32]) -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() }
    fn qw(lo: u64, vif0: u32, vif1: u32) -> Vec<u8> { [lo.to_le_bytes().to_vec(), w(&[vif0, vif1])].concat() }
    fn cnt(qwc: u64, vif0: u32, vif1: u32) -> Vec<u8> { qw(qwc | 1 << 28, vif0, vif1) }

    /// One batch of 3 vertices at base 0x10: header, UV, then stride-3 XYZ / signed / unsigned.
    fn packet(ws: [i8; 3]) -> Vec<u8> {
        let base = 0x10u32;
        let mut s = Vec::new();
        s.extend(w(&[0x0100_0404, 0x6c01_0000 | base, 0x8003, 0x302e_4000, 0x412, 0]));            // STCYCL 4,4; header
        s.extend(w(&[0x6403_0000 | (base + 0xc7)]));                                                // UV ×3
        for k in 0..3 { s.extend(w(&[(k as f32 * 0.5).to_bits(), 0.25f32.to_bits()])); }
        s.extend(w(&[0x0100_0103, 0x6803_0000 | (base + 1)]));                                      // STCYCL 1,3; XYZ ×3
        for k in 0..3 { s.extend(w(&[(k as f32).to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits()])); }
        s.extend(w(&[0x6e03_0000 | (base + 3)]));                                                   // signed V4-8 ×3
        for x in ws { s.extend([0x80u8, 0, 0x7f, x as u8]); }
        s.extend(w(&[0x6e03_4000 | (base + 2)]));                                                   // unsigned V4-8 ×3
        for _ in 0..3 { s.extend([255u8, 255, 255, 0x80]); }
        s.extend(w(&[0x1700_0000]));                                                                // MSCNT
        while (s.len() + 8) % 16 != 0 { s.extend(w(&[0])); }
        let qwc = (s.len() - 8) / 16;
        let mut out = vec![0u8; 16];                                                                // pad: packet at 0x10
        out.extend(qw(4 | 3 << 28 | 7u64 << 32, 0, 0));                                             // texture REF, index 7
        out.extend(cnt(qwc as u64, u32::from_le_bytes(s[0..4].try_into().unwrap()), u32::from_le_bytes(s[4..8].try_into().unwrap())));
        out.extend(&s[8..]);
        out.extend(qw(6 << 28, 0, 0));                                                              // RET
        out
    }

    #[test]
    fn decodes_a_synthetic_batch() {
        let pk = packet([-1, -1, 0]);
        let leaf = decode_packet(&pk, 0x10, None).unwrap();
        assert_eq!(leaf.batches.len(), 1);
        let bt = &leaf.batches[0];
        assert_eq!((bt.texture, bt.nloop, bt.prim, bt.eop), (Some(7), 3, 0x5c, true));
        assert_eq!(bt.verts[2].pos, [2.0, 2.0, 3.0]);
        assert_eq!(bt.verts[1].uv, [0.5, 0.25]);
        assert_eq!((bt.verts[0].normal, bt.verts[0].rgba), ([-128, 0, 127], [255, 255, 255, 0x80]));
        assert_eq!(bt.triangles(), [[0, 1, 2]]);
    }

    #[test]
    fn rejects_bad_kick_flags_and_missing_data() {
        assert!(decode_packet(&packet([-1, 5, 0]), 0x10, None).is_err(), "w must be 0 or -1");
        let mut pk = packet([-1, -1, 0]);
        // Break the UV unpack address: the batch would then read unwritten UV qwords.
        let i = pk.windows(4).position(|x| x == (0x6403_0000u32 | (0x10 + 0xc7)).to_le_bytes()).unwrap();
        pk[i..i + 4].copy_from_slice(&(0x6403_0000u32 | 0x200).to_le_bytes());
        assert!(decode_packet(&pk, 0x10, None).is_err());
    }

    #[test]
    fn strip_triangles_skip_no_kick_vertices_and_alternate_winding() {
        let v = |w| Vertex { pos: [0.0; 3], uv: [0.0; 2], rgba: [0; 4], normal: [0; 3], w, weight: 1.0, slots: [0, 0] };
        let b = Batch { texture: None, prim: 0x5c, nloop: 6, eop: true, skinned: false, verts: vec![v(-1), v(-1), v(0), v(0), v(-1), v(0)], alpha: 0x44, test: 0x5001b, zbuf: 0x3100_0000 };
        assert_eq!(b.triangles(), [[0, 1, 2], [2, 1, 3], [4, 3, 5]]);
    }
}
