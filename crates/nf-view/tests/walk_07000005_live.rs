//! map-mayhew: the 07000005 walkthrough (docs/levels/07000005.md). Drives the REAL path —
//! `nf_view::load` + `Sim::step_bond` with the viewer's input mapping — through the level's two
//! campaign routes, asserting the mission layer's stages (crate::mission, the [L] objective table)
//! and the exit.
//!
//! - Route (b) [D placement #118]: the cold start (#26, gate 0) walks to the castle-gate door and
//!   presses E → the fallback gate ch 0x1F [M Trigger_Activate 0x1b2318:91-107] → level_exit
//!   0x07000006. Sister: the objective stays incomplete — the door route never fires ch 0x70 (the
//!   first objective's complete channel, [L] ee_start_05.bin @0x2a4354 record 0).
//! - Route (a) [D; WORKLOG A-11a]: the truck ride (the fix-truck integration, cited there) parks
//!   crouched on the ch-0x70 plate → the mission's first objective completes through the monitor
//!   [M Mission_MonitorObjectives 0x1827d8] and ch 0x1D fires → the same exit.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn cold_start_reaches_the_castle_door_and_exits_to_the_castle() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    {
        let sim = scene.sim.as_mut().unwrap();
        // The cold start: Player_Start picks the first gate-0 start (#26 [D]) — (104.95, 25.12, 108.83).
        let p = sim.bond.pos;
        assert!(p[0] > 100.0 && p[0] < 110.0 && p[2] > 100.0 && p[2] < 115.0,
            "the cold start spawns at the gatehouse road (#26 [D]), got {p:?}");
        // Wait out the level-start NIS if one plays (the monitor and the triggers only matter after).
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        assert!(!sim.disabled, "Bond is free after the intro");
    }
    // The door route: teleport-walk to the gate door's volume (#118 "Door_Gate8" [D]) and press E.
    let exit = scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.gate == 0x1D)
        .expect("the castle-gate LOAD_LEVEL (#118, gate 0x1D) exists").clone();
    let c = exit.centre();
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [c[0], c[1], c[2] - 0.5];
        sim.bond.yaw = 0.0;
        for _ in 0..5 { sim.step_bond(Input::default(), false, false, true); }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.level_exit, Some(0x0700_0006), "E at the gate door loads 07000006");
        assert!(sim.drones.channels.on(0x1F), "the truck gate (0x1D) is off → the fallback write ch 0x1F [M 0x1b2318:102-107]");
        // The mission stage: the door route never fires ch 0x70, so the first objective (complete
        // ch 0x70 [L]) is not done — the mission layer carries the gap (its ch-0x45 writer is not on
        // disc either; in play the objective completes on the truck route).
        assert!(!sim.drones.channels.on(0x70), "the door route does not fire the truck-park channel");
        assert_ne!(sim.drones.mission.states[nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0005).expect("level in the mission table")][0], 4, "the first objective is not complete on the door route");
    }
}

/// The truck-ride route: the same integration fix_truck_live drives (cited there — the crouched
/// ride parks on the ch-0x70 plate), asserted here at the MISSION layer: the first objective is announced
/// (its completion channel is 0x46 [L]; no writer found yet) and the level exits to 07000006.
#[test]
fn the_truck_park_announces_the_first_objective_and_exits_to_the_castle() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    // ch 0x45 reveals the first objective. The game's channel writers are the trigger out-channels
    // (Trigger_Activate 0x1b2318:30), the SCRIPT event ops (Script_EventHandler 0x1d94c0:166,187 —
    // switch_channels[ch] = value straight from the script stream) and the mission init
    // (Mission_Init 0x182270:106-116 writes switch_channels[rec+0xc] = rec+0xd from the mission's
    // init records; 07000005's single record [L ee_start_05: mission entry at 0x2a4354 → records
    // 0x2a3e28, count 0x2a4360 = 1] clears ch 0x46 — it does NOT set 0x45). So 0x45's setter is the
    // level's script stream — the mechanism is [M], the exact script/op is not extracted yet [S]
    // (see docs/levels/07000005.md §2). Until the script extraction lands the test drives the
    // channel the way the game's HUD implies.
    sim.drones.channels.set(0x45, 1, sim.drones.frame);
    let mut prev_dis = sim.disabled;
    let mut crouch_armed = false;
    let mut input = Input::default();
    for t in 1..=7000u32 {
        if !sim.disabled && prev_dis && sim.bond.pos[0] < 95.0 { crouch_armed = true; }
        prev_dis = sim.disabled;
        if crouch_armed && !sim.bond.crouched { input.crouch = true; crouch_armed = false; } else { input.crouch = false; }
        sim.step_bond(input, false, false, false);
        if t % 500 == 0 {
            eprintln!("[ride t{t}] bond {:?} 0x70 {} obj1 state {}", sim.bond.pos, sim.drones.channels.on(0x70) as u8, sim.drones.mission.states[nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0005).expect("level in the mission table")][0]);
        }
        if sim.drones.level_exit.is_some() { break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x70), "the crouched ride parks on the ch-0x70 plate [D; fix-truck]");
    // Merge note (main agent): the objective record is `46 00 01 45` at +0x0C [L ee_start_05.bin 0x2a3e34]: it is
    // ANNOUNCED on ch 0x45 and COMPLETED on ch 0x46, not ch 0x70 (map-mayhew's first table read 0x70; fix-mission's
    // decode is the right one). No writer of ch 0x46 on 07000005 has been found yet, so the objective stays shown (3).
    // The open item is in docs/levels/07000005.md and the merge log.
    let st = sim.drones.mission.states[nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0005).expect("level in the mission table")][0];
    assert_eq!(st, 3, "'Breach the castle walls' is announced (ch 0x45) and waits on ch 0x46 [L]");
    assert_eq!(sim.drones.level_exit, Some(0x0700_0006), "the truck chain's ch 0x1D ends the level into 07000006");
}
