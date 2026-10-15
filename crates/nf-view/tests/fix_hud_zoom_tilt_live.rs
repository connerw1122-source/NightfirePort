//! fix-hud-zoom-tilt (GAPS G-044, task 110z) — the exact terms of the view-model tilt.
//!
//! `Player_WeaponRecoil` 0x1a3de8:52-57 writes the gun rot every tick:
//!   rot+0x50 = BL+0x124 · (−π/6) / viewer+0x108
//!   rot+0x54 = viewer+0x114 · −(BL+0x120 · π/6 / viewer+0x108) + BL+0x8e4
//! `Player_PositionCamera` 0x18f5d8 feeds viewer+0x108 from BL+0x8D0 every frame in cam modes 0/0xe
//! (case 1 pins it to 1.0) — the port's `Sim::zoom`. `Camera_CalcViewAngles` 0x1baf70:29-41 writes
//! viewer+0x114 = 4/3 (0x3faaaaab), or 16/9 under the widescreen option (DAT_002a37c4 =
//! switch_CONFIG_WIDESCREEN 0x216168:10), ×0.5/×2 per split-screen pair — the port models the 4:3
//! single view. [L ee_start_05 viewer 0x1fc6c10: +0x108 = 1.0, +0x114 = 0x3faaaaab, +0x118 = π/3.]
//!
//! The zoom divide + the 4/3 landed on master with fix-viewmodel-zoom (b12a1b77), whose scene.rs
//! diff ONLY appends `/ self.zoom` to the two terms — so at zoom 1 the terms are bit-for-bit the
//! pre-fix values (IEEE ÷1.0 is exact), which these tests pin behaviourally (the sister). What was
//! missing is the task's exact form: at zoom 2 each tilt term is half its zoom-1 value (IEEE ÷2 is
//! exact too), the aspect term is the decompile's constant, and the real input path's ramp tilt
//! follows the formula tick-by-tick — `fix_viewmodel_zoom_live` only checks the composed angle to a
//! loose 10% at an uncontrolled zoom. Opt-in via `NF_ISO`.

/// The gun's tilt terms from `Sim::gun_matrix()`: its forward row is
/// `gf = fwd·cos(gp)·cos(gy) − left·cos(gp)·sin(gy) + up·sin(gp)` (scene.rs, the same axes the
/// method builds), and the view frame (fwd, left, up) is orthonormal, so
/// `up·gf = sin(gp)`, `−left·gf = cos(gp)·sin(gy)`, `fwd·gf = cos(gp)·cos(gy)`:
/// gp = asin(up·gf), gy = atan2(−left·gf, fwd·gf). gp is the game's rot+0x50 term (−cy·π/6/zoom,
/// the sign folded into the axis build), gy the rot+0x54 x-term (aspect·−cx·π/6/zoom).
fn tilt_terms(yaw: f32, pitch: f32, look: [f32; 3], m: [[f32; 4]; 4]) -> (f32, f32) {
    let (y, pt) = (yaw, pitch * std::f32::consts::FRAC_PI_2);
    let left = [y.cos(), 0.0, -y.sin()];
    let up = [-pt.sin() * y.sin(), pt.cos(), -pt.sin() * y.cos()];
    let gf = [m[2][0], m[2][1], m[2][2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    ((-(dot(left, gf))).atan2(dot(look, gf)), dot(up, gf).asin())
}

/// The decompile's aspect constant: `Camera_CalcViewAngles` 0x1baf70 writes 0x3faaaaab (the 4:3
/// branch) to viewer+0x114 [L ee_start_05], and the port's `4.0 / 3.0` literal is that bit pattern.
#[test]
fn the_aspect_constant_is_the_games_bits() {
    assert_eq!(4.0f32 / 3.0, f32::from_bits(0x3faaaaab), "viewer+0x114 [L ee_start_05]");
}

/// The exact terms at zoom 1 and zoom 2 (task 110z: "at zoom 2 the tilt term is half its zoom-1
/// value ... and the aspect term matches the decompile's constant"). The Ronin (0x32) is scoped
/// [L ee_start_05 weapon_data 0x2bf150+0x32·0x10c+0x68 = 0x12040] with zoom 3.5; the zoom here is
/// set to 1.0 / 2.0 directly = BL+0x8D0 mid-ramp (the real ramp is exercised by the next test);
/// `Player_PositionCamera` 0x18f5d8 feeds viewer+0x108 from it verbatim every frame.
#[test]
fn the_tilt_terms_follow_the_game_formula() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x32; // the Ronin
        sim.gun.switch_request = 0x32; // clear the loadout's pending switch
        sim.aiming = true;
        sim.free_aim = true; // crosshair() returns BL+0x120/0x124 verbatim
        // The smoothed crosshair (BL+0x120/0x124) with the raw cursor equal to it, so the aim ease
        // holds it still (fix_viewmodel_zoom_live's setup).
        sim.cursor = [0.25, -0.2];
        sim.cross = [0.25, -0.2];
        sim.zoom = 1.0;
        sim.zoom_target = 1.0;
    }
    let kx = 4.0f32 / 3.0; // viewer+0x114 [L ee_start_05 0x1fc6c10+0x114 = 0x3faaaaab]
    let sim = scene.sim.as_mut().unwrap();
    let (cx, cy) = sim.crosshair();
    let look = sim.bond.look();
    let (yaw, pitch) = (sim.bond.yaw, sim.bond.pitch);
    // Zoom 1 — the terms equal the pre-fix (no-divide) formula: the merged diff (b12a1b77) only
    // appends `/ self.zoom`, and IEEE ÷1.0 is exact, so the zoom-1 matrix is bit-for-bit master's.
    // The 5e-6 is the asin/atan2 round-trip through the matrix (a missing divide errs ~0.065).
    let (gy1, gp1) = tilt_terms(yaw, pitch, look, sim.gun_matrix());
    assert!((gy1 - kx * cx * std::f32::consts::FRAC_PI_6).abs() < 5e-6, "zoom-1 yaw term {gy1} vs {} (aspect {}·cx {}·π/6)", kx * cx * std::f32::consts::FRAC_PI_6, kx, cx);
    assert!((gp1 - cy * std::f32::consts::FRAC_PI_6).abs() < 5e-6, "zoom-1 pitch term {gp1} vs {}", cy * std::f32::consts::FRAC_PI_6);
    // Zoom 2 — each term half its zoom-1 value (the game's ÷ viewer+0x108; IEEE ÷2 is exact).
    sim.zoom = 2.0;
    let (gy2, gp2) = tilt_terms(yaw, pitch, look, sim.gun_matrix());
    assert!((gy2 - gy1 / 2.0).abs() < 5e-6, "zoom-2 yaw term {gy2} vs half the zoom-1 {gy1}");
    assert!((gp2 - gp1 / 2.0).abs() < 5e-6, "zoom-2 pitch term {gp2} vs half the zoom-1 {gp1}");
    assert!((gy2 - kx * cx * std::f32::consts::FRAC_PI_6 / 2.0).abs() < 5e-6, "zoom-2 yaw vs the formula");
    assert!((gp2 - cy * std::f32::consts::FRAC_PI_6 / 2.0).abs() < 5e-6, "zoom-2 pitch vs the formula");
}

/// Through the real input path: aim + hold the zoom button on the Ronin (0x32, zoom 3.5
/// [L ee_start_05 weapon_data+0x88]) — `Player_Zoom` 0x1a1a70 ramps BL+0x8D0 up, and at EVERY tick
/// the tilt terms must equal the game's formula at that tick's zoom (both divided by viewer+0x108,
/// the x term scaled by viewer+0x114 = 4/3). Before the zoom divide the terms stayed at the zoom-1
/// values, so every tick past the first failed.
#[test]
fn the_zoomed_tilt_tracks_the_ramp_through_the_real_input_path() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x32;
        sim.gun.switch_request = 0x32;
        sim.aiming = true;
        sim.free_aim = true;
        sim.cursor = [0.25, -0.2];
        sim.cross = [0.25, -0.2];
        sim.zoom = 1.0;
        sim.zoom_target = 1.0;
    }
    let (cx, cy) = scene.sim.as_ref().unwrap().crosshair();
    let kx = 4.0f32 / 3.0;
    let mut settled = false;
    for step in 0..400 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, zoom: true, ..Default::default() }, false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.cross[0], 0.25, "the crosshair held (tick {step})");
        assert_eq!(sim.cross[1], -0.2, "the crosshair held (tick {step})");
        let z = sim.zoom;
        let look = sim.bond.look();
        let (yaw, pitch) = (sim.bond.yaw, sim.bond.pitch);
        let (gy, gp) = tilt_terms(yaw, pitch, look, sim.gun_matrix());
        let ex_gy = kx * cx * std::f32::consts::FRAC_PI_6 / z;
        let ex_gp = cy * std::f32::consts::FRAC_PI_6 / z;
        assert!((gy - ex_gy).abs() < 5e-6, "tick {step} zoom {z}: yaw term {gy} vs {ex_gy}");
        assert!((gp - ex_gp).abs() < 5e-6, "tick {step} zoom {z}: pitch term {gp} vs {ex_gp}");
        if z >= 3.4 {
            settled = true;
            break;
        }
    }
    assert!(settled, "the zoom ramp never neared the Ronin's 3.5 (got {})", scene.sim.as_ref().unwrap().zoom);
}
