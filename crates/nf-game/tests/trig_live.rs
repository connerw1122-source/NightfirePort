//! The port's trig (`trig.rs`) against the game's own `ATAN2_APPROX` (0x1e4528), `PS2Sinf` (0x120700) and
//! `Vec_AngleDifference` (0x1e2b78), run in the R5900 interpreter over the user's ACTION.ELF. Opt-in: `NF_ISO`.

use nf_formats::ee::Machine;
use nf_formats::iso9660::IsoImage;
use nf_game::trig;

#[test]
fn trig_matches_the_game_bit_for_bit() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let mut m = Machine::from_elf(&elf).unwrap();
    let mut state = 0x1234_5678u32;
    let mut next = || { state ^= state << 13; state ^= state >> 17; state ^= state << 5; (state >> 8) as f32 / (1 << 24) as f32 };
    let (mut atan_bad, mut sin_bad, mut diff_bad) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..3000 {
        let (a, b) = ((next() - 0.5) * 200.0, (next() - 0.5) * 200.0);
        m.set_fpr(12, a);
        m.set_fpr(13, b);
        m.call(0x1e4528, 0, 0, 1 << 40).unwrap();
        if m.fpr(0).to_bits() != trig::atan2_approx(a, b).to_bits() { atan_bad.push((a, b, m.fpr(0), trig::atan2_approx(a, b))); }
        let x = (next() - 0.5) * 40.0;
        m.set_fpr(12, x);
        m.call(0x120700, 0, 0, 1 << 40).unwrap();
        if m.fpr(0).to_bits() != trig::sinf(x).to_bits() { sin_bad.push((x, m.fpr(0), trig::sinf(x))); }
        let (p, q) = ((next() - 0.5) * 20.0, (next() - 0.5) * 20.0);
        m.set_fpr(12, p);
        m.set_fpr(13, q);
        m.call(0x1e2b78, 0, 0, 1 << 40).unwrap();
        if m.fpr(0).to_bits() != trig::angle_difference(p, q).to_bits() { diff_bad.push((p, q, m.fpr(0), trig::angle_difference(p, q))); }
    }
    assert!(atan_bad.is_empty(), "ATAN2_APPROX: {} of 3000 differ, e.g. {:?}", atan_bad.len(), &atan_bad[..atan_bad.len().min(4)]);
    assert!(sin_bad.is_empty(), "PS2Sinf: {} of 3000 differ, e.g. {:?}", sin_bad.len(), &sin_bad[..sin_bad.len().min(4)]);
    assert!(diff_bad.is_empty(), "Vec_AngleDifference: {} of 3000 differ, e.g. {:?}", diff_bad.len(), &diff_bad[..diff_bad.len().min(4)]);
}

// exact-sine-sites-2 (G-046 group 2): the missile-steering and head-track/aim-pose trig, golden'd
// against the game's own code run in the R5900 interpreter — the PS2 libm `atanf` (0x235310) and
// `atan2f` (0x232e48 → `__ieee754_atan2f` 0x2339f8; `vecutil_cartesian_to_spherical_acc` 0x1e3a70
// jal's THIS one, not ATAN2_APPROX), `Quat_AxisAngleToQuat` (0x1e3f20), `Quat_QuatToMat` (0x1e3e80)
// and `RotMatrixZYX` (0x1e25a8).

fn machine() -> Option<Machine> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    Some(Machine::from_elf(&elf).unwrap())
}

/// Scratch inside the interpreter's stack region, far below any callee frame ($sp starts at
/// STACK_TOP − 0x100 and frames descend; 0x1ef0000 is 60 KB lower) [M ee.rs STACK_TOP/STACK_SIZE].
const SCRATCH: u32 = 0x01ef_0000;

fn poke_f32(m: &mut Machine, a: u32, v: &[f32]) {
    let mut b = Vec::with_capacity(v.len() * 4);
    for f in v { b.extend_from_slice(&f.to_le_bytes()); }
    m.poke(a, &b).unwrap();
}
fn read_f32(m: &Machine, a: u32, n: usize) -> Vec<f32> {
    m.read(a, n * 4).unwrap().chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect()
}

#[test]
fn libm_atanf_atan2f_match_the_game_bit_for_bit() {
    let Some(mut m) = machine() else { eprintln!("skip: NF_ISO not set"); return };
    let mut state = 0x0bdd_a11eu32;
    let mut next = || { state ^= state << 13; state ^= state >> 17; state ^= state << 5; (state >> 8) as f32 / (1 << 24) as f32 };
    let (mut atanf_bad, mut atan2f_bad, mut pitch_bad) = (Vec::new(), Vec::new(), Vec::new());
    let mut old_site_differs = 0;
    for _ in 0..1500 {
        let x = (next() - 0.5) * 400.0;
        m.set_fpr(12, x);
        m.call(0x235310, 0, 0, 1 << 40).unwrap();
        if m.fpr(0).to_bits() != trig::atanf_libm(x).to_bits() { atanf_bad.push((x, m.fpr(0), trig::atanf_libm(x))); }
        let (y, x2) = ((next() - 0.5) * 300.0, (next() - 0.5) * 300.0);
        m.set_fpr(12, y); m.set_fpr(13, x2);
        m.call(0x232e48, 0, 0, 1 << 40).unwrap();
        if m.fpr(0).to_bits() != trig::atan2f_libm(y, x2).to_bits() { atan2f_bad.push((y, x2, m.fpr(0), trig::atan2f_libm(y, x2))); }
        // The steer site's pitch: vecutil(f12=x, f13=y, f14=z, a0=&out) — atan2f(y, √(x²+z²))
        // [M 0x1e3a70:9; the call shape at Bullet_update 0x12eb50-0x12eb54: f14 in the delay slot].
        let (dx, dy, dz) = (next() - 0.5, next() - 0.5, next() - 0.5);
        let l = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-9);
        let (nx, ny, nz) = (dx / l, dy / l, dz / l);
        m.set_fpr(12, nx); m.set_fpr(13, ny); m.set_fpr(14, nz);
        m.call(0x1e3a70, SCRATCH, 0, 1 << 40).unwrap();
        let out = read_f32(&m, SCRATCH, 2);
        let r2 = trig::ee_add(nf_formats::ps2float::mul(nx, nx), nf_formats::ps2float::mul(nz, nz));
        let port = trig::atan2f_libm(ny, nf_formats::ps2float::sqrt(r2));
        if out[0].to_bits() != port.to_bits() { pitch_bad.push((ny, port, out[0])); }
        // Fail-first proof: the OLD site pitch (asin of the clamped y) misses the game's bits.
        let old = ny.clamp(-0.995, 0.995).asin();
        if old.to_bits() != out[0].to_bits() { old_site_differs += 1; }
    }
    assert!(atanf_bad.is_empty(), "atanf: {} of 1500 differ, e.g. {:?}", atanf_bad.len(), &atanf_bad[..atanf_bad.len().min(4)]);
    assert!(atan2f_bad.is_empty(), "atan2f: {} of 1500 differ, e.g. {:?}", atan2f_bad.len(), &atan2f_bad[..atan2f_bad.len().min(4)]);
    assert!(pitch_bad.is_empty(), "vecutil pitch: {} of 1500 differ, e.g. {:?}", pitch_bad.len(), &pitch_bad[..pitch_bad.len().min(4)]);
    // Sister assertion: the old host-asin site would fail this golden — it misses the game's bits
    // on a large fraction of inputs (asin(y) vs atan2f(y, √(x²+z²)) agree only on some ranges).
    assert!(old_site_differs > 500, "old asin site matched the game on {old_site_differs}/1500 — fail-first premise broken");
}

#[test]
fn quat_pose_functions_match_the_game_bit_for_bit() {
    let Some(mut m) = machine() else { eprintln!("skip: NF_ISO not set"); return };
    let mut state = 0x5eed_cafeu32;
    let mut next = || { state ^= state << 13; state ^= state >> 17; state ^= state << 5; (state >> 8) as f32 / (1 << 24) as f32 };
    let (mut aa2q_bad, mut q2m_bad, mut row_bad) = (Vec::new(), Vec::new(), Vec::new());
    let mut old_row_differs = 0;
    for _ in 0..600 {
        // Axis roughly unit, angle in the head-track/aim range.
        let mut axis = [next() - 0.5, next() - 0.5, next() - 0.5];
        let l = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt().max(1e-9);
        axis = [axis[0] / l, axis[1] / l, axis[2] / l];
        let ang = (next() - 0.5) * 4.0;
        // Quat_AxisAngleToQuat: a0 = &{axis, angle}, a1 = &out (the game calls it in-place).
        poke_f32(&mut m, SCRATCH, &[axis[0], axis[1], axis[2], ang]);
        m.call(0x1e3f20, SCRATCH, SCRATCH + 0x40, 1 << 40).unwrap();
        let q = read_f32(&m, SCRATCH + 0x40, 4);
        let port_q = nf_game::drone::quat_axis_angle_to_quat(axis, ang);
        if q.iter().zip(port_q).any(|(a, b)| a.to_bits() != b.to_bits()) { aa2q_bad.push((ang, q.clone(), port_q)); }
        // Quat_QuatToMat: a0 = &quat, a1 = &matrix (12 floats, rows at 0/4/8).
        poke_f32(&mut m, SCRATCH + 0x80, &q);
        m.call(0x1e3e80, SCRATCH + 0x80, SCRATCH + 0xc0, 1 << 40).unwrap();
        let mat = read_f32(&m, SCRATCH + 0xc0, 12);
        let port_m = nf_game::drone::quat_quat_to_mat(port_q);
        let flat: Vec<f32> = port_m[..3].iter().flat_map(|r| r.iter().copied()).collect();
        for (i, (a, b)) in mat.iter().zip(flat).enumerate() {
            if a.to_bits() != b.to_bits() { q2m_bad.push((i, *a, b)); }
        }
        // RotMatrixZYX's dir row (m8, m9, m10) vs a transcription of its decompile formula — the
        // game's row with rz = 0, rx = −p2, ry = y2 carries the game's own sinf(π/2) = 0x3f800001
        // factor ([M 0x1e25a8:32-40]: m8 = (sin(rz+π/2)·sin ry)·cos rx + sin rz·sin rx, m9 =
        // (sin rz·sin ry)·cos rx − sin(rz+π/2)·sin rx, m10 = cos ry·cos rx, every product an EE
        // mul and every sum a no-guard EE add).
        let (p2, y2) = ((next() - 0.5) * 4.0, (next() - 0.5) * 6.0);
        poke_f32(&mut m, SCRATCH + 0x100, &[-p2, y2, 0.0]);
        m.call(0x1e25a8, SCRATCH + 0x100, SCRATCH + 0x140, 1 << 40).unwrap();
        let row = read_f32(&m, SCRATCH + 0x140, 12);
        let mul = nf_formats::ps2float::mul;
        let c = trig::sinf(1.5707964); // the game's sinf(π/2) = 0x3f800001, not 1.0
        let sy = trig::sinf(y2);
        let cy = trig::cosf(y2);
        let sn_rx = trig::sinf(-p2);
        let cp = trig::cosf(-p2); // fStack_60 = sinf(rx + π/2)
        let game_row = [
            trig::ee_add(mul(mul(c, sy), cp), mul(0.0f32, sn_rx)),
            trig::ee_sub(mul(mul(0.0f32, sy), cp), mul(c, sn_rx)),
            mul(cy, cp),
        ];
        for (i, k) in [8usize, 9, 10].into_iter().enumerate() {
            if row[k].to_bits() != game_row[i].to_bits() { row_bad.push((k, row[k], game_row[i])); }
        }
        // Fail-first proof: the OLD host sin_cos rebuild (no game sine, no sinf(π/2) factor) misses
        // the game's row bits.
        let (osp, ocp) = p2.sin_cos();
        let (osy, ocy) = y2.sin_cos();
        let old = [ocp * osy, osp, ocp * ocy];
        if old.iter().zip([row[8], row[9], row[10]]).any(|(a, b)| a.to_bits() != b.to_bits()) { old_row_differs += 1; }
    }
    assert!(aa2q_bad.is_empty(), "AA2Q: {} of 600 differ, e.g. {:?}", aa2q_bad.len(), &aa2q_bad[..aa2q_bad.len().min(4)]);
    assert!(q2m_bad.is_empty(), "QuatToMat: {} elements differ, e.g. {:?}", q2m_bad.len(), &q2m_bad[..q2m_bad.len().min(4)]);
    assert!(row_bad.is_empty(), "RotMatrixZYX dir row: {} of 1800 differ, e.g. {:?}", row_bad.len(), &row_bad[..row_bad.len().min(4)]);
    // Sister assertion: the old host-trig rebuild would fail this golden.
    assert!(old_row_differs > 300, "old host sin_cos matched the game on {old_row_differs}/600 — fail-first premise broken");
}
#[test]
fn probe_sinf3_lanes_vs_sinf() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let mut m = nf_formats::ee::Machine::from_elf(&elf).unwrap();
    let mut state = 0x1234_5678u32;
    let mut next = || { state ^= state << 13; state ^= state >> 17; state ^= state << 5; (state >> 8) as f32 / (1 << 24) as f32 };
    let mut bad = 0;
    for _ in 0..300 {
        let (a, b, c) = ((next() - 0.5) * 8.0, (next() - 0.5) * 8.0, (next() - 0.5) * 8.0);
        m.set_fpr(12, a); m.set_fpr(13, b); m.set_fpr(14, c);
        m.set_gpr(6, 0x01ef_0020);
        m.call(0x11fc20, 0x01ef_0000, 0x01ef_0010, 1 << 40).unwrap();
        let read = |m: &nf_formats::ee::Machine, addr: u32| {
            m.read(addr, 4).unwrap().chunks_exact(4).map(|ch| f32::from_le_bytes(ch.try_into().unwrap())).collect::<Vec<_>>()
        };
        let outs = [read(&m, 0x01ef_0000).remove(0), read(&m, 0x01ef_0010).remove(0), read(&m, 0x01ef_0020).remove(0)];
        for (i, (inp, got)) in [(0, (a, outs[0])), (1, (b, outs[1])), (2, (c, outs[2]))] {
            let want = nf_game::trig::sinf(inp);
            if got.to_bits() != want.to_bits() {
                bad += 1;
                if bad < 6 { eprintln!("lane {i}: sinf3({inp}) = {:08x} vs sinf {:08x}", got.to_bits(), want.to_bits()); }
            }
        }
    }
    eprintln!("probe: {bad} of 900 lane results differ");
}
