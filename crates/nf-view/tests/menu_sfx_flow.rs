//! menu-sfx: the front end's sound events assert the game's numbers only — the MENU_SOUND values
//! and the bank SFX ids from docs/systems/menu-sfx.md. Nothing here reads the disc (no ISO gate)
//! and nothing opens an audio device (the FrontEnd only builds the event list; the host plays it).

use nf_view::menu::{self, Action, FrontEnd, MenuSfx, MENU_SOUND_BACK, MENU_SOUND_FOCUS, MENU_SOUND_PAGE_OPEN,
                    MENU_SOUND_SELECT, MENU_SOUND_SIDE_CIRCLE, MENU_SOUND_VAL_DEC, MENU_SOUND_VAL_INC,
                    SFX_MENU_OPEN_MOVE, SFX_MENU_OPEN_SELECT, SFX_MENU_TICK, SFX_MENU_VAL_DEC, SFX_MENU_VAL_INC};

fn drain(fe: &mut FrontEnd) -> Vec<MenuSfx> {
    fe.drain_sfx().collect()
}

/// `FrontEnd::new()` boots onto P_START: the handlers' msg-0x4c pair `SFXStart(0x1d7)+SFXStart(0x470)`
/// plays with `menu_movie` clear [M P_START 0x20a408:12-22], and the static then blocks a replay
/// until a movie-end clears it [M P_INTRO 0x209e88:44,58].
#[test]
fn boot_plays_the_open_pair_once() {
    let mut fe = FrontEnd::new();
    let ev = drain(&mut fe);
    assert_eq!(ev, vec![
        MenuSfx { sound: MENU_SOUND_PAGE_OPEN, id: SFX_MENU_OPEN_MOVE },
        MenuSfx { sound: MENU_SOUND_PAGE_OPEN, id: SFX_MENU_OPEN_SELECT },
    ], "the boot pair is 0x1d7 then 0x470, both marked as the handlers' SFXStart path");
    // Sister: the static is set — a second open of a pair page replays nothing.
    assert!(fe.action(Action::Confirm), "Start confirms into the next page");
    assert_ne!(fe.page, menu::Page::Start, "the cursor left Start");
    assert!(drain(&mut fe).iter().all(|e| e.sound != MENU_SOUND_PAGE_OPEN),
        "menu_movie blocks a second pair until an intro-movie end clears it");
}

/// MENU_SOUND 7 → 0x1da on every handled cursor move; the top row's Up moves nothing and the
/// engine's Page_Update only sounds on a real focus change [M 0x21edc8:38-53].
#[test]
fn cursor_move_ticks_and_dead_moves_are_silent() {
    let mut fe = FrontEnd::new();
    drain(&mut fe);
    fe.page = menu::Page::Main;
    fe.cursor = 0;
    assert!(fe.action(Action::Down), "row 1 moves down");
    assert_eq!(drain(&mut fe), vec![MenuSfx { sound: MENU_SOUND_FOCUS, id: SFX_MENU_TICK }],
        "the focus tick is MENU_SOUND 7 → 0x1da");
    // The main wheel wraps [M the arm's % rows()] — the wrap is still a focus change, it ticks too.
    fe.cursor = 0;
    assert!(fe.action(Action::Up), "the wheel wraps");
    assert_eq!(drain(&mut fe), vec![MenuSfx { sound: MENU_SOUND_FOCUS, id: SFX_MENU_TICK }],
        "a wrapped move is still a focus change");
    // Sister: a guarded page refuses the move outright — no focus change, no sound.
    fe.page = menu::Page::CnMenu;
    fe.cursor = 0;
    assert!(!fe.action(Action::Up), "the top row's Up is refused on CnMenu");
    assert!(drain(&mut fe).is_empty(), "a refused move emits nothing");
}

/// MENU_SOUND 0 → 0x1da, gated by enabled_sound_flags bit 1 [M 0x1f3278 case 0; L 0x30d184 = 0x0b].
#[test]
fn confirm_blips_and_the_gate_silences_it() {
    let mut fe = FrontEnd::new();
    drain(&mut fe);
    fe.page = menu::Page::Main;
    fe.cursor = 0;
    assert!(fe.action(Action::Confirm), "the row confirms");
    assert_eq!(drain(&mut fe), vec![MenuSfx { sound: MENU_SOUND_SELECT, id: SFX_MENU_TICK }],
        "the ✕ select blip is MENU_SOUND 0 → 0x1da");
    // Sister: clear bit 1 — the same input starts nothing.
    let mut fe2 = FrontEnd::new();
    drain(&mut fe2);
    fe2.page = menu::Page::Main;
    fe2.cursor = 0;
    fe2.enabled_sound_flags = 0x0b & !1;
    assert!(fe2.action(Action::Confirm));
    assert!(drain(&mut fe2).is_empty(), "MENU_SOUND 0's gate drops the event");
}

/// MENU_SOUND 8 → 0x1d9: the △ back with no 0x5b script on the focus [M 0x21c3d8:78-105],
/// gated by enabled_sound_flags bit 8 [M 0x1f3278 case 8].
#[test]
fn back_buzzes_and_the_gate_silences_it() {
    let mut fe = FrontEnd::new();
    drain(&mut fe);
    fe.page = menu::Page::CnSelect;
    fe.cursor = 1;
    assert!(fe.action(Action::Back), "the sub-page pops");
    assert_eq!(drain(&mut fe), vec![MenuSfx { sound: MENU_SOUND_BACK, id: SFX_MENU_VAL_INC }],
        "the △ buzz is MENU_SOUND 8 → 0x1d9");
    // Sister: clear bit 8 — silent.
    let mut fe2 = FrontEnd::new();
    drain(&mut fe2);
    fe2.page = menu::Page::CnSelect;
    fe2.cursor = 1;
    fe2.enabled_sound_flags = 0x03;
    assert!(fe2.action(Action::Back));
    assert!(drain(&mut fe2).is_empty(), "MENU_SOUND 8's gate drops the event");
}

/// MENU_SOUND 2 (○) is disabled in the boot flags 0x0b [L 0x30d184] — the disc's ○ is silent;
/// with bit 4 set it blips 0x1da like the other side action [M 0x1f3278 case 2].
#[test]
fn side_is_silent_at_the_boot_flags() {
    let mut fe = FrontEnd::new();
    drain(&mut fe);
    fe.page = menu::Page::CnSelect;
    fe.profiles = vec!["A".into(), "B".into()]; // rows 2+ are the saved cards [M C_SBCNSELECT]
    fe.cursor = 2;
    assert!(fe.action(Action::Side), "the ~X side action arms the delete confirm");
    assert!(drain(&mut fe).is_empty(), "0x0b clears bit 4: ○ starts nothing on the disc");
    // Sister: set bit 4 — the same input blips.
    let mut fe2 = FrontEnd::new();
    drain(&mut fe2);
    fe2.page = menu::Page::CnSelect;
    fe2.profiles = vec!["A".into(), "B".into()];
    fe2.cursor = 2;
    fe2.enabled_sound_flags |= 4;
    assert!(fe2.action(Action::Side));
    assert_eq!(drain(&mut fe2), vec![MenuSfx { sound: MENU_SOUND_SIDE_CIRCLE, id: SFX_MENU_TICK }],
        "MENU_SOUND 2 → 0x1da with the bit set");
}

/// The spinner steps: MENU_SOUND 3 → 0x1d8 decrement, 4 → 0x1d9 increment, first-press ids; a
/// value that doesn't move (a slider at its bound, an Action row) starts nothing [M Scroll_Update
/// 0x221b20:186-208 — the min/max guards skip the play].
#[test]
fn value_steps_play_direction_ids_and_stuck_rows_stay_silent() {
    let mut fe = FrontEnd::new();
    drain(&mut fe);
    fe.page = menu::Page::CnAv; // AV_ROWS row 0 = the music slider 0x1000_0135 [L the page's rows]
    fe.cursor = 0;
    assert!(fe.action(Action::Right), "the slider steps up");
    assert_eq!(drain(&mut fe), vec![MenuSfx { sound: MENU_SOUND_VAL_INC, id: SFX_MENU_VAL_INC }],
        "the ▶ increment is MENU_SOUND 4 → 0x1d9");
    assert!(fe.action(Action::Left), "the slider steps down");
    assert_eq!(drain(&mut fe), vec![MenuSfx { sound: MENU_SOUND_VAL_DEC, id: SFX_MENU_VAL_DEC }],
        "the ◀ decrement is MENU_SOUND 3 → 0x1d8");
    // Sister 1: the slider pinned at 100 — Right changes nothing, no sound.
    fe.opts.insert(0x1000_0135, 100);
    assert!(fe.action(Action::Right), "the input is consumed at the bound");
    assert!(drain(&mut fe).is_empty(), "a clamped slider is silent [M the max guard]");
    // Sister 2: an Action row has no value to step.
    fe.cursor = 6; // AV_ROWS row 6 = Screen Adjust, OptKind::Action
    assert!(fe.action(Action::Left), "the input reaches the action row");
    assert!(drain(&mut fe).is_empty(), "a value-less row is silent");
}

/// The Screen Adjust bars select nothing [L page 0x40000047's controls — two slider bars, the
/// picture and the hint; no label rows to blip].
#[test]
fn screen_adjust_selects_nothing() {
    let mut fe = FrontEnd::new();
    drain(&mut fe);
    fe.page = menu::Page::ScreenAdjust;
    assert!(!fe.action(Action::Side), "the bars take no side action");
    // The Confirm arm opens the page's own flow; the sound stays empty either way.
    let _ = fe.action(Action::Confirm);
    assert!(drain(&mut fe).is_empty(), "no select blip on the adjust bars");
}

/// The letter grid's focus tick fires only when the cell changes [M Page_Update sounds on a
/// focus change; a clamped edge move keeps the cell]. The SECRET_GRID arms drive it through
/// `action()`; `cn_move` (the profile-name grid) through the direct path.
#[test]
fn grid_moves_tick_only_on_change() {
    let mut fe = FrontEnd::new();
    drain(&mut fe);
    fe.page = menu::Page::CnName;
    fe.secret.cursor = 0;
    // Sister: the top rows' Up is refused (the grid walks rows of 8) — no tick.
    assert!(!fe.action(Action::Up), "the top row's Up is refused");
    assert!(drain(&mut fe).is_empty(), "a refused grid move emits nothing");
    assert!(fe.action(Action::Right), "the cell moves right");
    assert_eq!(drain(&mut fe), vec![MenuSfx { sound: MENU_SOUND_FOCUS, id: SFX_MENU_TICK }],
        "the grid's focus tick is MENU_SOUND 7 → 0x1da");
    // The cn_move path: a clamped edge move keeps the cell and stays silent.
    fe.cn_cell = menu::FrontEnd::CN_GRID[0][0];
    fe.cn_move(0, -1);
    assert!(drain(&mut fe).is_empty(), "the clamped move kept the cell");
    fe.cn_move(1, 0);
    assert_eq!(drain(&mut fe), vec![MenuSfx { sound: MENU_SOUND_FOCUS, id: SFX_MENU_TICK }],
        "the profile grid's tick is the same MENU_SOUND 7 → 0x1da");
}

/// The credits' 0x4c is the SWAP, not a pair page: `SFXRemove(0,0x1d7); SFXRemove(0,0x470);
/// menu_movie = 0; SFXStartMusic(0x27,0)` [M 0x2152b0:90-93 — the decomp; the pair restart is
/// the LEAVE arm's, :58-63]. menu-audio-state corrected enter_page(Credits) to this.
#[test]
fn credits_open_swaps_to_the_track_and_clears_the_pair_static() {
    let mut fe = FrontEnd::new();
    drain(&mut fe);
    fe.page = menu::Page::CnAv;
    fe.menu_movie = true; // the pair is up on the options pages [M §3 group C]
    assert!(fe.open_credits(nf_view::credits::PAGE_CN_AV, 100, false), "the credits open");
    // No pair starts on the enter — the swap removes the pair through the music channel
    // (MenuMusic::page_enter(Credits) queues StopJingles + StartMusic(0x27)).
    let ev = drain(&mut fe);
    assert!(ev.is_empty(), "the credits 0x4c starts no pair [M 0x2152b0:90-93]");
    assert!(!fe.menu_movie, "the static clears on the swap [M 0x2152b0:92]");
    // The music state mirrors the swap: jingles down, the credits track up, the volume saved.
    assert!(!fe.music.jingles_up);
    assert!(fe.music.credits_music);
    assert_eq!(fe.music.credits_saved_vol, Some(100), "the entry volume is saved [M 0x2152b0:88]");
}


// ── menu-audio-leaks: the pair's SFXRemove side ─────────────────────────────────────────────

/// The 0x4c arm removes the pair before it restarts [M 0x20a408:17-22] — the boot open emits
/// the removes too (no-ops on an empty item list, exactly the IOP's), and every restart clears
/// the old loop first.
#[test]
fn pair_opens_carry_remove_events_before_the_starts() {
    let mut fe = FrontEnd::new();
    let rm: Vec<u32> = fe.drain_sfx_removes().collect();
    assert_eq!(rm, vec![SFX_MENU_OPEN_MOVE, SFX_MENU_OPEN_SELECT],
        "the boot 0x4c removes both ids [M 0x20a408:17-18]");
    assert_eq!(drain(&mut fe).len(), 2, "then starts the pair");
    assert!(fe.menu_movie, "the static is set [M 0x20a408:22]");
}

/// The intro start removes the pair and starts nothing — the menu music is silent under the EA
/// logo [M P_INTRO's 0x50 frame-1 0x209e88:37-47 removes the pair and clears the static; the
/// restart only comes at the movie-2 end :23-29].
#[test]
fn the_intro_start_silences_the_pair() {
    let mut fe = FrontEnd::new();
    let _ = drain(&mut fe);
    let _ = fe.drain_sfx_removes().collect::<Vec<u32>>();
    assert!(fe.action(Action::Confirm), "Start confirms into the intro");
    let rm: Vec<u32> = fe.drain_sfx_removes().collect();
    assert_eq!(rm, vec![SFX_MENU_OPEN_MOVE, SFX_MENU_OPEN_SELECT],
        "P_INTRO's frame-1 removes the pair [M 0x209e88:38-39]");
    assert!(!fe.menu_movie, "the static is cleared [M 0x209e88:44]");
    assert!(drain(&mut fe).iter().all(|e| e.sound != MENU_SOUND_PAGE_OPEN),
        "no pair restart under the movies [M — the restart rides the movie-2 end]");
    assert_eq!(fe.page, menu::Page::Main, "the page flip keeps menus_flow's contract");
}

/// The movie-2 end restarts the pair [M 0x209e88:23-29]: remove, then SFXStart both.
#[test]
fn the_intro_end_restarts_the_pair_with_a_remove() {
    let mut fe = FrontEnd::new();
    let _ = drain(&mut fe);
    let _ = fe.drain_sfx_removes().collect::<Vec<u32>>();
    assert!(fe.action(Action::Confirm));
    let _ = fe.drain_sfx_removes().collect::<Vec<u32>>();
    let budget = fe.flow.movie_frames + 4;
    for _ in 0..budget {
        fe.tick();
        if !fe.intro_playing { break; }
    }
    assert!(!fe.intro_playing, "the intro popped within {} ticks", budget);
    let rm: Vec<u32> = fe.drain_sfx_removes().collect();
    assert_eq!(rm, vec![SFX_MENU_OPEN_MOVE, SFX_MENU_OPEN_SELECT],
        "the movie-2 end removes before it restarts [M 0x209e88:24-25]");
    let opens: Vec<MenuSfx> = drain(&mut fe).into_iter().filter(|e| e.sound == MENU_SOUND_PAGE_OPEN).collect();
    assert_eq!(opens, vec![
        MenuSfx { sound: MENU_SOUND_PAGE_OPEN, id: SFX_MENU_OPEN_MOVE },
        MenuSfx { sound: MENU_SOUND_PAGE_OPEN, id: SFX_MENU_OPEN_SELECT },
    ], "the pair restarts [M 0x209e88:26-27]");
    assert!(fe.menu_movie, "the static stands again [M 0x209e88:28]");
}

/// The attract movie end removes+restarts the pair [M P_ATTRACT 0x20a1f0:45-50].
#[test]
fn the_attract_end_restarts_the_pair() {
    let mut fe = FrontEnd::new();
    let _ = drain(&mut fe);
    let _ = fe.drain_sfx_removes().collect::<Vec<u32>>();
    fe.attract_playing = true;
    fe.attract_movie = 0x073a_0048;
    fe.attract_movie_finished();
    let rm: Vec<u32> = fe.drain_sfx_removes().collect();
    assert_eq!(rm, vec![SFX_MENU_OPEN_MOVE, SFX_MENU_OPEN_SELECT],
        "the 0x50 removes the pair [M 0x20a1f0:45-46]");
    assert_eq!(drain(&mut fe).len(), 2, "then restarts it [M 0x20a1f0:47-50]");
}

/// `MenuManager_Create` zeroes the static on every manager open [M 0x21b908:125] — the pause
/// page's 0x4c starts no pair and removes none (P_PAUSE 0x203c28 has no pair calls
/// [M-negative]); the mission bank lacks the pair ids anyway [D SB_4/SB_5.SFX].
#[test]
fn open_pause_clears_menu_movie_and_starts_no_pair() {
    let mut fe = FrontEnd::new();
    let _ = drain(&mut fe);
    let _ = fe.drain_sfx_removes().collect::<Vec<u32>>();
    fe.flow.current_level = 0x0700_0001;
    assert!(fe.open_pause(), "the pause opens over a mission");
    assert!(!fe.menu_movie, "the manager open cleared the static [M 0x21b908:125]");
    assert!(drain(&mut fe).iter().all(|e| e.sound != MENU_SOUND_PAGE_OPEN),
        "P_PAUSE starts no pair [M-negative 0x203c28]");
    assert!(fe.drain_sfx_removes().collect::<Vec<u32>>().is_empty(),
        "and removes none");
}
