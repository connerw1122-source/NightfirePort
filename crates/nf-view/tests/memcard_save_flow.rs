//! memcard-save-flow headless tests: the card-flow machine on `FrontEnd` — the triggers write
//! exactly once with the right contents, a write failure surfaces the game's error page, and
//! the message/label/button tables pin the game's numbers. docs/systems/memcard-save-flow.md.
//!
//! Evidence: P_NFRESULTS_Handler 0x208e78 (the results save prompt + the re-ask), C_SBCNOPTIONS
//! 0x213528 (the wheel's save row), C_KEYBOARD 0x213a98 / P_CNNAME 0x213790 (the create save),
//! C_SBCNSELECT 0x212f58 (the delete), Menu_UpdateMessageBox 0x1f7de8 (case 1 = LS_Save under
//! the name buffer 0x2e1ae8), MemCard_SAVE_DATA 0x10fa08 + ASK_OVERWRITE 0x10ee38 (every save
//! to an existing codename asks), SAVEERROR 0x10f2d8 / CHECK_FREE_SPACE 0x10ef00 / LOADERROR
//! 0x10f138 / DELETE_ERROR 0x10f068 (the error dialogs), PS2_DoCardFlow 0x217bd8 (the message
//! → label → buttons table), HandlePS2CardState 0x110b98 (the per-frame pump).

use nf_view::menu::{Action, CardMsg, FrontEnd, Outcome, Page, Stats};

/// A scratch card dir per test.
fn card_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nf-card-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Creates a codename through the real flow (✓ on the keyboard page + the flow's tick). The
/// typed buffer is cleared first — cn_open seeds the proposal [M C_KEYBOARD 0x213a98's buffer
/// at 0x34e028; the helper mirrors codename_entry's type_name].
fn create_via_flow(fe: &mut FrontEnd, name: &str) {
    fe.cn_open(0x4000_0025);
    fe.cn_typed.clear();
    fe.cn_clear_on_key = false;
    for c in name.chars() {
        fe.cn_type_char(c);
    }
    assert!(fe.cn_press(FrontEnd::CN_KEY_CONFIRM), "the accept arms the save");
    fe.tick();
}

/// Reads USATxt.dat out of FILES.BIN [M Txt_LoadLanguage 0x1caad0 loads FNames[SelLang]].
fn iso_lang(iso_path: &str) -> Option<nf_formats::hud::Lang> {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat"))?;
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::hud::Lang::parse(&bytes).ok()
}

/// The message table is the game's numbers: `DAT_002462d4` → the Txt label → the button mask
/// [M PS2_DoCardFlow 0x217bd8:76-152 + the setting MemCard_* functions].
#[test]
fn the_message_tables_match_the_game() {
    let table = [
        (CardMsg::CardFull, 3u32, 0x26au32, 0x1u32),            // CHECK_FREE_SPACE 0x10ef00
        (CardMsg::AskOverwrite, 4, 0x27d, 0x6),                 // ASK_OVERWRITE 0x10ee38
        (CardMsg::SaveFailed, 5, 0x26d, 0xc),                   // SAVEERROR 0x10f2d8
        (CardMsg::LoadFailed, 7, 0x273, 0xc),                   // LOADERROR 0x10f138
        (CardMsg::Saving, 9, 0x26c, 0),                         // SAVE_DATA's write phases
        (CardMsg::Deleting, 0xc, 0x3e6, 0),                     // DELETE_DATA 0x110320
        (CardMsg::AskDelete, 0xd, 0x3e5, 0x24),                 // ASK_DELETE 0x10ec98
        (CardMsg::DeleteFailed, 0xe, 0x3e7, 0xc),               // DELETE_ERROR 0x10f068
    ];
    for (msg, id, label, buttons) in table {
        assert_eq!(msg.id(), id, "{msg:?}'s flow-state value [M 0x217bd8]");
        assert_eq!(msg.label(), label, "{msg:?}'s Txt label [M 0x217bd8]");
        assert_eq!(msg.buttons(), buttons, "{msg:?}'s PS2CardFlow buttons [M the setter]");
        assert_eq!(msg.buttons(), msg.buttons(), "sister: the mask is stable");
    }
    // The button bits and their answer values are the same numbers [M 0x217bd8:178-207 binds
    // Txt 0x3ce OK / 0x3cf Yes / 0x3cd Cancel / 0x3d0 Retry / 0x3d1 Format / 0x3d2 Delete;
    // the MemCard_* states test DAT_002462da against 1/2/4/8/0x10/0x20].
    assert_eq!(nf_view::menu::BTN_OK, 1);
    assert_eq!(nf_view::menu::BTN_YES, 2);
    assert_eq!(nf_view::menu::BTN_CANCEL, 4);
    assert_eq!(nf_view::menu::BTN_RETRY, 8);
    assert_eq!(nf_view::menu::BTN_FORMAT, 0x10);
    assert_eq!(nf_view::menu::BTN_DELETE, 0x20);
}

/// The card messages' strings resolve from the disc's USATxt.dat and are the game's texts
/// [D USATxt.dat in FILES.BIN; NF_ISO-gated — the numbers-only sisters above run without it].
#[test]
fn the_card_message_strings_resolve_from_the_disc() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let Some(lang) = iso_lang(&iso) else { panic!("USATxt.dat must parse — the dialogs are drawn from it") };
    let wants = [
        (0x26au32, "Insufficient space on memory card"),
        (0x26c, "Saving data. Do not remove memory card"),
        (0x26d, "Save failed! Check memory card"),
        (0x273, "Load failed! Check memory card"),
        (0x27d, "This codename already exists. Would you like to overwrite it?"),
        (0x3e5, "Are you sure you want to delete this codename?"),
        (0x3e6, "Deleting data. Do not remove memory card"),
        (0x3e7, "Delete failed! Check memory card"),
        // The prompt the results page opens [M P_NFRESULTS 0x208e78:27] and the dialog
        // buttons [M 0x217bd8:178-207].
        (0x100_0206, "Do you want to save your progress?"),
        (0x3cd, "Cancel"),
        (0x3ce, "OK"),
        (0x3cf, "Yes"),
        (0x3d0, "Retry"),
        (0x3d2, "Delete"),
    ];
    for (id, prefix) in wants {
        let s = lang.bind(id).unwrap_or_default();
        assert!(s.starts_with(prefix), "Txt {id:#x} = {s:?} must start with {prefix:?}");
    }
    // The honest no-PC-equivalent set still exists on the disc (the port never shows them —
    // doc §4); pinning them here proves they were READ, not assumed.
    for id in [0x26bu32, 0x267, 0x268, 0x269, 0x27a, 0x27c] {
        let s = lang.bind(id).unwrap_or_default();
        assert!(s.contains("memory card"), "Txt {id:#x} is a card-hardware message");
    }
}

/// The results commit's save: Confirm opens the prompt, Yes runs the flow (the overwrite
/// question for an existing codename), the write lands once with the synced contents, and the
/// page routes to the mission list [M P_NFRESULTS 0x208e78:24-52 + Menu_UpdateMessageBox
/// 0x1f7de8 case 1; the routing 0x4000001c].
#[test]
fn the_results_commit_writes_once_with_the_synced_contents() {
    let dir = card_dir("results");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    create_via_flow(&mut fe, "BONDSAVE");
    assert_eq!(fe.card.ops_done, 1, "sister: the create wrote once");

    // A completed mission with a live progress layer — the game's results flow rides
    // PlrStat_GetScore's commit (the same layer the profile's MSSN/BNUS blocks encode). The
    // mission tally is non-zero so the commit improves (DoneBetter [M 0x1984b0:301-308]).
    let mut progress = nf_game::progress::Progress::default();
    progress.stats[0].dispatched = 2;
    progress.stats[0].shots_fired = 10;
    progress.stats[0].shots_hit_enemy = 6;
    let map = 0x0700_0005;
    fe.mission_ended(Outcome::Complete, Stats { shots_fired: 30, shots_hit: 20, accuracy_pct: 66, time_s: 100 });
    fe.requested_level = None; // the headless leg skips the outro wrap
    fe.sync_profile_progress(&progress);
    assert!(fe.results_enter(&mut progress, map, 6, 2, None), "the fields build [M 0x208e78:77]");
    assert!(fe.results_improved, "the commit improved (first completion) [M PlrStats_DoneBetter]");
    assert!(!fe.results_box, "the prompt opens on the Confirm, not the enter [M 0x208e78:24-32]");
    // Confirm #1 opens the "Do you want to save your progress?" box.
    assert!(fe.action(Action::Confirm));
    assert!(fe.results_box, "the prompt stands [M Menu_CreateOptionBox(0x1000206, 0xb)]");
    assert!(fe.card_message().is_none(), "the card flow is not running yet");
    // Confirm #2 answers Yes: the file exists → the overwrite question [M 0x10fa08 sub-7].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.card_message(), Some((4, 0x27d)), "ASK_OVERWRITE [M 0x10ee38]");
    assert_eq!(fe.card.buttons, 0x6, "Yes+Cancel [M PS2CardFlow]");
    // Yes → "Saving data…" → the tick writes → the page routes to the mission list [M the
    // flow's terminal branch sends the page's 0x4000001c routing].
    assert!(fe.card_answer(nf_view::menu::BTN_YES), "Yes continues [M 0x10ee38's == 2 arm]");
    assert_eq!(fe.card_message(), Some((9, 0x26c)), "the saving message [M 0x10fa08 sub-8]");
    fe.tick();
    assert_eq!(fe.card.ops_done, 2, "the trigger wrote once more");
    assert_eq!(fe.card_message(), None, "the box closed on completion [M 0x217bd8:62-71]");
    assert_eq!(fe.page, Page::MissionSelect, "the results route on [M 0x208e78:203-211]");
    // The written file carries the profile's progress + the unlock mirror [M save_profile's
    // nightfire-status write; the game's own chain is the same six blocks + CRC].
    let path = nf_game::profile::Profile::path_in(&dir, "BONDSAVE");
    let back = nf_game::profile::Profile::load_file(&path).unwrap();
    assert_eq!(back.name, "BONDSAVE");
    assert_eq!(back.progress.nightfire_status() & (1 << 1), 1 << 1, "completing row 0 unlocked row 1");
    assert_eq!(fe.card.ops_done, 2, "sister: nothing else wrote");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The results prompt's NO never writes and routes straight on [M 0x208e78:186-189 — only the
/// completion flag stands]; a second completion with no improvement never prompts [M :24-32's
/// DoneBetter gate].
#[test]
fn the_results_no_answer_never_writes() {
    let dir = card_dir("nope");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    create_via_flow(&mut fe, "NOSAVE");
    let mut progress = nf_game::progress::Progress::default();
    progress.stats[0].dispatched = 2;
    fe.mission_ended(Outcome::Complete, Stats::default());
    fe.sync_profile_progress(&progress);
    fe.requested_level = None;
    assert!(fe.results_enter(&mut progress, 0x0700_0005, 6, 2, None));
    assert!(fe.action(Action::Confirm));
    assert!(fe.results_box, "the prompt stands");
    assert!(fe.action(Action::Back), "Back is the box's NO [M the msg-0x50 arm]");
    assert_eq!(fe.card.ops_done, 1, "only the create wrote");
    assert_eq!(fe.page, Page::MissionSelect, "the routing ran without a save [M :203-211]");
    assert!(fe.results_save_answer_no, "the No answer recorded [M the re-ask gate :196]");

    // Replay the same mission scoring LESS: no prompt at all — DoneBetter needs the total to
    // reach the best, and ties count as better [M 0x1984b0:307 `<=`].
    progress.stats[0].dispatched = 1;
    progress.stats[0].shots_fired = 4;
    progress.stats[0].shots_hit_enemy = 2;
    fe.mission_ended(Outcome::Complete, Stats::default());
    fe.sync_profile_progress(&progress);
    fe.requested_level = None;
    assert!(fe.results_enter(&mut progress, 0x0700_0005, 6, 2, None));
    assert!(!fe.results_improved, "no improvement [M DoneBetter]");
    assert!(fe.action(Action::Confirm));
    assert!(!fe.results_box, "no prompt on a non-improving commit [M :24-32]");
    assert_eq!(fe.page, Page::MissionSelect, "the Confirm routed straight on");
    assert_eq!(fe.card.ops_done, 1, "sister: still only the create wrote");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A write failure surfaces the game's error page: "Save failed!" (msg 5, Txt 0x26d) with
/// Retry+Cancel [M SAVEERROR 0x10f2d8]; Retry re-runs, Cancel ends the flow and the results
/// page re-opens the prompt (the answer wasn't No) [M 0x208e78:196-201]. The card points at a
/// FILE here, so every write fails with a plain IO error on any account.
#[test]
fn a_write_failure_surfaces_the_save_failed_page() {
    let dir = card_dir("fail");
    let card_file = dir.join("cardfile"); // a FILE where the profiles dir would be
    std::fs::write(&card_file, b"not a directory").unwrap();
    let mut fe = FrontEnd::new();
    fe.profile_dir = Some(card_file.clone());
    fe.profiles = vec!["STRUT".into()]; // the enumeration is a read; the failure test arms it by hand
    fe.active_profile = Some("STRUT".into());
    let mut progress = nf_game::progress::Progress::default();
    progress.stats[0].dispatched = 2;
    progress.stats[0].shots_fired = 10;
    progress.stats[0].shots_hit_enemy = 6;
    fe.profile = Some(nf_game::profile::Profile::default_codename("STRUT"));
    fe.mission_ended(Outcome::Complete, Stats::default());
    fe.sync_profile_progress(&progress);
    fe.requested_level = None;
    assert!(fe.results_enter(&mut progress, 0x0700_0005, 6, 2, None));
    assert!(fe.action(Action::Confirm), "the prompt opens");
    assert!(fe.action(Action::Confirm), "Yes requests the save");
    assert_eq!(fe.card_message(), Some((9, 0x26c)), "the run starts under the saving message");
    fe.tick();
    assert_eq!(fe.card_message(), Some((5, 0x26d)), "the write failed → SAVEERROR [M 0x10f2d8]");
    assert_eq!(fe.card.buttons, 0xc, "Retry+Cancel [M PS2CardFlow]");
    assert!(fe.card.awaiting, "the dialog waits [M DAT_002462d9]");
    assert!(fe.card.failed, "MENU_LS+0x3b [M 0x217bd8:45]");
    assert_eq!(fe.card.ops_done, 0, "sister: nothing was written");
    // Retry re-runs the op and fails again [M 0x10f2d8's == 8 chain].
    assert!(fe.card_answer(nf_view::menu::BTN_RETRY));
    assert_eq!(fe.card_message(), Some((9, 0x26c)));
    fe.tick();
    assert_eq!(fe.card_message(), Some((5, 0x26d)), "the retry fails the same way");
    assert_eq!(fe.card.ops_done, 0, "still nothing written");
    // Cancel ends the flow; the results page re-asks [M the PSIOP abort → the re-ask arm].
    assert!(fe.card_answer(nf_view::menu::BTN_CANCEL));
    assert!(fe.card_message().is_none(), "the dialog closed");
    assert!(fe.results_box, "the prompt re-opens (the answer wasn't No) [M 0x208e78:196-201]");
    assert_eq!(fe.page, Page::Results, "the page waits for the new answer");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The enumeration never writes: the boot/list enums map to `reload_profiles` on the port
/// [M P_PARISENUM 0x20a648 mode 4 / P_NFSELECT 0x20aec0 mode 2 — Menu_EnumSaves, no LS_Save],
/// and the results save never fires for a template codename in play [M 0x208e78:33-36].
#[test]
fn the_enumeration_and_the_defaults_never_write() {
    let dir = card_dir("enum");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    create_via_flow(&mut fe, "ENUMED");
    let ops = fe.card.ops_done;
    fe.reload_profiles();
    fe.set_profile_dir(dir.clone());
    assert_eq!(fe.card.ops_done, ops, "the enums are reads [M Menu_EnumSaves 0x1f7b78]");
    // The Default template in play: the results Confirm routes without a prompt [M :33-36].
    fe.active_profile = None;
    let mut progress = nf_game::progress::Progress::default();
    fe.mission_ended(Outcome::Complete, Stats::default());
    fe.requested_level = None;
    assert!(fe.results_enter(&mut progress, 0x0700_0005, 6, 2, None));
    assert!(fe.action(Action::Confirm));
    assert!(!fe.results_box, "no prompt without a saved codename [M DAT_002e1ae0 > 1]");
    assert_eq!(fe.page, Page::MissionSelect);
    assert_eq!(fe.card.ops_done, ops, "sister: the Default path never writes");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A corrupt profile file surfaces "Load failed!" (msg 7, Txt 0x273, Retry+Cancel) and Retry
/// re-runs the load [M LOADERROR 0x10f138; LOAD_DATA's CRC failure pushes 0xb at 0x110870:76].
#[test]
fn a_corrupt_load_surfaces_the_load_failed_page() {
    let dir = card_dir("corrupt");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    create_via_flow(&mut fe, "GUMMED");
    let path = nf_game::profile::Profile::path_in(&dir, "GUMMED");
    std::fs::write(&path, b"not a nightfire chain").unwrap();
    fe.reload_profiles();
    // The codename page's Confirm on the saved row = the load [M C_SBNFCN 0x20b390's mode 0].
    fe.page = Page::Codename;
    fe.cursor = 2;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.card_message(), Some((7, 0x273)), "LOADERROR [M 0x10f138]");
    assert_eq!(fe.card.buttons, 0xc, "Retry+Cancel");
    assert!(!matches!(fe.page, Page::Difficulty), "the flow blocked the page change");
    // Fix the file behind the dialog, Retry → the load succeeds → the difficulty page.
    let good = nf_game::profile::Profile::default_codename("GUMMED");
    std::fs::write(&path, good.encode()).unwrap();
    assert!(fe.card_answer(nf_view::menu::BTN_RETRY), "Retry re-runs the load [M 0x10f138's == 8]");
    assert_eq!(fe.card_message(), None, "the dialog closed on success");
    assert_eq!(fe.page, Page::Difficulty, "the load's continuation ran [M load_codename]");
    let _ = std::fs::remove_dir_all(&dir);
}
