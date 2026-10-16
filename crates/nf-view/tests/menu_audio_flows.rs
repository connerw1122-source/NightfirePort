//! menu-audio-state: the menu music/sfx state through every front-end flow, driven the way the
//! window drives it — `action()` / `tick()` plus the host-side calls the window makes
//! (`menu_after`'s sfx drain, `movie_tick`'s music calls, the page-enter music hook via the
//! extracted `fe_music_step`, the async-load swap's front-end arm, the pause's `get_or_insert`).
//! No page/cursor/state writes to get anywhere: the FrontEnd only moves by actions and the
//! game's own events (the attract timeout, `mission_ended` — Mission_Update 0x182c60:74's event).
//!
//! Spec: docs/systems/frontend-music.md. The driver model mirrors the IOP surface the host glue
//! plays onto: items per id (KillSFX removes every instance [M IRX 0x1f88 → 0x1ea4]), the
//! resident bank deciding audibility (SB_0 carries 0x1d7, SB_1 carries 0x470, mission banks
//! carry neither [D the SBINFO.SBI/SB probe]), the music-system track and the volume.
//! Invariant on EVERY frame: at most one audible menu-music source, and none during gameplay.
//!
//! Every test carries a sister assertion that can fail (audibility counts, the bank the jingle
//! came from, the gate over a mission).

use nf_view::menu::{aux_movie_for, Action, FrontEnd, Outcome, Page, Stats, ATTRACT_MOVIE_A,
                    ATTRACT_MOVIE_B, ATTRACT_FRAMES, INTRO_MOVIES, ESTHERO_GRACE, ESTHERO_MOVIE, ST_MOVIE};
use nf_view::menu_music::{fe_music_step, MusicCmd};

const FRONTEND_LEVEL: u32 = nf_view::menu::FRONTEND_LEVEL;
/// A mission the menu walk can start (The Exchange — menus_e2e unlocks it through the driving
/// row). Its bank is a mission bank: neither jingle [D the SB_12 probe].
const EXCHANGE: u32 = 0x0700_0005;
/// The simulated decoder's length for one non-looping movie (long enough that a skip lands
/// mid-movie, like a player's).
const MOVIE_FRAMES: u32 = 300;

// ── The driver model ────────────────────────────────────────────────────────────────────────

/// The IOP-side surface: live items per id, the resident bank hash, the music-system track, the
/// music volume. `log` records every applied op in order — the command stream under assertion.
#[derive(Default)]
struct Driver {
    /// The looping jingle instances (the menu-music sources under assertion).
    jingles: Vec<u32>,
    /// Everything else the menus start (the one-shot blips 0x1d8-0x1da) — they die at their own
    /// sample end in the real mixer; the model just counts them.
    others: Vec<u32>,
    bank: u32,
    music_up: Option<i32>,
    music_master: i32,
    fades: usize,
    log: Vec<String>,
}

impl Driver {
    /// The bank hash carries the id [D SBINFO.SBI + SB_0.SFX/SB_1.SFX: 0xc → 0x1d7, 0x38 →
    /// 0x470, neither carries both; every bank carries the blips 0x1d8-0x1da; mission banks
    /// carry neither jingle].
    fn carries(&self, id: u32) -> bool {
        match (self.bank, id) {
            (0xc, 0x1d7) | (0x38, 0x470) => true,
            (_, 0x1d7) | (_, 0x470) => false,
            _ => true, // the blips ride every bank
        }
    }
    fn remove(&mut self, id: u32) {
        self.jingles.retain(|&i| i != id);
        self.others.retain(|&i| i != id);
        self.log.push(format!("rm {id:#x}"));
    }
    fn start(&mut self, id: u32) {
        self.log.push(format!("start {id:#x}"));
        if !self.carries(id) { return; }
        if id == 0x1d7 || id == 0x470 { self.jingles.push(id); } else { self.others.push(id); }
    }
    /// The audible menu-music sources right now.
    fn audible_jingles(&self) -> usize { self.jingles.len() }
    fn starts_of(&self, id: u32, from: usize) -> usize {
        self.log[from..].iter().filter(|l| **l == format!("start {id:#x}")).count()
    }
}

// ── The host harness ────────────────────────────────────────────────────────────────────────

/// The window's movie player stand-in: the ONE BGFMV slot [M psiStartBackgroundMovie 0x1081a8].
struct Player { code: u32, frames: u32, looping: bool, shown: u32 }

struct Host {
    /// The App's `fe`: None while a level runs (the manager is gone [M MenuManager_Delete]).
    fe: Option<FrontEnd>,
    music_page: Option<Page>,
    drv: Driver,
    /// The loaded level is the front end (the StartJingle gate [M lib.rs menu_music_tick]).
    front_end_loaded: bool,
    /// The game's music statics on the App ([M DAT_002e1b11; L MusicMasterVolume 0x30c958]).
    stash: (bool, u32, i32),
    player: Option<Player>,
    /// The sound hold (`Sound::set_paused` — the pause manager's SFXPause [M 0x21b908:121-124]).
    hold: bool,
    pending_level: Option<u32>,
}

impl Host {
    /// run()'s boot: the window boot runs P_INTRO's logos first (`start_boot_intro`) — `boot_window`;
    /// `boot` is the headless --page shape that opens on the title directly. `Sound_Ready`'s case
    /// 0x7000048 consumes the flip [M 0x1dceb8:94-101], then the first redraws run the music hook.
    fn boot() -> Self { Self::boot_with(false) }
    fn boot_window() -> Self { Self::boot_with(true) }
    fn boot_with(intro: bool) -> Self {
        let mut fe = FrontEnd::new();
        if intro { fe.start_boot_intro(); }
        let hash = fe.music.front_end_sound_ready();
        let mut h = Host {
            fe: Some(fe),
            music_page: None,
            drv: Driver { bank: hash, music_master: 70, ..Default::default() },
            front_end_loaded: true,
            stash: (true, hash, 70),
            player: None,
            hold: false,
            pending_level: None,
        };
        assert_eq!(hash, 0xc, "a fresh boot takes bank 0xc [M .bss DAT_002e1b11 = 0]");
        h.frame();
        h
    }

    fn inherit(fe: &mut FrontEnd, stash: (bool, u32, i32)) {
        fe.music.bank_flip = stash.0;
        fe.music.loaded_bank = stash.1;
        fe.music.music_master = stash.2;
    }

    /// One redraw: fe.tick() (the flow machine + the page timers), the movie glue's music calls
    /// at App::movie_tick's sites, then the page-enter music hook + the gated drain
    /// (App::menu_music_tick), then the invariants. With no front end up, only the invariants.
    fn frame(&mut self) {
        if self.fe.is_none() {
            self.check_invariants();
            return;
        }
        let mut player = self.player.take();
        let mut close_now = false;
        {
            let fe = self.fe.as_mut().unwrap();
            fe.tick();
            // movie_tick's top: a pending level request rides the same path a menu pick does
            // [M lib.rs movie_tick — fe.requested_level.take() → the host's load].
            if let Some(code) = fe.requested_level.take() {
                self.pending_level = Some(code);
            }
            // fe-audit: the intro run is a menu background movie (P_INTRO / P_ESTHERO), not
            // GameFlow state 6 [M 0x209e88:62-63, 0x20a060:24-25] — lib.rs movie_tick's intro arm.
            let intro_up = fe.intro_playing;
            let attract_up = fe.attract_playing;
            let state6 = fe.flow.get_state() == ST_MOVIE;
            if intro_up {
                let code = fe.intro_movie_code();
                if fe.intro_movie {
                    let wrong = match &player { Some(p) => p.code != code || p.looping, None => true };
                    if wrong {
                        // [M movie_tick's intro start site: the movie replaces the one slot +
                        // fe.music.movie_started() — the movie's own audio replaces the jingles]
                        player = Some(Player { code, frames: MOVIE_FRAMES, looping: false, shown: 0 });
                        fe.music.movie_started();
                    }
                    // psiMovieFinished [M 0x1082e8]: the end feeds the page's next 0x50; the finish
                    // arm's pair restart is the FrontEnd's own (open_pair_sfx).
                    let p = player.as_mut().unwrap();
                    if p.frames == 0 { fe.intro_movie_finished(); } else { p.frames -= 1; p.shown += 1; fe.intro_live = true; }
                } else if player.as_ref().is_some_and(|p| !p.looping && p.code == code) {
                    player = None; // the skip's psiStopBackgroundMovie, or the movie's end
                } else if player.as_ref().is_some_and(|p| p.looping) {
                    player = None; // the intro pages carry no aux backdrop
                }
                fe.flow.movie_playing = false;
            } else if attract_up {
                let code = fe.attract_movie;
                let wrong = match &player { Some(p) => p.code != code || p.looping, None => true };
                if wrong {
                    // [M P_ATTRACT's 0x4c removes the pair before the trailer (0x20a1f0:20-22) —
                    // the trailer's own soundtrack replaces it; the host's movie_started does the
                    // music]
                    player = Some(Player { code, frames: MOVIE_FRAMES, looping: false, shown: 0 });
                    fe.music.movie_started();
                }
                let mut finished = false;
                if let Some(p) = player.as_mut() {
                    if p.code == code {
                        if p.frames == 0 { finished = true; } else { p.frames -= 1; p.shown += 1; }
                    }
                }
                if finished {
                    // [M the 0x50 finish arm restarts the pair (0x20a1f0:34-39);
                    // attract_movie_finished runs the 0x5f pop back to the opener]
                    fe.music.movie_finished();
                    fe.attract_movie_finished();
                    player = None;
                }
                fe.flow.movie_playing = player.as_ref().is_some_and(|p| p.code == code && p.frames > 0);
            } else if state6 {
                // A briefing/outro owns the slot; a looping page backdrop yields [M 0x1081a8].
                if player.as_ref().is_some_and(|p| p.looping) { player = None; }
                let code = fe.flow.movie_code;
                match &mut player {
                    Some(p) => { if p.frames > 0 { p.frames -= 1; p.shown += 1; } fe.flow.movie_playing = p.frames > 0; }
                    None => {
                        if code != 0 {
                            player = Some(Player { code, frames: MOVIE_FRAMES, looping: false, shown: 0 });
                            fe.flow.movie_playing = true; // BGFMVPlaying = 1 [M 0x1081a8]
                        } else {
                            fe.flow.movie_playing = false;
                        }
                    }
                }
            } else {
                // The skipped-attract arm: a trailer still in the slot restarts the pair the same
                // way the natural end does [M 0x20a1f0:34-39; lib.rs's else arm].
                if player.as_ref().is_some_and(|p| !p.looping && matches!(p.code, ATTRACT_MOVIE_A | ATTRACT_MOVIE_B)) {
                    fe.music.movie_finished();
                    player = None;
                }
                let aux = aux_movie_for(fe.page_id());
                let wrong = match &player { Some(p) => aux == 0 || p.code != aux || !p.looping, None => aux != 0 };
                if wrong {
                    player = None;
                    if aux != 0 { player = Some(Player { code: aux, frames: u32::MAX, looping: true, shown: 0 }); }
                }
                fe.flow.movie_playing = false;
            }
            // The phase-2 resolve [M 0x1d6360 LoadState 2]: the briefing's return loads the real
            // level and closes the front end; the outro's return lands on the front-end map with
            // the results page still up (flow.page = 0x40000036 [M 0x1d6360]).
            if let Some(resolved) = fe.flow.phase2_resolved.take() {
                self.pending_level = Some(resolved);
                if resolved != FRONTEND_LEVEL { close_now = true; }
            }
        }
        self.player = player;
        if close_now { self.close_front_end(); }
        self.apply_music_step();
        self.check_invariants();
    }

    /// App::menu_music_tick: the extracted transition step + the gated application.
    fn apply_music_step(&mut self) {
        if self.fe.is_none() { return; }
        let front_end_loaded = self.front_end_loaded;
        let cmds = {
            let fe = self.fe.as_mut().unwrap();
            fe_music_step(fe, &mut self.music_page)
        };
        for c in cmds {
            match c {
                MusicCmd::StopJingles => { self.drv.remove(0x1d7); self.drv.remove(0x470); }
                MusicCmd::StartJingle(id) => {
                    if front_end_loaded { self.drv.start(id); }
                    else { self.drv.log.push(format!("gated start {id:#x}")); }
                }
                MusicCmd::StartMusic(t, _s) => { self.drv.music_up = Some(t); self.drv.log.push(format!("music {t:#x}")); }
                MusicCmd::StopMusic => { self.drv.music_up = None; self.drv.log.push("stopmusic".into()); }
                MusicCmd::MusicVolume(v) => { self.drv.music_master = v; self.drv.log.push(format!("vol {v}")); }
                MusicCmd::FadeDownMusic => { self.drv.fades += 1; self.drv.log.push("fadedown".into()); }
            }
        }
    }

    fn check_invariants(&self) {
        assert!(self.drv.audible_jingles() <= 1,
            "two menu-music sources at once: {:?}", self.drv.log);
        assert!(!(self.drv.audible_jingles() >= 1 && self.drv.music_up.is_some()),
            "the jingles and the credits track sound together: {:?}", self.drv.log);
        if !self.front_end_loaded {
            assert_eq!(self.drv.audible_jingles(), 0,
                "menu music during gameplay: {:?}", self.drv.jingles);
        }
    }

    /// App::menu_action: the state-6 intercept for briefings/outros (only the skip matters, a
    /// second into the movie [S port choice]; the intro run is NOT state 6 and reaches fe.action,
    /// which honours the grace), then fe.action + menu_after (the sfx drain: removes BEFORE the same frame's starts [M
    /// 0x20a408:17-22]), the level request, the close. The pause's Back resumes.
    fn input(&mut self, a: Action) -> bool {
        let Some(fe) = self.fe.as_mut() else { return false };
        if fe.flow.get_state() == ST_MOVIE {
            let shown = self.player.as_ref().map_or(u32::MAX, |p| p.shown);
            if a == Action::Confirm && shown >= 60 {
                fe.flow.skip_movie();
                drop(fe);
                self.frame();
                return true;
            }
            return false;
        }
        if fe.page == Page::Pause && a == Action::Back {
            // The resume [S: the game's resume is its own row; Back resumes too] — the manager is
            // deleted (MenuManager_Delete unpause [M 0x21c2b0:11-15]); no music messages.
            self.fe = None;
            self.hold = false;
            self.music_page = None;
            self.frame();
            return false;
        }
        let r = fe.action(a);
        let rm: Vec<u32> = fe.drain_sfx_removes().collect();
        for id in rm { self.drv.remove(id); }
        let starts: Vec<u32> = fe.drain_sfx().map(|ev| ev.id).collect();
        for id in starts { self.drv.start(id); }
        if let Some(code) = fe.requested_level.take() { self.pending_level = Some(code); }
        let closed = fe.closed;
        drop(fe);
        if closed { self.close_front_end(); }
        self.frame();
        r
    }

    /// close_front_end [M menu-audio-leaks]: the pair's removes through the driver, the stash
    /// (the .bss statics outlive the manager [M 0x1dceb8]), the manager gone.
    fn close_front_end(&mut self) {
        if let Some(fe) = self.fe.as_mut() {
            let rm: Vec<u32> = fe.drain_sfx_removes().collect();
            for id in rm { self.drv.remove(id); }
            for id in [0x1d7u32, 0x470] { self.drv.remove(id); }
            self.stash = (fe.music.bank_flip, fe.music.loaded_bank, fe.music.music_master);
        }
        self.fe = None;
        self.music_page = None;
        self.player = None;
        self.check_invariants();
    }

    /// poll_async_load's swap: the scene lands and the sound stack is rebuilt fresh [M
    /// swap_in_scene's build_sound]. A level drops the manager (the SP pick's / the MP start's
    /// MenuManager_Delete); the front-end map keeps it with the bank flip [M 0x1dceb8 case
    /// 0x7000048] and the page-enter re-run [M 0x21b908:125's menu_movie reset].
    fn load(&mut self, next: u32) {
        self.pending_level = None;
        self.front_end_loaded = next == FRONTEND_LEVEL || next == 0;
        let mission_bank = !self.front_end_loaded;
        self.drv = Driver {
            bank: if mission_bank { 0x16 } else { self.stash.1 },
            music_master: self.drv.music_master,
            ..Default::default()
        };
        self.hold = false;
        self.player = None;
        if mission_bank {
            // The manager is gone [M poll_async_load's fe = None arm] — the statics stash.
            if let Some(fe) = self.fe.take() {
                self.stash = (fe.music.bank_flip, fe.music.loaded_bank, fe.music.music_master);
            }
            self.music_page = None;
        } else if let Some(fe) = self.fe.as_mut() {
            let hash = fe.music.front_end_sound_ready();
            fe.music.jingles_up = false;
            self.drv.bank = hash;
            self.music_page = None;
            assert!(hash == 0xc || hash == 0x38, "a front-end load takes a front-end bank");
        }
        self.frame();
    }

    /// The pause over a running mission (the window's Esc [M Player_Update 0x193600:73 opens
    /// the pause page]): the fresh manager inherits the statics, the hold comes up.
    fn open_pause(&mut self) {
        let level = 0x0700_0005u32;
        let fe = self.fe.get_or_insert_with(FrontEnd::new);
        Self::inherit(fe, self.stash);
        fe.flow.current_level = level;
        assert!(fe.open_pause(), "the pause opens over a mission");
        drop(fe);
        self.hold = true;
        self.music_page = None;
        self.frame();
    }

    /// The demo results hook (F8/F9 [S] standing for Mission_Update 0x182c60:74): the results
    /// page opens over the STILL-LOADED mission — the gate's stage.
    fn mission_end(&mut self, complete: bool) {
        let mut fe = FrontEnd::new();
        Self::inherit(&mut fe, self.stash);
        fe.flow.stack = vec![nf_view::menu::ST_RUNNER];
        fe.flow.current_level = EXCHANGE;
        fe.mission_ended(if complete { Outcome::Complete } else { Outcome::Failed }, Stats::default());
        self.fe = Some(fe);
        self.music_page = None;
        self.frame();
    }

    fn jingles(&self) -> usize { self.drv.audible_jingles() }
}

// ── Walk helpers (actions only) ─────────────────────────────────────────────────────────────

/// One action, then the frames a player would wait (menus_e2e's act, over the host frames).
fn act(h: &mut Host, a: Action) -> bool {
    let r = h.input(a);
    for _ in 0..60 { h.frame(); }
    for _ in 0..5000 { if h.fe.as_ref().is_none_or(|fe| !fe.intro_playing && !fe.attract_playing) { break; } h.frame(); }
    r
}

/// START → P_PARISENUM → P_ESTHERO's 0x073B0048 → Main when it ends [M 0x20a648:43, 0x20a060:16-45]
/// (act runs the movie out).
fn to_main(h: &mut Host) {
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Start));
    assert!(act(h, Action::Confirm), "the title's Enter");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Main), "after P_ESTHERO's movie, the main menu [M 0x20a060:44-45]");
}

fn to_missions(h: &mut Host) {
    to_main(h);
    assert!(act(h, Action::Confirm), "NightFire");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Codename && fe.cursor == 1),
        "the codename page opens on Play without saving [M 0x20aec0:96-97]");
    assert!(act(h, Action::Confirm), "Play without saving's confirm box");
    assert!(act(h, Action::Confirm), "the box's yes");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Difficulty));
    assert!(act(h, Action::Confirm), "the difficulty");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::MissionSelect));
}

/// The Exchange from the mission list (row 0 is the driving Paris Prelude — menus_e2e), through
/// the briefing to gameplay.
fn start_exchange(h: &mut Host) {
    // The list opens on the highest unlocked row — The Exchange on a fresh profile [M 0x20b608:60-64].
    assert!(h.fe.as_ref().is_some_and(|fe| fe.cursor == 1));
    assert!(act(h, Action::Confirm), "The Exchange starts");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.requested_level.is_none()), "the request was consumed into the load");
    // The briefing movie wraps the load [M 0x1d3800's 0x71 wrap]; the pair must stay DOWN from
    // the pick to gameplay: no restart page comes up, and the briefing owns the audio.
    for _ in 0..600 {
        if h.pending_level.is_some() { break; }
        h.frame();
    }
    assert_eq!(h.jingles(), 0, "no menu music once a level was picked");
    if h.fe.as_ref().is_some_and(|fe| fe.flow.get_state() == ST_MOVIE) {
        for _ in 0..61 { h.frame(); }
        assert!(h.input(Action::Confirm), "the briefing skip");
    }
    let mut loaded = false;
    for _ in 0..600 {
        if let Some(next) = h.pending_level.take() { h.load(next); loaded = true; break; }
        h.frame();
    }
    assert!(loaded, "the level loaded");
    assert!(!h.front_end_loaded);
    assert!(h.fe.is_none(), "the manager is gone for a level");
}

// ── The flows ───────────────────────────────────────────────────────────────────────────────

/// The window's fresh boot (`start_boot_intro`): P_INTRO plays the EA then the MGM logo with the
/// pair removed under each, UNSKIPPABLE (DAT_002e1c49 = 0), the first movie's finish restarts the
/// pair and the second movie's start removes it again on the same 0x50, and the second's finish
/// restarts it for the title — whose own 0x4c then sends nothing [M P_INTRO_Handler 0x209e88:16-62;
/// Menu_StopMovie 0x215a50:6; P_START 0x20a408:17-23].
#[test]
fn the_boot_logos_play_unskippable_and_hand_the_music_to_the_title() {
    let mut h = Host::boot_window();
    for _ in 0..10 { h.frame(); }
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Intro && fe.intro_movie));
    assert_eq!(h.player.as_ref().map(|p| p.code), Some(INTRO_MOVIES[0]), "the EA logo first [M 0x209e88:46]");
    assert_eq!(h.jingles(), 0, "no menu music under the EA logo [M 0x209e88:42-44]");
    // Unskippable: Confirm past any grace does nothing [M 0x209e88:41; 0x215a50:6].
    for _ in 0..ESTHERO_GRACE { h.frame(); }
    assert!(!h.input(Action::Confirm), "the boot logos refuse the skip");
    assert_eq!(h.player.as_ref().map(|p| p.code), Some(INTRO_MOVIES[0]), "sister: still the EA logo");
    // The EA logo ends → MGM; the between-movies restart is removed by MGM's start on the same 0x50.
    let mark = h.drv.log.len();
    for _ in 0..(MOVIE_FRAMES + 10) { h.frame(); if h.player.as_ref().is_some_and(|p| p.code == INTRO_MOVIES[1]) { break; } }
    assert_eq!(h.player.as_ref().map(|p| p.code), Some(INTRO_MOVIES[1]), "MGM second [M 0x209e88:60]");
    assert!(h.drv.starts_of(0x1d7, mark) >= 1, "the EA logo's finish restarted the pair [M 0x209e88:23-28]");
    assert_eq!(h.jingles(), 0, "MGM's start removed it again [M 0x209e88:56-58]");
    assert!(!h.input(Action::Confirm), "MGM is unskippable too [M 0x209e88:55]");
    // MGM ends → the title, with the pair up.
    for _ in 0..(MOVIE_FRAMES + 10) { h.frame(); if h.fe.as_ref().is_some_and(|fe| !fe.intro_playing) { break; } }
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Start), "the title 0x40000009 [M 0x209e88:50]");
    assert_eq!(h.jingles(), 1, "the title has its music after the logos");
    assert_eq!(h.drv.jingles, vec![0x1d7], "bank 0xc's jingle — the boot's one front-end load");
    // Sister: the title's 0x4c on the already-up pair sends nothing (menu_movie is 1).
    let mark = h.drv.log.len();
    for _ in 0..3 { h.frame(); }
    assert_eq!(h.drv.starts_of(0x1d7, mark), 0, "no second jingle start on the title's enter");
}

/// START's intro: P_ESTHERO's movie silences the pair [M 0x20a060:20-22] and its finish restarts it
/// for Main [M :34-40]. The skip is refused through the 0xb4-tick grace [M :17; 0x215a50:6] — the
/// window path reaches the same fe.action (lib.rs's state-6 intercept no longer swallows it).
#[test]
fn a_skipped_intro_still_leaves_the_main_menu_music_up() {
    let mut h = Host::boot();
    assert_eq!(h.jingles(), 1, "the title's pair");
    assert!(h.input(Action::Confirm), "the title's Enter starts the intro");
    for _ in 0..3 { h.frame(); }
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::EstHero && fe.intro_movie));
    assert_eq!(h.player.as_ref().map(|p| p.code), Some(ESTHERO_MOVIE));
    assert_eq!(h.jingles(), 0, "no menu music under the 0x073B intro");
    // 61 frames in (the old 1 s skip rule): the grace still holds — the skip is refused.
    for _ in 0..61 { h.frame(); }
    assert!(!h.input(Action::Confirm), "a skip inside the 180-tick grace is refused [M 0x215a50:6]");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.intro_movie && fe.page == Page::EstHero), "the intro keeps playing");
    // Past the grace the skip stops the movie; the page's next 0x50 takes the finish arm.
    for _ in 0..ESTHERO_GRACE { h.frame(); }
    assert!(h.input(Action::Confirm), "the skip past the grace");
    for _ in 0..3 { h.frame(); }
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Main && !fe.intro_playing));
    assert!(h.player.is_none() || h.player.as_ref().is_some_and(|p| p.looping), "the stopped intro left the slot");
    assert_eq!(h.jingles(), 1, "the main menu is NOT silent after a skipped intro [M 0x20a060:34-40]");
    assert_eq!(h.drv.jingles, vec![0x1d7]);
}

#[test]
fn the_attract_trailer_takes_over_and_hands_back() {
    let mut h = Host::boot();
    to_main(&mut h);
    assert_eq!(h.jingles(), 1);
    // Idle to the attract timeout [M P_MAIN 0x20a7b0's 0xa8c fire].
    for _ in 0..(ATTRACT_FRAMES + 120) { h.frame(); if h.fe.as_ref().is_some_and(|fe| fe.attract_playing) { break; } }
    assert!(h.fe.as_ref().is_some_and(|fe| fe.attract_playing), "the attract fires after the idle");
    assert_eq!(h.jingles(), 0, "the trailer's own soundtrack replaces the jingles [M 0x20a1f0:20-22]");
    assert!(h.drv.log.iter().any(|l| l == "rm 0x1d7"), "the pair was REMOVED, not left under the trailer");
    // The natural end: the 0x50 arm restarts the pair and the opener page returns [M :45-54].
    for _ in 0..(MOVIE_FRAMES + 120) { h.frame(); if h.fe.as_ref().is_some_and(|fe| !fe.attract_playing) { break; } }
    assert!(h.fe.as_ref().is_some_and(|fe| !fe.attract_playing));
    assert_eq!(h.jingles(), 1, "the pair is back after the trailer");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Main), "back on the opener");
    // Sister: a SKIPPED attract restarts the pair the same way (the skip funnels through the
    // same 0x50 path — the game's StopMovie [M 0x21c3d8:37]).
    for _ in 0..(ATTRACT_FRAMES + 120) { h.frame(); if h.fe.as_ref().is_some_and(|fe| fe.attract_playing) { break; } }
    assert!(h.fe.as_ref().is_some_and(|fe| fe.attract_playing));
    assert!(h.input(Action::Confirm), "the skip ends the attract");
    for _ in 0..120 { h.frame(); }
    assert!(h.fe.as_ref().is_some_and(|fe| !fe.attract_playing));
    assert_eq!(h.jingles(), 1, "the skipped trailer's pair restart");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Main));
}

#[test]
fn the_backdrop_pages_keep_one_pair_and_the_list_pages_touch_nothing() {
    let mut h = Host::boot();
    // to_missions walks boot → Main → the list; the jingle instance count never leaves 1 [M
    // the group-C handlers: no music calls on Codename/Difficulty/Dossier/Records/Options and
    // the MP chain — only the page blips sound].
    to_missions(&mut h);
    assert_eq!(h.jingles(), 1, "the pair survived the list-page walk");
    assert_eq!(h.drv.jingles, vec![0x1d7]);
    let starts = h.drv.starts_of(0x1d7, 0);
    // The mission list IS a backdrop page: leaving and coming back while the pair is up must
    // restart NOTHING (the menu_movie gate [M 0x20b608's if menu_movie == 0]).
    assert!(act(&mut h, Action::Back));
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Difficulty));
    assert!(act(&mut h, Action::Confirm));
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::MissionSelect));
    assert_eq!(starts, h.drv.starts_of(0x1d7, 0), "no restart while the pair is up");
    assert_eq!(h.jingles(), 1);
}

#[test]
fn the_credits_swap_to_track_0x27_and_back() {
    let mut h = Host::boot();
    to_main(&mut h);
    // The credits open through the game's own open (open_credits — the AV Options' Credits row's
    // page change [M P_CREDITS 0x2152b0's 0x4c: the SWAP — the pair removed, track 0x27, the
    // volume saved]).
    let saved = h.drv.music_master;
    assert!(h.fe.as_ref().is_some_and(|fe| fe.music.music_master == saved));
    {
        let fe = h.fe.as_mut().unwrap();
        assert!(fe.open_credits(nf_view::menu::PAGE_MAIN, fe.music.music_master, false),
            "the credits open from the main menu");
    }
    for _ in 0..10 { h.frame(); }
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Credits));
    assert_eq!(h.jingles(), 0, "the jingles are removed on the credits enter");
    assert_eq!(h.drv.music_up, Some(0x27), "the credits' own music-system track");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.music.credits_saved_vol == Some(saved)),
        "the entry volume is saved");
    // Sister: the leave restores the ENTRY volume (not the slider's last value) and restarts the
    // pair [M 0x2152b0:54-65].
    {
        let fe = h.fe.as_mut().unwrap();
        fe.music.music_slider(3); // the AV slider mid-visit [M 0x214818's 0x50 arm]
    }
    h.frame();
    assert_eq!(h.drv.music_master, 15);
    // The credits' own delayed exit routes the page back [M :103-195] — run the roll out.
    let mut left = false;
    for _ in 0..20000 { h.frame(); if h.fe.as_ref().is_some_and(|fe| fe.page != Page::Credits) { left = true; break; } }
    assert!(left, "the credits roll ends and leaves the page");
    assert_eq!(h.drv.music_up, None, "SFXStopMusic on the leave");
    assert_eq!(h.drv.music_master, saved, "the ENTRY volume, not the slider's last value");
    assert_eq!(h.jingles(), 1, "the pair always comes back after the credits");
    // And never both at once through the whole swap (checked every frame above).
}

#[test]
fn an_sp_mission_start_stops_the_menu_music_for_the_level() {
    let mut h = Host::boot();
    to_missions(&mut h);
    start_exchange(&mut h);
    assert!(h.drv.jingles.is_empty() && h.drv.others.is_empty(), "the fresh sound stack carries nothing of the menus");
    // The results page over the STILL-LOADED mission: its 0x4c restart queues the restart [M
    // 0x208e78:60-65] and the gate mutes it — the mission bank resolves neither jingle
    // [D SB_12]. [S port: the game opens no page over the mission on completion — case 4 sets
    // +0x1c = 0x40000036 and loads (0x182c60:72-75); the port shows the page during the load.]
    h.mission_end(true);
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Results));
    assert_eq!(h.jingles(), 0, "the results page's restart is silent over a mission");
    assert!(h.drv.log.iter().any(|l| l.starts_with("gated start")),
        "the command mirrors the game's call, the gate mutes it");
    // The mission end loads the front end (no outro for The Exchange): the bank flips [M
    // 0x1dceb8] and the page re-enters with the OTHER jingle, audibly now [M 0x21b908:125].
    let mut got_fe = false;
    for _ in 0..600 {
        if let Some(next) = h.pending_level.take() {
            assert_eq!(next, FRONTEND_LEVEL, "a no-outro mission returns to the front end");
            h.load(next);
            got_fe = true;
            break;
        }
        h.frame();
    }
    assert!(got_fe, "the front end reloads");
    assert!(h.front_end_loaded);
    assert_eq!(h.fe.as_ref().map(|fe| fe.music.loaded_bank), Some(0x38),
        "the second front-end load flips to 0x38 — the statics outlived the mission");
    for _ in 0..10 { h.frame(); }
    assert_eq!(h.jingles(), 1);
    assert_eq!(h.drv.jingles, vec![0x470], "SB_1's jingle sounds now [D SB_1.SFX]");
}

#[test]
fn the_pause_holds_the_sound_and_never_touches_the_pair() {
    let mut h = Host::boot();
    to_missions(&mut h);
    start_exchange(&mut h);
    assert!(!h.front_end_loaded, "in the mission");
    // Esc: the pause opens over the mission — the hold comes up and NO music commands fire (the
    // game's pause manager never starts or removes the pair [M 0x21b908:121-125; P_PAUSE
    // 0x203c28 has no pair calls]).
    let mark = h.drv.log.len();
    h.open_pause();
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::Pause));
    assert!(h.hold, "the sound hold (SFXPause + SFXPauseAllStreams)");
    assert!(!h.drv.log[mark..].iter().any(|l| *l == "start 0x1d7" || *l == "start 0x470"),
        "the pause sounds no jingle (the fresh manager's boot bookkeeping stays muted over the mission bank)");
    assert_eq!(h.jingles(), 0);
    // Resume (Back): the hold lifts; still no music during gameplay [M MenuManager_Delete's
    // unpause; the next pair restart waits for a front-end page].
    h.input(Action::Back);
    assert!(!h.hold);
    assert!(h.fe.is_none(), "the manager is deleted on the resume");
    assert_eq!(h.jingles(), 0, "no menu music returns on the resume");
    // Quit to the front end: the fade-down first [M C_GCPAUSE 0x204408:150], the SAME manager
    // walks to the mission list (+0x1c = 0x4000001c [M 0x204408:141-152]), the load flips the
    // bank, and P_NFMAP's re-entered 0x4c restarts with the OTHER jingle [M 0x21b908:125's reset +
    // 0x20b608:36-42].
    h.open_pause();
    let fades_before = h.drv.fades;
    assert!(h.input(Action::Down), "Resume → Restart");
    assert!(h.input(Action::Down), "Restart → Quit");
    assert!(h.input(Action::Confirm), "the quit row arms the YES/NO list");
    assert!(h.input(Action::Up), "NO → YES [M 0x204408's 0 = YES]");
    assert!(h.input(Action::Confirm), "the quit's YES");
    assert!(h.drv.fades > fades_before, "the quit ran the fade-down [M 0x204408:150]");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::MissionSelect), "the quit reopens on the mission list [M 0x204408:141-152]");
    // The front-end map loads over the SAME manager.
    let mut got_fe = false;
    for _ in 0..600 {
        if let Some(next) = h.pending_level.take() {
            assert_eq!(next, FRONTEND_LEVEL, "the quit loads the front-end map");
            h.load(next);
            got_fe = true;
            break;
        }
        h.frame();
    }
    assert!(got_fe, "the front-end load was requested");
    assert!(h.front_end_loaded);
    assert_eq!(h.fe.as_ref().map(|fe| fe.music.loaded_bank), Some(0x38),
        "the statics outlived the pause's fresh manager [M 0x1dceb8's .bss DAT]");
    for _ in 0..10 { h.frame(); }
    assert_eq!(h.drv.jingles, vec![0x470], "the quit-to-front-end plays the OTHER jingle");
    assert_eq!(h.jingles(), 1);
}

#[test]
fn the_mp_setup_keeps_the_pair_and_the_gate_mutes_the_debriefing_over_the_arena() {
    let mut h = Host::boot();
    to_main(&mut h);
    // Main → Multiplayer → Local → the setup chain to the confirm page (menus_e2e's walk). The
    // whole chain sends no music messages [M §3 group C].
    assert!(act(&mut h, Action::Down), "Multiplayer");
    assert!(act(&mut h, Action::Confirm), "the mode page");
    assert!(act(&mut h, Action::Confirm), "Local");
    assert!(act(&mut h, Action::Confirm), "join");
    assert!(act(&mut h, Action::Confirm), "codename");
    assert!(act(&mut h, Action::Confirm), "ready");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::MpScenario));
    let starts_before = h.drv.starts_of(0x1d7, 0) + h.drv.starts_of(0x470, 0);
    // The scenario page opens on the row of MPSettings+0x1A4 = 1 (boot) — Arena [M 0x217348:25;
    // 0x1c7e78:30].
    assert!(h.fe.as_ref().is_some_and(|fe| fe.mp_scenario_row == 1));
    assert!(act(&mut h, Action::Confirm), "the scenario");
    assert!(act(&mut h, Action::Confirm), "the map");
    assert!(act(&mut h, Action::Confirm), "the character");
    assert!(act(&mut h, Action::Confirm), "the handicap");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::MpOptions));
    assert!(act(&mut h, Action::Down), "the AI Bots row");
    assert!(act(&mut h, Action::Confirm));
    assert!(act(&mut h, Action::Confirm), "bot 1");
    assert!(act(&mut h, Action::Confirm), "the bot's character");
    assert!(act(&mut h, Action::Confirm), "back to the bots list");
    assert!(act(&mut h, Action::Back));
    assert!(act(&mut h, Action::Up), "back on Continue");
    assert!(act(&mut h, Action::Confirm), "Continue with a bot");
    assert!(h.fe.as_ref().is_some_and(|fe| fe.page == Page::MpConfirm));
    let starts_between = h.drv.starts_of(0x1d7, 0) + h.drv.starts_of(0x470, 0);
    assert_eq!(starts_before, starts_between, "the MP setup chain restarts nothing [M group C]");
    assert_eq!(h.jingles(), 1, "the pair plays over the whole MP setup");
    assert!(act(&mut h, Action::Confirm), "start the match");
    // The arena loads: the manager goes, the fresh stack carries nothing, the gate holds.
    for _ in 0..600 {
        if let Some(next) = h.pending_level.take() { h.load(next); break; }
        h.frame();
    }
    assert!(!h.front_end_loaded, "the arena is a level");
    assert!(h.fe.is_none());
    assert_eq!(h.jingles(), 0, "no menu music in the match");
    // Sister: the debriefing page's 0x4c queues the restart [M P_MPDEBRIEFING 0x212198:85-90] —
    // and over the STILL-LOADED arena the host's gate mutes it, exactly like the results page
    // over a mission. (The MP match-end → debriefing host glue is not built yet — the page's
    // own 0x4c is what the port owns.)
    let cmds = {
        let mut fe = FrontEnd::new();
        Host::inherit(&mut fe, h.stash);
        fe.music.page_enter(Page::MpDebriefing);
        fe.music.take_cmds()
    };
    assert_eq!(cmds, vec![MusicCmd::StopJingles, MusicCmd::StartJingle(0x1d7)],
        "the debriefing's enter queues the restart with the CURRENT bank's jingle");
    let mark = h.drv.log.len();
    for c in &cmds {
        match c {
            MusicCmd::StopJingles => { h.drv.remove(0x1d7); h.drv.remove(0x470); }
            MusicCmd::StartJingle(id) => {
                if h.front_end_loaded { h.drv.start(*id); } else { h.drv.log.push(format!("gated start {id:#x}")); }
            }
            _ => {}
        }
    }
    assert_eq!(h.drv.audible_jingles(), 0, "no debriefing music over the arena");
    assert!(h.drv.log.len() > mark);
}

#[test]
fn the_statics_survive_every_manager_drop_the_game_way() {
    // The game's DAT_002e1b11 flips on every front-end LOAD, not per manager [M 0x1dceb8 case
    // 0x7000048 — a .bss byte; Boot_LoadPTPData 0x1c8560:61 carries it across boots]. The
    // stash/restore must hand the SAME state to a fresh manager.
    let h = Host::boot();
    assert_eq!(h.stash, (true, 0xc, 70), "after the boot's Sound_Ready: flipped, bank 0xc, volume 70 [L 0x30c958]");
    // A fresh manager (the pause's get_or_insert_with) continues the alternation [M the static
    // is not per-manager].
    let mut fe = FrontEnd::new();
    Host::inherit(&mut fe, h.stash);
    assert_eq!(fe.music.front_end_sound_ready(), 0x38, "the second load flips to 0x38");
    assert_eq!(fe.music.jingle_id(), 0x470, "SB_1's jingle [D SB_1.SFX]");
    assert_eq!(fe.music.front_end_sound_ready(), 0xc, "the third flips back");
    // Sister: the volume rides the stash too — a slider move in the menus outlives the manager.
    let mut fe2 = FrontEnd::new();
    fe2.music.music_slider(20);
    let moved = (fe2.music.bank_flip, fe2.music.loaded_bank, fe2.music.music_master);
    let mut fe3 = FrontEnd::new();
    Host::inherit(&mut fe3, moved);
    assert_eq!(fe3.music.music_master, 100, "the moved volume is the next manager's entry volume");
}
