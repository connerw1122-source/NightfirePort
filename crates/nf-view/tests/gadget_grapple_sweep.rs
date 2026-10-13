//! Diagnostic: the grapple's acceptance window from the user's spot — yaw/pitch sweep.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn grapple_acceptance_window_sweep() {
    let iso = iso();
    for (dyaw, dpitch) in [
        (0.0f32, 0.0f32), (0.02, 0.0), (-0.02, 0.0), (0.04, 0.0), (-0.04, 0.0),
        (0.0, 0.02), (0.0, -0.02), (0.0, 0.04), (0.0, -0.04),
        (0.06, -0.03), (-0.05, 0.05),
    ] {
        let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [-18.28, 38.19, -71.83];
        let mut equipped = false;
        for _ in 0..12 {
            sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
            for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
            if sim.gun.weapon == 0x4a { equipped = true; break; }
        }
        assert!(equipped);
        sim.bond.pos = [-18.28, 38.19, -71.83];
        sim.bond.yaw = -4.717 + dyaw;
        sim.bond.pitch = 0.151 + dpitch;
        for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
        sim.bond.yaw = -4.717 + dyaw;
        sim.bond.pitch = 0.151 + dpitch;
        // Hold the fire for 3 ticks like a human click.
        for _ in 0..3 { sim.step_bond(Input::default(), true, false, false); }
        for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
        let latched = sim.bond.substate == nf_game::player::SS_GRAPPLE;
        let hung = sim.bond.grapple.as_ref().map(|g| g.phase);
        eprintln!("dyaw {dyaw:+.2} dpitch {dpitch:+.2} -> latch {latched} phase {hung:?} pos {:?}", sim.bond.pos);
    }
}
