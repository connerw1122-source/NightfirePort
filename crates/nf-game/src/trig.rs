//! The game's angle maths (ACTION.ELF), ported operation by operation. Rounding: EE FPU / VU0 chop
//! (`nf_formats::ps2float`); constants as their f32 bit patterns. Spec: docs/systems/drone-ai.md "Maths".

use nf_formats::ps2float::{add_noguard, mul};

/// EE FPU add / sub: no guard bit, rounded toward zero (`ps2float::add_noguard`; the model validated on poses and on
/// live `ATAN2_APPROX` results, WORKLOG 0026).
fn add(a: f32, b: f32) -> f32 { add_noguard(a, b, true) }
fn sub(a: f32, b: f32) -> f32 { add_noguard(a, -b, true) }

pub const PI: f32 = f32::from_bits(0x4049_0fdb);
pub const TAU: f32 = f32::from_bits(0x40c9_0fdb);
pub const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);

/// `ATAN2_APPROX(a, b)` (0x1e4528): an angle whose tangent is a / b (called with (dx, dz) → yaw).
pub fn atan2_approx(a: f32, b: f32) -> f32 {
    const C1: f32 = f32::from_bits(0x3f87_a38e);
    const C2: f32 = f32::from_bits(0x3e8a_e988);
    let mut r = if a == b {
        if b == 0.0 { 0.0 } else { f32::from_bits(0x3f49_0fdb) }
    } else {
        let (aa, bb) = (b.abs(), a.abs());
        if bb < aa {
            let t = bb / aa;
            mul(sub(C1, mul(t, C2)), t)
        } else {
            let t = aa / bb;
            sub(HALF_PI, mul(sub(C1, mul(t, C2)), t))
        }
    };
    if b < 0.0 { r = sub(PI, r); }
    if a < 0.0 { r = -r; }
    r
}

/// `PS2Sinf` (0x120700): the canonical port lives in `nf_formats::anim::sinf` (range reduction to [−π/2, π/2],
/// then the VU0 polynomial in the game's operation order) so the whole crate shares one implementation; it is
/// bit-exact on the 3000 interpreter samples (WORKLOG 0026) and on the 40 calls recorded from the running game
/// (`tests/diff_calls.rs`).
pub fn sinf(x: f32) -> f32 { nf_formats::anim::sinf(x) }

/// Cosine as the game takes it: `PS2Sinf(θ + 1.5707964)`.
pub fn cosf(x: f32) -> f32 { sinf(add(x, f32::from_bits(0x3fc9_0fdb))) }

/// EE FPU add / sub as shared helpers for sites that replay the game's float code outside this
/// module (the same `add_noguard(…, true)` model this file's private `add`/`sub` use).
pub fn ee_add(a: f32, b: f32) -> f32 { add(a, b) }
pub fn ee_sub(a: f32, b: f32) -> f32 { sub(a, b) }

/// The PS2 libm's `atanf` (ACTION.ELF 0x235310, fdlibm s_atanf), ported operation for operation:
/// every mul/div is an EE product chopped toward zero and every add/sub loses its guard bits
/// (`ps2float`), constants as their exact f32 bit patterns ([L ee_snapshot.bin 0x30ba08 atanhi,
/// 0x30ba18 atanlo, 0x30ba28 the interleaved T/C poly table, 0x30ba58 huge]).
pub fn atanf_libm(x: f32) -> f32 {
    use nf_formats::ps2float::{div as ee_div, mul};
    const ATANHI: [f32; 4] = [
        f32::from_bits(0x3eed_6338), // atan(0.5)hi
        f32::from_bits(0x3f49_0fda), // atan(1.0)hi
        f32::from_bits(0x3f7b_985e), // atan(1.5)hi
        f32::from_bits(0x3fc9_0fda), // atan(inf)hi
    ];
    const ATANLO: [f32; 4] = [
        f32::from_bits(0x31ac_3769),
        f32::from_bits(0x3322_2168),
        f32::from_bits(0x3314_0fb4),
        f32::from_bits(0x33a2_2168),
    ];
    // The game stores the two Horner chains interleaved: T0, C0, T1, C1, …, T5 ([L 0x30ba28-0x30ba50]).
    const T: [f32; 6] = [
        f32::from_bits(0x3eaa_aaab),
        f32::from_bits(0x3e12_4925),
        f32::from_bits(0x3dba_2e6e),
        f32::from_bits(0x3d88_6b35),
        f32::from_bits(0x3d4b_da59),
        f32::from_bits(0x3c85_69d7),
    ];
    const C: [f32; 5] = [
        f32::from_bits(0xbe4c_cccd),
        f32::from_bits(0xbde3_8e38),
        f32::from_bits(0xbd9d_8795),
        f32::from_bits(0xbd6e_f16b),
        f32::from_bits(0xbd15_a221),
    ];
    const HUGE: f32 = f32::from_bits(0x7149_f2ca); // 1e30 [L 0x30ba58]
    let ix = x.to_bits() & 0x7fff_ffff;
    // |x| ≥ 2^34: atanf(x) = π/2 (atanhi[3] + atanlo[3]); NaN → x + x [M 0x235310:14-22].
    if ix >= 0x5080_0000 {
        if ix > 0x7f80_0000 { return add(x, x); }
        return if x > 0.0 { add(ATANHI[3], ATANLO[3]) } else { sub(-ATANHI[3], ATANLO[3]) };
    }
    // The tiny-|x| early-out: huge + x > 1.0 holds for every |x| < 2^-126, so it returns x [M :26-29].
    if ix < 0x3ee0_0000 {
        if ix < 0x0080_0000 && HUGE + x > 1.0 { return x; }
        let z = mul(x, x);
        let p = poly(z, &T, &C);
        return sub(x, mul(x, p)); // id = −1: x − x·(p1 + p2) [M :68]
    }
    let mut id;
    let mut w = x.abs();
    if ix < 0x3f98_0000 {
        if ix < 0x3f30_0000 {
            id = 0;
            w = ee_div(sub(add(w, w), 1.0), add(w, 2.0)); // (2|x| − 1)/(|x| + 2) [M :39]
        } else {
            id = 1;
            w = ee_div(sub(w, 1.0), add(w, 1.0)); // (|x| − 1)/(|x| + 1) [M :43]
        }
    } else if ix < 0x401c_0000 {
        id = 2;
        w = ee_div(sub(w, 1.5), add(mul(w, 1.5), 1.0)); // (|x| − 1.5)/(1.5|x| + 1) [M :49]
    } else {
        id = 3;
        w = ee_div(-1.0, w); // −1/|x| [M :53]
    }
    let z = mul(w, w);
    let p = poly(z, &T, &C);
    // atanhi[id] − ((w·(p1 + p2) − atanlo[id]) − w), negated for x < 0 [M :71-75].
    let r = sub(ATANHI[id as usize], sub(sub(mul(w, p), ATANLO[id as usize]), w));
    if x < 0.0 { -r } else { r }
}

/// The two Horner chains of `atanf` (0x235310:59-66), in the game's evaluation order with EE ops:
/// p1 = z·(T0 + s·(T1 + … + s·T5)), p2 = s·(C0 + s·(C1 + … + s·C4)) — z = w², s = z² — returned as
/// p1 + p2, so the caller's w·(p1 + p2) is the w³/3 + w⁵/5 − … of the atan series.
fn poly(z: f32, t: &[f32; 6], c: &[f32; 5]) -> f32 {
    use nf_formats::ps2float::mul;
    let s = mul(z, z);
    let mut h = mul(s, t[5]);
    h = add(t[4], h); h = mul(s, h);
    h = add(t[3], h); h = mul(s, h);
    h = add(t[2], h); h = mul(s, h);
    h = add(t[1], h); h = mul(s, h);
    h = add(t[0], h);
    let p1 = mul(z, h); // fVar6 = fVar6 * (aT + …) — the game multiplies by z [M 0x235310:63]
    let mut g = mul(s, c[4]);
    g = add(c[3], g); g = mul(s, g);
    g = add(c[2], g); g = mul(s, g);
    g = add(c[1], g); g = mul(s, g);
    g = add(c[0], g);
    let p2 = mul(s, g);
    add(p1, p2)
}

/// The PS2 libm's `atan2f` core (ACTION.ELF `__ieee754_atan2f` 0x2339f8, fdlibm e_atan2f; the
/// 0x232e48 wrapper only routes (0,0) through matherr/errno and returns the core's value), ported
/// operation for operation on the EE FPU. `vecutil_cartesian_to_spherical_acc` 0x1e3a70 calls this
/// one (jal 0x232e48), NOT `ATAN2_APPROX` 0x1e4528 [M 0x1e3aa0/0x1e3ab0].
pub fn atan2f_libm(y: f32, x: f32) -> f32 {
    use nf_formats::ps2float::div as ee_div;
    const PI_DOWN: f32 = f32::from_bits(0x4049_0fda); // the π the sign adjustments subtract [M 0x233bb8]
    const PIO2_UP: f32 = f32::from_bits(0x3fc9_0fdc); // the |y|≫|x| cap [M 0x233bf8]
    const ATANLO0: f32 = f32::from_bits(0x3422_2168); // 1.50995788e-07 [L ee_snapshot.bin 0x30b530]
    const NEG_ZERO: f32 = f32::from_bits(0x8000_0000); // [L ee_snapshot.bin 0x30b534]
    let iy = y.to_bits() & 0x7fff_ffff;
    let ix = x.to_bits() & 0x7fff_ffff;
    // m = sign(y) | sign(x) << 1 [M 0x2339f8:21]
    let m = ((y.to_bits() >> 31) & 1) | (((x.to_bits() >> 31) & 1) << 1);
    if ix > 0x7f80_0000 || iy > 0x7f80_0000 { return add(x, y); } // NaN [M :14-16]
    if x == 1.0 { return atanf_libm(y); } // [M :17-20]
    if iy == 0 {
        // y = ±0: ±0 for x > 0, ±π for x < 0 [M :22-34]
        return match m {
            2 => PI_DOWN,
            3 => -PI_DOWN,
            _ => y,
        };
    }
    if ix != 0 {
        if ix == 0x7f80_0000 {
            // x = ±Inf [M :36-73]
            return match m {
                0 => 0.0,
                1 => NEG_ZERO,
                2 => PI_DOWN,
                3 => -PI_DOWN,
                _ => unreachable!(),
            };
        }
        if iy != 0x7f80_0000 {
            // Both finite: the atanf(|y/x|) core with the fdlibm exponent-difference gates [M :74-96].
            let z = {
                let k = ((iy as i32).wrapping_sub(ix as i32)) >> 23;
                if k < 0x3d {
                    if x.to_bits() & 0x8000_0000 == 0 || k > -0x3d {
                        atanf_libm(ee_div(y.abs(), x.abs()))
                    } else {
                        0.0
                    }
                } else {
                    PIO2_UP
                }
            };
            return match m {
                0 => z,
                1 => f32::from_bits(z.to_bits() ^ 0x8000_0000),
                2 => sub(PI_DOWN, sub(z, ATANLO0)),
                _ => sub(sub(z, ATANLO0), PI_DOWN),
            };
        }
    }
    // x = ±0 or y = ±Inf: ±π/2 (nearest) by the sign of y [M :35-34, 98-103]
    if y < 0.0 { f32::from_bits(0xbfc9_0fdb) } else { f32::from_bits(0x3fc9_0fdb) }
}

/// `Vec_AngleDifference(a, b)` (0x1e2b78): b − a wrapped to (−π, π], both first normalised to [0, 2π).
pub fn angle_difference(a: f32, b: f32) -> f32 {
    // Only values outside [0, 2π) go through `fmodf` (libm, exact like Rust's `%`); the adds are EE FPU adds.
    let norm = |v: f32| if v < 0.0 || v >= TAU { let r = v % TAU; if r < 0.0 { add(r, TAU) } else { r } } else { v };
    let mut d = sub(norm(b), norm(a));
    if d.abs() >= PI { d = if d >= 0.0 { sub(d, TAU) } else { add(d, TAU) } }
    d
}

/// `Ang_RotateToBearing(cur, target, speed, max)` (0x1e0f10): cur + clamp(wrap(target − cur), ±max) · speed,
/// snapping to the target within 1e-4.
pub fn rotate_to_bearing(cur: f32, target: f32, speed: f32, max: f32) -> f32 {
    let d = angle_difference(cur, target).clamp(-max, max);
    if d.abs() < 1e-4 { target } else { cur + d * speed }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live `ATAN2_APPROX` results (WORKLOG 0026): (dx, dz) = `Drone+0x1B0` / `+0x1B8`, result `+0x1C4`, read by PINE
    /// from 07000005 with Bond teleported to random spots; 60 of the 739 consistent samples (bit patterns).
    const LIVE_ATAN: [(u32, u32, u32); 60] = [
        (0x4223ccaa, 0x419ec900, 0x3f8f7a2a),
        (0xc2d6a1ac, 0xc063d5c0, 0xbfcd85f0),
        (0x41c65e1e, 0xc1c375a4, 0x40167f5d),
        (0x3f9da0a0, 0x41f1c5d4, 0x3d2f04bb),
        (0x41dba71c, 0xc1c694a4, 0x4013a6e9),
        (0xbf5ccb00, 0x420f2230, 0xbccff0d6),
        (0xc29431d1, 0xc1dc4418, 0xbff6aafe),
        (0x411a7c48, 0x41cfe4f0, 0x3eb668b0),
        (0xc257eebd, 0x4211b74c, 0xbf7ab060),
        (0xc1a3760c, 0xc20f069b, 0xc027fa33),
        (0xc18a0190, 0x420a3712, 0xbeec3e93),
        (0xc134cdec, 0x417b41b8, 0xbf1f3ed7),
        (0x4298c8da, 0xc0d11130, 0x3fd46884),
        (0xc227bda7, 0xc20d4af4, 0xc011564d),
        (0x4199fc8e, 0xc1a40fa8, 0x4018b43d),
        (0xc25872f4, 0x41fda73c, 0xbf85822e),
        (0x4265d78e, 0xc1f7b670, 0x400408f2),
        (0xc281cd36, 0x421c897e, 0xbf83e683),
        (0x424b3261, 0xc0bbf260, 0x3fd847b4),
        (0x42087c64, 0xc0d82f00, 0x3fe28e4b),
        (0xc1bc6674, 0x41aa6c84, 0xbf559076),
        (0x41d9d3ee, 0x41b0aea0, 0x3f63c876),
        (0xc25099d0, 0xc21f887a, 0xc00e3de5),
        (0x426e4730, 0x40f371e0, 0x3fb84e4c),
        (0x428dc9e7, 0x41a9d3d8, 0x3fa38f74),
        (0xc22c42e5, 0x41cace78, 0xbf854033),
        (0x4212fafa, 0x415bffa0, 0x3f9b2b43),
        (0xc218b30a, 0xc21a665e, 0xc016f91c),
        (0x42399499, 0x40d3e270, 0x3fb66948),
        (0x4031e000, 0xc1f8e8ca, 0x40432472),
        (0x3fd84ba0, 0x41e07278, 0x3d80b266),
        (0xc14fd0b0, 0x41735e20, 0xbf3500b2),
        (0x42109740, 0x41bf3960, 0x3f7d1c0a),
        (0x4294c3ef, 0x4217c7b8, 0x3f8ce7d4),
        (0xc2438f77, 0x406becc0, 0xbfbf083c),
        (0x40f9f4e8, 0x421934d4, 0x3e51bd89),
        (0xc2a877b8, 0x4216acc2, 0xbf9359a9),
        (0x41d89f04, 0xc1aa4450, 0x400f1c30),
        (0xc17b8300, 0x410a81e8, 0xbf88e5ca),
        (0xc203a6ba, 0xc1e99414, 0xc0130788),
        (0xc118504c, 0x41aab208, 0xbed669cf),
        (0xc264eb0e, 0xc06a6b20, 0xbfd199ce),
        (0x40490c00, 0xc0f7b9a0, 0x403066a4),
        (0x4255f558, 0x41845d4c, 0x3fa26db4),
        (0x41cc11d8, 0xc1cee364, 0x40170f81),
        (0x4272605a, 0x421b1330, 0x3f807eb5),
        (0x412f8210, 0xc1ff3884, 0x4033cbc7),
        (0xc170fab4, 0x41bd6648, 0xbf1077e9),
        (0x4278663b, 0xc1d524bc, 0x3ffcdcfc),
        (0x422d29f5, 0xbf8acbc0, 0x3fcc7007),
        (0xc1ae168c, 0xc20f459e, 0xc0264493),
        (0x421abeb7, 0xbea87600, 0x3fca3689),
        (0xc2be35f3, 0x411b2ac8, 0xbfbb9785),
        (0x4256e352, 0x3feea6c0, 0x3fc46575),
        (0xc2b8918a, 0x41820824, 0xbfb23fcf),
        (0xc11a1930, 0x4055f4c0, 0xbf9e2a23),
        (0xc14e296c, 0xc1b1a9ac, 0xc0278ee1),
        (0x42965150, 0x41107fc8, 0x3fb943c9),
        (0x418389da, 0xc05adec0, 0x3fe3c56e),
        (0xc182ba9c, 0x41d4622c, 0xbf0caa2c),
    ];

    #[test]
    fn atan2_matches_live_results() {
        for &(a, b, g) in &LIVE_ATAN {
            let r = atan2_approx(f32::from_bits(a), f32::from_bits(b));
            assert_eq!(r.to_bits(), g, "ATAN2_APPROX({}, {})", f32::from_bits(a), f32::from_bits(b));
        }
        // Sister: chopped division (the EE interpreter's first guess) disagrees with some samples.
        let chopped = |a: f32, b: f32| {
            let (aa, bb) = (b.abs(), a.abs());
            let q = |x: f32, y: f32| { let n = x / y; if (n as f64).abs() > (x as f64 / y as f64).abs() { f32::from_bits(n.to_bits() - 1) } else { n } };
            const C1: f32 = f32::from_bits(0x3f87_a38e);
            const C2: f32 = f32::from_bits(0x3e8a_e988);
            let mut r = if bb < aa { let t = q(bb, aa); mul(sub(C1, mul(t, C2)), t) } else { let t = q(aa, bb); sub(HALF_PI, mul(sub(C1, mul(t, C2)), t)) };
            if b < 0.0 { r = sub(PI, r); }
            if a < 0.0 { r = -r; }
            r
        };
        assert!(LIVE_ATAN.iter().any(|&(a, b, g)| chopped(f32::from_bits(a), f32::from_bits(b)).to_bits() != g), "sister");
    }

    #[test]
    fn atanf_libm_tracks_host_libm_and_differs_from_it() {
        // Sanity: the fdlibm float core stays within 2 ulp of Rust's correctly-rounded atanf.
        let mut differs = 0;
        let mut state = 0x9e37_79b9u32;
        let mut next = || { state ^= state << 13; state ^= state >> 17; state ^= state << 5; (state >> 8) as f32 / (1 << 24) as f32 };
        for _ in 0..2000 {
            let x = (next() - 0.5) * 400.0;
            let g = atanf_libm(x);
            assert!((g - x.atan()).abs() <= 2.0 * (x.atan().abs() + 1e-30) * f32::EPSILON + 1e-30, "atanf {x}: {g} vs {}", x.atan());
            if g.to_bits() != x.atan().to_bits() { differs += 1; }
        }
        // The game's core is NOT the host's correctly-rounded atanf: it must differ somewhere.
        assert!(differs > 0, "sister: game atanf == host atanf on all 2000 samples");
        assert_eq!(atanf_libm(0.0).to_bits(), 0.0f32.to_bits());
        assert!((atanf_libm(f32::INFINITY) - 1.5707963).abs() < 1e-6);
    }

    #[test]
    fn atan2f_libm_tracks_host_libm_and_differs_from_it() {
        let mut differs = 0;
        let mut state = 0x853c_42e1u32;
        let mut next = || { state ^= state << 13; state ^= state >> 17; state ^= state << 5; (state >> 8) as f32 / (1 << 24) as f32 };
        for _ in 0..2000 {
            let (y, x) = ((next() - 0.5) * 400.0, (next() - 0.5) * 400.0);
            let g = atan2f_libm(y, x);
            assert!((g - y.atan2(x)).abs() <= 3.0 * (y.atan2(x).abs() + 1e-30) * f32::EPSILON + 1e-30, "atan2f({y},{x}): {g} vs {}", y.atan2(x));
            if g.to_bits() != y.atan2(x).to_bits() { differs += 1; }
        }
        assert!(differs > 0, "sister: game atan2f == host atan2 on all 2000 samples");
        // The vecutil pitch formula on a unit vector: atan2f(y, √(x²+z²)) — the game's shape.
        let (yv, xv, zv) = (0.3f32, 0.5, -0.8);
        let r2 = nf_formats::ps2float::add(nf_formats::ps2float::mul(xv, xv), nf_formats::ps2float::mul(zv, zv));
        let pitch = atan2f_libm(yv, nf_formats::ps2float::sqrt(r2));
        assert!((pitch - yv.atan2((xv * xv + zv * zv).sqrt())).abs() < 1e-6);
        // Special cases per the decompile.
        assert_eq!(atan2f_libm(0.0, -1.0).to_bits(), 0x4049_0fda);
        assert_eq!(atan2f_libm(0.0, 2.0).to_bits(), 0.0f32.to_bits());
        assert_eq!(atan2f_libm(-0.0, -1.0).to_bits(), 0xc049_0fda_u32);
    }

    #[test]
    fn approximations_track_libm() {
        for k in -40..=40 {
            let x = k as f32 * 0.17;
            assert!((sinf(x) - x.sin()).abs() < 2e-5, "sin {x}: {} vs {}", sinf(x), x.sin());
            assert!((cosf(x) - x.cos()).abs() < 2e-5);
        }
        for (a, b) in [(1.0f32, 2.0f32), (-3.0, 1.0), (0.5, -4.0), (-1.0, -1.0), (2.0, 0.0), (0.0, -2.0)] {
            assert!((atan2_approx(a, b) - a.atan2(b)).abs() < 6e-3, "atan2 {a} {b}: {} vs {}", atan2_approx(a, b), a.atan2(b));
        }
        assert_eq!(atan2_approx(0.0, 0.0), 0.0);
        assert!((angle_difference(0.1, TAU - 0.1) + 0.2).abs() < 1e-5);
        assert!((rotate_to_bearing(0.0, 1.0, 0.1, PI) - 0.1).abs() < 1e-6);
    }
}
