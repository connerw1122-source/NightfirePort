//! map-powerstation: the 0700000d walkthrough (docs/levels/0700000d.md). Drives the REAL path —
//! `nf_view::load_with_carry` (the 0700000c exit's carried channels) + `Sim::step_bond` — through
//! the mission's second level: the arrival (the start trigger's ch 0x23), the ST-3 investigation
//! plate, the laser-prototype photograph and the C5 corridor. [L] the mission table's record 12:
//! objectives 0x400002f/30/31 on channels 0x1d/0x1e/0x1f (ee_start_05.bin @0x2a40c8); the mission
//! completes on the AND #434 [0x1e & 0x1f & 0x28] → ch 100 [D] and ends to the driving hand-off
//! 0x730000d [M Mission_Update 0x182c60:102-103].
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::{Input, RamSave};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    // A fresh Bond with only the carried gates: the save Trigger_Activate would make [M 0x1b2318:97-116].
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

/// The arrival: the carried 0x1b/0x1c ride the save (record 11 preserved by `Mission_Init`
/// [M 0x182270:92-95]); the start trigger #436 (out ch 0x23 [D]) fires under Bond's feet and the
/// monitor completes "Advance through Warehouse TR-2" [L record 11 obj 2] and reveals "Investigate
/// building ST-3" (its reveal channel is 0x23 [L record 12 obj 0]).
#[test]
fn the_arrival_completes_tr2_and_the_yard_completes_st3() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "0700000d.bin", false, Some(&carry(&[0x1b, 0x1c]))).expect("load 0700000d");
    let sim = scene.sim.as_mut().unwrap();
    let p = sim.bond.pos;
    assert!(p[0] > 177.0 && p[0] < 181.0 && p[2] > -207.0 && p[2] < -203.0,
        "the carried arrival spawns at the warehouse start (#1 [D]), got {p:?}");
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..200 { sim.step_bond(Input::default(), false, false, false); }
    let carried = (sim.drones.channels.on(0x1b), sim.drones.channels.on(0x1c), sim.drones.channels.on(0x23),
        sim.drones.mission.states[11][2], sim.drones.mission.states[12][0],
        sim.drones.channels.on(0x1d), sim.drones.channels.on(0x1e), sim.drones.channels.on(0x1f));
    drop(sim);
    let (c1b, c1c, c23, obj3, obj4, c1d, c1e, c1f) = carried;
    assert!(c1b && c1c, "the 0700000c channels survive the warm load (record 11 preserved [M 0x182270:92-95])");
    assert!(c23, "the start trigger #436 fires ch 0x23 under Bond [D]");
    assert_eq!(obj3, 4, "the monitor completes 'Advance through TR-2'");
    assert_eq!(obj4, 3, "ch 0x23 reveals 'Investigate building ST-3' [L]");
    assert!(!c1d && !c1e && !c1f, "record 12's channels cleared on the load [M 0x182270:106-113]");
    // Objective 4: the ST-3 yard TouchOnce #378 (out ch 0x1d [D]) at the yard's edge.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [138.0, 9.5, -305.0], std::f32::consts::PI);
        let mut input = Input::default();
        input.forward = 1.0;
        for _ in 1..=200 {
            sim.step_bond(input, false, false, false);
            if sim.drones.channels.on(0x1d) { break; }
        }
        for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x1d), "the ST-3 plate (#378 [D]) fires ch 0x1d");
    assert_eq!(sim.drones.mission.states[12][0], 4, "the monitor completes objective 4");
    assert_eq!(sim.drones.mission.states[12][1], 3, "ch 0x1d reveals the photograph objective [L record 12 obj 1]");
}

/// Objective 5: the camera gadget's (0x54) bullet into the laser display's `CamSubject` volumes
/// (#494/#495, both ch 0x1e = param 0 [D]) — `CamSubject_Update` 0x1bc668. Sister: without the
/// photograph (the carried gates only) the channel stays off and objective 5 never completes.
#[test]
fn the_camera_photograph_writes_the_laser_channel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The real carry: only what 0700000c earned — ch 0x1d is earned here, so the photograph test
    // walks the ST-3 plate first (the reveal chain [L]).
    let mut scene = nf_view::load_with_carry(&iso, "0700000d.bin", false, Some(&carry(&[0x1b, 0x1c]))).expect("load 0700000d");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        for _ in 0..200 { sim.step_bond(Input::default(), false, false, false); }
        // The ST-3 plate (out ch 0x1d [D]) reveals the photograph objective.
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [138.0, 9.5, -305.0], std::f32::consts::PI);
        let mut walk = Input::default();
        walk.forward = 1.0;
        for _ in 1..=200 { sim.step_bond(walk, false, false, false); if sim.drones.channels.on(0x1d) { break; } }
        assert!(sim.drones.channels.on(0x1d), "the ST-3 plate fires ch 0x1d before the photograph");
        // The camera from the trench in front of the display (the yard floor 7.98, the trench 3.98 [D coll-floor]).
        let mut input = Input::default();
        input.switch_to = Some(0x54);
        for _ in 0..300 { sim.step_bond(input, false, false, false); }
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [137.0, 5.1, -321.0], std::f32::consts::PI);
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
        sim.bond.yaw = std::f32::consts::PI;
        sim.bond.pitch = 0.28;
        let mut fired = false;
        for _ in 1..=60 {
            sim.step_bond(Input::default(), true, false, false);
            fired |= sim.shot_tick;
            if sim.drones.channels.on(0x1e) { break; }
        }
        assert!(fired, "the camera (0x54) fires through the real trigger path");
        assert!(sim.drones.channels.on(0x1e), "the photograph writes ch 0x1e [M 0x1bc668:20-22]");
        for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.mission.states[12][1], 4, "the monitor completes objective 5");
    assert!(sim.drones.props.subjects.iter().all(|s| s.channel == 0x1e), "both laser subjects carry ch 0x1e [D placements #494/#495]");
}

/// The C5 door and the mission end. The two `Destroy` leaves (#93/#96 `bigdoor01/02`, ch 0x1f =
/// param 0, health 1000/10000 = param 3 [D]; `Destroy_CollisionHandler` 0x12bd48 subtracts bullet
/// damage and writes the channel on death) and the corridor plate #433 (out ch 0x28 [D]) feed the
/// AND #434 [0x1e & 0x1f & 0x28] → ch 100. Sister: with the corridor walked but the door alive the
/// AND never completes the mission — the level end needs every one of its three channels.
///
/// The mounted-laser cut-scene that kills C5 in the real game is not ported (see the level doc's
/// "not ported" list) — the door's death channel has no live writer this build, so the end-to-end
/// ch 100 is asserted at the mission layer's unit tests (crate::mission) and the walkthrough
/// asserts the door data, the corridor channel and the open AND.
#[test]
fn the_c5_corridor_feeds_the_mission_end_and_the_door_gates_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "0700000d.bin", false, Some(&carry(&[0x1b, 0x1c, 0x1d, 0x1e]))).expect("load 0700000d");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..200 { sim.step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        // The C5 leaves: ch 0x1f with the game's healths [D placements #93/#96 params].
        let doors: Vec<_> = sim.drones.props.destroy.iter().filter(|d| d.channel == 0x1f).map(|d| d.health).collect();
        assert_eq!(doors.len(), 2, "both C5 leaves parse as Destroy props (0xE1 [M 0x12c318])");
        assert!(doors.contains(&1000.0) && doors.contains(&10000.0), "the leaves' healths are 1000/10000 [D], got {doors:?}");
    }
    // The corridor's sloped faces (#433's 6 tris [D]) hang to y ≈ 5.4 — the standing capsule
    // brushes them just inside the C5 end (floor 3.65 [D coll-floor]).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [134.0, 5.0, -267.0], 0.0);
        for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x28), "the corridor's sloped faces (#433's tris [D]) touch the standing capsule → ch 0x28");
        assert!(!sim.drones.channels.on(100), "the AND #434 is open: ch 0x1f (the C5 death) is missing");
        assert_eq!(sim.drones.level_exit, None, "the mission does not end without the C5 objective");
    }
}
