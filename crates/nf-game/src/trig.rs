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

/// `PS2Sinf` (0x120700): range reduction to [−π/2, π/2], then the VU0 polynomial in the game's operation order.
pub fn sinf(mut x: f32) -> f32 {
    const S1: f32 = f32::from_bits(0xbe2a_aaa4);
    const S2: f32 = f32::from_bits(0x3c08_873e);
    const S3: f32 = f32::from_bits(0xb94f_b21f);
    const S4: f32 = f32::from_bits(0x362e_9c14);
    if x >= PI { while x >= PI { x = sub(x, TAU); } } else { while x < -PI { x = add(x, TAU); } }
    if x > HALF_PI { x = sub(PI, x); } else if x < -HALF_PI { x = sub(-PI, x); }
    let x2 = mul(x, x);
    let a = mul(mul(x, S3), x2);
    let b = mul(x, S1);
    let c = mul(x, S2);
    let x4 = mul(x2, x2);
    let d = mul(x, S4);
    let x8 = mul(x4, x4);
    // ACC = b·x2; ACC += a·x4; ACC += c·x4; ACC += x·1; result = ACC + d·x8 (VU0: products and sums chopped, the
    // VU's adds keep their guard bit unlike the FPU's).
    let vadd = nf_formats::ps2float::add;
    let mut acc = mul(b, x2);
    acc = vadd(acc, mul(a, x4));
    acc = vadd(acc, mul(c, x4));
    acc = vadd(acc, x);
    vadd(acc, mul(d, x8))
}

/// Cosine as the game takes it: `PS2Sinf(θ + 1.5707964)`.
pub fn cosf(x: f32) -> f32 { sinf(add(x, f32::from_bits(0x3fc9_0fdb))) }

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
