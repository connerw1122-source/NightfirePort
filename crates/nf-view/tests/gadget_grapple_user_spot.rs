//! fix-gadgets2 bug 3: the grapple "doesn't work at all" from the user's play spot.
//!
//! The user's screenshot spot on 07000005: `--bond -18.28,38.19,-71.83,-4.717,0.151` — the gadget is
//! selected, labelled "Grapple", aimed at the point on the wall, and firing does nothing. The game's
//! attach: the claw's bolt (a flying bullet, substate 10) hits the class-0x51 grapple object's world
//! sphere and `Bullet_Delete` 0x12e660 calls `Player_SetGrapplePoint` 0x195da0 [M]. The port's
//! stand-in is the aim-ray vs the point's sphere (`pick_grapple_point`). This test drives the REAL
//! input path from the user's exact spot and yaw/pitch and must latch the grapple.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The user's spot: (-18.28, 38.19, -71.83), yaw -4.717, pitch 0.151.
const SPOT: [f32; 3] = [-18.28, 38.19, -71.83];
const YAW: f32 = -4.717;
const PITCH: f32 = 0.151;

#[test]
fn the_q_claw_latches_from_the_users_play_spot() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    assert!(!sim.drones.grapple_points.is_empty(), "07000005 carries grapple points [D #281/#282]");

    // Diagnostics: the parsed points and their ray geometry from the user's eye.
    let mut bond = sim.bond.clone();
    bond.pos = SPOT;
    bond.yaw = YAW;
    bond.pitch = PITCH;
    let head = bond.head();
    let look = bond.look();
    eprintln!("== the user's spot: head {head:?} look {look:?}");
    for (i, p) in sim.drones.grapple_points.iter().enumerate() {
        let to_p = [p.pos[0] - head[0], p.pos[1] - head[1], p.pos[2] - head[2]];
        let dist = (to_p[0] * to_p[0] + to_p[1] * to_p[1] + to_p[2] * to_p[2]).sqrt();
        let along = to_p[0] * look[0] + to_p[1] * look[1] + to_p[2] * look[2];
        let closest = [head[0] + look[0] * along, head[1] + look[1] * along, head[2] + look[2] * along];
        let miss = ((closest[0] - p.pos[0]).powi(2) + (closest[1] - p.pos[1]).powi(2) + (closest[2] - p.pos[2]).powi(2)).sqrt();
        eprintln!("   point #{i} pos {:?} radius {:.3} dist {dist:.2} along {along:.2} miss {miss:.3}", p.pos, p.radius);
    }

    // Stand at the spot, equip the claw through the gadget cycle (the real input path).
    sim.bond.pos = SPOT;
    sim.bond.yaw = YAW;
    sim.bond.pitch = PITCH;
    let mut equipped = false;
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if sim.gun.weapon == 0x4a { equipped = true; break; }
    }
    assert!(equipped, "the claw must equip through the gadget cycle; weapon {:#x}", sim.gun.weapon);
    // Re-assert the user's spot/yaw/pitch (the equip ticks may have moved Bond).
    sim.bond.pos = SPOT;
    sim.bond.yaw = YAW;
    sim.bond.pitch = PITCH;

    // The aim settle: the real look input from the user's angles.
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    sim.bond.yaw = YAW;
    sim.bond.pitch = PITCH;

    // Fire: the grapple must latch (the game's bolt hit → substate 2 [M 0x195da0]).
    sim.step_bond(Input::default(), true, false, false);
    for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.bond.substate, nf_game::player::SS_GRAPPLE,
        "the fire from the user's spot must latch the grapple (substate {:?}, grapple {:?})",
        sim.bond.substate, sim.bond.grapple);
}
