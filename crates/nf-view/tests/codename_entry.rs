//! codename-entry headless tests: the name-entry page's grid model, the validator wiring, and the
//! create / delete / save flows through the port's memory card (numbers/ids only).
//!
//! Evidence: P_CNNAME_Handler 0x213790 (the 0x4c init, the 0x50 tick's box 4 save), C_KEYBOARD_Handler
//! 0x213a98 (the cell ids 1000/1001/1002, the 8-char clamp, del_when_keypressed, the accept), 
//! Menu_IsValidCodename 0x1f9930 (trim + strupr, 0x3bc empty, 0x2c0 reserved-substring against the
//! def_codename labels 0x29b/0x211 at 0x2e1b18/0x2e1b50 [L ee_snapshot]), Menu_UpdateMessageBox 0x1f7de8
//! (box modes: 0 LS_Load, 1 LS_Save, 3 psiDeleteData), C_SBCNSELECT 0x212f58 (the ~X delete, the list
//! rows), C_SBCNOPTIONS 0x213528 (the wheel's last row = Save Codename).

use nf_view::menu::{Action, FrontEnd, Page};

/// A scratch card dir per test.
fn card_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nf-cn-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Types a name through the direct-keyboard path after the game's first-press clear
/// [M C_KEYBOARD 0x213a98:18-27 clears on the first press; the port's cn_type_char is the append].
fn type_name(fe: &mut FrontEnd, s: &str) {
    fe.cn_typed.clear();
    fe.cn_clear_on_key = false;
    for ch in s.chars() {
        fe.cn_type_char(ch);
    }
}

/// The game's proposals [M P_CNNAME 0x213790:36/43-44 — DAT_0030d2d8 empty for the cheat entry,
/// DAT_0030d530 "BOND" [L ee_snapshot] for a profile name].
#[test]
fn open_seeds_the_game_s_proposals() {
    let mut fe = FrontEnd::new();
    fe.cn_open(0x4000_0025);
    assert_eq!(fe.page, Page::CodenameEntry);
    assert_eq!(fe.page.id(), 0x4000_0020, "the page id is the game's own [M 0x2029e0:372]");
    assert_eq!(fe.cn_typed_name(), "BOND", "the non-cheat proposal [M 0x213790:43-44]");
    assert_eq!(fe.cn_cursor_cell(), 1002, "the ✓ holds the focus on open [M 0x213790:47]");
    fe.cn_open(0x4000_001d);
    // Sister: the cheat entry's proposal is the EMPTY default — the two paths must differ.
    assert_ne!(fe.cn_typed_name(), "BOND");
    assert_eq!(fe.cn_typed_name(), "", "the cheat proposal [M 0x213790:36]");
}

/// The first press after open clears the proposal before typing [M del_when_keypressed
/// 0x213a98:18-27]; the SPACE press is swallowed entirely [M :21-25].
#[test]
fn first_press_clears_and_first_space_is_swallowed() {
    let mut fe = FrontEnd::new();
    fe.cn_open(0x4000_0025);
    assert!(fe.cn_press(0x41), "the letter cell types");
    assert_ne!(fe.cn_typed_name(), "BOND", "sister: the proposal must be gone");
    assert_eq!(fe.cn_typed_name(), "A", "typed on the cleared buffer [M 0x213a98:84-85]");
    fe.cn_open(0x4000_0025);
    assert!(fe.cn_press(FrontEnd::CN_KEY_SPACE));
    assert_eq!(fe.cn_typed_name(), "", "sister: the swallowed space typed nothing");
    assert!(fe.cn_press(FrontEnd::CN_KEY_SPACE));
    assert_eq!(fe.cn_typed_name(), " ", "the second space types [M DAT_0030d380 = 0x20]");
}

/// The 8-character clamp [M 0x213a98:75-77] and the ← key [M id 1000 = :39-42].
#[test]
fn max_eight_chars_and_the_delete_key() {
    let mut fe = FrontEnd::new();
    fe.cn_open(0x4000_0025);
    fe.cn_clear_on_key = false;
    fe.cn_typed.clear();
    for ch in "ABCDEFGH".chars() {
        assert!(fe.cn_type_char(ch));
    }
    assert_eq!(fe.cn_typed.chars().count(), FrontEnd::CN_MAX_CHARS);
    assert!(!fe.cn_type_char('I'), "sister: the ninth character is refused [M :75-77]");
    assert!(fe.cn_press(FrontEnd::CN_KEY_DELETE));
    assert_eq!(fe.cn_typed.chars().count(), 7, "← removes the last character [M :39-42]");
}

/// The grid carries the game's cell ids: digits 0x30-0x39, letters 0x41-0x5a, the four glyph
/// cells 100-103 [M P_CNNAME 0x213790:50-57] and the three special keys 1000-1002 [M 0x213a98].
#[test]
fn grid_shape_matches_the_game_s_cell_ids() {
    let flat: Vec<u32> = FrontEnd::CN_GRID.iter().flat_map(|r| r.iter().copied()).collect();
    assert_eq!(flat.len(), 10 + 26 + 4 + 3);
    assert_eq!(&flat[..10], &[0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39]);
    assert_eq!(&flat[10..36], &(0x41u32..=0x5a).collect::<Vec<_>>());
    assert_eq!(&flat[36..40], &[100, 101, 102, 103], "the glyph cells [M 0x213790:50-57]");
    // Sister: the three special keys must be present and ordered ← → ✓ at the tail.
    assert_eq!(&flat[40..], &[1000, 1001, 1002]);
}

/// The glyph cells type the game's four font-slot bytes [M P_CNNAME 0x213790:50-57 writes
/// 0xc4/0xc2/0xc6/0xc0 to 0x30d8d8/da/dc/de; C_KEYBOARD appends the cell text's first byte].
#[test]
fn the_glyph_cells_type_their_font_bytes() {
    let mut fe = FrontEnd::new();
    fe.cn_open(0x4000_0025);
    fe.cn_clear_on_key = false;
    fe.cn_typed.clear();
    for cell in [100u32, 101, 102, 103] {
        assert!(fe.cn_press(cell), "cell {cell} types");
    }
    assert_eq!(fe.cn_typed, "\u{c4}\u{c2}\u{c6}\u{c0}");
    for cell in [0x41u32, 0x42, 0x43, 0x44] {
        assert!(fe.cn_press(cell), "the letters fill to 8");
    }
    // Sister: the ninth character is refused — the 8-char clamp binds here too [M :75-77].
    assert!(!fe.cn_press(0x45));
}

/// The validator's empty refusal binds label 0x3bc onto box 6 [M C_KEYBOARD 0x213a98:47-51 ←
/// Menu_IsValidCodename 0x1f9930:81] and creates nothing.
#[test]
fn empty_name_refused_with_box_6_label_3bc() {
    let dir = card_dir("empty");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.cn_open(0x4000_0025);
    fe.cn_typed.clear(); // an empty buffer — the validator's first refusal [M 0x1f9930:47-51]
    assert!(!fe.cn_press(FrontEnd::CN_KEY_CONFIRM), "the refused name is not accepted");
    assert_eq!(fe.cn_box, Some((6, 0x3bc)), "the empty label on the validator's box 6");
    assert!(fe.profiles.is_empty(), "sister: nothing was created");
    assert!(fe.cn_clear_on_key, "the next press clears again [M 0x213a98:50]");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The validator's reserved-substring refusal binds label 0x2c0 [M Menu_IsValidCodename
/// 0x1f9930:63-76 — strstr against the def_codename labels, uppercased both sides].
#[test]
fn reserved_name_refused_with_box_6_label_2c0() {
    let dir = card_dir("reserved");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.cn_reserved = vec!["Default".into(), "New Codename".into()];
    fe.cn_open(0x4000_0025);
    type_name(&mut fe, "ADEFAULT");
    assert!(!fe.cn_press(FrontEnd::CN_KEY_CONFIRM), "the reserved name is not accepted");
    assert_eq!(fe.cn_box, Some((6, 0x2c0)), "the reserved label [M 0x2c0]");
    assert!(fe.profiles.is_empty(), "sister: the reserved name created nothing");
    // A name without the reserved word passes the same validator.
    type_name(&mut fe, "MYGUARD");
    assert!(fe.cn_press(FrontEnd::CN_KEY_CONFIRM));
    assert_eq!(fe.cn_box, None, "the accepted name opens no error box");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The accept: the normalized name creates the profile through the save code, and the flow
/// routes to the wheel 0x4000001d [M C_KEYBOARD 0x213a98:53-65; strupr write-back 0x1f9930:77].
#[test]
fn valid_name_creates_saves_and_routes_to_the_wheel() {
    let dir = card_dir("create");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.cn_open(0x4000_0025);
    let path = nf_game::profile::Profile::path_in(&dir, "TESTING");
    assert!(!path.exists(), "sister: the file must not exist before the accept");
    type_name(&mut fe, "testing"); // lower-case on purpose — the game uppercases [M 0x1f9930:62]
    assert!(fe.cn_press(FrontEnd::CN_KEY_CONFIRM));
    assert!(path.exists(), "the chain is on the card [M LS_Save 0x1f7de8:21-29]");
    assert_eq!(fe.cn_typed_name(), "TESTING", "the buffer holds the normalized name [M 0x1f9930:77]");
    assert!(fe.profiles.iter().any(|p| p == "TESTING"), "the list re-enumerates");
    assert_eq!(fe.active_profile.as_deref(), Some("TESTING"));
    assert_eq!(fe.cn_exit, Some(0x4000_001d), "DAT_002e1ad8's continuation [M 0x213a98:58]");
    assert_eq!(fe.page, Page::CnMenu);
    // The stored name round-trips through the game's own encode/decode.
    let back = nf_game::profile::Profile::load_file(&path).unwrap();
    assert_eq!(back.name, "TESTING");
    let _ = std::fs::remove_dir_all(&dir);
}

/// An existing codename opens the overwrite box (box 4) and refuses [M P_CNNAME 0x50:62-76 —
/// the port keeps create_codename's MemCard_ASK_OVERWRITE refusal].
#[test]
fn an_existing_codename_asks_the_overwrite_box() {
    let dir = card_dir("exists");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.cn_open(0x4000_0025);
    type_name(&mut fe, "TWICE");
    assert!(fe.cn_press(FrontEnd::CN_KEY_CONFIRM));
    assert!(!fe.cn_press(FrontEnd::CN_KEY_CONFIRM), "the second accept is refused — the file exists");
    assert_eq!(fe.cn_box, Some((4, 0)), "the overwrite question [M P_CNNAME 0x50:62-76]");
    fe.cn_dismiss_box();
    assert_eq!(fe.cn_box, None, "sister: dismissing clears the box");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The Edit list's ~X delete: request → YES removes the file, Back keeps it [M C_SBCNSELECT
/// 0x212f58 case 0x5e → box mode 3 psiDeleteData 0x1f7de8:38-57].
#[test]
fn the_edit_list_delete_removes_the_file() {
    let dir = card_dir("delete");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.cn_open(0x4000_0025);
    type_name(&mut fe, "DROPPER");
    assert!(fe.cn_press(FrontEnd::CN_KEY_CONFIRM)); // created + active
    fe.page = Page::CnSelect;
    fe.reload_profiles();
    fe.cursor = 2; // the first saved row [M Menu_EnumSaves 0x1f7b78: the card's rows follow the defaults]
    let path = nf_game::profile::Profile::path_in(&dir, "DROPPER");
    assert!(path.exists(), "sister: the file must exist before the delete");
    assert!(fe.action(Action::Side), "the ~X arms the confirm");
    assert_eq!(fe.cn_delete_row, Some(2));
    assert!(fe.action(Action::Back), "NO keeps the file [M the card-flow cancel]");
    assert!(path.exists());
    assert!(fe.action(Action::Side));
    assert!(fe.action(Action::Confirm), "YES deletes [M psiDeleteData 0x1f7de8:38-57]");
    assert!(!path.exists());
    assert!(fe.profiles.is_empty(), "the list re-enumerates [M P_CNSELECT 0x6e]");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The wheel's LAST row is Save Codename [M C_SBCNOPTIONS 0x213528 case 6 — a saved codename
/// runs box mode 1 = LS_Save(0x2e1ae8)]: the port writes the loaded profile in place.
#[test]
fn the_wheel_s_last_row_saves_the_codename() {
    let dir = card_dir("save");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.cn_open(0x4000_0025);
    type_name(&mut fe, "SAVEME");
    assert!(fe.cn_press(FrontEnd::CN_KEY_CONFIRM)); // create + land on the wheel, cursor 1
    assert_eq!(fe.page, Page::CnMenu);
    let before = nf_game::profile::Profile::load_file(&nf_game::profile::Profile::path_in(&dir, "SAVEME")).unwrap();
    assert_eq!(before.progress.nightfire_status() & (1 << 5), 0, "sister: the bit must be unset first");
    fe.unlocked[5] = true; // the edit the save must capture
    fe.cursor = 6; // the wheel's last row [L CN_MENU[6] label 0x1f4]
    assert!(fe.action(Action::Confirm), "Save Codename consumes the press");
    assert_eq!(fe.page, Page::CnSelect, "the wheel leaves to the list [M case 6's iris]");
    let after = nf_game::profile::Profile::load_file(&nf_game::profile::Profile::path_in(&dir, "SAVEME")).unwrap();
    assert_ne!(after.progress.nightfire_status() & (1 << 5), 0, "the save wrote the edit");
    // A default codename ("Default" is not a card file) re-maps and leaves without a save [M case 6's
    // slot < 2 arm].
    fe.page = Page::CnMenu; // the wheel again [M case 6 leaves to the list, the wheel reopens]
    fe.cn_name = "Default".into();
    fe.cursor = 6;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::CnSelect);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The cheat entry's ✓ runs the special-codename check and creates nothing [M C_KEYBOARD
/// 0x213a98:67-73 — Menu_SpecialCodenameCheck's miss returns without a save].
#[test]
fn the_cheat_entry_s_confirm_creates_nothing() {
    let dir = card_dir("cheat");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    fe.cn_open(0x4000_001d); // from the wheel — cheat_mode [M P_CNNAME 0x213790:33]
    type_name(&mut fe, "PASSPORT");
    assert!(fe.cn_press(FrontEnd::CN_KEY_CONFIRM));
    assert!(fe.cn_cheat_checked, "the special-codename check ran (not ported — reported)");
    assert_eq!(fe.cn_box, None, "sister: no validator box stands on the cheat path");
    assert!(fe.profiles.is_empty(), "the cheat path never saves a profile");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The grid cursor walks CN_GRID and clamps at the edges (the port's packing [S]).
#[test]
fn the_grid_cursor_walks_and_clamps() {
    let mut fe = FrontEnd::new();
    fe.cn_open(0x4000_0025);
    assert_eq!(fe.cn_cursor_cell(), 1002);
    fe.cn_move(-1, 0);
    assert_eq!(fe.cn_cursor_cell(), 1001, "◀ from ✓ lands on the space key");
    fe.cn_move(0, -1);
    assert_eq!(fe.cn_cursor_cell(), 0x58, "▲ keeps the column: row S-Z's 6th cell is X");
    fe.cn_move(0, 1);
    assert_eq!(fe.cn_cursor_cell(), 1001, "▼ back to the special row's space key");
    fe.cn_move(0, 1); // already the bottom row — clamped
    fe.cn_move(1, 0);
    fe.cn_move(1, 0);
    assert_eq!(fe.cn_cursor_cell(), 1002, "▶▶ lands on ✓; the bottom-right clamp holds");
    fe.cn_move(1, 0); // already the last column — clamped
    fe.cn_move(0, 1); // already the last row — clamped
    assert_eq!(fe.cn_cursor_cell(), 1002, "sister: the clamp kept the ✓ focused");
    // Confirm presses the focused cell — without a card the accept refuses (NoCard) [M the
    // game's save goes through the card flow; the port's create_codename refuses without one].
    assert!(!fe.action(Action::Confirm));
    assert_ne!(fe.cn_typed_name(), "", "sister: the buffer is not empty — the proposal stood");
}

// --- The NF_ISO sister: the name-entry strings come from the language bank ---

/// Reads USATxt.dat out of FILES.BIN [M Txt_LoadLanguage 0x1caad0 — the same walk menus_flow uses].
fn iso_lang(iso_path: &str) -> Option<nf_formats::hud::Lang> {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat"))?;
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::hud::Lang::parse(&bytes).ok()
}

/// The name-entry page's label ids resolve against the disc [M every id below is a handler
/// argument: the titles P_CNNAME 0x213790:37/41, the validator's two refusals 0x1f9930:72/81,
/// the def_codename reserved labels 0x2e1b18/0x2e1b50's first dwords [L ee_snapshot], the wheel's
/// last-row label CN_MENU[6] [L table 0x2dfc28], the modified box P_CNMENU 0x6b, the cheat box
/// C_KEYBOARD 0x213a98:71, the Edit-%s header P_CNMENU 0x213288:32-33].
#[test]
fn the_name_entry_strings_come_from_the_language_bank() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let Some(lang) = iso_lang(&iso) else { panic!("USATxt.dat must parse — the name entry binds it") };
    let b = |id: u32| lang.bind(id).unwrap_or("");
    for id in [0x1f5, 0x29b, 0x211, 0x2c0, 0x3bc, 0x1f4, 0x100_02d3, 0x100_02d4, 0x100_029d, 0x100_01e4, 0x100_02ff] {
        println!("label {id:#x} = {:?}", b(id));
    }
    assert!(!b(0x1f5).is_empty(), "the normal-mode title resolves [M 0x213790:41]");
    assert!(!b(0x100_02d4).is_empty(), "the cheat title resolves [M 0x213790:37]");
    assert!(!b(0x2c0).is_empty() && !b(0x3bc).is_empty(), "the validator's two refusals resolve");
    assert_eq!(b(0x29b), "New Codename", "def_codename[0]'s label [L 0x2e1b18 dword 0]");
    assert_eq!(b(0x211), "Default", "def_codename[1]'s label [L 0x2e1b50 dword 0]");
    assert_ne!(b(0x2c0), b(0x3bc), "sister: the two refusals are different lines");
}
