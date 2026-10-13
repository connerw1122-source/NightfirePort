//! PS2 single-precision arithmetic as the EE FPU and VU0 round, as one facade for the port's bit-exact maths.
//! The models live in `nf_formats::ps2float` (computed exactly in f64, then rounded the way the hardware does);
//! this module names the two contexts so call sites say which unit they are porting:
//! - **EE FPU** (scalar float code): multiply truncates toward zero; add/sub have **no guard bit** — the operand
//!   with the smaller exponent loses the mantissa bits that would shift out beyond one position (d ≥ 25: it
//!   becomes ±0; d > 0: its low d−1 bits are cleared), then the result truncates. Sourced from PCSX2's iFPU
//!   `FPU_ADD_SUB` (the rule only), validated on live `ATAN2_APPROX` / `PS2Sinf` / `Vec_AngleDifference` results
//!   (WORKLOG 0026) and on the local pose (WORKLOG 0017).
//! - **VU0** (vector/integer-unit float ops, `_vmul`/`_vmadd…`): products and sums truncate, but the accumulator
//!   keeps its guard bit, so `VMADD` chains round differently from the FPU (validated on the `PS2Sinf` polynomial,
//!   WORKLOG 0026).
//! Spec: docs/formats/animation.md "Arithmetic"; diff harness `crates/nf-game/tests/diff_calls.rs`.

use nf_formats::ps2float;

/// EE FPU / VU0 multiply: the exact product truncated toward zero.
pub fn mul(a: f32, b: f32) -> f32 { ps2float::mul(a, b) }

/// EE FPU add: no guard bit on the smaller operand, result truncated toward zero.
pub fn fpu_add(a: f32, b: f32) -> f32 { ps2float::add_noguard(a, b, true) }

/// EE FPU subtract.
pub fn fpu_sub(a: f32, b: f32) -> f32 { ps2float::add_noguard(a, -b, true) }

/// VU0 add (also the FPU's behaviour when the exponents are close): the exact sum truncated toward zero.
pub fn vu_add(a: f32, b: f32) -> f32 { ps2float::add(a, b) }

/// VU0 subtract.
pub fn vu_sub(a: f32, b: f32) -> f32 { ps2float::add(a, -b) }

/// VU0 multiply-add: `ACC = ACC + d·e` — the product truncated, the sum keeping the guard bit.
pub fn vu_madd(d: f32, e: f32, acc: f32) -> f32 { ps2float::add(acc, ps2float::mul(d, e)) }

#[cfg(test)]
mod tests {
    use super::*;

    /// The f32 just below 1.0 (0x3F7FFFFF).
    const BELOW_1: f32 = f32::from_bits(0x3f7f_ffff);
    /// 1 + 2⁻²⁴ (0x3F800001).
    const ABOVE_1: f32 = f32::from_bits(0x3f80_0001);

    #[test]
    fn mul_truncates_toward_zero() {
        // 0x3EAAAAAB = the f32 above 1/3; × 3 = 1.0000000298…: nearest is 1.0, chop gives the value below.
        assert_eq!(mul(f32::from_bits(0x3eaa_aaab), 3.0).to_bits(), 0x3f7f_ffff);
        // The same product rounded to nearest would be 1.0 — the chop is the difference.
        assert_eq!((f32::from_bits(0x3eaa_aaab) * 3.0).to_bits(), 0x3f80_0000);
        // Exact products stay exact; negatives mirror.
        assert_eq!(mul(1.5, 2.0), 3.0);
        assert_eq!(mul(-f32::from_bits(0x3eaa_aaab), 3.0).to_bits(), 0xbf7f_ffff);
    }

    #[test]
    fn fpu_add_has_no_guard_bit() {
        // b = 2⁻²⁴·(1 + 2⁻⁶): its mantissa tail sits beyond one position for d = 24.
        let b = 2f32.powi(-24) * (1.0 + 2f32.powi(-6));
        // FPU: b is trimmed to 2⁻²⁴, the sum 1 + 2⁻²⁴ is the exact halfway point and truncates to 1.
        assert_eq!(fpu_add(1.0, b).to_bits(), 0x3f80_0000);
        // VU0 (guard bit kept): the exact sum 1 + 2⁻²⁴ + 2⁻³⁰ is above halfway → 1 + 2⁻²⁴.
        assert_eq!(vu_add(1.0, b).to_bits(), 0x3f80_0001);
        // d ≥ 25: the smaller operand becomes ±0 entirely.
        assert_eq!(fpu_add(1.0, 2f32.powi(-26)), 1.0);
        assert_eq!(vu_add(1.0, 2f32.powi(-26)), 1.0);
    }

    #[test]
    fn fpu_sub_mirrors_fpu_add() {
        let b = 2f32.powi(-24) * (1.0 + 2f32.powi(-6));
        assert_eq!(fpu_sub(1.0, b).to_bits(), BELOW_1.to_bits() + 1); // 1 − 2⁻²⁴ (halfway, chop → below)
        assert_eq!(fpu_add(1.0, -b), fpu_sub(1.0, b));
    }

    #[test]
    fn vu_madd_accumulates_with_a_guard_bit() {
        // ACC = 0 + d·e is a plain chopped product.
        assert_eq!(vu_madd(f32::from_bits(0x3eaa_aaab), 3.0, 0.0).to_bits(), 0x3f7f_ffff);
        // Adding 2⁻²⁴·(1+2⁻⁶) keeps the tail: the result is 1 + 2⁻²⁴, not the FPU's 1.0.
        let b = 2f32.powi(-24) * (1.0 + 2f32.powi(-6));
        assert_eq!(vu_madd(1.0, b, 0.0).to_bits(), ABOVE_1.to_bits());
        // A chain matches the PS2Sinf evaluation order: ACC = b·x², + a·x⁴ …
        let (x, x2) = (0.5f32, 0.25f32);
        let acc = vu_madd(x, x2, mul(0.5, x2));
        assert_eq!(acc, vu_add(mul(0.5, x2), mul(x, x2)));
    }

    #[test]
    fn sub_is_add_of_the_negation() {
        assert_eq!(vu_sub(1.0, 0.25), 0.75);
        assert_eq!(fpu_sub(0.25, 1.0), -0.75);
    }
}
