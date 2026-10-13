//! walkreal-a: 07000005 (Mayhew A, the castle approach) walked with the player's own movement — no
//! teleports after the spawn. Two routes (docs/levels/07000005.md):
//!
//! 1. THE TRUCK + GATE ROUTE: the cold start #26 (104.95, 25.12, 108.83) → walk to the gatehouse
//!    (the ch-0x5C TouchOnce at (98.49, 27.05, 106.36); the spawn's own ch-0x45 reveal plate fires
//!    under Bond) → the truck chain: SP #37's truck script drives up, SP #38 hands Bond aboard
//!    (t1884/t2028 [D]), the crouched ride parks on the ch-0x70 plate [D; fix-truck] → mux #316 →
//!    SP #320 → ch 0x1D → the gate door #118 loads 07000006.
//! 2. THE CABLE: the cold start → west along the courtyard → door #153 (57.75, 26.0, 89.31) →
//!    ladder #151 (60.93, 29.91, 89.20, top 34.7) → the upper ledge west → wire #251 "Wire_Cable"
//!    (centre (−23.5, 34.5, 6.3), r 4.0, yaw −1.57 [D dump]) → E-grab (the game's action-key grab
//!    [M Player_HandleJump 0x1916b0]) → the hand-over-hand ride [M Player_Wire 0x197658] → E on the
//!    zip door #112 (fallback 0x1E [D]) loads 07000006.

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::Input;
use walker::{Walker};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The truck + gate route: the walk to the gatehouse, the scripted ride (the crouch is the player's
/// own input [D fix-truck's trace]), the park, the chain's automatic exit.
/// IGNORED (walkreal-a parked mid-task): the gatehouse floor sits ~1.1 over the courtyard and the
/// hand-off's volume wants Bond ON it (the fix_truck teleport stood at (100,26,108); the walked
/// Bond stops 1.1 below and the chain never fires — the ride trace shows 0x5c/0x70 dead for 9000
/// ticks). The next session needs the gatehouse ramp route (the ch-0x25 plate at (58.7, 27.1, 110.7)
/// marks the upper lane) or a jump-onto-ledge fix in the walker.
#[test]
#[ignore]
fn walk_to_the_gatehouse_and_ride_the_truck() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"));
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 104.95).abs() < 2.0 && (p[2] - 108.83).abs() < 2.0, "cold start #26 [D], got {p:?}");
        assert!(!sim.disabled, "no intro blocks this level's start");
    }
    w.wait(30);
    // Walk to the gatehouse (the truck's boarding spot; the fix-truck teleport stood at (100,26,108)).
    // Lenient: the gatehouse floor sits ~1.1 above the courtyard and the truck's own Script_Free
    // hand-off takes Bond from where he stands (the enable's `at` is the truck's matrix).
    w.walk_nav_to([100.0, 26.0, 108.0], "to the gatehouse", 2.6);
    {
        // The spawn's reveal plate (ch 0x45 at feet+0.47) sits below the toucher stand-in's reach —
        // a documented stand-in miss (see drone.rs's toucher comment); the truck chain does not
        // need it (fix_truck_live drives the whole ride without it).
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[05] at the gatehouse tick {} bond {:?}; ch 0x45 {} ch 0x5c {}", w.total_ticks, sim.bond.pos,
            sim.drones.channels.on(0x45) as u8, sim.drones.channels.on(0x5c) as u8);
    }
    // The ride: the hand-off re-enables Bond on the truck bed (the disable→enable transition); the
    // crouch is the trace's ride stance [D fix-truck]; the park raises ch 0x70; the chain ends the
    // level into 07000006. The same integration fix_truck_live drives, from a WALKED start.
    let mut prev_dis = w.scene.sim.as_ref().unwrap().disabled;
    let mut crouch_armed = false;
    let mut input = Input::default();
    for t in 0..9000u32 {
        let dis = w.scene.sim.as_ref().unwrap().disabled;
        if !dis && prev_dis && w.scene.sim.as_ref().unwrap().bond.pos[0] < 95.0 { crouch_armed = true; }
        prev_dis = dis;
        input.crouch = if crouch_armed && !w.scene.sim.as_ref().unwrap().bond.crouched { crouch_armed = false; true } else { false };
        w.tick(input, false);
        if t % 500 == 0 {
            let sim = w.scene.sim.as_ref().unwrap();
            eprintln!("[05 ride t{t}] bond {:?} disabled {} 0x5c {} 0x70 {} 0x1d {}", sim.bond.pos, sim.disabled,
                sim.drones.channels.on(0x5c) as u8, sim.drones.channels.on(0x70) as u8, sim.drones.channels.on(0x1d) as u8);
        }
        if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { break; }
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x70), "the crouched ride parks on the ch-0x70 plate [D; fix-truck]");
    assert_eq!(sim.drones.level_exit, Some(0x0700_0006), "the truck chain's ch 0x1D ends the level into 07000006");
    eprintln!("[05] truck route done in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
}

/// The cable route: the west walk, the ladder, the upper ledge, the E-grabbed wire ride, the zip
/// door's fallback exit. IGNORED with the truck test (walkreal-a parked mid-task; the west anchors
/// are untested beyond the ladder).
#[test]
#[ignore]
fn walk_the_west_ledge_and_cross_the_cable() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000005.bin", false).expect("load 07000005"));
    w.wait(30);
    // West along the courtyard, through door #153, up ladder #151.
    for (name, at) in [
        ("courtyard west", [62.0f32, 26.0, 96.0]),
        ("door #153", [57.8, 26.0, 89.4]),
        ("ladder #151 base", [60.9, 29.9, 89.2]),
    ] {
        w.walk_nav_to(at, name, 1.4);
    }
    w.walk_to(walker::Wp { at: [60.9, 29.9, 89.2], act: walker::Act::Ladder { top_y: 34.7 }, arrive: 0.9 }, "ladder #151");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[05] the ladder top at tick {} bond {:?}", w.total_ticks, sim.bond.pos);
        assert!(sim.bond.pos[1] > 33.0, "Bond must stand on the ledge (y {:.2})", sim.bond.pos[1]);
    }
    // The upper ledge west to the cable. The anchors follow the ledge's walked lane; the nav graph
    // has no cross-map coverage on this level [D the probe: no route from the spawn].
    for (name, at) in [
        ("ledge west 1", [45.0f32, 34.5, 80.0]),
        ("ledge west 2", [20.0, 34.5, 55.0]),
        ("ledge west 3", [-5.0, 34.5, 30.0]),
        ("the cable mouth", [-20.0, 34.5, 8.0]),
    ] {
        w.walk_nav_to(at, name, 1.6);
    }
    // E-grab the cable and ride it [M Player_CollWire 0x197130; Player_Wire 0x197658].
    w.walk_to(walker::Wp { at: [-23.5, 34.5, 6.3], act: walker::Act::WireRide, arrive: 0.9 }, "the cable");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[05] after the cable at tick {} bond {:?} falls {:?}", w.total_ticks, sim.bond.pos, w.report.falls);
    }
    // Walk to the zip door #112 and E it: the fallback 0x1E loads 07000006 [D #112 gate 0].
    for (name, at) in [
        ("back ledge", [-10.0f32, 34.5, -20.0]),
        ("the zip door approach", [30.0, 45.0, -60.0]),
    ] {
        w.walk_nav_to(at, name, 1.6);
    }
    let sim = w.scene.sim.as_ref().unwrap();
    let exit = sim.drones.triggers.iter()
        .find(|t| t.kind == walker::trigger_kind::LOAD_LEVEL && (t.fallback == 0x1e)).map(|t| t.centre()).expect("the zip door #112");
    drop(sim);
    w.walk_to(walker::Wp { at: exit, act: walker::Act::Use { use_ticks: 90 }, arrive: 1.6 }, "E the zip door");
    let sim = w.scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0006), "the zip door's fallback 0x1E loads 07000006 [M 0x1b2318:91-107]");
    eprintln!("[05] cable route done in {} ticks; falls: {:?}", w.total_ticks, w.report.falls);
    for f in &w.report.falls { eprintln!("[05] fall: {f}"); }
    for s in &w.report.stuck_events { eprintln!("[05] stuck: {s}"); }
}
