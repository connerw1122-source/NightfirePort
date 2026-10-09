//! The game's angle maths (ACTION.ELF), ported operation by operation. Rounding: EE FPU / VU0 chop
//! (`nf_formats::ps2float`); constants as their f32 bit patterns. Spec: docs/systems/drone-ai.md "Maths".

use nf_formats::ps2float::{add, mul, sub};

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
    // ACC = b·x2; ACC += a·x4; ACC += c·x4; ACC += x·1; result = ACC + d·x8
    let mut acc = mul(b, x2);
    acc = add(acc, mul(a, x4));
    acc = add(acc, mul(c, x4));
    acc = add(acc, x);
    add(acc, mul(d, x8))
}

/// Cosine as the game takes it: `PS2Sinf(θ + 1.5707964)`.
pub fn cosf(x: f32) -> f32 { sinf(add(x, f32::from_bits(0x3fc9_0fdb))) }

/// `Vec_AngleDifference(a, b)` (0x1e2b78): b − a wrapped to (−π, π], both first normalised to [0, 2π).
pub fn angle_difference(a: f32, b: f32) -> f32 {
    let norm = |v: f32| { let r = v % TAU; if r < 0.0 { r + TAU } else { r } };
    let mut d = norm(b) - norm(a);
    if d.abs() >= PI { if d >= 0.0 { d -= TAU } else { d += TAU } }
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
