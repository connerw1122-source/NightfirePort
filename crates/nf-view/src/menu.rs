//! The front end (fix-menus + fix-menus2): the game's GameFlow state machine and the menu screens drawn
//! from the game's own page data.
//!
//! Evidence base (all [M] unless noted — ACTION.ELF decompile under `work/decomp/ACTION.ELF/`):
//! - `GameFlow_GetState` 0x1c9e80 / `GameFlow_PopState` 0x1c9eb0 / `GameFlow_PushState` 0x1c9f38 /
//!   `GameFlow_Main` 0x1c98c8 — the GameState stack (`GameStateStack` 0x2c7398, `StackIndex` 0x30cece,
//!   64 u32 entries, wraps at 0x40). Push converts 7 → 8 (fade hold) and 0xb → 0xc (frame wait). States
//!   {1, 3, 4, 5, 9} set GameState+0x40 = 1 (skip Game_Draw).
//! - `bootup_bootup` 0x1c7e78 — boot zeroes GameState, sets +0x20 = 0x80000002 (the menu-def file
//!   08000002), +0x28 = 0x40000034 (the P_START page), +0x24 = 1, +0x40 = 2.
//! - `Reset_MapLoadSettings` 0x1d6958 / `ResetMap_LevelToLoad` 0x1d3800 / `ResetMap_Load` 0x1d6360 — the
//!   loader (movie wrap for the 0x71/0x73 prefixes, GameState+0x12 = current level, `MenuManager_Create`
//!   on the front-end map).
//! - `P_START_Handler` 0x20a408 — the title page: the legal line is `Txt_BindLabel(0x375)`, the "Press
//!   the START button" label 0x10001eb flashing on a 0xf0 = 240-frame cycle, attract at 0xa8c frames.
//! - The menu screens' layouts are the disc's own page data: the kind-8 LevelBin entry `08000002` inside
//!   `07000048.bin` (FILES.BIN) — `MenuManager_Load` 0x21de88 stashes it and `MenuManager_Create`
//!   0x21b908 walks it as a tag stream (BIN_GetDWord/Word/Byte): PAGE −0x10 {u32 page id, u32 file hash,
//!   u32 aux movie, u8 kind, u32 size}; CONTROL −0x07 {u32 name, u8 b, u8 type, u16 x, y, w, h, u16 f1,
//!   u32 d1, u16 f2, u32 d2, u32 d3}; NAME −0x08 (32-byte string); SCRIPT −0x09 {u32, u32}; MSG −0x0a /
//!   MGRMSG −0x0d {u8 msg, u32 a, u32 b}; KMSG −0x0c {u32 time, u8, u32, u32}; KFRAME −0x0b (5×u16 +
//!   u32); SET −0x03 (u16); COMPONENT −0x04 {u16 count, 3×u16, u32 texture hash}; SPRITE −0x05 {8×u16,
//!   u32, u32, f32, f32}; SKIP −0x02 (u32); END −0x0e. MSG 0x18 = Txt_BindLabel, 0x24 = set texture
//!   (0x03xxxxxx hash), 0x1b/0x1c = colours (0x7d6d59ff rewritten to 0x645a49ff [M 0x21b908:233-244]).
//!   Parsed 2026-10-13: 56 pages, 1003 controls, clean END — every layout constant below cites its page.
//!   Coordinates are the game's 640 × 480 menu space; the overlay scales them onto the 512 × 448 screen.
//! - Page ids (menu-def data + page table): 0x40000002 main menu, 0x40000025 select codename,
//!   0x40000023 select difficulty, 0x4000001c single player, 0x4000002b dossier, 0x40000034 P_START
//!   (the boot page — the language-flag page), 0x40000009 the title shown after boot, 0x4000003b
//!   rewards, 0x40000047 difficulty wheel, 0x40000036 results, 0x40000038 stats, 0x4000000c pause.
//! - The campaign is `sp_level` 0x2df2e0 — 12 × 0x18-byte records {u32, u32 name label, u32 description
//!   label, u32 level, u8 unlocked, ...} [M C_SBNFMAP 0x20b8d8:21 reads +0xc/+0x10]. The records' name
//!   and description labels are the bank-3 Txt ids below; the labels resolve through the language bank
//!   (`Lang`, `Txt_LoadLanguage` 0x1caad0 / `Txt_BindLabel` 0x1cad60). Completing mission *i* unlocks
//!   *i+1* (`Menu_SetLevelBonus` 0x1ffa10 sets sp_level[(i+1)]+0x10 [M :63-65]); a fresh profile has
//!   only the first row unlocked [S: no writer of the other flags was found].
//! - Difficulty confirm → the single player page (`C_SBNFDFCTY` 0x216ca8:10-17 `Menu_ChangePageCloseIris`
//!   0x4000001c); mission confirm → `ResetMap_LevelToLoad(sp_level[i]+0xc, 0, 0)` + push fade + delete
//!   the manager [M `C_SBNFMAP` 0x20b8d8:18-41]. Locked rows: `if (!menu_unlock_everything &&
//!   !sp_level[i*0x18+0x10]) return` [M 0x20b8d8:18-20].

use crate::gpu::{HudFrame, HudVertex};
use crate::hud::{Hud, Anchor};
use nf_formats::hud::Lang;

/// The game's screen [M hud.rs SW/SH: Viewer+0x10C/+0x110, 512 × 448 shown at 4:3].
const SW: f32 = 512.0;
const SH: f32 = 448.0;
/// The menu pages are authored in a 640 × 480 space (their root control is (0,0,640,480) [M menu-def
/// control 0x40000002/0x40000009]; `FixupResolution` 0x21b908 scales it per output resolution). The
/// overlay maps that space onto the game's 512 × 448 screen.
const MW: f32 = 640.0;
const MH: f32 = 480.0;
fn mx(x: f32) -> f32 { x * SW / MW }
fn my(y: f32) -> f32 { y * SH / MH }

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
/// The GameState fields the flow machine reads and writes [M addresses in the doc comment].
pub const PAGE_START: u32 = 0x4000_0034;
/// The main menu [M menu-def page 0x40000002: rows NightFire/Multiplayer/Codenames].
pub const PAGE_MAIN: u32 = 0x4000_0002;
/// Select Codename [M menu-def page 0x40000025: header Txt 0x1f2].
pub const PAGE_CODENAME: u32 = 0x4000_0025;
/// Select Difficulty [M menu-def page 0x40000023: header Txt 0x1f8].
pub const PAGE_DIFFICULTY: u32 = 0x4000_0023;
/// Single Player, the mission list [M menu-def page 0x4000001c: header Txt 0x14c].
pub const PAGE_MISSION_SELECT: u32 = 0x4000_001c;
/// The Dossier page [M menu-def page 0x4000002b: header Txt 0x27f].
pub const PAGE_DOSSIER: u32 = 0x4000_002b;
/// The title layout the boot page leads to [M menu-def page 0x40000009: the art + Press START].
pub const PAGE_TITLE_LAYOUT: u32 = 0x4000_0009;
pub const PAGE_OPTIONS: u32 = 0x4000_0068;
pub const PAGE_PAUSE: u32 = 0x4000_000c;
pub const PAGE_RESULTS: u32 = 0x4000_0036;
pub const PAGE_RESULTS_QUIT_TARGET: u32 = 0x4000_001c;

/// The GameState stack machine's fields [M addresses in the doc comment].
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
    /// The state-6 movie countdown (the port can't decode the zeroed PSS files; headless model).
    pub movie_frames: u32,
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
            ST_MOVIE => { self.movie_frames = self.movie_frames.saturating_sub(1); if self.movie_frames == 0 { self.pop_state(); } }
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
        // Phase 0: only the 0x71/0x73 intro-wrapped codes record a movie code [M 0x1d6360 "the level
        // still carries the 0x71/0x73 prefix"]. The driving codes 0x09xxxxxx load directly.
        if matches!(level >> 16, 0x71 | 0x73) {
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
    /// the campaign rows here, so we only model the on-foot branch.
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

    /// The title→main-menu intro movie. The game plays it as background FMVs (`P_INTRO` 0x209e88
    /// psiStartBackgroundMovie 0x07370048 then 0x07380048 [M 0x209e88:38-62]); the trimmed ISO's
    /// /MOVIES/30_FPS PSS files are zeroed, so the port models the state machine only: a fixed frame
    /// countdown, skippable [M GameFlow_Main case 6 pops on the skip action].
    pub fn play_movie(&mut self, frames: u32) {
        self.push_state(ST_MOVIE);
        self.movie_frames = frames;
    }
}

// ---------------------------------------------------------------------------
// The front end pages — layouts from the disc's menu-def (08000002), strings from the language bank
// ---------------------------------------------------------------------------

/// The campaign [M `sp_level` 0x2df2e0's 12 records: +0x04 name label, +0x08 description label (bank-3
/// Txt ids), +0x0c level, +0x10 unlocked; read back from the EE snapshot]. The port keeps the game's
/// level codes verbatim — the 0x09xxxxxx rows are the driving maps (`C_SBNFMAP` passes them straight to
/// `ResetMap_LevelToLoad` [M 0x20b8d8:21-24]).
pub const CAMPAIGN: [(u32, u32, u32); 12] = [
    (0x0300_0001, 0x0300_0013, 0x0900_0001), // Paris Prelude
    (0x0300_0002, 0x0300_0014, 0x0700_0005), // The Exchange
    (0x0300_0003, 0x0300_0015, 0x0900_0005), // Alpine Escape
    (0x0300_0004, 0x0300_0016, 0x0900_0006), // Enemies Vanquished
    (0x0300_0005, 0x0300_0017, 0x0700_0001), // Double Cross
    (0x0300_0006, 0x0300_0018, 0x0700_0009), // Night Shift
    (0x0300_0007, 0x0300_0019, 0x0700_000c), // Chain Reaction
    (0x0300_0009, 0x0300_001b, 0x0700_0011), // Phoenix Fire
    (0x0300_000c, 0x0300_001e, 0x0900_0002), // Deep Descent
    (0x0300_000e, 0x0300_0020, 0x0900_0003), // Island Infiltration
    (0x0300_0034, 0x0300_0035, 0x0700_0014), // Deep Water
    (0x0300_0011, 0x0300_0023, 0x0700_001b), // Countdown
];

/// The difficulty choices [M Txt 0x14d/0x14e/0x14f = "Operative"/"Agent"/"00 Agent" (USATxt.dat), the
/// order of menu-def page 0x40000023's wheel rows; the `difficulty` global is 0x2df4c0 [M symtab]].
pub const DIFFICULTIES: [u32; 3] = [0x14d, 0x14e, 0x14f];
/// Each difficulty's description line [M Txt 0x10000006/7/8].
pub const DIFFICULTY_DESCS: [u32; 3] = [0x100_0006, 0x100_0007, 0x100_0008];
/// The main menu's item labels and one-line descriptions [M menu-def page 0x40000002 rows + Txt].
pub const MAIN_ITEMS: [u32; 3] = [0x28, 0x149, 0x14b];
pub const MAIN_DESCS: [u32; 3] = [0x100_0000, 0x100_0001, 0x100_0002];

/// Which front-end page is up. The ids are the game's own (the constants above); the enum is the
/// dispatcher the port drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page { Start, Main, Codename, Difficulty, MissionSelect, Dossier, Options, Pause, Results }

impl Page {
    pub fn id(self) -> u32 {
        match self {
            Page::Start => PAGE_START, Page::Main => PAGE_MAIN, Page::Codename => PAGE_CODENAME,
            Page::Difficulty => PAGE_DIFFICULTY, Page::MissionSelect => PAGE_MISSION_SELECT,
            Page::Dossier => PAGE_DOSSIER, Page::Options => PAGE_OPTIONS, Page::Pause => PAGE_PAUSE,
            Page::Results => PAGE_RESULTS,
        }
    }
    fn from_id(id: u32) -> Option<Page> {
        Some(match id {
            PAGE_START => Page::Start, PAGE_MAIN => Page::Main, PAGE_CODENAME => Page::Codename,
            PAGE_DIFFICULTY => Page::Difficulty, PAGE_MISSION_SELECT => Page::MissionSelect,
            PAGE_DOSSIER => Page::Dossier, PAGE_OPTIONS => Page::Options, PAGE_PAUSE => Page::Pause,
            PAGE_RESULTS => Page::Results, _ => return None,
        })
    }
}

/// The menu's per-frame input, mapped from mouse + keyboard by the caller (lib.rs). The game's own
/// engine routes cursor moves and confirms through `Menu_InputAction` / action 0x22 [M 0x1f1f40,
/// 0x21c3d8] and skips movies through action 0x19 [M 0x1c98c8 case 6].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { Up, Down, Confirm, Back, Side }

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
    /// The codename page's "Play without saving" confirm box [M the page's plate control
    /// 0x100001f2 (50,301,540×113) and the CANCEL/CONFIRM pattern of the other pages' boxes].
    pub confirming_default_profile: bool,
    /// `menu_unlock_everything` 0x30d2a7 — the dev unlock [M 0x20b8d8:18]. The host sets it with
    /// `--unlock-all` so every page can be rendered.
    pub unlock_all: bool,
    /// sp_level[i]+0x10 — which campaign rows are unlocked [M 0x2df2e0 layout]. A fresh profile has
    /// row 0 only [S]; completing row *i* unlocks *i+1* [M Menu_SetLevelBonus 0x1ffa10:63-65].
    pub unlocked: [bool; 12],
    /// The pause page's resume choice: the host closes the menu when this is set.
    pub closed: bool,
}

impl FrontEnd {
    pub fn new() -> Self {
        let flow = GameFlow::boot();
        let page = Page::from_id(flow.page).unwrap_or(Page::Start);
        let mut unlocked = [false; 12];
        unlocked[0] = true; // [S: a fresh profile — see the field doc]
        FrontEnd { flow, page, cursor: 0, mission: 0, difficulty: 0, no_input_frames: 0, press_start_on: true, outcome: Outcome::Failed, stats: Stats::default(), requested_level: None, confirming_quit: false, confirming_default_profile: false, unlock_all: false, unlocked, closed: false }
    }

    pub fn rows(&self) -> usize {
        match self.page {
            Page::Start => 1,
            Page::Main => MAIN_ITEMS.len(),
            Page::Codename => 1,                   // the stub offers "Play without saving" only
            Page::Difficulty => DIFFICULTIES.len(),
            Page::MissionSelect => CAMPAIGN.len(),
            Page::Dossier => 1,
            Page::Options => 3,                    // the options page stubs three rows [S]
            Page::Pause => 4,                      // resume / restart / options / quit [S rows; actions M 0x204408]
            Page::Results => 3,                    // restart / continue / quit [M 0x216618]
        }
    }

    pub fn row_label<'a>(&self, lang: Option<&'a Lang>, row: usize) -> String {
        let bind = |id: u32| lang.and_then(|l| l.bind(id)).map(str::to_string).unwrap_or_else(|| format!("<{id:#x}>"));
        match self.page {
            Page::Start => bind(0x100_01eb),       // "Press the START button" [M 0x20a408]
            Page::Main => bind(MAIN_ITEMS[row.min(2)]),
            Page::Codename => bind(0x100_02ff),    // "Play without saving" [M Txt; menu-def row labels are runtime-filled]
            Page::Difficulty => bind(DIFFICULTIES[row.min(2)]),
            Page::MissionSelect => bind(CAMPAIGN.get(row).map_or(0, |c| c.0)),
            Page::Dossier => bind(0x27f),          // "Dossier"
            Page::Options => ["Screen adjust", "Controls", "Back"][row.min(2)].into(),
            // The single-player pause rows [S labels — the disc page data holds the originals; the
            // ACTIONS are the game's: restart = ResetMap_LevelToLoad(BaseMap, 0, 1) and the quit
            // confirm two-step [M C_GCPAUSE_Handler 0x204408:139-152/210-220].]
            Page::Pause => ["Resume", "Restart mission", "Options", "Quit to menu"][row.min(3)].into(),
            Page::Results => ["Restart mission", "Continue", "Quit to menu"][row.min(2)].into(),
        }
    }

    /// The wheel's visible slots: the game's list pages show five rows (y = 123/152/182/211/241 [M
    /// menu-def rows 0x100000e3/0x100000e4/0x100000e6 on pages 0x4000001c/0x40000025/0x40000023]) with
    /// the selected row centred. Returns (row index, distance from the cursor) pairs.
    pub fn wheel(&self) -> Vec<(usize, i32)> {
        let n = self.rows();
        (-2i32..=2).filter_map(|d| {
            let r = self.cursor as i32 + d;
            (r >= 0 && r < n as i32).then_some((r as usize, d))
        }).collect()
    }

    /// One menu tick: advance the flow machine, run the page's own timers [M the 0x4c/0x50 messages].
    pub fn tick(&mut self) {
        self.flow.main_tick();
        if !self.flow.load_tick() { return; }
        // The title/main 240-frame flash [M 0x20a408's delayed 0x100000fa toggle at 0xf0 frames].
        if matches!(self.page, Page::Start | Page::Main) {
            self.press_start_on = self.flow.ticks % 240 < 120;
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
    /// sets +0x28 = 0x40000036, `GameFlow_PushState(60.0, 7, …)`]. Completing mission *i* unlocks *i+1*
    /// [M Menu_SetLevelBonus 0x1ffa10:63-65].
    pub fn mission_ended(&mut self, outcome: Outcome, stats: Stats) {
        self.outcome = outcome;
        self.stats = stats;
        if outcome == Outcome::Complete && self.mission + 1 < self.unlocked.len() {
            self.unlocked[self.mission + 1] = true;
        }
        self.flow.page = PAGE_RESULTS;
        self.flow.push_state(ST_FADE_HOLD); // [M 0x182c60:74 pushes a fade]
        self.page = Page::Results;
        self.cursor = 0;
    }

    /// One menu action. Returns true when the action changed the state.
    pub fn action(&mut self, a: Action) -> bool {
        self.no_input_frames = 0; // [M Menu_ResetNoInputCount on every handled input]
        match self.page {
            Page::Start => {
                if a == Action::Confirm {
                    // → the intro movie, then the main menu [M P_INTRO 0x209e88 plays the background
                    // FMVs then opens the next page; the driver's observed order Title → movie → Main].
                    self.flow.play_movie(450);
                    self.page = Page::Main;
                    self.cursor = 0;
                    return true;
                }
            }
            Page::Main => match (a, self.cursor) {
                (Action::Up, _) => { self.cursor = (self.cursor + self.rows() - 1) % self.rows(); return true; }
                (Action::Down, _) => { self.cursor = (self.cursor + 1) % self.rows(); return true; }
                // NightFire → Select Codename [M C_GONIGHTFIRE 0x2169b8:36-37 opens the next front-end
                // page; the driver's observed order NightFire → Select Codename → Select Difficulty →
                // Single Player].
                (Action::Confirm, 0) => { self.page = Page::Codename; self.cursor = 0; return true; }
                (Action::Confirm, 1) => { return false; }   // Multiplayer: not ported (reported)
                (Action::Confirm, 2) => { self.page = Page::Codename; self.cursor = 0; return true; }  // Codenames: the same codename page (stub; reported)
                (Action::Back, _) => { self.page = Page::Start; self.cursor = 0; return true; }
                _ => {}
            },
            Page::Codename => match a {
                Action::Up | Action::Down => { self.cursor = (self.cursor + 1) % self.rows(); return true; }
                Action::Confirm => {
                    // The stub offers "Play without saving" only; confirming pops the game's confirm
                    // box (the page's plate control 0x100001f2), ✕ continues, △ cancels.
                    if !self.confirming_default_profile {
                        self.confirming_default_profile = true;
                    } else {
                        self.confirming_default_profile = false;
                        self.page = Page::Difficulty;
                        self.cursor = 1; // Agent [M the game's default difficulty row is the middle one — the wheel starts centred]
                    }
                    return true;
                }
                Action::Back => {
                    if self.confirming_default_profile { self.confirming_default_profile = false; return true; }
                    self.page = Page::Main; self.cursor = 0; return true;
                }
                _ => {}
            },
            Page::Difficulty => match a {
                Action::Up => { self.cursor = (self.cursor + self.rows() - 1) % self.rows(); return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % self.rows(); return true; }
                Action::Confirm => {
                    self.difficulty = self.cursor.min(DIFFICULTIES.len() - 1);
                    // → the Single Player list [M C_SBNFDFCTY 0x216ca8:10-17 Menu_ChangePageCloseIris
                    // 0x4000001c — the driver's observed order Select Difficulty → Single Player].
                    self.page = Page::MissionSelect;
                    self.cursor = self.mission.min(self.first_unlocked());
                    return true;
                }
                Action::Back => { self.page = Page::Codename; self.cursor = 0; return true; }
                _ => {}
            },
            Page::MissionSelect => match a {
                Action::Up => { self.cursor = self.step(-1); return true; }
                Action::Down => { self.cursor = self.step(1); return true; }
                Action::Confirm => {
                    // Locked rows can't be chosen [M C_SBNFMAP 0x20b8d8:18-20 `if
                    // (!menu_unlock_everything && !sp_level[i*0x18+0x10]) return`]. Starting:
                    // ResetMap_LevelToLoad(sp_level[i]+0xc, 0, 0) + a fade push [M :24-41].
                    if !self.unlocked_row(self.cursor) { return false; }
                    self.mission = self.cursor.min(CAMPAIGN.len() - 1);
                    let level = CAMPAIGN[self.mission].2;
                    self.requested_level = Some(level);
                    self.flow.reset_map_level_to_load(level, false, false);
                    return true;
                }
                Action::Side => { self.page = Page::Dossier; return true; }   // □ Dossier
                Action::Back => { self.page = Page::Difficulty; self.cursor = self.difficulty; return true; }
            },
            Page::Dossier => match a {
                Action::Back | Action::Side => { self.page = Page::MissionSelect; return true; }
                _ => {}
            },
            Page::Options => match a {
                Action::Up => { self.cursor = (self.cursor + 2) % 3; return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % 3; return true; }
                Action::Back | Action::Confirm if self.cursor == 2 => { self.page = Page::Main; self.cursor = 2; return true; }
                _ => {}
            },
            Page::Pause => match a {
                Action::Side => {}
                Action::Up => { self.cursor = (self.cursor + 3) % 4; return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % 4; return true; }
                Action::Back => { self.confirming_quit = false; self.closed = true; return true; }   // resume [S]
                Action::Confirm => {
                    match self.cursor {
                        0 => { self.confirming_quit = false; self.closed = true; return true; }   // resume — the host closes the menu
                        1 => {                                                               // restart [M 0x204408:139-141:
                            // ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1) — the mission's FIRST map]
                            self.confirming_quit = false;
                            let level = CAMPAIGN[self.mission].2;
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
                Action::Side => {}
                Action::Up => { self.cursor = (self.cursor + 2) % 3; return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % 3; return true; }
                Action::Confirm => {
                    match self.cursor {
                        // [M 0x216618 message 0x4b: 1 = restart, 0 = continue, 2 = quit]
                        0 => {
                            let level = CAMPAIGN[self.mission].2;
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

    /// The first unlocked row (the cursor's landing spot when the page opens).
    fn first_unlocked(&self) -> usize {
        (0..CAMPAIGN.len()).find(|&i| self.unlocked_row(i)).unwrap_or(0)
    }

    /// sp_level[i]+0x10, with `menu_unlock_everything` [M 0x20b8d8:18].
    pub fn unlocked_row(&self, i: usize) -> bool {
        self.unlock_all || self.unlocked.get(i).copied().unwrap_or(false)
    }

    /// Scroll one row, skipping locked rows [M the driver: locked missions "can't be scrolled to"].
    fn step(&self, dir: i32) -> usize {
        let n = CAMPAIGN.len() as i32;
        let mut r = self.cursor as i32;
        for _ in 0..n {
            r = (r + dir).rem_euclid(n);
            if self.unlocked_row(r as usize) { return r as usize; }
        }
        self.cursor
    }
}

// ---------------------------------------------------------------------------
// The menu overlay: one HudFrame per tick, drawn with the game's own font, textures and page data.
// All positions are the menu-def's own (the doc comment on each page cites its controls); the
// strings come from USATxt.dat through `Lang` (`Txt_BindLabel` 0x1cad60).
// ---------------------------------------------------------------------------

/// The front-end archive textures the pages draw [M each page's msg-0x24 controls in the menu-def;
/// sizes from the archive: 0x03000074 512×512 title art, 0x030000b3 512×256 panel, 0x030000bf 128×128
/// ring emblem, 0x03000084/b6/c4/a5 the ring-masked previews, 0x0300016d 256×128 logo,
/// 0x0300014c 256×256 stamp, 0x03000042 64×64 button/arrow atlas].
pub const MENU_TEXTURES: [u32; 10] = [0x0300_0042, 0x0300_0074, 0x0300_00b3, 0x0300_00bf, 0x0300_0084, 0x0300_00b6, 0x0300_00c4, 0x0300_00a5, 0x0300_016d, 0x0300_014c];

/// The menu text colours [M the pages' msg-0x1b/0x1c colour pairs; 0x7d6d59ff is rewritten to
/// 0x645a49ff by `MenuManager_Create` [M 0x21b908:238-243] — cream text on the warm 007 red].
const CREAM: u32 = 0x645a49;
const BRIGHT: u32 = 0xfff8ec;
const ORANGE: u32 = 0xe6661e;
/// The title's yellow prompt [M page 0x40000009 control 0x100000fa msg 0x1b 0x7f621fff].
const YELLOW: u32 = 0xfec43e;

/// One menu frame's draw data. `menu_overlay` fills a `HudFrame` the host submits like the HUD's.
pub fn menu_overlay(fe: &FrontEnd, font: &nf_formats::hud::Font, lang: Option<&Lang>, textures: &[(u32, crate::scene::TextureImage)], aspect: f32) -> HudFrame {
    let mut h = Hud { quads: Vec::new(), aspect, textures };
    let bind = |id: u32| -> String { lang.and_then(|l| l.bind(id)).unwrap_or("").to_string() };
    let cream = gs(CREAM, 0xff);
    let bright = gs(BRIGHT, 0xff);
    let dim = gs(CREAM, 0x80);
    let dimmest = gs(CREAM, 0x40);
    // The backdrop: the pages' aux FMVs (0x07330048/0x07350048) are zeroed in the trimmed ISO, so the
    // port draws the dark plate the game fades from [M the pages' 0x1b/0x1c fade colours].
    plate(&mut h, 0.0, 0.0, MW, MH, gs(0x180c08, 0xff));
    match fe.page {
        Page::Start => {
            // The title [M menu-def page 0x40000009: the art control 0x10000196 (0,0,640,480) TEX
            // 0x03000074; the prompt control 0x100000fa (131,309,379×16) LBL 0x1c9, colours 0x7f621fff;
            // the legal plate 0x10000197 (50,332,540×108). The handler binds 0x10001eb to the prompt
            // [M 0x20a408] and 0x375 to the plate, flashing on the 240-frame cycle.]
            quad640(&mut h, 0x0300_0074, 0.0, 0.0, MW, MH, [0.0, 0.0, 512.0, 512.0], gs(0x7f7f7f, 0xff));
            if fe.press_start_on {
                h.text(Anchor::Left, font, -2, &bind(0x100_01eb), mx(131.0), my(309.0), false, gs(YELLOW, 0xff), true);
            }
            let legal = bind(0x375);
            if !legal.is_empty() {
                for (i, line) in wrap(&legal, 92).iter().take(4).enumerate() {
                    h.text(Anchor::Left, font, -3, line, mx(50.0), my(332.0 + i as f32 * 22.0), false, gs(CREAM, 0xc0), false);
                }
            }
        }
        Page::Main => {
            // The main menu [M menu-def page 0x40000002: rows 0x10000002/3/4 (167,262/294/326,306×28)
            // LBL 0x28/0x149/0x14b, colours 0x1b 0x7d6d59ff (→0x645a49ff) / 0x1c 0x73330f80 (the
            // selected row's red-orange); the description plate 0x10000225 (103,357,434×42) LBL
            // runtime-filled (Txt 0x10000000/1/2); the hint bar 0x10000001 (50,419,540×21) LBL 0x54e.]
            for (i, &lbl) in MAIN_ITEMS.iter().enumerate() {
                let y = 262.0 + i as f32 * 32.0;
                let (c, scale) = if i == fe.cursor { (gs(ORANGE, 0xff), 1.35) } else { (cream, 1.2) };
                h.text_scaled(Anchor::Left, font, -2, &bind(lbl), mx(167.0 + 153.0), my(y + 6.0), true, c, true, scale);
            }
            h.text(Anchor::Centre, font, -2, &bind(MAIN_DESCS[fe.cursor.min(2)]), mx(103.0 + 217.0), my(357.0 + 14.0), false, bright, true);
            hint_bar(&mut h, font, &bind(0x54e), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::Codename => {
            // Select Codename [M menu-def page 0x40000025: header 0x10000001 (50,40,376×17) LBL 0x1f2;
            // the panel 0x10000001 (50,53,590×295) TEX 0x030000b3; the emblem 0x10000103 +
            // 0x100001cc (114,118,150×150) TEX 0x030000bf/0x030000b6; the five wheel rows
            // 0x100000e4 (323/333/304/333/323, 123..241, 17 px high) LBL runtime-filled; the confirm
            // plate 0x100001f2 (50,301,540×113); the hint bar (50,419) LBL 0x216.]
            page_header(&mut h, font, &bind(0x1f2));
            panel(&mut h);
            emblem(&mut h, 0x0300_00b6, 0x0300_00bf);
            wheel(&mut h, fe, font, lang, |fe, row| fe.row_label(lang, row), if fe.confirming_default_profile { dimmest } else { cream });
            // The confirm box: the game pops it for "Play without saving" [M the plate + Txt 0x10000bb;
            // the driver's PCSX2 transcript].
            if fe.confirming_default_profile {
                confirm_box(&mut h, font, &bind(0x100_00bb), bright);
            }
            hint_bar(&mut h, font, &bind(0x216), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::Difficulty => {
            // Select Difficulty [M menu-def page 0x40000023: header LBL 0x1f8 (50,40); panel TEX
            // 0x030000b3 (50,53,590×295); the emblems 0x100000be/0x10000104 (114,118,150×150) TEX
            // 0x030000a5/0x030000bf; the five wheel rows 0x100000e6 with the selected row's full
            // alpha and the neighbours' 0x80/0x40 [M the rows' msg-0x1b colours]; the description
            // plate 0x100001ee (50,301,540×113); the hint bar LBL 0x218.]
            page_header(&mut h, font, &bind(0x1f8));
            panel(&mut h);
            emblem(&mut h, 0x0300_00a5, 0x0300_00bf);
            wheel(&mut h, fe, font, lang, |_fe, row| bind(DIFFICULTIES[row.min(2)]), cream);
            h.text(Anchor::Centre, font, -2, &bind(DIFFICULTY_DESCS[fe.cursor.min(2)]), mx(320.0), my(301.0 + 40.0), false, bright, true);
            hint_bar(&mut h, font, &bind(0x218), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::MissionSelect => {
            // Single Player [M menu-def page 0x4000001c: header 0x10000029 (50,40) LBL 0x14c; the
            // panel TEX 0x030000b3; the emblem 0x10000020 (114,118,150×150) TEX 0x03000084 (the
            // ring-masked preview) over 0x10000102 TEX 0x030000bf (the ring); the five wheel rows
            // 0x100000e3 (runtime-filled with sp_level's name labels); the description plate
            // 0x1000001f (50,301,540×113); the hint bar 0x1000010e LBL 0x218 (the game swaps the
            // □ Dossier hint in [L PCSX2 — the driver's transcript]; Txt 0x208 has it).]
            page_header(&mut h, font, &bind(0x14c));
            panel(&mut h);
            emblem(&mut h, 0x0300_0084, 0x0300_00bf);
            wheel_dim(&mut h, fe, font, lang, |_fe, row| bind(CAMPAIGN[row].0), |fe, row| !fe.unlocked_row(row), cream);
            h.text(Anchor::Centre, font, -2, &wrap(&bind(CAMPAIGN[fe.cursor.min(CAMPAIGN.len() - 1)].1), 70).join(" "), mx(320.0), my(301.0 + 40.0), false, bright, true);
            hint_bar(&mut h, font, &bind(0x208), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::Dossier => {
            // The Dossier [M menu-def page 0x4000002b: header LBL 0x27f (50,40); panel TEX
            // 0x030000b3; the emblems 0x1000010a/0x1000010b TEX 0x030000c4/0x030000bf; the rows LBL
            // runtime-filled; the hint bar LBL 0x218. The port shows the selected mission's
            // briefing description (its dossier text lives in the campaign's NIS subtitles —
            // reported).]
            page_header(&mut h, font, &bind(0x27f));
            panel(&mut h);
            emblem(&mut h, 0x0300_00c4, 0x0300_00bf);
            let m = fe.mission.min(CAMPAIGN.len() - 1);
            h.text(Anchor::Centre, font, -2, &bind(CAMPAIGN[m].0), mx(320.0), my(123.0 + 10.0), false, bright, true);
            for (i, line) in wrap(&bind(CAMPAIGN[m].1), 70).iter().take(6).enumerate() {
                h.text(Anchor::Centre, font, -2, line, mx(320.0), my(160.0 + i as f32 * 20.0), false, cream, true);
            }
            hint_bar(&mut h, font, &bind(0x218), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::Options => { page_rows(&mut h, fe, font, "OPTIONS", dim, bright); }
        Page::Pause => { page_rows(&mut h, fe, font, "PAUSE", dim, bright); }
        Page::Results => {
            let head = match fe.outcome { Outcome::Complete => "MISSION COMPLETE", Outcome::Failed => "MISSION FAILED" };
            h.text(Anchor::Centre, font, -4, head, SW / 2.0, my(96.0), false, bright, true);
            let st = &fe.stats;
            let line = format!("TIME {:02}:{:02}:{:02}   SHOTS {}   HITS {}   ACCURACY {}%", st.time_s / 60, st.time_s % 60, 0, st.shots_fired, st.shots_hit, st.accuracy_pct);
            h.text(Anchor::Centre, font, -3, &line, SW / 2.0, my(140.0), false, dim, true);
            page_rows_no_head(&mut h, fe, font, my(200.0), dim, bright);
        }
    }
    h.finish()
}

/// The five-slot wheel the list pages draw [M the rows at y = 123/152/182/211/241 with the selected row
/// centred and widest (x = 304) and the neighbours dimmed 0x80/0x40 (their msg-0x1b colours)].
const WHEEL_Y: [f32; 5] = [123.0, 152.0, 182.0, 211.0, 241.0];
const WHEEL_X: [f32; 5] = [323.0, 333.0, 304.0, 333.0, 323.0];

fn wheel(h: &mut Hud, fe: &FrontEnd, font: &nf_formats::hud::Font, lang: Option<&Lang>, label: impl Fn(&FrontEnd, usize) -> String, base: [f32; 4]) {
    wheel_dim(h, fe, font, lang, label, |_, _| false, base);
}

fn wheel_dim(h: &mut Hud, fe: &FrontEnd, font: &nf_formats::hud::Font, _lang: Option<&Lang>, label: impl Fn(&FrontEnd, usize) -> String, locked: impl Fn(&FrontEnd, usize) -> bool, base: [f32; 4]) {
    for (row, d) in fe.wheel() {
        let slot = (d + 2) as usize;
        let (mut c, scale) = match d {
            0 => (base, 1.25),
            -1 | 1 => (dimmed(base, 0x80), 1.0),
            _ => (dimmed(base, 0x40), 1.0),
        };
        if locked(fe, row) { c = dimmed(base, 0x30); }
        let s = label(fe, row);
        h.text_scaled(Anchor::Left, font, -2, &s, mx(WHEEL_X[slot]), my(WHEEL_Y[slot] + 2.0), false, c, true, scale);
    }
}

fn dimmed(mut c: [f32; 4], a: u32) -> [f32; 4] { c[3] = gs(0, a)[3]; c }

/// The page header (cream text at (50,40) [M every list page's 0x10000001 control]).
fn page_header(h: &mut Hud, font: &nf_formats::hud::Font, s: &str) {
    h.text_scaled(Anchor::Left, font, -1, s, mx(50.0), my(40.0), false, gs(BRIGHT, 0xff), true, 1.2);
}

/// The panel behind the rows [M pages 0x40000025/0x40000023/0x4000001c/0x4000002b control (50,53,590×295)
/// TEX 0x030000b3].
fn panel(h: &mut Hud) {
    quad640(h, 0x0300_00b3, 50.0, 53.0, 590.0, 295.0, [0.0, 0.0, 512.0, 256.0], gs(0x7f7f7f, 0xff));
}

/// The ring emblem stack: the page's ring-masked preview over the gold ring, both (114,118,150×150)
/// [M the emblem controls on pages 0x40000025/0x40000023/0x4000001c/0x4000002b].
fn emblem(h: &mut Hud, preview: u32, ring: u32) {
    quad640(h, preview, 114.0, 118.0, 150.0, 150.0, [0.0, 0.0, 128.0, 128.0], gs(0x7f7f7f, 0xff));
    quad640(h, ring, 114.0, 118.0, 150.0, 150.0, [0.0, 0.0, 128.0, 128.0], gs(0x7f7f7f, 0xff));
}

/// The logo (434,38,156×43 TEX 0x0300016d) and the stamp (the 0x10000224 control, whose position
/// `MenuManager_Create` forces to (340,198) [M 0x21b908:305-314], 255×255 TEX 0x0300014c).
fn logo_stamp(h: &mut Hud) {
    quad640(h, 0x0300_014c, 340.0, 198.0, 255.0, 255.0, [0.0, 0.0, 256.0, 256.0], gs(0x7f7f7f, 0x60));
    quad640(h, 0x0300_016d, 434.0, 38.0, 156.0, 43.0, [0.0, 0.0, 256.0, 128.0], gs(0x7f7f7f, 0xff));
}

/// The "Play without saving" confirm box [M the pages' plate controls (50,301,540×113) LBL
/// runtime-filled; the text Txt 0x10000bb].
fn confirm_box(h: &mut Hud, font: &nf_formats::hud::Font, text: &str, c: [f32; 4]) {
    plate(h, mx(50.0), my(301.0), mx(540.0), my(113.0), gs(0x201008, 0xe0));
    for (i, line) in wrap(text, 78).iter().take(4).enumerate() {
        h.text(Anchor::Centre, font, -1, line, mx(320.0), my(306.0 + i as f32 * 22.0), false, c, true);
    }
}

/// The hint bar with the game's button glyphs [M the bars at (50,419,540×21); the strings carry ~A/~B/
/// ~X/~Y/~V/~H/~S placeholders the game's text renderer draws as button icons. The icons are the
/// menu-def sprite atlas 0x03000042's own tiles: the square face (0,0), the circle face (0,16), the
/// scroll arrows (16,0)/(32,0), the left/right arrows (48,0)/(48,16) and the big pair (0,48)/(32,48)
/// [M the atlas tiles, 07000048.bin tex 0x03000042]. `~A` (the cross) has no face tile in the atlas —
/// the port draws the font's X glyph (reported; live check listed in docs/reports/fix-menus2.md).]
fn hint_bar(h: &mut Hud, font: &nf_formats::hud::Font, text: &str, x640: f32, y640: f32, c: [f32; 4]) {
    let mut pen = mx(x640);
    let y = my(y640);
    let icon = mx(16.0);
    let mut rest = text;
    while let Some(t) = rest.find('~') {
        let (before, after) = rest.split_at(t);
        if !before.is_empty() {
            h.text(Anchor::Left, font, -2, before, pen, y, false, c, true);
            pen += font.layout(before, 0.0, y).1;
        }
        let token = after.chars().nth(1);
        let atlas = 0x0300_0042;
        match token {
            Some('A') => { h.text(Anchor::Left, font, -2, "X", pen, y, false, c, false); pen += font.layout("X", 0.0, y).1; }
            Some('Y') => { h.quad(Anchor::Left, -2, atlas, 0, pen, y - 2.0, icon, icon, [0.0, 0.0, 16.0, 16.0], (false, false), c); pen += icon + 2.0; }
            Some('X') => { h.quad(Anchor::Left, -2, atlas, 0, pen, y - 2.0, icon, icon, [0.0, 16.0, 16.0, 16.0], (false, false), c); pen += icon + 2.0; }
            Some('B') => { h.quad(Anchor::Left, -2, atlas, 0, pen, y - 2.0, icon, icon, [16.0, 0.0, 16.0, 16.0], (false, false), c); pen += icon + 2.0; }
            Some('V') => {
                h.quad(Anchor::Left, -2, atlas, 0, pen + icon * 0.25, y - 3.0, icon * 0.5, icon * 0.5, [16.0, 0.0, 16.0, 16.0], (false, false), c);
                h.quad(Anchor::Left, -2, atlas, 0, pen + icon * 0.25, y + icon * 0.5 - 1.0, icon * 0.5, icon * 0.5, [32.0, 0.0, 16.0, 16.0], (false, false), c);
                pen += icon + 2.0;
            }
            Some('H') => {
                h.quad(Anchor::Left, -2, atlas, 0, pen, y - 2.0, icon * 0.5, icon, [48.0, 0.0, 16.0, 16.0], (false, false), c);
                h.quad(Anchor::Left, -2, atlas, 0, pen + icon * 0.5, y - 2.0, icon * 0.5, icon, [48.0, 16.0, 16.0, 16.0], (false, false), c);
                pen += icon + 2.0;
            }
            Some('S') | Some('W') => { h.quad(Anchor::Left, -2, atlas, 0, pen, y - 2.0, icon, icon, [48.0, 16.0, 16.0, 16.0], (false, false), c); pen += icon + 2.0; }
            _ => { pen += icon; }
        }
        rest = &after[2.min(after.len())..];
        if rest.starts_with(' ') { rest = &rest[1..]; pen += 4.0; }
    }
    if !rest.is_empty() { h.text(Anchor::Left, font, -2, rest, pen, y, false, c, true); }
}

/// A textured quad in the menu's 640 × 480 space.
#[allow(clippy::too_many_arguments)]
fn quad640(h: &mut Hud, tex: u32, x: f32, y: f32, w: f32, hh: f32, uv: [f32; 4], c: [f32; 4]) {
    h.quad(Anchor::Stretch, 0, tex, 0, mx(x), my(y), mx(w), my(hh), uv, (false, false), c);
}

/// A flat coloured plate (the menu backdrop / the confirm box) — degenerate UVs on the font texture,
/// like the dev overlay's `ndc_quad` [M hud.rs].
fn plate(h: &mut Hud, x: f32, y: f32, w: f32, hh: f32, c: [f32; 4]) {
    let (a, b) = (h.ndc(Anchor::Stretch, x, y), h.ndc(Anchor::Stretch, x + w, y + hh));
    let p = |q: [f32; 2]| HudVertex { p: q, uv: [0.03, 0.03], c };
    let v = [p([a[0], a[1]]), p([b[0], a[1]]), p([b[0], b[1]]), p([a[0], a[1]]), p([b[0], b[1]]), p([a[0], b[1]])];
    h.ndc_quad(-5, v, c);
}

/// Naive greedy word wrap for the legal / confirm / description plates (the game's own text renderer
/// wraps; `Font::layout` draws one line).
fn wrap(s: &str, cols: usize) -> Vec<String> {
    let mut out = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        for w in para.split_whitespace() {
            if !line.is_empty() && line.chars().count() + w.chars().count() + 1 > cols {
                out.push(std::mem::take(&mut line));
            }
            if !line.is_empty() { line.push(' '); }
            line.push_str(w);
        }
        out.push(line);
    }
    out
}

fn page_rows(h: &mut Hud, fe: &FrontEnd, font: &nf_formats::hud::Font, head: &str, row_c: [f32; 4], sel_c: [f32; 4]) {
    h.text(Anchor::Centre, font, -4, head, SW / 2.0, my(80.0), false, sel_c, true);
    page_rows_no_head(h, fe, font, my(140.0), row_c, sel_c);
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
        KeyCode::Tab | KeyCode::KeyE => Action::Side,
        _ => return None,
    })
}
