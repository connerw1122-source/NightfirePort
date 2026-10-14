//! The dev window (devwin.rs): the dev menu in its own native window beside the game (the user, 2026-10-13: "the dev
//! menu should manifest outside of the game, but not in a browser"). The window itself can't open in a test, so these
//! drive its pure parts: the layout at small and large window sizes, and the input path (`devwin::input`, which the
//! window's event handler calls after `devwin::decode`) through to the shared action router
//! (`dev::apply_menu_action_to_scene` → `panel::apply`, what `App::apply_menu_action` runs).
//!
//! The layout and wrap tests need no disc; the flows are opt-in via `NF_ISO`.

use nf_view::dev;
use nf_view::devwin::{self, Input, Rect, Request};
use winit::keyboard::KeyCode;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Every part of the layout lies inside the window, the rows don't overlap, and the rows fit the list.
fn assert_fits(w: f32, h: f32, dpi: f32, n: usize, scroll: usize) {
    let l = devwin::layout(w, h, dpi, n, scroll);
    let parts = [("header", l.header), ("filter", l.filter), ("list", l.list), ("desc", l.desc), ("status", l.status), ("footer", l.footer)];
    for (name, r) in parts { assert!(r.inside(w, h), "{name} {r:?} leaves the {w}x{h} window (dpi {dpi})"); }
    for (i, r) in l.tabs.iter().enumerate() { assert!(r.inside(w, h), "tab {i} {r:?} leaves the {w}x{h} window (dpi {dpi})"); }
    for (i, r) in &l.rows {
        assert!(r.inside(w, h), "row {i} {r:?} leaves the window");
        assert!(r.y >= l.list.y - 0.5 && r.y + r.h <= l.list.y + l.list.h + 0.5, "row {i} {r:?} leaves the list {:?}", l.list);
    }
    for pair in l.rows.windows(2) { assert!(pair[0].1.y + pair[0].1.h <= pair[1].1.y + 0.01, "rows overlap: {pair:?}"); }
    assert!(l.desc.y >= l.list.y + l.list.h - 0.5, "the description sits under the list");
    assert_eq!(l.rows.len(), l.visible.min(n.saturating_sub(scroll.min(n.saturating_sub(l.visible)))), "every visible slot holds a row");
}

#[test]
fn the_layout_fits_small_and_large_windows() {
    // The default size, the minimum size, 200% scaling, a 4K window, a long list scrolled to its end.
    assert_fits(640.0, 760.0, 1.0, 120, 0);
    assert_fits(420.0, 360.0, 1.0, 120, 0);
    assert_fits(1280.0, 1520.0, 2.0, 120, 0);
    assert_fits(3840.0, 2160.0, 1.5, 120, 200);
    assert_fits(640.0, 760.0, 1.0, 3, 0);
    // A 760-px window at 100% shows at least 20 rows (the in-game overlay showed 7).
    assert!(devwin::layout(640.0, 760.0, 1.0, 120, 0).visible >= 20);
    // Sister: the check can fail — a rectangle wider than the window is not inside it.
    assert!(!Rect { x: 0.0, y: 0.0, w: 650.0, h: 10.0 }.inside(640.0, 760.0));
}

#[test]
fn scrolling_keeps_the_selection_on_screen_and_text_wraps() {
    assert_eq!(devwin::scroll_to(0, 25, 20), 6);
    assert_eq!(devwin::scroll_to(10, 3, 20), 3);
    assert_eq!(devwin::scroll_to(10, 15, 20), 10);
    // Wrap by a fake width (one unit per char): no line wider than the limit, nothing dropped.
    let s = "the game's own loadout for this level (Player_InitWeapon 0x1a7ef8)";
    let lines = devwin::wrap(s, 24.0, |t| t.len() as f32);
    assert!(lines.iter().all(|l| l.len() <= 24), "{lines:?}");
    assert_eq!(lines.join(" "), s);
    // Sister: a limit wider than the text keeps one line.
    assert_eq!(devwin::wrap(s, 999.0, |t| t.len() as f32).len(), 1);
}

const SIZE: (f32, f32) = (640.0, 760.0);

/// Feed inputs through the window's handler, re-reading the page's rows each time (what the App does).
fn feed(d: &mut dev::Dev, ui: &mut devwin::Ui, scene: &nf_view::scene::Scene, inputs: &[Input]) -> Request {
    let mut last = Request::None;
    for i in inputs {
        let rows = dev::menu_items(d, Some(scene));
        last = devwin::input(i, d, ui, &rows, SIZE, 1.0);
    }
    last
}

fn typed(s: &str) -> Vec<Input> { s.chars().map(|c| Input::Key(KeyCode::KeyA, Some(c.to_string()))).collect() }

/// The weapons tab: type "frag", Enter — the action is the frag grenade's, and the shared router equips it. The game
/// keeps running throughout: the window never sets `Dev::menu` (the F2 overlay's pause).
#[test]
fn typing_frag_and_enter_gives_the_frag_grenade_without_pausing_the_game() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let mut d = dev::Dev::default();
    let mut ui = devwin::Ui::default();
    // The window opens on the Weapons tab (the tabs are the index).
    feed(&mut d, &mut ui, &scene, &[Input::Move(1.0, 1.0)]);
    assert_eq!(dev::PAGES[d.page], "weapons");
    assert!(!scene.sim.as_ref().unwrap().drones.inventory.owned.get(0x34).copied().unwrap_or(false), "sister: 0x34 not owned before");
    let mut inputs = typed("frag");
    inputs.push(Input::Key(KeyCode::Enter, None));
    let req = feed(&mut d, &mut ui, &scene, &inputs);
    let Request::Apply(action) = req else { panic!("Enter on the filtered row gives an action, got {req:?} (filter {:?})", d.filter) };
    assert!(matches!(action, dev::MenuAction::GiveWeapon { id: 0x34, .. }), "the frag grenade row: {action:?}");
    let sim = scene.sim.as_ref().unwrap();
    let (eye, yaw) = (sim.bond.head(), sim.bond.yaw);
    let out = dev::apply_menu_action_to_scene(&mut d, &mut scene, action, eye, yaw, false);
    assert_eq!(out.switch_to, Some(0x34));
    assert!(scene.sim.as_ref().unwrap().drones.inventory.owned.get(0x34).copied().unwrap_or(false), "the inventory has 0x34");
    assert!(!d.menu, "the dev window never pauses the game (Dev::menu stays off)");
    // Esc clears the filter; Backspace on an empty filter does nothing harmful.
    feed(&mut d, &mut ui, &scene, &[Input::Key(KeyCode::Escape, None), Input::Key(KeyCode::Backspace, None)]);
    assert!(d.filter.is_empty());
}

/// Clicking a tab switches page; a single click selects a row, a double click applies it.
#[test]
fn clicks_switch_tabs_and_a_double_click_applies_the_row() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let mut d = dev::Dev::default();
    let mut ui = devwin::Ui::default();
    feed(&mut d, &mut ui, &scene, &[Input::Move(1.0, 1.0)]);
    let n = dev::menu_items(&d, Some(&scene)).len();
    let lay = devwin::layout(SIZE.0, SIZE.1, 1.0, n, 0);
    // The Teleport tab.
    let t = devwin::TAB_LABELS.iter().position(|l| *l == "Teleport").unwrap();
    let r = lay.tabs[t];
    feed(&mut d, &mut ui, &scene, &[Input::Move(r.x + 5.0, r.y + 5.0), Input::Click]);
    assert_eq!(dev::PAGES[d.page], "teleport");
    // Row 2: one click selects only.
    let n = dev::menu_items(&d, Some(&scene)).len();
    let lay = devwin::layout(SIZE.0, SIZE.1, 1.0, n, 0);
    let (_, row) = lay.rows[2];
    let req = feed(&mut d, &mut ui, &scene, &[Input::Move(row.x + 5.0, row.y + 5.0), Input::Click]);
    assert!(matches!(req, Request::Redraw), "a single click only selects: {req:?}");
    assert_eq!(d.menu_sel, 2);
    // The second click applies it.
    let req = feed(&mut d, &mut ui, &scene, &[Input::Click]);
    let Request::Apply(a) = req else { panic!("a double click applies the row, got {req:?}") };
    let rows = dev::menu_items(&d, Some(&scene));
    assert!(matches!((a, rows[2].2), (dev::MenuAction::SaveSpot, dev::MenuAction::SaveSpot) | (dev::MenuAction::RestoreSpot, dev::MenuAction::RestoreSpot) | (dev::MenuAction::TeleportTo(_), dev::MenuAction::TeleportTo(_))), "the applied action is row 2's: {a:?} vs {:?}", rows[2].2);
    // Sister: Close hides the window instead of quitting.
    assert!(matches!(feed(&mut d, &mut ui, &scene, &[Input::Close]), Request::Hide));
}
