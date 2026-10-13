//! Animation sequences (type 4, prefix 04) and skeletal posing / skinning. Spec: docs/formats/animation.md.
//!
//! Channel decoding mirrors `AnimFrameCopy` (ACTION.ELF 0x1c5668); the quaternion → matrix formula is
//! `Quat_QuatTransToMat` (0x1e3fa8); bone composition follows `psiBuildMatrixPalette` (0x111498). Matrices are
//! row-vector (`v' = v·M`, translation in row 3), as everywhere in the game. `sinf` here is the game's own
//! `PS2Sinf` (0x120700) — every matrix code that needs a sine takes it from this module (GAPS G-017/G-024).

use crate::buf::{invalid, Buf, Result};
use crate::character::{Skeleton, Skin};

pub type Mat4 = [[f32; 4]; 4];

/// Float evaluation: the game's order with round-toward-zero (EE FPU / VU0 as PCSX2 runs them), or IEEE nearest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arith { Ps2, Nearest }

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
    /// reaches `t = frame − 1`, evaluated at absolute `t` in the game's order and rounding (`Arith::Ps2`).
    pub fn value(&self, channel: usize, frame: u32) -> f32 { self.value_with(channel, frame, Arith::Ps2) }

    pub fn value_with(&self, channel: usize, frame: u32, arith: Arith) -> f32 {
        let t = frame.saturating_sub(1);
        let segs = &self.channels[channel];
        let s = segs.iter().find(|s| t <= s.end).unwrap_or(segs.last().unwrap());
        let tf = t as f32;
        match arith {
            Arith::Nearest => {
                let mut p = 1.0f32;
                let mut v = 0.0f32;
                for c in s.c { v += c * p; p *= tf; }
                v
            }
            Arith::Ps2 => {
                // FPU: t, t², …, t⁵ by repeated multiplication (DAT_0031aa04…14); VU0: ACC = c0·1, ACC += ck·tᵏ.
                use crate::ps2float::{add, mul};
                let mut pw = [1.0f32; 6];
                pw[1] = tf;
                for k in 2..6 { pw[k] = mul(pw[k - 1], tf); }
                let mut acc = mul(s.c[0], 1.0);
                for k in 1..6 { acc = add(acc, mul(s.c[k], pw[k])); }
                acc
            }
        }
    }
}

/// Animation script (archive entry type 5, prefix 06): what `AnimScriptInit` (0x1c1600) and `AnimProcessScriptData`
/// (0x1bd3d8) read. `+4` u16 length in frames; `+6` u16: low 9 bits = command count, bit 15 = loop (script flag
/// 0x80000000, which `AnimScriptEnd` turns into wrap-around); commands from `+8`.
#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    pub hashcode: u32,
    pub length: u16,
    pub looping: bool,
    pub commands: Vec<ScriptCommand>,
}

/// One command: opcode, then its u16 arguments (count = byte 1, + 3 for opcode 0, + 2 for opcodes 1–4).
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptCommand { pub op: u8, pub args: Vec<u16> }

impl Script {
    pub fn parse(data: &[u8]) -> Result<Script> {
        let b = Buf(data);
        let word = b.u16(6)?;
        let mut p = 8usize;
        let mut commands = Vec::new();
        for _ in 0..word & 0x1ff {
            let (op, n) = (b.u8(p)?, b.u8(p + 1)? as usize);
            let units = n + match op { 0 => 3, 1..=4 => 2, _ => 0 };
            let args = (0..units).map(|k| b.u16(p + 2 + 2 * k)).collect::<Result<_>>()?;
            commands.push(ScriptCommand { op, args });
            p += 2 * (units + 1);
        }
        Ok(Script { hashcode: b.u32(0)?, length: b.u16(4)?, looping: word & 0x8000 != 0, commands })
    }

    /// Opcode-0 commands: `(first script frame, last script frame, sequence hashcode)` — `AnimScriptPeekSeq` takes the
    /// sequence of the first one (`0x04000000 | u16 @ +0xE`).
    pub fn sequences(&self) -> impl Iterator<Item = (u16, u16, u32)> + '_ {
        self.commands.iter().filter(|c| c.op == 0).map(|c| (c.args[0], c.args[1], 0x0400_0000 | c.args[2] as u32))
    }

    /// Script time after `ticks` 60 Hz updates at speed 1 (`AnimScriptTick` + `AnimScriptEnd`): time starts at 1 and
    /// advances 1 per tick; a looping script subtracts its length once past it (clamped to ≥ 1), others hold the end.
    pub fn time_at(&self, ticks: f32) -> f32 {
        let len = self.length.max(1) as f32;
        let t = 1.0 + ticks.max(0.0);
        if t <= len { t } else if self.looping { ((t - 1.0) % len + 1.0).max(1.0) } else { len }
    }
}

/// `PS2Sinf` (0x120700): range reduction to [−π/2, π/2] on the FPU (no guard bit), then the VU0 polynomial in
/// the game's operation order (coefficients at 0x2f3b20; `PS2Sinf2` 0x11f968 / `PS2Sinf3` 0x11fc20 run the same
/// polynomial on 2/3 lanes). Bit-exact: 3000 interpreter samples over ACTION.ELF (WORKLOG 0026) and the 40 calls
/// recorded from the running game (`crates/nf-game/tests/diff_calls.rs`).
pub fn sinf(mut x: f32) -> f32 {
    use crate::ps2float as pf;
    const S1: f32 = f32::from_bits(0xbe2a_aaa4);
    const S2: f32 = f32::from_bits(0x3c08_873e);
    const S3: f32 = f32::from_bits(0xb94f_b21f);
    const S4: f32 = f32::from_bits(0x362e_9c14);
    let (pi, tau, half) = (f32::from_bits(0x4049_0fdb), f32::from_bits(0x40c9_0fdb), f32::from_bits(0x3fc9_0fdb));
    let fadd = |a, b| pf::add_noguard(a, b, true);
    let fsub = |a: f32, b: f32| pf::add_noguard(a, -b, true);
    if x >= pi { while x >= pi { x = fsub(x, tau); } } else { while x < -pi { x = fadd(x, tau); } }
    if x > half { x = fsub(pi, x); } else if x < -half { x = fsub(-pi, x); }
    let x2 = pf::mul(x, x);
    let a = pf::mul(pf::mul(x, S3), x2);
    let b = pf::mul(x, S1);
    let c = pf::mul(x, S2);
    let x4 = pf::mul(x2, x2);
    let d = pf::mul(x, S4);
    let x8 = pf::mul(x4, x4);
    // ACC = b·x²; ACC += a·x⁴; ACC += c·x⁴; ACC += x·1; result = ACC + d·x⁸ (VU0: products and sums chopped,
    // the VU's adds keep their guard bit unlike the FPU's).
    let mut acc = pf::mul(b, x2);
    acc = pf::add(acc, pf::mul(a, x4));
    acc = pf::add(acc, pf::mul(c, x4));
    acc = pf::add(acc, x);
    pf::add(acc, pf::mul(d, x8))
}

/// `Quat_QuatTransToMat` (0x1e3fa8; `Quat_QuatToMat` 0x1e3e80 is the same 3×3 without the translation):
/// (x, y, z, w) and a translation → row-vector matrix, in the decompile's operation order with EE FPU rounding —
/// doubling adds, chopped products, no-guard sums, `1.0 −` subtracts. Bit-exact on the recorded calls
/// (`Quat_QuatToMat` 40/40 in crates/nf-game/tests/diff_calls.rs).
pub fn quat_trans_to_mat(q: [f32; 4], t: [f32; 3]) -> Mat4 {
    use crate::ps2float as pf;
    let [x, y, z, w] = q;
    let fadd = |a, b| pf::add_noguard(a, b, true);
    let fsub = |a: f32, b: f32| pf::add_noguard(a, -b, true);
    let m = pf::mul;
    let (y2, z2, x2) = (fadd(y, y), fadd(z, z), fadd(x, x));
    let (xx, xw) = (m(x2, x), m(x2, w));
    let (zy, zz) = (m(y2, y), m(z2, z));
    [
        [fsub(1.0, fadd(zy, zz)), fadd(m(y2, x), m(z2, w)), fsub(m(z2, x), m(y2, w)), 0.0],
        [fsub(m(y2, x), m(z2, w)), fsub(1.0, fadd(xx, zz)), fadd(m(z2, y), xw), 0.0],
        [fadd(m(z2, x), m(y2, w)), fsub(m(z2, y), xw), fsub(1.0, fadd(xx, zy)), 0.0],
        [t[0], t[1], t[2], 1.0],
    ]
}

/// Matrix composition as `psiBuildMatrixPalette` (0x111498) runs it on the VU0: the products truncate, the
/// running sum keeps its guard bit (`ACC = a₀·b₀; ACC += a₁·b₁; …`). Measured against the EE snapshots by
/// `nf-extract pose-check` (docs/formats/animation.md "Arithmetic").
pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    use crate::ps2float::{add, mul};
    let mut r = [[0f32; 4]; 4];
    for i in 0..4 { for j in 0..4 {
        let mut acc = mul(a[i][0], b[0][j]);
        for k in 1..4 { acc = add(acc, mul(a[i][k], b[k][j])); }
        r[i][j] = acc;
    } }
    r
}

pub fn transform(m: &Mat4, v: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|c| v[0] * m[0][c] + v[1] * m[1][c] + v[2] * m[2][c] + m[3][c])
}

/// Local (translation, quaternion) per bone at an integer frame: per bone tx, ty, tz (only if the skeleton mask bit
/// is set; × skin scale) then qx, qy, qz with w rebuilt from the qz ≥ 2 sign trick. Bones without translation
/// channels use the skeleton offset × skin scale.
/// Whether `seq` has a channel for every value `local_pose` reads on `skel` (3 per translated bone, 3 per rotation);
/// some sequences on a skeleton id carry fewer.
pub fn fits(seq: &Sequence, skel: &Skeleton) -> bool {
    let need: usize = (0..skel.bones as usize).map(|b| if skel.has_translation(b) { 6 } else { 3 }).sum();
    seq.skeleton == skel.id as u8 && seq.channels.len() >= need
}

pub fn local_pose(seq: &Sequence, skel: &Skeleton, skin: &Skin, frame: u32) -> Vec<([f32; 3], [f32; 4])> {
    local_pose_with(seq, skel, skin, frame, Arith::Ps2)
}

pub fn local_pose_with(seq: &Sequence, skel: &Skeleton, skin: &Skin, frame: u32, arith: Arith) -> Vec<([f32; 3], [f32; 4])> {
    use crate::ps2float as pf;
    // Measured against PCSX2 snapshots (WORKLOG 0017): VU0 squares round toward zero; the FPU sums add without guard
    // bits and round toward zero; the square root rounds to nearest. With this every component of 102 bone poses in
    // three drones is bit-identical to the game's.
    let (mul, add, sub, sqrt): (fn(f32, f32) -> f32, fn(f32, f32) -> f32, fn(f32, f32) -> f32, fn(f32) -> f32) = match arith {
        Arith::Ps2 => (pf::mul, |a, b| pf::add_noguard(a, b, true), |a, b| pf::add_noguard(a, -b, true), f32::sqrt),
        Arith::Nearest => (|a, b| a * b, |a, b| a + b, |a, b| a - b, f32::sqrt),
    };
    let mut ch = 0;
    let mut out = Vec::with_capacity(skel.bones as usize);
    for bone in 0..skel.bones as usize {
        let t = if skel.has_translation(bone) {
            let t = [0, 1, 2].map(|k| mul(seq.value_with(ch + k, frame, arith), skin.scale[k]));
            ch += 3;
            t
        } else {
            [0, 1, 2].map(|k| mul(skel.offsets[bone][k], skin.scale[k]))
        };
        let v = |k: usize| seq.value_with(ch + k, frame, arith);
        let (x, y, mut z) = (v(0), v(1), v(2));
        ch += 3;
        let negative = z >= 2.0;
        if negative { z = sub(z, 4.0); }
        // AnimFrameCopy: VU squares, then FPU (x² + y²) + z², 1 − sum, sqrt.
        let ww = sub(1.0, add(add(mul(x, x), mul(y, y)), mul(z, z)));
        let w = if ww > 0.0 { if negative { -sqrt(ww) } else { sqrt(ww) } } else { 0.0 };
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
    fn script_commands_and_loop_time() {
        // Header: hash, length 130, word 0x8002 (loop, 2 commands); op 0 (3 args: frames 1..130, sequence 0x0133),
        // then op 4 with one extra argument (1 + 2 = 3 args).
        let words: [u16; 12] = [0x01bb, 0x0600, 130, 0x8002, 0x0000, 1, 130, 0x0133, 0x0104, 1, 2, 0x10];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let s = Script::parse(&bytes).unwrap();
        assert_eq!((s.hashcode, s.length, s.looping, s.commands.len()), (0x0600_01bb, 130, true, 2));
        assert_eq!(s.commands[1], ScriptCommand { op: 4, args: vec![1, 2, 0x10] });
        assert_eq!(s.sequences().collect::<Vec<_>>(), vec![(1, 130, 0x0400_0133)]);
        // AnimScriptEnd: 1, 2, …, 130, then 131 − 130 = 1.
        assert_eq!((s.time_at(0.0), s.time_at(129.0), s.time_at(130.0), s.time_at(131.0)), (1.0, 130.0, 1.0, 2.0));
        let once = Script { looping: false, ..s.clone() };
        assert_eq!(once.time_at(500.0), 130.0);
        // A command list longer than the data is an error, not a short read.
        assert!(Script::parse(&bytes[..20]).is_err());
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
