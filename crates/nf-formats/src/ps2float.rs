//! Single-precision arithmetic as the EE FPU and VU0 round: toward zero (chop). Each operation is computed exactly
//! (or with an exact error term) in f64 and truncated to f32. Which mode reproduces the game is measured against
//! PCSX2 snapshots (docs/formats/animation.md "Arithmetic").

/// f64 value plus an exact error term `e` (true value = v + e) → f32 rounded toward zero.
fn chop(v: f64, e: f64) -> f32 {
    let n = v as f32;
    let mut r = n;
    if (n as f64).abs() > v.abs() || ((n as f64) == v && e != 0.0 && (e < 0.0) != (v < 0.0)) {
        // Nearest overshot the true magnitude (or v is exact but the true value is slightly smaller): one ulp
        // toward zero.
        if r != 0.0 { r = f32::from_bits(r.to_bits() - 1); }
    }
    r
}

pub fn mul(a: f32, b: f32) -> f32 { chop(a as f64 * b as f64, 0.0) }

pub fn add(a: f32, b: f32) -> f32 {
    let (a, b) = (a as f64, b as f64);
    let s = a + b;
    let bb = s - a;
    let e = (a - (s - bb)) + (b - bb);
    chop(s, e)
}

pub fn sub(a: f32, b: f32) -> f32 { add(a, -b) }

/// Candidate model of the EE FPU's add without guard bits (as PCSX2's `FPU_ADD_SUB` emulates it, *sourced*: PCSX2
/// iFPU, GPL — the rule only): before adding, the operand with the smaller exponent loses the mantissa bits that would
/// be shifted out beyond one position (exponent difference d ≥ 25: it becomes ±0; d > 0: its low d − 1 bits are
/// cleared). `chop` selects the final rounding.
pub fn add_noguard(a: f32, b: f32, chop_result: bool) -> f32 {
    let (ea, eb) = ((a.to_bits() >> 23) & 0xff, (b.to_bits() >> 23) & 0xff);
    let trim = |small: f32, d: u32| -> f32 {
        if d >= 25 { f32::from_bits(small.to_bits() & 0x8000_0000) }
        else if d > 1 { f32::from_bits(small.to_bits() & !((1u32 << (d - 1)) - 1)) }
        else { small }
    };
    let (a, b) = if ea > eb { (a, trim(b, ea - eb)) } else if eb > ea { (trim(a, eb - ea), b) } else { (a, b) };
    if chop_result { add(a, b) } else { a + b }
}

pub fn sqrt(a: f32) -> f32 {
    let r = (a as f64).sqrt();
    // r² vs a decides whether the f64 root rounded up.
    chop(r, a as f64 - r * r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chops_toward_zero() {
        // 1/3 is below the nearest f32 0.33333334 (rounded up): chop gives the value below.
        let third = chop(1.0 / 3.0, 0.0);
        assert!(third < 1.0f32 / 3.0);
        assert_eq!(f32::from_bits(third.to_bits() + 1), 1.0f32 / 3.0);
        assert_eq!(chop(-1.0 / 3.0, 0.0), -third);
        // Exact results stay exact.
        assert_eq!(mul(1.5, 2.0), 3.0);
        assert_eq!(add(1.0, 2.0), 3.0);
        // 1 + 2^-30: nearest is 1.0 and so is chop; 1 − 2^-30 chops to the f32 just below 1.
        assert_eq!(add(1.0, 2f32.powi(-30)), 1.0);
        assert_eq!(add(1.0, -2f32.powi(-30)), f32::from_bits(1.0f32.to_bits() - 1));
        assert_eq!(sqrt(4.0), 2.0);
        assert!(sqrt(2.0) <= std::f32::consts::SQRT_2);
    }
}
