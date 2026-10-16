//! The GameFlow stack across a full SP run, against the game's push/pop sequence.
//!
//! Game: `GameFlow_Main` 0x1c98c8 ticks only the TOP of `GameStateStack` (0x2c7398) [M]; the
//! sequence here is the decompile's: boot 1 → 2 → (Reset_MapLoadSettings 0x1d6958 →
//! ResetMap_LevelToLoad 0x1d3800) 3 → the loader's pop-3/push-4 tail (ResetMap_Load 0x1d6360)
//! → 4 pops itself (case 4) → the front end sits in 2; a mission pick pushes 3 then the
//! 84-frame fade (C_SBNFMAP 0x20ba50: PushState(84.0f, 7) → 8); the briefing movie rides state
//! 6 (ResetMap_Load phase 0), the load finishes under the pops, and the mission runs in 2; the
//! mission end pushes 3 then the 60-frame fade (Mission_Update 0x183054: PushState(60.0f, 7)),
//! and the next level repeats. States 5 and 10 are pushed by nothing on NTSC-U [M: the 16
//! PushState call sites are all enumerated] — 5 is only the dead arm of the draw-skip set
//! {1,3,4,5,9}.
use nf_view::menu::{
    self, Action, FrontEnd, Outcome, Stats, ST_BOOT, ST_FADE_HOLD, ST_LAUNCH_DRIVING,
    ST_MAPLOAD, ST_MOVIE, ST_RUNNER, ST_WAIT,
};

fn tick_to_title(fe: &mut FrontEnd) {
    fe.flow.main_tick();
    let _ = fe.flow.load_tick();
}

/// The boot's stack: [1] → [1,2] → [1,2,3] in ONE GameFlow_Main tick (case 1 pushes 2 inline and
/// Reset_MapLoadSettings pushes 3 in the same tick [M 0x1c98c8 case 1 + 0x1d6958]), then the
/// front-end load's pop-3/push-4 tail leaves [1,2] [M 0x1d6360 tail + case 4].
#[test]
fn the_boot_stack_sequence() {
    let mut fe = FrontEnd::new();
    assert_eq!(fe.flow.stack, vec![ST_BOOT]);
    fe.flow.main_tick();
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD],
        "case 1 pushes 2, Reset_MapLoadSettings pushes 3 in the same tick [M 0x1c98c8 case 1]");
    assert!(fe.flow.load_tick(), "the front-end map loads in one headless tick");
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER],
        "pop 3 + push 4 + case 4's Locks_Init pop [M 0x1d6360 tail + 0x1c98c8 case 4]");
    assert_eq!(fe.flow.current_level, menu::FRONTEND_LEVEL);
}

/// The full SP run: title → briefing → level → outro → results → next level. Every step's stack
/// is the game's.
#[test]
fn the_sp_run_stack_matches_the_game() {
    let mut fe = FrontEnd::new();
    tick_to_title(&mut fe);
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER]);

    // --- the title → intro movie → main menu: the game's intro is P_INTRO's own background FMVs
    // (it never touches the stack [M 0x209e88]) — the port models it as state 6 [S port choice].
    assert!(fe.action(Action::Confirm));
    assert_eq!(*fe.flow.stack.last().unwrap(), ST_MOVIE);
    for _ in 0..fe.flow.movie_frames { fe.flow.main_tick(); }
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER], "the intro popped back to the runner");

    // --- the menu walk to a mission pick (the game's page path, no other stack changes
    // [M the menu handlers only push through ResetMap_LevelToLoad + their fade]).
    assert!(fe.action(Action::Confirm));  // NightFire → Codename
    assert!(fe.action(Action::Down));     // Play without saving
    assert!(fe.action(Action::Confirm));  // the box arms
    assert!(fe.action(Action::Back));     // cancel
    assert!(fe.action(Action::Confirm));  // arm again
    assert!(fe.action(Action::Confirm));  // through
    assert!(fe.action(Action::Confirm));  // Difficulty → Single Player
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER], "menus ride state 2 — the MenuManager is not a flow state");
    fe.unlock_all = true;
    fe.cursor = 1;

    // --- the mission pick [M C_SBNFMAP 0x20ba50-5c]: ResetMap_LevelToLoad pushes 3, then
    // PushState(84.0f, 7) — the fade hold 8 — on top.
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD, ST_FADE_HOLD],
        "the loader under the 84-frame fade [M 0x1d3800 + 0x20ba50]");
    assert_eq!(fe.flow.fade_frames, 84, "0x42a00000 = 84.0f [M disasm 0x20ba50]");
    for _ in 0..=84 { fe.flow.main_tick(); }   // 84 run frames + the finished-frame pop [M 0x1bada0]
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD],
        "case 8 pops when the fade finishes [M 0x1c98c8 case 8]");
    assert!(!fe.flow.fade_active, "the fade slot freed [M 0x1bada0's 1→2→0]");

    // --- the loader: phase 0 pushes the briefing movie [M ResetMap_Load 0x1d6360 phase 0].
    assert!(!fe.flow.load_tick(), "phase 0 wraps to the movie");
    assert_eq!(fe.flow.movie_code, 0x0710_0005, "GameState+0x10 [M phase 0]");
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD, ST_MOVIE],
        "state 6 over the loader [M phase 0]");
    // The briefing holds while BGFMVPlaying is set [M 0x1082e8 + case 6]...
    fe.flow.movie_playing = true;
    fe.flow.main_tick();
    assert_eq!(*fe.flow.stack.last().unwrap(), ST_MOVIE, "case 6 holds while the movie plays");
    // ...and pops when it ends (psiMovieFinished = BGFMVPlaying == 0).
    fe.flow.movie_playing = false;
    fe.flow.main_tick();
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD], "the movie popped [M case 6]");
    assert!(!fe.flow.load_tick(), "phase 2: GetLevelToLoadFromMovieCode [M 0x1d3560]");
    assert_eq!(fe.flow.level_to_load, 0x0700_0005);
    assert!(fe.flow.load_tick(), "phase 3: the load + the pop-3/push-4 tail");
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER], "the mission runs in state 2 [M 0x1d6360 tail]");
    assert_eq!(fe.flow.current_level, 0x0700_0005, "GameState+0x12 [M phase 2]");

    // --- the mission end [M Mission_Update 0x182c60 case 4 / disasm 0x183054-60]:
    // +0x28 = 0x40000036, LevelToLoad pushes 3, then PushState(60.0f, 7) — a 60-frame fade.
    fe.mission_ended(Outcome::Complete, Stats { shots_fired: 120, shots_hit: 40, accuracy_pct: 33, time_s: 754 });
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD, ST_FADE_HOLD],
        "the end fade over the next load [M 0x182c60 case 4]");
    assert_eq!(fe.flow.fade_frames, 60, "0x42700000 = 60.0f [M disasm 0x183054]");
    for _ in 0..=60 { fe.flow.main_tick(); }
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD]);

    // --- 0x07000005 has no outro — the end_to default arm is the front end [M 0x182c60:89-116
    // default]; phase 0 does not wrap it (0x7000048 is not in the 0x71/0x73 test), so the load
    // goes straight through.
    let wrapped = matches!((fe.flow.level_to_load >> 20) & 0xff, 0x71 | 0x73);
    assert!(!wrapped, "the map load carries no 0x71/0x73 wrap [M 0x1d3800's switch has no 0x7000048]");
    assert!(fe.flow.load_tick(), "the front-end load finishes");
    fe.requested_level = None;   // the host takes the request when it schedules the load
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER]);
    assert_eq!(fe.flow.page, menu::PAGE_RESULTS, "the results page [M 0x1d6360's MenuManager_Create on 0x7000048]");

    // --- next level from the results page: Confirm opens the mission list [M 0x208e78:39-44 —
    // a page change, not a load], then the row pick runs the same push sequence as before. Row 2
    // (Alpine Escape 0x09000005) is another driving handover — row 4 is the next on-foot mission
    // [D sp_level 0x2df2e0, the CAMPAIGN table].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, menu::Page::MissionSelect);
    fe.cursor = 4;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD, ST_FADE_HOLD]);
    for _ in 0..=84 { fe.flow.main_tick(); }
    assert!(!fe.flow.load_tick(), "phase 0: the next briefing");
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD, ST_MOVIE]);
    fe.flow.main_tick(); // the headless briefing pops at once
    assert!(!fe.flow.load_tick(), "phase 2");
    assert!(fe.flow.load_tick(), "phase 3");
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER], "the next level runs in state 2");
    assert_eq!(fe.flow.current_level, 0x0700_0001, "Double Cross — CAMPAIGN[4].3 [D sp_level]");
}

/// PushState's contract details [M 0x1c9f38]: the duration rides the push (f12), 0xb becomes the
/// 0xc wait without touching the fade, and a 7/0xd push is ABORTED while a fade is in flight
/// (Camera_SetFade 0x1bbdd8's busy pair → `beqz $v0` → return, no push).
#[test]
fn push_state_contracts() {
    let mut fe = FrontEnd::new();
    fe.flow.main_tick();          // boot: [1] → [1,2,3] (case 1 pushes 2, Reset_MapLoadSettings 3)
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD]);
    // P_NFMAP's hold: PushState(84.0f, 0xb) → the 0xc wait, no fade [M 0x20b864-70].
    fe.flow.push_state(0xb, 84);
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD, ST_WAIT]);
    assert_eq!(fe.flow.wait_frames, 84, "0x42a00000 = 84.0f into +0x22c [M 0x1c9fbc]");
    assert!(!fe.flow.fade_active, "the 0xb push runs no SetFade [M 0x1c9fbc]");
    for _ in 0..=84 { fe.flow.main_tick(); }
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD], "case 0xc pops at 0 [M 0x1c98c8 case 0xc]");

    // The abort: a second fade push while one is live pushes NOTHING [M 0x1c9f88/0x1c9fa8].
    fe.flow.push_state(7, 60);
    assert_eq!(*fe.flow.stack.last().unwrap(), ST_FADE_HOLD);
    let depth = fe.flow.stack.len();
    fe.flow.push_state(7, 30);
    fe.flow.push_state(0xd, 20);
    assert_eq!(fe.flow.stack.len(), depth, "both pushes aborted while the fade runs [M 0x1bbdd8's guard]");
    assert_eq!(fe.flow.fade_frames, 60, "the live fade keeps its own duration");
    for _ in 0..=60 { fe.flow.main_tick(); }
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD]);
    assert!(!fe.flow.fade_active);
    // After the pop the slot is free again.
    fe.flow.push_state(0xd, 20);
    assert_eq!(*fe.flow.stack.last().unwrap(), menu::ST_FADE_IN, "SP_Update's 20-frame fade-in [M 0x1dbfd8]");
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD, menu::ST_FADE_IN]);
}

/// State 0xe freezes the sim: SkipCodeFrame is set while the fade-in runs and the Main tail skips
/// Game_Run for that frame [M 0x1c98c8 case 0xe + LAB_001c9e28]. States 5 and 10: nothing on
/// NTSC-U pushes them [M the 16 enumerated PushState sites] — 5 is the dead arm of the draw-skip
/// set {1,3,4,5,9}.
#[test]
fn the_fade_in_freezes_the_sim_like_skipcodeframe() {
    let mut fe = FrontEnd::new();
    fe.flow.main_tick();          // boot: [1] → [1,2,3]
    fe.flow.push_state(0xd, 20);
    assert_eq!(*fe.flow.stack.last().unwrap(), menu::ST_FADE_IN);
    assert!(!fe.flow.skip_code, "not set before the state ticks");
    fe.flow.main_tick();
    assert!(fe.flow.skip_code, "case 0xe sets SkipCodeFrame while the fade runs [M]");
    for _ in 1..20 { fe.flow.main_tick(); assert!(fe.flow.skip_code, "every running fade frame skips Game_Run"); }
    fe.flow.main_tick();
    assert!(!fe.flow.skip_code, "the pop frame runs Game_Run again [M SkipCodeFrame = 0]");
    assert_eq!(fe.flow.stack, vec![ST_BOOT, ST_RUNNER, ST_MAPLOAD]);
    // State 5's arm exists only in the draw-skip set; the constant list keeps 9 and 10 pinned.
    assert!(menu::GameFlow::draw_skip_for(5) && menu::GameFlow::draw_skip_for(ST_LAUNCH_DRIVING),
        "states 1/3/4/5/9 skip the draw [M 0x1c98c8's switch]");
    assert!(!menu::GameFlow::draw_skip_for(10));
}
