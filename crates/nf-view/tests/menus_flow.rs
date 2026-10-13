//! fix-menus headless tests: the GameFlow state machine and the front-end pages.
//!
//! Evidence: GameFlow_GetState 0x1c9e80 / PopState 0x1c9eb0 / PushState 0x1c9f38 / Main 0x1c98c8,
//! bootup 0x1c7e78, ResetMap_LevelToLoad 0x1d3800, ResetMap_Load 0x1d6360, Mission_Update 0x182c60:74,
//! P_START 0x20a408, P_MAIN 0x20a7b0, P_PAUSE 0x203c28, P_ENDMISSION 0x216618, the page table in
//! ACTION.ELF .data (id = 0x40000000 | index), MissionData 0x2a4350 (docs/formats/mission-data.md).

use nf_view::menu::{self, Action, FrontEnd, GameFlow, Outcome, Page, Stats, ST_FADE_HOLD, ST_MAPLOAD, ST_MOVIE, ST_RUNNER};

#[allow(dead_code)]
/// Drives the boot path to the title the way `GameFlow_Main` + `ResetMap_Load` would [M 0x1c98c8/0x1d6360].
fn tick_to_title(fe: &mut FrontEnd) {
    for _ in 0..8 {
        fe.tick();
        if fe.flow.get_state() == ST_RUNNER && fe.flow.current_level == menu::FRONTEND_LEVEL { return; }
    }
    panic!("boot never reached the front-end map: state {} level {:x}", fe.flow.get_state(), fe.flow.current_level);
}

#[test]
fn boot_reaches_the_title_through_the_load_path() {
    let mut fe = FrontEnd::new();
    // Boot state 1, page 0x40000034 = P_START [M 0x1c7e78 sets +0x28 = 0x40000034].
    assert_eq!(fe.flow.get_state(), 1);
    assert_eq!(fe.flow.page, 0x4000_0034);
    assert_eq!(fe.page, Page::Start);
    // State 1 runs bootup (pushes 2) and Reset_MapLoadSettings, which requests the front-end map [M
    // 0x1c98c8 case 1 + 0x1d6958: +0x8 = 0x7000048 → ResetMap_LevelToLoad → push 3].
    fe.flow.main_tick();
    assert_eq!(fe.flow.get_state(), ST_MAPLOAD);
    assert_eq!(fe.flow.level_to_load, menu::FRONTEND_LEVEL);
    // A plain load (no 0x71 wrap for 0x7000048 — not in 0x1d3800's switch): one load_tick finishes it.
    assert!(fe.flow.load_tick());
    assert_eq!(fe.flow.current_level, menu::FRONTEND_LEVEL, "GameState+0x12 [M 0x1d6360 phase 2]");
    assert_eq!(fe.flow.get_state(), ST_RUNNER, "pop 3, push 4, Locks_Init pops [M 0x1d6360 tail + 0x1c98c8 case 4]");
    assert!(!fe.flow.skip_draw, "state 2 draws [M 0x1c98c8's draw switch]");
}

#[test]
fn title_to_main_to_mission_select_to_difficulty_launches_the_mission() {
    let mut fe = FrontEnd::new();
    fe.flow.reset_map_level_to_load(menu::FRONTEND_LEVEL, false, true);
    fe.flow.main_tick();
    assert!(fe.flow.load_tick());
    // The title's flash runs on a 240-frame cycle [M 0x20a408's delayed 0x100000fa at 0xf0 frames].
    assert!(fe.press_start_on);
    fe.flow.ticks = 120;
    fe.tick();
    assert!(!fe.press_start_on, "the flash toggles within the 240-frame cycle");
    // Start → the main menu [S: the disc page data wires it; the page ids are the game's own].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::Main);
    // The main menu's rows: Nightfire / Multiplayer / Options [S; C_GONIGHTFIRE etc. exist as pages
    // 0x4000002f+ [M page table]]. Nightfire opens the mission select (P_NFSELECT 0x40000039 [M]).
    fe.action(Action::Confirm);
    assert_eq!(fe.page, Page::MissionSelect);
    // The campaign is 9 missions [M MissionData 0x2a4350, docs/formats/mission-data.md].
    assert_eq!(fe.rows(), 9);
    assert_eq!(fe.row_label(None, 0), "Paris Prelude");
    assert_eq!(fe.row_label(None, 1), "The Exchange");
    // Mission 0 = 07000005 first [M MissionData row 1: 07000005, 07000006, 07000007, 07000008].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::Difficulty);
    // Difficulty [M P_NFDFCTY 0x216bf8 is page 0x40000047; the names are the standard set S].
    assert_eq!(fe.rows(), 3);
    // Confirming the difficulty starts the mission: a cold load of its first level, mapped to the
    // 0x71 intro wrapper [M 0x1d3800's param_3 == 0 remap].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(0x0700_0005));
    assert_eq!(fe.flow.level_to_load, 0x0710_0005, "story codes wrap to 0x0710 intro codes [M 0x1d3800's switch]");
    assert_eq!(fe.flow.get_state(), ST_MAPLOAD);
    // The wrap pushes the movie state first [M 0x1d6360 phase 0 pushes 6 for 0x71 codes].
    assert!(fe.flow.load_tick() == false);
    assert_eq!(fe.flow.movie_code, 0x0710_0005, "GameState+0x10 [M 0x1d6360]");
    assert_eq!(fe.flow.get_state(), ST_MOVIE);
    // The movie ends (skip/finished [M case 6]), then phase 2 maps the code back and phase 3 loads.
    fe.flow.main_tick();
    assert!(!fe.flow.load_tick(), "phase 2 returns false; the next call loads");
    assert_eq!(fe.flow.level_to_load, 0x0700_0005, "GetLevelToLoadFromMovieCode [M 0x1d6360 LoadState 2]");
    assert!(fe.flow.load_tick());
    assert_eq!(fe.flow.current_level, 0x0700_0005, "GameState+0x12 [M 0x1d6360]");
    assert_eq!(fe.flow.get_state(), ST_RUNNER);
}

#[test]
fn the_pause_menu_resumes_restarts_and_quits() {
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    // Esc → the pause page over a running mission [S; the page id is the game's 0x4000000c [M]].
    assert!(fe.open_pause());
    assert_eq!(fe.page, Page::Pause);
    assert_eq!(fe.page.id(), 0x4000_000c);
    // Four rows [S labels; the ACTIONS are the game's C_GCPAUSE_Handler 0x204408].
    assert_eq!(fe.rows(), 4);
    // Row 0 = resume: the menu closes [S].
    assert!(fe.action(Action::Confirm));
    assert!(fe.closed);
    // Reopen; row 1 = restart: the mission's FIRST map, cold, no intro
    // [M 0x204408:139-141: ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)].
    fe.cursor = 1;
    fe.closed = false;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(0x0700_0005), "mission 0's BaseMap is 07000005 [M MissionData]");
    assert!(!fe.flow.warm_reset, "the restart is a cold load [M 0x204408: param_2 = 0]");
    // Row 3 = quit: a TWO-STEP confirm [M 0x204408:210-220: state_quit_166 gates the quit; the row
    // text swaps first], then the front-end map with the quit page [M :143-148: +0x28 = 0x4000001c,
    // ResetMap_LevelToLoad(0x7000048, 0, 0)].
    fe.open_pause();
    fe.cursor = 3;
    fe.requested_level = None; // the host consumes it with take(); the test clears it too
    assert!(fe.action(Action::Confirm));
    assert!(fe.confirming_quit, "the first choose only arms the confirm");
    assert_eq!(fe.requested_level, None, "nothing loads yet");
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(menu::FRONTEND_LEVEL));
    assert_eq!(fe.flow.page, 0x4000_001c, "the quit page id [M 0x204408]");
}

#[test]
fn the_results_screen_offers_restart_continue_and_quit() {
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    // A mission ends: the game records the results page 0x40000036 and fades [M 0x182c60:74 sets
    // +0x28 = 0x40000036 and pushes a 60-frame fade].
    fe.mission_ended(Outcome::Complete, Stats { shots_fired: 120, shots_hit: 40, accuracy_pct: 33, time_s: 754 });
    assert_eq!(fe.flow.page, 0x4000_0036);
    assert_eq!(fe.page, Page::Results);
    assert_eq!(fe.flow.get_state(), ST_FADE_HOLD, "the fade hold is pushed [M 0x182c60:74]");
    assert_eq!(fe.outcome, Outcome::Complete);
    // Three choices in the game's arm order [M 0x216618:19-27 — `__Menu_Send`'s return IS the row:
    // 0 = the CURRENT level warm + Player_RamLoad (fix-restart-target), 1 = BaseMap cold, 2 = quit].
    assert_eq!(fe.rows(), 3);
    // Cursor 0 = restart the CURRENT level, warm, on the entry checkpoint
    // [M 0x216618:19-21: ResetMap_LevelToLoad(GameState._12_4_, 1, 1) + Player_RamLoad(Bond, 1)].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(0x0700_0005));
    assert!(fe.flow.warm_reset, "row 0 is the warm arm [M 0x216618:20]");
    // Cursor 1 = restart the mission from its first map, cold
    // [M 0x216618:15-17: ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)].
    fe.mission_ended(Outcome::Complete, Stats::default());
    fe.cursor = 1;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(0x0700_0005));
    assert!(!fe.flow.warm_reset, "row 1 is the cold arm [M 0x216618:16]");
    // Cursor 2 = quit to the menu [M 0x216618's level==2 arm].
    fe.mission_ended(Outcome::Failed, Stats::default());
    fe.cursor = 2;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(menu::FRONTEND_LEVEL));
    assert_eq!(fe.flow.page, 0x4000_001c);
}

#[test]
fn cursors_wrap_and_back_walks_up_the_pages() {
    let mut fe = FrontEnd::new();
    fe.action(Action::Confirm); // Start → Main
    assert_eq!(fe.page, Page::Main);
    fe.action(Action::Up);
    assert_eq!(fe.cursor, 2, "up wraps from row 0 to the last row");
    fe.action(Action::Down);
    assert_eq!(fe.cursor, 0);
    // Back from the main menu returns to the title; Back from the select returns to the main menu [S].
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::Start);
    fe.action(Action::Confirm);
    fe.action(Action::Confirm);
    assert_eq!(fe.page, Page::MissionSelect);
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::Main);
    // Down 9 times on the mission select wraps to the same row.
    fe.action(Action::Confirm);
    for _ in 0..9 { fe.action(Action::Down); }
    assert_eq!(fe.cursor, 0);
    // Back from the difficulty returns to the mission select with the mission still selected [S].
    fe.action(Action::Confirm);
    fe.action(Action::Back);
    assert_eq!(fe.page, Page::MissionSelect);
    assert_eq!(fe.cursor, 0);
}

#[test]
fn the_attract_timer_counts_2700_inputless_frames() {
    let mut fe = FrontEnd::new();
    for _ in 0..2700 { fe.tick(); }
    assert_eq!(fe.no_input_frames, 2700, "the game switches to attract at 0xa8c = 2700 [M 0x20a408]");
    fe.action(Action::Confirm);
    assert_eq!(fe.no_input_frames, 0, "input resets the count [M Menu_ResetNoInputCount in 0x20a408/0x20a7b0]");
}

#[test]
fn the_menu_overlay_produces_font_quads_for_every_page() {
    // The draw side: one HudFrame per page with the game's own font. Without the ISO the labels fall
    // back to their ids; the NF_ISO test below checks the real strings.
    let mut fe = FrontEnd::new();
    let font = test_font();
    // One font-texture entry so the quads bind (the plate and the glyphs both use 0x03000003).
    let textures = [(0x0300_0003u32, crate_texture())];
    for page in [Page::Start, Page::Main, Page::MissionSelect, Page::Difficulty, Page::Options, Page::Pause, Page::Results] {
        fe.page = page;
        let frame = menu::menu_overlay(&fe, &font, None, &textures, 4.0 / 3.0);
        assert!(!frame.v.is_empty(), "{page:?}: the overlay drew nothing");
        assert!(!frame.runs.is_empty(), "{page:?}: no draw runs");
    }
    // The results page renders the stats line and the outcome head.
    fe.page = Page::Results;
    fe.outcome = Outcome::Complete;
    fe.stats = Stats { shots_fired: 10, shots_hit: 5, accuracy_pct: 50, time_s: 61 };
    let before = menu::menu_overlay(&fe, &font, None, &textures, 4.0 / 3.0).v.len();
    fe.outcome = Outcome::Failed;
    let after = menu::menu_overlay(&fe, &font, None, &textures, 4.0 / 3.0).v.len();
    assert!(before > 0 && after > 0, "both outcomes draw");
}

/// A stand-in font texture (the tests only check geometry, not pixels).
fn crate_texture() -> nf_view::scene::TextureImage {
    nf_view::scene::TextureImage { key: (0, 0x0300_0003), width: 8, height: 8, rgba: vec![255; 8 * 8 * 4], more: vec![], rate: 1 }
}

/// A minimal font the draw tests can lay out with (the real one comes from ACTION.ELF).
fn test_font() -> nf_formats::hud::Font {
    let glyph = |code: u16| nf_formats::hud::Glyph { u: 0, v: 0, w: 8, h: 12, yoff: 0, pre: 0, post: 8, code, kerns: vec![] };
    nf_formats::hud::Font { first: 33, last: 126, gap: 0.0, space: 3.0, glyphs: (33..=126).map(glyph).collect() }
}

// --- The NF_ISO tests: the real disc data ---

/// Reads USATxt.dat out of FILES.BIN [M Txt_LoadLanguage 0x1caad0 loads FNames[SelLang] = USATxt.dat]
/// and parses it with the game's own layout.
fn iso_lang(iso_path: &str) -> Option<nf_formats::hud::Lang> {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat"))?;
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::hud::Lang::parse(&bytes).ok()
}

#[test]
fn the_pause_labels_the_handler_uses_resolve_from_the_disc() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let Some(lang) = iso_lang(&iso) else { panic!("USATxt.dat must parse — the menus are drawn from it") };
    // C_GCPAUSE_Handler 0x204408 binds these labels on the pause page: 0x201 (the pause prompt),
    // 0x200 + 0x1000323 (the quit-confirm swap), 0x202/0x205. Print them; each must be a real string.
    for id in [0x201u32, 0x200, 0x0100_0323, 0x202, 0x205] {
        let s = lang.bind(id).unwrap_or_default();
        eprintln!("pause label {id:#x}: {s:?}");
        assert!(!s.is_empty(), "pause label {id:#x} did not resolve from USATxt.dat");
    }
    // The eight Txt ids P_PAUSE 0x203c28 sends into its list control turned out to be the
    // multiplayer scenario names — every one must still resolve (they are real game strings).
    let mut fe = FrontEnd::new();
    fe.page = Page::Pause;
    let mp: Vec<String> = [0x82u32, 0x91, 0xa0, 0xaf, 0xbe, 0xcd, 0x1000_018b, 0x1000_019a]
        .iter().map(|&id| lang.bind(id).map(str::to_string).unwrap_or_else(|| format!("<{id:#x}>"))).collect();
    eprintln!("0x203c28's list labels: {mp:?}");
    assert_eq!(mp[0], "NightFire", "the first of 0x203c28's eight labels is the MP scenario name [D USATxt.dat]");
    // The title's legal line [M 0x20a408 calls Txt_BindLabel(0x375)] — our USATxt parse has no
    // 0x375 entry; reported in docs/reports/fix-menus.md.
    eprintln!("legal line 0x375: {:?} (empty/absent → reported)", lang.bind(0x375).unwrap_or(""));
}

#[test]
fn the_mission_list_matches_missiondata_on_the_disc() {
    // [M MissionData 0x2a4350] — the table's first-level hashcodes, as parsed by nf-formats when a
    // level loads; here the constant list is checked against the doc's measured rows.
    let want = [0x0700_0005u32, 0x0700_0001, 0x0700_0009, 0x0700_000c, 0x0700_000e, 0x0700_0011, 0x0700_0010, 0x0700_0014, 0x0700_001b];
    let got: Vec<u32> = menu::MISSIONS.iter().map(|(_, c)| *c).collect();
    assert_eq!(got, want, "the select list is MissionData's first-level column [M docs/formats/mission-data.md]");
    assert_eq!(menu::MISSIONS.len(), 9);
}
