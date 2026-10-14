//! fix-scope-zoom: the zoom levels, the persistence and the aim+scoped gates.
//!
//! The game's zoom [M `Player_Weapon` 0x1a24c8:154-187 + `Player_Zoom` 0x1a1a70 +
//! `Player_InitAmmoWeapons` 0x1a17f8:51]:
//! • aim start targets the BASE weapon's slot (BL+0x1B4 + base·0xC), filled with `sqrt(+0x88)` at
//!   init and rewritten with the reached zoom while the axis is held — re-aiming resumes there;
//! • the zoom axis (action 6) scales target and value ×1.025 per tick up to the record's +0x88;
//! • releasing the axis KEEPS the zoom; releasing AIM targets 1.0 (the value eases ×1.15 per tick);
//! • clamps: target ≤ +0x88, target < 1 → 1, target > 50 → 50.
//! The overlay / crosshair / view-model gates are aim + scoped (`HUD_UpdateCrossHair` 0x19e6d0:106-153,
//! `Player_PositionCamera` 0x18f5d8:100-107) — no zoom value in any of them.
//! The per-weapon numbers below are [L ee_start_05.bin weapon_data 0x2bf150] (+0x02 base, +0x88 f32).
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Quads of one texture in the frame: (first vertex, count) runs.
fn tex_runs(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, hash: u32) -> Vec<(usize, usize)> {
    let Some(ti) = scene.hud_textures.iter().position(|(h, _)| *h == hash) else { return Vec::new() };
    f.runs.iter().filter(|r| r.0 == ti + 1).map(|r| (r.2 as usize, r.3 as usize)).collect()
}

/// (weapon id, aim-start zoom = sqrt(+0x88 of the BASE record), max zoom = +0x88)
/// [L ee_start_05.bin weapon_data 0x2bf150 — the census in the report].
const CASES: &[(u8, f32, f32)] = &[
    (0x11, 2.0, 4.0),        // the Delta Repeater (scoped)
    (0x18, 1.732_050_8, 3.0),// unscoped mag 3 — the axis zooms it with no overlay
    (0x1a, 2.449_489_8, 6.0),// the AIMS-20
    (0x1e, 3.162_277_6, 10.0),// the Winter Tactical Sniper
    (0x20, 4.0, 16.0),
    (0x22, 4.0, 16.0),
    (0x24, 3.162_277_6, 10.0),// the Winter Covert Sniper
    (0x26, 4.0, 16.0),
    (0x28, 4.0, 16.0),
    (0x2a, 1.414_213_5, 2.0),// unscoped mag 2
    (0x32, 1.870_828_7, 3.5),// the Phoenix Samurai (the laser pane)
    (0x54, 2.828_427_0, 8.0),// the Micro-Camera (Upgrade_Weapon 0x1a8f30 writes 16.0 on the upgrade)
];

/// Per weapon: the aim-start zoom lands on the game's sqrt(+0x88) default, the axis caps at +0x88,
/// releasing the axis KEEPS the zoom, releasing aim eases it back. Before the fix the port had no
/// slots and no persistence: the zoom snapped to +0x88 while held and back to 1.0 on release.
#[test]
fn zoom_levels_match_the_game_per_weapon() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (w, def, max) in CASES {
        let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.gun.weapon = *w;
            sim.gun.switch_request = *w; // clear the loadout's pending switch
            sim.cursor = [0.0, 0.0];
            sim.cross = [0.0, 0.0];
        }
        // Aim: the value ramps toward the slot default (sqrt of the base's magnification).
        for _ in 0..80 {
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, ..Default::default() }, false, false, false);
        }
        let sim = scene.sim.as_ref().unwrap();
        assert!((sim.zoom - def).abs() < 1e-3, "w {w:#x}: aim-start zoom {} != sqrt(+0x88) {def}", sim.zoom);
        assert!((sim.zoom_target - def).abs() < 1e-3, "w {w:#x}: aim-start target {}", sim.zoom_target);
        // Hold the axis: target and value ride ×1.025 per tick up to the record's magnification.
        for _ in 0..400 {
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, zoom: true, ..Default::default() }, false, false, false);
        }
        let sim = scene.sim.as_ref().unwrap();
        assert!((sim.zoom - max).abs() < 1e-4, "w {w:#x}: held zoom {} != +0x88 {max}", sim.zoom);
        // Release the axis, keep aiming: the zoom KEEPS (target = value [M 0x1a1a70:70-71]).
        // Before the fix the target dropped to 1.0 the moment the key released.
        for _ in 0..30 {
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, ..Default::default() }, false, false, false);
        }
        let sim = scene.sim.as_ref().unwrap();
        assert!((sim.zoom - max).abs() < 1e-4, "w {w:#x}: zoom {} drifted after the axis released (must hold {max})", sim.zoom);
        // The reached zoom persisted into the base's slot.
        let base = sim.weapons.base(*w as usize) as usize;
        assert!((sim.zoom_slots[base] - max).abs() < 1e-4, "w {w:#x}: slot[base {base}] = {} != {max}", sim.zoom_slots[base]);
        // Release aim: the value eases back toward 1.0.
        for _ in 0..60 {
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input::default(), false, false, false);
        }
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.zoom < 1.02, "w {w:#x}: zoom {} must ease back toward 1.0 after aim released", sim.zoom);
    }
}

/// The twins share the base's slot: zooming 0x1b (base 0x1a) to its max, then re-aiming 0x1a,
/// resumes at that zoom [M 0x1a24c8:173-177 reads the slot of record +0x02].
#[test]
fn the_twins_share_the_base_zoom_slot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x1b; // the AIMS-20 twin: base 0x1a [L weapon_data 0x1b+0x02 = 0x1a]
        sim.gun.switch_request = 0x1b;
        sim.cursor = [0.0, 0.0];
        sim.cross = [0.0, 0.0];
    }
    for _ in 0..400 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, zoom: true, ..Default::default() }, false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!((sim.zoom - 6.0).abs() < 1e-4, "0x1b held zoom {} != 6.0", sim.zoom);
    // Switch to the base 0x1a and re-aim: the slot 0x1b wrote is 0x1a's — the aim resumes at 6.0.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x1a;
        sim.gun.switch_request = 0x1a;
    }
    for _ in 0..80 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, ..Default::default() }, false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!((sim.zoom - 6.0).abs() < 1e-3, "0x1a re-aim zoom {} must resume at the shared 6.0", sim.zoom);
    // Sister: the slot is the BASE's — 0x1a's own slot now holds 6.0.
    assert!((sim.zoom_slots[0x1a] - 6.0).abs() < 1e-4, "slot[0x1a] = {}", sim.zoom_slots[0x1a]);
}

/// The overlay / crosshair gates follow aim + scoped, not the zoom value [M 0x19e6d0:106-153]:
/// aiming a scoped weapon draws its pane and hides the crosshair even while the zoom is still 1.0;
/// an unscoped zoomable (0x18) zooms with NO pane and the crosshair stays. 07000014 ships the scope,
/// viewfinder and OICW textures [D tex-list, fix-viewmodel-zoom].
#[test]
fn the_overlay_and_crosshair_gates_follow_aim_and_scoped() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000014.bin", false).expect("load");
    // Not aiming: the crosshair, no overlay.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x24;
        sim.gun.switch_request = 0x24;
    }
    scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input::default(), false, false, false);
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    assert!(!tex_runs(&scene, &f, 0x0300_0026).is_empty(), "the crosshair draws when not aiming");
    assert!(tex_runs(&scene, &f, 0x0300_0034).is_empty(), "no scope when not aiming");
    // Aim the sniper while the zoom is still ~1.0: the scope draws and the crosshair hides.
    scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, ..Default::default() }, false, false, false);
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    assert!(!tex_runs(&scene, &f, 0x0300_0034).is_empty(), "the scope draws on aim (the gate is aim+scoped, not the zoom value)");
    assert!(tex_runs(&scene, &f, 0x0300_0026).is_empty(), "the crosshair hides on aim of a scoped weapon");
    // Each zoomable's own overlay, one render each: the OICW circle, the viewfinder, the laser strips.
    // (Aim is released between weapons — the game's aim bit drops while a switch runs, so the next
    // aim re-reads the fresh slot.)
    for (w, tex, name) in [(0x1a_u8, 0x0300_0141_u32, "OICW circle"), (0x54, 0x0300_0055, "viewfinder"), (0x32, 0x0300_0002, "laser strips")] {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input::default(), false, false, false);
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = w;
        sim.gun.switch_request = w;
        sim.step_bond(nf_game::player::Input { aiming: true, ..Default::default() }, false, false, false);
        let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
        assert!(!tex_runs(&scene, &f, tex).is_empty(), "aimed {name} draws");
        assert!(tex_runs(&scene, &f, 0x0300_0034).is_empty(), "no sniper scope on {name}");
    }
    // The unscoped zoomable 0x18 (mag 3, no 0x40 flag): aiming it ZOOMS but draws no pane and
    // keeps the crosshair. (Fresh aim after the released tick above.)
    scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input::default(), false, false, false);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x18;
        sim.gun.switch_request = 0x18;
        sim.cursor = [0.0, 0.0];
        sim.cross = [0.0, 0.0];
    }
    for _ in 0..80 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, ..Default::default() }, false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!((sim.zoom - 1.732_050_8).abs() < 1e-2, "the unscoped 0x18 zooms while aimed, got {}", sim.zoom);
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    assert!(tex_runs(&scene, &f, 0x0300_0034).is_empty(), "no scope pane on the unscoped 0x18");
    assert!(!tex_runs(&scene, &f, 0x0300_0026).is_empty(), "the unscoped 0x18 keeps the crosshair");
}

/// The overlay selection census over every zoomable id (the switch at 0x19e6d0:112-147):
/// 0x1a/0x1b OICW, 0x32/0x33 (+ the MP twin 0x6f falls to the default) laser via 0x32/0x33 only,
/// 0x54/0x55 camera, everything else the Sight scope. Pure — no ISO.
#[test]
fn the_zoomable_census_selection() {
    use nf_view::hud::{zoom_overlay, ZoomOverlay};
    for w in [0x11_u8, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x52, 0x53, 0x6f] {
        assert_eq!(zoom_overlay(w), ZoomOverlay::Sight, "w {w:#x} falls to the default scope");
    }
    for w in [0x1a_u8, 0x1b] { assert_eq!(zoom_overlay(w), ZoomOverlay::Oicw, "w {w:#x}"); }
    for w in [0x32_u8, 0x33] { assert_eq!(zoom_overlay(w), ZoomOverlay::Laser, "w {w:#x}"); }
    for w in [0x54_u8, 0x55] { assert_eq!(zoom_overlay(w), ZoomOverlay::Camera, "w {w:#x}"); }
}
