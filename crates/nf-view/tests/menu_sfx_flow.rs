//! menu-sfx: the front end's sound events assert the game's numbers only — the MENU_SOUND values
//! and the bank SFX ids from docs/systems/menu-sfx.md. Nothing here reads the disc (no ISO gate)
//! and nothing opens an audio device (the FrontEnd only builds the event list; the host plays it).

use nf_view::menu::{self, Action, FrontEnd, MenuSfx, Page, MENU_SOUND_BACK, MENU_SOUND_FOCUS, MENU_SOUND_PAGE_OPEN,
                    MENU_SOUND_SELECT, MENU_SOUND_SIDE_CIRCLE, MENU_SOUND_VAL_DEC, MENU_SOUND_VAL_INC,
                    SFX_MENU_OPEN_MOVE, SFX_MENU_OPEN_SELECT, SFX_MENU_TICK, SFX_MENU_VAL_DEC, SFX_MENU_VAL_INC};

fn drain(fe: &mut FrontEnd) -> Vec<MenuSfx> {
    fe.drain_sfx().collect()
}

/// `FrontEnd::new()` boots onto P_START: the handlers' msg-0x4c pair `SFXStart(0x1d7)+SFXStart(0x470)`
/// plays with `menu_movie` clear [M P_START 0x20a408:12-22], and the static then blocks a replay
/// until an intro movie's START clears it [M P_INTRO 0x209e88:44, :58; P_ESTHERO 0x20a060:22].
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
        "menu_movie blocks a second pair until an intro movie's start clears it");
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
fn credits_open_swaps_the_pair_for_the_credits_track() {
    let mut fe = FrontEnd::new();
    drain(&mut fe);
    fe.page = menu::Page::CnAv;
    fe.menu_movie = false; // an intro/attract movie START cleared it [M 0x209e88:44, :58; 0x20a1f0:20-22]
    assert!(fe.open_credits(nf_view::credits::PAGE_CN_AV, 100, false), "the credits open");
    // menu-audio-state (672): the credits' 0x4c is the music SWAP — the pair is removed,
    // `menu_movie = 0` and `SFXStartMusic(0x27, 0)` plays the credits track [M P_CREDITS
    // 0x2152b0:89-94]; the pair is NOT restarted (it used to start for under a frame).
    let ev = drain(&mut fe);
    assert!(ev.iter().all(|e| e.id != SFX_MENU_OPEN_MOVE && e.id != SFX_MENU_OPEN_SELECT),
        "the credits open starts no menu-music pair: {ev:?}");
    assert!(!fe.menu_movie, "the static stays clear while the credits track plays [M 0x2152b0]");
}

/// menu-audio-state's variant: the pair static UP before the open (the options pages' state) is
/// cleared by the swap [M 0x2152b0:92].
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
    // (The music state's half of the swap — jingles down, the track up, the entry volume
    // saved — is the page-enter hook's, asserted end to end in menu_audio_flows's
    // the_credits_swap_to_track_0x27_and_back.)
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

fn wait_for_page(fe: &mut FrontEnd, page: menu::Page, budget: u32) -> bool {
    for _ in 0..budget { fe.tick(); if fe.page == page { return true; } }
    false
}

/// START's intro start removes the pair and starts nothing — the menu music is silent under the
/// 0x073B intro [M P_ESTHERO 0x20a060:16-24: its 2nd 0x50 removes the pair and clears the static
/// before psiStartBackgroundMovie; the restart only comes at the movie's end :34-40].
#[test]
fn the_intro_start_silences_the_pair() {
    let mut fe = FrontEnd::new();
    let _ = drain(&mut fe);
    let _ = fe.drain_sfx_removes().collect::<Vec<u32>>();
    assert!(fe.action(Action::Confirm), "START confirms into the intro run");
    // P_PARISENUM holds for the card machine's walk (G-048(4) — HandlePS2CardState 0x110b98's
    // no-card path takes the GetInfo + GetDir wait), then P_ESTHERO's 2nd 0x50 starts the movie.
    assert!(wait_for_page(&mut fe, menu::Page::EstHero, 32), "P_ESTHERO opens after the card walk");
    for _ in 0..3 { fe.tick(); }
    assert!(fe.intro_movie);
    let rm: Vec<u32> = fe.drain_sfx_removes().collect();
    assert_eq!(rm, vec![SFX_MENU_OPEN_MOVE, SFX_MENU_OPEN_SELECT],
        "P_ESTHERO's movie start removes the pair [M 0x20a060:20-21]");
    assert!(!fe.menu_movie, "the static is cleared [M 0x20a060:22]");
    assert!(drain(&mut fe).iter().all(|e| e.sound != MENU_SOUND_PAGE_OPEN),
        "no pair restart under the movie [M — the restart rides its end, 0x20a060:34-40]");
    assert_eq!(fe.page, menu::Page::EstHero, "the up-page is P_ESTHERO during the movie [M 0x20a060:44-45]");
}

/// The intro movie's end restarts the pair [M P_ESTHERO 0x20a060:34-40]: remove, then SFXStart both,
/// then Main opens.
#[test]
fn the_intro_end_restarts_the_pair_with_a_remove() {
    let mut fe = FrontEnd::new();
    let _ = drain(&mut fe);
    let _ = fe.drain_sfx_removes().collect::<Vec<u32>>();
    assert!(fe.action(Action::Confirm));
    assert!(wait_for_page(&mut fe, menu::Page::EstHero, 32),
        "P_ESTHERO opens after the card walk [G-048(4): the 0x44 waits on DAT_002e1b0a, M 0x20a648:39-47]");
    for _ in 0..3 { fe.tick(); }
    let _ = fe.drain_sfx_removes().collect::<Vec<u32>>();
    let budget = fe.intro_frames + 4;
    for _ in 0..budget {
        fe.tick();
        if !fe.intro_playing { break; }
    }
    assert!(!fe.intro_playing, "the intro ended within {} ticks", budget);
    assert_eq!(fe.page, menu::Page::Main);
    let rm: Vec<u32> = fe.drain_sfx_removes().collect();
    assert_eq!(rm, vec![SFX_MENU_OPEN_MOVE, SFX_MENU_OPEN_SELECT],
        "the movie end removes before it restarts [M 0x20a060:35-36]");
    let opens: Vec<MenuSfx> = drain(&mut fe).into_iter().filter(|e| e.sound == MENU_SOUND_PAGE_OPEN).collect();
    assert_eq!(opens, vec![
        MenuSfx { sound: MENU_SOUND_PAGE_OPEN, id: SFX_MENU_OPEN_MOVE },
        MenuSfx { sound: MENU_SOUND_PAGE_OPEN, id: SFX_MENU_OPEN_SELECT },
    ], "the pair restarts once [M 0x20a060:37-38]");
    assert!(fe.menu_movie, "the static stands again [M 0x20a060:39]");
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

/// menu-music-stack: the page-open pair never reaches the driver through the sfx channel — the music
/// channel alone starts the loaded bank's one jingle (the user's "multiple songs" playtests).
#[test]
fn the_jingle_pair_is_not_started_twice() {
    use nf_view::menu::{SFX_MENU_OPEN_MOVE, SFX_MENU_OPEN_SELECT};
    use nf_view::menu_music::menu_sfx_reaches_driver;
    assert!(!menu_sfx_reaches_driver(SFX_MENU_OPEN_MOVE));
    assert!(!menu_sfx_reaches_driver(SFX_MENU_OPEN_SELECT));
    // Sister: the ordinary menu blips still play.
    assert!(menu_sfx_reaches_driver(0x1d8));
}

// ── G-048(2): the per-page sound flags ────────────────────────────────────────────────────────

/// `Menu_EnableSounds` 0x1f31f0's switch [M]: every page open re-keys `enabled_sound_flags`
/// [M 0x21c858:268], so Back on Main is SILENT (0x40000002 → 1) while Back on the codename
/// select buzzes (0x4000001b → 9, the default arm). The boot value is P_LANGUAGE's default-arm 9
/// [M the manager-create 0x43's direct 0x44, 0x21c858:252] — the ee_start 0x0b is the mission
/// list's value, not a boot one.
#[test]
fn page_opens_rekey_the_sound_flags() {
    assert_eq!(menu::menu_enable_sounds(0x4000_0002), 1, "Main [M 0x1f31f0 case 0x40000002]");
    assert_eq!(menu::menu_enable_sounds(0x4000_0009), 1, "the title [M 0x1f31f0 case 0x40000009]");
    assert_eq!(menu::menu_enable_sounds(0x4000_001c), 0xb, "the SP list [M case 0x4000001c]");
    assert_eq!(menu::menu_enable_sounds(0x4000_004b), 0xb, "pause/debriefing [M case 0x4000004b]");
    assert_eq!(menu::menu_enable_sounds(0x4000_0036), 7, "the results page [M case 0x40000036]");
    assert_eq!(menu::menu_enable_sounds(0x4000_0037), 8, "stats [M case 0x40000037]");
    assert_eq!(menu::menu_enable_sounds(0x4000_003a), 8, "records [M case 0x4000003a]");
    assert_eq!(menu::menu_enable_sounds(0x4000_0034), 9, "P_LANGUAGE takes the default arm [M 0x1f31f0]");
    // The walk: the 0x44 applies them [M 0x21c858:268] — the port's funnel is enter_page, driven
    // here through the load tail's reopen (the MenuManager_Create stand-in [M 0x1d6360:160-163]).
    let mut fe = FrontEnd::new();
    assert_eq!(fe.enabled_sound_flags, 9, "the boot value is P_LANGUAGE's default [M 0x21c858:252]");
    fe.flow.page = menu::PAGE_MAIN;
    assert!(fe.reopen_queued_page());
    assert_eq!(fe.enabled_sound_flags, 1, "Main's open re-keys the flags [M 0x21c858:268]");
    // Sister: Back on Main starts NOTHING (bit 3 clear) — the old global 0x0b buzzed here.
    fe.cursor = 0;
    assert!(fe.action(Action::Back), "the page pops");
    assert!(drain(&mut fe).iter().all(|e| e.sound != MENU_SOUND_BACK),
        "Back on Main is silent [M 0x1f31f0 case 0x40000002 → 1; 0x1f3278 case 8's & 8]");
    // …and on a default-arm page it buzzes (the sister that can fail).
    let mut fe2 = FrontEnd::new();
    fe2.flow.page = menu::PAGE_CN_SELECT;
    assert!(fe2.reopen_queued_page());
    assert_eq!(fe2.enabled_sound_flags, 9);
    fe2.cursor = 1;
    assert!(fe2.action(Action::Back));
    assert!(drain(&mut fe2).iter().any(|e| e.sound == MENU_SOUND_BACK),
        "Back on a default-arm page buzzes [M 0x1f31f0 default → 9]");
}

// ── G-048(1): the CloseIris delayed page change ───────────────────────────────────────────────

/// `Menu_ChangePageCloseIris` 0x202190: the difficulty confirm opens the SP list 10 ticks later
/// [M 0x216ca8:16 → 0x202190:8-9], input blocked from the tick after the call [M :7] until the
/// 0x44's own 0x68/0 lifts it [M 0x21c858:315].
#[test]
fn the_difficulty_confirm_rides_the_close_iris_delay() {
    let mut fe = FrontEnd::new();
    fe.flow.page = menu::PAGE_DIFFICULTY;
    assert!(fe.reopen_queued_page());
    fe.cursor = 0;
    assert!(fe.action(Action::Confirm), "the difficulty confirms");
    assert_eq!(fe.page, Page::Difficulty, "the page does NOT flip at once [M 0x202190 — the 0x44 is 10 ticks out]");
    assert_eq!(fe.delayed_page.map(|(n, p, _)| (n, p)), Some((menu::CLOSE_IRIS_PAGE_TICKS, Page::MissionSelect)));
    fe.tick();
    assert_eq!(fe.input_block_ticks, u32::MAX, "the delayed 0x68/1 lands the next tick [M 0x202190:7]");
    // The block holds until the 0x44 arrives: an action inside the window is refused.
    for _ in 0..menu::CLOSE_IRIS_PAGE_TICKS - 2 { fe.tick(); }
    assert_eq!(fe.delayed_page.map(|(n, _, _)| n), Some(1));
    assert!(!fe.action(Action::Confirm), "the dispatcher skips while the block holds [M 0x21c3d8:70]");
    fe.tick();
    assert_eq!(fe.page, Page::MissionSelect, "the 0x44 lands [M 0x202190:8-9]");
    assert_eq!(fe.input_block_ticks, 0, "the arrival lifts the block [M 0x21c858:315]");
    assert!(fe.close_iris_in_flight() == false, "the iris window closed");
    assert_eq!(fe.cursor, 1, "the list opens on the highest unlocked row [M 0x20b608:60-64 — a fresh profile's rows 0-1, D sp_level boot]");
    // Sister that can fail: without the delay the page would have flipped on the action tick —
    // the assert at the top of this test pins the old behaviour was WRONG.
}

// ── G-048(4): the boot card check ─────────────────────────────────────────────────────────────

/// P_PARISENUM's box lasts the card machine's no-card walk [M 0x20a648:36-47 + HandlePS2CardState
/// 0x110b98 + Menu_EnumSaves 0x1f7b78 + PS2_DoCardFlow 0x217bd8:68], not one tick.
#[test]
fn the_paris_card_check_walks_the_machine_before_esthero() {
    let mut fe = FrontEnd::new();
    let _ = drain(&mut fe);
    assert!(fe.action(Action::Confirm), "START → P_PARISENUM");
    assert_eq!(fe.page, Page::ParisEnum);
    // The box holds: the machine walks GET_SAVE_GAMES → GET_CARD_STATUS → the sync waiter
    // [M 0x110b98's stack walk] — the first ticks do NOT open P_ESTHERO.
    fe.tick();
    assert_eq!(fe.page, Page::ParisEnum, "the box waits on the card machine (the old 1-tick [S] is gone)");
    // The 0x50's first message-box pump armed the enum: psiStartSaveEnum pushed GET_SAVE_GAMES
    // with GettingSaveGames = -1 [M 0x110fe0:8-15; Menu_EnumSaves state 0 → 1, 0x1f7b78:58-68].
    assert_eq!((fe.paris_card.enum_state, fe.paris_card.getting_saves), (1, -1));
    let mut ticks = 1u32;
    while fe.page == Page::ParisEnum && ticks < 64 { fe.tick(); ticks += 1; }
    assert_eq!(fe.page, Page::EstHero, "the 0x44 fires when the flow completes [M 0x20a648:39-47]");
    assert!(ticks > 2, "the walk takes more than the old single tick ({} ticks)", ticks);
    // The machine's own walk pins the state order [M 0x110b98/0x10e798/0x110188], armed the way
    // the 0x4c's message-box pump arms it [M 0x1f7de8:24,53-58 → 0x1f7b78:58-68 → 0x110fe0]:
    let mut m = menu::ParisCardCheck::default();
    assert_eq!(m.enum_step(), 1, "the first pump arms: psiStartSaveEnum pushed GET_SAVE_GAMES");
    // One tick runs the loop until an sceMc op pends [M 0x110b98:16-21's do-while]: 0xe's sub 1
    // pushes GET_CARD_STATUS [M 0x110188:27-29], whose sub 0 starts the GetInfo and pushes the
    // sync waiter [M 0x10e798:10-16].
    m.tick();
    assert_eq!(m.stack[m.sp as usize], menu::CARD_ST_SYNC, "the waiter is on top after tick 1");
    assert_eq!(m.sub[0], 1, "0xe's sub advanced [M 0x110188:22-24]");
    // Tick 2: the GetInfo landed — the waiter pops, GET_CARD_STATUS pops, 0xe advances past the
    // one-codename skip [M 0x110188:31-33] to the GetDir step [M :37] and pends again.
    m.tick();
    assert_eq!(m.stack[m.sp as usize], menu::CARD_ST_GET_SAVE_GAMES, "drained back to 0xe");
    assert_eq!(m.sub[0], 3, "the sub walked 1→2→3 [M 0x110188:31-43]");
    assert_eq!(m.flow_msg, 0xb, "the GetDir step shows the 0xb message state [M 0x110188:37]");
    assert!(!m.flow_done());
    // Tick 3: the GetDir lands [S mc_ticks] — sub 4 pops with the sync result [M :46-49].
    m.tick();
    assert_eq!(m.sp, 0xff, "the machine drained [M 0x110188:46-49]");
    assert_eq!(m.next_save(), 0, "psiGetNextSave returns 0 [M 0x111038:12-14]");
    assert_eq!(m.enum_step(), 7, "the enum completes [M 0x1f7b78:27]");
    assert!(m.flow_done(), "DoCardFlow's completion flag lands [M 0x217bd8:68]");
}
