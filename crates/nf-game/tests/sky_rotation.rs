//! The 0x0700001B rotating sky [M View_DrawSky 0x1e5ad0:186-198] against the decompile's arithmetic,
//! replicated in `scripts/sky_golden.py`: the `rotatosky.168` counter advances once per slot pass that
//! reaches the block — both of 0700001b's slots qualify [D disc: SKY_StarField and SKY_Earth, glist flags
//! 0x222, bit 0x8 clear] — so the angle runs at 2/1440 rad per frame. The sun vector is
//! `Vec_Spherical_2_Cartesian(1000.0, 0x409f2c8e, 0xbf29c920)` = 285.00002° / −38.00000° [M 0x1e3948,
//! 0x1e5ad0:187], rotated through the slot's full matrix. Pure test: no NF_ISO needed.

use nf_game::sky::*;
use nf_formats::ps2float::mul;

#[test]
fn rotation_advances_two_counts_per_frame() {
    let mut rot = RotatingSky::default();
    let mut spins = Vec::new();
    for _ in 0..600 {
        // One frame = two slot passes (StarField then Earth), unpaused.
        let s0 = rot.slot_pass(0x0700_001b, false).unwrap();
        let s1 = rot.slot_pass(0x0700_001b, false).unwrap();
        spins.push((s0, s1));
    }
    // The angle: (float)count × 0.00069444446, increment before the read [M 0x1e5ad0:189-192].
    // After n frames the second pass reads count 2n: f32 bits from the golden.
    let golden: [(u32, u32); 5] = [
        (1, 0x3a36_0b61), // frame 1, pass 1 (count 1) — the angle constant itself
        (2, 0x3ab6_0b61), // frame 1, pass 2 (count 2)
        (60, 0x3d2a_aaaa), // frame 30, pass 2 (count 60) — 0.041666664
        (1200, 0x3f55_5555), // frame 600, pass 2 (count 1200) — 0.8333333
        (2881, 0x4000_0b60), // frame 1441, pass 1 (count 2881)
    ];
    assert_eq!(spins[0].0.to_bits(), golden[0].1, "frame 1 pass 1");
    assert_eq!(spins[0].1.to_bits(), golden[1].1, "frame 1 pass 2");
    assert_eq!(spins[29].1.to_bits(), golden[2].1, "frame 30");
    assert_eq!(spins[599].1.to_bits(), golden[3].1, "frame 600");
    let mut rot2 = RotatingSky::default();
    for _ in 0..1440 { rot2.slot_pass(0x0700_001b, false).unwrap(); rot2.slot_pass(0x0700_001b, false).unwrap(); }
    assert_eq!(rot2.slot_pass(0x0700_001b, false).unwrap().to_bits(), golden[4].1, "frame 1441 pass 1 (count 2881)");
    // A full turn: 2π rad needs 2π·1440/2 ≈ 4524 frames ≈ 75.4 s at 60 Hz.
    // Sister: the two slots' angles stay one count apart, forever (the game's two sky layers rotate
    // offset by exactly one count).
    assert_eq!((spins[599].1 - spins[598].1) as u32, 0, "second pass keeps advancing");
    assert_eq!(rot2.count, 2881, "count after 1441 frames");
}

#[test]
fn rotation_gates_on_level_and_pause() {
    let mut rot = RotatingSky::default();
    // Only 0x0700001B runs the block [M 0x1e5ad0:186].
    assert_eq!(rot.slot_pass(0x0700_0005, false), None);
    assert_eq!(rot.count, 0, "other levels never advance the counter");
    // Paused (GameState[0x50] != 0): the matrix still uses the current angle, but the counter freezes
    // [M 0x1e5ad0:188-192].
    let a = rot.slot_pass(0x0700_001b, true).unwrap();
    assert_eq!(rot.count, 0, "paused does not advance");
    assert_eq!(a.to_bits(), 0.0f32.to_bits(), "angle 0 × const = 0");
    let b = rot.slot_pass(0x0700_001b, false).unwrap();
    assert_eq!(rot.count, 1);
    assert_eq!(b.to_bits(), 0.00069444446f32.to_bits(), "first unpaused pass = one count");
    // Sister: pause again — the angle keeps being returned (the draw needs it) without advancing.
    let c = rot.slot_pass(0x0700_001b, true).unwrap();
    assert_eq!(c.to_bits(), b.to_bits(), "paused re-reads the same angle");
}

#[test]
fn sun_vector_and_spin_matrix_match_the_decompile() {
    // Vec_Spherical_2_Cartesian(1000.0, 0x409f2c8e, 0xbf29c920) [M 0x1e5ad0:187], f32 bits from the golden.
    let (r, theta, phi) = ROTATING_SKY_SUN_ARGS;
    assert_eq!(theta.to_bits(), 0x409f_2c8e, "theta literal kept at its bits (285.00002°)");
    assert_eq!(phi.to_bits(), 0xbf29_c920, "phi literal kept at its bits (−38.00000°)");
    let sun = spherical_2_cartesian(r, theta, phi);
    let golden: [(u32, [u32; 3]); 1] = [(0, [0xc43e_4a3a, 0xc419_ea56, 0x434b_f3be])];
    for (k, g) in golden[0].1.iter().enumerate() {
        assert_eq!(sun[k].to_bits(), *g, "sun component {k}");
    }
    // The spin matrix for count 1200 (frame 600) with an identity camera and no slot rotation:
    // RotMatrix([spin, 0, 0]) [M 0x1e2368], then ApplyMatrixLVI on the sun [M 0x1e4960].
    let spin = mul(1200.0, 0.00069444446);
    assert_eq!(spin.to_bits(), 0x3f55_5555);
    let id = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let m = rotating_sky_matrix(&id, [0.0, 0.0, 0.0], spin);
    let mut v = sun;
    apply_matrix_lvi(&m, &mut v);
    let golden_rot: [u32; 3] = [0xc43e_4a3f, 0xc40d_3c1b, 0xc39f_477d];
    for (k, g) in golden_rot.iter().enumerate() {
        assert_eq!(v[k].to_bits(), *g, "rotated sun component {k}");
    }
    // Sister: the unrotated sun differs from the rotated one in every component (the rotation is real).
    assert_ne!(v, sun);
    // And RotMatrix([θ,0,0]) keeps the X row (m0) at cy·cz > 1 — the game's sinf(π/2) oversioots to
    // 0x3f800001, so the rotation rows are not exactly orthonormal; the port reproduces that.
    let rm = rot_matrix([spin, 0.0, 0.0]);
    assert_eq!(rm[0].to_bits(), 0x3f80_0002, "m0 = cy·cz with cy = sinf(π/2) = 1.0000001");
    assert_eq!(rm[1].to_bits(), 0.0f32.to_bits(), "m1 = sx·sy·cz + cx·sz = 0 at sy = sz = 0");
    assert_eq!(rm[8].to_bits(), 0.0f32.to_bits(), "m8 = sy = 0");
}
