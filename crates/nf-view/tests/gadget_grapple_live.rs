//! fix-gadgets: the Q-Claw grapple through the real sim path.
//!
//! The game: the claw (0x4A/0x4C) out, the fire's bolt hits the class-0x51 point and
//! `Player_SetGrapplePoint` 0x195da0 enters the substate 2, phase 1; `Player_SSGrapple` 0x192860
//! reels Bond toward the point (the pull 0.05/tick up to 1.5 single-player [M 0x192860:96-104]),
//! hangs him under 2.0 from it — and the polarity is the game's own: the HANG HOLDS while the fire
//! is held [M 0x192860:57-62] and the button's RELEASE detaches [M 0x192860:64-66] (fix-gadgets2).
//! The bolt dies at weapon_data+0x1c = 5.0 for the 0x4a claw [M 0x12e7b0:158], so Bond stands close.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// 07000005's grapple point #282 (13.75, 32.29, 57.30 [D placement probe]): Bond stands ~4 units
/// away (inside the 5.0 bolt range [M 0x12e7b0:158 + weapon_data[0x4a]+0x1c]), equips the claw
/// through the gadget cycle, aims through the look input, and HOLDS the fire — the reel pulls him
/// to the point, the hang holds while held; the button's RELEASE puts him back in the walk substate.
#[test]
fn the_q_claw_reels_bond_to_the_grapple_point_and_releases() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    assert!(!sim.drones.grapple_points.is_empty(), "07000005 carries grapple points [D #281/#282]");
    assert!(*sim.drones.inventory.owned.get(0x4a).unwrap_or(&false), "the claw is owned at the level start [L]");
    // Stand ~4.1 units from point #282, on the ground below it.
    sim.bond.pos = [16.5, 29.6, 58.6];
    sim.bond.yaw = 0.0;
    let mut equipped = false;
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if sim.gun.weapon == 0x4a { equipped = true; break; }
    }
    assert!(equipped, "the claw must equip through the gadget cycle; weapon {:#x}", sim.gun.weapon);
    let target = [13.752469f32, 32.291878, 57.303413];
    // Aim at the point first (no fire), then HOLD the fire: the substate 2 latches and the reel
    // runs while held (phase 1 reads no fire input [M 0x192860:79-121]).
    for _ in 0..200 {
        let (turn, look) = aim_inputs(&scene, target);
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    let mut hung = false;
    for _ in 0..1200 {
        let (turn, look) = aim_inputs(&scene, target);
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
        let sim = scene.sim.as_ref().unwrap();
        let d = ((sim.bond.pos[0] - target[0]).powi(2) + (sim.bond.pos[1] - target[1]).powi(2) + (sim.bond.pos[2] - target[2]).powi(2)).sqrt();
        if sim.bond.grapple.as_ref().map(|g| g.phase) == Some(2) && d < 2.5 { hung = true; break; }
        assert!(!(sim.bond.substate == nf_game::player::SS_WALK && sim.bond.grapple.is_none()),
            "the grapple must not detach while the fire is held (the game holds the hang [M 0x192860:57-62])");
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(hung, "the reel must hang Bond at the point (pos {:?}, phase {:?})",
        sim.bond.pos, sim.bond.grapple.as_ref().map(|g| g.phase));
    // The button's RELEASE detaches him [M 0x192860:64-66] back to the walk substate.
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "the release returns Bond to the walk substate");
    assert!(sim.bond.grapple.is_none(), "the grapple state clears on release");
}

fn aim_inputs(scene: &nf_view::scene::Scene, target: [f32; 3]) -> (f32, f32) {
    let sim = scene.sim.as_ref().unwrap();
    let bp = sim.bond.pos;
    let dx = target[0] - bp[0];
    let dz = target[2] - bp[2];
    let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
    let desired = dx.atan2(dz);
    let err = desired - sim.bond.yaw;
    let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
    (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
}

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }
