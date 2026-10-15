//! objective-hints — the pause OBJECTIVES tab's row source: `Mission::num_vis_objectives()`
//! (`Mission_NumVisObjectives` 0x182540) and `Mission::objective_state()` (`Mission_ObjectiveState`
//! 0x182630). The walk: LEVELS table order, skipping other missions' records, STOPPING at the
//! first record of the mission whose part is beyond the current one, counting each objective
//! whose runtime state is > 1 (announced: 2 fail-objective / 3 active / 4 complete / 5 failed
//! [M 0x1827d8's state machine]), carrying record+0x00 (text) and record+0x08 (hint).
//!
//! The data is the embedded per-level records [L ee_start_05.bin — crates/nf-game/src/mission/
//! c0*.rs]; mission 0x07000009's three records (parts 0/1/2 = levels 0x07000009/0a/0b) carry the
//! hints 0x04000021..0x04000029 [L]. Pure headless: no ISO.

use nf_game::mission::{Mission, LEVELS};

/// LEVELS' table index of a level's record [L the MissionData table order].
fn level_index(level: u32) -> usize {
    LEVELS.iter().position(|l| l.level == level).expect("the level has a MissionData record")
}

#[test]
fn a_fresh_mission_lists_no_objectives() {
    // Mission_Init's fresh states are 0 (pending) [M 0x182270:98-105] — nothing is announced, so
    // the tab lists nothing (the game's populate then falls to the no-objectives arm).
    for (base, order) in [(0x0700_0009u32, 0u32), (0x0700_0005, 0), (u32::MAX, 0)] {
        let m = Mission::new();
        let mut m = m;
        m.base_map = base;
        m.order = order;
        assert_eq!(m.num_vis_objectives(), 0, "no announced objective yet for {base:#x}");
        // Sister: the row getter is empty too.
        assert!(m.objective_state(0).is_none(), "no first row for {base:#x}");
    }
}

#[test]
fn announced_rows_come_in_table_order_with_their_text_and_hint() {
    let mut m = Mission::new();
    m.base_map = 0x0700_0009;
    m.order = 0; // 0x07000009 = part 0: 3 objectives, hints 0x04000021-23 [L c07000009.rs]
    let li = level_index(0x0700_0009);
    // Announce the first two; the third stays pending (state 1 with its enable channel).
    m.states[li][0] = 3;
    m.states[li][1] = 3;
    m.states[li][2] = 1;
    assert_eq!(m.num_vis_objectives(), 2, "the announced pair lists, the pending one does not");
    let first = m.objective_state(0).expect("a first row");
    assert_eq!((first.state, first.text, first.hint), (3, 0x0400_0016, 0x0400_0021),
        "row 0 = record+0x14/+0x00/+0x08 of the first announced objective [M 0x182630]");
    let second = m.objective_state(1).expect("a second row");
    assert_eq!((second.state, second.text, second.hint), (3, 0x0400_0017, 0x0400_0022));
    // Sister: past the end is None (the caller clamps; the game reads stack garbage there).
    assert!(m.objective_state(2).is_none(), "no third row while the third objective is pending");
}

#[test]
fn prior_parts_rows_come_first_and_future_parts_are_cut() {
    let mut m = Mission::new();
    m.base_map = 0x0700_0009;
    // Every objective of parts 0 and 1 announced; part 2's too (it would list only once order
    // reaches 2).
    for level in [0x0700_0009u32, 0x0700_000a, 0x0700_000b] {
        let li = level_index(level);
        for s in m.states[li].iter_mut() { *s = 3; }
    }
    m.order = 1; // on 0x0700000a (part 1): parts 0 and 1 list, part 2 (0x0700000b) does not
    assert_eq!(m.num_vis_objectives(), 6, "parts 0+1 = 6 announced rows [M the part gate]");
    let first = m.objective_state(0).expect("a first row");
    assert_eq!(first.text, 0x0400_0016, "the PRIOR part's rows come first [M table order]");
    let last = m.objective_state(5).expect("a sixth row");
    assert_eq!((last.text, last.hint), (0x0400_001b, 0x0400_0026), "part 1's last objective");
    // Sister: the walk stops at the future record — a 7th row does not exist.
    assert!(m.objective_state(6).is_none(), "part 2's objectives are beyond the gate");
    // Advancing the current part admits the next record's rows.
    m.order = 2;
    assert_eq!(m.num_vis_objectives(), 11, "all three parts once order reaches 2 (3+3+5 [L c0*.rs])");
}

#[test]
fn other_missions_records_never_leak_into_the_walk() {
    let mut m = Mission::new();
    m.base_map = 0x0700_0009;
    m.order = 2;
    // Announce EVERYTHING (all 24 records' objectives) — only mission 0x07000009's 11 rows list.
    for row in m.states.iter_mut() { for s in row.iter_mut() { *s = 3; } }
    assert_eq!(m.num_vis_objectives(), 11, "only the mission's own records count [M mission != BaseMap skips]");
    for i in 0..11 {
        let o = m.objective_state(i).expect("a row");
        assert!((0x0400_0016..=0x0400_0020).contains(&o.text), "row {i} is a 0x07000009 objective, not another mission's");
    }
    // Sister: the neighbouring mission's texts (0x04000000 "Breach the castle walls") are absent.
    assert!(m.objective_state(11).is_none());
}

#[test]
fn completed_and_failed_rows_still_list_with_their_state() {
    let mut m = Mission::new();
    m.base_map = 0x0700_0009;
    m.order = 0;
    let li = level_index(0x0700_0009);
    m.states[li][0] = 4; // complete — the ~J marker [M the populate's state == 4]
    m.states[li][1] = 5; // failed — still listed (> 1 [M 0x182540's count])
    m.states[li][2] = 2; // a pending fail-objective — listed too
    assert_eq!(m.num_vis_objectives(), 3, "states 4/5/2 are all visible");
    assert_eq!(m.objective_state(0).unwrap().state, 4);
    assert_eq!(m.objective_state(1).unwrap().state, 5);
    assert_eq!(m.objective_state(2).unwrap().state, 2);
    // Sister: a 0-state row would drop the count to 2.
    m.states[li][0] = 0;
    assert_eq!(m.num_vis_objectives(), 2, "a re-armed (pending) row leaves the list");
}
