//! difficulty-effects: the front-end → sim difficulty wiring, end to end.
//!
//! The game writes the difficulty page's pick into GameState+0x28 (0x2a3790) at confirm time
//! [M `C_SBNFDFCTY_Handler` 0x216ca8:15 — `GameState._40_4_ = difficulty[idx*0x18+0xc]`]
//! and every later level load reads it (the create-time gates: `Drone_Create` 0x136be0:49,
//! `captain_setup` 0x14b300:504-535, `Lock_Create` 0x1ae1f8:67, `Copter_Create` 0x17d340:76,
//! `Hint_Create` 0x1b3248). The port's chain: the wheel's Confirm sets
//! `FrontEnd::requested_difficulty`, the App adopts it into `selected_difficulty`, and
//! `load_with_carry_difficulty` builds the Sim with it.

use nf_view::menu::{Action, FrontEnd, Page};

/// The wheel's three rows are the game's difficulty values 1/2/3 — Operative / Agent /
/// 00 Agent [M difficulty table 0x2df4c0: rows' +0x0c = 1, 2, 3].
#[test]
fn difficulty_confirm_records_the_pick() {
    for (cursor, want) in [(0usize, 1u32), (1, 2), (2, 3)] {
        let mut fe = FrontEnd::new();
        fe.page = Page::Difficulty;
        fe.cursor = cursor;
        fe.action(Action::Confirm);
        assert_eq!(fe.requested_difficulty, Some(want), "cursor {cursor} must record difficulty {want}");
    }
    // Sister: the pre-task code recorded nothing (the field did not exist) — the page's
    // Confirm must never leave the pick unset.
    let mut fe = FrontEnd::new();
    fe.page = Page::Difficulty;
    fe.cursor = 2;
    fe.action(Action::Confirm);
    assert_ne!(fe.requested_difficulty, None);
}

/// `load_with_carry_difficulty` builds the whole Sim with the picked difficulty: the runtime
/// mirror and every create-time effect (the drone roster shrinks on Operative — 07000011's
/// main map tags 13 of its 37 0x0F placements param2=2 and 4 more param2=3).
#[test]
fn front_end_pick_drives_the_sim_load() {
    let Some(iso) = std::env::var_os("NF_ISO").map(|p| p.to_string_lossy().into_owned()) else {
        eprintln!("skipping: NF_ISO not set");
        return;
    };
    let d1 = nf_view::load_with_carry_difficulty(&iso, "07000011.bin", false, None, 1).unwrap();
    let d3 = nf_view::load_with_carry_difficulty(&iso, "07000011.bin", false, None, 3).unwrap();
    let (s1, s3) = (d1.sim.as_ref().unwrap(), d3.sim.as_ref().unwrap());
    assert_eq!(s1.drones.difficulty, 1);
    assert_eq!(s3.drones.difficulty, 3);
    assert_eq!(s3.drones.progress.difficulty, 3, "the Progress mirror follows");
    // The create-time roster: 20 guards at Operative, 37 at 00 Agent (level_drones' gate
    // counts, verified in difficulty_spawn_gate_live.rs). `list` = the created roster;
    //  = the game's 0x298E14 linked list (created + spawned, placement order).
    assert_eq!(s1.drones.list.len(), 20, "Operative roster");
    assert_eq!(s3.drones.list.len(), 37, "00 Agent roster");
    // Sister: the pre-task build hardcoded difficulty 1 — a d3 load's mirror would read 1.
    assert_ne!(s1.drones.difficulty, s3.drones.difficulty);
}
