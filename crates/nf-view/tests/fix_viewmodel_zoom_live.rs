//! fix-viewmodel-zoom (GAPS G-044 + the zoom overlays):
//! 1. `Player_WeaponRecoil` 0x1a3de8:52-57 divides the view-model tilt by `glb_viewer+0x108` (the zoom,
//!    fed from `BLData+0x8D0` by `Player_PositionCamera` 0x18f5d8:129) and scales the x term by
//!    `viewer+0x114` = 4/3 on a 4:3 screen [L ee_start_05 viewer 0x1fc6c10+0x114 = 0x3faaaaab; the
//!    widescreen value 16/9 is `Camera_CalcViewAngles` 0x1baf70's DAT_002a37c4 branch]. The port
//!    divided by nothing: a zoomed view tilted the gun as if unzoomed.
//! 2. The zoom overlay per weapon id [M `HUD_UpdateCrossHair` 0x19e6d0:106-147]: in aim mode a scoped
//!    weapon (flags +0x68 & 0x40 [L weapon_data 0x2bf150]) enables its own zoom pane — 0x1a/0x1b the
//!    OICW pane (17), 0x32/0x33 the Laser pane (19, the Ronin), 0x54/0x55 the Camera pane (11, the
//!    watch's viewfinder; 0x55 is the taser twin that saves as 0x54), any other scoped weapon (the
//!    sniper 0x24, zoom 10) the Sight pane (6, the scope). The port drew the scope for every zoom —
//!    the user: "The micro camera, when you zoom in, doesn't actually have the camera overlay, but
//!    the scope of a sniper."
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Quads of one texture in the frame: (first vertex, count) runs.
fn tex_runs(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, hash: u32) -> Vec<(usize, usize)> {
    let Some(ti) = scene.hud_textures.iter().position(|(h, _)| *h == hash) else { return Vec::new() };
    f.runs.iter().filter(|r| r.0 == ti + 1).map(|r| (r.2 as usize, r.3 as usize)).collect()
}

/// G-044: the gun tilt divides by the zoom. Through the real input path: aim + hold the zoom button
/// on the sniper (0x24), the tilt at zoom 2+ must be the zoom-1 tilt divided by the zoom.
#[test]
fn the_gun_tilt_divides_by_the_zoom() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x24; // the sniper: flags 0x4080D0 (scoped), zoom 10 [L weapon_data]
        sim.gun.switch_request = 0x24; // clear the loadout's pending switch (it would take the slot back)
        sim.aiming = true;
        // The smoothed crosshair offset (BL+0x120/0x124) with the raw cursor (BL+0x118/0x11C) equal to
        // it, so `Player_AutoAim`'s ease toward the cursor holds it still while the zoom ramps.
        sim.cursor = [0.25, -0.2];
        sim.cross = [0.25, -0.2];
        sim.zoom = 1.0;
        sim.zoom_target = 1.0;
    }
    let fwd = scene.sim.as_ref().unwrap().bond.look();
    let tilt = |m: [[f32; 4]; 4]| {
        let gf = [m[2][0], m[2][1], m[2][2]];
        let d = (gf[0] * fwd[0] + gf[1] * fwd[1] + gf[2] * fwd[2]).clamp(-1.0, 1.0);
        d.acos()
    };
    let a1 = tilt(scene.sim.as_ref().unwrap().gun_matrix());
    assert!(a1 > 0.01, "a non-zero crosshair tilts the gun, got {a1}");
    // Hold the zoom through `Player_Zoom` (0x1a1a70) — the zoom ramps toward the sniper's 10.
    for _ in 0..400 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, zoom: true, ..Default::default() }, false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.zoom > 1.9, "the zoom rose on the scoped weapon, got {}", sim.zoom);
    let a2 = tilt(sim.gun_matrix());
    // Both tilt terms divide by `viewer+0x108`: at zoom z the tilt is a1/z. Before the fix the port
    // kept the zoom-1 tilt at every zoom (a2 == a1), so this failed.
    assert!((a2 - a1 / sim.zoom).abs() < 0.1 * a1, "tilt {a2} at zoom {} vs {a1} at zoom 1", sim.zoom);
    // Sister: the tilt shrinks strictly as the zoom rises.
    assert!(a2 < a1, "zoomed tilt {a2} must be smaller than the unzoomed {a1}");
}

/// The zoom overlay selection, drawn: the sniper keeps the scope; the watch (0x54) draws the camera
/// viewfinder (the user's bug — it drew the sniper scope); the OICW its circle; the Ronin its laser
/// frame; unzoomed, nothing. 07000014 ships the scope (0x03000034), viewfinder (0x03000055/56/57)
/// and OICW (0x03000141) textures [D tex-list].
#[test]
fn each_zoomable_weapon_draws_its_own_zoom_overlay() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000014.bin", false).expect("load");
    let mut zoomed = |scene: &mut nf_view::scene::Scene, w: u8| {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = w;
        sim.zoom_target = sim.weapons.zoom(w as usize).max(2.0);
        sim.zoom = sim.zoom_target;
        nf_view::hud::overlay(scene, 4.0 / 3.0)
    };
    // The sniper (0x24): the scope — the port's existing behaviour, kept.
    let f = zoomed(&mut scene, 0x24);
    assert!(!tex_runs(&scene, &f, 0x0300_0034).is_empty(), "the sniper draws the scope");
    // The watch (0x54, the zoomable gadget): the camera viewfinder, NOT the scope.
    let f = zoomed(&mut scene, 0x54);
    assert!(!tex_runs(&scene, &f, 0x0300_0055).is_empty(), "the watch draws the camera viewfinder");
    assert!(tex_runs(&scene, &f, 0x0300_0034).is_empty(), "no sniper scope on the watch");
    // The OICW (0x1a): its zoom circle, not the scope.
    let f = zoomed(&mut scene, 0x1a);
    assert!(!tex_runs(&scene, &f, 0x0300_0141).is_empty(), "the OICW draws its zoom circle");
    assert!(tex_runs(&scene, &f, 0x0300_0034).is_empty(), "no sniper scope on the OICW");
    // The Ronin (0x32): the laser frame — six strips of the solid texture, no scope, no viewfinder.
    let f = zoomed(&mut scene, 0x32);
    assert!(!tex_runs(&scene, &f, 0x0300_0002).is_empty(), "the Ronin draws its laser frame strips");
    assert!(tex_runs(&scene, &f, 0x0300_0034).is_empty(), "no sniper scope on the Ronin");
    assert!(tex_runs(&scene, &f, 0x0300_0055).is_empty(), "no viewfinder on the Ronin");
    // Sister: unzoomed, no overlay at all.
    let f = zoomed(&mut scene, 0x24);
    scene.sim.as_mut().unwrap().zoom_target = 1.0;
    scene.sim.as_mut().unwrap().zoom = 1.0;
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    assert!(tex_runs(&scene, &f, 0x0300_0034).is_empty(), "no scope unzoomed");
    assert!(tex_runs(&scene, &f, 0x0300_0055).is_empty(), "no viewfinder unzoomed");
}

/// The selection itself, per weapon id (the game's switch at 0x19e6d0:112-147): 0x1a/0x1b → OICW,
/// 0x32/0x33 → Laser, 0x54/0x55 → Camera, everything else → the Sight scope. The 0x52/0x53 mapping
/// changed in fix-zoom-overlays: the name table [L ee_start_05/07000001/0700000a weapon_data+0x38
/// over USATxt.dat] names them "Phoenix Ronin" (the Micro-Camera is 0x54/0x55), and the switch has
/// no case 0x38/0x39 — they fall to the default scope. Pure — no ISO.
#[test]
fn the_zoom_overlay_selection_follows_the_weapon_id() {
    use nf_view::hud::{zoom_overlay, ZoomOverlay};
    assert_eq!(zoom_overlay(0x24), ZoomOverlay::Sight, "the sniper keeps the scope");
    assert_eq!(zoom_overlay(0x02), ZoomOverlay::Sight, "any other scoped weapon falls to the scope");
    assert_eq!(zoom_overlay(0x1a), ZoomOverlay::Oicw);
    assert_eq!(zoom_overlay(0x1b), ZoomOverlay::Oicw, "the OICW twin variant");
    assert_eq!(zoom_overlay(0x32), ZoomOverlay::Laser);
    assert_eq!(zoom_overlay(0x33), ZoomOverlay::Laser, "the Ronin twin variant");
    assert_eq!(zoom_overlay(0x54), ZoomOverlay::Camera);
    assert_eq!(zoom_overlay(0x55), ZoomOverlay::Camera, "the watch's taser twin (the game rewrites it to 0x54)");
    assert_eq!(zoom_overlay(0x52), ZoomOverlay::Sight, "the Phoenix Ronin — the switch's default (fix-zoom-overlays)");
    assert_eq!(zoom_overlay(0x53), ZoomOverlay::Sight, "the Phoenix Ronin twin");
}
