//! fix-lighting, first deliverable: the muzzle-flash light (`Player_MuzzleFlash` 0x1a4298 →
//! `Light_Create` 0x1cc3d8) now lives in the simulation's `LightList` — the create's `Rand_Rand(0)`
//! draw rides the shared RNG in the game's order, the record carries the weapon's radius (+0x58),
//! colour (+0x56/+0x55/+0x54) × intensity 2.0 and kind 1, and `Light_Update` (0x1cafb8) deletes it
//! one tick later (life 0 → +0x20 = 1). The `Gas_Init` smoke draw rides the same order. Opt-in: NF_ISO.

use nf_view::scene::ViewOpts;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn rng_of(r: &nf_game::drone::Rng) -> nf_game::drone::Rng { nf_game::drone::Rng { x: r.x, y: r.y } }

#[test]
fn muzzle_light_lives_one_tick_with_the_weapon_values() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // Sister: no shot, no light.
    assert!(!scene.sim.as_ref().unwrap().drones.lights.lights.iter().any(|l| l.kind == 1), "no light without a shot");
    let rng_before = rng_of(&scene.sim.as_ref().unwrap().drones.rng);
    scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.shot_tick && sim.last_shot.is_some(), "the P2K fired this tick");
        // The shot's draws ride the shared RNG: Light_Create's `Rand_Rand(0)` and Gas_Init's
        // `Rand_FRand(2π)` sit between the accuracy draws and the bullet's spread draws (the
        // game's order; the exact draw sequence is unit-tested in nf-game's light_selection).
        let rng_after = rng_of(&sim.drones.rng);
        assert!((rng_after.x, rng_after.y) != (rng_before.x, rng_before.y), "the shot drew from the shared RNG");
        // The record: the weapon's values [M 0x1a4370-0x1a43a8].
        let l = sim.drones.lights.lights.iter().find(|l| l.kind == 1).expect("the muzzle light");
        let rec = &sim.weapons.records[sim.gun.weapon as usize];
        let radius = f32::from_le_bytes([rec[0x58], rec[0x59], rec[0x5a], rec[0x5b]]);
        assert!((l.radius - radius).abs() < 1e-6, "radius = weapon_data+0x58");
        assert!((l.intensity - 2.0).abs() < 1e-6, "intensity 2.0");
        assert_eq!(l.rgb, [rec[0x56], rec[0x55], rec[0x54]], "rgb = +0x56/+0x55/+0x54");
        assert_eq!(l.life, 1, "life arg 0 → +0x20 = 1 (Light_Update deletes it next tick)");
        assert!(l.active, "created active (+0x48 = 1)");
        // The light sits at the muzzle, near the eye (the sim's eye + aim·0.5 stand-in [S]).
        let eye = sim.bond.head();
        let d = ((l.pos[0] - eye[0]).powi(2) + (l.pos[1] - eye[1]).powi(2) + (l.pos[2] - eye[2]).powi(2)).sqrt();
        assert!(d < 2.0, "the light sits at the gun's muzzle, near the eye: {d}");
        // The frame's resolved view carries it for the renderer.
        let view = scene.frame_lights();
        assert!(view.lights.iter().any(|v| (v.pos[0] - l.pos[0]).abs() < 1e-4 && v.inv_r2 == 1.0 / (radius * radius)),
            "frame_lights resolves the muzzle light for the draws");
    }
    // One tick later it is gone (Light_Update deletes a life-1 light) and the shot flag is clear.
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    assert!(!scene.sim.as_ref().unwrap().drones.lights.lights.iter().any(|l| l.kind == 1), "the light lasts one tick");
    let _ = ViewOpts::default();
}

/// A smoke puff appeared with the shot and rides the same RNG order.
#[test]
fn the_shot_leaves_a_smoke_puff() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    assert!(!scene.sim.as_ref().unwrap().smoke.is_empty(), "Gas_Init made a smoke puff (the P2K has no 0x2000 flag)");
    // Sister: no shot, no puff.
    let mut calm = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..31 { calm.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert!(calm.sim.as_ref().unwrap().smoke.is_empty(), "no smoke without a shot");
}
