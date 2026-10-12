//! Headless NF_ISO walkthrough for 0700000a (Tower B, the office floor) — the real Sim path
//! (`nf_view::load_with_carry` + `Sim::step_bond`). The chain (docs/levels/0700000a.md):
//! arrival plate #58 → ch 0x47 (objective 3 "Take main elevator to office floor"); the Q-Worm
//! gadget (weapon 0x58) fired at Q-WORM_PC_1 (0x35 = PCQWorm, param 1 = ch 0x48) installs the
//! virus [M PCQWorm_Update 0x1aee20] → objective 4; the security keyboard (SP #60, mask 4) →
//! ch 0x3A → AND #741 → ch 0x49 (objective 5); AND #742 → ch 0x4A unlocks the exit; E at the
//! Load Point #59 (key_gate ch 0x4A) loads 0700000b [M 0x1b2318:76-90].

use nf_game::player::RamSave;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_carry(iso: &str, save: Option<&RamSave>) -> nf_view::scene::Scene {
    nf_view::load_with_carry(iso, "0700000a.bin", false, save).expect("load 0700000a")
}

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// The warm-transition save: the elevator chain left ch 0x46 on (objective 2's AND held it on
/// 07000009 and channels carry through `Player_RamSave` [M 0x195670:13]).
fn save_with_elevator(scene: &nf_view::scene::Scene) -> RamSave {
    let sim = scene.sim.as_ref().unwrap();
    RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
}

/// Smoke + arrival: the level loads warm (ch 0x46 carried), Bond stands at the office-floor
/// start, the arrival plate completes objective 3 (ch 0x47) the same way the game's own load
/// does ([L] ee_start_0700000a.bin: ch 0x47 is the only channel on at load), and 600 ticks run
/// clean with no level end.
#[test]
fn arrival_and_smoke() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let cold = load_carry(&iso, None);
    let save = save_with_elevator(&cold);
    let mut scene = load_carry(&iso, Some(&save));
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.level, 0x0700_000a);
        assert!((sim.bond.pos[0] + 3.93).abs() < 1.0, "the office-floor start #0, not {:?}", sim.bond.pos);
        assert_eq!(sim.drones.list.len(), 20, "20 drone placements [D survey]");
    }
    tick(&mut scene, 30);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x47), "the arrival plate #58 completes objective 3");
    }
    tick(&mut scene, 570);
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.level_exit.is_none(), "an idle run must not end the level");
    assert!(!sim.drones.channels.on(0x61), "no alarm without a cause");
}

/// The Q-Worm install through the real fire path: Bond owns the Q-Worm gadget (0x58), stands in
/// the PC room, fires at Q-WORM_PC_1 — the bullet reaches it and the install writes ch 0x48
/// (objective 4) [M 0x1aee20]. Sister: without firing, ch 0x48 stays off.
#[test]
fn qworm_install_by_firing_the_gadget() {
    let Some(iso) = iso() else { return };
    let cold = load_carry(&iso, None);
    let save = save_with_elevator(&cold);
    let mut scene = load_carry(&iso, Some(&save));
    let pc = scene.sim.as_ref().unwrap().drones.useobjs.pcs.iter().find(|p| p.ch == 0x48)
        .expect("Q-WORM_PC_1 (0x35, param 1 = ch 0x48) exists on this level").pos;
    eprintln!("[qworm] PC at {:?}", pc);
    // Hand Bond the gadget and stand him with his EYE (pos + 0.7 [M player.rs EYE]) level with
    // the PC, 0.71 away — inside the shot's 2-unit range (weapon_data[0x58]+0x1C = 2 [D probe]) —
    // looking straight at it.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.inventory.owned[0x58] = true;
        sim.gun.weapon = 0x58;
        sim.gun.switch_request = 0x58;
        sim.bond.pos = [pc[0] + 0.5, pc[1] - 0.7, pc[2] + 0.5];
        sim.bond.yaw = (-0.5f32).atan2(-0.5);
        sim.bond.pitch = 0.0;
    }
    tick(&mut scene, 30);
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false); }
    tick(&mut scene, 90);
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x48), "the Q-Worm install completed objective 4's channel");
    // Sister: no trigger pull → no install.
    let mut scene2 = load_carry(&iso, Some(&save));
    {
        let sim2 = scene2.sim.as_mut().unwrap();
        sim2.drones.inventory.owned[0x58] = true;
        sim2.gun.weapon = 0x58;
        sim2.gun.switch_request = 0x58;
        sim2.bond.pos = [pc[0] + 0.5, pc[1] - 0.7, pc[2] + 0.5];
        sim2.bond.yaw = (-0.5f32).atan2(-0.5);
        sim2.bond.pitch = 0.0;
    }
    tick(&mut scene2, 40);
    assert!(!scene2.sim.as_ref().unwrap().drones.channels.on(0x48), "not firing → no install");
}

/// The exit gate: with ch 0x4A on (Q-Worm AND keyboard — the keyboard channel driven where the
/// doc records the placement-transform limitation), E at the hatch loads 0700000b. Sisters:
/// E without ch 0x4A is refused (the key gate [M 0x1b2318:76-90]), and the alarm fails the level.
#[test]
fn hatch_exit_and_sisters() {
    let Some(iso) = iso() else { return };
    let cold = load_carry(&iso, None);
    let save = save_with_elevator(&cold);
    // The exit's volume centre.
    let exit = load_carry(&iso, Some(&save)).sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.next_level == 0x0700_000b)
        .expect("the hatch Load Point").clone();
    let centre = [(exit.lo[0] + exit.hi[0]) * 0.5, exit.lo[1] + 1.0, (exit.lo[2] + exit.hi[2]) * 0.5];

    // Locked: E without ch 0x4A → no load (the key gate).
    let mut scene = load_carry(&iso, Some(&save));
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = centre;
        sim.bond.yaw = 0.0;
        sim.step_bond(Default::default(), false, false, true);
    }
    assert_eq!(scene.sim.as_ref().unwrap().drones.level_exit, None,
        "the hatch refuses E while ch 0x4A is off [M 0x1b2318:76-90]");

    // Unlocked: Q-Worm (0x48) + keyboard (0x3A → AND #741 → 0x49) → AND #742 → 0x4A → E loads.
    let mut scene = load_carry(&iso, Some(&save));
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.set_channel(0x3a); // SP #60 KeyboardMH's out (docs/levels/0700000a.md)
        sim.drones.set_channel(0x48); // the Q-Worm install's channel (the test above shows the fire path)
    }
    tick(&mut scene, 5);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x49), "AND #741: the keyboard completes objective 5");
        assert!(sim.drones.channels.on(0x4a), "AND #742: the exit unlocks");
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = centre;
        sim.bond.yaw = 0.0;
        sim.step_bond(Default::default(), false, false, true);
    }
    assert_eq!(scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_000b),
        "E at the unlocked hatch loads 0700000b");

    // The alarm fails the level (the 8 alarm switches write ch 0x61 directly here [D]) — through
    // Mission_Update's 300-tick banner [M 0x182c60:45-57].
    let mut scene = load_carry(&iso, Some(&save));
    scene.sim.as_mut().unwrap().drones.set_channel(0x61);
    tick(&mut scene, 340);
    assert_eq!(scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_0048),
        "the alarm fails the mission [M 0x1827d8:128-131]");
}
