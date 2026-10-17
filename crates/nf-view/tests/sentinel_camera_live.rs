//! fix-sentinel-camera-play: the Sentinel through the REAL input path — the give, the switch and
//! the fire all ride the window's own `Input` lanes (`switch_to`, `mode_switch`, `fire` through
//! `Sim::step_bond`), never the gun's internal setters.
//!
//! The user's playtest 1014-1725 (`nf-view <iso> <level> --dev`, 07000005 `--give 0x2c,0x2a`, and
//! 08): "The rocket launcher doesn't do the first person. The rockets are way too slow." Two
//! halves, both failing on master:
//!
//! 1. The rockets fly at the QUARTERED INIT speed only (0.0625 u/tick for the guided 0x2c) —
//!    the game's flight is 4.04× that: the 'Unguided' 0x2d's BU+0xF8 reads 0.33666 u/frame,
//!    constant [L PCSX2 2026-10-14, docs/live/2026-10-14-weapon-measurements-pcsx2.md]. The init
//!    arm is [M `Bullet_init` 0x12c5a0:207-211] (`+0x20`, ×0.25 on `+0x6C & 0x200`); the flight
//!    step is rewritten per tick (`Bullet_update` 0x12e7b0 at 0x12ecf8) and the object's control
//!    integrates the move — the port now applies the measured ×4.04 on the measured twins.
//! 2. The window never consulted `Sim::view_override()` (the ride's view [M
//!    `Player_PositionCamera` 0x18f5d8 case 0xc]) — the sim rode, the screen stayed with Bond —
//!    and the ride drew Bond's arms and the crosshair, which the game hides ("no view model, no
//!    crosshair" [L PCSX2]).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;
use nf_view::scene::{Scene, ViewOpts};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn step(scene: &mut Scene, input: Input, trigger: bool) {
    scene.sim.as_mut().unwrap().step_bond(input, trigger, false, false);
}

/// The `--give 0x2c,0x2a` setup: the weapon into the inventory with its spare, then the REAL
/// switch lane (`Input::switch_to` — the window's `weapon_switch_to`) and the draw wait.
fn give_and_select(scene: &mut Scene, weapon: u8, spare: u32) {
    let at = scene.sim.as_ref().map_or(0, |s| s.weapons.ammo_type(weapon as usize));
    {
        let sim = scene.sim.as_mut().unwrap();
        let owned = sim.drones.inventory.owned.len();
        if (weapon as usize) < owned { sim.drones.inventory.owned[weapon as usize] = true; }
        // The twins share the base's clip slot [M 0x1a1988:18-22]: the 0x2d's rounds live in clip[0x2c].
        let slot = if weapon == 0x2d { 0x2c } else { weapon as usize };
        if let Some(c) = sim.drones.inventory.clip.get_mut(slot) { *c = 4; }
        // The `--give 0x2c,0x2a` spare rides the ammo-type slot (`carried[ammo_type]`).
        if let Some(s) = sim.drones.inventory.carried.get_mut(at) { *s = spare as i16; }
    }
    step(scene, Input { switch_to: Some(weapon), ..Input::default() }, false);
    // Wait out the draw script (the merged assets play the real draw anim).
    for _ in 0..90 {
        step(scene, Input::default(), false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.gun.weapon == weapon && sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
    }
    for _ in 0..10 { step(scene, Input::default(), false); }
}

/// The Sentinel rockets leave at the record's QUARTER speed (+0x6C & 0x200 [M Bullet_init 0x12c5a0:
/// 207-211]) and gain 2% of +0x20 per tick while below it [M Bullet_DoTrails 0x12d890:134-138] —
/// ending at the MEASURED flight speed: 0x2c 0.0625 + 38 × 0.005 = 0.2525 u/tick and 0x2d 0.0833 +
/// 38 × 0.00667 = 0.33666 [L PCSX2 2026-10-14, BU+0xF8 in flight]. (rocket-ramp: the old pin read an
/// early-flight displacement against a constant ×4.04 stand-in; the PS2's slow launch is the plume
/// by the camera, PCSX2 side by side 2026-10-17.) Sister: the start is the quarter, not the cruise.
#[test]
fn the_sentinel_rockets_fly_at_the_measured_flight_speed() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // After the fire tick the rocket has had its first update: the state-0 arm falls through to the
    // shared tail that moves it and runs `Bullet_DoTrails` [M Bullet_update 0x12e7b0:115-148 → 374-475],
    // so one ramp step is already in: quarter + 2% of +0x20.
    for (weapon, start_want, want) in [(0x2cu8, 0.25f32 * 0.25 + 0.25 * 0.02, 0.2525f32), (0x2du8, 0.333_333_3 * 0.25 + 0.333_333_3 * 0.02, 0.336_666_6)] {
        let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x2c, 0x2d]).expect("load 07000005 with the Sentinel merged (the --give path)");
        for _ in 0..30 { step(&mut scene, Input::default(), false); }
        give_and_select(&mut scene, weapon, 0x2a);
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.gun.weapon, weapon, "the switch lane landed on {weapon:#x}");
        let before = sim.drones.player_bullets.len();
        step(&mut scene, Input { fire: true, ..Input::default() }, true);
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.player_bullets.len() > before, "the shot spawned a bullet");
        let b = sim.drones.player_bullets.last().unwrap();
        assert_eq!(b.weapon, weapon as u16);
        assert!((b.speed - start_want).abs() < 1e-5, "the {weapon:#04x} rocket starts at the quarter speed + one ramp step {start_want:.5}, got {:.5}", b.speed);
        // Ride until the ramp is done (38 ticks) or the rocket dies.
        let mut last = b.speed;
        let mut steps = 0u32;
        for _ in 0..60 {
            step(&mut scene, Input::default(), false);
            let sim = scene.sim.as_ref().unwrap();
            match sim.drones.player_bullets.last() {
                Some(bb) if !bb.dead && bb.weapon == weapon as u16 => { last = bb.speed; steps += 1; }
                _ => break,
            }
        }
        eprintln!("[sentinel] weapon {weapon:#04x}: speed {last:.5} after {steps} ticks (want {want:.5})");
        assert!(steps >= 40, "the rocket flew {steps} ticks — too short to reach its cruise");
        assert!((last - want).abs() < 1e-4,
            "the {weapon:#04x} rocket cruises at {last:.5} u/tick, the measured flight speed is {want:.5}");
    }
}

/// The guided ride takes the WINDOW's view: `view_override` = the live missile's pos/dir, Bond's
/// arms are hidden (`frame_state`'s ride hide — the palette collapse) — on master the arms drew
/// on (`hidden[vm] == false`) and nothing consulted the override. Sisters: the override tracks
/// the bullet for the whole ride, and the unguided twin never rides.
#[test]
fn the_guided_ride_takes_the_window_view_and_hides_the_arms() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x2c, 0x2d]).expect("load 07000005 with the Sentinel merged (the --give path)");
    for _ in 0..30 { step(&mut scene, Input::default(), false); }
    give_and_select(&mut scene, 0x2c, 0x2a);
    step(&mut scene, Input { fire: true, ..Input::default() }, true);
    // The ride is up: the view IS the missile's, and the arms are hidden.
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.missile_cam.is_some(), "the guided shot rides the camera");
    let b = sim.drones.player_bullets.last().expect("the rocket").clone();
    let (p, d) = sim.view_override().expect("the window's view override is Some during the ride");
    // The override is captured at the spawn (the same tick's drone move may have advanced the
    // bullet once — within a step of it).
    let dp = ((p[0] - b.pos[0]).powi(2) + (p[1] - b.pos[1]).powi(2) + (p[2] - b.pos[2]).powi(2)).sqrt();
    assert!(dp < 0.5, "the override's position is the missile's (delta {dp})");
    let dd = (d[0] - b.dir[0]).abs() + (d[1] - b.dir[1]).abs() + (d[2] - b.dir[2]).abs();
    assert!(dd < 0.05, "the override's direction is the missile's (delta {dd})");
    // The window contract (what the renderer consumes): the arms' palette collapses.
    let vm_ci = scene.view_models.get(&0x2c).copied().expect("07000005 has the Sentinel's view model (the dev merge)");
    let fs = scene.frame_state(None, 0.0, ViewOpts::default());
    assert!(fs.hidden[vm_ci], "the ride hides Bond's arms (master drew them on)");
    // The override tracks the live missile.
    for _ in 0..20 {
        step(&mut scene, Input::default(), false);
        let sim = scene.sim.as_ref().unwrap();
        let Some(b) = sim.drones.player_bullets.iter().find(|b| b.guided_cam && !b.dead) else { break };
        if let Some((p, _)) = sim.view_override() {
            // The override captures the missile's pos at the tick's start; the drones' move in the
            // same step_bond advances it once more — within a step (0.2525).
            let dp = ((p[0] - b.pos[0]).powi(2) + (p[1] - b.pos[1]).powi(2) + (p[2] - b.pos[2]).powi(2)).sqrt();
            assert!(dp < 0.5, "the override follows the missile (delta {dp})");
        }
    }
    // Wait out the flight + the 15-tick restore: the view returns to Bond.
    for _ in 0..2400 {
        step(&mut scene, Input::default(), false);
        if scene.sim.as_ref().unwrap().missile_cam.is_none() { break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.missile_cam.is_none(), "the ride ends (the flight + the 15-tick restore)");
    let fs = scene.frame_state(None, 0.0, ViewOpts::default());
    assert!(!fs.hidden[vm_ci], "the arms draw again after the ride");
}

/// The unguided twin 0x2d, reached through the REAL mode-switch lane (`Input::mode_switch` — the
/// window's B key), never rides the camera [L record 0x2d's +0x6C = 0x1a200, no 0x4].
#[test]
fn the_mode_switch_reaches_the_unguided_twin_and_it_never_rides() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x2c, 0x2d]).expect("load 07000005 with the Sentinel merged (the --give path)");
    for _ in 0..30 { step(&mut scene, Input::default(), false); }
    give_and_select(&mut scene, 0x2c, 0x2a);
    // The mode press: the twin switch through the input lane (the gate needs the view anim idle —
    // a real player presses again; retry until the twin lands).
    for _ in 0..6 {
        step(&mut scene, Input { mode_switch: true, ..Input::default() }, false);
        for _ in 0..25 { step(&mut scene, Input::default(), false); }
        if scene.sim.as_ref().unwrap().gun.weapon == 0x2d { break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.gun.weapon, 0x2d, "the mode press switched to the unguided twin [M 0x1a24c8:246-260]");
    // Wait out the twin's draw, then fire.
    for _ in 0..90 {
        step(&mut scene, Input::default(), false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
    }
    for _ in 0..10 { step(&mut scene, Input::default(), false); }
    // Fire: no ride.
    step(&mut scene, Input { fire: true, ..Input::default() }, true);
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.missile_cam.is_none(), "the unguided twin has no `+0x6C & 4` — the camera stays with Bond");
    assert!(sim.drones.player_bullets.last().is_some_and(|b| b.weapon == 0x2d), "the twin's rocket spawned");
}
