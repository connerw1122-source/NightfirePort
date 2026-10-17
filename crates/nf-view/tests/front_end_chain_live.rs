//! front-end-chain: the front-end leg after a mission ends (docs/reports/campaign-flow.md
//! blocker 7): the results/medal page (P_NFRESULTS 0x208e78) with the game's own fields, the
//! unlock write into the progress layer (Menu_SetLevelBonus 0x1ffa10 → sp_level+0x10, packed by
//! Menu_GetNightfireStatus 0x202360), and the mission select offering the next mission
//! (P_NFMAP 0x20b608 / C_SBNFMAP 0x20b8d8).

use nf_view::menu::{Action, FrontEnd, Outcome, Page, Stats};

/// P_NFRESULTS 0x208e78 case 0x4b: Confirm routes to the mission list 0x4000001c
/// (`Manager_SendMessage(0x44, 0x4000001c, 1)` [M :39-44]) — but ONLY through the gate chain:
/// (1) the DoneBetter save box swallows the press first — `PlrStats_DoneBetter() != 0 &&
/// DAT_002e1ae0 > 1` opens the 0x1000206 box and RETURNS [M :29-34]; (2) an improved score with
/// no box only refreshes the codename [M :35-38]; (3) the list route needs DAT_002e1c2c == 0 (no
/// fresh reward standing) AND just_completed == 0 [M :39-46]; a fresh reward routes to the bonus
/// page 0x40000038 [M :49-52] and the final mission (just_completed) to the win page
/// 0x40000053 [M :46]. The flag-poke sisters for branches 3 live in unlock_popups.rs; this test
/// pins the plain complete-mission walk with its preconditions pinned before the press.
#[test]
fn the_completed_results_page_continues_to_the_mission_list() {
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0004; // 01's final part — the outro wraps 0x0730004 [M 0x182c60:89-91]
    fe.mission_ended(Outcome::Complete, Stats::default());
    assert_eq!(fe.page, Page::Results);
    assert_eq!(fe.flow.page, 0x4000_0036);
    // The outro rode the load request [M Mission_Update 0x182c60 case 6 + case 4's
    // ResetMap_LevelToLoad(LevelToEndTo)] — the host consumes it for the movie-wrapped load.
    assert_eq!(fe.requested_level.take(), Some(0x0730_0004));
    // The three gates stand open on this path [M 0x208e78:29-46]: no improved score (no DoneBetter
    // box, :29-34), no fresh reward (DAT_002e1c2c == 0, :39), not the final mission
    // (just_completed == 0, :46 — 0x0700004 != 0x0700001b).
    assert!(!fe.results_box && !fe.results_improved, "no save box stands [M :29-34]");
    assert!(!fe.new_reward, "no reward standing [M :39-48]");
    assert!(!fe.just_completed, "not the final mission [M :46]");
    fe.cursor = 0;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MissionSelect, "Confirm → the mission list [M 0x208e78:43]");
    assert_eq!(fe.flow.page, 0x4000_001c);
    assert_eq!(fe.requested_level, None, "a page change, not a level load [M 0x208e78:43]");
}

/// Sister: the FAILED page keeps P_ENDMISSION's three arms [M 0x216618:19-27] — row 0 restarts
/// the CURRENT level warm. The complete-page rerouting must not touch the fail menu.
#[test]
fn the_failed_page_keeps_the_fail_menu_arms() {
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0004;
    fe.mission_ended(Outcome::Failed, Stats::default());
    assert_eq!(fe.page, Page::Results);
    assert_eq!(fe.rows(), 2, "the fail menu's REPLAY/QUIT rows [L page 0x40000042: items 0x5bc (value 0) and 0x16c (value 2)]");
    // fe-audit: a failure loads nothing — the fail menu opens over the paused mission [M Mission_Update
    // 0x182c60:77-80; cases 2/3 (:45-64) only set LevelToEndTo]. The load comes from the row.
    assert_eq!(fe.requested_level, None, "the failure itself requests no level [M 0x182c60:77-80]");
    fe.cursor = 0;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(0x0700_0004), "row 0 = the current level [M 0x216618:19-21]");
    assert!(fe.flow.warm_reset, "row 0 is the warm arm [M 0x216618:20]");
}
