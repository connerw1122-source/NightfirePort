//! map-tower2 walkthrough: 07000013 "HT Tower2 C" — the tower lobby, reached warm through the mission
//! chain 07000011 → 07000012 → 0700004A (the connector lobby; part 2 of mission 0x07000011 [L
//! crates/nf-game/src/mission/c0700004a.rs]).
//! Run with: `NF_ISO=… cargo test -p nf-view --test walk_07000013_live -- --nocapture --test-threads=1`
//!
//! The chain [D placements; M decompile addresses cited inline]:
//! 1. 07000012's exit (E8 #669, gate 0x1F [D]) loads 0700004A. Its start #43 (4.26, −7.71, −4.76)
//!    sits INSIDE TouchOnce #277 (out 0x4D, y −9.87…−7.65 [D]) — "Escape through the elevator shaft"
//!    (ch 0x4D, the 07000012 objective) completes as the lift passes the lobby floor.
//! 2. 0700004A ends through the hatch cut-scene SP #276 (trigger ch 0x84 ← TouchOnce #275 at the top
//!    floor y −157…−155 [D]); its script 0x060006D3 (pack 0790004A) ends with op 0x12 → the next
//!    level [D nis decode; M Script_EventHandler 0x1d94c0 case 0x12]. The port walks this leg by
//!    carrying the RamSave (channels) — the lift ride itself is a 5040-tick SP (0x06000571 [D]).
//! 3. 07000013's start #9 (2.16, −369.10, 10.96): the intro SP #65 (mode 1, script 0x060006D7 from
//!    pack 07900013 [D levels.md §5.1]) plays; TouchOnces #58 (0x28) and #153 (0x4A) cover the spawn
//!    [D] — ch 0x4A completes 0700004A's "Gain access to lobby" and enables this level's objective.
//! 4. "Raise security gate and escape through front entrance" (0x0400003A, ch 0x50, enable 0x4A [L]):
//!    TouchOnce #156 at the front entrance writes ch 0x50 [D]; the three TouchOnces #73/#164/#165
//!    write ch 0x64 (=100, mission success [M 0x1827d8:119-123]) — with every part's channel on
//!    (0x48, 0x49, 0x4D, 0x4A, 0x50) the monitor raises success and Mission_Update posts "Mission
//!    Complete" and LevelToEndTo = 0x07300013 (the driving interlude, logged and skipped by the port
//!    [M 0x182c60; scene.rs LevelEnd]).
//! 5. The level's own E8 #72 "oTrigger Load level" (next 0x07300013, gate 0x4E) is DEAD DATA: ch 0x4E
//!    has no writer on 07000013 or its packs [D — every trigger out, every kind-5/7 script stream,
//!    every door/pickup param checked]. The mission end runs through the ch-0x64 touches instead.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn free(sim: &mut nf_view::scene::Sim, ticks: u32) -> bool {
    for _ in 0..ticks { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { return true; } }
    !sim.disabled
}

fn mission_state(sim: &nf_view::scene::Sim, level: u32, obj: usize) -> u32 {
    let (i, _) = nf_game::mission::LEVELS.iter().enumerate().find(|(_, d)| d.level == level).unwrap();
    sim.drones.mission.states[i][obj]
}

/// The warm chain into 07000013: 07000012's exit → 0700004A's spawn raises ch 0x4D → the carried
/// RamSave → 07000013, where ch 0x4A fires at the spawn and the objective announces.
#[test]
fn the_warm_chain_arrives_with_all_previous_channels_on() {
    let iso = iso();
    // 07000012: spawn (0x48), the override console (0x49), the shaft touch (0x1F) → exit.
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load 07000012");
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim, 400));
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x48), "the spawn touch fires 0x48 [D]");
    sim.bond.pos = [-1.6, 0.5, 7.2]; sim.bond.yaw = 0.0;
    for _ in 0..6 { sim.step_bond(Input::default(), false, false, true); if sim.drones.channels.on(0x49) { break; } }
    assert!(sim.drones.channels.on(0x49), "the console writes 0x49 [D]");
    sim.bond.pos = [2.2, 0.7, 12.6];
    for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); if sim.drones.level_exit.is_some() { break; } }
    assert_eq!(sim.drones.level_exit, Some(0x0700_004a));
    let save = nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request,
        &sim.drones.inventory, &sim.drones.channels).with_mission(&sim.drones.mission);
    // 0700004A: the start sits above the 0x4D touch [D #277 y −9.87…−7.65] — Bond drops to the lobby
    // floor (≈−8.4) and his capsule enters the volume.
    let mut scene = nf_view::load_with_carry(&iso, "0700004a.bin", false, Some(&save)).expect("load 0700004A");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_004a);
    let mut landed = false;
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); if sim.drones.channels.on(0x4d) { landed = true; break; } }
    eprintln!("[4a] bond after spawn {:?}, ch4d {landed}", sim.bond.pos);
    assert!(landed, "the lobby-floor touch #277 fires 0x4D once Bond lands [D]");
    // fix-objective-banner: the monitor completes the objective the tick after the channel rises.
    for _ in 0..2 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(mission_state(sim, 0x0700_0012, 1), 4, "'Escape through the elevator shaft' completes (late AND)");
    let save = nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request,
        &sim.drones.inventory, &sim.drones.channels).with_mission(&sim.drones.mission);
    // 07000013: warm load — the carried channels survive Mission_Init [M 0x182270 keeps previous parts].
    let mut scene = nf_view::load_with_carry(&iso, "07000013.bin", false, Some(&save)).expect("load 07000013");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0013);
    assert!(sim.drones.channels.on(0x48) && sim.drones.channels.on(0x49) && sim.drones.channels.on(0x4d),
        "the previous parts' channels carry through the warm load");
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x4a), "the spawn touch #153 fires 0x4A [D]");
    assert_eq!(mission_state(sim, 0x0700_004a, 0), 4, "'Gain access to lobby' completes (late AND)");
    assert_eq!(mission_state(sim, 0x0700_0013, 0), 3, "'Raise security gate…' announces (enable 0x4A)");
}

/// The mission end: walking the front entrance fires ch 0x50 (the objective) and the ch-0x64 touches
/// complete the AND → success → "Mission Complete" → LevelToEndTo 0x07300013 (the driving interlude —
/// the port logs it and leaves level_exit unset). Sisters: skipping the end zone keeps the mission
/// open; the level's own E8 #72 loads the driving code on E regardless of its gate 0x4E [M 0x1b2318
/// case 10: the E arm falls through to the load on both the gate-on and gate-off branches], which the
/// port requests faithfully (the driving level itself is not ported).
#[test]
fn the_front_entrance_completes_the_mission() {
    let iso = iso();
    // Warm chain again (the same three legs).
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load 07000012");
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim, 400));
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    sim.bond.pos = [-1.6, 0.5, 7.2]; sim.bond.yaw = 0.0;
    for _ in 0..6 { sim.step_bond(Input::default(), false, false, true); }
    sim.bond.pos = [2.2, 0.7, 12.6];
    for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); if sim.drones.level_exit.is_some() { break; } }
    let save = nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request,
        &sim.drones.inventory, &sim.drones.channels).with_mission(&sim.drones.mission);
    let mut scene = nf_view::load_with_carry(&iso, "0700004a.bin", false, Some(&save)).expect("load 0700004A");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); if sim.drones.channels.on(0x4d) { break; } }
    let save = nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request,
        &sim.drones.inventory, &sim.drones.channels).with_mission(&sim.drones.mission);
    let mut scene = nf_view::load_with_carry(&iso, "07000013.bin", false, Some(&save)).expect("load 07000013");
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim, 900), "the intro cut-scene (0x060006D7 [D]) releases Bond");
    // Sister: away from the end zone the mission stays open.
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    assert_ne!(sim.drones.mission.state, 6, "no success away from the entrance");
    // Walk the front entrance: #156 (ch 0x50, −8.61…−4.15, 65.09…67.16) then the success touches
    // #73/#164/#165 (ch 0x64) [D].
    sim.bond.pos = [-6.4, -368.0, 66.2];
    let mut success_at = None;
    for t in 0..400u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(0x50) { sim.step_bond(Input::default(), false, false, false); assert_eq!(mission_state(sim, 0x0700_0013, 0), 4, "the objective completes"); } // fix-objective-banner: the monitor is one tick after the channel
        if sim.drones.channels.on(0x64) && success_at.is_none() { success_at = Some(t); }
        if sim.drones.mission.state == 6 { break; }
    }
    assert!(sim.drones.channels.on(0x50), "the entrance touch #156 writes ch 0x50 [D]");
    assert!(success_at.is_some(), "the success touches write ch 0x64 [D]");
    assert_eq!(sim.drones.mission.state, 6, "the AND over all parts raises mission success [M 0x1827d8]");
    // Mission_Update's banner (240 ticks) then LevelEnd → 0x07300013 — a driving code: logged, and
    // level_exit stays unset by the mission layer [M 0x182c60; scene.rs LevelEnd].
    let mut ended = false;
    for _ in 0..400u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.mission.to_end == 0x0730_0013 { ended = true; break; }
    }
    assert!(ended, "LevelToEndTo must be 0x07300013 (the driving interlude) [M 0x182c60]");
    // The level's own E8 #72 [D]: E inside its volume requests 0x07300013 — the game's E arm falls
    // through to the load with the gate 0x4E off (the fallback write, param 3 = 0, is empty)
    // [M 0x1b2318 case 10]. The driving level itself is not ported (mission.md §3).
    let gate_exit = sim.drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.gate == 0x4e)
        .expect("the oTrigger Load level #72 (gate 0x4E) exists").clone();
    sim.bond.pos = [(gate_exit.lo[0] + gate_exit.hi[0]) * 0.5, gate_exit.lo[1] + 0.2, (gate_exit.lo[2] + gate_exit.hi[2]) * 0.5];
    for _ in 0..6 { sim.step_bond(Input::default(), false, false, true); }
    assert_eq!(sim.drones.level_exit, Some(0x0730_0013), "E at the front-gate exit requests the driving interlude [M 0x1b2318]");
}
