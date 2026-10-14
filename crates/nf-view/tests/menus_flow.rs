//! fix-menus + fix-menus2 headless tests: the GameFlow state machine, the front-end pages, and the
//! game's own strings/sprites.
//!
//! Evidence: GameFlow_GetState 0x1c9e80 / PopState 0x1c9eb0 / PushState 0x1c9f38 / Main 0x1c98c8,
//! bootup 0x1c7e78, ResetMap_LevelToLoad 0x1d3800, ResetMap_Load 0x1d6360, Mission_Update 0x182c60:74,
//! P_START 0x20a408, C_SBNFMAP 0x20b8d8, C_SBNFDFCTY 0x216ca8, the page table in ACTION.ELF .data,
//! the menu-def (kind-8 entry 08000002 in 07000048.bin) and `sp_level` 0x2df2e0 (fix-menus2).

use nf_view::menu::{self, Action, FrontEnd, Outcome, Page, Stats, ST_FADE_HOLD, ST_MAPLOAD, ST_MOVIE, ST_RUNNER};
use nf_view::gpu;

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

/// fix-menus2: the driver's observed page order — Title → (movie) → Main → Codename → Difficulty →
/// Single Player — with the game's own page ids [L PCSX2 2026-10-13; the ids from the menu-def].
#[test]
fn the_page_sequence_matches_the_real_game() {
    let mut fe = FrontEnd::new();
    tick_to_title(&mut fe);
    assert_eq!(fe.page.id(), 0x4000_0034, "the boot page is P_START [M 0x1c7e78]");
    // Start → the intro movie, then the main menu [M P_INTRO 0x209e88 plays the background FMVs then
    // opens the next page; GameFlow_Main case 6 pops on the skip action].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.flow.get_state(), ST_MOVIE, "the gun-barrel intro is the game's movie state");
    fe.flow.main_tick();
    assert_eq!(fe.page, Page::Main);
    assert_eq!(fe.page.id(), 0x4000_0002, "the main menu is menu-def page 0x4000002e→0x40000002's data: rows NightFire/Multiplayer/Codenames [M]");
    // The movie pops when it finishes (skippable [M case 6, Input_Action(-1, 0x19)]).
    let skip_ticks = fe.flow.movie_frames;
    for _ in 0..skip_ticks { fe.flow.main_tick(); }
    assert_eq!(fe.flow.get_state(), ST_RUNNER, "the movie state popped after its frames");
    // NightFire → Select Codename [M C_GONIGHTFIRE 0x2169b8; the driver's observed order].
    fe.action(Action::Confirm); // cursor 0 = NightFire
    assert_eq!(fe.page, Page::Codename);
    assert_eq!(fe.page.id(), 0x4000_0025, "Select Codename [M menu-def page 0x40000025 header Txt 0x1f2]");
    // "Play without saving" → the game's confirm box pops, ✕ continues → Select Difficulty
    // [M the page's plate control 0x100001f2; the driver's PCSX2 transcript]. Row 0 is now
    // "New Codename" [M Menu_EnumSaves 0x1f7b78:58-66]; Down reaches row 1.
    assert!(fe.action(Action::Down), "Down moves to Play without saving");
    assert!(fe.action(Action::Confirm));
    assert!(fe.confirming_default_profile, "the first confirm arms the box");
    assert!(fe.action(Action::Back), "△ cancels the box");
    assert!(!fe.confirming_default_profile);
    assert!(fe.action(Action::Confirm));
    assert!(fe.confirming_default_profile);
    assert!(fe.action(Action::Confirm), "✕ continues through the box");
    assert_eq!(fe.page, Page::Difficulty);
    assert_eq!(fe.page.id(), 0x4000_0023, "Select Difficulty [M menu-def page 0x40000023 header Txt 0x1f8]");
    // Confirming the difficulty → the Single Player list [M C_SBNFDFCTY 0x216ca8:10-17 opens
    // 0x4000001c; the driver's observed order Select Difficulty → Single Player].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MissionSelect);
    assert_eq!(fe.page.id(), 0x4000_001c, "Single Player [M menu-def page 0x4000001c header Txt 0x14c]");
    // The campaign is sp_level's 12 rows [M 0x2df2e0, 12 × 0x18-byte records].
    assert_eq!(fe.rows(), 12);
    // Confirming a mission requests the level the game does [M C_SBNFMAP 0x20b8d8:21-24
    // ResetMap_LevelToLoad(sp_level[i]+0xc, 0, 0); the row's level is the record's 4th lane].
    fe.unlock_all = true;
    fe.cursor = 1;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(menu::CAMPAIGN[1].3));
    assert_eq!(fe.flow.level_to_load, 0x0710_0005, "story codes wrap to 0x0710 intro codes [M 0x1d3800's switch]");
    assert!(fe.flow.load_tick() == false);
    assert_eq!(fe.flow.movie_code, 0x0710_0005, "GameState+0x10 [M 0x1d6360]");
    fe.flow.main_tick();
    assert!(!fe.flow.load_tick(), "phase 2 returns false; the next call loads");
    assert_eq!(fe.flow.level_to_load, 0x0700_0005, "GetLevelToLoadFromMovieCode [M 0x1d6360 LoadState 2]");
    assert!(fe.flow.load_tick());
    assert_eq!(fe.flow.current_level, 0x0700_0005, "GameState+0x12 [M 0x1d6360]");
    assert_eq!(fe.flow.get_state(), ST_RUNNER);
}

/// fix-menus2: the unlock rule — locked rows are dimmed and can't be scrolled to [L the driver:
/// "locked missions are dimmed and can't be scrolled to"; the gate is C_SBNFMAP 0x20b8d8:18-20].
#[test]
fn locked_missions_cannot_be_scrolled_to_or_picked() {
    let mut fe = FrontEnd::new();
    tick_to_title(&mut fe);
    fe.action(Action::Confirm); // Start → the intro movie [M P_INTRO 0x209e88]
    fe.action(Action::Confirm); // Confirm = action 0x19: the skip [M GameFlow_Main 0x1c98c8 case 6]
    fe.action(Action::Confirm); // NightFire
    fe.action(Action::Down);    // row 1 = Play without saving [M 0x1f79c0:15-23]
    fe.action(Action::Confirm); // arm the confirm box
    fe.action(Action::Confirm); // through it → difficulty
    fe.action(Action::Confirm); // difficulty → missions
    assert_eq!(fe.page, Page::MissionSelect);
    // A fresh profile has rows 0 AND 1 unlocked [D sp_level's boot state in ACTION.ELF .data (file
    // offset 0x1d91e0): rows 0-1 hold +0x10 = 1, rows 2-11 hold 0].
    assert!(fe.unlocked_row(0), "Paris Prelude starts unlocked");
    assert!(fe.unlocked_row(1), "The Exchange starts unlocked too [D sp_level boot +0x10]");
    assert!(!fe.unlocked_row(2), "the rest start locked");
    // Down from row 0 walks to the next unlocked row (row 1 is unlocked at boot too [D sp_level +0x10]).
    assert!(fe.action(Action::Down));
    assert_eq!(fe.cursor, 1, "the cursor walks the unlocked rows");
    // Picking the locked row 3 does nothing.
    fe.cursor = 3;
    assert!(!fe.action(Action::Confirm), "C_SBNFMAP 0x20b8d8:18-20 returns on a locked row");
    assert_eq!(fe.requested_level, None);
    // Completing mission 1 unlocks mission 2 (row 1 was already unlocked at boot [D sp_level
    // +0x10], so row 2 is the one the unlock actually opens) [M Menu_SetLevelBonus 0x1ffa10:63-65].
    fe.cursor = 1;
    fe.mission = 1;
    fe.mission_ended(Outcome::Complete, Stats::default());
    assert!(fe.unlocked_row(2), "completing row i unlocks row i+1");
    // Back on the mission list, Down walks onto the newly unlocked row.
    fe.page = Page::MissionSelect;
    fe.cursor = 1;
    assert!(fe.action(Action::Down));
    assert_eq!(fe.cursor, 2, "the cursor reaches the newly unlocked row");
    // --unlock-all is the game's menu_unlock_everything [M 0x30d2a7 read at 0x20b8d8:18].
    fe.unlock_all = true;
    assert!(fe.unlocked_row(11));
}

#[test]
fn the_pause_menu_resumes_restarts_and_quits() {
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    fe.mission = 1; // The Exchange — its first map is 07000005 [M sp_level 0x2df2e0 record 1]
    // Esc → the pause page over a running mission [M Player_Update 0x193600:73 opens
    // MenuManager_Create(0x80000002, 0x4000004b, …) + GS_PauseGame(1); the old 0x4000000c id was
    // the NIS page (the handler table maps it to P_NIS_Handler [M Handler_HandleMessage 0x2029e0])].
    assert!(fe.open_pause());
    assert_eq!(fe.page, Page::Pause);
    assert_eq!(fe.page.id(), 0x4000_004b);
    // The MISSION tab's THREE rows [L page 0x4000004b's d1=6 list m0x11 labels 0x16a/0x16b/0x16c
    // = CONTINUE/RESTART/QUIT — the old fourth "Options" row was a guess; options are the game's
    // CONTROLS tab].
    assert_eq!(fe.rows(), 3);
    // Row 0 = continue: the menu closes [M C_GCPAUSE_Handler 0x204408's d1=6 case-0 arm →
    // MenuManager_Delete].
    assert!(fe.action(Action::Confirm));
    assert!(fe.closed);
    // Reopen; row 1 = restart → the game's RESTART CONFIRM first [M 0x204408's case 1 binds 0x1ff
    // "RESTART: ARE YOU SURE?" and shows the d1=0xd YES/NO list], YES (row 0) then loads the
    // mission's FIRST map, cold [M 0x204408:138-141: ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)].
    fe.open_pause();
    fe.cursor = 1;
    fe.closed = false;
    assert!(fe.action(Action::Confirm));
    assert!(fe.confirming_restart, "the first choose only arms the RESTART confirm");
    assert_eq!(fe.requested_level, None, "nothing loads yet");
    assert_eq!(fe.confirm_sel, 1, "the YES/NO list opens on NO [L page data m0x1d:1; M 0x204408:181]");
    assert!(fe.action(Action::Confirm), "choosing NO returns to the rows");
    assert!(!fe.confirming_restart && !fe.closed, "NO cancels the confirm");
    fe.action(Action::Confirm); // arm again
    fe.action(Action::Up);      // NO → YES
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(0x0700_0005), "mission 1's BaseMap is 07000005 [M sp_level record 1 +0xc]");
    assert!(!fe.flow.warm_reset, "the restart is a cold load [M 0x204408: param_2 = 0]");
    // Row 2 = quit: a TWO-STEP confirm [M 0x204408:210-220: state_quit_166 gates the quit; the row
    // text swaps to 0x200 "QUIT: ARE YOU SURE?"], then the front-end map with the quit page [M
    // :143-148: +0x28 = 0x4000001c, ResetMap_LevelToLoad(0x7000048, 0, 0)].
    fe.open_pause();
    fe.cursor = 2;
    fe.requested_level = None; // the host consumes it with take(); the test clears it too
    assert!(fe.action(Action::Confirm));
    assert!(fe.confirming_quit, "the first choose only arms the confirm");
    assert_eq!(fe.requested_level, None, "nothing loads yet");
    fe.action(Action::Up); // NO → YES
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.requested_level, Some(menu::FRONTEND_LEVEL));
    assert_eq!(fe.flow.page, 0x4000_001c, "the quit page id [M 0x204408]");
    // ~H cycles the tabs [L the page hint 0x201 "~A Select ~V Scroll ~H Next"].
    fe.open_pause();
    assert!(fe.action(Action::Side));
    assert_eq!(fe.pause_tab, 1);
}

#[test]
fn the_results_screen_offers_restart_continue_and_quit() {
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    fe.mission = 1;
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
    fe.unlock_all = true; // the walk visits the mission list freely
    fe.action(Action::Confirm); // Start → the intro movie [M P_INTRO 0x209e88]
    assert_eq!(fe.page, Page::Main);
    fe.action(Action::Confirm); // Confirm = action 0x19: the skip [M GameFlow_Main 0x1c98c8 case 6]
    fe.action(Action::Up);
    assert_eq!(fe.cursor, 2, "up wraps from row 0 to the last row");
    fe.action(Action::Down);
    assert_eq!(fe.cursor, 0);
    // Back from the main menu returns to the title; Back from the select returns up the chain [S].
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::Start);
    fe.action(Action::Confirm);
    fe.action(Action::Confirm); // the skip — the intro replays after Back to the title
    fe.action(Action::Confirm);
    assert_eq!(fe.page, Page::Codename);
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::Main);
    fe.action(Action::Confirm);
    fe.action(Action::Down);    // row 1 = Play without saving [M 0x1f79c0:15-23]
    fe.action(Action::Confirm); // the confirm box
    fe.action(Action::Confirm); // through it → difficulty
    assert_eq!(fe.page, Page::Difficulty);
    // The difficulty wheel wraps over its three rows; the box's ✕ lands the cursor on Agent.
    fe.action(Action::Up);
    assert_eq!(fe.cursor, 0, "up from Agent reaches Operative");
    fe.action(Action::Down);
    fe.action(Action::Down);
    assert_eq!(fe.cursor, 2, "down twice reaches 00 Agent");
    fe.action(Action::Confirm);
    assert_eq!(fe.page, Page::MissionSelect);
    // Down 12 times on the mission select wraps to the same row.
    for _ in 0..12 { fe.action(Action::Down); }
    assert_eq!(fe.cursor, 0);
    // The Dossier opens on □/E/Tab and returns [L the driver: hints "□ Dossier"].
    assert!(fe.action(Action::Side));
    assert_eq!(fe.page, Page::Dossier);
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::MissionSelect);
    // Back from the mission select returns to the difficulty [S].
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::Difficulty);
}

#[test]
fn the_attract_timer_counts_2700_inputless_frames() {
    let mut fe = FrontEnd::new();
    for _ in 0..2700 { fe.tick(); }
    assert_eq!(fe.no_input_frames, 2700, "the game switches to attract at 0xa8c = 2700 [M 0x20a408]");
    fe.action(Action::Confirm);
    assert_eq!(fe.no_input_frames, 0, "input resets the count [M Menu_ResetNoInputCount in 0x20a408/0x20a7b0]");
}

/// fix-menus2: every page draws with the game's own textures and layout. The sprites each page draws
/// are the menu-def's msg-0x24 texture hashes (numbers only, so the test stays byte-free); a hash that
/// isn't in the textures list draws nothing, so every listed hash must be present.
#[test]
fn the_menu_overlay_draws_the_game_pages_with_their_own_sprites() {
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    let mut fe = FrontEnd::new();
    fe.unlock_all = true;
    let mut saw: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for page in [Page::Start, Page::Main, Page::Codename, Page::Difficulty, Page::MissionSelect, Page::Dossier] {
        fe.page = page;
        let frame = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
        assert!(!frame.v.is_empty(), "{page:?}: the overlay drew nothing");
        assert!(!frame.runs.is_empty(), "{page:?}: no draw runs");
        // Which textures the page referenced (run.tex is hud_textures index + 1).
        let used: Vec<u32> = frame.runs.iter().filter(|r| r.0 > 0).map(|r| textures[r.0 - 1].0).collect();
        assert!(!used.is_empty(), "{page:?}: no textured runs");
        saw.insert(page.id(), used);
    }
    // Per-page sprite hashes (numbers only) — the pages' msg-0x24 sets [M the menu-def]:
    let has = |page: u32, tex: u32| saw[&page].iter().any(|&t| t == tex);
    assert!(has(0x4000_0034, 0x0300_0074), "the title draws the full-screen art [M page 0x40000009 control 0x10000196]");
    assert!(has(0x4000_0025, 0x0300_00b3), "the codename page draws the panel [M page 0x40000025 control (50,53) TEX 0x030000b3]");
    assert!(has(0x4000_0025, 0x0300_00bf), "the codename page draws the ring emblem [M control 0x10000103]");
    // The codename preview follows the cursor [M Menu_UpdateCodenameWheel 0x1f89f0:80-99 — the
    // create row 0x300013b, the default row 0x3000135; the page's static 0x030000b6 is the boot look].
    assert!(has(0x4000_0025, 0x0300_013b), "the codename page draws the create row's preview [M 0x1f89f0:80-99]");
    assert!(has(0x4000_0023, 0x0300_00a5), "the difficulty page draws its emblem [M control 0x100000be TEX 0x030000a5]");
    // The mission list's ring preview is PER ROW: row 0 (Paris Prelude) must draw Paris Prelude's own
    // preview 0x03000085 = sp_level[0]+0x00 [L ee_snapshot 0x2df2e0; M 0x1f8388's msg 0x24] — the port
    // drew The Exchange's 0x03000084 here (the constant 0x10000020 default), which is exactly the
    // symptom the driver reported.
    assert!(has(0x4000_001c, 0x0300_0085), "Paris Prelude's own ring preview [M sp_level[0]+0x00]");
    assert!(!has(0x4000_001c, 0x0300_0084), "row 0 must NOT show The Exchange's preview 0x03000084 (the reported symptom)");
    // The dossier's emblem is the wheel table's per-row preview [L 0x2dfcd0 row 0 = 0x300019b Records;
    // the page's static 0x030000c4 is the Gadgets row].]
    assert!(has(0x4000_002b, 0x0300_019b), "the dossier draws its Records row's preview [L table 0x2dfcd0]");
    assert!(has(0x4000_001c, 0x0300_016d), "the list pages draw the logo [M control (434,38,156×43) TEX 0x0300016d]");
    assert!(has(0x4000_0023, 0x0300_014c), "the list pages draw the stamp [M control 0x10000224 TEX 0x0300014c]");
    // The hint bars draw the game's own specialchar icon font [M specialchar 0x2dd8d8: every entry's
    // texture is 0x03000075] — not the 64×64 atlas 0x03000042 the port guessed before, and the ✕ is
    // the table's (235,1,19×19) tile, not the font's letter X.
    assert!(has(0x4000_001c, 0x0300_0075), "the hint bars draw the specialchar icon font [M 0x2dd8d8]");
    assert!(!has(0x4000_001c, 0x0300_0042), "the hint bars no longer use the guessed 0x03000042 atlas");
    // The title page is the only one with the full-screen art (the others use the dark plate).
    assert!(!has(0x4000_0002, 0x0300_0074));
}

/// The label ids each page binds, pinned against USATxt.dat on the disc [M every id is the menu-def's
/// msg-0x18 argument or `sp_level`'s name/description labels; the strings are `Txt_BindLabel`
/// 0x1cad60's own].
#[test]
fn the_menu_strings_come_from_the_language_bank() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let Some(lang) = iso_lang(&iso) else { panic!("USATxt.dat must parse — the menus are drawn from it") };
    let b = |id: u32| lang.bind(id).unwrap_or("");
    // The title [M 0x20a408: Txt_BindLabel(0x375) legal + 0x10001eb prompt; the menu-def's 0x1c9].
    assert_eq!(b(0x100_01eb), "Press the START button");
    assert!(b(0x375).starts_with("NIGHTFIRE Interactive Game"), "the legal line resolves [M 0x375]");
    // The main menu [M menu-def page 0x40000002 rows + the runtime description labels].
    assert_eq!(b(0x28), "NightFire");
    assert_eq!(b(0x149), "Multiplayer");
    assert_eq!(b(0x14b), "Codenames");
    assert_eq!(b(0x100_0000), "Start your 007 adventure.");
    assert_eq!(b(0x100_0001), "Test your skills against other agents.");
    assert_eq!(b(0x54e), "~V Scroll  ~A Select");
    // Select Codename [M page 0x40000025 header 0x1f2; the stub's row + its confirm box].
    assert_eq!(b(0x1f2), "Select Codename");
    assert_eq!(b(0x29b), "New Codename");
    assert_eq!(b(0x100_02ff), "Play without saving");
    assert!(b(0x100_00bb).starts_with("You won't be able to save your progress"), "the confirm-box text [L the driver's transcript]");
    assert_eq!(b(0x216), "~A Select  ~Y Create  ~V Scroll  ~B Back");
    // Select Difficulty [M page 0x40000023 header 0x1f8 + the wheel rows; descriptions 0x1000006/7/8].
    assert_eq!(b(0x1f8), "Select Difficulty");
    assert_eq!(b(0x14d), "Operative");
    assert_eq!(b(0x14e), "Agent");
    assert_eq!(b(0x14f), "00 Agent");
    assert!(b(0x100_0006).starts_with("For trainee Agents."));
    assert!(b(0x100_0007).starts_with("For skilled Agents."));
    assert!(b(0x100_0008).starts_with("For 00 Agents only."));
    // Single Player [M page 0x4000001c header 0x14c; the rows are sp_level's name labels; the
    // descriptions its description labels; the hints the game's 0x208 (□ Dossier) set].
    assert_eq!(b(0x14c), "Single Player");
    assert_eq!(b(0x0300_0001), "Paris Prelude");
    assert_eq!(b(0x0300_0002), "The Exchange");
    assert_eq!(b(0x0300_0003), "Alpine Escape");
    assert_eq!(b(0x0300_0004), "Enemies Vanquished", "the disc's own name — the port's old hard-coded list said 'Enemies Vanish'");
    assert!(b(0x0300_0014).starts_with("Drop in on Raphael Drake's reception"));
    assert_eq!(b(0x208), "~A Select  ~Y Dossier  ~V Scroll  ~B Back");
    // Every campaign row's name and description must resolve.
    for (preview, name, desc, _) in menu::CAMPAIGN {
        assert_eq!(preview >> 24, 0x03, "row's preview is a texture hash [M sp_level+0x00]");
        assert!(!b(name).is_empty(), "campaign name {name:#x} did not resolve");
        assert!(!b(desc).is_empty(), "campaign description {desc:#x} did not resolve");
    }
    // The locked rows' description resolves too [L sp_level+0x14 = 0x01000009; M 0x1f8388's gate].
    assert!(b(menu::LOCKED_DESC).starts_with("Continue playing"), "the locked-row description [L USATxt 0x01000009]");
    // The Dossier [M page 0x4000002b header 0x27f].
    assert_eq!(b(0x27f), "Dossier");
}

#[test]
fn the_menu_overlay_produces_font_quads_for_every_page() {
    // The draw side: one HudFrame per page with the game's own font. Without the ISO the labels fall
    // back to their ids; the NF_ISO test above checks the real strings.
    let mut fe = FrontEnd::new();
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).chain(std::iter::once((0x0300_0003u32, crate_texture(0x0300_0003)))).collect();
    for page in [Page::Start, Page::Main, Page::Codename, Page::Difficulty, Page::MissionSelect, Page::Dossier, Page::Options, Page::Pause, Page::Results] {
        fe.page = page;
        let frame = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
        assert!(!frame.v.is_empty(), "{page:?}: the overlay drew nothing");
        assert!(!frame.runs.is_empty(), "{page:?}: no draw runs");
    }
    // The results page renders the stats line and the outcome head.
    fe.page = Page::Results;
    fe.outcome = Outcome::Complete;
    fe.stats = Stats { shots_fired: 10, shots_hit: 5, accuracy_pct: 50, time_s: 61 };
    let before = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0).v.len();
    fe.outcome = Outcome::Failed;
    let after = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0).v.len();
    assert!(before > 0 && after > 0, "both outcomes draw");
}

/// A stand-in font texture (the tests only check geometry, not pixels). The specialchar font gets its
/// real 256-wide pitch so the icon uvs normalize to the table's own values.
fn crate_texture(hash: u32) -> nf_view::scene::TextureImage {
    // Real dims [L nf-extract tex-list 07000048.bin]: 0x03000075 256×128, the header face
    // 0x03000005 512×128 — the tests normalise the icon/header uvs by the real pitch.
    let (w, h) = if hash == 0x0300_0075 { (256, 128) } else if hash == 0x0300_0005 { (512, 128) } else { (8, 8) };
    nf_view::scene::TextureImage { key: (0, hash), width: w as u32, height: h as u32, rgba: vec![255; (w * h * 4) as usize], more: vec![], rate: 1 }
}

/// fix-pause-menu: the in-game pages draw from the mission menu def's own data — the sprite hashes
/// per page (numbers only) and the pause confirm box's extra quads.
#[test]
fn the_pause_page_draws_the_mission_menu_defs_data() {
    let font = test_font();
    // 0x03000003 = FONT_TEX [M hud.rs] — without it the text quads are dropped and the count pins see nothing.
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).chain(std::iter::once((0x0300_0003u32, crate_texture(0x0300_0003)))).collect();
    // The in-game pages' art hashes are in MENU_TEXTURES: the pause/fail backdrop plate 0x03000070 +
    // Bond image 0x030000db (every mission archive [L nf-extract tex-list 07000029/0700004b/…]) and
    // the complete-results photo 0x03000140 (the front-end 07000048.bin [L]).
    for hash in [0x0300_0070u32, 0x0300_00db, 0x0300_0140] {
        assert!(menu::MENU_TEXTURES.contains(&hash), "{hash:#x} must be in MENU_TEXTURES (the mission menu def's msg-0x24 art)");
    }
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    assert!(fe.open_pause());
    let base = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0).v.len();
    assert!(base > 0, "the pause page draws");
    // The RESTART confirm draws the YES/NO box over the rows [L page 0x4000004b's d1=0xc text +
    // d1=0xd list; M 0x204408 case 1].
    fe.cursor = 1;
    fe.action(Action::Confirm);
    let with_confirm = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0).v.len();
    assert!(with_confirm > base, "the confirm box adds quads");
    // Each pause tab draws [L the four tabs 0x167/0x176/0x168/0x169].
    for tab in 0..4 {
        fe.pause_tab = tab;
        fe.confirming_restart = false;
        let f = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
        assert!(!f.v.is_empty(), "pause tab {tab} draws");
    }
    // The fail menu (page 0x40000042) and the complete results (0x40000036) both draw.
    fe.page = Page::Results;
    fe.outcome = Outcome::Failed;
    assert!(!menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0).v.is_empty(), "the fail menu draws");
    fe.outcome = Outcome::Complete;
    assert!(!menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0).v.is_empty(), "the complete results draw");
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

/// The game's own menu font [M HudTables::from_action_elf reads FONTSerpLight/FontDataSerpLight —
/// the font the menu overlay draws with].
fn iso_font(iso_path: &str) -> nf_formats::hud::Font {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).expect("iso");
    let elf = iso.read_file(iso.find("/ACTION.ELF").expect("ACTION.ELF")).expect("read");
    nf_formats::hud::HudTables::from_action_elf(&elf).expect("HUD tables").font
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
    // fix-pause-menu: the pause PAGE DATA's own labels [L mission menu-def page 0x4000004b: the four
    // tab controls LBL 0x167/0x176/0x168/0x169, the d1=6 list's m0x11 rows 0x16a/0x16b/0x16c, the
    // confirm text 0x1ff/0x200 (d1=0xc) and YES/NO 0x5df/0x5e0 (d1=0xd), the hint 0x201].
    let pause_ids = [(0x167u32, "MISSION"), (0x176, "OBJECTIVES"), (0x168, "CONTROLS"), (0x169, "SCORE"),
                    (0x16a, "C O N T I N U E"), (0x16b, "R E S T A R T"), (0x16c, "Q U I T"),
                    (0x1ff, "RESTART: ARE YOU SURE?"), (0x5df, "Y E S"), (0x5e0, "N O")];
    for (id, want) in pause_ids {
        let s = lang.bind(id).unwrap_or_default();
        assert_eq!(s, want, "pause page label {id:#x} is the menu def's own string [L page 0x4000004b]");
    }
    // The fail/death menu 0x40000042's labels [L the same menu def: header 0x2fd, rows 0x5bc/0x16c,
    // hint 0x5c8] and the complete results 0x40000036's [L the front-end def: 0x2c5/0x2cb/0x2cd/0x2d0].
    let end_ids = [(0x2fdu32, "Mission Failed"), (0x5bc, "R E P L A Y"), (0x5c8, "~A Select  ~V Scroll"),
                   (0x2c5, "Results"), (0x2cb, "Total Score:"), (0x2cd, "Gold Medal Target:")];
    for (id, want) in end_ids {
        let s = lang.bind(id).unwrap_or_default();
        assert_eq!(s, want, "mission-end label {id:#x} [L pages 0x40000042/0x40000036]");
    }
    // The eight Txt ids P_PAUSE 0x203c28 sends into its list control turned out to be the
    // multiplayer scenario names — every one must still resolve (they are real game strings).
    let mut fe = FrontEnd::new();
    fe.page = Page::Pause;
    let mp: Vec<String> = [0x82u32, 0x91, 0xa0, 0xaf, 0xbe, 0xcd, 0x1000_018b, 0x1000_019a]
        .iter().map(|&id| lang.bind(id).map(str::to_string).unwrap_or_else(|| format!("<{id:#x}>"))).collect();
    eprintln!("0x203c28's list labels: {mp:?}");
    assert_eq!(mp[0], "NightFire", "the first of 0x203c28's eight labels is the MP scenario name [D USATxt.dat]");
}

/// fix-menus2: the campaign list is `sp_level` 0x2df2e0's 12 records — name/description labels (bank 3),
/// level codes and unlock flags — cross-checked against the EE snapshot when it is around.
#[test]
fn the_campaign_is_sp_level_on_the_disc() {
    assert_eq!(menu::CAMPAIGN.len(), 12, "sp_level holds 12 × 0x18-byte records [M 0x2df2e0, 288 bytes]");
    // The preview textures + level codes + name labels read from the EE snapshot (`work/ee_snapshot.bin`,
    // file offset = EE address) — a mid-game save has all 12 rows unlocked [L].
    let Ok(snap) = std::fs::read("../work/ee_snapshot.bin").or_else(|_| std::fs::read("../../work/ee_snapshot.bin")) else { eprintln!("no ee_snapshot.bin — skipping the [L] cross-check"); return };
    for (i, &(preview, name, _, level)) in menu::CAMPAIGN.iter().enumerate() {
        let rec = &snap[0x2df2e0 + i * 0x18..0x2df2e0 + (i + 1) * 0x18];
        let snap_preview = u32::from_le_bytes(rec[0x00..0x04].try_into().unwrap());
        let snap_name = u32::from_le_bytes(rec[0x04..0x08].try_into().unwrap());
        let snap_level = u32::from_le_bytes(rec[0x0c..0x10].try_into().unwrap());
        assert_eq!(snap_preview, preview, "sp_level[{i}] preview texture [L ee_snapshot 0x{:x}]", 0x2df2e0 + i * 0x18);
        assert_eq!(snap_name, name, "sp_level[{i}] name label [L ee_snapshot 0x{:x}]", 0x2df2e0 + i * 0x18 + 4);
        assert_eq!(snap_level, level, "sp_level[{i}] level [L ee_snapshot 0x{:x}]", 0x2df2e0 + i * 0x18 + 0xc);
        assert_eq!(rec[0x10], 1, "the snapshot's save has every row unlocked [L]");
    }
}

// --- fix-menus3: the 2026-10-13 render-vs-PCSX2 gaps ---

/// Framing: the real game draws the 640×480 menu space as a CENTRED 4:3 picture with black bars at
/// the sides [L the main agent's PCSX2 captures 2026-10-13]; the port stretched it across the 16:9
/// window (Anchor::Stretch quads). Every overlay vertex must stay inside the window's central 4:3
/// band (|x| ≤ (4/3)/aspect), and at a 4:3 window the full-screen plate still spans it edge to edge.
#[test]
fn the_menus_draw_inside_a_centred_4_3_band() {
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    let mut fe = FrontEnd::new();
    fe.unlock_all = true;
    for page in [Page::Start, Page::Main, Page::Codename, Page::Difficulty, Page::MissionSelect] {
        fe.page = page;
        let frame = menu::menu_overlay(&fe, &font, &font, None, &textures, 16.0 / 9.0);
        let band = (4.0 / 3.0) / (16.0 / 9.0); // 0.75
        let worst = frame.v.iter().fold(0.0f32, |m, vtx| m.max(vtx.p[0].abs()));
        assert!(worst <= band + 1e-3, "{page:?}: a quad vertex reaches NDC x {worst}, past the 4:3 band {band} (the 16:9 stretch symptom)");
    }
    // Sister (4:3 unchanged): the title art still spans the whole window at aspect 4/3.
    fe.page = Page::Start;
    let frame = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    let worst = frame.v.iter().fold(0.0f32, |m, vtx| m.max(vtx.p[0].abs()));
    assert!(worst >= 0.999, "at 4:3 the full-screen art reaches the window edges (worst x {worst})");
}

/// The ring preview is clipped to the ring's circle [L the main agent's render: a visible square box
/// around the emblem / mission preview]. Every preview vertex (640-space) must sit inside the
/// inscribed circle of the emblem rect (114,118,150×150); the ring quad itself may reach the box.
#[test]
fn the_ring_preview_is_clipped_to_the_ring_circle() {
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    let mut fe = FrontEnd::new();
    fe.unlock_all = true;
    fe.page = Page::MissionSelect;
    let frame = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    // The preview's texture index in the frame's runs (1-based into the textures slice).
    let preview_hash = menu::CAMPAIGN[0].0;
    let ti = textures.iter().position(|(h, _)| *h == preview_hash).unwrap() + 1;
    // 640-space back-mapping at aspect 4/3: x640 = (ndc+1)/2·512/0.8, y640 = (1−ndc)·224/0.9333.
    let to640 = |p: [f32; 2]| [(p[0] + 1.0) * 256.0 / 0.8, (1.0 - p[1]) * 224.0 / (448.0 / 480.0)];
    let (cx, cy, r) = (114.0 + 75.0, 118.0 + 75.0, 75.0);
    let mut preview_verts = 0;
    for run in &frame.runs {
        if run.0 != ti { continue; }
        for vtx in &frame.v[run.2 as usize..(run.2 + run.3) as usize] {
            let [x, y] = to640(vtx.p);
            let d2 = ((x - cx) / r).powi(2) + ((y - cy) / r).powi(2);
            assert!(d2 <= 1.0 + 1e-3, "preview vertex ({x:.1},{y:.1}) is outside the ring circle (d²={d2:.3}) — the square-box symptom");
            preview_verts += 1;
        }
    }
    // 22 of the 24 strips survive — the two outermost chords degenerate to zero height (they are
    // the circle's leftmost/rightmost points; the ring quad covers them).
    assert!(preview_verts >= 20 * 6, "the preview drew as the circle strips ({preview_verts} verts)");
}

/// The hint bar's ~A draws the specialchar table's ✕ tile [M specialchar 0x2dd8d8 entry 0: texture
/// 0x03000075, u 235, v 1, 19×19, advance 19] — not the font's letter X. Sister: the codename page's
/// hint has ~Y — the table's triangle tile (235,61).
#[test]
fn the_hint_icons_come_from_the_specialchar_table() {
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    // The codename page's hint 0x216 = "~A Select  ~Y Create  ~V Scroll  ~B Back" [L USATxt.dat].
    let mut fe = FrontEnd::new();
    fe.page = Page::Codename;
    let frame = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    let ti = textures.iter().position(|(h, _)| *h == 0x0300_0075).unwrap() + 1;
    let mut cross = false;
    let mut triangle = false;
    for run in &frame.runs {
        if run.0 != ti { continue; }
        for vtx in &frame.v[run.2 as usize..(run.2 + run.3) as usize] {
            // The uv rect is normalized by the test texture's real pitch (256×128).
            let (u, v) = (vtx.uv[0] * 256.0, vtx.uv[1] * 128.0);
            if (u - 235.0).abs() < 0.6 && (v - 1.0).abs() < 0.6 { cross = true; }
            if (u - 235.0).abs() < 0.6 && (v - 61.0).abs() < 0.6 { triangle = true; }
        }
    }
    assert!(cross, "the ✕ hint icon is specialchar's (235,1,19×19) tile [M 0x2dd8d8:0] — the letter-X symptom");
    assert!(triangle, "the △ hint icon is specialchar's (235,61) tile [M 0x2dd8d8:16]");
}

/// The title's legal block wraps inside its plate (50,332,540×108) and every line is centred at the
/// plate's centre x=320 [L the main agent's captures: the raw run went off the right edge]; and "©"
/// lays out as ONE glyph (the CP-1252 decode + a char walk — the byte walk drew C2+A9 = "Â©").
#[test]
fn the_legal_line_wraps_inside_its_plate_and_keeps_the_copyright() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let Some(lang) = iso_lang(&iso) else { panic!("USATxt.dat must parse") };
    let legal = lang.bind(0x375).unwrap_or_default();
    assert!(legal.contains('\u{a9}'), "the legal line holds © [M the CP-1252 0xA9 byte]");
    let font = iso_font(&iso);
    let lines = nf_view::menu::wrap_px_for_test(&legal, &font, 500.0);
    assert!(lines.len() >= 3, "the legal block wraps to several lines (got {})", lines.len());
    for line in &lines {
        let w = font.layout(line, 0.0, 0.0).1;
        assert!(w <= 500.0, "legal line {line:?} is {w}px wide, past the 540px plate");
    }
    // One glyph for ©: the char walk maps U+00A9 to the font's own 0xA9 glyph, never two glyphs.
    let (placed, _) = font.layout("\u{a9}", 0.0, 0.0);
    assert_eq!(placed.len(), 1, "© is one glyph (the byte walk drew C2 A9 = Â©)");
    assert_eq!(font.glyphs[placed[0].glyph].code, 0xa9);
}

/// The difficulty and mission descriptions wrap inside their 540px plates [L the captures: the
/// single-line descriptions ran off the right edge].
#[test]
fn the_descriptions_wrap_inside_their_plates() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let Some(lang) = iso_lang(&iso) else { panic!("USATxt.dat must parse") };
    let font = iso_font(&iso);
    let b = |id: u32| lang.bind(id).unwrap_or("");
    for id in menu::DIFFICULTY_DESCS.iter().copied().chain(menu::CAMPAIGN.iter().map(|c| c.2)) {
        for line in nf_view::menu::wrap_px_for_test(b(id), &font, 500.0) {
            let w = font.layout(&line, 0.0, 0.0).1;
            assert!(w <= 500.0, "description {id:#x} line {line:?} is {w}px wide, past the plate");
        }
    }
    // Sister: the wraps cover the whole string (no words dropped).
    let words = |s: &str| s.split_whitespace().count();
    let d = b(0x100_0007);
    let total: usize = nf_view::menu::wrap_px_for_test(d, &font, 500.0).iter().map(|l| words(l)).sum();
    assert_eq!(total, words(d), "wrapping keeps every word");
}


// ── fix-menus-real: the pages match the real game's data ────────────────────────────────────────
// Evidence: the menu-def tag stream parsed 2026-10-14 (MenuManager_Create 0x21b908's walk — the
// CONTROL records quoted per test) + Label_Update 0x218cb8:177-182/:202-206 (labels centre in their
// control; the default label format DAT_003096c0 = ff 02 fe 03 [M Label_Init 0x218638:53]).

/// The font-text quads of an overlay frame, clustered into text rows in the 512×448 HUD space:
/// (x0, x1, y0, y1) per row.
struct TextRows { rows: Vec<(f32, f32, f32, f32)> }

fn text_rows(frame: &nf_view::gpu::HudFrame, n_tex: usize, aspect: f32) -> TextRows {
    // The font texture and the specialchar icon texture both carry text-row ink; runs name them by
    // index+1 (the font was appended last; 0x03000075 is MENU_TEXTURES[10]).
    let font_tex = n_tex;
    let icon_tex = menu::MENU_TEXTURES.iter().position(|&t| t == 0x0300_0075).expect("icon tex listed") + 1;
    let head_tex = menu::MENU_TEXTURES.iter().position(|&t| t == 0x0300_0005).expect("header atlas listed") + 1;
    let mut is_text = vec![false; frame.v.len()];
    for &(tex, _, start, count) in &frame.runs {
        if tex == font_tex || tex == icon_tex || tex == head_tex { for v in &mut is_text[start as usize..start as usize + count as usize] { *v = true; } }
    }
    let k = (2.0 / 512.0) * (4.0 / 3.0) / aspect;
    // Per-quad bboxes (runs concatenate 6-vertex quads; the full-screen plates also ride the font
    // texture — Hud::ndc_quad pins 0x03000003 — so keep only quads of text size).
    let mut quads: Vec<(f32, f32, f32, f32)> = Vec::new();
    let mut vi = 0usize;
    while vi + 6 <= frame.v.len() {
        if is_text[vi] {
            let (x0, x1, y0, y1) = frame.v[vi..vi + 6].iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |acc, q| {
                let x = q.p[0] / k + 256.0; let y = (1.0 - q.p[1]) * 224.0;
                (acc.0.min(x), acc.1.max(x), acc.2.min(y), acc.3.max(y))
            });
            if x1 - x0 <= 130.0 && y1 - y0 <= 60.0 { quads.push((x0, x1, y0, y1)); }
        }
        vi += 6;
    }
    // Group quads into rows by vertical overlap (glyph tops and bottoms share the row's band).
    quads.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
    let mut rows: Vec<(f32, f32, f32, f32)> = Vec::new();
    for (x0, x1, y0, y1) in quads {
        match rows.iter_mut().find(|r| y0 <= r.3 && y1 >= r.2) {
            Some(r) => { r.0 = r.0.min(x0); r.1 = r.1.max(x1); r.2 = r.2.min(y0); r.3 = r.3.max(y1); }
            None => rows.push((x0, x1, y0, y1)),
        }
    }
    rows.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
    TextRows { rows }
}

impl TextRows {
    /// The row whose band covers `y`, as (x0, x1) — None when no text sits there.
    fn band(&self, y: f32) -> Option<(f32, f32)> {
        self.rows.iter().filter(|r| r.2 <= y && y <= r.3 + 0.5).map(|r| (r.0, r.1)).fold(None, |acc, (a, b)| {
            Some(match acc { Some((x0, x1)) => (x0.min(a), x1.max(b)), None => (a, b) })
        })
    }
    fn all_inside_the_screen(&self) -> bool {
        self.rows.iter().all(|r| r.0 >= -1.0 && r.1 <= 513.0 && r.2 >= -1.0 && r.3 <= 449.0)
    }
}

fn frame_with_font(fe: &FrontEnd, lang: Option<&nf_formats::hud::Lang>, aspect: f32) -> (nf_view::gpu::HudFrame, usize) {
    let font = iso_font(&std::env::var("NF_ISO").unwrap_or_default());
    let mut textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    textures.push((0x0300_0003, crate_texture(0x0300_0003)));   // FONT_TEX so the text quads land
    (menu::menu_overlay(fe, &font, &font, lang, &textures, aspect), textures.len())
}

/// The footer hint bar centres its label: Label_Update 0x218cb8:177-182 puts the sprite at
/// ctrl.x + ctrl.w/2 = 320 (640-space) = 256 HUD — the port's old draw ran left from x=50.
#[test]
fn the_footer_hints_are_centred_in_their_bars() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = iso_lang(&iso);
    for page in [Page::Main, Page::Codename, Page::Difficulty, Page::MissionSelect] {
        let mut fe = FrontEnd::new();
        fe.page = page;
        let (frame, n) = frame_with_font(&fe, lang.as_ref(), 4.0 / 3.0);
        let rows = text_rows(&frame, n, 4.0 / 3.0);
        eprintln!("[dbg] {page:?} rows: {:?}", rows.rows);
        let band = rows.band(390.0).or_else(|| rows.band(385.0)).or_else(|| rows.band(394.0)).expect("the footer hint draws");
        let centre = (band.0 + band.1) / 2.0;
        assert!((centre - 256.0).abs() < 5.0, "{page:?}: the hint row spans {band:?}, centre {centre} — the bar centres its label at 320/640 [M Label_Update 0x218cb8:177-182]");
        // Sister: the hint sits inside the bar control (50..590 → 40..472 HUD x).
        assert!(band.0 >= 39.0 && band.1 <= 473.0, "{page:?}: the hint {band:?} leaves its bar");
    }
}

/// The header label centres in its (50,40,376×17) band [M the pages' header CONTROL records;
/// Label_Update 0x218cb8:177-182/:202-206] — the old draw centred the text AT x=50, half off-screen.
#[test]
fn the_page_headers_are_centred_in_their_bands() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = iso_lang(&iso);
    for (page, header) in [(Page::Codename, 0x1f2), (Page::Difficulty, 0x1f8), (Page::MissionSelect, 0x14c), (Page::Dossier, 0x27f)] {
        let mut fe = FrontEnd::new();
        fe.page = page;
        let (frame, n) = frame_with_font(&fe, lang.as_ref(), 4.0 / 3.0);
        let rows = text_rows(&frame, n, 4.0 / 3.0);
        let band = rows.rows.iter().filter(|r| r.3 >= 40.0 && r.2 <= 56.0).map(|r| (r.0, r.1)).fold(None, |acc: Option<(f32, f32)>, (a, b)| {
            Some(match acc { Some((x0, x1)) => (x0.min(a), x1.max(b)), None => (a, b) })
        }).expect("the header draws");
        eprintln!("[hdr] {page:?} band {band:?} rows {:?}", rows.rows);
        let centre = (band.0 + band.1) / 2.0;
        let want = (50.0 + 376.0 / 2.0) * 512.0 / 640.0;   // 190.9 HUD
        assert!((centre - want).abs() < 5.0, "{page:?}: header centre {centre}, want {want} (ctrl.x + ctrl.w/2 [M Label_Update])");
        // Sister: the header text is the page data's own LBL.
        let s = lang.as_ref().and_then(|l| l.bind(header)).unwrap_or("");
        assert!(!s.is_empty(), "{page:?}: header LBL {header:#x} binds");
    }
}

/// The wheel rows centre in their controls: slot centres = ctrl.x + ctrl.w/2 [L the parsed rows:
/// (323,123,317) (333,152,307) (304,182,336) (333,211,307) (323,241,317)].
#[test]
fn the_wheel_rows_center_in_their_controls() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = iso_lang(&iso);
    let mut fe = FrontEnd::new();
    fe.page = Page::MissionSelect;
    let (frame, n) = frame_with_font(&fe, lang.as_ref(), 4.0 / 3.0);
    let rows = text_rows(&frame, n, 4.0 / 3.0);
    let wheel_y = [123.0f32, 152.0, 182.0, 211.0, 241.0];
    let wheel_cx = [(323.0 + 317.0 / 2.0) * 512.0 / 640.0, (333.0 + 307.0 / 2.0) * 512.0 / 640.0, (304.0 + 336.0 / 2.0) * 512.0 / 640.0, (333.0 + 307.0 / 2.0) * 512.0 / 640.0, (323.0 + 317.0 / 2.0) * 512.0 / 640.0];
    for (slot, y) in wheel_y.iter().enumerate() {
        let hy = y * 448.0 / 480.0 + 8.0;
        if let Some(band) = rows.band(hy) {
            let centre = (band.0 + band.1) / 2.0;
            assert!((centre - wheel_cx[slot]).abs() < 5.0, "slot {slot}: row centre {centre}, want {}", wheel_cx[slot]);
        }
    }
    // Sister: every text row on the page lies inside the screen.
    assert!(rows.all_inside_the_screen(), "a text row left the 512×448 screen");
}

/// The main menu's rows carry the page data's own colours: the selected row the msg-0x1b cream
/// (0x7d6d59ff rewritten 0x645a49ff [M MenuManager_Create 0x21b908:238-243]), the others the
/// msg-0x1c dark orange 0x73330f80 [L page 0x40000002's row controls, parsed 2026-10-14]. The old
/// draw had them swapped (orange selected, cream others).
#[test]
fn the_main_rows_use_the_page_colours() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = iso_lang(&iso);
    let font = iso_font(&iso);
    let mut textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    textures.push((0x0300_0003, crate_texture(0x0300_0003)));
    let mut fe = FrontEnd::new();
    fe.page = Page::Main;
    let frame = menu::menu_overlay(&fe, &font, &font, lang.as_ref(), &textures, 4.0 / 3.0);
    let k = (2.0 / 512.0) * (4.0 / 3.0) / (4.0 / 3.0);
    let near = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.02);
    let cream = [0x64 as f32 / 128.0, 0x5a as f32 / 128.0, 0x49 as f32 / 128.0, 0.996];
    let dark_orange = [0x73 as f32 / 128.0, 0x33 as f32 / 128.0, 0x0f as f32 / 128.0, 0.502];
    let mut saw_sel = false; let mut saw_other = false;
    for q in &frame.v {
        let y_hud = (1.0 - q.p[1]) * 224.0;
        // The rows sit at 640-space y 262..354 → HUD 244.5..330.4; the desc plate is below 357*0.933=333.2.
        if !(240.0..=332.0).contains(&y_hud) { continue; }
        if near(q.c, cream) { saw_sel = true; }
        if near(q.c, dark_orange) { saw_other = true; }
    }
    assert!(saw_sel, "the selected row draws the msg-0x1b cream 0x645a49ff");
    assert!(saw_other, "the other rows draw the msg-0x1c dark orange 0x73330f80");
    let _ = k;
}

/// The codename page lists the game's own rows — "New Codename" (Txt 0x29b [M Menu_EnumSaves
/// 0x1f7b78:58-66]) and "Play without saving" (Txt 0x10002ff [M Menu_PutCodenamesIntoControl
/// 0x1f79c0:15-23]) — and swaps the ring preview per cursor [M Menu_UpdateCodenameWheel 0x1f89f0:80-99].
#[test]
fn the_codename_page_lists_new_codename_and_play_without_saving() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = iso_lang(&iso).expect("USATxt.dat parses");
    let mut fe = FrontEnd::new();
    fe.page = Page::Codename;
    assert_eq!(fe.rows(), 2, "New Codename + Play without saving (profiles need a memory card)");
    assert_eq!(fe.row_label(Some(&lang), 0), lang.bind(0x29b).unwrap(), "row 0 is New Codename");
    assert_eq!(fe.row_label(Some(&lang), 1), lang.bind(0x100_02ff).unwrap(), "row 1 is Play without saving");
    // The ring preview follows the cursor: 0x300013b (create) / 0x3000135 (default) [M 0x1f89f0:80-99].
    let (frame0, n0) = frame_with_font(&fe, Some(&lang), 4.0 / 3.0);
    let hash = |f: &nf_view::gpu::HudFrame, n: usize, want: u32| f.runs.iter().any(|&(t, _, _, _)| t > 0 && t <= n && true) && f.runs.iter().any(|r| r.0 > 0);
    let _ = hash;
    let tex_of = |f: &nf_view::gpu::HudFrame| f.runs.iter().filter(|r| r.0 > 0).map(|r| r.0).collect::<Vec<usize>>();
    let has = |f: &nf_view::gpu::HudFrame, h: u32| {
        // runs' tex is an index into the caller's list; MENU_TEXTURES order is fixed.
        let idx = menu::MENU_TEXTURES.iter().position(|&t| t == h).expect("hash in MENU_TEXTURES") + 1;
        tex_of(f).contains(&idx)
    };
    assert!(has(&frame0, 0x0300_013b), "cursor 0 (New Codename) shows the create preview 0x300013b");
    fe.cursor = 1;
    let (frame1, _) = frame_with_font(&fe, Some(&lang), 4.0 / 3.0);
    assert!(has(&frame1, 0x0300_0135), "cursor 1 (Play without saving) shows the default preview 0x3000135");
    assert!(!has(&frame1, 0x0300_013b), "sister: the create preview is gone");
    let _ = n0;
}

/// The Dossier is the game's own wheel menu — Records / Rewards / Gadgets / Weapons [L table
/// 0x2dfcd0; M C_SBDOSSIER 0x216eb0:48-49] with the per-row preview textures, not the mission
/// briefing the port drew before.
#[test]
fn the_dossier_lists_records_rewards_gadgets_weapons() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = iso_lang(&iso).expect("USATxt.dat parses");
    let mut fe = FrontEnd::new();
    fe.page = Page::Dossier;
    assert_eq!(fe.rows(), 4, "the wheel table's four rows");
    let names = [0x280u32, 0x281, 0x282, 0x19c];
    let descs = [0x100_000au32, 0x100_000b, 0x100_000c, 0x100_000d];
    for (i, (&n, &d)) in names.iter().zip(descs.iter()).enumerate() {
        assert_eq!(fe.row_label(Some(&lang), i), lang.bind(n).unwrap(), "row {i} is the table's name label");
        assert!(lang.bind(d).is_some(), "row {i}'s description label binds");
    }
    // Confirming row 0 opens the Records table [M C_SBDOSSIER 0x216eb0:12-31 index 0 → 0x4000003a].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::Records);
    assert_eq!(fe.page.id(), 0x4000_003a);
}

/// The Records page draws the disc's own table [L PAGE 0x4000003a parsed 2026-10-14]: header 0x280,
/// hint 0x49e, the mission list, the three score rows (0x302/0x165/0x305) with their value controls,
/// the medal image 0x0300013d + label 0x307.
#[test]
fn the_records_page_draws_the_score_table() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = iso_lang(&iso).expect("USATxt.dat parses");
    let mut fe = FrontEnd::new();
    fe.page = Page::Records;
    let (frame, n) = frame_with_font(&fe, Some(&lang), 4.0 / 3.0);
    // The medal image is the page data's own texture control.
    let idx = menu::MENU_TEXTURES.iter().position(|&t| t == 0x0300_013d).expect("medal tex listed") + 1;
    assert!(frame.runs.iter().any(|r| r.0 == idx), "the medal image 0x0300013d draws");
    // The three score-row labels + the medal label bind from the language bank.
    for id in [0x302u32, 0x165, 0x305, 0x307] {
        let s = lang.bind(id).unwrap_or("");
        assert!(!s.is_empty(), "records label {id:#x} binds (got empty)");
    }
    // Sister: every text row lies inside the screen.
    let rows = text_rows(&frame, n, 4.0 / 3.0);
    assert!(rows.all_inside_the_screen(), "a records text row left the screen");
}

/// The title prompt flashes in the page data's own gold 0x7f621fff [L page 0x40000009 control
/// 0x100000fa's msg-0x1b payload, parsed 2026-10-14] — the old constant 0xfec43e didn't match its
/// own citation.
#[test]
fn the_title_prompt_uses_the_page_colour() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = iso_lang(&iso);
    let font = iso_font(&iso);
    let mut textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    textures.push((0x0300_0003, crate_texture(0x0300_0003)));
    let mut fe = FrontEnd::new();
    fe.page = Page::Start;
    fe.press_start_on = true;
    let frame = menu::menu_overlay(&fe, &font, &font, lang.as_ref(), &textures, 4.0 / 3.0);
    let want = [0x7f as f32 / 128.0, 0x62 as f32 / 128.0, 0x1f as f32 / 128.0, 0.996];
    let hit = frame.v.iter().any(|q| q.c.iter().zip(want).all(|(a, b)| (a - b).abs() < 0.02));
    assert!(hit, "the prompt draws in 0x7f621f gold (gs scale)");
}

// --- fix-pause-menu2: the pause page's tab contents ---

/// The yellow arrow cursor left of the selected MISSION row [L the main agent's PCSX2 captures
/// 2026-10-13; the port had none]. The cursor is the page's only yellow draw (menu.rs `YELLOW`, the page
/// data's 0x7f621f since fix-menus-real; it was 0xfec43e when this test was written), so its colour pins it.
#[test]
fn the_pause_mission_tab_has_the_yellow_row_cursor() {
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).chain(std::iter::once((0x0300_0003u32, crate_texture(0x0300_0003)))).collect();
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    assert!(fe.open_pause());
    let yellow = |f: &gpu::HudFrame| f.v.iter().filter(|v| (v.c[0] - 0x7f as f32 / 128.0).abs() < 0.02 && (v.c[1] - 0x62 as f32 / 128.0).abs() < 0.02).count();
    let f = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    let n = yellow(&f);
    assert!(n >= 3, "the MISSION tab draws the yellow row cursor (found {n} yellow verts)");
    // Sister: the cursor follows the selection — its verts move down a row (the 640-space rows are
    // 40 px apart [L the d1=6 list's 183+20+i*40 rows], 448-space 35 px).
    fe.cursor = 2;
    let f2 = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    // NDC y runs DOWN as the row index grows (row 0 ≈ +0.15, row 2 ≈ −0.18).
    let y0 = f.v.iter().filter(|v| (v.c[0] - 0x7f as f32 / 128.0).abs() < 0.02 && (v.c[1] - 0x62 as f32 / 128.0).abs() < 0.02).map(|v| v.p[1]).fold(f32::MIN, f32::max);
    let y2 = f2.v.iter().filter(|v| (v.c[0] - 0x7f as f32 / 128.0).abs() < 0.02 && (v.c[1] - 0x62 as f32 / 128.0).abs() < 0.02).map(|v| v.p[1]).fold(f32::MIN, f32::max);
    assert!(y0 > f32::MIN && y2 < y0 - 0.1, "the cursor moved to the selected row (y {y0} -> {y2})");
}

/// The OBJECTIVES tab lists the level's objectives, a ~J (done) / ~I (pending) marker before each
/// name [M C_GCPAUSE_Handler 0x204408:251-271; the markers are ACTION.ELF .data 0x30d2e0/0x30d2e8].
/// The marker glyphs draw through the specialchar table ('J' = u176 v57 w16 h12 [M 0x2dd8d8]).
#[test]
fn the_pause_objectives_tab_draws_the_objective_rows_with_their_markers() {
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).chain(std::iter::once((0x0300_0003u32, crate_texture(0x0300_0003)))).collect();
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    assert!(fe.open_pause());
    fe.pause_tab = 1;
    let empty = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0).v.len();
    // Two objectives: the first pending, the second done (state 4 [M 0x204408:259]).
    fe.pause_objectives = vec![(0x0400_0000, false), (0x0400_0001, true)];
    let f = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    assert!(f.v.len() > empty, "the objectives rows add quads ({} -> {})", empty, f.v.len());
    // The 'J' marker's glyph quad: uv = (176,57) texels of the specialchar atlas, normalised by its
    // real 256×128 pitch [L tex-list 07000048.bin; the runtime table's v is in texels].
    let j = f.v.iter().filter(|v| (v.uv[0] - 176.0 / 256.0).abs() < 0.01 && (v.uv[1] - 57.0 / 128.0).abs() < 0.02).count();
    assert!(j >= 2, "the done row draws the ~J marker glyph (found {j} verts)");
}

/// The CONTROLS tab draws the controller-style page: the style name and the per-action
/// "<glyph> <label>" rows [M Menu_DisplayControllerStyleList 0x1f44d8 case 0 — 6 rows × 2 columns,
/// the glyph strings ACTION.ELF .data 0x30d1c8-0x30d228]. Before the fix the tab showed only
/// "Inverted" and an empty plate.
#[test]
fn the_pause_controls_tab_draws_the_button_rows() {
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).chain(std::iter::once((0x0300_0003u32, crate_texture(0x0300_0003)))).collect();
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    assert!(fe.open_pause());
    fe.pause_tab = 2;
    let f = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    // The 12 glyph quads (6 rows × 2 columns) draw from the specialchar atlas (u ≥ 150 in the
    // game's table [M 0x2dd8d8's entries]).
    let glyphs = f.v.iter().filter(|v| v.uv[0] >= 150.0 / 256.0 && v.uv[0] <= 1.0 && v.uv[1] < 0.5).count();
    assert!(glyphs >= 12, "the CONTROLS rows draw their button glyphs (found {glyphs} glyph verts)");
    // Sister: the rows are the game's own label ids (0x8f/0x8c, 0x8b/0x8d, 0x84/0x89, 0x85/0x8a,
    // 0x88/0x90, 0x87/0x86 [M 0x1f44d8 case 0]) — they resolve from USATxt.dat.
}

/// The SCORE tab shows the game's own table: the Category/Current/Target header and the 8 rows
/// [M C_GCPAUSE_Handler 0x204408:447-498], with the level's TARGETS from ACTION.ELF's static
/// score records [L 0x2bc658 pointer table, extracted 2026-10-14].
#[test]
fn the_pause_score_tab_shows_the_disc_targets() {
    // 07000005's row: Bond Moves 2, Dispatched 30, Subdued 3, Surrendered 3, —, Accuracy 100,
    // Health 100, Time 09:00, Bonus 7 — the PCSX2 capture's numbers [L the task's captures].
    assert_eq!(menu::score_row_for_level(0x0700_0005), Some(1), "the campaign row of 07000005 [M Menu_GetLevelIndex 0x2023e8 walks sp_level[i]+0xc]");
    let t = menu::SCORE_TARGETS[1];
    assert_eq!((t[0], t[1], t[2], t[3]), (2, 30, 3, 3), "Bond Moves / Dispatched / Subdued / Surrendered targets");
    assert_eq!((t[5], t[6], t[7], t[8]), (100, 100, 540, 7), "Accuracy / Health / Time (09:00 = 540 s) / 007 Bonus targets");
    // Every campaign level has a row (the game's 12 sp_level entries [M PlrStat_Init 0x198318 loops 12]).
    assert_eq!(menu::SCORE_TARGETS.len(), 12);
    // The drawn tab carries the header + 8 rows + the Time row: at least 10 text draws' worth of
    // quads beyond the empty plate.
    let font = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).chain(std::iter::once((0x0300_0003u32, crate_texture(0x0300_0003)))).collect();
    let mut fe = FrontEnd::new();
    fe.flow.current_level = 0x0700_0005;
    assert!(fe.open_pause());
    fe.pause_tab = 3;
    let f = menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    assert!(f.v.len() > 60, "the SCORE tab draws the header and the rows ({} verts)", f.v.len());
}

/// (NF_ISO) The pause page's figure: texture 0x030000db loads as the 64×128 alpha mask (the 5551
/// alpha-bit silhouette), not the 8bpp blob card. Before the fix the load produced the palette-card
/// picture (coloured blobs + white figure) the user saw.
#[test]
fn the_pause_figure_texture_loads_as_the_silhouette_mask() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (_, t) = scene.hud_textures.iter().find(|(h, _)| *h == 0x0300_00db).expect("the pause figure texture loads");
    assert_eq!((t.width, t.height), (128, 128), "the figure texture is 128×128 [L the map header]");
    // The figure = index 0x43 whose palette entry is pure WHITE [L pal[067]=ffffff80; the index
    // histogram: a flat run of 987 0x4343 words in the figure's bbox]. The white pixels sit in the
    // texture's RIGHT half (x ≥ 64) — the blobs (the left half) never enter the menu's uv rect.
    let white: Vec<(usize, usize)> = (0..128usize).flat_map(|y| (0..128usize).map(move |x| (x, y)))
        .filter(|&(x, y)| { let i = (y * 128 + x) * 4; t.rgba[i] == 255 && t.rgba[i + 1] == 255 && t.rgba[i + 2] == 255 })
        .collect();
    assert!(white.len() >= 900, "the white silhouette is present ({} px)", white.len());
    let left = white.iter().filter(|(x, _)| *x < 64).count();
    assert!(left * 10 < white.len(), "the silhouette sits in the texture's right half ({} left-half px of {})", left, white.len());
}

// ── fix-menus-real2: the headers' bold-italic face ──────────────────────────────────────────────
// Evidence: the header controls' page-def NAME (−8) format strings are "\xff\x01" = font select 1
// [L the menu-def tag-stream dump 2026-10-14 — header CTRL (50,40,376×17) name 0x10000001 followed
// by NAME bytes ff 00 01 00 … (UTF-16, compacted by MenuManager_Create 0x21b908:255-274's walk);
// M Label_SendMessage 0x218890:100-104 applies the string over Label_Init's stamped default;
// Font_ParseFormat 0x1ee3e0 stops at the first NUL]. FontTable[0] = {tex 0x03000005, FONTNFont2
// 0x305a58} [L ee_snapshot 0x2dd8b0], and 0x03000005's atlas is the bold-italic face [D tex-export
// 07000048.bin entry 3 sub 2].

/// The header font parses from ACTION.ELF's FONTNFont2 and is a DIFFERENT face from the body font.
#[test]
fn the_header_font_is_the_games_second_face() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let image = nf_formats::iso9660::IsoImage::open(&iso).expect("iso");
    let entry = image.find("/ACTION.ELF").expect("ACTION.ELF");
    let elf = image.read_file(entry).expect("read");
    let t = nf_formats::hud::HudTables::from_action_elf(&elf).expect("HUD tables");
    // Both faces parse with the same char range (149 glyphs, '!'–0xff [L FontTable header +8/+10]).
    assert_eq!(t.font_header.first, t.font.first, "the header face covers the same chars");
    assert_eq!(t.font_header.glyphs.len(), t.font.glyphs.len());
    assert!(t.font_header.glyphs.len() > 100, "the header face parses its glyph table");
    // …and it is a different cut: the glyph metrics differ (the italic face is a condensed heavy
    // weight [D texture 0x03000005]).
    let differs = t.font_header.glyphs.iter().zip(t.font.glyphs.iter()).filter(|(a, b)| a.w != b.w || a.h != b.h || (a.u, a.v) != (b.u, b.v)).count();
    assert!(differs > 100, "the header face's glyphs differ from SerpLight's ({differs} of {})", t.font.glyphs.len());
}

/// The page headers draw through the header face's own atlas 0x03000005, not the body font texture.
#[test]
fn the_page_headers_draw_the_header_font_atlas() {
    // The header text is the page data's LBL (0x1f8 etc.) — without the language file the label is
    // empty and draws nothing, so this needs the disc's USATxt.dat.
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = iso_lang(&iso);
    let font = test_font();
    let hfont = test_font();
    let textures: Vec<(u32, nf_view::scene::TextureImage)> = menu::MENU_TEXTURES.iter().map(|&h| (h, crate_texture(h))).collect();
    let mut fe = FrontEnd::new();
    for page in [Page::Difficulty, Page::MissionSelect, Page::Dossier] {
        fe.page = page;
        let frame = menu::menu_overlay(&fe, &font, &hfont, lang.as_ref(), &textures, 4.0 / 3.0);
        let ti = textures.iter().position(|(h, _)| *h == 0x0300_0005).expect("header atlas listed in MENU_TEXTURES") + 1;
        let mut header_quads = 0;
        for run in &frame.runs {
            if run.0 != ti { continue; }
            header_quads += run.3 as usize / 6;
        }
        assert!(header_quads > 0, "{page:?}: the header draws glyph quads from the 0x03000005 atlas [L FontTable[0]]");
    }
}
