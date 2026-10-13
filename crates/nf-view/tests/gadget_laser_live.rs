//! fix-gadgets: the Q-Laser / laser watch beam through the real sim path.
//!
//! The game: the gadget beams are the SHOTS the held fire spawns — each refire tick runs
//! `Player_WeaponFiring` 0x1a5460's held-fire branch → `Player_WeaponInitBullet` 0x1a4bf0 → the
//! beam/bullet reaches the lock's contact list; `Lock_Update` 0x1ae3d8 case 0 reads the contact's
//! weapon id and advances the burn; the open writes the lock's out channel (07000008's safe bolts:
//! ch 134/135 [D #207/#208]). The watch laser/taser (0x54/0x55) additionally fires only while AIMED
//! [M 0x1a5460:512-514]. fix-gadgets2: the burn runs only while the fire is HELD (fix-gadgets ran
//! it whenever the weapon was equipped — the user's "click in, they start ... let go, they actually
//! do what they're supposed to do briefly").

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
    // Aim at the top bolt through the real look input, AIMED (the watch's fire gate [M 0x1a5460:512-514]),
    // and HOLD the fire: the beam burns it.
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
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, aiming: true, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(134), "the top bolt's burn completes: the out channel 134 rises [M 0x1ae3d8:194-195]");
    assert!(sim.drones.locks.iter().find(|l| l.out == 134).map(|l| l.burned).unwrap_or(false), "the lock record is burned");
    assert!(sim.laser_beam.is_some(), "the beam endpoints are parked for the renderer while the laser is out");
}

/// fix-gadgets2: the burn follows the button. The same aim and hold, then the release: the channel
/// stops rising the tick the fire comes up (the game's shots stop spawning [M 0x1a5460:442-501's
/// no-fire branch]). The pre-fire ticks must NOT burn anything (the old model burned while equipped).
#[test]
fn the_watch_laser_burns_only_while_the_fire_is_held() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [160.3, 44.6, -115.4];
    sim.bond.yaw = 0.85;
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if sim.gun.weapon == 0x54 { break; }
    }
    let target = [162.2365f32, 43.875843, -117.14455];
    // Aimed at the bolt WITHOUT the fire: nothing burns (the fail-first property — the pre-fix port
    // burned the lock in this state).
    for _ in 0..120 {
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
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, aiming: true, ..Default::default() }, false, false, false);
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(134), "no burn without the fire held (the old model burned while equipped)");
    }
    // Hold the fire (aimed): the burn completes.
    for _ in 0..240 {
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
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, aiming: true, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(134), "the held fire burns the bolt");
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
    // Aim at the ceiling (the bolt is below the eye line) with the fire HELD and aiming: the beam never touches it.
    for _ in 0..300 {
        let (_, look) = {
            let sim = scene.sim.as_ref().unwrap();
            let want_pitch = 0.9f32;
            (0.0f32, (want_pitch - sim.bond.pitch).clamp(-0.08, 0.08))
        };
        scene.sim.as_mut().unwrap().step_bond(Input { look, aiming: true, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(134) && !sim.drones.channels.on(135), "no bolt burns when the beam points away");
    assert!(!sim.drones.locks.iter().any(|l| l.burned), "the lock records stay intact");
}

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

/// The Q-Decryptor (0x4E/0x4F) burns the same locks [M Lock_Update 0x1ae3d8:57-63 — the contact's
/// weapon id 0x4E/0x4F, the burn 3/tick]: the same safe, the same aim, the bolt burns.
#[test]
fn the_decryptor_burns_the_safe_bolt() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [160.3, 45.5, -115.4];
    sim.bond.yaw = 0.85;
    let mut equipped = false;
    for _ in 0..12 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        if sim.gun.weapon == 0x4e { equipped = true; break; }
    }
    assert!(equipped, "the decryptor must equip through the gadget cycle; weapon {:#x}", sim.gun.weapon);
    let target = [162.2365f32, 43.875843, -117.14455];
    // The decryptor's fire has no aim gate [M 0x1a5460:512-514 reads only 0x54] — hold the fire and burn.
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
        scene.sim.as_mut().unwrap().step_bond(Input { turn, look, ..Default::default() }, true, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(134), "the decryptor's beam burns the top bolt [M 0x1ae3d8:57-63]");
}
