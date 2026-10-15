//! The Mission Stats page's data model — `P_NFSTATS_Handler` 0x209938 through
//! `nf_game::stats::stats_rows` (docs/systems/mission-stats.md).
//!
//! Each test drives a headless mission end through the REAL tally path (`Progress::log_*`, the
//! same entry points the Sim calls) and asserts the page's cells — display strings + ids. No
//! ISO: the scoring tables are the committed [L] snapshots in `progress.rs`, so every assertion
//! here can fail on a formula regression (the fail-first sister is the status-gate test, whose
//! assertions die if the game's step-6 zeroing is dropped).

use nf_game::progress::Progress;
use nf_game::stats::{separate_number, stats_rows, timer_string};

/// 07000005's design rows ([L] ee_start_05 0x2bc640 entry 1): (count, target) per row.
const EXPECTED_DESIGN: [(u32, u32); 9] = [
    (2, 50000), (30, 50000), (3, 25000), (3, 25000), (30, 25000),
    (100, 50000), (100, 50000), (540, 25000), (7, 150000),
];

#[test]
fn separate_number_groups_like_the_game() {
    // [M SeparateNumber 0x1dec78: %d / %d,%.3d / %d,%.3d,%.3d]
    assert_eq!(separate_number(0), "0");
    assert_eq!(separate_number(887), "887");
    assert_eq!(separate_number(25000), "25,000");
    assert_eq!(separate_number(170887), "170,887");
    assert_eq!(separate_number(295887), "295,887");
    assert_eq!(separate_number(1234567), "1,234,567");
    assert_eq!(separate_number(u32::MAX), "4294,967,295"); // the millions group is v/1e6 (integer div)
}

#[test]
fn timer_string_formats_mmss_like_the_game() {
    // [M Timer_Seconds2String 0x1deaa8 mode 0: round-half-up seconds, %.2d:%.2d]
    assert_eq!(timer_string(0), "00:00");
    assert_eq!(timer_string(7900), "01:19"); // the capture's current
    assert_eq!(timer_string(54000), "09:00"); // the capture's target (par × 100)
    assert_eq!(timer_string(-5), "00:00"); // [M the negative arm]
    assert_eq!(timer_string(360000), "01:00:00"); // the hour branch [S — args dropped]
}

/// A known mission end, every cell asserted. The tally: 1 Bond moment, 2 dispatched, 1 subdued,
/// 0 surrendered, never detected, 10/10 shots, full health, 01:19 elapsed, no bonus pickups;
/// Agent (difficulty 1 → the ×0.0 multiplier [M 0x1984b0:294-300]).
#[test]
fn known_mission_end_produces_the_handlers_rows() {
    let mut p = Progress::default();
    p.reset_for_mission();
    // The real log path, behind the mission gate (ms 1 = playing [M the Log family's gate]).
    p.done_bond_moment(1, 0x65); // rows[0].value = 1 (of 2)
    p.log_enemy_dispatched(1);
    p.log_enemy_dispatched(1);
    p.log_enemy_disabled(1);
    for _ in 0..10 {
        p.log_shot_fired(1);
        p.log_shot_hit_enemy(1);
    }
    p.log_health(1, 100.0);
    p.log_update_elapsed(7900); // 01:19 in 1/100 s

    let rows = stats_rows(&mut p, 0x0700_0005, 6, 1, None).expect("07000005 is a ScoringTable hash");
    let cell = |id: u32| rows.iter().find(|r| r.id == id).unwrap_or_else(|| panic!("row {id:#x} missing"));

    // The row order + ids, with the Opponents group row and the blank spacer [M 0x209938:60-123].
    let ids: Vec<u32> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![0x165, 0x15a, 0x15b, 0x15c, 0x16e, 0x15d, 0x15e, 0x15f, 0x160, 0x161, 0, 0x162, 0x164, 0x163]);

    // Design data intact ([L] the static rows): Target cells carry rows[].count.
    for (i, (count, _)) in EXPECTED_DESIGN.iter().enumerate() {
        assert_eq!(p.table.iter().find(|e| e.hash == 0x0700_0005).unwrap().rows[i].count, *count, "row {i} count");
    }

    assert_eq!(cell(0x165).current, "1");
    assert_eq!(cell(0x165).target, "2");
    assert_eq!(cell(0x165).total, "25,000"); // ceil(50000·1/2)
    assert_eq!(cell(0x15a).category, "<0x15a>"); // no bank: the port's bind fallback
    assert_eq!(cell(0x15a).total, ""); // the group row carries no cells [M the msg-0x10 send]
    assert_eq!(cell(0x15b).current, "2");
    assert_eq!(cell(0x15b).target, "30");
    assert_eq!(cell(0x15b).total, "3,334"); // ceil(50000·2/30) — the game's 1e-5 ceil [M]
    assert_eq!(cell(0x15b).indented, true);
    assert_eq!(cell(0x15c).current, "1");
    assert_eq!(cell(0x15c).total, "8,334"); // ceil(25000·1/3)
    assert_eq!(cell(0x16e).current, "0");
    assert_eq!(cell(0x16e).total, "0");
    assert_eq!(cell(0x15d).current, "100%"); // 0 detections ≤ 30 allowed → ratio 1.0 [M]
    assert_eq!(cell(0x15d).target, "----"); // the −1 target [M 0x201df0]
    assert_eq!(cell(0x15d).total, "25,000");
    assert_eq!(cell(0x15e).current, "100%"); // 10/10 shots
    assert_eq!(cell(0x15e).target, "100%"); // the accuracy par 100 [M the +0x64 read]
    assert_eq!(cell(0x15e).total, "50,000");
    assert_eq!(cell(0x15f).current, "100%");
    assert_eq!(cell(0x15f).target, "100%"); // the literal [M 0x209938:105-108]
    assert_eq!(cell(0x15f).total, "50,000");
    assert_eq!(cell(0x160).current, "01:19"); // Timer_Seconds2String(7900)
    assert_eq!(cell(0x160).target, "09:00"); // Timer_Seconds2String(540 × 100)
    assert_eq!(cell(0x160).total, "170,887"); // ceil(25000 · 540 / 79) — the capture's number
    assert_eq!(cell(0x161).current, "0");
    assert_eq!(cell(0x161).target, "7");
    assert_eq!(cell(0x161).total, "0");
    assert_eq!(cell(0x162).total, "332,555"); // Σ raw: 25000+3334+8334+0+25000+50000+50000+170887
    assert_eq!(cell(0x164).total, "0"); // Agent: ×0.0 [M ScoringTableDifMul[0]]
    assert_eq!(cell(0x163).total, "332,555"); // base + bonus
}

/// The PCSX2 capture on 07000005 (the task's golden, numbers only): a clean 01:19 run —
/// nothing killed, nothing collected, full health, perfect accuracy — at Agent. Every cell
/// matches the real page.
#[test]
fn the_pcsx2_capture_is_the_golden() {
    let mut p = Progress::default();
    p.reset_for_mission();
    for _ in 0..10 {
        p.log_shot_fired(1);
        p.log_shot_hit_enemy(1);
    }
    p.log_health(1, 100.0);
    p.log_update_elapsed(7900);

    let rows = stats_rows(&mut p, 0x0700_0005, 6, 1, None).unwrap();
    let cell = |id: u32| rows.iter().find(|r| r.id == id).unwrap();
    assert_eq!(cell(0x165).current, "0");
    assert_eq!(cell(0x165).total, "0");
    assert_eq!(cell(0x15b).total, "0");
    assert_eq!(cell(0x15c).total, "0");
    assert_eq!(cell(0x16e).total, "0");
    assert_eq!(cell(0x15d).current, "100%");
    assert_eq!(cell(0x15d).target, "----");
    assert_eq!(cell(0x15d).total, "25,000");
    assert_eq!(cell(0x15e).current, "100%");
    assert_eq!(cell(0x15e).target, "100%");
    assert_eq!(cell(0x15e).total, "50,000");
    assert_eq!(cell(0x15f).current, "100%");
    assert_eq!(cell(0x15f).target, "100%");
    assert_eq!(cell(0x15f).total, "50,000");
    assert_eq!(cell(0x160).current, "01:19");
    assert_eq!(cell(0x160).target, "09:00");
    assert_eq!(cell(0x160).total, "170,887");
    assert_eq!(cell(0x161).total, "0");
    assert_eq!(cell(0x162).total, "295,887"); // the capture's Subtotal
    assert_eq!(cell(0x164).total, "0"); // the capture's Difficulty Bonus at Agent
    assert_eq!(cell(0x163).total, "295,887"); // the capture's TOTAL = Subtotal
}

/// The fail-first sister: the game zeroes every runtime field when the status isn't 6
/// [M 0x1984b0:316-333] — the page on a non-6 status shows the design data with zero scores.
/// If the zeroing gate is ever dropped, these asserts die.
#[test]
fn a_non_6_status_zeroes_the_scores() {
    let mut p = Progress::default();
    p.reset_for_mission();
    p.log_enemy_dispatched(1);
    for _ in 0..10 {
        p.log_shot_fired(1);
        p.log_shot_hit_enemy(1);
    }
    p.log_health(1, 100.0);
    p.log_update_elapsed(7900);

    let rows = stats_rows(&mut p, 0x0700_0005, 1, 1, None).unwrap(); // status 1 = playing
    let cell = |id: u32| rows.iter().find(|r| r.id == id).unwrap();
    assert_eq!(cell(0x165).current, "0", "the tally's value is zeroed [M 0x1984b0:316-333]");
    assert_eq!(cell(0x15b).current, "0");
    assert_eq!(cell(0x160).current, "00:00");
    assert_eq!(cell(0x160).total, "0");
    assert_eq!(cell(0x15d).current, "0%");
    assert_eq!(cell(0x162).total, "0");
    assert_eq!(cell(0x163).total, "0");
    // The design data stays: Target cells keep the static rows.
    assert_eq!(cell(0x160).target, "09:00");
    assert_eq!(cell(0x15b).target, "30");
}

/// The 0x09-prefixed hashes hide the stealth trio (the +0x38 flag) [M 0x209938:68] — here with
/// 09000005's NewScores rows.
#[test]
fn new_scores_levels_hide_the_stealth_rows() {
    let mut p = Progress::default();
    let rows = stats_rows(&mut p, 0x0900_0005, 6, 1, None).unwrap();
    let ids: Vec<u32> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![0x165, 0x15a, 0x15b, 0x15e, 0x15f, 0x160, 0x161, 0, 0x162, 0x164, 0x163]);
    assert!(!ids.contains(&0x15c) && !ids.contains(&0x16e) && !ids.contains(&0x15d));
}

/// A hash outside the table → the game's `score_table == 0` arm: no page [M 0x209938:38-40].
#[test]
fn an_unknown_hash_draws_nothing() {
    let mut p = Progress::default();
    assert!(stats_rows(&mut p, 0x0700_0048, 6, 1, None).is_none());
}
