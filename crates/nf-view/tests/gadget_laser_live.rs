//! fix-gadgets: the Q-Laser / laser watch beam through the real sim path.
//!
//! The game: while a laser weapon is out, `Player_LaserPointer` 0x1a65d8 casts the beam every tick
//! (the head along the view, 100 units) and the beam object lands in the lock's contact list;
//! `Lock_Update` 0x1ae3d8 case 0 reads the beam's weapon id (0x56/0x57 band) and advances the burn;
//! the open writes the lock's out channel (07000008's safe bolts: ch 134/135 [D #207/#208]).
//! Before this port the sim's tick never burned a lock from gameplay (fix_objects2_live drove
//! `Lock::burn` directly).

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The laser watch (0x54, owned at the 07000008 start [D probe]) aimed at the safe's top bolt
/// (162.24, 43.88, −117.14 [D lock dump; fix_objects2]) burns it: the out channel 134 rises.
/// Sister: the beam pointed away leaves the lock intact.
#[test]
fn the_watch_laser_burns_the_safe_bolt() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    assert!(*sim.drones.inventory.owned.get(0x54).unwrap_or(&false), "the watch is owned at the 07000008 start");
    // Stand before the safe and equip the watch through the real gadget cycle.
    sim.bond.pos = [160.3, 44.6, -115.4];
    sim.bond.yaw = 0.85;
    let mut equipped = false;
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if sim.gun.weapon == 0x54 { equipped = true; break; }
    }
    assert!(equipped, "the watch must equip through the gadget cycle; weapon {:#x}", sim.gun.weapon);
    // Aim at the top bolt through the real look input and hold: the beam burns it.
    let target = [162.2365f32, 43.875843, -117.14455];
    for _ in 0..300 {
        let (turn, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - (bp[1] + sim.bond.eye - sim.bond.crouch_drop);
            let desired = dx.atan2(dz);
            let err = desired - sim.bond.yaw;
            let want_pitch = (dy / dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            (err.clamp(-0.08, 0.08), (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(134), "the top bolt's burn completes: the out channel 134 rises [M 0x1ae3d8:194-195]");
    assert!(sim.drones.locks.iter().find(|l| l.out == 134).map(|l| l.burned).unwrap_or(false), "the lock record is burned");
    assert!(sim.laser_beam.is_some(), "the beam endpoints are parked for the renderer while the laser is out");
}

/// Sister: the same setup aimed at the wall (away from the bolts) burns nothing.
#[test]
fn the_laser_away_from_the_bolts_burns_nothing() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [160.3, 44.6, -115.4];
    sim.bond.yaw = 0.85;
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if sim.gun.weapon == 0x54 { break; }
    }
    // Aim at the ceiling (the bolt is below the eye line): the beam never touches it.
    for _ in 0..300 {
        let (_, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let want_pitch = 0.9f32;
            (0.0f32, (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { look, ..Default::default() }, false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(134) && !sim.drones.channels.on(135), "no bolt burns when the beam points away");
    assert!(!sim.drones.locks.iter().any(|l| l.burned), "the lock records stay intact");
}

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }
