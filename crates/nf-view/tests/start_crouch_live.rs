//! fix-start-crouch: a level start whose placement param 3 is 3 starts Bond crouched
//! (`Player_Init` 0x1910f8:172-176 → `Player_StandAtNewPosition(.., 4)` + `BLData+0x90C = 0.45`) [M].
//! 07000014 starts inside its crawl vent this way [L: PCSX2 at the start — substate 4, BL+0x90C 0.45,
//! Bond at (40.82, 3.01, 65.15)]. Standing there put the eye above the duct roof: the view showed the
//! duct's top and the gun was buried in it.
use nf_view::load;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn settle(scene: &mut nf_view::scene::Scene) {
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input::default(), false, false, false);
    }
}

#[test]
fn level_14_starts_crouched_inside_the_vent() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Some(iso) = iso() else { return };
    let mut scene = load(&iso, "07000014.bin", false).expect("load 07000014");
    assert!(scene.sim.as_ref().unwrap().bond.crouched, "07000014 starts crouched (placement p3 == 3)");
    settle(&mut scene);
    let sim = scene.sim.as_ref().unwrap();
    let b = &sim.bond;
    assert!(b.crouched, "still crouched after 2 s with no input (no headroom to stand)");
    assert!((b.crouch_drop - 0.45).abs() < 1e-4, "the full crouch eye drop, got {}", b.crouch_drop);
    // The measured PS2 start (x, z within the placement; y the crouched rest).
    assert!((b.pos[0] - 40.82).abs() < 0.1 && (b.pos[2] - 65.15).abs() < 0.1, "at the vent start, got {:?}", b.pos);
    assert!((b.pos[1] - 3.01).abs() < 0.1, "the crouched rest height, got {}", b.pos[1]);
    // The eye is inside the duct: something solid within 0.6 above it (the duct roof).
    let eye = b.head();
    assert!(!sim.world.line_of_sight(eye, [eye[0], eye[1] + 0.6, eye[2]]), "the duct roof sits just above the eye {eye:?}");
}

/// Sister: a level whose start has p3 == 0 still starts standing (the fix is keyed on p3, not on 14).
#[test]
fn level_05_starts_standing() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Some(iso) = iso() else { return };
    let mut scene = load(&iso, "07000005.bin", false).expect("load 07000005");
    settle(&mut scene);
    let b = &scene.sim.as_ref().unwrap().bond;
    assert!(!b.crouched && b.crouch_drop == 0.0, "07000005 starts standing");
}
