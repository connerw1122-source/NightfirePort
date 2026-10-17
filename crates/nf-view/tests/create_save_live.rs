//! create-save: the user's 2026-10-16 report "you wont let me create a save".
//!
//! Select Codename → New Codename opens the name entry (P_CNNAME 0x40000020, the
//! C_KEYBOARD 0x213a98 grid), the ✓ confirms, the codename is created and saved to the
//! port's memory card (the profile files under %APPDATA%/NightfirePort), and progress then
//! saves after each mission. These tests drive the REAL window path: the front end built the
//! way the window's boot builds it, the keys the window maps (`key_action`'s
//! arrows/Enter/Esc/Tab — no field writes, no teleports), and the card pump the window's
//! redraw runs (`fe.tick()` → `card_tick`). Docs: docs/systems/profile-session.md.
//!
//! The pre-fix reproduce run failed at the create asserts: the ✓ ran nothing on master —
//! no file, no box, no page change — because the window never wired the card
//! (`set_profile_dir` had no caller outside the tests).

use nf_view::menu::{Action, FrontEnd, Outcome, Page, Stats, BTN_YES};
use winit::keyboard::KeyCode::{self, Digit0, KeyA, KeyB, KeyD, KeyE, KeyJ, KeyM, KeyN, KeyO, KeyX};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nf-create-save-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// The window's typed-key branch (lib.rs's CnName arm): the key's grid cell through
/// `cn_press` — the exact call the window makes [docs/systems/profile-session.md].
fn menu_cell_press(fe: &mut FrontEnd, k: KeyCode) -> bool {
    nf_view::menu::cnname_char_key(k).is_some_and(|cell| fe.cn_press(cell))
}

/// One action, then the frames a player would wait: the page-change frames and any intro /
/// attract movie run out (menus-e2e-mp's harness — actions only, never a field assignment).
fn act(fe: &mut FrontEnd, a: Action) -> bool {
    let r = fe.action(a);
    for _ in 0..60 {
        fe.tick();
    }
    for _ in 0..5000 {
        if !fe.intro_playing && !fe.attract_playing {
            break;
        }
        fe.tick();
    }
    r
}

/// The whole journey through the window's action path, on a boot the way the window builds it.
/// The profile card the tests use is the temp dir; the window seeds it at boot through the
/// same `ProfileSession` the real boot runs (the fix's surface — on master NOTHING seeded it,
/// which is exactly the user's bug).
fn walk_to_name_entry(fe: &mut FrontEnd) {
    assert_eq!(fe.page, Page::Start, "the boot opens the title");
    assert!(act(fe, Action::Confirm), "the title's Enter");
    assert_eq!(fe.page, Page::Main, "after P_ESTHERO's movie, the main menu [M 0x20a060:44-45]");
    assert!(act(fe, Action::Confirm), "Main row 0 → the codename page [M C_SBNFCN's open]");
    assert_eq!(fe.page, Page::Codename);
    assert_eq!(fe.cursor, 1, "opened from Main on row 1 [M P_NFSELECT 0x20aec0:96-97]");
    assert!(act(fe, Action::Up), "up to row 0, New Codename");
    assert!(act(fe, Action::Confirm), "row 0 opens the name entry [M 0x20b390:46-49]");
    assert_eq!(fe.page, Page::CnName, "P_CNNAME 0x40000020");
}

/// The user's journey: a fresh boot, Select Codename → New Codename, the game's own proposal
/// "BOND" confirmed with the ✓ — the codename must be created and saved to the card.
/// [M the proposal 0x30d530 = "BOND" (codename-entry.md), the ✓ focuses itself on open
/// [M 0x213790:47-48], the accept arms the save [M 0x213a98:53-65] and the card flow's tick
/// writes the file under the codename [M Menu_UpdateMessageBox 0x1f7de8 case 1].]
#[test]
fn creating_bond_from_a_fresh_boot_saves_it_to_the_card() {
    let dir = temp_dir("bond");
    let mut fe = FrontEnd::new();
    // The window's boot seed (the fix's surface; on master the window ran nothing here).
    nf_view::menu::ProfileSession {
        dir: Some(dir.clone()),
        name: None,
        profile: None,
    }
    .apply_to(&mut fe);

    walk_to_name_entry(&mut fe);
    // The proposal "BOND" stands in the buffer and the ✓ (cell 1002) holds the focus
    // [M 0x213790:36/47-48] — Enter presses the focused cell, the game's X.
    assert_eq!(fe.cn_typed_name(), "BOND");
    assert_eq!(fe.cn_cursor_cell(), FrontEnd::CN_KEY_CONFIRM);
    assert!(act(&mut fe, Action::Confirm), "the ✓ accepts [M 0x213a98:45-65]");
    // The completion routes to the wheel 0x4000001d [M DAT_002e1ad8 = 0x4000001d] once the
    // card flow's tick has written the file (the window's redraw pumps the flow).
    for _ in 0..10 {
        fe.tick();
        if fe.card_message().is_none() && fe.card.ops_done > 0 {
            break;
        }
    }
    assert_eq!(
        fe.active_profile.as_deref(),
        Some("BOND"),
        "the working codename 0x2e1ae8 holds the new name [M 0x213a98:54]"
    );
    let path = nf_game::profile::Profile::path_in(&dir, "BOND");
    assert!(path.exists(), "the codename is saved to the card [M LS_Save(0x2e1ae8)]");
    assert_eq!(fe.page, Page::CnMenu, "the flow lands on the wheel 0x4000001d [M :58]");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The card the boot seeded is offered on the next boot's Select Codename, and the row loads
/// (the restart half of the pin — through the same session handover the window does when it
/// rebuilds a front end).
#[test]
fn the_saved_codename_is_offered_on_the_next_boot() {
    let dir = temp_dir("nextboot");
    let mut fe = FrontEnd::new();
    nf_view::menu::ProfileSession {
        dir: Some(dir.clone()),
        name: None,
        profile: None,
    }
    .apply_to(&mut fe);
    walk_to_name_entry(&mut fe);
    assert!(act(&mut fe, Action::Confirm), "the ✓ creates BOND");
    for _ in 0..10 {
        fe.tick();
        if fe.card_message().is_none() && fe.card.ops_done > 0 {
            break;
        }
    }
    assert!(nf_game::profile::Profile::path_in(&dir, "BOND").exists());

    // The app restart: the session the window stashed at the manager drop rides back in.
    let session = nf_view::menu::ProfileSession::from_fe(&fe);
    drop(fe);
    let mut fe2 = FrontEnd::new();
    session.apply_to(&mut fe2);
    assert!(act(&mut fe2, Action::Confirm), "the title's Enter");
    assert!(act(&mut fe2, Action::Confirm), "Main row 0 → the codename page");
    assert_eq!(
        fe2.rows(),
        3,
        "the two defaults + the saved codename [M 0x1f7b78's row order]"
    );
    assert!(act(&mut fe2, Action::Down), "row 2, the saved codename");
    assert!(act(&mut fe2, Action::Confirm), "the row loads [M C_SBNFCN 0x20b390 mode 0]");
    assert_eq!(fe2.page, Page::Difficulty, "the load continues to the difficulty page");
    assert_eq!(fe2.active_profile.as_deref(), Some("BOND"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The typed letters reach the grid through the window's key mapping: the user types their
/// own name (the proposal "JAMES 0" first press clears it [M del_when_keypressed 0x213a98:
/// 18-27 — the clear falls through to the append; only the space CELL is swallowed]), the
/// 8-char cap refuses the ninth [M :75-77], and the ✓ saves the normalized name.
#[test]
fn typed_letters_reach_the_grid_and_the_name_saves() {
    let dir = temp_dir("typed");
    let mut fe = FrontEnd::new();
    nf_view::menu::ProfileSession { dir: Some(dir.clone()), name: None, profile: None }
        .apply_to(&mut fe);
    walk_to_name_entry(&mut fe);
    // The window's typing branch: the letter/digit keys map to grid cells [M 0x213a98:79-85]
    // and go through the same cn_press; Backspace is the ← cell 1000 [M :39-42].
    let type_key = |fe: &mut FrontEnd, k: winit::keyboard::KeyCode| -> bool {
        match k {
            winit::keyboard::KeyCode::Backspace => fe.cn_press(FrontEnd::CN_KEY_DELETE),
            _ => menu_cell_press(fe, k),
        }
    };
    for k in [KeyB, KeyO, KeyN, KeyD] {
        assert!(type_key(&mut fe, k), "{k:?} types");
    }
    // The first press cleared the proposal, so typing B-O-N-D over "BOND" lands on "BOND".
    assert_eq!(fe.cn_typed_name(), "BOND");
    // A real second name: the user erases back to empty (4 ← presses), then types JAMES 0.
    for _ in 0..4 {
        assert!(type_key(&mut fe, KeyCode::Backspace), "the ← cell erases");
    }
    assert_eq!(fe.cn_typed_name(), "", "the buffer emptied [M the erase to strlen 0]");
    for k in [KeyJ, KeyA, KeyM, KeyE, KeyCode::KeyS, Digit0] {
        assert!(type_key(&mut fe, k), "{k:?} types");
    }
    assert_eq!(fe.cn_typed_name(), "JAMES0");
    // The 8-char cap [M :75-77]: the append is refused once strlen > 7 — chars 7 and 8 fit,
    // the ninth is refused.
    assert!(type_key(&mut fe, KeyX), "the seventh character fits");
    assert!(type_key(&mut fe, KeyA), "the eighth character fits");
    assert_eq!(fe.cn_typed_name(), "JAMES0XA");
    assert!(!type_key(&mut fe, KeyD), "the ninth character is refused");
    assert_eq!(fe.cn_typed_name(), "JAMES0XA");
    // The ✓ saves the uppercased name [M Menu_IsValidCodename 0x1f9930 strupr] — the full
    // 8-character buffer (the cap test's last two characters stand in the name).
    assert!(act(&mut fe, Action::Confirm), "the ✓ accepts");
    for _ in 0..10 {
        fe.tick();
        if fe.card_message().is_none() && fe.card.ops_done > 0 { break; }
    }
    assert_eq!(fe.active_profile.as_deref(), Some("JAMES0XA"));
    assert!(nf_game::profile::Profile::path_in(&dir, "JAMES0XA").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Finishing a mission under the codename saves progress: the session the window stashes at
/// the manager drop rides the results handover's fresh front end, the sim's tally merges
/// into the profile [M the game writes its MSSN from the live state the results read], and
/// the save prompt's Yes writes the file (the overwrite question first — the codename
/// exists [M ASK_OVERWRITE 0x10ee38]).
#[test]
fn finishing_a_mission_under_the_codename_saves_progress() {
    let dir = temp_dir("progress");
    let mut fe = FrontEnd::new();
    nf_view::menu::ProfileSession { dir: Some(dir.clone()), name: None, profile: None }
        .apply_to(&mut fe);
    walk_to_name_entry(&mut fe);
    assert!(act(&mut fe, Action::Confirm), "the ✓ creates BOND");
    for _ in 0..10 {
        fe.tick();
        if fe.card_message().is_none() && fe.card.ops_done > 0 { break; }
    }
    assert!(nf_game::profile::Profile::path_in(&dir, "BOND").exists());
    // The mission load drops the manager: the window stashes the session (the App's
    // stash_profile_session) — the session the results handover inherits.
    let session = nf_view::menu::ProfileSession::from_fe(&fe);
    assert_eq!(session.name.as_deref(), Some("BOND"), "the stash holds the codename");

    // The results handover: a fresh manager the way demo_results builds it (bare), the
    // session applied, then the results enter + the tally merge (the fix's two lines).
    drop(fe);
    let mut fe2 = FrontEnd::new();
    session.apply_to(&mut fe2);
    fe2.mission_ended(Outcome::Complete, Stats { shots_fired: 30, shots_hit: 20, accuracy_pct: 66, time_s: 100 });
    fe2.requested_level = None; // the headless leg skips the outro wrap
    // G-048(3): the results page opens at the load tail's MenuManager_Create, not at the end
    // [M 0x182c60:72-75 + 0x1d6360:160-163].
    assert!(fe2.reopen_queued_page(), "the queued results page lands");
    assert_eq!(fe2.page, Page::Results);
    let mut progress = nf_game::progress::Progress::default();
    progress.stats[0].dispatched = 2;
    progress.stats[0].shots_fired = 10;
    progress.stats[0].shots_hit_enemy = 6;
    assert!(
        fe2.results_enter(&mut progress, 0x0700_0005, 6, 2, None),
        "the fields built [M 0x208e78:77]"
    );
    fe2.sync_profile_progress(&progress);
    // The save prompt through the real confirms [M 0x208e78:24-36 + ASK_OVERWRITE]:
    assert!(act(&mut fe2, Action::Confirm), "the prompt opens");
    assert!(fe2.results_box, "the save prompt stands [M the box 0xb]");
    assert!(act(&mut fe2, Action::Confirm), "the Yes answer");
    assert_eq!(fe2.card_message(), Some((4, 0x27d)), "ASK_OVERWRITE — the codename exists");
    assert!(fe2.card_answer(BTN_YES), "Yes continues the flow");
    fe2.tick();
    assert_eq!(fe2.card_message(), None, "the write completed");
    // The file on the card carries the award: row 1's completion sets rows 1+2 on the boot
    // 0x3 [M Menu_SetLevelBonus 0x1ffa10:62-66's award; save_progress_roundtrip's anchors].
    let saved = nf_game::profile::Profile::load_file(&nf_game::profile::Profile::path_in(&dir, "BOND"))
        .expect("the saved profile decodes");
    assert_eq!(saved.progress.nightfire_status(), 0b111, "the award persisted to the card");
    let _ = std::fs::remove_dir_all(&dir);
}
