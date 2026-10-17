//! scorpion-rocket-speed: the AT-600 Scorpion's rockets fly at the game's flight speed — playtest
//! 1016-1355 (MP): "Scorpion rockets are way too slow".
//!
//! The Sentinel's flight scale was measured live: the 'Unguided' 0x2d's BU+0xF8 reads 0.33666
//! u/frame, constant over the flight, = (+0x20 0.3333 [L] × the 0x200 quarter [M `Bullet_init`
//! 0x12c5a0:207-211]) × 4.04 [L PCSX2 2026-10-14, docs/live/2026-10-14-weapon-measurements-pcsx2.md;
//! fix-sentinel-camera-play]. The Scorpion's twins 0x2E/0x2F ride the same mover with the same
//! flight-relevant record shape [L ee_start_05.bin / ee_snapshot.bin weapon_data @0x2bf150 +
//! w·0x10C, identical in all four snapshots]:
//!
//! - 0x2e 'Heatseeker': +0x20 = 0.25, +0x6C = 0x00032200 — the 0x200 quarter, the 0x20000 homing,
//!   no 0x4 guided cam;
//! - 0x2f 'Unguided':    +0x20 = 0.3333, +0x6C = 0x0001a200 — byte-identical to the measured 0x2d's.
//!
//! Neither the homing bit nor the guided-cam bit is read by the per-tick step: the state-1 mover
//! moves dir·speed [M `Bullet_update` 0x12e7b0:157,380-474], `Bullet_homing` 0x12f358 steers the
//! direction only and never writes BU+0xF8 [M :126-166], and the guided cam is init-time only [M
//! `Bullet_init` 0x12c5a0:234-254]. So the ×4.04 scale applies: predicted flight 0x2e = 0.2525,
//! 0x2f = 0.336667 u/tick (the 0x2d's measured value — its flags are identical). The port flew
//! them init-only (0.0625 / 0.0833) — 4.04× slow, the user's complaint to the digit.
//!
//! Through the REAL input path (the give, the switch and the fire ride the window's `Input`
//! lanes through `Sim::step_bond`), like the sister `sentinel_camera_live.rs`. Opt-in via NF_ISO.

use nf_game::player::Input;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn step(scene: &mut Scene, input: Input, trigger: bool) {
    scene.sim.as_mut().unwrap().step_bond(input, trigger, false, false);
}

/// The give + the REAL switch lane (`Input::switch_to`) + the draw wait — the twins share the
/// base's clip slot [M 0x1a1988:18-22]: the 0x2f's rounds live in clip[0x2e].
fn give_and_select(scene: &mut Scene, weapon: u8) {
    {
        let sim = scene.sim.as_mut().unwrap();
        let owned = sim.drones.inventory.owned.len();
        if (weapon as usize) < owned { sim.drones.inventory.owned[weapon as usize] = true; }
        let slot = if weapon == 0x2f { 0x2e } else { weapon as usize };
        if let Some(c) = sim.drones.inventory.clip.get_mut(slot) { *c = 4; }
    }
    step(scene, Input { switch_to: Some(weapon), ..Input::default() }, false);
    for _ in 0..90 {
        step(scene, Input::default(), false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.gun.weapon == weapon && sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
    }
    for _ in 0..10 { step(scene, Input::default(), false); }
}

/// The Scorpion's rockets leave at the record's quarter speed and ramp up 2% of +0x20 per tick
/// [M Bullet_init 0x12c5a0:207-211; Bullet_DoTrails 0x12d890:134-138] to the game's flight speed:
/// the Heatseeker 0x2e cruises at 0.0625 + 38 × 0.005 = 0.2525 u/tick and the Unguided 0x2f at
/// 0.0833 + 38 × 0.00667 = 0.336667 (= the measured 0x2d's flight [L PCSX2 2026-10-14] — its +0x6C
/// is byte-identical [L]) — not the init-only 0.0625/0.0833 the playtest called "way too slow".
/// Sisters: the homing bit lands on the bullet the game's way (0x2e true, 0x2f false [L +0x6C 0x32200
/// vs 0x1a200]), and neither twin rides the camera (no 0x4 [L]). (rocket-ramp: the old pin read an
/// early-flight displacement against a constant ×4.04 stand-in.)
#[test]
fn the_scorpion_rockets_fly_at_the_game_flight_speed() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (weapon, want, want_homing) in [
        (0x2eu8, 0.2525f32, true),
        (0x2fu8, 0.336_666_6f32, false),
    ] {
        let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x2e, 0x2f]).expect("load 07000005 with the Scorpion merged (the --give path)");
        for _ in 0..30 { step(&mut scene, Input::default(), false); }
        give_and_select(&mut scene, weapon);
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.gun.weapon, weapon, "the switch lane landed on {weapon:#x}");
        let before = sim.drones.player_bullets.len();
        step(&mut scene, Input { fire: true, ..Input::default() }, true);
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.player_bullets.len() > before, "the shot spawned a bullet");
        let b = sim.drones.player_bullets.last().unwrap();
        assert_eq!(b.weapon, weapon as u16);
        // The record's homing bit lands [L +0x6C: 0x2e 0x32200, 0x2f 0x1a200] and neither twin
        // rides the camera (no 0x4 — only the Sentinel's 0x2c does).
        assert_eq!(b.homing, want_homing, "weapon {weapon:#04x}'s +0x6C 0x20000 homing bit");
        assert!(!b.guided_cam, "weapon {weapon:#04x} has no 0x4 guided-cam bit");
        // Ride until the ramp is done (38 ticks) or the rocket dies; the speed field is BU+0xF8.
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
        eprintln!("[scorpion] weapon {weapon:#04x}: speed {last:.5} after {steps} ticks (want {want:.5})");
        assert!(steps >= 40, "the rocket flew {steps} ticks — too short to reach its cruise");
        assert!((last - want).abs() < 1e-4,
            "the {weapon:#04x} rocket cruises at {last:.5} u/tick, the game's flight speed is {want:.5} (master's init-only speed was 4.04× slower — the playtest's complaint)");
    }
}
