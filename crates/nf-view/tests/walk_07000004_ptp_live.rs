//! drive-ptp: the ACTION↔DRIVING handover on 07000004 — the mission-complete route runs into the
//! PTP block. Run with: `NF_ISO=… cargo test -p nf-view --test walk_07000004_ptp_live -- --nocapture`
//!
//! The chain [M addresses cited inline; the kill route is walk_07000004_live's]:
//! 1. The target's death raises ch 0x54 → the OUT trigger #46 copies it to ch 0x64 (mission
//!    complete) [D placements; M Trigger_Update's OUT case].
//! 2. `Mission_Update` 0x182c60 case 6 sets `MissionState 6` + `LevelToEndTo = 0x7300004` (case
//!    0x7000004) and posts the 240-tick banner; case 7 counts it down; case 4 emits the level end.
//! 3. Before this task the port logged the 0x0730xxxx code and held ("driving level, not
//!    ported") — the mission chain stopped at the castle. Now the view's LevelEnd arm (drained
//!    per step in `hud_tick`) runs `Sim::ptp_handover`: the PTP block (`Boot_GetPTPData`
//!    0x1c8198), the dev stand-in for the drive, and the resume at `Boot_LoadPTPData` 0x1c8560's
//!    unconditional `ResetMap_LevelToLoad(0x7000048, 0, 0)`.

use nf_game::mission::Event;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// Settle the intro + story scenes (the AI re-enabled, ch 0x53 written) — walk_07000004_live's
/// prologue [D probe: settles by ~tick 1600].
fn settle(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000u32 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.npc_enabled && !sim.disabled && sim.drones.channels.on(0x53) { return; }
    }
    panic!("the intro + story scenes must settle");
}

/// Kill the target and step through the mission-complete banner until the handover runs. The
/// per-step `hud_tick` drains `mission_events` (scene.rs 3847), so the LevelEnd arm — and with it
/// `ptp_handover` — fires inside `step_bond` itself; the test polls the result it leaves.
fn run_to_handover(scene: &mut nf_view::scene::Scene) {
    {
        let sim = scene.sim.as_mut().unwrap();
        let target_i = sim.drones.list.iter().position(|d| d.skin == 0x0500_001e).expect("the target drone exists");
        let pos = sim.drones.list[target_i].pos;
        // The grenade at his feet (the game's own Explode_Propagate path [M 0x17c518]).
        sim.drones.grenades.push(nf_game::grenade::Grenade {
            pos: [pos[0], pos[1] + 0.5, pos[2]], velocity: [0.0, 0.0, 0.0], speed: 0.0,
            travelled: 0.0, range: 60.0, damage: 100.0, radius: 3.0, weapon_type: 0x34, mesh: 0x0200_0726, owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false, resting: false,
            stuck: false, seek_target: None,
            phase: None,
        });
    }
    for _ in 0..3000u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.driving_return.is_some() { return; }
    }
    panic!("the mission-complete banner must finish into the PTP handover (case 6 → 7 → 4 [M 0x182c60])");
}

/// The golden path: mission complete → the 0x0730004 interlude → the PTP handover arms the
/// resume at the front end with the success return (result 1 → MissionState 6, the paris key).
#[test]
fn mission_complete_hands_over_through_the_ptp_block() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000004.bin", false).expect("load 07000004");
    assert_eq!(scene.sim.as_ref().unwrap().level, 0x0700_0004);
    settle(&mut scene);
    run_to_handover(&mut scene);
    let sim = scene.sim.as_ref().unwrap();
    // The resume target [M 0x1c8560 tail — the game's unconditional front-end load].
    assert_eq!(sim.drones.level_exit, Some(nf_game::ptp::RESUME_LEVEL));
    assert_eq!(nf_game::ptp::RESUME_LEVEL, 0x0700_0048);
    assert!(sim.drones.level_exit_cold, "the return is a fresh ACTION boot [M 0x1c8560]");
    // The return: the dev Success stands in for FUN_00300058(0x100000, 1) [M 0x174bf0:322].
    let ret = sim.drones.driving_return.expect("the return is stashed for the reload");
    assert_eq!(ret.result, 1);
    assert_eq!(ret.status_byte, 0);
    assert_eq!(ret.mission_state(), 6, "Boot_LoadPTPData: status 0 → Mission_SetStatus(6) [M 0x1c8560:12-20]");
    assert_eq!(ret.next_index, 1, "the castle's interlude → drive index 1 [S — see ptp.rs]");
    assert_eq!(ret.next_key, "paris_mis01", "DRIVING main's switch case 1 [M 0x174bf0:82-93]");
    // The block's key bytes: PTP+0x4f4 holds the 16-byte buffer [M 0x175a28 reads it].
    let block = nf_game::ptp::PtpBlock::build(&nf_game::ptp::BuildInputs {
        map_id: 0x0900_0001,
        game_state_40: sim.drones.difficulty,
        game_state_52: 0,
        stats: sim.drones.driving_stats,
        result: sim.drones.driving_result,
    });
    assert_eq!(&block.level_key[..11], b"paris_mis01");
    assert!(block.level_key[11..].iter().all(|&b| b == 0));
    // The valid magic the DRIVING main gates on [M 0x174bf0:68].
    assert_eq!(block.valid, 1);
    // The mission layer reached the complete state through the real chain [M 0x182c60].
    assert_eq!(sim.drones.mission.state, 6);
}

/// Sister: the dev Fail choice takes the abort path — result 0, MissionState 3 [M 0x174bf0:316
/// + 0x1c8560:16-20], the same resume level.
#[test]
fn the_fail_choice_takes_the_abort_path() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000004.bin", false).expect("load 07000004");
    scene.sim.as_mut().unwrap().drones.driving_result = nf_game::ptp::DriveResult::Fail;
    settle(&mut scene);
    run_to_handover(&mut scene);
    let sim = scene.sim.as_ref().unwrap();
    let ret = sim.drones.driving_return.expect("the return");
    assert_eq!(ret.result, 0, "FUN_00300058(0x100000, 0) [M 0x174bf0:316]");
    assert_eq!(ret.status_byte, 4);
    assert_eq!(ret.mission_state(), 3, "status 4 → Mission_SetStatus(3) [M 0x1c8560:16-20]");
    assert_eq!(sim.drones.level_exit, Some(0x0700_0048), "the resume level does not depend on the result [M 0x1c8560 tail]");
}

/// The stats carried: whatever the session's 12 slots hold rides the return into the next
/// session's slots (`PlrStats_GetDrivingData` → PTP+0x154/+0x184 → `PlrStats_SetDrivingData`
/// [M 0x1c8198:61 / 0x1c8560]).
#[test]
fn the_twelve_stat_slots_ride_the_handover() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000004.bin", false).expect("load 07000004");
    settle(&mut scene);
    // The session earned values before the handover (the port's slots start zero [L ee_start_05.bin]).
    let sim = scene.sim.as_mut().unwrap();
    sim.drones.driving_stats.values[0] = 12;
    sim.drones.driving_stats.values[11] = 3;
    sim.drones.driving_result = nf_game::ptp::DriveResult::Success;
    sim.ptp_handover(0x0730_0004);
    let ret = sim.drones.driving_return.expect("the return");
    assert_eq!(ret.stats.values[0], 12);
    assert_eq!(ret.stats.values[11], 3);
    // The apply mirror (lib.rs runs it after the reload): MissionState + the slots.
    let mut mission = nf_game::mission::Mission::new();
    let mut carried = nf_game::ptp::DrivingStats::default();
    ret.apply(&mut mission, &mut carried);
    assert_eq!(mission.status(), 6);
    assert_eq!(carried.values[0], 12, "the slots survive the round trip [M 0x199ac8/0x199b18]");
    assert_eq!(carried.values[11], 3);
}

/// The interlude map is the one `Mission_Update` case 6 emits [M 0x182c60:88-116] — a live walk
/// of the pairing without the ISO (the pure table).
#[test]
fn the_interlude_table_matches_mission_update() {
    let _ = std::mem::size_of::<Event>; // the Event import is documentation-only here
    for (level, interlude) in [
        (0x0700_0004u32, 0x0730_0004u32), (0x0700_0008, 0x0730_0008), (0x0700_000b, 0x0730_000b),
        (0x0700_000d, 0x0730_000d), (0x0700_000f, 0x0730_000f), (0x0700_0013, 0x0730_0013),
        (0x0700_0016, 0x0730_0017), (0x0700_001b, 0x0730_001b),
    ] {
        assert_eq!(nf_game::ptp::drive_interlude_for(level), Some(interlude));
        assert!(nf_game::ptp::is_driving_interlude(interlude));
        // Every interlude resolves to a reachable drive index 1..8 [S pairing — see ptp.rs].
        let idx = nf_game::ptp::interlude_drive_index(interlude);
        assert!((1..=8).contains(&idx));
        assert_eq!(nf_game::ptp::key_for_index(idx), nf_game::ptp::DRIVING_KEYS[(idx - 1) as usize]);
    }
}
