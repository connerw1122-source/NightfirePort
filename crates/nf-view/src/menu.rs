//! The front end (fix-menus): the game's GameFlow state machine and the menu screens.
//!
//! Evidence base (all [M] unless noted — ACTION.ELF decompile under `work/decomp/ACTION.ELF/`):
//! - `GameFlow_GetState` 0x1c9e80 / `GameFlow_PopState` 0x1c9eb0 / `GameFlow_PushState` 0x1c9f38 /
//!   `GameFlow_Main` 0x1c98c8 — the GameState stack (`GameStateStack` 0x2c7398, `StackIndex` 0x30cece,
//!   64 u32 entries, wraps at 0x40). Push converts 7 → 8 (fade hold, after `Camera_SetFade(6, t, 1)`)
//!   and 0xb → 0xc (a frame-count wait stored at DAT_002d88c8+0x22c). States {1, 3, 4, 5, 9} set
//!   GameState+0x40 = 1 (skip Game_Draw); {2, 6, 7, 8, 10, 0xc, 0xd, 0xe} clear it.
//! - `bootup_bootup` 0x1c7e78 — boot zeroes GameState, sets +0x20 = 0x80000002, +0x28 = 0x40000034
//!   (the P_START page), +0x24 = 1 at :68 ("load requested"), +0x40 = 2.
//! - `ResetMap_LevelToLoad` 0x1d3800 — sets `IsWarmReset`; when the current state is {2, 7, 8, 0xd, 0xe}
//!   it clears GameState+0x40/+0x54, maps story codes 0x0700000X → 0x7100000X when the third arg is 0,
//!   stores the level at GameState+0x8 and pushes state 3 (driving levels push 9 instead).
//! - `ResetMap_Load` 0x1d6360 — a 4-phase loader: phase 0 loads the loading screen, and when the level
//!   still carries the 0x71/0x73 prefix, records the movie code (GameState+0x10 = 0x60009xx), pushes
//!   state 6 and returns; phase 2 maps the movie code back (0x7100001 → 0x07000001, ...), stores the
//!   **current level at GameState+0x12**, and for 0x7000048 sets +0x28 = 0x40000036; phase 3 builds the
//!   world and, when +0x12 == 0x7000048, calls `MenuManager_Create(GameState+0x20, GameState+0x28, …)`;
//!   then pops and pushes state 4 (`Locks_Init` 0x1b0548 then pops itself).
//! - `Mission_Update` 0x182c60:74 — mission end sets GameState+0x28 = 0x40000036 (the P_NFRESULTS page)
//!   and pushes a 60-frame fade (0x42700000 = 60.0 f32).
//! - `P_ENDMISSION_Handler` 0x216618 — the results screen's three choices: 1 = restart
//!   (`ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)`), 0 = continue
//!   (`ResetMap_LevelToLoad(GameState+0x12, 1, 1)` + `Player_RamLoad(glb_players+0xe0, 1)`),
//!   2 = quit to menu (GameState+0x28 = 0x4000001c, `ResetMap_LevelToLoad(0x7000048, 0, 0)`).
//! - `P_START_Handler` 0x20a408 — the title page: the legal line is `Txt_BindLabel(0x375)`, the
//!   "Press Start" label is 0x10001eb flashing on a 0xf0 = 240-frame cycle, and after
//!   `Menu_GetNoInputCount() == 0xa8c` (2700 frames = 45 s) it sends 0x44/0x40000035 (attract mode).
//! - `P_PAUSE_Handler` 0x203c28 — the pause page builds eight option rows from Txt labels
//!   0x82, 0x91, 0xa0, 0xaf, 0xbe, 0xcd, 0x100018b, 0x100019a.
//! - `P_MAIN_Handler` 0x20a7b0 — the main menu page also uses the 240-frame attract timer and the
//!   background-movie SFX pair (0x1d7, 0x470).
//! - Page table (ACTION.ELF `.data`, entries {handler fn, size, flags 0x12, 1, name offset}, 16 B each,
//!   found by probing for `P_START_Handler` 0x20a408; id = 0x40000000 | index) — page ids used here:
//!   0x4000000c P_PAUSE 0x203c28, 0x40000026 P_ENDMISSION 0x216618, 0x4000002e P_MAIN 0x20a7b0,
//!   0x40000034 P_START 0x20a408, 0x40000036 P_NFRESULTS 0x208e78, 0x40000038 P_NFSTATS 0x209938,
//!   0x40000039 P_NFSELECT 0x20aec0, 0x4000003b P_NFMAP 0x20b608, 0x40000047 P_NFDFCTY 0x216bf8,
//!   0x40000048 C_SBNFDFCTY 0x216ca8, 0x40000068 P_CNOPTIONS 0x213f08.
//! - `MissionData` 0x2a4350 (docs/formats/mission-data.md): the campaign is 9 missions; mission *n*'s
//!   first level is the select list the game offers.
//!
//! What is *suggested* (marked [S] at the use site): the exact row order inside the main/pause menus
//! (the disc's page data wires the buttons, which this port does not parse), and the difficulty
//! choices' names (the standard Agent/Operative/007 set; the `difficulty` global is 0x2df4c0 [M
//! symtab]).

use crate::gpu::{HudFrame, HudVertex};
use crate::hud::{Hud, Anchor};
use nf_formats::hud::Lang;

/// The game's screen [M hud.rs SW/SH: Viewer+0x10C/+0x110, 512 × 448 shown at 4:3].
const SW: f32 = 512.0;
const SH: f32 = 448.0;
/// A colour 0xRRGGBB on the GS scale with an alpha byte [M hud.rs `gs`].
fn gs(rgb: u32, a: u32) -> [f32; 4] {
    [((rgb >> 16) & 0xff) as f32 / 128.0, ((rgb >> 8) & 0xff) as f32 / 128.0, (rgb & 0xff) as f32 / 128.0, ((a.min(255) >> 1) as f32 / 128.0).min(1.0)]
}

// ---------------------------------------------------------------------------
// GameFlow — the GameState stack machine [M 0x1c9e80/0x1c9eb0/0x1c9f38/0x1c98c8]
// ---------------------------------------------------------------------------

pub const ST_BOOT: u8 = 1;
/// Runs whatever GameState+0x12 holds: the front end map 0x7000048, or a mission [M 0x1d6360].
pub const ST_RUNNER: u8 = 2;
pub const ST_MAPLOAD: u8 = 3;
pub const ST_LOCKS: u8 = 4;
pub const ST_MOVIE: u8 = 6;
pub const ST_FADE_HOLD: u8 = 8;
pub const ST_LAUNCH_DRIVING: u8 = 9;
/// Nothing on NTSC-U pushes 10 — every push site is enumerated in the decompile and none has 10
/// [M: `grep -rh "GameFlow_PushState" work/decomp/ACTION.ELF`]. Kept as a constant for completeness.
pub const ST_GAME_UNUSED: u8 = 10;
pub const ST_WAIT: u8 = 0xc;
pub const ST_FADE_IN: u8 = 0xe;

/// The front-end map — the menus run "on" this level [M 0x1d6360: `GameState._8_4_ != 0x7000048`].
pub const FRONTEND_LEVEL: u32 = 0x0700_0048;
/// The results page Mission_Update opens on the front-end map [M 0x182c60:74]. Which page the quit
/// path reopens: P_ENDMISSION's quit writes 0x4000001c (index 0x1c = 28 in the page table) [M 0x216618].
pub const PAGE_START: u32 = 0x4000_0034;
pub const PAGE_MAIN: u32 = 0x4000_002e;
pub const PAGE_MISSION_SELECT: u32 = 0x4000_0039;
pub const PAGE_DIFFICULTY: u32 = 0x4000_0047;
pub const PAGE_OPTIONS: u32 = 0x4000_0068;
pub const PAGE_PAUSE: u32 = 0x4000_000c;
pub const PAGE_RESULTS: u32 = 0x4000_0036;
pub const PAGE_RESULTS_QUIT_TARGET: u32 = 0x4000_001c;

/// The GameState fields the flow machine reads and writes [M addresses in the doc comment].
#[derive(Clone, Debug, Default)]
pub struct GameFlow {
    /// GameStateStack [M 0x2c7398] — the game keeps 64 u32s and wraps; ours grows.
    pub stack: Vec<u8>,
    /// GameState+0x08: the level the loader should fetch [M 0x1d3800/0x1d6360].
    pub level_to_load: u32,
    /// GameState+0x10: the intro-movie code while a 0x71-wrapped level loads [M 0x1d6360].
    pub movie_code: u32,
    /// GameState+0x12: the current level [M 0x1d6360 phase 2].
    pub current_level: u32,
    /// GameState+0x28: the page `MenuManager_Create` opens on the front-end map [M 0x1c7e78/0x1d6360/0x216618].
    pub page: u32,
    /// GameState+0x24: "a load is pending" — boot sets it to 1 [M 0x1c7e78:68], ResetMap_GameInit clears it.
    pub load_pending: u32,
    /// GameState+0x40: skip Game_Draw this frame [M 0x1c98c8].
    pub skip_draw: bool,
    /// GameState+0x60: the frame counter [M 0x1c98c8 prologue].
    pub ticks: u32,
    /// IsWarmReset [M 0x1d3800 prologue].
    pub warm_reset: bool,
    /// Set while a fade (state 8/0xe) is in flight; the game waits on the fade camera's flag
    /// DAT_002d88c8+0x235 [M 0x1c98c8 cases 8/0xe]. Headless port: a frame countdown.
    pub fade_frames: u32,
    /// The state-0xc frame wait [M 0x1c9f38 stores the arg at DAT_002d88c8+0x22c; 0x1c98c8 case 0xc counts down].
    pub wait_frames: u32,
}

impl GameFlow {
    /// Boot: stack = [ST_BOOT], the boot page 0x40000034 [M 0x1c7e78]. `Reset_MapLoadSettings`
    /// 0x1d6958 sets GameState+0x8 = 0x7000048 when it is 0 and calls `ResetMap_LevelToLoad(level, 0, 0)`
    /// — the boot load of the front-end map.
    pub fn boot() -> Self {
        GameFlow { stack: vec![ST_BOOT], page: PAGE_START, level_to_load: FRONTEND_LEVEL, ..Default::default() }
    }

    /// [M 0x1c9e80]
    pub fn get_state(&self) -> u8 { *self.stack.last().unwrap_or(&0) }

    /// [M 0x1c9f38] — 7 becomes 8 (a fade hold), 0xb becomes 0xc (a frame wait).
    pub fn push_state(&mut self, s: u8) {
        self.skip_draw = Self::draw_skip_for(s);
        self.stack.push(match s {
            7 => { self.fade_frames = 8; ST_FADE_HOLD }
            0xb => { self.wait_frames = 8; ST_WAIT }
            0xd => { self.fade_frames = 8; ST_FADE_IN }
            other => other,
        });
    }

    /// [M 0x1c9eb0]
    pub fn pop_state(&mut self) -> u8 {
        if self.stack.len() > 1 { self.stack.pop(); }
        let s = self.get_state();
        self.skip_draw = Self::draw_skip_for(s);
        s
    }

    /// States {1, 3, 4, 5, 9} skip Game_Draw [M 0x1c98c8's switch after every push/pop].
    pub fn draw_skip_for(s: u8) -> bool { matches!(s, 1 | 3 | 4 | 5 | 9) }

    /// One `GameFlow_Main` iteration [M 0x1c98c8]. The loader's real streaming is driven by the caller
    /// through [`GameFlow::load_tick`] (headless tests finish loads instantly).
    pub fn main_tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        match self.get_state() {
            // Boot: Mem_Init, bootup (pushes 2), Boot_LoadPTPData, Reset_MapLoadSettings → the boot
            // load of the front-end map [M case 1 + 0x1d6958].
            ST_BOOT => { self.push_state(ST_RUNNER); self.reset_map_level_to_load(self.level_to_load, false, false); }
            // Runner: when a load was requested (GameState+0x24 [M case 2]), push the loader.
            ST_RUNNER if self.load_pending != 0 => { self.load_pending = 0; self.push_state(ST_MAPLOAD); }
            // Movie: pops when the movie finishes or the skip action fires [M case 6, Input_Action(-1, 0x19)].
            ST_MOVIE => { self.pop_state(); }
            // Fade holds pop when the fade finishes [M cases 8/0xe; headless countdown].
            ST_FADE_HOLD | ST_FADE_IN => { if self.fade_frames > 0 { self.fade_frames -= 1; } else { self.pop_state(); } }
            ST_WAIT => { if self.wait_frames > 0 { self.wait_frames -= 1; } else { self.pop_state(); } }
            _ => {}
        }
    }

    /// The loader body [M 0x1d6360] reduced to its state machine: returns false while more ticks are
    /// needed (a movie wrap costs two calls, a plain load one).
    pub fn load_tick(&mut self) -> bool {
        if self.get_state() != ST_MAPLOAD { return true; }
        if self.movie_code != 0 {
            // Phase 2 [M LoadState 2: `GetLevelToLoadFromMovieCode(GameState+0x10)` maps the 0x71/0x73
            // movie codes back to level codes; 0x07100018 → 0x0700001b is the one special case].
            let mc = self.movie_code;
            self.movie_code = 0;
            self.level_to_load = if mc == 0x0710_0018 { 0x0700_001b } else { 0x0700_0000 | (mc & 0xffff) };
            return false;
        }
        let level = self.level_to_load;
        if level & 0xff0000 != 0 {
            // Phase 0: a 0x71/0x73 intro-wrapped code — record the movie code (GameState+0x10), push 6.
            self.movie_code = level;
            self.push_state(ST_MOVIE);
            return false;
        }
        self.current_level = level;      // [M phase 2: GameState+0x12]
        // Phase 3 done: pop 3, push 4 (Locks_Init pops itself) [M 0x1d6360 tail].
        self.pop_state();
        self.push_state(ST_LOCKS);
        self.pop_state();
        true
    }

    /// [M 0x1d3800] — a menu asked for a level. `no_intro` = the game's third arg (1 keeps the code as
    /// is; 0 maps story codes to their 0x71 intro wrappers). Driving levels push 9 — none are in
    /// MissionData, so we only model the on-foot branch here.
    pub fn reset_map_level_to_load(&mut self, level: u32, warm: bool, no_intro: bool) {
        self.warm_reset = warm;
        if !matches!(self.get_state(), ST_RUNNER | 7 | ST_FADE_HOLD | 0xd | ST_FADE_IN) { return; }
        self.skip_draw = true; // [M: GameState[0x40] = 1 in the menu states]
        // [M 0x1d3800's switch wraps exactly these story codes when param_3 == 0; 0x7000048 and the
        // other wrapper/multiplayer codes are not in the switch and pass through.]
        let plain = level & 0xff00_ffff;
        let code = if !no_intro && level & 0xff0000 == 0 && ((0x0700_0001..=0x0700_0016).contains(&plain) || plain == 0x0700_001b || plain == 0x0700_004a) {
            level + 0x0010_0000
        } else { level };
        self.level_to_load = code;
        self.push_state(ST_MAPLOAD);
    }
}

// ---------------------------------------------------------------------------
// The front end pages
// ---------------------------------------------------------------------------

/// The campaign [M MissionData 0x2a4350, docs/formats/mission-data.md]: (name, first level).
pub const MISSIONS: [(&str, u32); 9] = [
    ("Paris Prelude", 0x0700_0005),
    ("The Exchange", 0x0700_0001),
    ("Alpine Escape", 0x0700_0009),
    ("Enemies Vanish", 0x0700_000c),
    ("Night of the Agent", 0x0700_000e),
    ("Isle of Despair", 0x0700_0011),
    ("Underworld", 0x0700_0010),
    ("Deep Water", 0x0700_0014),
    ("Countdown", 0x0700_001b),
];

/// The difficulty choices [S — the names are the game's standard set; `difficulty` global 0x2df4c0 [M]].
pub const DIFFICULTIES: [&str; 3] = ["Agent", "Operative", "007"];

/// Which front-end page is up. The ids are the game's own (the constants above); the enum is the
/// dispatcher the port drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page { Start, Main, MissionSelect, Difficulty, Options, Pause, Results }

impl Page {
    pub fn id(self) -> u32 {
        match self {
            Page::Start => PAGE_START, Page::Main => PAGE_MAIN, Page::MissionSelect => PAGE_MISSION_SELECT,
            Page::Difficulty => PAGE_DIFFICULTY, Page::Options => PAGE_OPTIONS, Page::Pause => PAGE_PAUSE,
            Page::Results => PAGE_RESULTS,
        }
    }
    fn from_id(id: u32) -> Option<Page> {
        Some(match id {
            PAGE_START => Page::Start, PAGE_MAIN => Page::Main, PAGE_MISSION_SELECT => Page::MissionSelect,
            PAGE_DIFFICULTY => Page::Difficulty, PAGE_OPTIONS => Page::Options, PAGE_PAUSE => Page::Pause,
            PAGE_RESULTS => Page::Results, _ => return None,
        })
    }
}

/// The menu's per-frame input, mapped from mouse + keyboard by the caller (lib.rs). The game's own
/// engine routes cursor moves and confirms through `Menu_InputAction` / action 0x22 [M 0x1f1f40,
/// 0x21c3d8] and skips movies through action 0x19 [M 0x1c98c8 case 6].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { Up, Down, Confirm, Back }

/// The end-of-mission outcome the results page shows [M P_NFRESULTS is reached with +0x28 = 0x40000036
/// from Mission_Update; P_ENDMISSION renders the stats pane 0x100001a4].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Complete, Failed }

/// Mission-end stats for the results screen [S — P_NFRESULTS 0x208e78 renders the stats; the fields
/// the port tracks are the player state's own counters].
#[derive(Clone, Debug, Default)]
pub struct Stats { pub shots_fired: u32, pub shots_hit: u32, pub accuracy_pct: u32, pub time_s: u32 }

/// The front end: which page is up, the cursor, and the flow machine it drives.
pub struct FrontEnd {
    pub flow: GameFlow,
    pub page: Page,
    pub cursor: usize,
    pub mission: usize,
    pub difficulty: usize,
    /// The attract timer: the game counts input-less frames and switches to attract at 0xa8c = 2700
    /// [M 0x20a408/0x20a7b0 `Menu_GetNoInputCount() == 0xa8c`]. The port records it; switching to the
    /// attract demo needs demo playback (not ported — reported).
    pub no_input_frames: u32,
    pub press_start_on: bool,
    pub outcome: Outcome,
    pub stats: Stats,
    /// The level the front end wants the host to load next (set together with a MAPLOAD push).
    pub requested_level: Option<u32>,
    /// The pause page's quit confirm: the game swaps the row text and only quits on the second
    /// choose (`state_quit_166` [M C_GCPAUSE_Handler 0x204408:210-220]).
    pub confirming_quit: bool,
    /// The pause page's resume choice: the host closes the menu when this is set.
    pub closed: bool,
}

impl FrontEnd {
    pub fn new() -> Self {
        let flow = GameFlow::boot();
        let page = Page::from_id(flow.page).unwrap_or(Page::Start);
        FrontEnd { flow, page, cursor: 0, mission: 0, difficulty: 0, no_input_frames: 0, press_start_on: true, outcome: Outcome::Failed, stats: Stats::default(), requested_level: None, confirming_quit: false, closed: false }
    }

    pub fn rows(&self) -> usize {
        match self.page {
            Page::Start => 1,
            Page::Main => 3,                       // Nightfire / Multiplayer / Options [S row set]
            Page::MissionSelect => MISSIONS.len(),
            Page::Difficulty => DIFFICULTIES.len(),
            Page::Options => 3,                    // the options page stubs three rows [S]
            Page::Pause => 4,                      // resume / restart / options / quit [S rows; actions M 0x204408]
            Page::Results => 3,                    // restart / continue / quit [M 0x216618]
        }
    }

    pub fn row_label<'a>(&self, lang: Option<&'a Lang>, row: usize) -> String {
        let bind = |id: u32| lang.and_then(|l| l.bind(id)).map(str::to_string).unwrap_or_else(|| format!("<{id:#x}>"));
        match self.page {
            Page::Start => "Press Start".into(),
            Page::Main => ["Nightfire", "Multiplayer", "Options"][row.min(2)].into(),
            Page::MissionSelect => MISSIONS.get(row).map_or(String::new(), |(n, _)| (*n).into()),
            Page::Difficulty => DIFFICULTIES.get(row).map_or(String::new(), |n| (*n).into()),
            Page::Options => ["Screen adjust", "Controls", "Back"][row.min(2)].into(),
            // The single-player pause rows [S labels — the disc page data holds the originals; the
            // ACTIONS are the game's: restart = ResetMap_LevelToLoad(BaseMap, 0, 1) and the quit
            // confirm two-step [M C_GCPAUSE_Handler 0x204408:139-152/210-220]. The eight Txt ids
            // 0x82..0x100019a in P_PAUSE 0x203c28 turned out to be the multiplayer scenario names
            // ("NightFire"…"Classic Bond") — see docs/reports/fix-menus.md.]
            Page::Pause => ["Resume", "Restart mission", "Options", "Quit to menu"][row.min(3)].into(),
            Page::Results => ["Restart mission", "Continue", "Quit to menu"][row.min(2)].into(),
        }
    }

    /// One menu tick: advance the flow machine, run the page's own timers [M the 0x4c/0x50 messages].
    pub fn tick(&mut self) {
        self.flow.main_tick();
        if !self.flow.load_tick() { return; }
        // The title/main 240-frame flash [M 0x20a408's delayed 0x100000fa toggle at 0xf0 frames].
        if matches!(self.page, Page::Start | Page::Main) {
            self.press_start_on = self.flow.ticks % 240 < 120;
        }
        if self.no_input_frames >= 2700 && matches!(self.page, Page::Start | Page::Main) {
            // [M: attract at 0xa8c] — the demo itself is not ported; reported in docs/reports/fix-menus.md.
        }
        self.no_input_frames = self.no_input_frames.saturating_add(1);
    }

    /// The pause page opens over a running mission (the game's GS_PauseGame [M 0x17d178 pauses the
    /// sim]; the port only tracks the page — the caller pauses its own loop).
    pub fn open_pause(&mut self) -> bool {
        if self.page != Page::Pause && self.flow.current_level != FRONTEND_LEVEL && self.flow.current_level != 0 {
            self.page = Page::Pause;
            self.cursor = 0;
            true
        } else { false }
    }

    /// A mission ended; the game records the results page and pushes the 60-frame fade [M 0x182c60:74
    /// sets +0x28 = 0x40000036, `GameFlow_PushState(60.0, 7, …)`]. The front-end map itself is only
    /// loaded once the player picks restart / continue / quit [M 0x216618's message 0x4b arms].
    pub fn mission_ended(&mut self, outcome: Outcome, stats: Stats) {
        self.outcome = outcome;
        self.stats = stats;
        self.flow.page = PAGE_RESULTS;
        self.flow.push_state(ST_FADE_HOLD); // [M 0x182c60:74 pushes a fade]
        self.page = Page::Results;
        self.cursor = 0;
    }

    /// Handle one menu action. Returns true when the action changed the state.
    pub fn action(&mut self, a: Action) -> bool {
        self.no_input_frames = 0; // [M Menu_ResetNoInputCount on every handled input]
        match self.page {
            Page::Start => {
                if a == Action::Confirm {                       // → the main menu [S: the disc page wires Start → 0x4000002e]
                    self.page = Page::Main; self.cursor = 0; return true;
                }
            }
            Page::Main => match (a, self.cursor) {
                (Action::Up, _) => { self.cursor = (self.cursor + self.rows() - 1) % self.rows(); return true; }
                (Action::Down, _) => { self.cursor = (self.cursor + 1) % self.rows(); return true; }
                (Action::Confirm, 0) => { self.page = Page::MissionSelect; self.cursor = 0; return true; }   // Nightfire [M: C_GONIGHTFIRE page 0x4000002f exists; wiring S]
                (Action::Confirm, 1) => { return false; }   // Multiplayer: not ported (reported)
                (Action::Confirm, 2) => { self.page = Page::Options; self.cursor = 0; return true; }
                (Action::Back, _) => { self.page = Page::Start; self.cursor = 0; return true; }
                _ => {}
            },
            Page::MissionSelect => match a {
                Action::Up => { self.cursor = (self.cursor + self.rows() - 1) % self.rows(); return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % self.rows(); return true; }
                Action::Confirm => { self.mission = self.cursor.min(MISSIONS.len() - 1); self.page = Page::Difficulty; self.cursor = 0; return true; }
                Action::Back => { self.page = Page::Main; self.cursor = 0; return true; }
            },
            Page::Difficulty => match a {
                Action::Up => { self.cursor = (self.cursor + self.rows() - 1) % self.rows(); return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % self.rows(); return true; }
                Action::Confirm => {
                    self.difficulty = self.cursor.min(DIFFICULTIES.len() - 1);
                    // Start the mission: fresh (cold) load of its first level [M 0x1d3800; the game's
                    // select path goes through P_NFMAP/C_SBNFMAP 0x20b608/0x20b8d8 → the same call].
                    let level = MISSIONS[self.mission].1;
                    self.requested_level = Some(level);
                    self.flow.reset_map_level_to_load(level, false, false);
                    return true;
                }
                Action::Back => { self.page = Page::MissionSelect; self.cursor = self.mission; return true; }
            },
            Page::Options => match a {
                Action::Up => { self.cursor = (self.cursor + 2) % 3; return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % 3; return true; }
                Action::Back | Action::Confirm if self.cursor == 2 => { self.page = Page::Main; self.cursor = 2; return true; }
                _ => {}
            },
            Page::Pause => match a {
                Action::Up => { self.cursor = (self.cursor + 3) % 4; return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % 4; return true; }
                Action::Back => { self.confirming_quit = false; self.page = Page::Main; return true; }   // resume [S]
                Action::Confirm => {
                    match self.cursor {
                        0 => { self.confirming_quit = false; self.closed = true; return true; }   // resume — the host closes the menu
                        1 => {                                                               // restart [M 0x204408:139-141:
                            // ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1) — the mission's FIRST map]
                            self.confirming_quit = false;
                            let level = MISSIONS[self.mission].1;
                            self.requested_level = Some(level);
                            self.flow.reset_map_level_to_load(level, false, true);
                            self.page = Page::Main;
                            return true;
                        }
                        2 => { self.page = Page::Options; self.cursor = 0; return true; }    // options
                        3 => {                                                               // quit, two-step [M 0x204408:210-220]
                            if !self.confirming_quit {
                                self.confirming_quit = true;   // the game swaps the row text to 0x200
                                return true;
                            }
                            self.flow.page = PAGE_RESULTS_QUIT_TARGET;                          // +0x28 = 0x4000001c [M :143-148]
                            self.requested_level = Some(FRONTEND_LEVEL);
                            self.flow.reset_map_level_to_load(FRONTEND_LEVEL, false, true);
                            self.page = Page::Main;
                            self.cursor = 0;
                            return true;
                        }
                        _ => {}
                    }
                }
            },
            Page::Results => match a {
                Action::Up => { self.cursor = (self.cursor + 2) % 3; return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % 3; return true; }
                Action::Confirm => {
                    match self.cursor {
                        // [M 0x216618 message 0x4b: 1 = restart, 0 = continue, 2 = quit]
                        0 => {
                            let level = MISSIONS[self.mission].1;
                            self.requested_level = Some(level);
                            self.flow.reset_map_level_to_load(level, false, true);
                            self.page = Page::Main;
                            return true;
                        }
                        1 => {
                            let level = self.flow.current_level;
                            self.requested_level = Some(level);
                            self.flow.reset_map_level_to_load(level, true, true);
                            self.page = Page::Main;
                            return true;
                        }
                        2 => {
                            self.flow.page = PAGE_RESULTS_QUIT_TARGET;
                            self.requested_level = Some(FRONTEND_LEVEL);
                            self.flow.reset_map_level_to_load(FRONTEND_LEVEL, false, true);
                            self.page = Page::Main;
                            self.cursor = 0;
                            return true;
                        }
                        _ => {}
                    }
                }
                Action::Back => { self.cursor = 2; return true; }
            },
        }
        false
    }
}

// ---------------------------------------------------------------------------
// The menu overlay: one HudFrame per tick, drawn with the game's own font [M FONTSerpLight] and
// strings [M USATxt.dat via Lang]. Layout: the game's 512 × 448 screen [M hud.rs SW/SH].
// ---------------------------------------------------------------------------

/// One menu frame's draw data. `menu_overlay` fills a `HudFrame` the host submits like the HUD's.
pub fn menu_overlay(fe: &FrontEnd, font: &nf_formats::hud::Font, lang: Option<&Lang>, textures: &[(u32, crate::scene::TextureImage)], aspect: f32) -> HudFrame {
    let mut h = Hud { quads: Vec::new(), aspect, textures };
    let dim = gs(0x101418, 0xd0);
    // The backdrop: the menus run on the front-end map; without a GPU frame behind us this stays a
    // dim plate [S: the real background is the menu FMV (menu_movie, SFX 0x1d7/0x470 [M 0x20a408])].
    ndc_plate(&mut h, 0.0, 0.0, SW, SH, dim);
    let title_c = gs(0xd8d2c2, 0xff);
    let row_c = gs(0xbdb6a4, 0xff);
    let sel_c = gs(0xffffff, 0xff);
    match fe.page {
        Page::Start => {
            h.text(Anchor::Centre, font, -4, "007: NIGHTFIRE", SW / 2.0, 120.0, false, title_c, true);
            if fe.press_start_on { h.text(Anchor::Centre, font, -3, "PRESS START", SW / 2.0, 260.0, false, sel_c, true); }
            // The legal line [M 0x20a408: Txt_BindLabel(0x375)].
            let legal = lang.and_then(|l| l.bind(0x375)).unwrap_or("");
            if !legal.is_empty() { h.text(Anchor::Centre, font, -3, legal, SW / 2.0, 424.0, false, row_c, true); }
        }
        Page::Main => { page_rows(&mut h, fe, font, "MAIN MENU", row_c, sel_c); }
        Page::MissionSelect => { page_rows(&mut h, fe, font, "SELECT MISSION", row_c, sel_c); }
        Page::Difficulty => { page_rows(&mut h, fe, font, "SELECT DIFFICULTY", row_c, sel_c); }
        Page::Options => { page_rows(&mut h, fe, font, "OPTIONS", row_c, sel_c); }
        Page::Pause => { page_rows(&mut h, fe, font, "PAUSE", row_c, sel_c); }
        Page::Results => {
            let head = match fe.outcome { Outcome::Complete => "MISSION COMPLETE", Outcome::Failed => "MISSION FAILED" };
            h.text(Anchor::Centre, font, -4, head, SW / 2.0, 96.0, false, title_c, true);
            let st = &fe.stats;
            let line = format!("TIME {:02}:{:02}:{:02}   SHOTS {}   HITS {}   ACCURACY {}%", st.time_s / 60, st.time_s % 60, 0, st.shots_fired, st.shots_hit, st.accuracy_pct);
            h.text(Anchor::Centre, font, -3, &line, SW / 2.0, 140.0, false, row_c, true);
            page_rows_no_head(&mut h, fe, font, 200.0, row_c, sel_c);
        }
    }
    h.finish()
}

fn page_rows(h: &mut Hud, fe: &FrontEnd, font: &nf_formats::hud::Font, head: &str, row_c: [f32; 4], sel_c: [f32; 4]) {
    h.text(Anchor::Centre, font, -4, head, SW / 2.0, 80.0, false, sel_c, true);
    page_rows_no_head(h, fe, font, 140.0, row_c, sel_c);
}

fn page_rows_no_head(h: &mut Hud, fe: &FrontEnd, font: &nf_formats::hud::Font, y0: f32, row_c: [f32; 4], sel_c: [f32; 4]) {
    for row in 0..fe.rows() {
        let label = fe.row_label(None, row);
        let y = y0 + row as f32 * 26.0;
        if row == fe.cursor { h.text(Anchor::Centre, font, -2, &format!("> {}", label), SW / 2.0, y, false, sel_c, true); }
        else { h.text(Anchor::Centre, font, -2, &label, SW / 2.0, y, false, row_c, true); }
    }
    // The quit confirm swaps in a confirm line [M 0x204408: label 0x200 replaces the quit row's text].
    if fe.page == Page::Pause && fe.confirming_quit {
        h.text(Anchor::Centre, font, -2, "QUIT TO MENU — CONFIRM?", SW / 2.0, SH - 40.0, false, sel_c, true);
    }
}

/// A flat coloured plate (the menu backdrop) — degenerate UVs on the font texture, like the dev
/// overlay's `ndc_quad` [M hud.rs].
fn ndc_plate(h: &mut Hud, x: f32, y: f32, w: f32, hh: f32, c: [f32; 4]) {
    let (a, b) = (h.ndc(Anchor::Stretch, x, y), h.ndc(Anchor::Stretch, x + w, y + hh));
    let p = |q: [f32; 2]| HudVertex { p: q, uv: [0.03, 0.03], c };
    let v = [p([a[0], a[1]]), p([b[0], a[1]]), p([b[0], b[1]]), p([a[0], a[1]]), p([b[0], b[1]]), p([a[0], b[1]])];
    h.ndc_quad(-5, v, c);
}

/// The keyboard mapping the host uses. The game's menu engine walks cursors with the d-pad and
/// confirms with the action buttons [M 0x21c3d8's per-controller action loop]; the port adds keyboard
/// per the task ("mouse and keyboard work everywhere") [S].
pub fn key_action(key: winit::keyboard::KeyCode) -> Option<Action> {
    use winit::keyboard::KeyCode;
    Some(match key {
        KeyCode::ArrowUp | KeyCode::KeyW => Action::Up,
        KeyCode::ArrowDown | KeyCode::KeyS => Action::Down,
        KeyCode::Enter | KeyCode::Space | KeyCode::ArrowRight | KeyCode::KeyD => Action::Confirm,
        KeyCode::Escape | KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::Backspace => Action::Back,
        _ => return None,
    })
}
