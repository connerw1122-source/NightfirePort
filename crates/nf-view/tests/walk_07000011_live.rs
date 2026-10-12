//! map-tower2 walkthrough: 07000011 "HT Tower2 A" — the real path from the level's own data.
//! Run with: `NF_ISO=… cargo test -p nf-view --test walk_07000011_live -- --nocapture --test-threads=1`
//!
//! The chain [D placements; M decompile addresses cited inline]:
//! 1. Cold start #146 (gate 0) at (−0.40, 46.97, 25.83) yaw π [D placements 0x24]. The spawn
//!    TouchOnce #282 (out 0x7E, mask 8) covers the start tile [D].
//! 2. Objectives [L ee_start_05.bin @0x2a4188; crates/nf-game/src/mission/c07000011.rs]:
//!    "Escape from the tower" (0x04000035) completes on ch 0x64 (=100, mission success — the AND of
//!    every part of mission 0x07000011 [M Mission_MonitorObjectives 0x1827d8:119-123]);
//!    "Gain access to lower office floor" (0x04000036) completes on ch 0x48 — whose only writer is
//!    TouchOnce #679 on the NEXT level 07000012 [D; no 0x48 writer exists on 07000011 — checked every
//!    trigger out, script stream, door/pickup param and kind-5 script [D]].
//! 3. The atrium fall: the five TouchOnces #314–318 (out 0x7C, mask 8) are planes at y=42.01 spanning
//!    the open atrium (x −60…59, z 8…46) [D]. ScriptPlayer #313 (trigger ch 0x7C) plays the level
//!    archive's cut-scene 0x0600098A (camera, 110 ticks): stream 6 writes event 0xD [0x62, 1] at
//!    tick 0x67 [D nis decode] — ch 0x62 is the player-dead channel [M Player_HandleDeath 0x190bb0:32],
//!    Mission_Update turns it into the death restart [M 0x182c60 case 1].
//! 4. The exit: "Load Point" #158 (0xE8, next 0x07000012, key gate 0x1E, gate param 0) and
//!    "DOOR_loadlevel" #193 (0xE8, next 0x07000012, gate 0x1F, key 0) — E-pressing either loads
//!    07000012 [M Trigger_Activate 0x1b2318: the gate/fallback branches both fall through to the load;
//!    the port: triggers.rs activate_request].

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn wait_free(sim: &mut nf_view::scene::Sim, ticks: u32) -> bool {
    for _ in 0..ticks {
        sim.step_bond(Input::default(), false, false, false);
        if !sim.disabled { return true; }
    }
    !sim.disabled
}

/// The cold start: Bond spawns at #146, the spawn touch fires 0x7E, both objectives announce, and no
/// channel of the mission chain (0x48) is on yet.
#[test]
fn cold_start_and_objective_announce() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000011.bin", false).expect("load 07000011");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0011);
    let p = sim.bond.pos;
    assert!(p[0] > -1.5 && p[0] < 1.0 && p[1] > 46.0 && p[2] > 24.0 && p[2] < 27.5,
        "the cold start spawns at #146 [D], got {p:?}");
    // The spawn TouchOnce #282 (out 0x7E) fires within a few ticks.
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x7e), "the spawn touch #282 must fire ch 0x7E [D]");
    // Both objectives announced (state 3) — enable 0 for both [L c07000011.rs].
    let (i, _) = nf_game::mission::LEVELS.iter().enumerate().find(|(_, d)| d.level == 0x0700_0011).unwrap();
    let st = &sim.drones.mission.states[i];
    assert_eq!(st[0], 3, "'Escape from the tower' announced at level start");
    assert_eq!(st[1], 3, "'Gain access to lower office floor' announced at level start");
    assert!(!sim.drones.channels.on(0x48), "ch 0x48 has no writer on 07000011 [D] — it fires on 07000012");
}

/// The atrium fall: dropping into the void crosses the y=42 planes → ch 0x7C → the fall cut-scene
/// 0x0600098A → its event 0xD [0x62,1] → the mission death restart. Sister: standing on the office
/// floor never raises 0x7C.
#[test]
fn the_atrium_fall_plays_the_death_cutscene_and_restarts() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000011.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    // Sister first: standing on the floor, the fall plates stay off (Bond's capsule is 0.55–1.86 above
    // his feet; the plates are planes at y=42.01 [D]).
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    assert!(!sim.drones.channels.on(0x7c), "standing on the office floor must not fire the fall plates");
    // Drop Bond into the atrium void (plate #314 spans x −20.35…19.18, z 22.10…46.24 at y=42.01 [D]).
    sim.bond.pos = [2.0, 46.0, 30.0];
    sim.bond.vel = [0.0, 0.0, 0.0];
    let mut fired = false;
    for _ in 0..600u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(0x7c) { fired = true; break; }
    }
    assert!(fired, "falling through the atrium must cross the y=42 planes (ch 0x7C) [D]");
    // The fall cut-scene: Bond is disabled by the script (event 4), the script writes ch 0x62 at tick
    // 0x67 of 0x0600098A [D], and the mission layer restarts the level [M 0x182c60 case 1].
    let mut dead = false;
    let mut restarted = false;
    for _ in 0..600u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(0x62) { dead = true; }
        if sim.drones.level_exit == Some(sim.level) { restarted = true; break; }
    }
    assert!(dead, "the cut-scene must write ch 0x62 (player dead) [D 0x0600098A stream 6]");
    assert!(restarted, "Mission_Update must restart the level on the death channel [M 0x182c60]");
}

/// The exit: pressing E at the Load Point #158 loads 07000012. Sister: E with a spent trigger (or
/// elsewhere) does nothing.
#[test]
fn e_at_the_load_point_exits_to_07000012() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000011.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    assert!(wait_free(sim, 400), "Bond is free after load");
    // Sister: E far from any exit must not end the level.
    for _ in 0..3 { sim.step_bond(Input::default(), false, false, true); }
    assert_eq!(sim.drones.level_exit, None, "E in the open must not fire an exit");
    // Walk (teleport) up to the Load Point #158 [D (14.58, 16.41, −3.94), a plate only 0.05 thick in
    // z] and press E facing −z: the use sphere (head + look·1.0 [M 0x1a5170]) must meet the volume.
    let exit = sim.drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.next_level == 0x0700_0012 && t.placement == 158)
        .expect("the Load Point #158 exists").clone();
    sim.bond.pos = [(exit.lo[0] + exit.hi[0]) * 0.5, exit.lo[1] + 0.2, exit.hi[2] + 1.0];
    sim.bond.yaw = std::f32::consts::PI;
    for _ in 0..6 { sim.step_bond(Input::default(), false, false, true); if sim.drones.level_exit.is_some() { break; } }
    assert_eq!(sim.drones.level_exit, Some(0x0700_0012), "E at the Load Point loads 07000012 [M 0x1b2318]");
    // The reload path (the viewer's own next-level step).
    let next = nf_view::level_archive_name(0x0700_0012).to_string();
    let scene2 = nf_view::load(&iso, &next, false).expect("07000012 must load after the exit");
    assert_eq!(scene2.sim.as_ref().expect("sim").level, 0x0700_0012);
}
