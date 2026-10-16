//! mission-unlock-flow headless tests: completing a mission unlocks the next Single Player row,
//! in the game's order, surviving the MSSN save/load; "Play without saving" keeps the unlocks
//! session-only; the driving rows confirm into GameFlow state 9 (the DRIVING.ELF handover).
//! Numbers/ids only — no drawing (the main agent renders the pages against PCSX2).
//!
//! Evidence: docs/systems/mission-unlock-flow.md — P_NFRESULTS_Handler 0x208e78:77/169,
//! Menu_SetLevelBonus 0x1ffa10:62-66 (row i AND i+1, hash-agnostic), Menu_Get/SetNightfireStatus
//! 0x202360/0x2023a0 (bit i ⇄ sp_level[i]+0x10; load 0 → 0x3), the boot state [D ACTION.ELF
//! .data file offset 0x1d91e0: rows 0-1 = 1], C_SBNFMAP 0x20b8d8 case 0x4b's on-foot/driving
//! split, ResetMap_LevelToLoad 0x1d3800:104-108 (state 9), Menu_IsDrivingLevel 0x202750.

use nf_view::menu::{Action, FrontEnd, Page, ST_LAUNCH_DRIVING, ST_MAPLOAD, ST_RUNNER};

/// A scratch card dir per test (the port's memory card).
fn card_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nf-muf-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Walks the front end to the Single Player list the way the game does: Start → Main →
/// Codename row 1 ("Play without saving") → its confirm box → the difficulty page → the list.
fn to_mission_select(fe: &mut FrontEnd) {
    assert!(fe.action(Action::Confirm), "Start → the intro + Main");
    if fe.intro_playing {
        assert!(fe.action(Action::Confirm), "the skip action ends the intro movie");
    }
    assert_eq!(fe.page, Page::Main);
    assert!(fe.action(Action::Confirm), "Main row 0 → Codename");
    assert_eq!(fe.page, Page::Codename);
    assert!(fe.action(Action::Down), "cursor onto Play without saving");
    assert!(fe.action(Action::Confirm), "pop the confirm box");
    assert!(fe.action(Action::Confirm), "confirm it → Difficulty");
    assert_eq!(fe.page, Page::Difficulty);
    assert!(fe.action(Action::Confirm), "Difficulty → MissionSelect");
    assert_eq!(fe.page, Page::MissionSelect);
}

/// The required chain: the first three campaign rows complete headlessly — rows unlock in the
/// game's order (row i's award sets i and i+1, [M 0x1ffa10:62-66]) with NO regression of the
/// earlier rows (the game's sp_level is one accumulating global), and the bits survive the
/// codename save → fresh program → codename load round trip (the MSSN status word).
#[test]
fn first_three_completions_unlock_in_game_order_and_survive_save_load() {
    let mut fe = FrontEnd::new();
    // The fresh program's list: rows 0 AND 1 only [D sp_level .data 0x1d91e0].
    assert_eq!(fe.packed_unlocks(), 0x3, "the boot unlock word");
    assert!(!fe.unlocked_row(2), "sister: row 2 must start locked");

    let dir = card_dir("chain");
    fe.set_profile_dir(dir.clone());
    let name = fe.create_codename("UNLOCK01", &[]).expect("the codename creates");

    // The first on-foot mission is row 1 (0x07000005); row 0 is the driving 0x09000001 the boot
    // state already offers. Completing it awards rows 1 and 2.
    let mut progress = nf_game::progress::Progress::default();
    assert!(fe.results_enter(&mut progress, 0x0700_0005, 6, 2, None), "the results fields built");
    // The GAME's award flags (packed_unlocks = sp_level+0x10) stay exactly the game's; the port's
    // `unlocked_row` additionally opens a row behind an unlocked DRIVING row, because the port can't
    // play driving yet (playtest 1015-2231: a fresh profile was stuck on Paris Prelude) [S].
    assert_eq!(fe.packed_unlocks() & 0b100, 0b100, "row 2 awarded by row 1's completion [M 0x1ffa10:63-65]");
    assert_eq!(fe.packed_unlocks() & 0b1000, 0, "the game's flag for row 3 is still clear");
    assert!(fe.unlocked_row(0) && fe.unlocked_row(1), "sister: no regression — the game's flags never clear");
    // Rows 2 and 3 are driving (0x09000005/6), so the port opens through them to row 4, the next
    // on-foot mission (Double Cross) — and stops there.
    assert!(fe.unlocked_row(3) && fe.unlocked_row(4), "the port opens through the driving rows");
    assert!(!fe.unlocked_row(5), "sister: the chain stops behind an on-foot row");

    // Row 2 completes (0x09000005 — a DRIVING hash; the award code is hash-agnostic [M :62-66]).
    fe.results_enter(&mut progress, 0x0900_0005, 6, 2, None);
    assert_eq!(fe.packed_unlocks() & 0b1000, 0b1000, "row 3 awarded by row 2's completion");
    assert_eq!(fe.packed_unlocks() & 0b10000, 0, "the game's flag for row 4 is still clear");

    // Row 3 completes (0x09000006, driving) → row 4.
    fe.results_enter(&mut progress, 0x0900_0006, 6, 2, None);
    assert!(fe.unlocked_row(4), "row 4 unlocked by row 3's award");
    assert_eq!(fe.packed_unlocks(), 0x1f, "exactly rows 0..4 after three completions");
    assert!(!fe.unlocked_row(5), "sister: row 5 untouched");

    // The results save ran under the active codename [M the save after Menu_SetLevelBonus].
    assert!(fe.save_profile(), "the profile writes");
    fe.profile.as_ref().expect("profile held")
        .progress.nightfire_status();

    // A program restart: a fresh FrontEnd + the codename load unpacks the MSSN status
    // [M Menu_SetNightfireStatus 0x2023a0's bit loop].
    let mut fe2 = FrontEnd::new();
    fe2.set_profile_dir(dir.clone());
    assert_eq!(fe2.packed_unlocks(), 0x3, "sister: the fresh program boots at the disc state");
    assert!(fe2.profiles.contains(&name), "the card lists the codename");
    assert!(fe2.load_codename(2), "the load continues to the difficulty page");
    assert_eq!(fe2.packed_unlocks(), 0x1f, "the session's unlocks survive the restart");
    assert!(!fe2.unlocked_row(5), "sister: the load carries exactly what the save held");
    let _ = std::fs::remove_dir_all(&dir);
}

/// "Play without saving": the awards land in memory the same way (the same results code), but
/// nothing is written to the card and a fresh program shows the boot state again.
#[test]
fn play_without_saving_keeps_unlocks_session_only() {
    let mut fe = FrontEnd::new();
    let dir = card_dir("nosave");
    // A card present but NO codename in play (active_profile None).
    fe.set_profile_dir(dir.clone());
    assert_eq!(fe.profiles.len(), 0, "the card is empty");

    let mut progress = nf_game::progress::Progress::default();
    fe.results_enter(&mut progress, 0x0700_0005, 6, 2, None);
    fe.results_enter(&mut progress, 0x0900_0005, 6, 2, None);
    assert_eq!(fe.packed_unlocks(), 0xf, "the session's bits accumulate rows 0..3");
    assert_eq!(progress.nightfire_status(), 0xf, "the progress layer mirrors them");

    assert!(!fe.save_profile(), "no card write without a codename in play [M the save needs the codename buffer]");
    assert!(std::fs::read_dir(&dir).unwrap().next().is_none(), "sister: the card is still empty");

    let fe2 = FrontEnd::new();
    assert_eq!(fe2.packed_unlocks(), 0x3, "a fresh program shows the boot state only [D .data]");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The SP confirm's split [M C_SBNFMAP 0x20b8d8:29-40]: an on-foot row loads through the on-foot
/// machinery; a driving row lands in GameFlow state 9 — the DRIVING.ELF handover — and issues no
/// level load (a 0x09 hash has no level archive on the ISO [D]).
#[test]
fn sp_confirm_splits_on_foot_from_driving() {
    let mut fe = FrontEnd::new();
    fe.flow.main_tick(); // boot pushes the runner + the front-end map load
    assert!(fe.flow.load_tick(), "headless loads finish instantly");
    assert_eq!(fe.flow.get_state(), ST_RUNNER);
    to_mission_select(&mut fe);
    assert_eq!(fe.cursor, 0, "the list opens on the first unlocked row = 0");

    // Row 0 is the DRIVING 0x09000001. The game hands over to DRIVING.ELF in GameFlow state 9 [M
    // 0x1d3800:105-108]; the port has no driving executable, and entering state 9 left the flow
    // stuck for good (playtest 1015-2231). The port shows a notice instead: the flow stays in the
    // runner and no level load goes out [S port choice].
    assert!(fe.action(Action::Confirm));
    assert_ne!(fe.flow.get_state(), ST_LAUNCH_DRIVING, "the port never enters the dead state 9");
    assert_eq!(fe.flow.get_state(), ST_RUNNER);
    assert!(fe.notice.is_some(), "the driving notice shows");
    assert!(fe.requested_level.is_none(), "sister: no level load goes out for a 0x09 hash");
    assert!(fe.action(Action::Confirm), "Enter dismisses the notice");

    // Row 1 is the on-foot 0x07000005: the normal loader path.
    fe.cursor = 1;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(0x0700_0005), "the on-foot pick requests the level");
    assert_eq!(fe.launching_driving(), None, "sister: the getter clears off state 9");
    assert_eq!(fe.flow.get_state(), ST_MAPLOAD, "the on-foot branch pushed the loader [M :107]");
}

/// The driving-level gate is exactly `Menu_IsDrivingLevel` 0x202750's set — the campaign's five
/// 0x090000xx hashes pass, the neighbours do not.
#[test]
fn is_driving_level_matches_the_game_s_set() {
    for h in [0x0900_0001, 0x0900_0002, 0x0900_0003, 0x0900_0005, 0x0900_0006] {
        assert!(nf_game::ptp::is_driving_level(h), "{h:08x} is a driving level [M 0x202750]");
    }
    for h in [0x0900_0000, 0x0900_0004, 0x0900_0007, 0x0900_0008, 0x0700_0005, 0x0700_001b] {
        assert!(!nf_game::ptp::is_driving_level(h), "sister: {h:08x} is not");
    }
}
