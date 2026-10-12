//! fix-lighting, first deliverable: the muzzle-flash light (`Player_MuzzleFlash` 0x1a4298 → `Light_Create`). It is
//! active on the tick of Bond's shot only (life 0 → `Light_Update` deletes it next tick), at the view model's datum
//! 0, with the weapon's radius (+0x58) and colour (+0x56/+0x55/+0x54) × intensity 2.0. Opt-in: NF_ISO.

use nf_view::scene::ViewOpts;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn muzzle_light_lives_one_tick_with_the_weapon_values() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let opts = ViewOpts::default();
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // Sister: no shot, no light.
    assert_eq!(scene.dyn_light(30.0, opts)[7], 0.0, "no light without a shot");
    scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.shot_tick && sim.last_shot.is_some(), "the P2K fired this tick");
    let l = scene.dyn_light(31.0, opts);
    eprintln!("light {l:?}");
    assert_eq!(l[7], 1.0, "the muzzle light is on during the shot tick");
    let rec = &sim.weapons.records[sim.gun.weapon as usize];
    let radius = f32::from_le_bytes([rec[0x58], rec[0x59], rec[0x5a], rec[0x5b]]);
    assert!((l[3] - 1.0 / (radius * radius)).abs() < 1e-6, "1/r² from weapon_data+0x58");
    assert!((l[4] - rec[0x56] as f32 * 2.0 / 255.0).abs() < 1e-6, "red from +0x56 × 2.0");
    let eye = sim.bond.head();
    let d = ((l[0] - eye[0]).powi(2) + (l[1] - eye[1]).powi(2) + (l[2] - eye[2]).powi(2)).sqrt();
    assert!(d < 2.0, "the light sits at the gun's muzzle, near the eye: {d}");
    // One tick later it is gone (Light_Update deletes a life-1 light).
    scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    assert_eq!(scene.dyn_light(32.0, opts)[7], 0.0, "the light lasts one tick");
}
