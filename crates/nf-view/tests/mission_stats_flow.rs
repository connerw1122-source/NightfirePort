//! The Mission Stats page's flow — the results page's side action opens 0x40000037, △ returns
//! (docs/systems/mission-stats.md). Data + state only (no pixels asserted).

use nf_view::menu::{menu_overlay, Action, FrontEnd, Outcome, Page, PAGE_RESULTS, PAGE_STATS};

fn crate_texture(hash: u32) -> nf_view::scene::TextureImage {
    let (w, h) = if hash == 0x0300_0075 { (256, 128) } else if hash == 0x0300_0005 { (512, 128) } else { (8, 8) };
    nf_view::scene::TextureImage { key: (0, hash), width: w as u32, height: h as u32, rgba: vec![255; (w * h * 4) as usize], more: vec![], rate: 1 }
}

fn test_font() -> nf_formats::hud::Font {
    let glyph = |code: u16| nf_formats::hud::Glyph { u: 0, v: 0, w: 8, h: 12, yoff: 0, pre: 0, post: 8, code, kerns: vec![] };
    nf_formats::hud::Font { first: 33, last: 126, gap: 0.0, space: 3.0, glyphs: (33..=126).map(glyph).collect() }
}

/// Results (Complete) + the side action → Mission Stats; `stats_enter` fills the rows; △
/// returns to the results page. [M P_NFRESULTS 0x208e78 case 0x5e → Manager 0x44 0x40000037]
#[test]
fn the_side_action_opens_stats_and_back_returns() {
    let font = test_font();
    let hfont = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> =
        nf_view::menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    let mut fe = FrontEnd::new();
    fe.outcome = Outcome::Complete;
    fe.page = Page::Results;
    fe.flow.page = PAGE_RESULTS;

    // The side action opens the stats page and the flow word follows [M 0x208e78:224-229].
    assert!(fe.action(Action::Side), "the side action opens Mission Stats");
    assert_eq!(fe.page, Page::Stats);
    assert_eq!(fe.flow.page, PAGE_STATS);
    assert_eq!(fe.page_id(), PAGE_STATS);

    // The page idles every other key — P_NFSTATS arms only the 0x4c build [M 0x209938:31-124].
    assert!(!fe.action(Action::Confirm));
    assert!(!fe.action(Action::Up));
    assert!(!fe.action(Action::Side));

    // △ returns to the opener [S — the menu-def script target; live check in the report].
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::Results);
    assert_eq!(fe.flow.page, PAGE_RESULTS);

    // The page draws (the rows list may be empty before stats_enter — the draw must not panic).
    let frame = menu_overlay(&fe, &font, &hfont, None, &textures, 4.0 / 3.0);
    let _ = frame;
}

/// `stats_enter` against a live `Progress`: the rows land on the front end and the page draw
/// carries them (numbers/ids only — the pixel compare is the main agent's).
#[test]
fn stats_enter_fills_the_rows_and_the_draw_takes_them() {
    let font = test_font();
    let hfont = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> =
        nf_view::menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    let mut fe = FrontEnd::new();
    fe.outcome = Outcome::Complete;
    fe.page = Page::Stats;
    fe.flow.page = PAGE_STATS;

    let mut progress = nf_game::progress::Progress::default();
    progress.reset_for_mission();
    for _ in 0..10 {
        progress.log_shot_fired(1);
        progress.log_shot_hit_enemy(1);
    }
    progress.log_health(1, 100.0);
    progress.log_update_elapsed(7900);
    assert!(fe.stats_enter(&mut progress, 0x0700_0005, 6, 1, None), "07000005 builds the rows");
    let rows = fe.mission_stats.as_ref().unwrap();
    assert_eq!(rows.len(), 14); // 9 data rows + the Opponents group + the spacer + the 3 tail rows
    let total = rows.iter().find(|r| r.id == 0x163).unwrap();
    assert_eq!(total.total, "295,887"); // the capture's TOTAL

    let frame = menu_overlay(&fe, &font, &hfont, None, &textures, 4.0 / 3.0);
    let _ = frame;
}

/// The results page's back goes to the Dossier [M 0x208e78 case 0x5d → 0x4000002b] — the arm
/// this task added next to the side action.
#[test]
fn the_results_back_opens_the_dossier() {
    let mut fe = FrontEnd::new();
    fe.outcome = Outcome::Complete;
    fe.page = Page::Results;
    fe.flow.page = PAGE_RESULTS;
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::Dossier);
    assert_eq!(fe.cursor, 0);
}
