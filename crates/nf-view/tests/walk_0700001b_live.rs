//! map-finale: the 0700001b walkthrough (docs/levels/0700001b.md). Drives the REAL path —
//! `nf_view::load` + `Sim::step_bond` — through the space station's arrival: the first objective
//! shows at once (the state-0 arm [M Mission_MonitorObjectives 0x1827d8:41-54]; enable 0 [L record
//! 1b]) and the mission stays open: the missile consoles' AND (#139 [ch 21 & 22 & 27]) and Drake's
//! defeat (ch 0x0b) have no ported writers (docs/levels/0700001b.md §6). Opt-in via NF_ISO.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn the_arrival_shows_the_missile_objective_and_the_station_holds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 0700001b");
    let sim = scene.sim.as_mut().unwrap();
    let p = sim.bond.pos;
    assert!(p[0] > 21.0 && p[0] < 25.0 && p[2] > 1.0 && p[2] < 5.0,
        "the shuttle arrival spawns in the station hub (#1 [D]), got {p:?}");
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.mission.states[23][0], 3, "'Disable the missiles' shows at once (enable 0 [L], the state-0 arm [M 0x1827d8:41-54])");
    assert_eq!(sim.drones.mission.states[23][1], 1, "'Defeat Drake' hides until ch 0x09 [L record 1b obj 2]");
    // The consoles' AND (#139 [ch 21 & 22 & 27] [D]) has no ported writers (the class-0x38 Monitor
    // script entities — docs/levels/0700001b.md §6): the objective never completes here and the
    // mission never ends.
    assert!(!sim.drones.channels.on(0x09), "the console AND never fires without the Monitor activations");
    assert!(!sim.drones.channels.on(0x0b), "Drake's defeat channel never fires");
    assert!(!sim.drones.channels.on(100), "the mission stays open (the monitor's live AND is unmet)");
    assert_eq!(sim.drones.level_exit, None, "the station has no LOAD_LEVEL placement [D] and no mission end");
    // The station is alive: Bond survived the smoke run and the garrison is around him (the console
    // squads 0x05000078 hold in groups 115-120 [D] — the count at load is the spawners' business,
    // so assert presence loosely).
    assert!(sim.drones.health.health > 0.0, "Bond survived the smoke run");
    assert!(!sim.drones.list.is_empty(), "the station garrison's placements parsed");
}
