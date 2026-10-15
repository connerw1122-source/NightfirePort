//! objective-hints — the pause OBJECTIVES tab's ✕ flow, driven through `FrontEnd::action()` (the
//! real input path): the list's last-row initial selection (the populate's msg 0x1d = numVis−1
//! [M C_GCPAUSE_Handler 0x204408 0x204840-0x204850]), ✕ opening the selected row's hint box
//! (control 1's 0x4b case [M 0x205b98-0x205c34]) with the 0xffffffff → 0x31d fallback
//! [M 0x205bd0-0x205bdc], the modal box (Menu_CreateOptionBox 0x202218, ok_only [M 0x1f9608]),
//! and the dismissal re-showing the list [M P_PAUSE_Handler 0x203c28:19-24 — msg 0x50 re-shows
//! control 8; the ok-only box's option_return is 3 (OK) / 4 (cancel), both non-zero]. Numbers/ids
//! only; one NF_ISO check binds the ids against the disc's USATxt.dat.

use nf_view::menu::{Action, FrontEnd, PauseObjective};

fn fe_with_rows(n: usize) -> FrontEnd {
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005; // a mission level so open_pause's guard matches
    assert!(fe.open_pause());
    // The host's rows (pause_objective_rows fills them per frame; here: two objectives of
    // mission 0x07000009's part 0 — the real ids [L c07000009.rs], plus a hint-less row).
    fe.pause_objectives = (0..n).map(|i| PauseObjective {
        text: 0x0400_0016 + i as u32,
        hint: if i == n - 1 { 0xffff_ffff } else { 0x0400_0021 + i as u32 },
        done: false,
    }).collect();
    fe
}

#[test]
fn entering_the_objectives_tab_selects_the_last_row() {
    let mut fe = fe_with_rows(3);
    fe.pause_tab = 0;
    fe.pause_objectives_sel = 0;
    assert!(fe.action(Action::Side), "~H Next cycles the tab");
    assert_eq!(fe.pause_tab, 1);
    // The populate re-selects the LAST row [M 0x204408 0x204840-0x204850 msg 0x1d = numVis−1].
    assert_eq!(fe.pause_objectives_sel, 2, "the tab opens on the last objective");
    // Sister: with one row it opens on row 0 — the selection tracks the count, not a constant.
    let mut one = fe_with_rows(1);
    one.pause_tab = 0;
    one.action(Action::Side);
    assert_eq!(one.pause_objectives_sel, 0);
}

#[test]
fn cross_opens_the_selected_rows_hint() {
    let mut fe = fe_with_rows(3);
    fe.pause_tab = 1;
    fe.pause_objectives_sel = 2; // the last row (the populate's start)
    fe.action(Action::Up);       // ~V up to row 1
    assert_eq!(fe.pause_objectives_sel, 1);
    assert!(fe.action(Action::Confirm), "✕ on an objective shows its hint");
    assert_eq!(fe.pause_hint, Some(0x0400_0022), "row 1's own hint label (record+0x08)");
    // Sister: another row shows another hint — the box is per-row, not a fixed text.
    fe.pause_hint = None;
    fe.pause_objectives_sel = 0;
    fe.action(Action::Confirm);
    assert_eq!(fe.pause_hint, Some(0x0400_0021), "row 0's hint");
}

#[test]
fn a_hintless_row_shows_the_0x31d_fallback_label() {
    let mut fe = fe_with_rows(2);
    fe.pause_tab = 1;
    fe.pause_objectives_sel = 1; // the hint-less row (0xffffffff [M the −1 sentinel])
    fe.action(Action::Confirm);
    assert_eq!(fe.pause_hint, Some(0x31d), "the handler substitutes 0x31d [M 0x205bd0-0x205bdc]");
    // Sister: a hinted row never lands on the fallback.
    fe.pause_hint = None;
    fe.pause_objectives_sel = 0;
    fe.action(Action::Confirm);
    assert_eq!(fe.pause_hint, Some(0x0400_0021));
}

#[test]
fn the_hint_box_is_modal_until_confirmed() {
    let mut fe = fe_with_rows(3);
    fe.pause_tab = 1;
    fe.pause_objectives_sel = 1;
    fe.action(Action::Confirm);
    assert!(fe.pause_hint.is_some());
    // While the box is up the list keys idle (option-box page 0x4000002f is over the pause
    // [M Menu_CreateOptionBox 0x1f9608:45 Manager_SendMessage 0x44]).
    let before = fe.pause_objectives_sel;
    assert!(!fe.action(Action::Down), "the box swallows ~V");
    assert_eq!(fe.pause_objectives_sel, before, "the selection did not move under the box");
    assert!(!fe.action(Action::Side), "the box swallows ~H");
    assert_eq!(fe.pause_tab, 1, "no tab cycle under the box");
    // ✕ dismisses (the OK row [M C_LBMSGOPTIONS_Handler 0x2022a0's 0x4b arm]) and the list is
    // interactive again (msg 0x50 re-shows control 8 [M P_PAUSE_Handler 0x203c28:19-24]).
    assert!(fe.action(Action::Confirm), "✕ pops the box");
    assert!(fe.pause_hint.is_none(), "the box is gone");
    assert!(fe.action(Action::Down), "the list scrolls again");
    assert_eq!(fe.pause_objectives_sel, 2);
    // Sister: the dismissal is a real state change — a fresh ✕ opens a NEW box.
    assert!(fe.action(Action::Confirm));
    assert!(fe.pause_hint.is_some());
}

#[test]
fn back_on_the_box_closes_it_without_unpausing() {
    let mut fe = fe_with_rows(2);
    fe.pause_tab = 1;
    fe.action(Action::Confirm);
    assert!(fe.pause_hint.is_some());
    // Circle cancels the box (msg 0x6d → option_return 4, ok_only [M 0x2022a0]) — it must NOT
    // reach the pause page's resume arm (Back → closed).
    assert!(fe.action(Action::Back));
    assert!(fe.pause_hint.is_none(), "the box closed");
    assert!(!fe.closed, "the pause page is still up");
    // Sister: Back WITHOUT the box is the resume (the pre-existing arm).
    assert!(fe.action(Action::Back));
    assert!(fe.closed);
}

#[test]
fn the_selection_stays_within_the_rows() {
    let mut fe = fe_with_rows(2);
    fe.pause_tab = 1;
    fe.pause_objectives_sel = 0;
    fe.action(Action::Up);
    assert_eq!(fe.pause_objectives_sel, 0, "clamped at the top");
    fe.action(Action::Down);
    fe.action(Action::Down);
    assert_eq!(fe.pause_objectives_sel, 1, "clamped at the last row");
    // Sister: an empty list takes no ✕ (the game's populate falls to the no-objectives arm and
    // control 8's row is meaningless).
    let mut none = fe_with_rows(0);
    none.pause_tab = 1;
    assert!(!none.action(Action::Confirm), "no hint without rows");
    assert!(none.pause_hint.is_none());
}

/// (NF_ISO) The ids the flow shows are the disc's own strings: the OBJECTIVES tab's hint bar
/// 0x202 ("✕ Hint ✚ Scroll ✚ Next" — the populate's bind [M 0x204408 0x2046a0-0x2046b0]), the
/// no-hint fallback 0x31d [M 0x205bd8] and the ok-only box's button 0x266 [M Menu_CreateOptionBox
/// 0x1f9608:34-41]. Txt_LoadLanguage loads USATxt.dat out of FILES.BIN [M 0x1caad0].
#[test]
fn the_hint_labels_bind_from_the_disc() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let image = nf_formats::iso9660::IsoImage::open(&iso).expect("the ISO opens");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&image).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat")).expect("USATxt.dat");
    let bytes = image.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    let lang = nf_formats::hud::Lang::parse(&bytes).expect("the lang parses");
    let bar = lang.bind(0x202).expect("the OBJECTIVES hint bar is a real label");
    assert!(bar.contains("~A"), "the hint bar starts with the ✕ glyph: {bar:?}");
    let no_hint = lang.bind(0x31d).expect("the no-hint fallback is a real label");
    assert!(!no_hint.is_empty());
    let ok = lang.bind(0x266).expect("the ok-only box's button label is a real label");
    assert!(!ok.is_empty());
    // Sister: the fallback is NOT the hint bar's text (distinct labels).
    assert_ne!(no_hint, bar, "0x31d and 0x202 are different strings");
}
