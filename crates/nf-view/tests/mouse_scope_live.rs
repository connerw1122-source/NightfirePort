//! The mouse's scope button (playtest 1014-1725, zoom2: "Scope doesn't work, you broke it. Right
//! clicking literally does nothing.").
//!
//! lib.rs mapped the right mouse button to `Input::zoom` only (PLAN.md D-002: the mouse aims directly, no
//! free-aim cursor). fix-aim-raise made the zoom start on the aim BIT's rise (`Player_Weapon` 0x1a24c8:
//! 156-187, the bit set at the raise script's end [M 0x1a2e38:554-561]), and only `Input::aiming`
//! drove that bit — so with the mouse the scope never raised and never zoomed. The fix: the mouse button
//! also sets `Input::scope`, which drives the aim bit. These tests drive the mouse path (`scope` + `zoom`
//! held, `aiming` never set) through `Sim::step_bond` and require: the raise plays,
//! the zoom leaves 1.0 after it, turning still turns the VIEW (no free-aim cursor window for the mouse),
//! and releasing the button zooms back out. Sister: the pad path (`aiming`) still enters free-aim.

use nf_game::player::Input;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[0x02, 0x1a, 0x1b, 0x24]).expect("load 07000005");
    for _ in 0..7200 {
        if scene.sim.as_ref().map_or(false, |s| !s.disabled) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert!(scene.sim.as_ref().map_or(false, |s| !s.disabled), "07000005 never enabled (intro stuck)");
    scene
}

fn idle(scene: &Scene) -> bool {
    let s = scene.sim.as_ref().unwrap();
    s.gun.switch_state == 0 && s.gun.reload_state == 0 && !s.gun.fire_state
}

fn give_and_settle(scene: &mut Scene, id: u8) {
    let (status, sw) = nf_view::panel::apply(scene, &[
        ("do".to_string(), "weapon".to_string()),
        ("id".to_string(), format!("0x{id:02x}")),
        ("rounds".to_string(), "20".to_string()),
    ]);
    assert_eq!(sw, Some(id), "give refused: {status}");
    for _ in 0..60 {
        if scene.sim.as_ref().unwrap().gun.weapon == id { break; }
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the switch never landed");
    for _ in 0..7200 {
        let s = scene.sim.as_ref().unwrap();
        if idle(scene) && s.view_script == s.weapons.idle_script(id as usize) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
}

/// Holds the mouse's scope button (zoom only) for `n` ticks; returns the zoom after.
fn hold_mouse_scope(scene: &mut Scene, n: usize) -> f32 {
    for _ in 0..n {
        scene.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, ..Default::default() }, false, false, false);
    }
    scene.sim.as_ref().unwrap().zoom
}

#[test]
fn the_mouse_scope_button_raises_and_zooms_the_sniper_and_the_aims20() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for id in [0x24u8, 0x1a] {
        let mut scene = load(&iso);
        give_and_settle(&mut scene, id);
        assert_eq!(scene.sim.as_ref().unwrap().zoom, 1.0, "{id:#04x}: unzoomed before the press");
        // The raise (19 ticks for 0x24, 16 for 0x1a at 1.25 [fix_aim_raise_live's census]) then the zoom ramp.
        let z = hold_mouse_scope(&mut scene, 90);
        let s = scene.sim.as_ref().unwrap();
        assert!(s.aiming, "{id:#04x}: the mouse scope button never raised the aim bit");
        assert!(z > 1.5, "{id:#04x}: the mouse scope button never zoomed (zoom {z})");
        assert!(!s.free_aim, "{id:#04x}: the mouse must not enter the free-aim cursor window (D-002)");
        // Turning while scoped turns the VIEW, not a cursor.
        let yaw0 = s.bond.yaw;
        for _ in 0..10 {
            scene.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, turn: 0.02, ..Default::default() }, false, false, false);
        }
        let yaw1 = scene.sim.as_ref().unwrap().bond.yaw;
        assert!((yaw1 - yaw0).abs() > 0.05, "{id:#04x}: turning while scoped must turn the view (yaw {yaw0} → {yaw1})");
        // Release: the bit drops and the zoom returns toward 1.
        for _ in 0..90 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
        let s = scene.sim.as_ref().unwrap();
        assert!(!s.aiming, "{id:#04x}: the aim bit stays up after release");
        assert!(s.zoom < 1.05, "{id:#04x}: the zoom never came back out ({})", s.zoom);
    }
}

/// Sister: the pad's aim button (`aiming`) still enters free-aim once the raise ends — the split keeps the
/// pad path as it was.
#[test]
fn the_pad_aim_button_still_enters_free_aim() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_settle(&mut scene, 0x02);
    for _ in 0..10 {
        scene.sim.as_mut().unwrap().step_bond(Input { aiming: true, ..Default::default() }, false, false, false);
    }
    let s = scene.sim.as_ref().unwrap();
    assert!(s.aiming && s.free_aim, "the P2K (no raise script) aims at once on the pad button and enters free-aim");
}
