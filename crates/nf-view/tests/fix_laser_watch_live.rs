//! fix-laser-watch-look: the Laser watch 0x4E's beam must match the measured game.
//!
//! Ground truth [D docs/live/2026-10-14-weapon-measurements-pcsx2.md "The laser watch 0x4e"]: on the
//! press the wrist turns so the watch face points along the aim, and a red beam runs from the watch
//! face to the crosshair point — wide pink-white glow at the watch, tapering to a thin red line at
//! the aimed-at surface — while the meter drains ~31/s (≈0.5 per frame) from 100.
//!
//! The game's model:
//! - The beam's start = the view model's posed BONE 0 world position — `AnimGetBoneWorldTrans
//!   (BLData+0x7e8, 0, 0, …)` [M Draw_LaserBeam 0x1a4560:20], re-positioned every held tick (the
//!   refire denormal fires every tick [L weapon_data 0x4e +0x40 = 1] and the +0x44 = 0 record
//!   spawns/re-draws on each [M 0x1a5460:688-690]).
//! - The beam object's angles: `vecutil_cartesian_to_spherical_acc(start − end)` with the elevation
//!   negated [M 0x1a4560:44-62, `PositionBeam` 0x1a9400 tail] — the object's +z points from the hit
//!   BACK to the watch, and the sprite's geometry extends along −z [D the "lazer" mesh 0x2000131 on
//!   07000005: z ∈ [−1.0198, 0.00016]] so the wide origin end sits at the watch and the beam flows
//!   to the hit.
//! - While held (view state 0xb [M Player_WeaponFiring 0x1a5460 — the +0x6C & 0x400 arm]) the HUD's
//!   ammo pane reads a 0–100 charge, NOT the clip percentage [M HUD_UpdateAmmoPane 0x19a7f0:160-176
//!   — the state-0xb branch returns before the kind-0x1a percent path], and that charge drains
//!   ~31/s [D the measured 100 → 59 over ~1.3 s]. The clip itself spends 1/tick [M Player_RoundToFire
//!   0x1a18e0 with +0x91 = 1].
//!
//! Master's failures this file pins:
//! 1. The renderer started the beam at the view model Character's STATIC creation matrix (identity)
//!    — the world origin — so the beam stretched from the map's origin and the user never saw it
//!    ("the laser isnt visible").
//! 2. The beam's +z pointed start→end (the port's `object_matrix` convention), so the −z sprite
//!    stretched backwards through the camera instead of to the wall ("The laser watch sucks").
//! 3. No held charge existed — the HUD kept the clip percent, draining ~12%/s of the 500 clip
//!    instead of the game's ~31/s.

use nf_game::player::Input;
use nf_formats::anim::Mat4;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The Laser equipped through the real switch input at 07000005's spawn, settled, aimed level at the
/// gatehouse road (the measurement's spot). Returns the eye and the aim direction at settle.
fn setup_laser(scene: &mut nf_view::scene::Scene) -> ([f32; 3], [f32; 3]) {
    let draw = Input { switch_to: Some(0x4e), ..Default::default() };
    let mut ok = false;
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(draw, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == 0x4e { ok = true; break; }
    }
    assert!(ok, "the Laser (0x4e) did not equip");
    // Settle: the draw script (0x60004ee, 76 frames [L weapon_data 0x4e +0xc0]) finishes.
    for _ in 0..90 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let (eye, dir) = {
        let sim = scene.sim.as_ref().unwrap();
        (sim.bond.head(), sim.bond.look())
    };
    (eye, dir)
}

/// The press + held ticks: the beam computes on the shot tick [M Player_MuzzleFlash 0x1a4298:107-137
/// via the +0x95b latch] and the fire script 0x60004e9 queues on the press [M Player_SetFiringAnim
/// 0x1a3c70].
fn hold_fire(scene: &mut nf_view::scene::Scene, ticks: u32) {
    for _ in 0..ticks {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    }
}

/// The attachment index of the Laser's beam slot (TRACER_SLOTS = 32, the "lazer" 0x2000131 sprite).
fn beam_attachment(scene: &nf_view::scene::Scene) -> usize {
    scene.attachments.iter().position(|a| a.tracer == Some(32)).expect("the lazer beam attachment exists")
}

/// The game's own matrices for a frame: `Scene::frame_state` (the same path the renderer draws with).
fn beam_matrix(scene: &nf_view::scene::Scene, slot: usize) -> nf_formats::anim::Mat4 {
    let fs = scene.frame_state(None, 0.0, Default::default());
    fs.attach[slot]
}

/// The beam leaves the posed watch bone (bone 0 of the view model) — NOT the world origin, NOT the
/// eye. Master drew it from the identity matrix's translation: [0, 0, 0].
#[test]
fn the_beam_starts_at_the_watch_face_bone() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (eye, _) = setup_laser(&mut scene);
    hold_fire(&mut scene, 2);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.laser_beam.is_some(), "the held laser computes a beam [M 0x1a4298:107-137]");
    }
    let slot = beam_attachment(&scene);
    let m = beam_matrix(&scene, slot);
    let start = [m[3][0], m[3][1], m[3][2]];
    // The old code: start = the identity creation matrix → the world origin.
    assert!(start[0].abs() + start[1].abs() + start[2].abs() > 1.0,
        "the beam must not draw from the world origin (master's bug)");
    // The watch face sits off the eye — the hand is low and left in the frame [D the measurement]:
    // the posed bone-0 world position differs from the eye by the view model's offset.
    let off = ((start[0] - eye[0]).powi(2) + (start[1] - eye[1]).powi(2) + (start[2] - eye[2]).powi(2)).sqrt();
    assert!(off > 0.1, "the beam starts at the posed watch bone, off the eye (offset {off:.3}) — not at the eye itself");
    // And it stays near the player: the watch is in front of the camera, not across the map.
    assert!(off < 3.0, "the watch bone sits within the view model's reach (offset {off:.3})");
}

/// The beam's taper: the game negates the elevation of (start − end) [M 0x1a4560:44-62], aiming the
/// object's +z from the hit back to the watch; the sprite's geometry extends along −z [D the lazer
/// mesh's z ∈ [−1.02, 0]], so the matrix's +z row must point end→start. Master aimed +z start→end —
/// the sprite stretched backwards through the camera.
#[test]
fn the_beam_points_its_negative_z_at_the_hit() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (_, _) = setup_laser(&mut scene);
    hold_fire(&mut scene, 2);
    let sim = scene.sim.as_ref().unwrap();
    let (a, b) = sim.laser_beam.expect("the held laser computes a beam");
    drop(sim);
    let slot = beam_attachment(&scene);
    let m = beam_matrix(&scene, slot);
    // +z (row 2) dotted with (start − end) must be positive: the +z axis points from the hit back to
    // the watch, so the −z geometry (scaled by the distance) flows watch → hit.
    let v = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let z = [m[2][0], m[2][1], m[2][2]];
    let dot = v[0] * z[0] + v[1] * z[1] + v[2] * z[2];
    assert!(dot > 0.0, "the beam's +z must point end→start (the sprite extends −z): dot {dot:.4}");
    // The stretch: the matrix's z row length equals the muzzle→hit distance (obj+0xE8 [M 0x1a4560:40]).
    let zlen = (z[0] * z[0] + z[1] * z[1] + z[2] * z[2]).sqrt();
    // The drawn beam starts at the view model's DATUM 0 (the watch face) [M 0x1c3120: a non-negative id
    // is a datum], a little off the sim's start point `a`: measure the stretch from the drawn start.
    let t = [m[3][0], m[3][1], m[3][2]];
    let dist = ((b[0] - t[0]).powi(2) + (b[1] - t[1]).powi(2) + (b[2] - t[2]).powi(2)).sqrt();
    assert!((zlen - dist).abs() < 0.5, "the beam stretches by the watch→hit distance (z {zlen:.2} vs dist {dist:.2})");
    // The translation sits at the start (the watch): obj+0x30 = the bone-0 position [M 0x1a4560:33-37].
    let dt = ((t[0] - a[0]).powi(2) + (t[1] - a[1]).powi(2) + (t[2] - a[2]).powi(2)).sqrt();
    assert!(dt < 1.0, "the beam object sits at the watch, near the sim's start ({dt:.2} off)");
}

/// The held meter: a 0–100 charge drained while the beam runs [M HUD_UpdateAmmoPane 0x19a7f0:160-176
/// — the state-0xb pane is NOT the clip percent; D the measured ~31/s ≈ 0.5/frame], while the clip
/// itself keeps its own 1/tick spend [M Player_RoundToFire 0x1a18e0, +0x91 = 1]. On the release the
/// beam stops and the fire script rings out into the release script [M Player_SetWeaponAnimObj
/// 0x1a2e38 case 0xb].
#[test]
fn the_held_meter_drains_31_per_second() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (_, _) = setup_laser(&mut scene);
    hold_fire(&mut scene, 1);
    let c0 = scene.sim.as_ref().unwrap().laser_charge;
    assert!((c0 - 100.0).abs() < 0.01, "the meter starts at 100 [D the measured first sample]");
    // 60 held ticks = 1 s → ~31 drained [D]; per tick ≈ 0.517.
    hold_fire(&mut scene, 60);
    let (c1, clip) = { let s = scene.sim.as_ref().unwrap(); (s.laser_charge, s.gun.clip) };
    let drained = c0 - c1;
    assert!((drained - 31.0).abs() < 2.0, "the meter drains ~31/s while held (drained {drained:.1} in 60 ticks)");
    assert_eq!(clip, 499 - 60, "the clip keeps its own 1/tick spend [M Player_RoundToFire]");
    // The release: the beam stops, the fire script rings out, and the meter refills while idle
    // (the game's gadget regen shape [M Player_Update 0x193600 clip +1 per 4 frames] — [S] for the
    // charge's own refill, live check in the report).
    for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.laser_beam.is_none(), "the beam stops on the release [M 0x1a2e38 case 0xb → 0xc]");
        // The release script 0x60004e8 (26 frames [L weapon_data 0x4e +0xb4]) plays at 1.5x [M
        // 0x1a2e38 case 0xb] and rings out into the idle 0x60004ea — within the 40 settle ticks
        // either endpoint is the correct state.
        assert!(matches!(sim.view_script, 0x6000_4e8 | 0x6000_4ea),
            "the release rings out (view_script {:#x})", sim.view_script);
    }
    let c2 = scene.sim.as_ref().unwrap().laser_charge;
    assert!(c2 > c1, "the meter refills while idle (live check: the game's refill shape)");
}
