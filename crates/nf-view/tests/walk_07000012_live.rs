//! map-tower2 walkthrough: 07000012 "HT Tower2 B" — the real path from the level's own data.
//! Run with: `NF_ISO=… cargo test -p nf-view --test walk_07000012_live -- --nocapture --test-threads=1`
//!
//! The chain [D placements; M decompile addresses cited inline]:
//! 1. Cold start #0 (gate 0) at (22.49, 0.19, 19.16) yaw −π/2 [D placements 0x24] — the elevator
//!    from 07000011 deposits Bond here. TouchOnce #679 (out 0x48, mask 8) covers the spawn tile
//!    [D (21.48, 0.23, 18.64)–(23.59, 1.21, 19.62)]: ch 0x48 is "Gain access to lower office floor"
//!    — the previous level's objective, completed late here (the mission monitor runs every level of
//!    mission 0x07000011 [M 0x1827d8]) and the enable of this level's first objective [L c07000012.rs].
//! 2. The security center: TouchOnces #749/#773 (out 0x87, mask 8) at the lift lobby [D] arm ch 0x87;
//!    the override console #731 (TouchOnce, mask 4 = E-interactable, out 0x49, lock 0x87) then writes
//!    ch 0x49 on E [D; M Trigger_Activate 0x1b2318:38-49 the mask&4 arm] — "Activate elevator override
//!    control in security center" (0x04000037) completes.
//! 3. The elevator: Touch #683 (type 2, out 0x1F, mask 8) in the shaft entrance [D] holds ch 0x1F on;
//!    the exit #669 (0xE8, next 0x0700004A, gate 0x1F) fires on the channel [M Trigger_Update case 10
//!    0x1b1a78:162-166] → the lobby connector level 0700004A.
//! 4. Sister assertions: the console refuses E before ch 0x87 (the lock [M 0x1b1a78's lock check]);
//!    skipping the shaft touch leaves 0x1F off and the level never ends.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn free(sim: &mut nf_view::scene::Sim) -> bool {
    for _ in 0..400 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { return true; } }
    !sim.disabled
}

fn mission_state(sim: &nf_view::scene::Sim, level: u32, obj: usize) -> u32 {
    let (i, _) = nf_game::mission::LEVELS.iter().enumerate().find(|(_, d)| d.level == level).unwrap();
    sim.drones.mission.states[i][obj]
}

/// The spawn: ch 0x48 fires under Bond's feet and the previous level's "Gain access to lower office
/// floor" objective completes (late AND, [M 0x1827d8]); this level's first objective announces.
#[test]
fn spawn_touch_completes_the_lower_office_objective() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load 07000012");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0012);
    let p = sim.bond.pos;
    assert!(p[0] > 21.0 && p[0] < 24.0 && p[1] > 0.0 && p[1] < 2.0 && p[2] > 18.0 && p[2] < 20.0,
        "the cold start spawns at #0 [D], got {p:?}");
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x48), "the spawn touch #679 must fire ch 0x48 [D]");
    assert_eq!(mission_state(sim, 0x0700_0011, 1), 4, "07000011's 'Gain access to lower office floor' completes [M 0x1827d8]");
    assert_eq!(mission_state(sim, 0x0700_0012, 0), 3, "'Activate elevator override control' announces (enable 0x48)");
    assert_eq!(mission_state(sim, 0x0700_0012, 1), 1, "'Escape through the elevator shaft' waits for 0x49");
}

/// The override console: E writes ch 0x49 while its lock ch 0x87 is off [M Trigger_Update 0x1b1a78:
/// `if (switch_channels[lock] != 0) return` — the lock DISABLES the trigger while ON], and the
/// objective completes. Sister: once the lobby touches #749/#773 have raised 0x87 the console is dead.
#[test]
fn the_override_console_writes_ch_49_until_the_lobby_locks_it() {
    let iso = iso();
    // Positive: fresh load, E at the console → ch 0x49.
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim));
    let console = sim.drones.triggers.iter()
        .find(|t| t.out == 0x49 && (t.mask & 4) != 0).expect("the override console #731 (mask 4, out 0x49, lock 0x87)").clone();
    assert_eq!(console.lock, 0x87, "the console's lock is ch 0x87 [D]");
    sim.bond.pos = [console.lo[0] + 0.2, console.lo[1] - 0.4, console.lo[2] - 1.0];
    sim.bond.yaw = 0.0; // look +z into the console volume (z 8.07…8.34)
    for _ in 0..6 { sim.step_bond(Input::default(), false, false, true); if sim.drones.channels.on(0x49) { break; } }
    assert!(sim.drones.channels.on(0x49), "E at the console writes ch 0x49 [D]");
    assert_eq!(mission_state(sim, 0x0700_0012, 0), 4, "the override objective completes [M 0x1827d8]");
    assert_eq!(mission_state(sim, 0x0700_0012, 1), 3, "'Escape through the elevator shaft' announces (enable 0x49)");
    // Sister (the lock's real reach): the game's E-activation arm has NO lock read [M 0x1b2318 type-1:
    // mask&4 + out only] — the lock 0x87 gates only the contact path (Trigger_Update 0x1b1a78), which
    // this mask-4 console never takes. So even with ch 0x87 raised, E still works.
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim));
    sim.bond.pos = [2.2, 0.6, 14.4];
    for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x87), "the lobby touch #749 must raise ch 0x87 [D]");
    sim.bond.pos = [console.lo[0] + 0.2, console.lo[1] - 0.4, console.lo[2] - 1.0];
    for _ in 0..6 { sim.step_bond(Input::default(), false, false, true); }
    assert!(sim.drones.channels.on(0x49), "E fires the console even with the lock raised [M 0x1b2318 has no lock read]");
    assert_eq!(mission_state(sim, 0x0700_0012, 0), 4);
}

/// The exit: the shaft touch holds ch 0x1F on, the exit fires to 0700004A. Sister: without entering
/// the shaft area the level never ends.
#[test]
fn the_elevator_shaft_touch_ends_the_level_to_0700004a() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000012.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    assert!(free(sim));
    // Sister: standing at the spawn, no 0x1F, no exit.
    for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); }
    assert!(!sim.drones.channels.on(0x1f) && sim.drones.level_exit.is_none());
    // Walk into the shaft entrance touch #683 (1.14…3.25, 0.20…1.18, 12.10…13.08) [D].
    sim.bond.pos = [2.2, 0.7, 12.6];
    for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); if sim.drones.level_exit.is_some() { break; } }
    assert!(sim.drones.channels.on(0x1f), "the shaft touch #683 must hold ch 0x1F on [D]");
    assert_eq!(sim.drones.level_exit, Some(0x0700_004a), "the gated exit #669 loads the lobby 0700004A [M case 10]");
    // The reload path.
    let next = nf_view::level_archive_name(0x0700_004a).to_string();
    let scene2 = nf_view::load(&iso, &next, false).expect("0700004A must load after the exit");
    assert_eq!(scene2.sim.as_ref().expect("sim").level, 0x0700_004a);
}
