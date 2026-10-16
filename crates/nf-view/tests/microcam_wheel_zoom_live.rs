//! microcam-wheel-zoom (the 2026-10-15 playtest, results id 1015-1738):
//! "It'd be cool if we could add some zoom to the micro camera and have the scroll wheel do the
//! zoom while you're right-clicking to aim in. And if you're not right-clicking, of course,
//! weapon switch."
//!
//! While the aim (right) button is held, a wheel notch steps the zoom target ×2 / ÷2 within the
//! held weapon's own range [1.0, weapon_data+0x88] — the max `Player_Weapon` 0x1a24c8:156-187
//! writes to BL+0x8D8 [M]; the ×2 step is the port's own [S] (the game's pad zoom is continuous:
//! `Player_Zoom` 0x1a1a70 ramps the held target ×1.025/tick). Once the wheel has stepped, the
//! hold-ramp to max yields until the aim falls (which resets the target to 1.0 [M the aim-bit
//! arm] and the latch with it). The window routes the notch into `Input.zoom_cycle` when the
//! held weapon can zoom (`wheel_steps_zoom`), else into `weapon_cycle` as before.
//!
//! Everything here runs through the real input path (`nf_view::load` + `Sim::step_bond` with the
//! same `Input` the window loop builds). Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The sniper's record pre-merged so the draw's fp skin resolves (the fire_modes3 recipe).
const SNIPER: &[u8] = &[0x24];

fn load_sniper(iso: &str) -> nf_view::scene::Scene {
    nf_view::load_with_carry_weapons(iso, "07000005.bin", false, None, SNIPER).expect("load 07000005")
}

/// Hold the right button (scope+zoom, the window's mouse-first mapping) until the aim bit is up
/// and the zoom target has settled on `expected`; returns the tick count.
fn hold_aim_until_target(w: &mut nf_view::scene::Scene, expected: f32) -> usize {
    for t in 0..400 {
        w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, ..Input::default() }, false, false, false);
        let sim = w.sim.as_ref().unwrap();
        if sim.aiming && (sim.zoom_target - expected).abs() < 1e-4 { return t; }
    }
    panic!("the aim/zoom never settled on {expected} (target {:?}, aiming {:?})",
        w.sim.as_ref().unwrap().zoom_target, w.sim.as_ref().unwrap().aiming);
}

/// One held tick carrying a wheel notch (the window mem::takes `zoom_cycle` into the tick's Input).
fn wheel(w: &mut nf_view::scene::Scene, notch: i8) {
    w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, zoom_cycle: notch, ..Input::default() }, false, false, false);
}

/// THE SNIPER'S RANGE: hold-to-aim zooms to the weapon's max (unchanged), then the wheel steps the
/// target ×2 / ÷2 between 1.0 and weapon_data+0x88 (0x24 = 10.0 [L weapon_data: flags 0x4080d0]),
/// the hold-ramp yields while a stepped notch is in force, and the aim's fall resets both.
#[test]
fn the_wheel_steps_the_zoom_between_the_weapon_min_and_max() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = load_sniper(&iso);
    // Own the sniper + ammo, then draw it through the REAL switch, and settle until the draw is
    // done (a raise queued mid-swap is refused and a held button never re-edges — fire_modes3's
    // arm_record + draw recipe).
    {
        let sim = w.sim.as_mut().unwrap();
        if (0x24 as usize) < sim.drones.inventory.owned.len() { sim.drones.inventory.owned[0x24] = true; }
        if let Some(c) = sim.drones.inventory.clip.get_mut(0x24) { *c = 5; }
    }
    for _ in 0..60 {
        w.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x24), ..Input::default() }, false, false, false);
        if w.sim.as_ref().unwrap().gun.weapon == 0x24 { break; }
    }
    for _ in 0..200 {
        let sim = w.sim.as_ref().unwrap();
        if sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
        w.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    }
    for _ in 0..5 { w.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(w.sim.as_ref().unwrap().gun.weapon, 0x24, "the sniper equipped");
    // Hold the right button: the plain hold-ramp still zooms to the weapon's max (the pad's zoom
    // button keeps the same arm; the wheel is purely additive).
    hold_aim_until_target(&mut w, 10.0);
    for _ in 0..250 { wheel(&mut w, 0); } // let the zoom chase the target up (~1.025×/tick)
    assert!(w.sim.as_ref().unwrap().zoom > 9.0, "zoom rose to the max, got {}", w.sim.as_ref().unwrap().zoom);

    // Wheel down: the target halves and STAYS there (the hold-ramp yields) while the button is held.
    wheel(&mut w, -1);
    assert_eq!(w.sim.as_ref().unwrap().zoom_target, 5.0, "one notch down = half the max");
    for _ in 0..40 { wheel(&mut w, 0); }
    let sim = w.sim.as_ref().unwrap();
    assert!((sim.zoom_target - 5.0).abs() < 1e-4, "the hold-ramp must not push past the notch, got {}", sim.zoom_target);
    assert!(sim.zoom > 5.0 && sim.zoom < 6.0, "the zoom chases the stepped target down, got {}", sim.zoom);

    // More notches walk the range: down to 2.5, back up through 5 to the 10 clamp.
    wheel(&mut w, -1);
    assert!((w.sim.as_ref().unwrap().zoom_target - 2.5).abs() < 1e-4);
    wheel(&mut w, 1);
    assert!((w.sim.as_ref().unwrap().zoom_target - 5.0).abs() < 1e-4);
    wheel(&mut w, 1);
    assert!((w.sim.as_ref().unwrap().zoom_target - 10.0).abs() < 1e-4, "two notches up from 5 clamps at the max");
    wheel(&mut w, 1);
    assert!((w.sim.as_ref().unwrap().zoom_target - 10.0).abs() < 1e-4, "the max is a ceiling");

    // Releasing the aim resets the target to 1.0 [M the aim-bit arm] and clears the latch: the next
    // hold ramps to the max again (the plain behaviour). The lower anim must finish before the
    // re-press (a raise queued mid-anim is refused and a held button never re-edges), so idle first.
    for _ in 0..3 { w.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(w.sim.as_ref().unwrap().zoom_target, 1.0, "the aim's fall resets the zoom target");
    assert!(!w.sim.as_ref().unwrap().zoom_wheel, "the wheel latch cleared with the aim");
    for _ in 0..200 {
        let sim = w.sim.as_ref().unwrap();
        if !sim.aiming && sim.view_aim_anim.is_none() { break; }
        w.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    }
    hold_aim_until_target(&mut w, 10.0);
}

/// THE MICRO-CAMERA'S OWN RANGE (0x54, zoom 8 [L weapon_data: flags 0x20e0]): the same wheel
/// stepping through the REAL number-key switch, with the viewfinder pane (CameraPane 11) enabled
/// while the target is above 1 and hidden again at exactly 1 (`camera_scan_roots`'s gate).
#[test]
fn the_micro_camera_steps_through_its_own_range() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..10 { w.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    // Equip the Micro-Camera through the real switch, settle the swap (the brackets test's recipe).
    for _ in 0..4 { w.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x54), ..Input::default() }, false, false, false); }
    for _ in 0..90 { w.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(w.sim.as_ref().unwrap().gun.weapon, 0x54, "the micro camera equipped");

    // Hold the right button: the micro camera zooms to ITS max (8) — "add some zoom to the micro
    // camera" is already the hold arm; the wheel then steps within [1, 8].
    hold_aim_until_target(&mut w, 8.0);
    let roots = |s: &nf_view::scene::Sim| nf_view::hud::camera_scan_roots(s).len();
    assert!(roots(w.sim.as_ref().unwrap()) > 0, "the viewfinder brackets are up at the max");

    // Wheel down: 8 → 4 → 2, brackets still up; one more reaches the 1.0 floor and the pane hides;
    // wheel up brings it back.
    wheel(&mut w, -1);
    assert!((w.sim.as_ref().unwrap().zoom_target - 4.0).abs() < 1e-4);
    assert!(roots(w.sim.as_ref().unwrap()) > 0, "the brackets stay up at 4");
    wheel(&mut w, -1);
    assert!((w.sim.as_ref().unwrap().zoom_target - 2.0).abs() < 1e-4);
    wheel(&mut w, -1);
    assert!((w.sim.as_ref().unwrap().zoom_target - 1.0).abs() < 1e-4, "the min is 1.0 (no zoom)");
    for _ in 0..10 { wheel(&mut w, 0); }
    assert_eq!(roots(w.sim.as_ref().unwrap()), 0, "the pane hides at the zoom floor");
    wheel(&mut w, 1);
    assert!((w.sim.as_ref().unwrap().zoom_target - 2.0).abs() < 1e-4);
    assert!(roots(w.sim.as_ref().unwrap()) > 0, "the pane returns above the floor");
}

/// NOT RIGHT-CLICKING: the wheel keeps switching weapons — the `weapon_cycle` lane is untouched,
/// and a notch aimed at a weapon that cannot zoom is dropped by the zoom block (the window would
/// have routed it to `weapon_cycle`).
#[test]
fn the_wheel_keeps_switching_weapons_when_not_aiming() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut w = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..10 { w.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let start = w.sim.as_ref().unwrap().gun.weapon;
    // The wheel without the aim button: a switch request goes out [M Player_WeaponChange lane].
    w.sim.as_mut().unwrap().step_bond(Input { weapon_cycle: 1, ..Input::default() }, false, false, false);
    assert!(w.sim.as_ref().unwrap().gun.switch_request != 0, "the wheel switched weapons (no aim held)");
    for _ in 0..90 { w.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_ne!(w.sim.as_ref().unwrap().gun.weapon, start, "the switch landed");
    assert!((w.sim.as_ref().unwrap().zoom - 1.0).abs() < 1e-4, "no zoom without the aim button");

    // A zoom notch on a weapon that cannot zoom (the loadout gun, no scope flag) does nothing.
    let w0 = w.sim.as_ref().unwrap().gun.weapon;
    let scoped = {
        let s = w.sim.as_ref().unwrap();
        s.weapons.flags(w0 as usize) & 0x40 != 0
    };
    if !scoped {
        w.sim.as_mut().unwrap().step_bond(Input { scope: true, zoom: true, zoom_cycle: -1, ..Input::default() }, false, false, false);
        assert!((w.sim.as_ref().unwrap().zoom_target - 1.0).abs() < 1e-4, "no zoom on a non-scoped weapon");
    }
}

/// The window's routing predicate with the disc's own weapon_data values [L ACTION.ELF 0x2bf150].
#[test]
fn the_window_routes_the_wheel_by_the_weapons_zoom() {
    use nf_view::wheel_steps_zoom;
    // The sniper 0x24 / the micro camera 0x54: scoped with a magnification above 1.
    assert!(wheel_steps_zoom(true, 0x4080d0, 10.0));
    assert!(wheel_steps_zoom(true, 0x20e0, 8.0));
    // Not right-clicking: weapon switch, whatever the weapon.
    assert!(!wheel_steps_zoom(false, 0x4080d0, 10.0));
    // Scoped weapons the zoom block ignores (magnification 1) still switch.
    assert!(!wheel_steps_zoom(true, 0x40, 1.0));
    // A zoom above 1 without the scope flag (rec 0x18: zoom 3, flags 0x8) never zooms.
    assert!(!wheel_steps_zoom(true, 0x8, 3.0));
}
