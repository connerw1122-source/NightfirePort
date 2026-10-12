//! map-finale: the 07000016 walkthrough (docs/levels/07000016.md). Drives the REAL path —
//! `nf_view::load_with_carry` (07000015's exit carried channels) + `Sim::step_bond` — through the
//! final mission's part 2: the spawn plate completes 07000015's objective (ch 0x8e [D placement
//! #252]), the silo plate completes "Reach Drake's launch silo" (ch 0x8f [D #253]), and the drone
//! tripwire (#266, out ch 0x7b) fails the mission [M Mission_MonitorObjectives 0x1827d8:108-116].
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::{Input, RamSave};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    // A fresh Bond with only the carried gates: the save Trigger_Activate would make [M 0x1b2318:97-116].
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

/// The arrival: the carried 07000014 chain (ch 0x47/0x48/0x4a/0x54/0xce [L record 20]) and 07000015's
/// reveal (ch 0x4b) ride the save; the spawn plate #252 fires ch 0x8e and completes "Find the exit to
/// Omega Sector" for the carried record 21.
#[test]
fn the_arrival_completes_the_omega_sector_objective() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "07000016.bin", false, Some(&carry(&[0x47, 0x48, 0x4a, 0x54, 0xce, 0x4b]))).expect("load 07000016");
    let sim = scene.sim.as_mut().unwrap();
    let p = sim.bond.pos;
    assert!(p[0] > 3.0 && p[0] < 7.0 && p[2] > -67.0 && p[2] < -63.0,
        "the arrival spawns at the Omega exit's far side (#1 [D]), got {p:?}");
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..200 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x8e), "the spawn plate #252 fires ch 0x8e [D]");
    for (i, want) in [20usize, 21usize].into_iter().enumerate() {
        let done: Vec<u32> = sim.drones.mission.states[want].clone();
        assert!(done.iter().all(|&s| s == 4), "record {want} (the 07000014/15 chain, test {i}) fully complete, got {done:?}");
    }
    assert_eq!(sim.drones.mission.states[22][0], 3, "ch 0x8e reveals 'Reach Drake's launch silo' [L record 16 obj 1]");
    assert_eq!(sim.drones.mission.states[22][1], 1, "the Kiko objective still hides (enable 0x8f not fired)");
}

/// Objective 1: the silo plate #253 (0.45-4.62, -154.93..-152.85, ch 0x8f [D]). Sister: the Kiko
/// objective stays hidden until it, and the mission AND (#271 [ch 0x90]) never completes without it.
#[test]
fn the_silo_plate_completes_objective_1_and_the_mission_stays_open() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "07000016.bin", false, Some(&carry(&[0x47, 0x48, 0x4a, 0x54, 0xce, 0x4b]))).expect("load 07000016");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        for _ in 0..200 { sim.step_bond(Input::default(), false, false, false); }
        // The silo pit: the plate #253's volume is a slab over the shaft bottom under the pit's mid
        // ledge (0.45-4.62, -0.20..1.88, -154.93..-152.85; the ledges stand at 3.1-6.3, the bottom
        // floor at -0.20 [D coll-floor]) — drop into the slab from just above it.
        sim.bond = nf_game::player::Bond::new([2.5, 1.2, -153.9], std::f32::consts::PI);
        let mut input = Input::default();
        input.forward = 1.0;
        for _ in 1..=120 { sim.step_bond(input, false, false, false); if sim.drones.channels.on(0x8f) { break; } }
        for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x8f), "the silo plate #253 fires ch 0x8f [D]");
    assert_eq!(sim.drones.mission.states[22][0], 4, "the monitor completes 'Reach Drake's launch silo'");
    assert_eq!(sim.drones.mission.states[22][1], 3, "ch 0x8f reveals 'Prevent Kiko from launching into space' [L record 16 obj 2]");
    // The mission end: the AND #271 needs ch 0x90 — Kiko's defeat, whose writer is not ported
    // (docs/levels/07000016.md §6). The level stays open.
    assert!(!sim.drones.channels.on(100), "the AND #271 is open without ch 0x90");
    assert_eq!(sim.drones.level_exit, None, "no mission end without the Kiko objective");
}

/// The failure condition: the drone tripwire #266 (mask 0x10, out ch 0x7b [D]) — a drone's capsule
/// entering the silo-mouth volume fires ch 0x7b and the monitor fails the mission on 0x07000016
/// [M 0x1827d8:108-116, FailLabel 0x2000059] to the fail screen 0x07000048 [M 0x182c60:17-57].
#[test]
fn the_drone_tripwire_fails_the_mission() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "07000016.bin", false, Some(&carry(&[0x47, 0x48, 0x4a, 0x54, 0xce, 0x4b]))).expect("load 07000016");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        for _ in 0..100 { sim.step_bond(Input::default(), false, false, false); }
    }
    // Sister first: away from the volume the mission holds.
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x7b), "the tripwire is cold away from the silo mouth");
        assert_eq!(sim.drones.level_exit, None, "the mission holds");
    }
    // Walk a guard into the tripwire volume (17.36-22.02, -0.02..2.06, -152.35..-148.76 [D]).
    {
        let sim = scene.sim.as_mut().unwrap();
        let guard = sim.drones.list.iter().position(|d| d.present && d.enabled).expect("a live guard exists");
        sim.drones.list[guard].pos = [19.5, 1.0, -150.5];
        for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
    }
    // The monitor's 0x07000016 special [M 0x1827d8:108-116]: ch 0x7b → FailLabel 0x2000059 + ch 99;
    // Mission_Update's state-1 sees ch 99 and enters the fail flow [M 0x182c60:17-57] — the port
    // carries it as the mission layer's state 2 and the "Mission Failed" event (the viewer maps the
    // mission_events to the restart at the frame loop, outside the headless step path).
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x7b), "the drone contact fires ch 0x7b [D trigger #266 mask 0x10]");
        assert_eq!(sim.drones.mission.fail_label, 0x0200_0059, "FailLabel = 'the launch went ahead' [M 0x1827d8:108-111]");
        assert!(sim.drones.channels.on(99), "the mission-fail channel fired");
        assert_eq!(sim.drones.mission.state, 2, "Mission_Update entered the fail flow [M 0x182c60 case 2]");
    }
    // The state-2 tick posts the banner — the HUD tick (inside step_bond) drains the events into
    // the mission banner pane [M scene.rs hud_tick ← step_bond].
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.hud.mission_msgs.iter().any(|m| m.text.contains("Fail")), "the 'Mission Fail' banner posted");
    assert_eq!(sim.drones.mission.internal, 7, "the banner's timeout state [M 0x182c60 case 2 → 7]");
}
