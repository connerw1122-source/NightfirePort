//! fix-movie-playback headless tests: the mission-flow movie codes (briefing wrap, outros), the
//! state-6 wait/skip/resolve, and the return to the level load.
//!
//! Evidence: `ResetMap_LevelToLoad__FUiScSc` 0x1d3800 (the briefing wrap switch, compiled code —
//! 22 `lui rt,0x710` words at 0x1d392c-0x1d3a40, no jump table [L ee_snapshot scan]), `ResetMap_Load__Fv`
//! 0x1d6360 (phase 0 plays the movie and pushes state 6; phase 2 resolves it with
//! `GetLevelToLoadFromMovieCode__Fv` 0x1d3560), `GameFlow_Main__Fv` 0x1c98c8 case 6 (`psiMovieFinished()
//! || Input_Action(-1, 0x19, 1)` → `psiStopBackgroundMovie` → pop), `Mission_Update__Fv` 0x182c60
//! case 6 (the level → outro-code switch, lui/ori pairs at 0x182e60-0x182eb4 [L ee_snapshot]),
//! `UsingMovie__FUi` 0x107eb8 (the 23-hash whitelist), `psiStartBackgroundMovie__FUiScUi` 0x1081a8.

use nf_view::menu::{self, Action, FrontEnd, Outcome, Page, Stats, ST_MAPLOAD, ST_MOVIE, ST_RUNNER};
use nf_view::movie::{self, StartError};

/// Drives the boot path to the title the way `GameFlow_Main` + `ResetMap_Load` would.
fn tick_to_title(fe: &mut FrontEnd) {
    for _ in 0..8 {
        fe.tick();
        if fe.flow.get_state() == ST_RUNNER && fe.flow.current_level == menu::FRONTEND_LEVEL { return; }
    }
    panic!("boot never reached the front-end map");
}

/// The game wraps story levels to their 0x71 briefing codes through a COMPILED switch, not a table
/// [M 0x1d3800]: identity on the low byte for 0x7000001..0x7000016, plus exactly two special cases —
/// 0x700001b → 0x7100018 (NOT identity: the Equinox briefing is movie 18; 07100018.PSS is the file on
/// the disc [D /MOVIES/30_FPS]) and 0x700004a → 0x710004a. The old port wrap (`level + 0x100000`)
/// produced 0x0710001b for level 0x0700001b, a movie that does not exist.
#[test]
fn equinox_briefing_wraps_to_07100018_like_the_game_switch() {
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![ST_RUNNER]; // Mission_Update / the menu handlers run in state 2 [M 0x1d3800's gate]
    fe.flow.reset_map_level_to_load(0x0700_001b, false, false);
    assert_eq!(fe.flow.level_to_load, 0x0710_0018,
        "ResetMap_LevelToLoad 0x1d3800 case 0x700001b: param_1 = 0x7100018 [M]");
}

/// The full briefing table [M 0x1d3800's switch]: identity for 0x01..0x16 and 0x4a, the 0x1b → 0x18
/// special case, and no wrap for the front-end map, the driving codes, or the codes the switch does
/// not list (0x7000017..0x1a, 0x700004b).
#[test]
fn the_briefing_wrap_matches_the_game_switch() {
    let wrap = |level: u32| {
        let mut fe = FrontEnd::new();
        fe.flow.stack = vec![ST_RUNNER];
        fe.flow.reset_map_level_to_load(level, false, false);
        fe.flow.level_to_load
    };
    for b in 0x01..=0x16u32 {
        assert_eq!(wrap(0x0700_0000 + b), 0x0710_0000 + b, "level {b:#x} wraps to its own briefing [M]");
    }
    assert_eq!(wrap(0x0700_004a), 0x0710_004a, "level 0x4a wraps to 0x710004a [M]");
    assert_eq!(wrap(0x0700_0048), 0x0700_0048, "the front-end map is not in the switch [M]");
    assert_eq!(wrap(0x0900_0001), 0x0900_0001, "driving codes load directly [M]");
    assert_eq!(wrap(0x0700_0017), 0x0700_0017, "0x7000017 has no briefing entry [M]");
    assert_eq!(wrap(0x0700_004b), 0x0700_004b, "0x700004b has no briefing entry [M]");
}

/// Mission_Update 0x182c60 case 6's outro switch [M; the lui/ori pairs at 0x182e60-0x182eb4 [L]]:
/// own 0x73 codes for 04/08/0b/0d/0f/13/1b, 0x7000016 → 0x7300017 (the one non-identity entry), the
/// map for everything else.
#[test]
fn the_outro_codes_match_mission_update_case6() {
    assert_eq!(menu::outro_for(0x0700_0004), 0x0730_0004);
    assert_eq!(menu::outro_for(0x0700_0008), 0x0730_0008);
    assert_eq!(menu::outro_for(0x0700_000b), 0x0730_000b);
    assert_eq!(menu::outro_for(0x0700_000d), 0x0730_000d);
    assert_eq!(menu::outro_for(0x0700_000f), 0x0730_000f);
    assert_eq!(menu::outro_for(0x0700_0013), 0x0730_0013);
    assert_eq!(menu::outro_for(0x0700_0016), 0x0730_0017, "the one non-identity outro [M 0x182c60 case 6]");
    assert_eq!(menu::outro_for(0x0700_001b), 0x0730_001b);
    assert_eq!(menu::outro_for(0x0700_0005), menu::FRONTEND_LEVEL, "no outro for 0x05 — the map [M default arm]");
    assert_eq!(menu::outro_for(0x0700_004a), menu::FRONTEND_LEVEL, "no outro for 0x4a [M default arm]");
}

/// The mission start through the real input path (menus_flow's walk): confirming a campaign row
/// wraps the level to the briefing code, load_tick phase 0 records the movie code and pushes
/// state 6 [M 0x1d6360], the countdown model (no movie data on the slim ISO) pops it, and phase 2
/// resolves the plain level for the load [M 0x1d3560].
#[test]
fn a_menu_started_mission_rides_state6_back_to_the_level_load() {
    let mut fe = FrontEnd::new();
    tick_to_title(&mut fe);
    fe.action(Action::Confirm); // title → intro movie state (the countdown model)
    // The intro runs its 450 frames (or the skip pops it) before the menu pages work — case 6
    // holds the flow until the movie ends [M 0x1c98c8 case 6].
    for _ in 0..fe.flow.movie_frames { fe.flow.main_tick(); }
    assert_eq!(fe.flow.get_state(), ST_RUNNER, "the intro popped; state 2");
    fe.action(Action::Confirm); // NightFire → Codename
    fe.action(Action::Down);    // row 1 = Play without saving [M 0x1f79c0:15-23; fix-menus-real]
    fe.action(Action::Confirm); // Codename → the confirm box armed
    fe.action(Action::Confirm); // the box → Select Difficulty
    fe.action(Action::Confirm); // Difficulty → Single Player
    fe.unlock_all = true;
    fe.cursor = 1;
    assert!(fe.action(Action::Confirm)); // The Exchange (0x07000005)
    assert_eq!(fe.flow.level_to_load, 0x0710_0005);
    assert!(fe.flow.movie_pending, "the host must hold the level load until phase 2 [M 0x1d6360 phase 0]");
    assert!(!fe.flow.load_tick(), "phase 0: the movie wrap");
    assert_eq!(fe.flow.movie_code, 0x0710_0005, "GameState+0x10 [M 0x1d6360]");
    assert_eq!(fe.flow.get_state(), ST_MOVIE, "state 6 pushed [M]");
    // The countdown model pops (no real movie started — movie_playing is false); phase 2 resolves.
    fe.flow.main_tick();
    assert!(!fe.flow.load_tick(), "phase 2 returns false; the next call loads");
    assert_eq!(fe.flow.level_to_load, 0x0700_0005, "GetLevelToLoadFromMovieCode [M 0x1d3560]");
    assert_eq!(fe.flow.phase2_resolved, Some(0x0700_0005), "the host's load hook reads this");
    assert!(!fe.flow.movie_pending, "the load may proceed now");
    assert!(fe.flow.load_tick());
    assert_eq!(fe.flow.get_state(), ST_RUNNER);
}

/// The skip: GameFlow_Main case 6 ends the movie on the skip action [M 0x1c98c8 —
/// `Input_Action(-1, 0x19, 1)`], and everything else (movement, back) does nothing while it plays.
#[test]
fn the_movie_skips_on_the_action_and_returns_to_the_load() {
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![ST_RUNNER];
    fe.flow.reset_map_level_to_load(0x0700_0005, false, false);
    assert!(!fe.flow.load_tick(), "phase 0");
    assert_eq!(fe.flow.get_state(), ST_MOVIE);
    // A real movie is playing: BGFMVPlaying = 1 holds state 6 against the countdown [M 0x1082e8].
    fe.flow.movie_playing = true;
    for _ in 0..120 { fe.flow.main_tick(); }
    assert_eq!(fe.flow.get_state(), ST_MOVIE, "state 6 holds while the movie plays");
    // The skip pops it (case 6 reads only action 0x19 [M]).
    assert!(fe.flow.skip_movie(), "skip_movie on state 6 pops [M 0x1c98c8]");
    let mut fe2 = FrontEnd::new();
    fe2.flow.stack = vec![ST_RUNNER];
    fe2.flow.reset_map_level_to_load(0x0700_0005, false, false);
    assert!(!fe2.flow.load_tick());
    fe2.flow.movie_playing = true;
    assert!(fe2.flow.skip_movie(), "the skip pops state 6 [M 0x1c98c8]");
    assert_eq!(fe2.flow.get_state(), ST_MAPLOAD);
    assert!(fe2.flow.movie_playing, "the host clears BGFMVPlaying when it stops the player");
    assert!(!fe2.flow.load_tick(), "phase 2 runs after the pop");
    assert_eq!(fe2.flow.phase2_resolved, Some(0x0700_0005), "the skip returns to the level load [M]");
}

/// A mission complete routes through the outro: mission_ended requests Mission_Update case 6's
/// LevelToEndTo [M 0x182c60], the wrap plays it in state 6, and phase 2 returns to the front-end
/// map with the results page [M 0x1d6360 phase 2 + 0x1d3560].
#[test]
fn a_completed_mission_plays_the_outro_and_returns_to_the_map() {
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![ST_RUNNER]; // Mission_Update fires in state 2 [M]
    fe.flow.current_level = 0x0700_0004;
    fe.mission_ended(Outcome::Complete, Stats::default());
    // Mission_Update 0x182c60 case 4 pushes the load and THEN the 60-frame fade on top [M]: the fade plays out
    // first (GameFlow_Main state 8 pops when its frames run out), then the load runs.
    while fe.flow.get_state() == menu::ST_FADE_HOLD { fe.flow.main_tick(); }
    assert_eq!(fe.requested_level, Some(0x0730_0004), "LevelToEndTo = the level's outro code [M 0x182c60 case 6]");
    assert_eq!(fe.flow.level_to_load, 0x0730_0004, "ResetMap_LevelToLoad(LevelToEndTo, 0, 0) [M case 4]");
    assert!(fe.flow.movie_pending);
    // Mission_Update case 4's order [M; 568a6e8]: the load pushes first, THEN the 8-frame fade
    // holds on top — run it out so the load machine is on top again.
    for _ in 0..8 { fe.flow.main_tick(); }
    assert!(!fe.flow.load_tick(), "phase 0: the outro wrap");
    assert_eq!(fe.flow.movie_code, 0x0730_0004);
    assert_eq!(fe.flow.get_state(), ST_MOVIE);
    fe.flow.main_tick(); // the countdown model pops (or the skip/end does with a real movie)
    assert!(!fe.flow.load_tick(), "phase 2");
    assert_eq!(fe.flow.phase2_resolved, Some(menu::FRONTEND_LEVEL), "every outro routes to the map [M 0x1d3560]");
    assert_eq!(fe.flow.page, menu::PAGE_RESULTS, "phase 2 sets +0x28 = 0x40000036 on the map [M 0x1d6360]");
}

/// A failed mission goes straight to the map — Mission_Update cases 2/3 set LevelToEndTo = 0x7000048
/// with no movie [M 0x182c60].
#[test]
fn a_failed_mission_returns_to_the_map_without_a_movie() {
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![ST_RUNNER];
    fe.flow.current_level = 0x0700_0004;
    fe.mission_ended(Outcome::Failed, Stats::default());
    // Mission_Update 0x182c60 case 4 pushes the load and THEN the 60-frame fade on top [M]: the fade plays out
    // first (GameFlow_Main state 8 pops when its frames run out), then the load runs.
    while fe.flow.get_state() == menu::ST_FADE_HOLD { fe.flow.main_tick(); }
    assert_eq!(fe.requested_level, Some(menu::FRONTEND_LEVEL), "failures skip the outro [M cases 2/3]");
    assert!(!fe.flow.movie_pending, "a plain load, no wrap");
    // The 8-frame fade holds on top of the load [M 0x182c60 case 4's order; 568a6e8] — run it out.
    for _ in 0..8 { fe.flow.main_tick(); }
    assert_eq!(fe.flow.get_state(), ST_MAPLOAD, "the load is on its way");
    assert!(fe.flow.load_tick(), "no movie phase — one tick loads");
}

/// The `UsingMovie` whitelist [M 0x107eb8]: an unlisted code never starts (the game's
/// psiStartBackgroundMovie returns before opening anything, so state 6 exits on frame 1), whatever
/// image is mounted. The check runs before any file IO, so this needs no ISO.
#[test]
fn unwhitelisted_codes_never_start_a_movie() {
    assert!(matches!(movie::Player::start("/nonexistent/no.iso", 0x0710_0002), Err(StartError::NotWhitelisted)),
        "0x7100002 is not in UsingMovie [M 0x107eb8]");
    assert!(matches!(movie::Player::start("/nonexistent/no.iso", 0x0730_000f), Err(StartError::NotWhitelisted)),
        "0x730000f is not whitelisted — level 0x0f's silent outro [M]");
    assert!(matches!(movie::Player::start("/nonexistent/no.iso", 0x0710_0005), Err(StartError::NoMovieData(_))),
        "a whitelisted code passes the gate and reaches the file open [M 0x1081a8]");
}

/// With the retail image mounted (NF_MOVIE_ISO) the briefing movie really plays: the player starts,
/// the first decoded frame is the game's 512×448 plane, and the stream has not ended. Skips when
/// only the stubbed on-foot image is available.
#[cfg(feature = "movies")]
#[test]
fn the_briefing_movie_plays_from_the_retail_image() {
    let Some(path) = std::env::var_os("NF_MOVIE_ISO") else { eprintln!("skip: NF_MOVIE_ISO (a retail image with movie data) is not set"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&path).unwrap();
    let mut p = match movie::Player::start(path.to_string_lossy().as_ref(), 0x0710_0001) {
        Ok(p) => p,
        Err(StartError::NoMovieData(_)) => { eprintln!("skip: this image has no movie data"); return; }
        Err(e) => panic!("start failed: {e:?}"),
    };
    assert_eq!(p.code(), 0x0710_0001);
    // Decode a few presents: the whole-stream decode takes ~23 s on 07100001, so wait up to 90 s
    // for the first frame (the test skips on images without movie data; the suite never runs it).
    let mut frame = None;
    for _ in 0..9000 {
        if let Some((w, h, rgba)) = p.present() {
            frame = Some((w, h, rgba.len()));
            if frame.is_some() { break; }
        }
        if p.finished() { break; }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let (w, h, len) = frame.expect("the briefing movie decoded a frame from the retail image");
    assert_eq!((w, h), (512, 448), "the game's full-screen plane [M AllocateMovieMemory 0x107d70]");
    assert_eq!(len, (512 * 448 * 4) as usize, "RGBA8, one pixel per byte-four");
    assert!(!p.finished(), "the Paris Prelude briefing is ~40 s — the stream is not over");
    p.stop(); // psiStopBackgroundMovie [M 0x108328]
}

/// Sister of the data test on a stubbed image: a whitelisted code with no movie data is refused and
/// the flow falls back to the countdown model (the state machine test above covers the rest).
#[test]
fn the_stubbed_image_declines_real_playback() {
    let Some(path) = std::env::var_os("NF_ISO").or_else(|| std::env::var_os("NF_MOVIE_ISO")) else { eprintln!("skip: no NF_ISO"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&path).unwrap();
    let probe = iso.find("/MOVIES/30_FPS/07100005.PSS");
    match probe.map(|e| e.size) {
        Some(0) => assert!(matches!(movie::Player::start(path.to_string_lossy().as_ref(), 0x0710_0005), Err(StartError::NoMovieData(_))),
            "the 0-byte stub is refused (the game would hang in strFileOpen [M 0x108cc0])"),
        Some(_) => { let _ = iso; eprintln!("this image has movie data — nothing to assert here"); }
        None => eprintln!("no movie directory on this image"),
    }
}
