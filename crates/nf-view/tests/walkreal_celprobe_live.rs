//! merged: master's grapple-release probe + walkreal-a's 05 nav probe.
#[path = "common/walker.rs"]
mod walker;
use nf_game::player::Input;
fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }
#[test]
fn probe_05_nav() {
    let iso = iso();
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    let start = sim.bond.pos;
    for (name, dest) in [("the wire", [-23.5f32, 34.5, 6.3]), ("the zip door", [37.3, 50.1, -68.4]), ("the gate door", [30.0, 38.8, 38.2])] {
        match walker::leg(sim, start, dest) {
            pts if pts.len() > 2 => {
                eprintln!("{name}: {} nodes:", pts.len() - 2);
                for p in pts.iter().skip(1).take(pts.len() - 2).step_by(3) {
                    eprintln!("  ({:.1},{:.1},{:.1})", p[0], p[1], p[2]);
                }
            }
            _ => eprintln!("{name}: no nav route"),
        }
    }
}
#[test]
fn probe_grapple_release() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [22.0, 28.5, 62.0];
    let target = [13.752469f32, 32.291878, 57.303413];
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if sim.gun.weapon == 0x4a { break; }
    }
    for _ in 0..200 {
        let (turn, look) = aim(&scene, target);
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    eprintln!("after fire 1: substate {} grapple {:?}", scene.sim.as_ref().unwrap().bond.substate, scene.sim.as_ref().unwrap().bond.grapple.as_ref().map(|g| (g.phase, g.pull)));
    let mut n = 0;
    loop {
        let (turn, look) = aim(&scene, target);
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
        n += 1;
        let sim = scene.sim.as_ref().unwrap();
        if sim.bond.grapple.as_ref().map(|g| g.phase) == Some(2) || n > 1500 { break; }
    }
    eprintln!("hang after {n}: substate {} phase {:?}", scene.sim.as_ref().unwrap().bond.substate, scene.sim.as_ref().unwrap().bond.grapple.as_ref().map(|g| g.phase));
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("after fire 2: substate {} grapple {:?}", sim.bond.substate, sim.bond.grapple.as_ref().map(|g| (g.phase, g.pull)));
    scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("after fire 3: substate {} grapple {:?}", sim.bond.substate, sim.bond.grapple.as_ref().map(|g| (g.phase, g.pull)));
}
fn aim(scene: &nf_view::scene::Scene, target: [f32; 3]) -> (f32, f32) {
    let sim = scene.sim.as_ref().unwrap();
    let bp = sim.bond.pos;
    let dx = target[0] - bp[0]; let dz = target[2] - bp[2];
    let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
    let desired = dx.atan2(dz);
    let err = desired - sim.bond.yaw;
    let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
    (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
}
fn dxz(a: [f32;3], b: [f32;3]) -> f32 { ((a[0]-b[0]).powi(2)+(a[2]-b[2]).powi(2)).sqrt() }
