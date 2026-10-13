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

    #[test]
    fn mul_truncates_toward_zero() {
        // A genuine chop: the exact square of 0x3F001907 rounds to nearest UP (0x3E803213); truncation gives
        // the value below.
        let a = f32::from_bits(0x3f00_1907);
        assert_eq!(mul(a, a).to_bits(), 0x3e80_3212);
        assert_eq!((a * a).to_bits(), 0x3e80_3213);
        // Here the nearest f32 (1.0) is below the true product, so chop keeps it: truncation only steps down
        // when nearest overshot.
        assert_eq!(mul(f32::from_bits(0x3eaa_aaab), 3.0).to_bits(), 0x3f80_0000);
        // Exact products stay exact; negatives mirror.
        assert_eq!(mul(1.5, 2.0), 3.0);
        assert_eq!(mul(-a, a).to_bits(), 0xbe80_3212);
    }

    #[test]
    fn fpu_add_has_no_guard_bit() {
        // d ≥ 25: the smaller operand becomes ±0 entirely.
        assert_eq!(fpu_add(1.0, 2f32.powi(-26)), 1.0);
        assert_eq!(vu_add(1.0, 2f32.powi(-26)), 1.0);
        // d = 23 trims 0x407FFFFF (= 3.99999976, fraction all ones) down to 1.5·2¹ = 3.0 (the fraction below one
        // position is cleared), which moves the sum onto the exact tie 16777219; ties-to-even overshoots and chop
        // lands on 16777218.
        assert_eq!(fpu_add(16777216.0, f32::from_bits(0x407f_ffff)), 16777218.0);
    }

    #[test]
    fn fpu_sub_mirrors_fpu_add() {
        let b = 2f32.powi(-24) * (1.0 + 2f32.powi(-6));
        // 1 − b' (b' = the trimmed 2⁻²⁴) is the exact tie below 1: nearest overshoots to 1.0, chop takes it down.
        assert_eq!(fpu_sub(1.0, b).to_bits(), BELOW_1.to_bits());
        assert_eq!(fpu_add(1.0, -b), fpu_sub(1.0, b));
    }

    #[test]
    fn vu_madd_accumulates_with_a_guard_bit() {
        // ACC = 0 + d·e is a plain chopped product (the chop case from mul_truncates_toward_zero).
        let a = f32::from_bits(0x3f00_1907);
        assert_eq!(vu_madd(a, a, 0.0).to_bits(), 0x3e80_3212);
        // Adding 1.0 keeps 1.0: the sum 1 + 2⁻²⁴·(1+2⁻⁶) sits between 1.0 and 1+2⁻²³ and truncates to 1.0
        // (guard bit or not — the fpu-vs-VU0 difference is exercised on the recorded PS2Sinf calls in
        // tests/diff_calls.rs, where an FPU-sum polynomial mismatches the game and the VU-sum one does not).
        let b = 2f32.powi(-24) * (1.0 + 2f32.powi(-6));
        assert_eq!(vu_madd(1.0, b, 1.0).to_bits(), 0x3f80_0000);
        // A chain matches the PS2Sinf evaluation order: ACC = b·x², then ACC += a·x⁴.
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
