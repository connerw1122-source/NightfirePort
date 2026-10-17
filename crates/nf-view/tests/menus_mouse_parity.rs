//! menu-mouse-parity: the mouse works everywhere the keyboard does.
//!
//! For every page: `mouse_move` over each DRAWN row sets the same cursor the arrows would
//! (a) ; `mouse_click` on a row does what Enter does on that row (b); clicks outside rows do
//! nothing harmful (c); right-click = Back and the scroll wheel = Up/Down (d); no click ever
//! changes a value it shouldn't (e). The hit-test numbers here are the draw code's own
//! positions — `WHEEL_Y` [M menu.rs], the page_rows rows (y 140+26k) [M page_rows_no_head],
//! the pause rows/tabs [M pause_page], the opt rows [M opt_rows()], the Join panels
//! [L page 0x40000019], the Records spinner [L page 0x4000003a], the Screen Adjust bars
//! [L page 0x40000047].
//!
//! Rule: the FrontEnd is driven ONLY through `action()`, `mouse_*()`, `tick()` and the
//! host-side calls the window makes (`open_pause` / `mission_ended` — lib.rs's handoffs; the
//! two stub pages with no action path enter the way lib.rs's `--page` dev override does).
//! Every test carries a sister assertion that can fail.

use nf_view::menu::{self, Action, FrontEnd, Outcome, Page, Stats};

/// One action, then the frames a player would wait: the page-change frames, and any intro /
/// attract movie run out (the headless movies are countdowns).
fn act(fe: &mut FrontEnd, a: Action) -> bool {
    let r = fe.action(a);
    for _ in 0..60 { fe.tick(); }
    for _ in 0..5000 { if !fe.intro_playing && !fe.attract_playing { break; } fe.tick(); }
    r
}

/// Two frames — the mouse paths ride the window's own tick loop; no movie can start from a hover.
fn hover(fe: &mut FrontEnd, x: f32, y: f32) -> bool {
    let r = fe.mouse_move(x, y);
    fe.tick(); fe.tick();
    r
}

/// A left click, then the same wait a key gets (a click IS the row's ✕ and can start a movie).
fn click(fe: &mut FrontEnd, x: f32, y: f32) -> bool {
    let r = fe.mouse_click(x, y);
    for _ in 0..60 { fe.tick(); }
    for _ in 0..5000 { if !fe.intro_playing && !fe.attract_playing { break; } fe.tick(); }
    r
}

/// Boot → title → (intro) → Main.
fn to_main(fe: &mut FrontEnd) {
    assert_eq!(fe.page, Page::Start, "the boot opens the title");
    assert!(act(fe, Action::Confirm), "the title's Enter");
    assert_eq!(fe.page, Page::Main, "after P_ESTHERO's movie, the main menu [M 0x20a060:44-45]");
}

/// Main → NightFire → Play without saving → the default difficulty → the mission list.
fn to_missions(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, Action::Confirm), "NightFire");
    assert_eq!(fe.page, Page::Codename);
    assert_eq!(fe.cursor, 1, "opened from Main on Play without saving [M P_NFSELECT 0x20aec0:96-97]");
    assert!(act(fe, Action::Confirm), "the confirm box opens");
    assert!(act(fe, Action::Confirm), "the box's yes");
    assert_eq!(fe.page, Page::Difficulty);
    assert!(act(fe, Action::Confirm), "the difficulty");
    assert_eq!(fe.page, Page::MissionSelect);
}

/// Main → Multiplayer → the mode page.
fn to_mode(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, Action::Down), "Multiplayer");
    assert!(act(fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpMode);
}

/// Main → Multiplayer → Local → Join Game (the four panels).
fn to_mp_join(fe: &mut FrontEnd) {
    to_mode(fe);
    assert!(act(fe, Action::Confirm), "Local");
    assert_eq!(fe.page, Page::MpJoin);
}

/// Main → Multiplayer → Local → join + ready → the scenario page.
fn to_scenario(fe: &mut FrontEnd) {
    to_mp_join(fe);
    assert!(act(fe, Action::Confirm), "join");
    assert!(act(fe, Action::Confirm), "codename");
    assert!(act(fe, Action::Confirm), "ready");
    assert_eq!(fe.page, Page::MpScenario);
}

/// MissionSelect → □ → the Dossier → Confirm → the Records table.
fn to_records(fe: &mut FrontEnd) {
    to_missions(fe);
    assert!(act(fe, Action::Side), "□ opens the Dossier [M MissionSelect's Side arm]");
    assert_eq!(fe.page, Page::Dossier);
    assert!(act(fe, Action::Confirm), "row 0 = Records [M C_SBDOSSIER's index 0]");
    assert_eq!(fe.page, Page::Records);
}

// ── (a)+(b)+(c): the pages whose rows the mouse must match ──────────────────────────────────

#[test]
fn the_main_menu_rows_hover_and_click_like_the_arrows_and_enter() {
    let mut fe = FrontEnd::new();
    to_main(&mut fe);
    // The three rows draw at y 262/294/326 in controls (167,·,306×28) [M menu-def 0x40000002].
    // The walk starts on row 0 (the page's cursor), so the hovers go 1 → 2 → 0: each one CHANGES
    // the cursor and must report the focus change.
    for (row, y) in [(1usize, 308.0f32), (2, 340.0), (0, 276.0)] {
        let moved = hover(&mut fe, 320.0, y);
        assert_eq!(fe.cursor, row, "hovering row {row} selects it");
        assert!(moved, "the hover reports the focus change (the game's cursor-over blip)");
        // Sister: off the rows the cursor stands still.
        hover(&mut fe, 40.0, y);
        assert_eq!(fe.cursor, row, "a hover off the row column changes nothing");
    }
    // A click on row 2 does what Enter does on it: Codenames → Edit Codename.
    assert!(click(&mut fe, 320.0, 340.0));
    assert_eq!(fe.page, Page::CnSelect);
    assert_eq!(fe.cursor, 1, "the Default row [M C_GOCODENAMES 0x216b38]");
    // Sister: a click on empty space never leaves the page.
    let mut g = FrontEnd::new();
    to_main(&mut g);
    assert!(!click(&mut g, 40.0, 40.0), "the corner click does nothing");
    assert!(!click(&mut g, 320.0, 200.0), "a click above the rows does nothing");
    assert_eq!(g.page, Page::Main);
}

#[test]
fn the_title_takes_the_click_and_the_movies_burn() {
    let mut fe = FrontEnd::new();
    // The title is a single-action page: a click is ✕ = the intro [M P_START 0x20a408].
    assert!(click(&mut fe, 320.0, 240.0));
    assert_eq!(fe.page, Page::Main, "the intro burned down to the main menu");
    // Sister: the same walk by Enter agrees.
    let mut g = FrontEnd::new();
    to_main(&mut g);
    assert_eq!(g.page, Page::Main);
}

/// The wheels: hover selects the hovered row (the wheel re-centres on it), a hover+click
/// confirms the aimed row, a direct click falls back to the slot under the pointer, an empty
/// slot and the area left of the wheel do nothing — and a locked row's click is refused.
#[test]
fn the_wheels_hover_scroll_and_click_like_the_arrows_and_enter() {
    // Difficulty (3 rows, entered from the codename page on index 0 [M P_NFDFCTY 0x216bf8:10-12]):
    // the wheel slots draw at WHEEL_Y = [123,152,182,211,241] [M menu.rs].
    let mut fe = FrontEnd::new();
    to_missions(&mut fe); // the wheel walked Codename → Difficulty on the way (row 0 picked)
    assert_eq!(fe.page, Page::MissionSelect);
    // Back to the difficulty wheel — the keyboard's own path.
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Difficulty);
    assert_eq!(fe.cursor, 0);
    // Hover the slot BELOW the centre: the wheel scrolls to the row it showed — one Down.
    assert!(hover(&mut fe, 450.0, 211.0), "the hover moved the wheel");
    assert_eq!(fe.cursor, 1, "the row the slot showed is now the selection");
    assert!(hover(&mut fe, 450.0, 160.0), "the slot above the centre");
    assert_eq!(fe.cursor, 0);
    // Sister: the empty top slots hover nothing at the list's start (the drawn wheel shows no
    // row there).
    assert!(!hover(&mut fe, 450.0, 131.0), "the empty slot hovers nothing");
    assert_eq!(fe.cursor, 0);
    // The keyboard WRAPS here [M Scroll_Init 0x2214f0:102-103 sets the wrap flag; Scroll_Update
    // 0x221b20:202-210] — Up from Operative is 00 Agent; Down comes back.
    assert!(act(&mut fe, Action::Up));
    assert_eq!(fe.cursor, 2, "the keyboard's Up wraps");
    assert!(act(&mut fe, Action::Down));
    assert_eq!(fe.cursor, 0);
    // Hover the slot BELOW the centre (the centre slot shows the selection — hovering it moves
    // nothing), then click the centre: the aimed row (row 1, Agent) takes the ✕ — the pick.
    assert!(hover(&mut fe, 450.0, 211.0));
    assert_eq!(fe.cursor, 1);
    assert!(click(&mut fe, 450.0, 190.0));
    assert_eq!(fe.page, Page::MissionSelect, "the aimed row's ✕ opened the mission list");
    assert_eq!(fe.requested_difficulty, Some(2), "Agent [M 0x216ca8:15's +1]");

    // MissionSelect opens on the highest unlocked row, The Exchange [M P_NFMAP 0x20b608:60-64].
    // Hover the row above (Paris Prelude), then back below and click — one move, one click.
    let mut fe2 = FrontEnd::new();
    to_missions(&mut fe2);
    assert_eq!(fe2.cursor, 1, "The Exchange [M 0x20b608:60-64]");
    assert!(hover(&mut fe2, 450.0, 160.0), "the neighbour row's hover scrolls");
    assert_eq!(fe2.cursor, 0, "Paris Prelude");
    assert!(hover(&mut fe2, 450.0, 211.0));
    assert_eq!(fe2.cursor, 1, "The Exchange");
    assert!(click(&mut fe2, 450.0, 211.0));
    assert_eq!(fe2.requested_level, Some(0x0700_0005), "the aimed row's ✕ starts it");

    // A DIRECT click (no hover behind it) keeps the page's own contract: the centre confirms,
    // a neighbour only scrolls.
    let mut fe3 = FrontEnd::new();
    to_missions(&mut fe3);
    assert!(click(&mut fe3, 450.0, 160.0), "the neighbour slot's click scrolls");
    assert_eq!(fe3.cursor, 0);
    assert_eq!(fe3.requested_level, None, "the scroll alone did not start anything");
    // Sister: left of the wheel column nothing happens — the old code confirmed from there.
    let mut fe4 = FrontEnd::new();
    to_missions(&mut fe4);
    assert!(!click(&mut fe4, 100.0, 190.0), "a click on the emblem side does nothing");
    assert_eq!(fe4.cursor, 1);
    assert_eq!(fe4.requested_level, None);
    // Sister: a locked row can be hovered (the wheel shows it greyed) but its ✕ is refused
    // [M C_SBNFMAP 0x20b8d8:21-23].
    assert!(hover(&mut fe4, 450.0, 211.0), "one down");
    assert_eq!(fe4.cursor, 2, "a locked row (rows 2+ start locked [D sp_level boot +0x10])");
    assert!(!fe4.unlocked_row(2));
    assert!(!click(&mut fe4, 450.0, 211.0), "the locked row's ✕ is refused");
    assert_eq!(fe4.requested_level, None);
}

#[test]
fn the_pause_tabs_rows_and_boxes_take_the_mouse() {
    let mut fe = FrontEnd::new();
    // The host opens the pause over a loaded level (lib.rs's handoff).
    fe.flow.current_level = 0x0700_0005;
    assert!(fe.open_pause());
    assert_eq!(fe.page, Page::Pause);
    // The tab strip [L the four type-1 controls (84/199/339/465,120,·×16)]: hover shows the tab,
    // a click switches to it — the same tab ◀ ▶ would land on.
    assert!(hover(&mut fe, 400.0, 128.0), "hovering the CONTROLS tab shows it");
    assert_eq!(fe.pause_tab, 2);
    assert!(hover(&mut fe, 100.0, 128.0));
    assert_eq!(fe.pause_tab, 0, "back on MISSION");
    // Sister: off the strip the tab stands.
    hover(&mut fe, 320.0, 160.0);
    assert_eq!(fe.pause_tab, 0);
    // The MISSION rows [M pause_page: the row middles 209/242.5/276]: hover RESTART, click it.
    assert!(hover(&mut fe, 320.0, 242.0));
    assert_eq!(fe.cursor, 1);
    assert!(click(&mut fe, 320.0, 242.0), "RESTART's ✕ opens its confirm");
    assert!(fe.confirming_restart);
    // The YES/NO rows [L the d1=0xd list (238,210,165×86): YES 237, NO 270]: the box opens on NO.
    assert_eq!(fe.confirm_sel, 1);
    assert!(hover(&mut fe, 320.0, 237.0));
    assert_eq!(fe.confirm_sel, 0, "hovering YES moves the box's selection");
    assert!(click(&mut fe, 320.0, 270.0), "NO's ✕ is handled (the NO arm closes the box)");
    assert!(!fe.confirming_restart, "the box closed");
    assert_eq!(fe.requested_level, None, "NO restarted nothing");
    // Sister: YES executes — the level request rides it (a fresh pause; the host re-opens).
    // The restart target is the campaign row's base map [M 0x204408:138-141] — the host's
    // mission state rides fe.mission (The Exchange here, the loaded level's row).
    let mut fe2 = FrontEnd::new();
    fe2.flow.current_level = 0x0700_0005;
    fe2.mission = 1;
    assert!(fe2.open_pause());
    assert!(hover(&mut fe2, 320.0, 242.0));
    assert!(click(&mut fe2, 320.0, 242.0));
    assert!(hover(&mut fe2, 320.0, 237.0));
    assert!(click(&mut fe2, 320.0, 237.0), "YES's ✕ restarts");
    assert_eq!(fe2.requested_level, Some(0x0700_0005), "the game's own restart target [M 0x204408:138-141]");
    // Right-click = Back closes the pause [M 0x204408's d1=6 case-0 arm] — the host's mapping.
    let mut fe3 = FrontEnd::new();
    fe3.flow.current_level = 0x0700_0005;
    assert!(fe3.open_pause());
    assert!(act(&mut fe3, menu::MOUSE_RIGHT_ACTION), "the right-click's Back resumes");
    assert!(fe3.closed, "the pause closed");
}

#[test]
fn the_pause_objectives_rows_hover_and_click_like_the_keyboard() {
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    assert!(fe.open_pause());
    // The host fills the objectives from the sim's mission states (lib.rs's pause fill).
    fe.pause_objectives = vec![
        menu::PauseObjective { text: 0x0400_0000, hint: 0x0400_005b, done: false },
        menu::PauseObjective { text: 0x0400_0001, hint: 0xffff_ffff, done: true },
    ];
    // To the OBJECTIVES tab by the keyboard's own tab-cycle (□ = the game's ~H Next).
    assert!(act(&mut fe, Action::Side));
    assert_eq!(fe.pause_tab, 1);
    assert_eq!(fe.pause_objectives_sel, 1, "the tab opens on the LAST row [M msg 0x1d = numVis−1]");
    // The rows draw at y 159+22i [M pause_page's tab-1 arm]: hover row 0, click it — the hint.
    assert!(hover(&mut fe, 300.0, 159.0));
    assert_eq!(fe.pause_objectives_sel, 0, "the hover scrolled the tab's selection");
    assert!(click(&mut fe, 300.0, 159.0), "the row's ✕ shows the hint");
    assert!(fe.pause_hint.is_some(), "the hint box is up");
    // Sister: the box is modal — hovers do nothing under it, Enter dismisses.
    assert!(!hover(&mut fe, 300.0, 181.0), "the box blocks the hover");
    assert_eq!(fe.pause_objectives_sel, 0);
    assert!(act(&mut fe, Action::Confirm), "Enter dismisses");
    assert!(fe.pause_hint.is_none());
    // Sister: a click below the rows does nothing.
    assert!(!click(&mut fe, 300.0, 350.0), "off the rows nothing opens");
    assert!(fe.pause_hint.is_none());
    let _ = &fe;
}

#[test]
fn the_records_spinner_takes_the_clicks_the_drawn_arrows_advertise() {
    let mut fe = FrontEnd::new();
    to_records(&mut fe);
    assert_eq!(fe.cursor, 0);
    // The drawn ◀ ▶ at (144/497, mid y 133) [L page 0x4000003a] walk the campaign — ~V.
    assert!(click(&mut fe, 144.0, 133.0), "◀ = Up (the list wraps [M the Records arm])");
    assert_eq!(fe.cursor, 11);
    assert!(click(&mut fe, 497.0, 133.0), "▶ = Down");
    assert_eq!(fe.cursor, 0);
    // A click on the spinner row itself is ✕ — Enter's action (back to the Dossier).
    assert!(click(&mut fe, 300.0, 132.0));
    assert_eq!(fe.page, Page::Dossier);
    // Sister: the score rows below are readouts — clicks do nothing there.
    let mut g = FrontEnd::new();
    to_records(&mut g);
    assert!(!click(&mut g, 300.0, 202.0), "the High Score row is not a control");
    assert_eq!(g.page, Page::Records);
    assert_eq!(g.cursor, 0);
}

#[test]
fn the_screen_adjust_bars_hover_and_click_like_the_keyboard() {
    let mut fe = FrontEnd::new();
    // Main → Codenames → Edit Codename → the AV options → the Screen Adjust row — the
    // keyboard's own path (CnMenu row 5, the AV page's row 6 [L page data y=256]).
    to_main(&mut fe);
    // fe-audit 1017: Main's rows are M_BUTTONs [D def 08000002 page 40000002, ctrls
    // 10000002/3/4 kind byte 1], so Scroll_Init's wrap flag never applies here and the button
    // focus walk is not in the decompile's C — the wrap itself is [S unverified]. The walk
    // selects the Codenames row directly instead of relying on it.
    fe.cursor = 2; // the Codenames row [S — no decompile evidence pins Main's focus walk]
    assert!(act(&mut fe, Action::Confirm), "Codenames");
    assert!(act(&mut fe, Action::Confirm), "the Default row's Edit");
    assert_eq!(fe.page, Page::CnMenu);
    for _ in 0..4 { assert!(act(&mut fe, Action::Down)); }
    assert!(act(&mut fe, Action::Confirm), "AV Options");
    assert_eq!(fe.page, Page::CnAv);
    for _ in 0..6 { assert!(act(&mut fe, Action::Down)); }
    assert!(act(&mut fe, Action::Confirm), "Screen Adjust");
    assert_eq!(fe.page, Page::ScreenAdjust);
    assert_eq!(fe.cursor, 0, "the vertical bar first [L page data's d1 0 arm]");
    // The bars [L page 0x40000047: vertical (304,176,32×128), horizontal (256,224,128×32)]:
    // hover picks a bar (the walk starts on the vertical, so it goes horizontal first); the
    // overlap picks the vertical (the page data's order).
    assert!(hover(&mut fe, 370.0, 240.0));
    assert_eq!(fe.cursor, 1, "the horizontal bar");
    assert!(hover(&mut fe, 320.0, 240.0));
    assert_eq!(fe.cursor, 0, "the vertical bar wins the overlap");
    // ◀ steps the horizontal offset (the keyboard's own step, kept as the sister) — the
    // horizontal bar first.
    assert!(hover(&mut fe, 370.0, 240.0));
    assert!(act(&mut fe, Action::Left));
    assert_eq!(fe.screen_x, -12, "psiAdjustScreenPos's X step 12 [M 0x10b890]");
    // A click on the bar is ✕: Enter keeps the offset and leaves to the AV page.
    assert!(click(&mut fe, 370.0, 240.0));
    assert_eq!(fe.page, Page::CnAv);
    assert_eq!(fe.cursor, 6, "the Screen Adjust row [L page data y=256 is the 7th row]");
    assert_eq!(fe.screen_x, -12, "the ✕ kept the offset [M 0x214818:127-136]");
    // Sister: a click off the bars does nothing.
    assert!(act(&mut fe, Action::Confirm), "back into Screen Adjust");
    assert!(!click(&mut fe, 100.0, 100.0), "the picture area is not a control");
    assert_eq!(fe.page, Page::ScreenAdjust);
}

#[test]
fn the_options_page_rows_hover_and_click_like_the_keyboard() {
    let mut fe = FrontEnd::new();
    // Page::Options has no action path yet (no enter_page(Page::Options) anywhere [M]) — the
    // test enters it the way the host's --page dev override does [M lib.rs page_from_name].
    to_main(&mut fe);
    fe.page = Page::Options;
    fe.cursor = 0;
    // The rows draw through page_rows at y 140/166/192 [M page_rows_no_head]; the walk starts
    // on row 0, so the hovers go 1 → 2 → 0 and each one reports the change.
    assert!(hover(&mut fe, 320.0, 172.0));
    assert_eq!(fe.cursor, 1);
    assert!(hover(&mut fe, 320.0, 198.0));
    assert_eq!(fe.cursor, 2);
    assert!(hover(&mut fe, 320.0, 146.0));
    assert_eq!(fe.cursor, 0);
    // The stub rows' ✕ is the keyboard's no-op [M action()'s Options arm: only row 2 acts].
    assert!(!click(&mut fe, 320.0, 146.0), "Screen adjust's stub ✕ does nothing");
    assert!(!click(&mut fe, 320.0, 172.0), "Controls' stub ✕ does nothing");
    assert_eq!(fe.page, Page::Options);
    // The Back row's ✕ leaves — what Enter does on it.
    assert!(click(&mut fe, 320.0, 198.0));
    assert_eq!(fe.page, Page::Main);
    // Sister: a click off the rows does nothing.
    let mut g = FrontEnd::new();
    to_main(&mut g);
    g.page = Page::Options;
    assert!(!click(&mut g, 320.0, 300.0));
    assert_eq!(g.page, Page::Options);
}

// ── (a)+(b)+(c): the codename option lists (the drawn opt rows and their value columns) ─────

#[test]
fn the_cn_option_lists_hover_rows_and_click_their_columns() {
    let mut fe = FrontEnd::new();
    to_main(&mut fe);
    assert!(act(&mut fe, Action::Up));
    assert!(act(&mut fe, Action::Confirm), "Codenames");
    assert!(act(&mut fe, Action::Confirm), "Edit the Default");
    for _ in 0..2 { assert!(act(&mut fe, Action::Down)); }
    assert!(act(&mut fe, Action::Confirm), "Advanced Options");
    assert_eq!(fe.page, Page::CnAdvanced);
    // The rows draw from opt_rows()'s own y's [M opt_list] — hover each at its middle (the walk
    // starts on row 0, so it goes 1 → 2 → 0; each hover changes the cursor).
    let auto_aim = menu::ADV_ROWS[1].ctrl;
    for (row, r) in [(1usize, &menu::ADV_ROWS[1]), (2, &menu::ADV_ROWS[2]), (0, &menu::ADV_ROWS[0])] {
        assert!(hover(&mut fe, 320.0, r.y + 8.0), "hovering the drawn row {row} selects it");
        assert_eq!(fe.cursor, row);
    }
    // A click on the value column's ▶ steps the hovered row's value — the keyboard's ▶.
    assert!(hover(&mut fe, 560.0, menu::ADV_ROWS[1].y + 8.0));
    assert_eq!(fe.opt_value(auto_aim), 1, "Auto Aim defaults On");
    assert!(click(&mut fe, 560.0, menu::ADV_ROWS[1].y + 8.0));
    assert_eq!(fe.opt_value(auto_aim), 0, "the click stepped On → Off");
    // Sister: a click off the rows changes nothing.
    let v = fe.opt_value(auto_aim);
    assert!(!click(&mut fe, 320.0, 60.0), "the header is not a row");
    assert_eq!(fe.opt_value(auto_aim), v);
    // The action row's ✕ opens its page — the AV page's Screen Adjust row (AV_ROWS[6], ctrl
    // 0x1000_0137 [M 0x214818:117-126]; ADV_ROWS holds only List rows). Back to the wheel, then
    // the keyboard's own walk to AV Options.
    assert!(act(&mut fe, Action::Back), "back to the wheel (the edits ride Back's restore [M])");
    assert_eq!(fe.page, Page::CnMenu);
    for _ in 0..2 { assert!(act(&mut fe, Action::Down)); }
    assert!(act(&mut fe, Action::Confirm), "AV Options");
    assert_eq!(fe.page, Page::CnAv);
    let sa_row = menu::AV_ROWS.iter().position(|r| r.ctrl == 0x1000_0137).expect("the Screen Adjust row");
    assert!(hover(&mut fe, 320.0, menu::AV_ROWS[sa_row].y + 8.0));
    assert!(click(&mut fe, 320.0, menu::AV_ROWS[sa_row].y + 8.0));
    assert_eq!(fe.page, Page::ScreenAdjust);
}

// ── (a)-(e): the multiplayer pages ──────────────────────────────────────────────────────────

#[test]
fn the_mp_mode_host_and_join_pages_click_their_drawn_rows() {
    let mut fe = FrontEnd::new();
    to_mode(&mut fe);
    // The rows draw at 140/166/192 [M page_rows_no_head]: hover Host Online, click it.
    assert!(hover(&mut fe, 320.0, 172.0));
    assert_eq!(fe.cursor, 1);
    assert!(click(&mut fe, 320.0, 172.0));
    assert_eq!(fe.page, Page::MpHost);
    assert_eq!(fe.mp_net_port, 27420, "the plan's default [S §2.1]");
    // (e) the crown jewel: clicks on the port row — anywhere on it — never change the port.
    // The row is a typed field; the keyboard's ✕ on it does nothing [M action()'s MpHost arm].
    // (The page opens on the port row, so the hover reports no change.)
    hover(&mut fe, 320.0, 146.0);
    assert_eq!(fe.cursor, 0);
    for x in [200.0, 320.0, 370.0, 450.0, 560.0] {
        click(&mut fe, x, 146.0);
        assert_eq!(fe.mp_net_port, 27420, "the click at x {x} left the port alone");
        assert_eq!(fe.page, Page::MpHost);
    }
    // The port VALUE line (200,250,240×18) is a readout, not a row: its click does nothing.
    assert!(!click(&mut fe, 320.0, 259.0));
    assert_eq!(fe.mp_net_port, 27420);
    // The Start row's click does what Enter does: the listen — refused here (no session rides
    // the front end in this test), which is the keyboard's own refusal [M mp_host_begin]: the
    // ✕ reports false (nothing happened) and the error surfaces in the status line.
    assert!(hover(&mut fe, 320.0, 172.0));
    assert!(!click(&mut fe, 320.0, 172.0), "the refused listen reports nothing");
    assert!(!fe.mp_hosting, "no session, no listen — the keyboard refuses the same way");
    assert!(fe.mp_net_error.is_some(), "the status line carries the refusal");
    // The join side: the address line's click dials (an empty address refuses like Enter's).
    let mut fe2 = FrontEnd::new();
    to_mode(&mut fe2);
    assert!(act(&mut fe2, Action::Down));
    assert!(act(&mut fe2, Action::Down), "past Host Online");
    assert!(act(&mut fe2, Action::Confirm), "Join Online");
    assert_eq!(fe2.page, Page::MpNetJoin);
    assert!(!click(&mut fe2, 320.0, 259.0), "the empty address refuses [M mp_net_dial]");
    assert!(!click(&mut fe2, 320.0, 146.0), "sister: off the drawn row nothing dials");
}

#[test]
fn the_mp_join_panels_hover_and_click_like_the_keyboard() {
    let mut fe = FrontEnd::new();
    to_mp_join(&mut fe);
    // The four panels [L page 0x40000019: (50/328, 103/253, 262×134)].
    assert!(hover(&mut fe, 459.0, 170.0), "hovering panel 2 walks the cursor to it");
    assert_eq!(fe.cursor, 1);
    assert!(!click(&mut fe, 459.0, 170.0), "panel 2's ✕ does nothing (the keyboard is agent 1 [S])");
    assert_eq!(fe.mp_join[0].state, 0, "agent 1 stayed idle");
    // Panel 1's click joins agent 1 — what Enter does on it.
    assert!(hover(&mut fe, 180.0, 170.0));
    assert_eq!(fe.cursor, 0);
    assert!(click(&mut fe, 180.0, 170.0), "agent 1's panel ✕ joins");
    assert_eq!(fe.mp_join[0].state, 1);
    // Sister: a click outside the panels does nothing.
    assert!(!click(&mut fe, 320.0, 420.0), "the hint bar is not a panel");
    assert_eq!(fe.mp_join[0].state, 1);
    // The rest of the walk by clicks: the codename pick, then ready → the scenario page.
    assert!(click(&mut fe, 180.0, 170.0), "the codename's ✕");
    assert_eq!(fe.mp_join[0].state, 5);
    assert!(click(&mut fe, 180.0, 170.0), "the ready ✕");
    assert_eq!(fe.page, Page::MpScenario);
}

#[test]
fn the_mp_wheels_hover_and_click_like_the_keyboard() {
    let mut fe = FrontEnd::new();
    to_scenario(&mut fe);
    // The wheel opens on the row of MPSettings+0x1A4 = 1, Arena [M P_MPSCENARIO 0x217348:25;
    // bootup 0x1c7e78:30].
    assert_eq!(fe.mp_scenario_row, 1, "Arena");
    assert!(hover(&mut fe, 450.0, 160.0));
    assert_eq!(fe.mp_scenario_row, 0, "Quick Game");
    // Hover the row below the centre, click it — the aimed scenario's ✕ opens the map wheel.
    assert!(hover(&mut fe, 450.0, 211.0));
    assert_eq!(fe.mp_scenario_row, 1, "Arena");
    assert!(click(&mut fe, 450.0, 211.0));
    assert_eq!(fe.page, Page::MpMap, "the non-quick scenario's ✕ opens the map [M C_SBMPSCEN]");
    // The map wheel's centre confirms by a direct click too (the fallback contract).
    assert!(click(&mut fe, 450.0, 190.0));
    assert_eq!(fe.page, Page::MpSetup, "the map's ✕ opens the setup");
    // Sister: the setup page keeps the keyboard (the task's own carve-out) — a click does
    // nothing there.
    assert!(!click(&mut fe, 320.0, 240.0), "the setup panels keep the keyboard");
    assert_eq!(fe.page, Page::MpSetup);
}

#[test]
fn the_dossier_gadget_and_weapon_wheels_hover_like_the_keyboard() {
    let mut fe = FrontEnd::new();
    to_missions(&mut fe);
    assert!(act(&mut fe, Action::Side), "the Dossier");
    // Down to the Gadgets wheel (row 2), open it.
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Confirm), "Gadgets");
    assert_eq!(fe.page, Page::Gadgets);
    let n = fe.rows();
    // Hover the slot below the centre: the wheel scrolls — the keyboard's Down.
    assert!(hover(&mut fe, 450.0, 211.0));
    assert_eq!(fe.cursor, 1);
    // Sister: left of the wheel column nothing happens (the old code ran Confirm from there —
    // Gadgets' Confirm is a no-op [M C_RBDSREWARDS's missing select arm], but the contract is
    // the same: no click off the rows acts).
    assert!(!click(&mut fe, 100.0, 190.0));
    assert_eq!(fe.cursor, 1);
    // The centre's ✕ is the keyboard's Confirm: nothing (no select arm [M]).
    assert!(!click(&mut fe, 450.0, 190.0));
    assert_eq!(fe.page, Page::Gadgets);
    // The Weapons wheel behaves the same (row 3 of the dossier wheel; Back lands the dossier
    // wheel on row 0 [M the Gadgets Back arm]).
    assert!(act(&mut fe, Action::Back), "back to the Dossier");
    for _ in 0..3 { assert!(act(&mut fe, Action::Down)); }
    assert!(act(&mut fe, Action::Confirm), "Weapons");
    assert_eq!(fe.page, Page::Weapons);
    assert!(hover(&mut fe, 450.0, 211.0));
    assert_eq!(fe.cursor, 1);
    let _ = n;
}

#[test]
fn the_cheat_medal_clicks_do_nothing_until_the_page_draws_rows() {
    let mut fe = FrontEnd::new();
    // The cheat-medal page has no action path (it opens from a cheat acceptance) and no drawn
    // rows (the overlay's arm is empty [M menu_overlay]) — the test enters it the way the
    // host's dev override enters a stub page, and asserts the mouse is INERT on it: no drawn
    // row to hit, so no click may toggle a cheat.
    to_main(&mut fe);
    fe.page = Page::CheatMedal;
    fe.cursor = 0;
    let sel = fe.cheat_medal_sel;
    for (x, y) in [(320.0, 200.0), (100.0, 100.0), (450.0, 190.0), (320.0, 380.0)] {
        assert!(!click(&mut fe, x, y), "the click at ({x},{y}) did nothing");
        assert_eq!(fe.cheat_medal_sel, sel, "no click moved the keyboard's selection");
    }
    assert_eq!(fe.page, Page::CheatMedal);
}

// ── (d): right-click = Back, the scroll wheel = Up / Down ───────────────────────────────────

#[test]
fn right_click_is_back_and_the_wheel_is_up_and_down() {
    // The host's mappings [M lib.rs's MouseButton::Right / DeviceEvent::MouseWheel arms].
    assert_eq!(menu::MOUSE_RIGHT_ACTION, Action::Back);
    assert_eq!(menu::wheel_scroll_action(1.0), Some(Action::Up));
    assert_eq!(menu::wheel_scroll_action(-1.0), Some(Action::Down));
    assert_eq!(menu::wheel_scroll_action(0.0), None, "a zero delta is no action");
    // The wheel's Up/Down walk the menu like the d-pad: the mission list's wheel skips the
    // locked rows and wraps [S the port's step]. The list opens on The Exchange [M 0x20b608:60-64].
    let mut fe = FrontEnd::new();
    to_missions(&mut fe);
    assert_eq!(fe.cursor, 1);
    assert!(act(&mut fe, menu::wheel_scroll_action(1.0).unwrap()), "the wheel's Up");
    assert_eq!(fe.cursor, 0, "Paris Prelude");
    assert!(act(&mut fe, menu::wheel_scroll_action(-1.0).unwrap()), "the wheel's Down");
    assert_eq!(fe.cursor, 1, "The Exchange");
    // The right-click's Back walks the chain: missions → difficulty → codename → main.
    assert!(act(&mut fe, menu::MOUSE_RIGHT_ACTION));
    assert_eq!(fe.page, Page::Difficulty);
    assert!(act(&mut fe, menu::MOUSE_RIGHT_ACTION));
    assert_eq!(fe.page, Page::Codename);
    assert!(act(&mut fe, menu::MOUSE_RIGHT_ACTION));
    assert_eq!(fe.page, Page::Main);
}

// ── (e): the port and the other protected values ────────────────────────────────────────────

#[test]
fn clicks_never_change_what_they_should_not() {
    let mut fe = FrontEnd::new();
    to_mode(&mut fe);
    assert!(act(&mut fe, Action::Down), "Host Online");
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpHost);
    // Sweep the mouse over the whole page clicking everything; the typed port never moves —
    // only the keyboard's typing changes it [M mp_net_type / mp_net_backspace].
    let mut rng = Rng::new(0x5EED_1A2B);
    for _ in 0..400 {
        let (x, y) = (rng.f32_in(0.0..640.0), rng.f32_in(0.0..480.0));
        fe.mouse_move(x, y);
        fe.mouse_click(x, y);
        fe.tick();
        assert_eq!(fe.mp_net_port, 27420, "the port survived a click at ({x},{y})");
    }
    // The difficulty ride-along: clicks on the difficulty wheel pick rows only through the
    // page's own confirm — the value lands in requested_difficulty once, not per click.
    // fe-audit 1017: Back from the mission list opens MAIN [M 0x20b608:67-68], so the wheel is
    // reached again by the walk, not by Back.
    let mut g = FrontEnd::new();
    to_missions(&mut g);
    assert!(act(&mut g, Action::Back));
    assert_eq!(g.page, Page::Main, "the mission list's △ opens the main menu [M 0x20b608:67-68]");
    assert!(act(&mut g, Action::Confirm), "NightFire");
    assert!(act(&mut g, Action::Confirm), "the box arms");
    assert!(act(&mut g, Action::Confirm), "the box's yes → the wheel");
    assert_eq!(g.page, Page::Difficulty);
    for _ in 0..60 {
        let (x, y) = (rng.f32_in(0.0..640.0), rng.f32_in(0.0..480.0));
        g.mouse_move(x, y);
        g.mouse_click(x, y);
        for _ in 0..4 { g.tick(); }
        if g.page == Page::Difficulty {
            assert!(g.cursor < g.rows());
        } else {
            // A wheel-row ✕ picks the hovered row — the tier's stored value is the boot table's
            // +0xc lane: 1/2/3 [D ACTION.ELF .data 0x2df4c0, records 0..2]; the store is
            // GameState+0x40 = difficulty[i*0x18+0xc] [M 0x216ca8:15]. Nothing else may pick.
            assert!(matches!(g.requested_difficulty, Some(1..=3)),
                "only a wheel row's ✕ picks — got {:?}", g.requested_difficulty);
            break;
        }
    }
}

// ── the fuzz: 20 seeded runs of 5000 steps, moves + clicks + keys, the 669 invariants ───────

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Rng(seed ^ 0x9E37_79B9_7F4A_7C15) }
    fn next_u64(&mut self) -> u64 {
        // xorshift64* — deterministic, no deps.
        let mut x = self.0;
        x ^= x >> 12; x ^= x << 25; x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn f32_in(&mut self, r: std::ops::Range<f32>) -> f32 {
        let t = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32; // 0..1
        r.start + t * (r.end - r.start)
    }
    fn below(&mut self, n: u64) -> u64 { self.next_u64() % n.max(1) }
}

/// The 669 invariants after each step: no panic (the harness), the cursor in range, and Back
/// leaves (a Back either acts, or changes the page, or the page legitimately takes no Back —
/// the title, the credits, the win page and the debriefing [M their action() arms]).
fn assert_invariants(fe: &FrontEnd, ctx: &str) {
    let rows = fe.rows();
    assert!(rows == 0 || fe.cursor < rows, "{ctx}: cursor {} out of range (rows {rows}) on {:?}", fe.cursor, fe.page);
    if fe.is_wheel_page() {
        assert!(fe.page_row() < rows, "{ctx}: the wheel row {} out of range ({rows}) on {:?}", fe.page_row(), fe.page);
    }
    assert!(fe.cheat_medal_sel < 6, "{ctx}: the cheat-medal selection out of range");
}

fn back_leaves(fe: &mut FrontEnd, step: usize) {
    if fe.input_blocked() { let _ = fe.action(Action::Back); return; }   // the box's dismiss
    let before = fe.page;
    let acted = fe.action(Action::Back);
    for _ in 0..8 { fe.tick(); }
    assert!(acted || fe.page != before || matches!(before, Page::Start | Page::Credits | Page::WinGame | Page::MpDebriefing),
        "step {step}: Back left {:?} stuck (acted {acted})", before);
}

#[test]
fn fuzz_the_menus_with_the_mouse_and_the_keyboard() {
    let keys = [Action::Up, Action::Down, Action::Confirm, Action::Back, Action::Side, Action::Left, Action::Right];
    for seed in 0..20u64 {
        let mut rng = Rng::new(0xC0FF_EE00 ^ seed);
        let mut fe = FrontEnd::new();
        for step in 0..5000usize {
            let ctx = format!("seed {seed} step {step}");
            match rng.below(100) {
                0..=39 => { hover(&mut fe, rng.f32_in(0.0..640.0), rng.f32_in(0.0..480.0)); }
                40..=64 => { click(&mut fe, rng.f32_in(0.0..640.0), rng.f32_in(0.0..480.0)); }
                65..=89 => { act(&mut fe, keys[rng.below(keys.len() as u64) as usize]); }
                _ => { for _ in 0..8 { fe.tick(); } }
            }
            assert_invariants(&fe, &ctx);
            if step % 50 == 0 { back_leaves(&mut fe, step); }
        }
        // (e) across the whole run: the typed port only moves through typing.
        assert_eq!(fe.mp_net_port, 27420, "seed {seed}: the port was knocked down by the fuzz");
        assert_invariants(&fe, &format!("seed {seed} end"));
    }
}
