//! map-finale: the 07000015 walkthrough (docs/levels/07000015.md). Drives the REAL path —
//! `nf_view::load` + `Sim::step_bond` — through the final mission's part 1: the reveal plate at the
//! spawn ("Find the exit to Omega Sector", enable ch 0x4b [L the mission table's record 15]) and the
//! USE-activated exit gate #445 into 07000016 ([M Player_Activate 0x1a5170 case 0x1d →
//! Trigger_Activate 0x1b2318]; the gate's own param 2 is 0 so `Trigger_Update` case 10 never
//! self-fires [M 0x1b1a78:162-166]).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The mission states: the LEVELS order puts 07000014's record at 20, 07000015's at 21, 07000016's
/// at 22, 0700001b's at 23 [M crate::mission::LEVELS]; states 0 pending / 1 pending+enable /
/// 3 announced / 4 complete [M 0x1827d8's switch].
#[test]
fn the_arrival_reveals_the_omega_sector_objective() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000015.bin", false).expect("load 07000015");
    let sim = scene.sim.as_mut().unwrap();
    let p = sim.bond.pos;
    assert!(p[0] > 93.0 && p[0] < 96.0 && p[2] > -2.0 && p[2] < 2.0,
        "the cold start spawns on the silo gantry (#1 [D]), got {p:?}");
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..200 { sim.step_bond(Input::default(), false, false, false); }
    // The reveal plate #484 sits under the spawn (93.69-94.69, 40.91-41.89, -1.23..1.30 [D]):
    // walk a little to be sure the standing capsule brushes it.
    let mut input = Input::default();
    input.forward = 1.0;
    for _ in 1..=120 { sim.step_bond(input, false, false, false); if sim.drones.channels.on(0x4b) { break; } }
    for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x4b), "the spawn plate #484 fires ch 0x4b [D]");
    assert_eq!(sim.drones.mission.states[21][0], 3, "ch 0x4b reveals 'Find the exit to Omega Sector' [L record 15]");
    // The objective's completing channel (0x8e) is 070000016's spawn plate: nothing completes here.
    assert_eq!(sim.drones.mission.states[21][0], 3, "the objective stays announced on this level");
    assert_eq!(sim.drones.level_exit, None, "no mission end on 07000015 (the AND needs 0x8e/0x8f/0x90)");
}

/// The exit: the gate #445 (-0.64..0.68, 40.18-42.49, -24.96..-24.60, next 0x07000016, gate param 0
/// [D]) is USE-activated. Sister: the other gate #343 (gate ch 0x42) never self-fires with the
/// channel off, and standing at #445 without pressing E loads nothing.
#[test]
fn the_use_exit_loads_07000016_and_the_gated_one_stays_shut() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // Sister: the gated gate #343 — a brief stand beside its plane with ch 0x42 off (a longer walk
    // feeds Bond to the gantry guards, whose fail flow would swamp the assertion).
    let mut scene = nf_view::load(&iso, "07000015.bin", false).expect("load 07000015");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [70.9, 34.0, 0.9], std::f32::consts::PI);
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(0x42), "nothing writes ch 0x42 on this level [D scans]");
        assert_eq!(sim.drones.level_exit, None, "the ch-0x42-gated gate #343 stays shut [M 0x1b1a78:162-166]");
    }
    // The real exit: E at the #445 plane.
    let mut scene = nf_view::load(&iso, "07000015.bin", false).expect("load 07000015");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [0.0, 41.6, -23.8], std::f32::consts::PI);
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    // The doorway may stop the walk short of the plane; the use sphere (head + look, radius 1 [M
    // 0x1a5170]) reaches the plane from the doorway, so press E from the second tick on.
    let mut input = Input::default();
    input.forward = 1.0;
    for t in 1..=200 {
        sim.step_bond(input, false, false, t > 20);
        if sim.drones.level_exit.is_some() { break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0016), "E at the #445 plane loads 07000016 [D placement #445 param 0]");
}
