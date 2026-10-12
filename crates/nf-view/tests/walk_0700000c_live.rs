//! map-powerstation: the 0700000c walkthrough (docs/levels/0700000c.md). Drives the REAL path —
//! `nf_view::load` + `Sim::step_bond` with the viewer's input mapping — through the mission's first
//! level: the ST-1 investigation plate, the jetpack photograph (the camera gadget 0x54 into the
//! `CamSubject` 0x1bc668 path) and the gate exit into 0700000d ([L] the mission table's record 11:
//! objectives 0x400002c/2d/2e on channels 0x1b/0x1c/0x23, ee_start_05.bin @0x2a4080).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The load sequence is the game's: `Mission_Init` (0x182270) runs in the load, the monitor
/// (`Mission_MonitorObjectives` 0x1827d8) every tick after.
#[test]
fn the_loadout_and_the_first_objective_match_the_game() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    // The cold start: the first gate-0 start (#1 [D]) on the ST-1 walkway.
    let p = sim.bond.pos;
    assert!(p[0] > 7.0 && p[0] < 10.0 && p[2] > -151.0 && p[2] < -148.0,
        "the cold start spawns on the ST-1 walkway (#1 [D]), got {p:?}");
    assert!(sim.drones.inventory.owned.get(0x54).copied().unwrap_or(false),
        "the camera gadget (0x54) is in the loadout [M 0x1a7ef8 case 0x700000c]");
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..200 { sim.step_bond(Input::default(), false, false, false); }
    assert!(!sim.disabled, "Bond is free after the intro");
    // [M 0x1a7ef8 case 0x700000c]: the sniper selected (+0x63 = Upgrade_Weapon(0x1e) = 0x1e [L slot 0]) —
    // the draw machine completes the switch during the intro wait.
    assert_eq!(sim.gun.weapon, 0x1e, "the power-station loadout selects the sniper");
    // The mission states: the LEVELS order puts 0700000c's record at 11, 0700000d's at 12 [M
    // crate::mission::LEVELS]; states are 0 pending / 1 pending+enable / 3 announced / 4 complete
    // [M 0x1827d8's switch].
    let objs = |sim: &nf_view::scene::Sim| {
        let mut v: Vec<u32> = sim.drones.mission.states[11].clone();
        v.extend(sim.drones.mission.states[12].iter());
        v
    };
    // [M 0x1827d8:41-54] the first objective shows at once (reveal 0 → state 0 → the level arm);
    // the next two wait for their reveal channels.
    assert_eq!(objs(sim), vec![3, 1, 1, 1, 1, 1], "objective 1 shows, the rest hide until their reveals");
}

/// Objective 1: the ST-1 investigation plates (#506/#274, both out ch 0x1b [D]) sit on the lower
/// walkway (floor 11.84 [D coll-floor]); walking it fires ch 0x1b and the monitor completes
/// "Investigate building ST-1" [L record 11 obj 0, complete ch 0x1b].
#[test]
fn walking_st1_completes_objective_1() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [5.8, 13.2, -154.5], std::f32::consts::PI);
    let mut input = Input::default();
    input.forward = 1.0;
    for _ in 1..=300 {
        sim.step_bond(input, false, false, false);
        if sim.drones.channels.on(0x1b) { break; }
    }
    for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x1b), "the ST-1 plate (#506 [D]) fires ch 0x1b");
    assert_eq!(sim.drones.mission.states[11][0], 4, "the monitor completes objective 1 [M 0x1827d8:88-99]");
    assert_eq!(sim.drones.mission.states[11][1], 3, "ch 0x1b reveals objective 2 (the photograph) [L record 11 obj 1]");
}

/// Objective 2: the camera gadget's (0x54) bullet into the jetpack display's `CamSubject` volume
/// (#414/#454, both ch 0x1c = param 0 [D]) — `CamSubject_Update` 0x1bc668 writes the channel for a
/// class-5 contact with the 0x54 weapon byte. Sister: the same shot with the sniper never does.
#[test]
fn the_camera_photograph_writes_the_jetpack_channel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    // Switch to the camera and photograph the display from the ground floor.
    let mut input = Input::default();
    input.switch_to = Some(0x54);
    for _ in 0..300 { sim.step_bond(input, false, false, false); }
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [5.4, 8.0, -113.0], 0.0);
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    sim.bond.yaw = 0.0;
    sim.bond.pitch = 0.0;
    let mut fired = false;
    for _ in 1..=60 {
        sim.step_bond(Input::default(), true, false, false);
        fired |= sim.shot_tick;
        if sim.drones.channels.on(0x1c) { break; }
    }
    assert!(fired, "the camera (0x54) fires through the real trigger path");
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x1c), "the photograph writes ch 0x1c [M 0x1bc668:20-22]");
    assert!(sim.drones.props.subjects.iter().all(|s| s.channel == 0x1c), "both jetpack subjects carry ch 0x1c [D placements #414/#454]");
}

/// The gate: the LOAD_LEVEL placement #556 (next 0x0700000d, key gate ch 0x1c [D]) is USE-activated
/// ([M Player_Activate 0x1a5170 case 0x1d → Trigger_Activate 0x1b2318]). With the photograph the E
/// press loads 0700000d; the sister walk (no photograph) stays locked — the level cannot be left
/// without the mission step.
#[test]
fn the_gate_loads_0700000d_only_after_the_photograph() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The sister first: no photograph — the gate's key gate (ch 0x1c) blocks the load.
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [95.0, 11.4, -209.7], std::f32::consts::FRAC_PI_2);
        let mut input = Input::default();
        input.forward = 1.0;
        for t in 1..=240 {
            let use_key = sim.bond.pos[0] > 96.3;
            sim.step_bond(input, false, false, use_key);
        }
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x1c), "the sister never photographs the jetpack");
        assert_eq!(sim.drones.level_exit, None, "the locked gate (key gate ch 0x1c [M 0x1b2318:76-90]) loads nothing");
    }
    // The real run: the plate, the photograph, then the gate.
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [5.8, 13.2, -154.5], std::f32::consts::PI);
    let mut input = Input::default();
    input.forward = 1.0;
    for _ in 1..=300 { sim.step_bond(input, false, false, false); if sim.drones.channels.on(0x1b) { break; } }
    let mut input = Input::default();
    input.switch_to = Some(0x54);
    for _ in 0..300 { sim.step_bond(input, false, false, false); }
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [5.4, 8.0, -113.0], 0.0);
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    sim.bond.yaw = 0.0;
    sim.bond.pitch = 0.0;
    for _ in 1..=60 { sim.step_bond(Input::default(), true, false, false); if sim.drones.channels.on(0x1c) { break; } }
    assert!(sim.drones.channels.on(0x1c), "the photograph unlocks the gate");
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [95.0, 11.4, -209.7], std::f32::consts::FRAC_PI_2);
    let mut input = Input::default();
    input.forward = 1.0;
    for t in 1..=240 {
        let use_key = sim.bond.pos[0] > 96.3;
        sim.step_bond(input, false, false, use_key);
        if sim.drones.level_exit.is_some() { break; }
    }
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_000d), "E at the gate loads 0700000d [D placement #556 param 0]");
    // The mission layer: objectives 1-2 complete; the mission itself cannot end on 0700000c —
    // 0700000d's three channels are still off [L record 12].
    assert_eq!(sim.drones.mission.states[11][0], 4);
    assert_eq!(sim.drones.mission.states[11][1], 4);
    assert!(sim.drones.mission.states[11][0] == 4 && sim.drones.mission.states[11][1] == 4,
        "the 0700000c objectives are done");
}
