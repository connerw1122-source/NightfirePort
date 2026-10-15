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
/// The Dossier's Records table [M menu-def PAGE 0x4000003a (parsed 2026-10-14: header LBL 0x280,
/// hint LBL 0x49e, the mission list 0x10000174 (132,124,376×16), the three score rows + the medal
/// image 0x10000176 (435,282,128×64) TEX 0x0300013d); P_DOSSIER 0x216de0's rows open it
/// [M C_SBDOSSIER 0x216eb0:12-31 — index 0 → 0x4000003a]].
pub const PAGE_RECORDS: u32 = 0x4000_003a;
/// The title layout the boot page leads to [M menu-def page 0x40000009: the art + Press START].
pub const PAGE_TITLE_LAYOUT: u32 = 0x4000_0009;
pub const PAGE_OPTIONS: u32 = 0x4000_0068;
/// The IN-GAME pause page [M Player_Update 0x193600:73 — pressing Start in a mission runs
/// `MenuManager_Create(0x80000002, 0x4000004b, …, 0x1e, 1, 3)` + `GS_PauseGame(1)`; the page lives in
/// the MISSION archives' own kind-8 menu def (entry 08000001 of each level archive, e.g. 07000001.bin
/// [L parsed 2026-10-14]), NOT the front-end 07000048 stream — the old 0x4000000c id was the NIS page
/// (the handler table maps 0x4000000c → P_NIS_Handler [M Handler_HandleMessage 0x2029e0])]. Its main
/// control is 0x10000028, handled by C_GCPAUSE_Handler 0x204408.
pub const PAGE_PAUSE: u32 = 0x4000_004b;
/// The fail/death menu [M Mission_Update 0x182c60:78 — `MenuManager_Create(0x80000004, 0x40000042, …)`;
/// handler P_ENDMISSION_Handler 0x216618; same mission menu def, PAGE record fh=0x80000004 [L]].
pub const PAGE_FAIL_MENU: u32 = 0x4000_0042;
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
    /// `BGFMVPlaying` [L 0x30c69c]: the host sets this while a real movie decodes; state 6 holds
    /// while it is set and pops when it clears — `psiMovieFinished` is `BGFMVPlaying == 0`
    /// [M 0x1082e8, GameFlow_Main 0x1c98c8 case 6]. Never set by the countdown model.
    pub movie_playing: bool,
    /// A movie-wrapped load is on its way to state 6: `reset_map_level_to_load` set it when the
    /// level wrapped to a 0x71/0x73 code [M 0x1d3800's switch], `load_tick` phase 2 clears it.
    /// The host defers the level load while it stands (the game loads the backdrop, the movie, and
    /// only then the real level — 0x1d6360 phases 0/3).
    pub movie_pending: bool,
    /// The level phase 2 resolved the movie code to [M 0x1d6360 LoadState 2,
    /// `GetLevelToLoadFromMovieCode` 0x1d3560]. The host reads and clears it to schedule the real
    /// level load after the movie (and to drop the front end when it is a mission).
    pub phase2_resolved: Option<u32>,
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
            // Movie: pops when the movie finishes or the skip action fires [M case 6,
            // Input_Action(-1, 0x19, 1)]. With a real movie playing (BGFMVPlaying != 0) the state
            // holds — psiMovieFinished is BGFMVPlaying == 0 [M 0x1082e8], and the host clears
            // movie_playing when the decoder ends; the skip goes through skip_movie. The frame
            // countdown only runs when no real movie ever started (the slim ISO's zeroed PSS
            // files, fix-menus' headless model).
            ST_MOVIE => {
                if self.movie_playing { return; }
                self.movie_frames = self.movie_frames.saturating_sub(1);
                if self.movie_frames == 0 { self.pop_state(); }
            }
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
            // movie codes back to level codes; 0x07100018 → 0x0700001b is the one special case, and
            // every 0x73 outro code routes to the front-end map 0x7000048 — 0x1d3560's switch].
            let mc = self.movie_code;
            self.movie_code = 0;
            self.movie_pending = false;
            let resolved = match mc {
                // [M 0x1d3560's switch] 0x07100018 → 0x0700001b is the one 0x71 special case, and the
                // enumerated 0x73 codes route to the front-end map 0x7000048 (0x730000f/10 are in the
                // routing list even though 0x730000f is not whitelisted); everything else strips the
                // 0x71/0x73 middle byte (the default arm: `GameState._8_4_ & 0xff00ffff`).
                0x0710_0018 => 0x0700_001b,
                0x0730_0004 | 0x0730_0008 | 0x0730_000b | 0x0730_000d | 0x0730_000f | 0x0730_0010 | 0x0730_0013 | 0x0730_0017 | 0x0730_001b => FRONTEND_LEVEL,
                _ => 0x0700_0000 | (mc & 0xffff),
            };
            self.level_to_load = resolved;
            self.phase2_resolved = Some(resolved);
            // Phase 2 sets +0x28 = 0x40000036 when the resolve lands on the map [M 0x1d6360:
            // `if (uVar4 == 0x7000048) { GameState._28_4_ = 0x40000036; }`] — the results page the
            // outro returns to (Mission_Update 0x182c60 case 4 set it too).
            if resolved == FRONTEND_LEVEL { self.page = PAGE_RESULTS; }
            return false;
        }
        let level = self.level_to_load;
        // Phase 0: only the 0x71/0x73 intro-wrapped codes record a movie code [M 0x1d6360 "the level
        // still carries the 0x71/0x73 prefix"]. The driving codes 0x09xxxxxx load directly.
        if matches!((level >> 20) & 0xff, 0x71 | 0x73) {
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
        // other wrapper/multiplayer codes are not in the switch and pass through. The switch is
        // compiled code — 22 `lui rt,0x710` words at 0x1d392c-0x1d3a40, no jump table [L] — with
        // identity on the low byte for 0x7000001..0x7000016 and two special cases:
        // 0x700001b → 0x7100018 (the Equinox briefing is movie 18; 0x0710001b is not on the disc
        // [D /MOVIES/30_FPS]) and 0x700004a → 0x710004a.]
        let plain = level & 0xff00_ffff;
        let code = if !no_intro && level & 0xff0000 == 0 && ((0x0700_0001..=0x0700_0016).contains(&plain) || plain == 0x0700_001b || plain == 0x0700_004a) {
            if plain == 0x0700_001b { 0x0710_0018 } else { level + 0x0010_0000 }
        } else { level };
        self.level_to_load = code;
        // A wrapped load plays its movie first: the host defers the real load until phase 2
        // resolves [M 0x1d6360 phase 0 → state 6 → phase 2]. (The 0x71/0x73 prefix is the
        // (code >> 20) & 0xff byte — 0x07300004 >> 20 = 0x73 — the same test load_tick phase 0
        // makes.)
        self.movie_pending = matches!((code >> 20) & 0xff, 0x71 | 0x73);
        self.push_state(ST_MAPLOAD);
    }

    /// GameFlow_Main case 6's skip arm: the skip action (0x19, newly pressed) ends the movie
    /// [M 0x1c98c8 case 6 — `Input_Action(-1, 0x19, 1)` → `psiStopBackgroundMovie` → pop]. The
    /// host feeds it from the Confirm keys (the port's stand-ins for the pad's X/Start [S,
    /// docs/systems/input.md §3]) and stops the decoder with it.
    pub fn skip_movie(&mut self) -> bool {
        if self.get_state() == ST_MOVIE { self.pop_state(); true } else { false }
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

/// The campaign [M `sp_level` 0x2df2e0's 12 × 0x18-byte records: +0x00 the row's ring-preview texture
/// hash (a 0x03xxxxxx archive id, NOT a Txt label — `Menu_UpdateWheel` 0x1f8388 sends it as msg 0x24 to
/// the Single Player page's preview control 0x10000020 [M 0x20b94c-0x20b984 call site, 0x1f849c-0x1f84b4
/// msg send]); +0x04 name label, +0x08 description label (bank-3 Txt ids), +0x0c level, +0x10 unlocked,
/// +0x14 the locked rows' description label (0x01000009 [L USATxt "Continue playing to unlock this
/// mission."]); the record bytes read back from the EE snapshot at 0x2df2e0 [L work/ee_snapshot.bin].
/// The port keeps the game's level codes verbatim — the 0x09xxxxxx rows are the driving maps
/// (`C_SBNFMAP` passes them straight to `ResetMap_LevelToLoad` [M 0x20b8d8:21-24]).
pub const CAMPAIGN: [(u32, u32, u32, u32); 12] = [
    // (ring-preview texture, name label, description label, level) = sp_level[i] +0x00/+0x04/+0x08/+0x0c
    (0x0300_0085, 0x0300_0001, 0x0300_0013, 0x0900_0001), // Paris Prelude
    (0x0300_0084, 0x0300_0002, 0x0300_0014, 0x0700_0005), // The Exchange
    (0x0300_00b0, 0x0300_0003, 0x0300_0015, 0x0900_0005), // Alpine Escape
    (0x0300_00b2, 0x0300_0004, 0x0300_0016, 0x0900_0006), // Enemies Vanquished
    (0x0300_008c, 0x0300_0005, 0x0300_0017, 0x0700_0001), // Double Cross
    (0x0300_008e, 0x0300_0006, 0x0300_0018, 0x0700_0009), // Night Shift
    (0x0300_008d, 0x0300_0007, 0x0300_0019, 0x0700_000c), // Chain Reaction
    (0x0300_008f, 0x0300_0009, 0x0300_001b, 0x0700_0011), // Phoenix Fire
    (0x0300_00ae, 0x0300_000c, 0x0300_001e, 0x0900_0002), // Deep Descent
    (0x0300_00af, 0x0300_000e, 0x0300_0020, 0x0900_0003), // Island Infiltration
    (0x0300_0151, 0x0300_0034, 0x0300_0035, 0x0700_0014), // 'Countdown' [L USATxt 0x03000034]
    (0x0300_0142, 0x0300_0011, 0x0300_0023, 0x0700_001b), // 'Equinox' [L USATxt 0x03000011]
];
/// sp_level[i]+0x14 — the description a LOCKED row shows [L ee_snapshot 0x2df2e0+0x14 = 0x01000009 on
/// every locked row; USATxt 0x01000009 = "Continue playing to unlock this mission."] [M
/// `Menu_UpdateWheel` binds +0x08 when unlocked, +0x14 when not].
pub const LOCKED_DESC: u32 = 0x0100_0009;

/// A front-end page's aux background-movie code [M the menu-def PAGE record's third dword, stored at
/// M_PAGE+0xC4 by `MenuManager_Create` 0x21b908:169-183 and played as a LOOPING background movie on
/// page change by `Manager_SendMessage` 0x21c858:338 (`Menu_PlayMovie(aux, 0, 1, 0, 0, 0)` — loop = 1;
/// `psiStopBackgroundMovie` when the new page's aux is 0)]. 0 = no movie. Parsed from the disc's
/// menu-def (08000002 in 07000048.bin) 2026-10-13: the main menu carries 0x07330048; the boot page
/// 0x40000034, every list page (codename 0x40000025, difficulty 0x40000023, missions 0x4000001c,
/// dossier 0x4000002b) and the results page 0x40000036 carry 0x07350048; P_INTRO 0x40000032, the
/// title layout 0x40000009 and attract 0x40000035 carry 0 (P_INTRO plays 0x7370048/0x7380048 from
/// its handler [M 0x209e88]). The test
/// `menu_movies::the_page_aux_movie_map_comes_from_the_page_data` re-parses the disc's PAGE records
/// and pins this mapping to them.
pub fn aux_movie_for(page_id: u32) -> u32 {
    match page_id {
        PAGE_MAIN => 0x0733_0048,
        PAGE_START | PAGE_CODENAME | PAGE_DIFFICULTY | PAGE_MISSION_SELECT | PAGE_DOSSIER | PAGE_RESULTS => 0x0735_0048,
        _ => 0,
    }
}

/// Mission_Update 0x182c60 case 6's level → outro-code switch [M; snapshot bytes 0x182e60-0x182eb4
/// are lui/ori pairs, no table [L ee_snapshot]]: 0x7000004/08/0b/0d/0f/13/1b map to their own 0x73
/// codes, 0x7000016 → 0x7300017 (the one non-identity entry), everything else → the front-end map
/// (no outro). 0x730000f is not in the `UsingMovie` whitelist [M 0x107eb8], so level 0x700000f's
/// "outro" exits state 6 on frame 1 — its explicit routing entry exists for exactly that.
pub fn outro_for(level: u32) -> u32 {
    match level & 0xff00_ffff {
        0x0700_0004 => 0x0730_0004,
        0x0700_0008 => 0x0730_0008,
        0x0700_000b => 0x0730_000b,
        0x0700_000d => 0x0730_000d,
        0x0700_000f => 0x0730_000f,
        0x0700_0013 => 0x0730_0013,
        0x0700_0016 => 0x0730_0017,
        0x0700_001b => 0x0730_001b,
        _ => FRONTEND_LEVEL,
    }
}

/// The difficulty choices [M Txt 0x14d/0x14e/0x14f = "Operative"/"Agent"/"00 Agent" (USATxt.dat), the
/// order of menu-def page 0x40000023's wheel rows; the `difficulty` global is 0x2df4c0 [M symtab]].
pub const DIFFICULTIES: [u32; 3] = [0x14d, 0x14e, 0x14f];
/// Each difficulty's description line [M Txt 0x10000006/7/8].
pub const DIFFICULTY_DESCS: [u32; 3] = [0x100_0006, 0x100_0007, 0x100_0008];
/// The main menu's item labels and one-line descriptions [M menu-def page 0x40000002 rows + Txt].
pub const MAIN_ITEMS: [u32; 3] = [0x28, 0x149, 0x14b];
pub const MAIN_DESCS: [u32; 3] = [0x100_0000, 0x100_0001, 0x100_0002];

/// The intro movies the P_INTRO page plays between the title and the front end, in order
/// [M `P_INTRO_Handler` 0x209e88:38-62: `psiStartBackgroundMovie(0x7370048, 0, vol)` then
/// `psiStartBackgroundMovie(0x7380048, 0, vol)` (loop = 0), each waited on with
/// `psiMovieFinished` 0x001082e8; the page change to the next page only after the second ends].
pub const INTRO_MOVIES: [u32; 2] = [0x0737_0048, 0x0738_0048];

/// The headless intro budget: the two movies at 30 fps, one frame per two 60 Hz ticks [M the game
/// decodes one MPEG frame per game tick and skips to even vblanks — 30 fps NTSC, main 0x00109080].
/// Measured on the retail image 2026-10-13: 0x07370048 = 96 pictures (3.2 s, the EON logo),
/// 0x07380048 = 329 pictures (11 s) — 425 × 2 = 850 ticks. The test
/// `menu_movies::the_intro_movie_counts_pin_the_tick_budget` re-measures the disc's files against it.
pub const INTRO_TICKS: u32 = 850;

/// Which front-end page is up. The ids are the game's own (the constants above); the enum is the
/// dispatcher the port drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page { Start, Main, Codename, Difficulty, MissionSelect, Dossier, Records, Options, Pause, Results }

impl Page {
    pub fn id(self) -> u32 {
        match self {
            Page::Start => PAGE_START, Page::Main => PAGE_MAIN, Page::Codename => PAGE_CODENAME,
            Page::Difficulty => PAGE_DIFFICULTY, Page::MissionSelect => PAGE_MISSION_SELECT,
            Page::Dossier => PAGE_DOSSIER, Page::Records => PAGE_RECORDS,
            Page::Options => PAGE_OPTIONS, Page::Pause => PAGE_PAUSE, Page::Results => PAGE_RESULTS,
        }
    }
    fn from_id(id: u32) -> Option<Page> {
        Some(match id {
            PAGE_START => Page::Start, PAGE_MAIN => Page::Main, PAGE_CODENAME => Page::Codename,
            PAGE_DIFFICULTY => Page::Difficulty, PAGE_MISSION_SELECT => Page::MissionSelect,
            PAGE_DOSSIER => Page::Dossier, PAGE_RECORDS => Page::Records,
            PAGE_OPTIONS => Page::Options, PAGE_PAUSE => Page::Pause, PAGE_RESULTS => Page::Results,
            _ => return None,
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

/// The results page's fields, built on page enter from the live progress layer — P_NFRESULTS
/// 0x208e78 case 0x4c: `PlrStat_GetScore(0)` (the commit), the medal compare of the last score
/// (ScoringTable+0x34) against the thresholds (+0x4 bronze .. +0x10 platinum), `SeparateNumber`
/// for the two numbers, and `Menu_SetLevelBonus(GameState+0x72, medal, 0)` [M :77-169].
#[derive(Clone, Debug, Default)]
pub struct ResultsFields {
    /// The score line's number: ScoringTable+0x34 (the last score) [M :96].
    pub score: u32,
    /// The target line's number: the threshold ABOVE the score — the medal just missed's own
    /// threshold, platinum shows its own [M :104-136].
    pub target: u32,
    /// The target line's label [M :108/115/123/131/139 — 0x2cc platinum .. 0x2cf bronze].
    pub target_label: u32,
    /// The medal name label [M :138/130/122/114 — 0x2c7 platinum, 0x2c8 gold, 0x2c9 silver,
    /// 0x2ca bronze]; 0x10001fd composes the no-medal line [M :144-154].
    pub medal_label: u32,
    /// The medal plate's final text: the bound medal name, or the composed no-medal line.
    pub medal_text: String,
    /// The emblem texture the photo control shows [M :109/116/124/132/139 — msg 0x24 into
    /// 0x10000166: 0x300013d bronze, 0x300013e gold, 0x300013f platinum, 0x300019a none].
    pub emblem: u32,
    /// The awarded medal counter (1 bronze .. 4 platinum, 5 none) — Menu_SetLevelBonus's medal
    /// [M :169].
    pub medal: u32,
}

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
    /// choose (`state_quit_166` [M C_GCPAUSE_Handler 0x204408:210-220]). RESTART confirms the same
    /// way (the same YES/NO list, text 0x1ff [M 0x204408's msg-0x4b d1=6 cases bind 0x1ff/0x200 and
    /// `state_quit_166` picks the arm]).
    pub confirming_quit: bool,
    /// The pause page's RESTART confirm (the same YES/NO box; text 0x1ff "RESTART: ARE YOU SURE?"
    /// [M 0x204408 msg 0x4b case 1]).
    pub confirming_restart: bool,
    /// The YES/NO list's row: starts on NO (the page data's initial selection m0x1d:1 [L page
    /// 0x4000004b control d1=0xd; the game re-sends msg 0x1d=1 on every open [M 0x204408:181/204]).
    pub confirm_sel: usize,
    /// The pause page's tab: 0 MISSION / 1 OBJECTIVES / 2 CONTROLS / 3 SCORE [L page 0x4000004b's
    /// four type-1 controls LBL 0x167/0x176/0x168/0x169].
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
    /// The P_INTRO movie run started by the title's Start: true from the confirm until the last
    /// intro movie finishes (or the skip action pops it) [M 0x209e88]. While it is up the overlay
    /// draws the intro movie fullscreen (the intro page has no controls) and Confirm skips.
    pub intro_playing: bool,
    /// Which of [`INTRO_MOVIES`] is up [M 0x209e88's `movie_283` counter: 1 → 0x07370048,
    /// 2 → 0x07380048, then the page change].
    pub intro_index: usize,
    /// fix-menu-movies2: the page's aux backdrop movie has a live frame on the ONE movie plane
    /// (fix-movie-playback's `movie_tex`) this frame — the overlay then draws no dark plate (the
    /// plane IS the backdrop, under these quads); without it the plate stands in (the pre-movies
    /// look, and every image without movie data). The host sets it per frame.
    pub backdrop_live: bool,
    /// The pause page's tab: 0 MISSION / 1 OBJECTIVES / 2 CONTROLS / 3 SCORE [L page 0x4000004b's
    /// four type-1 controls LBL 0x167/0x176/0x168/0x169].
    pub pause_tab: usize,
    /// fix-pause-menu2: the OBJECTIVES tab's rows — (label id, done) per visible objective, filled
    /// by the host each frame from the sim's mission state [M C_GCPAUSE_Handler 0x204408:251-271
    /// fills the game's list from Mission_ObjectiveState; state 4 = the ~J done marker].
    pub pause_objectives: Vec<(u32, bool)>,
    /// fix-save-profile: the port's memory card — the profiles dir next to the port's config,
    /// one file per codename [M Menu_UpdateMessageBox 0x1f7de8:19/28 passes the codename buffer
    /// 0x2e1ae8 to LS_Load/LS_Save as the file name]. The host points it at the dir; None keeps
    /// the headless behaviour (no card).
    pub profile_dir: Option<std::path::PathBuf>,
    /// fix-save-profile: the saved codenames in the game's row order — the two defaults, then
    /// the card's rows [M Menu_EnumSaves 0x1f7b78:24-38 copies each enumerated name after
    /// def_codename's two rows and appends it to the control].
    pub profiles: Vec<String>,
    /// fix-save-profile: the codename in play — None for "Play without saving" (the game maps a
    /// template onto the slot [M Menu_MapDefaultCodename 0x1f7fa8] but never writes a file).
    pub active_profile: Option<String>,
    /// fix-save-profile: the active profile's data. The Records page reads its progress (the
    /// level's best at ScoringTable+0x30 [M C_RBDSRECORDS 0x20bf00:26]); the host syncs the
    /// sim's tally in with `sync_profile_progress` before the results save.
    pub profile: Option<nf_game::profile::Profile>,
    /// The results page's fields, built by `results_enter` (P_NFRESULTS case 0x4c). None until
    /// the page enter runs against the live progress layer.
    pub results: Option<ResultsFields>,
    /// `just_completed` 0x30d344 — Menu_SetLevelBonus's return: the FINAL mission (0x0700001b)
    /// just finished and the results page routes to the win page [M 0x208e78:40-52, 0x1ffa10
    /// tail]. Not ported past the flag (the win page is reported).
    pub just_completed: bool,
    /// `DAT_002e1c2c` — a new reward unlocked at this results page [M 0x208e78:169 → 0x1ffa10
    /// :74-79]: the page's Confirm routes to the bonus page 0x40000038 while it stands (not
    /// ported — reported).
    pub new_reward: bool,
}

impl FrontEnd {
    pub fn new() -> Self {
        let flow = GameFlow::boot();
        let page = Page::from_id(flow.page).unwrap_or(Page::Start);
        let mut unlocked = [false; 12];
        // A fresh profile has rows 0 AND 1 unlocked [D sp_level's boot state in ACTION.ELF .data (file
        // offset 0x1d91e0): rows 0-1 hold +0x10 = 1, rows 2-11 hold 0; the ee_start snapshots captured a
        // post-unlock session and read 1 everywhere].
        unlocked[0] = true;
        unlocked[1] = true;
        FrontEnd { flow, page, cursor: 0, mission: 0, difficulty: 0, no_input_frames: 0, press_start_on: true, outcome: Outcome::Failed, stats: Stats::default(), requested_level: None, confirming_quit: false, confirming_default_profile: false, unlock_all: false, unlocked, closed: false, intro_playing: false, intro_index: 0,
            backdrop_live: false,
            // fix-pause-menu: the pause page's own state (each on its line [WORKLOG lesson 4]).
            confirming_restart: false,
            confirm_sel: 1,
            pause_tab: 0,
            // fix-pause-menu2 (each new field on its own line [WORKLOG lesson 4]).
            pause_objectives: Vec::new(),
            // fix-save-profile (each new field on its own line [WORKLOG lesson 4]).
            profile_dir: None,
            profiles: Vec::new(),
            active_profile: None,
            profile: None,
            // front-end-chain (each new field on its own line [WORKLOG lesson 4]).
            results: None,
            just_completed: false,
            new_reward: false }
    }

    pub fn rows(&self) -> usize {
        match self.page {
            Page::Start => 1,
            Page::Main => MAIN_ITEMS.len(),
            // fix-menus-real: the game's own rows — Menu_EnumSaves 0x1f7b78:58-66 fills slot 0 from
            // def_codename[0] = Txt 0x29b "New Codename" and slot 1 from def_codename[1], and
            // Menu_PutCodenamesIntoControl 0x1f79c0:15-23 overwrites slot 1 with Txt 0x10002ff "Play
            // without saving" on this page; fix-save-profile: the card's rows follow (one per
            // saved codename [M Menu_EnumSaves 0x1f7b78:24-38 appends each enumerated name]).
            Page::Codename => 2 + self.profiles.len(),
            Page::Difficulty => DIFFICULTIES.len(),
            Page::MissionSelect => CAMPAIGN.len(),
            Page::Records => CAMPAIGN.len(),        // the Records list walks the campaign [M 0x217248:11-13]
            // fix-menus-real: the dossier wheel's four rows from the game's own table 0x2dfcd0
            // [L ACTION.ELF .data dump: {0x300019b, 0x280 Records}, {0x300019c, 0x281 Rewards},
            // {0x30000c4, 0x282 Gadgets}, {0x30000ee, 0x19c Weapons}; C_SBDOSSIER 0x216eb0:48-49
            // walks it].
            Page::Dossier => 4,
            Page::Options => 3,                    // the options page stubs three rows [S]
            // The MISSION tab's rows [L page 0x4000004b's d1=6 list (153,183,334×120) binds labels
            // 0x16a CONTINUE / 0x16b RESTART / 0x16c QUIT via its m0x11 messages].
            Page::Pause => 3,
            // The FAILED results page is P_ENDMISSION's three rows [M 0x216618:19-27]; the
            // COMPLETED results page (P_NFRESULTS 0x208e78) has NO row list — the page's own
            // action is the hint's ~A Continue (Confirm anywhere).
            Page::Results => if self.outcome == Outcome::Complete { 1 } else { 3 },
        }
    }

    pub fn row_label<'a>(&self, lang: Option<&'a Lang>, row: usize) -> String {
        let bind = |id: u32| lang.and_then(|l| l.bind(id)).map(str::to_string).unwrap_or_else(|| format!("<{id:#x}>"));
        match self.page {
            Page::Start => bind(0x100_01eb),       // "Press the START button" [M 0x20a408]
            Page::Main => bind(MAIN_ITEMS[row.min(2)]),
            // (CAMPAIGN's name label is +0x04 [M Menu_UpdateWheel 0x1f8388 fills the wheel rows from it].)
            // Row 0 = "New Codename" (Txt 0x29b [M Menu_EnumSaves 0x1f7b78:58-66]), row 1 = "Play
            // without saving" (Txt 0x10002ff [M Menu_PutCodenamesIntoControl 0x1f79c0:15-23]).
            // fix-save-profile: rows 2+ are the card's codenames themselves [M the enumerated
            // rows ARE the names — Menu_EnumSaves copies them straight into the control].
            Page::Codename if row >= 2 => self.profiles.get(row - 2).cloned().unwrap_or_default(),
            Page::Codename => bind([0x29b, 0x100_02ff][row.min(1)]),
            Page::Difficulty => bind(DIFFICULTIES[row.min(2)]),
            Page::MissionSelect => bind(CAMPAIGN.get(row).map_or(0, |c| c.1)),   // +0x04 name [M 0x1f8388]
            Page::Records => bind(CAMPAIGN.get(row).map_or(0, |c| c.1)),          // the list is sp_level's names [M 0x217248:11-13]
            // The dossier wheel's own name labels [L table 0x2dfcd0; C_SBDOSSIER 0x216eb0:48-49].
            Page::Dossier => bind([0x280, 0x281, 0x282, 0x19c][row.min(3)]),
            Page::Options => ["Screen adjust", "Controls", "Back"][row.min(2)].into(),
            // The pause rows are the page data's own labels [L page 0x4000004b's d1=6 list m0x11:
            // 0x16a/0x16b/0x16c — the game's letter-spaced strings]. The ACTIONS are the handler's
            // [M C_GCPAUSE_Handler 0x204408: d1=6 case 0 → MenuManager_Delete (resume), case 1 → the
            // RESTART confirm then ResetMap_LevelToLoad(BaseMap, 0, 1), case 2 → the QUIT confirm
            // (state_quit_166) then 0x7000048].
            Page::Pause => bind([0x16a, 0x16b, 0x16c][row.min(2)]),
            // The FAILED page's three rows in the game's arm order [M P_ENDMISSION_Handler
            // 0x216618:19-27 — `__Menu_Send`'s return IS the row index, so row 0 is the arm-0
            // behaviour]: row 0 = Restart level (the CURRENT level, warm, on the entry checkpoint —
            // fix-restart-target); row 1 = Restart mission (Mission_BaseMapHCode(), cold); row 2 =
            // Quit (0x7000048). The labels are [S — the disc page data holds the originals.]
            Page::Results if self.outcome == Outcome::Failed =>
                ["Restart level", "Restart mission", "Quit to menu"][row.min(2)].into(),
            // The COMPLETED page draws no rows (P_NFRESULTS has none [L page 0x40000036's
            // controls]); rows() is 1 for the wheel arithmetic only.
            Page::Results => String::new(),
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
        // The intro run ends when the movie state pops (its frames ran out or the host ended it
        // after the second intro movie) [M 0x209e88: the page change follows the movies].
        if self.intro_playing && self.flow.get_state() != ST_MOVIE {
            self.intro_playing = false;
            self.intro_index = 0;
        }
        // The title/main 240-frame flash [M 0x20a408's delayed 0x100000fa toggle at 0xf0 frames].
        if matches!(self.page, Page::Start | Page::Main) {
            self.press_start_on = self.flow.ticks % 240 < 120;
        }
        self.no_input_frames = self.no_input_frames.saturating_add(1);
    }

    /// The pause page opens over a running mission (the game's GS_PauseGame [M 0x17d178 pauses the
    /// sim; Player_Update 0x193600:73 opens page 0x4000004b]; the port only tracks the page — the
    /// caller pauses its own loop, and the sound hold:
    /// `MenuManager_Create` with `pause_sound` runs `SFXPause`+`SFXPauseAllStreams` [M 0x21b908:121-124],
    /// which the host applies through `Sound::set_paused` when this page comes up).
    pub fn open_pause(&mut self) -> bool {
        if self.page != Page::Pause && self.flow.current_level != FRONTEND_LEVEL && self.flow.current_level != 0 {
            self.page = Page::Pause;
            self.cursor = 0;
            self.pause_tab = 0;
            self.confirming_restart = false;
            self.confirming_quit = false;
            self.confirm_sel = 1;   // the YES/NO list opens on NO [L m0x1d:1; M 0x204408:181/204]
            true
        } else { false }
    }

    /// A mission ended; the game records the results page and pushes the 60-frame fade [M 0x182c60:74
    /// sets +0x28 = 0x40000036, `GameFlow_PushState(60.0, 7, …)`]. The unlock itself is NOT here:
    /// Mission_Update's case 4 does no award — the results page's enter runs
    /// `Menu_SetLevelBonus(GameState+0x72, medal, 0)` [M 0x208e78:169], which the port drives
    /// through [`FrontEnd::results_enter`].
    pub fn mission_ended(&mut self, outcome: Outcome, stats: Stats) {
        self.outcome = outcome;
        self.stats = stats;
        self.flow.page = PAGE_RESULTS;
        // Mission_Update 0x182c60 case 6 sets `LevelToEndTo` from the finished level: the outro
        // code 0x073000xx for the levels with an outro (compiled lui/ori switch at 0x182e60-0x182eb4
        // [L ee_snapshot]; 0x7000016 → 0x7300017 is the one non-identity entry), the front-end map
        // for the rest (no outro); failures go straight to the map [M cases 2/3]. Case 4 then runs
        // `ResetMap_LevelToLoad(LevelToEndTo, 0, 0)` — the outro rides the briefing machinery, so
        // the movie wrap plays it and phase 2 returns to the map with the results page
        // [M 0x1d6360/0x1d3560; P_NFRESULTS 0x208e78 reopens 0x40000036 on it].
        let end_to = if outcome == Outcome::Complete { outro_for(self.flow.current_level) } else { FRONTEND_LEVEL };
        self.requested_level = Some(end_to);
        self.flow.reset_map_level_to_load(end_to, false, false);
        // The game's order [M 0x182c60 case 4]: `ResetMap_LevelToLoad(LevelToEndTo, 0, 0)` pushes the load
        // first, THEN `GameFlow_PushState(60.0, 7, …)` puts the fade on top, so the fade plays before the load.
        self.flow.push_state(ST_FADE_HOLD);
        self.page = Page::Results;
        self.cursor = 0;
    }

    /// P_NFRESULTS 0x208e78 case 0x4c against the LIVE progress layer — the results page's enter:
    /// `PlrStat_GetScore(0)` (the commit — the game's param 0 [M :77]) builds the page's fields,
    /// then `Menu_SetLevelBonus(GameState+0x72, medal, 0)` writes the award — the level's
    /// `sp_level+0x10` flag AND the next level's [M :169 → 0x1ffa10:62-66], which
    /// `Menu_GetNightfireStatus` 0x202360 packs for the save. `base_map` = GameState+0x72
    /// (`Mission_BaseMapHCode`), `status` = `Mission_Status()` (6 on this path), `difficulty` =
    /// GameState+0x40. The no-medal line composes Txt 0x10001fd with the codename (len > 1) or
    /// Txt 0x10001fe [M :144-154]. Returns whether the fields were built (the game's
    /// score_table != 0 arm — always true for the SP hashes).
    pub fn results_enter(&mut self, progress: &mut nf_game::progress::Progress, base_map: u32, status: u32, difficulty: u32, lang: Option<&Lang>) -> bool {
        let scored = progress.get_score(base_map, status, difficulty, true); // [M :77 — GetScore(0)]
        let bind = |id: u32| -> String {
            lang.and_then(|l| l.bind(id)).map(str::to_string).unwrap_or_else(|| format!("<{id:#x}>"))
        };
        let fields = scored.map(|s| {
            let idx = progress.table.iter().position(|e| e.hash == base_map).unwrap();
            let th = progress.table[idx].thresholds;
            // The medal compare on the last score, with the target it shows — the NEXT threshold,
            // labelled by the medal just missed [M :98-142]:
            // platinum (≥ +0x10): target +0x10, name 0x2c7, label 0x2cc, emblem 0x300013f [M :135-141];
            // gold (≥ +0xc): target +0x10, name 0x2c8, label 0x2cc, emblem 0x300013e [M :127-133];
            // silver (≥ +8): target +0xc, name 0x2c9, label 0x2cd, emblem 0x3000140 [M :119-125];
            // bronze (≥ +4): target +8, name 0x2ca, label 0x2ce, emblem 0x300013d [M :111-117];
            // none: target +4, name 0x10001fd (composed), label 0x2cf, emblem 0x300019a [M :104-110].
            let (medal_label, target, target_label, emblem, medal) = if s.last >= th[3] {
                (0x2c7u32, th[3], 0x2ccu32, 0x0300_013fu32, 4u32)
            } else if s.last >= th[2] {
                (0x2c8, th[3], 0x2cc, 0x0300_013e, 3)
            } else if s.last >= th[1] {
                (0x2c9, th[2], 0x2cd, 0x0300_0140, 2)
            } else if s.last >= th[0] {
                (0x2ca, th[1], 0x2ce, 0x0300_013d, 1)
            } else {
                (0x100_01fd, th[0], 0x2cf, 0x0300_019a, 5)
            };
            // The medal plate's text [M :144-158]: a real medal binds its name; the no-medal line
            // is a FORMAT string (Txt 0x10001fd, "%s") filled with the codename when the buffer
            // holds one (DAT_002e1ae0 > 1 [M :145/151]) or with Txt 0x10001fe ("the next agent").
            let codename = self.active_profile.clone().or_else(|| self.profile.as_ref().map(|p| p.name.clone()));
            let medal_text = if medal_label == 0x100_01fd {
                let fmt = bind(0x100_01fd);
                let sub = match codename.filter(|c| c.chars().count() > 1) {
                    Some(c) => c,
                    None => bind(0x100_01fe),
                };
                fmt.replacen("%s", &sub, 1)
            } else {
                bind(medal_label)
            };
            ResultsFields { score: s.last, target, target_label, medal_label, medal_text, emblem, medal }
        });
        self.results = fields;
        // [M :169 — runs outside the score_table != 0 arm.] The return value (just_completed) is
        // Menu_SetLevelBonus's own: level == 0x0700001b [M 0x1ffa10 tail].
        let medal = self.results.as_ref().map_or(5, |r| r.medal);
        self.just_completed = base_map == 0x0700_001b;
        progress.set_level_bonus(base_map, medal, 0);
        self.new_reward = progress.new_reward;
        // sp_level[i]+0x10 is the packed nightfire status's bits [M 0x202360's OR loop] — mirror
        // the award into the front end's wheel flags (the game reads the same bytes).
        let bits = progress.nightfire_status();
        for (i, u) in self.unlocked.iter_mut().enumerate() {
            *u = bits & (1 << i) != 0;
        }
        // The results flow's save [M the save prompt after the award — Menu_UpdateMessageBox
        // 0x1f7de8 case 1 writes the chain under the codename; the fail menu has no save arm].
        if self.active_profile.is_some() {
            self.save_profile();
        }
        self.results.is_some()
    }

    // ── fix-save-profile: the codename profiles (the port's memory card) ──────────────────────

    /// Point the front end at the profiles dir (the port's memory card) and enumerate the saved
    /// codenames — Menu_EnumSaves 0x1f7b78's psiStartSaveEnum/psiGetNextSave walk.
    pub fn set_profile_dir(&mut self, dir: std::path::PathBuf) {
        self.profile_dir = Some(dir);
        self.reload_profiles();
    }

    /// Re-enumerate the card [M 0x1f7b78's row order: the two defaults, then the card's names].
    pub fn reload_profiles(&mut self) {
        self.profiles = self
            .profile_dir
            .as_ref()
            .map(|d| nf_game::profile::Profile::list_dir(d))
            .unwrap_or_default();
    }

    /// The "New Codename" save [M the game's flow: the keyboard page validates through
    /// Menu_IsValidCodename 0x1f9930, Menu_MapDefaultCodename 0x1f7fa8 maps the default
    /// template (fresh unlocks via Menu_SetNightfireStatus 0x2023a0's 0x3, the option
    /// defaults, the cheats reset), then Menu_UpdateMessageBox 0x1f7de8 case 1 writes the
    /// chain under the codename]. `reserved` = the def_codename labels resolved through the
    /// language bank (the game strstr's the name against them [M 0x1f9930's loop]).
    pub fn create_codename(
        &mut self,
        name: &str,
        reserved: &[&str],
    ) -> Result<String, nf_game::profile::CodenameError> {
        let name = nf_game::profile::Profile::check_codename(name, reserved)?;
        let Some(dir) = self.profile_dir.clone() else {
            return Err(nf_game::profile::CodenameError::NoCard);
        };
        let path = nf_game::profile::Profile::path_in(&dir, &name);
        if path.exists() {
            // [M MemCard_ASK_OVERWRITE 0x10ee38 — the port refuses; the host decides]
            return Err(nf_game::profile::CodenameError::Exists);
        }
        let profile = nf_game::profile::Profile::default_codename(&name);
        profile
            .save_to_dir(&dir)
            .map_err(|_| nf_game::profile::CodenameError::Io)?;
        self.reload_profiles();
        self.active_profile = Some(name.clone());
        self.profile = Some(profile);
        Ok(name)
    }

    /// The codename page's Confirm on a saved row: load it [M Menu_UpdateMessageBox 0x1f7de8
    /// case 0 → LS_Load 0x1cdfc0 → LS_GetLoadStuff 0x1cd2b0 applies each found block] and move
    /// on to the difficulty page (the same continuation the "Play without saving" confirm
    /// uses [M C_SBNFCN 0x20b390:19-21]).
    pub fn load_codename(&mut self, row: usize) -> bool {
        let Some(name) = self.profiles.get(row.wrapping_sub(2)).cloned() else { return false };
        let Some(dir) = self.profile_dir.clone() else { return false };
        let path = nf_game::profile::Profile::path_in(&dir, &name);
        let Ok(profile) = nf_game::profile::Profile::load_file(&path) else { return false };
        // The MSSN nightfire status IS the unlock mask [M Menu_SetNightfireStatus 0x2023a0
        // unpacks it into sp_level[i]+0x10; the MSSN load maps 0 → 0x3].
        let status = profile.progress.nightfire_status();
        for (i, u) in self.unlocked.iter_mut().enumerate() {
            *u = status & (1 << i) != 0;
        }
        self.active_profile = Some(name);
        self.profile = Some(profile);
        self.page = Page::Difficulty;
        self.cursor = 1; // Agent [M the wheel starts centred on the middle row]
        true
    }

    /// The results save [M P_NFRESULTS 0x208e78 awards the level, then the save prompt's
    /// Menu_UpdateMessageBox 0x1f7de8 case 1 writes the chain under the codename]: mirror the
    /// front-end unlock bits into the profile's progress and write the file. The host syncs
    /// the tally/medals in with `sync_profile_progress` first. No-op without a card/profile.
    pub fn save_profile(&mut self) -> bool {
        let (Some(dir), Some(name)) = (self.profile_dir.clone(), self.active_profile.clone()) else {
            return false;
        };
        let Some(profile) = self.profile.as_mut() else { return false };
        let mut status = 0u32;
        for (i, &u) in self.unlocked.iter().enumerate() {
            if u {
                status |= 1 << i;
            }
        }
        profile.progress.set_nightfire_status(status);
        profile.name = name;
        profile.save_to_dir(&dir).is_ok()
    }

    /// The host merges the sim's mission tally (best scores, moments, medal bits) into the
    /// active profile before `save_profile` — the game writes its MSSN/BNUS blocks from the
    /// same live state the results page reads [M PlrStat_GetScore's DoneBetter commit +
    /// Menu_SetLevelBonus 0x1ffa10, then the save].
    pub fn sync_profile_progress(&mut self, p: &nf_game::progress::Progress) {
        if let Some(profile) = self.profile.as_mut() {
            profile.progress = p.clone();
        }
    }

    /// The Dossier Records' per-row values [M C_RBDSRECORDS 0x20bf00: the level's best score
    /// at ScoringTable+0x30, the moments value, the bonus count] for the records cursor —
    /// from the ACTIVE profile (zeros without one, the pre-task behaviour).
    pub fn record_row(&self, row: usize) -> (u32, u32, u32) {
        let Some(profile) = self.profile.as_ref() else { return (0, 0, 0) };
        let Some(entry) = profile.progress.table.get(row) else { return (0, 0, 0) };
        let hash = nf_game::progress::SP_LEVEL_HASHES.get(row).copied().unwrap_or(0);
        let bonuses = (1..=4).filter(|&k| profile.progress.has_medal(hash, k, 0)).count() as u32;
        (entry.best, entry.rows[0].value, bonuses)
    }

    /// The intro movie the P_INTRO run currently shows [M 0x209e88's movie_283 counter].
    pub fn intro_movie_code(&self) -> u32 { INTRO_MOVIES[self.intro_index.min(INTRO_MOVIES.len() - 1)] }

    /// One menu action. Returns true when the action changed the state.
    pub fn action(&mut self, a: Action) -> bool {
        self.no_input_frames = 0; // [M Menu_ResetNoInputCount on every handled input]
        // The intro movie is up: action 0x19 (X/Start — Enter here) skips it [M GameFlow_Main
        // 0x1c98c8 case 6 pops the movie state on `Input_Action(-1, 0x19)`]; other input does
        // nothing while the movie owns the screen (the intro page has no controls).
        if self.intro_playing && self.flow.get_state() == ST_MOVIE {
            if a == Action::Confirm {
                self.flow.pop_state();
                self.intro_playing = false;
                self.intro_index = 0;
                return true;
            }
            return false;
        }
        match self.page {
            Page::Start => {
                if a == Action::Confirm {
                    // → the intro movie, then the main menu [M P_INTRO 0x209e88 plays 0x07370048
                    // then 0x07380048 (non-looping) and only then opens the next page; the driver's
                    // observed order Title → movie → Main]. The page flips at once (menus_flow's
                    // contract); the overlay shows the movie fullscreen until the state pops.
                    self.flow.play_movie(INTRO_TICKS);
                    self.intro_playing = true;
                    self.intro_index = 0;
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
                    // Row 0 "New Codename" opens the game's name-entry page (not ported; reported).
                    // Row 1 "Play without saving" pops the confirm box [M C_SBNFCN 0x20b390:19-21 —
                    // selecting index 1 pops Menu_CreateOptionBox(0x10000bb, 9, 1, 1)], ✕ continues,
                    // △ cancels.
                    // fix-save-profile: rows 2+ are the card's codenames — Confirm loads the
                    // profile [M the select handler's LS_Load through Menu_UpdateMessageBox
                    // 0x1f7de8 case 0] and continues to the difficulty page.
                    if self.cursor >= 2 {
                        self.load_codename(self.cursor);
                        return true;
                    }
                    if self.cursor == 1 {
                        if !self.confirming_default_profile {
                            self.confirming_default_profile = true;
                        } else {
                            self.confirming_default_profile = false;
                            self.page = Page::Difficulty;
                            self.cursor = 1; // Agent [M the game's default difficulty row is the middle one — the wheel starts centred]
                        }
                        return true;
                    }
                    return false;
                }
                Action::Back => {
                    if self.confirming_default_profile { self.confirming_default_profile = false; return true; }
                    self.page = Page::Main; self.cursor = 0; return true;
                }
                _ => {}
            },
            Page::Difficulty => match a {
                // The difficulty wheel clamps too: the same Scroll_Update rule (0x2221b20:186-210,
                // wrap only under msg 0x69 [M 0x222810:136-139]) governs every wheel scroll, and the
                // front-end menu-def carries NO msg 0x69 on any control [L the tag-stream dump,
                // 891 messages, zero 0x69]; C_SBNFDFCTY 0x216ca8:25-28 sets this wheel's range
                // [0,2]. The port's old % wrap bounced Easy↔Little Leak on Up.
                Action::Up if self.cursor > 0 => { self.cursor -= 1; return true; }
                Action::Down if self.cursor + 1 < self.rows() => { self.cursor += 1; return true; }
                Action::Up | Action::Down => return false,
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
                    let level = CAMPAIGN[self.mission].3;   // +0x0c [M 0x20b8d8:21]
                    self.requested_level = Some(level);
                    self.flow.reset_map_level_to_load(level, false, false);
                    return true;
                }
                // The Dossier opens with the wheel at row 0 [M P_DOSSIER_Handler 0x216de0:9-16 —
                // the page msg 0x4c from 0x4000001c/36/38 sends msg 0x2e param 0 to the scroll
                // 0x1000010c, which rejects out-of-range indices and redraws (Scroll_SendMessage
                // 0x222810:98-106); without the reset the port entered with the missions cursor
                // and the wheel showed Weapons on screen — the playtest's "the port adds Weapons"].
                Action::Side => { self.page = Page::Dossier; self.cursor = 0; return true; }   // □ Dossier
                Action::Back => { self.page = Page::Difficulty; self.cursor = self.difficulty; return true; }
            },
            Page::Dossier => match a {
                // The wheel CLAMPS, it never wraps [M Scroll_Update 0x2221b20:186-194 (Down at the
                // max) / :202-210 (Up at the min) wrap only when the scroll's +0x144 flag is set,
                // and that flag is set only by msg 0x69 (Scroll_SendMessage 0x222810:136-139), which
                // nothing sends to 0x1000010c; msg 0x2e rejects out-of-range indices]. The port's
                // old % wrap teleported Records → Weapons on Up.
                Action::Up if self.cursor > 0 => { self.cursor -= 1; return true; }
                Action::Down if self.cursor + 1 < self.rows() => { self.cursor += 1; return true; }
                Action::Up | Action::Down => return false,
                Action::Confirm => {
                    // The wheel's row opens its page [M C_SBDOSSIER 0x216eb0:12-31 — index 0 →
                    // 0x4000003a Records, 1 → 0x4000003b Rewards, 2 → 0x4000003c Gadgets,
                    // 3 → 0x4000003d Weapons; the port draws Records, the rest are reported].
                    if self.cursor == 0 { self.page = Page::Records; self.cursor = self.mission.min(CAMPAIGN.len() - 1); return true; }
                    return false;
                }
                Action::Back | Action::Side => { self.page = Page::MissionSelect; return true; }
                _ => {}
            },
            Page::Records => match a {
                // The mission list walks the campaign [M the list control 0x10000174 is filled from
                // sp_level by Menu_AddItemsToControl (P_DSRECORDS 0x217248:11-13); a row's selection
                // drives C_RBDSRECORDS 0x20bf00's binds].
                Action::Up => { self.cursor = (self.cursor + CAMPAIGN.len() - 1) % CAMPAIGN.len(); return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % CAMPAIGN.len(); return true; }
                Action::Back | Action::Confirm | Action::Side => { self.page = Page::Dossier; self.cursor = 0; return true; }
            },
            Page::Options => match a {
                Action::Up => { self.cursor = (self.cursor + 2) % 3; return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % 3; return true; }
                Action::Back | Action::Confirm if self.cursor == 2 => { self.page = Page::Main; self.cursor = 2; return true; }
                _ => {}
            },
            Page::Pause => {
                // The YES/NO confirm list runs while either confirm is armed [M 0x204408's msg-0x4b
                // d1=0xd case reads the list's row (msg 0x34) and 0 = YES executes].
                if self.confirming_restart || self.confirming_quit {
                    match a {
                        Action::Up | Action::Down => { self.confirm_sel = 1 - self.confirm_sel; return true; }
                        Action::Confirm => {
                            if self.confirm_sel == 1 {
                                // NO: back to the MISSION rows [M the case's cVar4==1 arm re-binds the
                                // 0x201 hint, re-enables d1=6 and sets state_ignore_167].
                                self.confirming_restart = false;
                                self.confirming_quit = false;
                                self.confirm_sel = 1;
                                return true;
                            }
                            let restart = self.confirming_restart;
                            self.confirming_restart = false;
                            self.confirming_quit = false;
                            self.confirm_sel = 1;
                            if restart {
                                // YES on RESTART [M 0x204408:138-141 — state_quit_166 == 0:
                                // ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)].
                                let level = CAMPAIGN[self.mission].3;
                                self.requested_level = Some(level);
                                self.flow.reset_map_level_to_load(level, false, true);
                                self.page = Page::Main;
                                return true;
                            }
                            // YES on QUIT [M 0x204408:142-152 — +0x28 = 0x4000001c (the SP quit page),
                            // ResetMap_LevelToLoad(0x7000048, 0, 0), the fade push and the delete].
                            self.flow.page = PAGE_RESULTS_QUIT_TARGET;
                            self.requested_level = Some(FRONTEND_LEVEL);
                            self.flow.reset_map_level_to_load(FRONTEND_LEVEL, false, true);
                            self.page = Page::Main;
                            self.cursor = 0;
                            return true;
                        }
                        Action::Back => {
                            self.confirming_restart = false;
                            self.confirming_quit = false;
                            self.confirm_sel = 1;
                            return true;
                        }
                        Action::Side => return false,
                    }
                }
                match a {
                    // ~H Next: the tab cycle [L the page hint 0x201 "~A Select ~V Scroll ~H Next";
                    // the four tabs are the page's type-1 controls].
                    Action::Side => { self.pause_tab = (self.pause_tab + 1) % 4; return true; }
                    Action::Up => { self.cursor = (self.cursor + 2) % 3; return true; }
                    Action::Down => { self.cursor = (self.cursor + 1) % 3; return true; }
                    Action::Back => { self.closed = true; return true; }   // resume [M the d1=6 case-0 arm → MenuManager_Delete]
                    Action::Confirm => {
                        match self.cursor {
                            0 => { self.closed = true; return true; }   // continue/resume — the host closes the menu
                            1 => {                                       // restart → its confirm [M 0x204408 case 1 binds 0x1ff]
                                self.confirming_restart = true;
                                self.confirm_sel = 1;
                                return true;
                            }
                            2 => {                                       // quit → its confirm [M 0x204408 case 2 binds 0x200]
                                self.confirming_quit = true;
                                self.confirm_sel = 1;
                                return true;
                            }
                            _ => {}
                        }
                    }
                }
            },
            Page::Results if self.outcome == Outcome::Complete => match a {
                Action::Side => {}
                // P_NFRESULTS has no row list [L page 0x40000036's controls] — the wheel keys idle.
                Action::Up | Action::Down | Action::Back => return false,
                Action::Confirm => {
                    // Case 0x4b [M 0x208e78:28-52]: a NEW reward routes to the bonus page
                    // 0x40000038, whose own Confirm lands on the mission list [M 0x209670:50-53];
                    // just_completed routes to the win page 0x40000053 — not ported (reported).
                    // The port elides the bonus page's stop and takes the same landing.
                    if self.just_completed { return false; }
                    // P_NFMAP 0x20b608 case 0x4c's cursor rule: scan sp_level+0x10 from row 11 down
                    // — the list opens on the HIGHEST unlocked row [M :57-64].
                    self.cursor = (0..CAMPAIGN.len()).rev().find(|&i| self.unlocked_row(i)).unwrap_or(0);
                    self.page = Page::MissionSelect;
                    self.flow.page = PAGE_MISSION_SELECT;
                    return true;
                }
            },
            Page::Results => match a {
                Action::Side => {}
                Action::Up => { self.cursor = (self.cursor + 2) % 3; return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % 3; return true; }
                Action::Confirm => {
                    match self.cursor {
                        // The arms in row order [M P_ENDMISSION_Handler 0x216618:19-27]:
                        // row 0 → `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` + `Player_RamLoad(Bond, 1)`
                        // — the CURRENT level, WARM, on the entry checkpoint (fix-restart-target; the old
                        // row 0 sent arm 1). Row 1 → `ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)`
                        // — BaseMap, COLD. Row 2 → `GameState+0x28 = 0x4000001c` + 0x7000048.
                        0 => {
                            let level = self.flow.current_level;
                            self.requested_level = Some(level);
                            self.flow.reset_map_level_to_load(level, true, true);
                            self.page = Page::Main;
                            return true;
                        }
                        1 => {
                            // The mission's base map: the campaign row's level code (fix-menus2's `sp_level`
                            // table; `Mission_BaseMapHCode`).
                            let level = CAMPAIGN[self.mission].3;
                            self.requested_level = Some(level);
                            self.flow.reset_map_level_to_load(level, false, true);
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
pub const MENU_TEXTURES: [u32; 32] = [0x0300_0042, 0x0300_0074, 0x0300_00b3, 0x0300_00bf, 0x0300_0084, 0x0300_00b6, 0x0300_00c4, 0x0300_00a5, 0x0300_016d, 0x0300_014c,
    // The hint-bar icon font `specialchar` [M 0x2dd8d8: every entry's texture is 0x03000075].
    0x0300_0075,
    // The 12 per-mission ring previews = sp_level[i]+0x00 [L ee_snapshot 0x2df2e0; all 12 live in the
    // 07000048.bin menu archive's texture entry 5 [L nf-extract tex-list]].
    0x0300_0085, 0x0300_00b0, 0x0300_00b2, 0x0300_008c, 0x0300_008e, 0x0300_008d, 0x0300_008f, 0x0300_00ae, 0x0300_00af, 0x0300_0151, 0x0300_0142,
    // fix-pause-menu: the in-game pages' art [L the mission menu-def's msg-0x24 controls; sizes from
    // the archives' tex lists — 0x03000070 512×256 backdrop plate and 0x030000db 128×128 Bond image
    // live in every mission archive (07000029/0700004b/07000043/…), 0x03000140 256×128 results photo
    // in the front-end 07000048.bin].
    0x0300_0070, 0x0300_00db, 0x0300_0140,
    // fix-menus-real: the codename wheel's per-cursor previews (Menu_UpdateCodenameWheel 0x1f89f0:80-99
    // sends 0x300013b for the create row, 0x3000135 for the default/profile rows) and the dossier
    // wheel's per-row previews + Records' medal (table 0x2dfcd0; P_DSRECORDS 0x217248's 0x0300013d).
    0x0300_013b, 0x0300_0135, 0x0300_019b, 0x0300_019c, 0x0300_00ee, 0x0300_013d,
    // fix-menus-real2: the page headers' bold-italic face — FontTable[0] = FONTNFont2, texture
    // 0x03000005 [L ee_snapshot 0x2dd8b0; the page dump's header NAME strings \xff\x01]. 512×128
    // in the front-end archive [L nf-extract tex-list 07000048.bin].
    0x0300_0005];

/// The menu text renderer's special-character table `specialchar` (0x2dd8d8, 17 × 0x18 bytes: the token
/// char, the texture hash 0x03000075, u0, v0, w, h, advance, height) [M ACTION.ELF symtab + the table's
/// bytes; the hint strings' ~A/~B/... tokens draw through it — the game's own button glyphs, NOT the
/// 64×64 atlas 0x03000042 the port guessed before]. (token, u, v, w, h, advance)
pub const SPECIALCHAR: [(char, u32, f32, f32, f32, f32, f32); 17] = [
    ('A', 0x0300_0075, 235.0, 1.0, 19.0, 19.0, 19.0),   // the blue ✕ cross [D atlas (235,1,19×19)]
    ('B', 0x0300_0075, 235.0, 21.0, 19.0, 19.0, 19.0),  // the green △ triangle [D atlas (235,21)]
    ('C', 0x0300_0075, 213.0, 111.0, 22.0, 17.0, 20.0),
    ('D', 0x0300_0075, 190.0, 111.0, 22.0, 17.0, 20.0),
    ('E', 0x0300_0075, 192.0, 72.0, 18.0, 18.0, 20.0),
    ('F', 0x0300_0075, 215.0, 72.0, 18.0, 18.0, 20.0),
    ('H', 0x0300_0075, 236.0, 80.0, 18.0, 19.0, 20.0),
    ('I', 0x0300_0075, 198.0, 59.0, 7.0, 7.0, 7.0),
    ('J', 0x0300_0075, 176.0, 57.0, 16.0, 12.0, 16.0),
    ('L', 0x0300_0075, 213.0, 93.0, 22.0, 13.0, 20.0),
    ('R', 0x0300_0075, 190.0, 93.0, 22.0, 13.0, 20.0),
    ('S', 0x0300_0075, 162.0, 113.0, 26.0, 13.0, 20.0),
    ('T', 0x0300_0075, 150.0, 109.0, 9.0, 7.0, 16.0),
    ('V', 0x0300_0075, 236.0, 100.0, 18.0, 19.0, 20.0), // the gold D-pad [D atlas (236,100)]
    ('W', 0x0300_0075, 167.0, 90.0, 20.0, 20.0, 20.0),
    ('X', 0x0300_0075, 235.0, 41.0, 19.0, 19.0, 19.0),  // the orange ○ circle [D atlas (235,41)]
    ('Y', 0x0300_0075, 235.0, 61.0, 19.0, 19.0, 19.0),  // the pink □ square [D atlas (235,61)]
];

/// The menu text colours [M the pages' msg-0x1b/0x1c colour pairs; 0x7d6d59ff is rewritten to
/// 0x645a49ff by `MenuManager_Create` [M 0x21b908:238-243] — cream text on the warm 007 red].
const CREAM: u32 = 0x645a49;
const BRIGHT: u32 = 0xfff8ec;
const ORANGE: u32 = 0xe6661e;
/// The title's yellow prompt [M page 0x40000009 control 0x100000fa's msg-0x1b/0x1c payload
/// 0x7f621fff (parsed 2026-10-14) — the GS-scale gold the prompt flashes in].
const YELLOW: u32 = 0x7f621f;

/// One menu frame's draw data. `menu_overlay` fills a `HudFrame` the host submits like the HUD's.
pub fn menu_overlay(fe: &FrontEnd, font: &nf_formats::hud::Font, hfont: &nf_formats::hud::Font, lang: Option<&Lang>, textures: &[(u32, crate::scene::TextureImage)], aspect: f32) -> HudFrame {
    let mut h = Hud { quads: Vec::new(), aspect, textures };
    let bind = |id: u32| -> String { lang.and_then(|l| l.bind(id)).unwrap_or("").to_string() };
    // Without the language file the hint bars keep their button layout (the ~N tokens) so the pages
    // still render [S].
    let hint = |id: u32, fallback: &str| -> String { let s = bind(id); if s.is_empty() { fallback.into() } else { s } };
    let cream = gs(CREAM, 0xff);
    let bright = gs(BRIGHT, 0xff);
    let dim = gs(CREAM, 0x80);
    let dimmest = gs(CREAM, 0x40);
    // The intro movie owns the screen: P_INTRO's page has no controls, the two FMVs are the whole
    // picture (the fog pass is suppressed while a movie plays [M psiDrawFogPass 0x0010b960:
    // BGFMVPlaying == 0]), and the frame fills the 512×448 screen [M DecodeBGFMVFrame 0x00108550:
    // every movie is exactly 512×448, centred]. fix-menu-movies2: the frame draws on the ONE movie
    // plane (gpu.rs), so the host suppresses this overlay entirely while a real frame is up; what
    // is drawn here is only the plate stand-in for the headless countdown (no `movies` feature, or
    // the stubbed slim ISO — the pre-movies look).
    if fe.intro_playing {
        plate(&mut h, 0.0, 0.0, MW, MH, gs(0x180c08, 0xff));
        return h.finish();
    }
    // The backdrop: the pages' aux FMVs are the real background — parsed from the menu-def's PAGE
    // records (aux_movie_for: the main menu's 0x07330048, the boot/list/results pages' 0x07350048;
    // `Manager_SendMessage` 0x21c858:338 plays the page's aux code as a LOOPING background movie
    // under the page controls on every page change). fix-menu-movies2: the decoded frame draws on
    // the ONE movie plane (gpu.rs), BEFORE this overlay's quads — the game's backdrop is the page's
    // own background texture, not an overlay [M movies.md §Presentation; PS2UploadBGMImage 0x108740].
    // When the plane has no live frame (no `movies` feature, or the slim ISO's zeroed PSS files [L])
    // the port keeps the dark plate the pages fade from [M the pages' 0x1b/0x1c fade colours].
    let aux = aux_movie_for(fe.page.id());
    // fix-pause-menu: the IN-GAME pages (pause 0x4000004b, fail menu 0x40000042) draw OVER THE PAUSED
    // FRAME — the game dims nothing full-screen; their own backdrop plate (45,107,550×266, TEX 0x03000070
    // [L the pages' 0x10000001 control]) carries the menu.
    if !(aux != 0 && fe.backdrop_live) && !matches!(fe.page, Page::Pause | Page::Results) {
        plate(&mut h, 0.0, 0.0, MW, MH, gs(0x180c08, 0xff));
    }
    match fe.page {
        Page::Start => {
            // The title [M menu-def page 0x40000009: the art control 0x10000196 (0,0,640,480) TEX
            // 0x03000074; the prompt control 0x100000fa (131,309,379×16) LBL 0x1c9, colours 0x7f621fff;
            // the legal plate 0x10000197 (50,332,540×108). The handler binds 0x10001eb to the prompt
            // [M 0x20a408] and 0x375 to the plate, flashing on the 240-frame cycle.]
            quad640(&mut h, 0x0300_0074, 0.0, 0.0, MW, MH, [0.0, 0.0, 512.0, 512.0], gs(0x7f7f7f, 0xff));
            if fe.press_start_on {
                centred(&mut h, font, -2, &bind(0x100_01eb), mx(320.5), my(317.0), gs(YELLOW, 0xff), true, 1.0);
            }
            let legal = bind(0x375);
            if !legal.is_empty() {
                // The legal text wraps inside its plate (50,332,540×108) and each line is centred in it
                // [L the main agent's PCSX2 captures 2026-10-13; the port's raw 92-column run went off
                // the right edge]. Wrap to the plate's pixel width, centre at the plate's centre x=320.
                for (i, line) in wrap_px(&legal, font, 500.0).iter().take(6).enumerate() {
                    centred(&mut h, font, -3, line, mx(320.0), my(334.0 + i as f32 * 18.0), gs(CREAM, 0xc0), false, 1.0);
                }
            }
        }
        Page::Main => {
            // The main menu [M menu-def page 0x40000002 (parsed 2026-10-14): rows 0x10000002/3/4
            // (167,262/294/326,306×28) LBL 0x28/0x149/0x14b, colours msg-0x1b 0x7d6d59ff (→0x645a49ff
            // — the SELECTED row's bright cream) / msg-0x1c 0x73330f80 (the dark orange the others
            // keep); the description plate 0x10000225 (103,357,434×42) LBL runtime-filled (Txt
            // 0x10000000/1/2), colour cream; the hint bar 0x10000001 (50,419,540×21) LBL 0x54e.
            // The page has NO static art control: the backdrop is its aux FMV 0x07330048 [L PAGE
            // record] — see the plate comment above.]
            for (i, &lbl) in MAIN_ITEMS.iter().enumerate() {
                let y = 262.0 + i as f32 * 32.0;
                let (c, scale) = if i == fe.cursor { (cream, 1.35) } else { (gs(0x73330f, 0x80), 1.2) };
                centred(&mut h, font, -2, &bind(lbl), mx(167.0 + 153.0), my(y + 6.0), c, true, scale);
            }
            centred(&mut h, font, -2, &bind(MAIN_DESCS[fe.cursor.min(2)]), mx(103.0 + 217.0), my(357.0 + 14.0), cream, true, 1.0);
            hint_bar(&mut h, font, &hint(0x54e, "~V Scroll  ~A Select"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::Codename => {
            // Select Codename [M menu-def page 0x40000025 (parsed 2026-10-14): header 0x10000001
            // (50,40,376×17) LBL 0x1f2; the panel 0x10000001 (50,53,590×295) TEX 0x030000b3; the
            // emblem 0x10000103 (ring TEX 0x030000bf) + 0x100001cc (preview, static TEX 0x030000b6)
            // (114,118,150×150); the five wheel rows 0x100000e4; the confirm plate 0x100001f2
            // (50,301,540×113); the hint bar (50,419) LBL 0x216. Runtime: Menu_UpdateCodenameWheel
            // 0x1f89f0:80-99 swaps the preview to 0x300013b (the create row) / 0x3000135 (the
            // default row) and binds the row's description 0x1000004 (create) / 0x1000003 (default)
            // [M].]
            page_header(&mut h, hfont, &bind(0x1f2));
            panel(&mut h);
            let preview = if fe.cursor == 0 { 0x0300_013b } else { 0x0300_0135 };
            emblem(&mut h, preview, 0x0300_00bf);
            wheel(&mut h, fe, font, lang, |fe, row| fe.row_label(lang, row), if fe.confirming_default_profile { dimmest } else { cream });
            // The row's description under the wheel [M 0x1f89f0's desc binds].
            let desc = if fe.cursor == 0 { bind(0x100_0004) } else { bind(0x100_0003) };
            for (i, line) in wrap_px(&desc, font, 500.0).iter().take(4).enumerate() {
                centred(&mut h, font, -2, line, mx(320.0), my(301.0 + 28.0 + i as f32 * 18.0), bright, true, 1.0);
            }
            // The confirm box: the game pops it for "Play without saving" [M C_SBNFCN 0x20b390:19-21
            // Menu_CreateOptionBox(0x10000bb, 9, 1, 1); the plate 0x100001f2].
            if fe.confirming_default_profile {
                confirm_box(&mut h, font, &bind(0x100_00bb), bright);
            }
            hint_bar(&mut h, font, &hint(0x216, "~A Select  ~Y Create  ~V Scroll  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::Difficulty => {
            // Select Difficulty [M menu-def page 0x40000023: header LBL 0x1f8 (50,40); panel TEX
            // 0x030000b3 (50,53,590×295); the emblems 0x100000be/0x10000104 (114,118,150×150) TEX
            // 0x030000a5/0x030000bf; the five wheel rows 0x100000e6 with the selected row's full
            // alpha and the neighbours' 0x80/0x40 [M the rows' msg-0x1b colours]; the description
            // plate 0x100001ee (50,301,540×113); the hint bar LBL 0x218.]
            page_header(&mut h, hfont, &bind(0x1f8));
            panel(&mut h);
            emblem(&mut h, 0x0300_00a5, 0x0300_00bf);
            wheel(&mut h, fe, font, lang, |_fe, row| bind(DIFFICULTIES[row.min(2)]), cream);
            // The description wraps inside its plate (50,301,540×113) [L the main agent's captures:
            // the port's single line ran off the right edge].
            for (i, line) in wrap_px(&bind(DIFFICULTY_DESCS[fe.cursor.min(2)]), font, 500.0).iter().take(4).enumerate() {
                centred(&mut h, font, -2, line, mx(320.0), my(301.0 + 28.0 + i as f32 * 18.0), bright, true, 1.0);
            }
            hint_bar(&mut h, font, &hint(0x218, "~A Select  ~V Scroll  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::MissionSelect => {
            // Single Player [M menu-def page 0x4000001c: header 0x10000029 (50,40) LBL 0x14c; the
            // panel TEX 0x030000b3; the emblem 0x10000020 (114,118,150×150) TEX = the ring-masked
            // preview UNDER 0x10000102 TEX 0x030000bf (the ring, f1 0x1c < the preview's 0x1d — the
            // ring draws on top); the five wheel rows 0x100000e3; the description plate 0x1000001f
            // (50,301,540×113); the hint bar LBL 0x218 (□ Dossier, Txt 0x208). The preview texture is
            // PER ROW: sp_level[cursor]+0x00 [M Menu_UpdateWheel 0x1f8388's msg 0x24 to 0x10000020].]
            page_header(&mut h, hfont, &bind(0x14c));
            panel(&mut h);
            let row = fe.cursor.min(CAMPAIGN.len() - 1);
            let (c, ring_c) = if fe.unlocked_row(row) { (gs(0x7f7f7f, 0xff), gs(0x7f7f7f, 0xff)) } else { (gs(0x7f7f7f, 0x60), gs(0x7f7f7f, 0x60)) }; // [M 0x1f8388: locked rows' 0x60606060 colour]
            emblem_col(&mut h, CAMPAIGN[row].0, 0x0300_00bf, c, ring_c);
            wheel_dim(&mut h, fe, font, lang, |_fe, row| bind(CAMPAIGN[row].1), |fe, row| !fe.unlocked_row(row), cream);
            // The description: +0x08 for an unlocked row, +0x14 for a locked one [M 0x1f8388], wrapped
            // inside the plate.
            let desc = if fe.unlocked_row(row) { bind(CAMPAIGN[row].2) } else { bind(LOCKED_DESC) };
            for (i, line) in wrap_px(&desc, font, 500.0).iter().take(4).enumerate() {
                centred(&mut h, font, -2, line, mx(320.0), my(301.0 + 28.0 + i as f32 * 18.0), bright, true, 1.0);
            }
            hint_bar(&mut h, font, &hint(0x208, "~A Select  ~Y Dossier  ~V Scroll  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::Dossier => {
            // The Dossier [M menu-def page 0x4000002b (parsed 2026-10-14): header LBL 0x27f (50,40);
            // hint LBL 0x218; panel TEX 0x030000b3; the emblem 0x1000010a (preview TEX 0x030000c4)
            // UNDER 0x1000010b (ring TEX 0x030000bf); the five wheel rows 0x1000010d. The wheel's
            // rows + previews + descriptions come from the game's table 0x2dfcd0 [L ACTION.ELF .data
            // dump: {0x300019b, 0x280, 0x100000a}, {0x300019c, 0x281, 0x100000b}, {0x30000c4, 0x282,
            // 0x100000c}, {0x30000ee, 0x19c, 0x100000d}], walked by C_SBDOSSIER 0x216eb0:48-49 and
            // drawn by Menu_UpdateWheel 0x1f8388 (the per-row preview msg-0x24 + desc bind).]
            page_header(&mut h, hfont, &bind(0x27f));
            panel(&mut h);
            const DOSSIER_ROWS: [(u32, u32, u32); 4] = [
                (0x0300_019b, 0x280, 0x100_000a), (0x0300_019c, 0x281, 0x100_000b),
                (0x0300_00c4, 0x282, 0x100_000c), (0x0300_00ee, 0x19c, 0x100_000d)];
            let row = fe.cursor.min(3);
            emblem(&mut h, DOSSIER_ROWS[row].0, 0x0300_00bf);
            wheel(&mut h, fe, font, lang, |_fe, r| bind(DOSSIER_ROWS[r.min(3)].1), cream);
            for (i, line) in wrap_px(&bind(DOSSIER_ROWS[row].2), font, 500.0).iter().take(4).enumerate() {
                centred(&mut h, font, -2, line, mx(320.0), my(301.0 + 28.0 + i as f32 * 18.0), bright, true, 1.0);
            }
            hint_bar(&mut h, font, &hint(0x218, "~A Select  ~V Scroll  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::Records => {
            // The Dossier's Records table [M menu-def PAGE 0x4000003a (parsed 2026-10-14): header
            // LBL 0x280 (50,40,376×17); hint LBL 0x49e (50,419,540×21); the mission list 0x10000174
            // (132,124,376×16) filled from sp_level by Menu_AddItemsToControl [M P_DSRECORDS
            // 0x217248:11-13]; the score rows: LBL 0x302 (83,169,248×25) + value 0x10000175
            // (375,169,185×16), LBL 0x165 (83,202,352×25) + value 0x10000177 (375,202,185×16),
            // LBL 0x305 (83,235,352×25) + value 0x10000178 (375,235,185×16); the medal image
            // 0x10000176 (435,282,128×64) TEX 0x0300013d + the medal label 0x100001f8 LBL 0x307
            // (76,306,240×16); the logo 0x0300016d. Per-row values are C_RBDSRECORDS 0x20bf00:79-92's
            // binds (the level's own high score / best / upgrades) — headless has no save data, so
            // the values draw as 0.]
            page_header(&mut h, hfont, &bind(0x280));
            panel(&mut h);
            let m = fe.cursor.min(CAMPAIGN.len() - 1);
            // The selected mission's name in the list control [M P_DSRECORDS 0x217248:11-13 fills
            // 0x10000174 from sp_level — the name labels are CAMPAIGN's +0x04].]
            centred(&mut h, font, -2, &bind(CAMPAIGN[m].1), mx(132.0 + 188.0), my(124.0 + 8.0), bright, true, 1.0);
            const RECORD_ROWS: [(u32, f32, f32, f32); 3] = [(0x302, 83.0, 169.0, 248.0), (0x165, 83.0, 202.0, 352.0), (0x305, 83.0, 235.0, 352.0)];
            for (lbl, x, y, w) in RECORD_ROWS {
                centred(&mut h, font, -2, &bind(lbl), mx(x + w / 2.0), my(y + 8.0), cream, true, 1.0);
            }
            // The value controls (375,169/202/235,185×16) [L parsed; Label_Update centres them].
            // fix-save-profile: the values come from the ACTIVE profile's progress [M
            // C_RBDSRECORDS 0x20bf00:26-28 sprintf's the level's best (ScoringTable+0x30), the
            // moments value and the bonus count]; without one they stay 0 (headless).
            let (best, moments, bonuses) = fe.record_row(m);
            for (y, v) in [(169.0f32, best), (202.0, moments), (235.0, bonuses)] {
                centred(&mut h, font, -2, &v.to_string(), mx(375.0 + 92.5), my(y + 8.0), cream, true, 1.0);
            }
            quad640(&mut h, 0x0300_013d, 435.0, 282.0, 128.0, 64.0, [0.0, 0.0, 255.0, 127.0], gs(0x7f7f7f, 0xff));
            centred(&mut h, font, -2, &bind(0x307), mx(76.0 + 120.0), my(306.0 + 8.0), cream, true, 1.0);
            hint_bar(&mut h, font, &hint(0x49e, "~V Change  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::Options => { page_rows(&mut h, fe, font, "OPTIONS", dim, bright); }
        Page::Pause => pause_page(&mut h, fe, font, lang, textures, cream, bright, dim),
        Page::Results => results_page(&mut h, fe, font, lang, textures, cream, bright, dim),
    }
    h.finish()
}

/// A textured 640-space quad; when the texture isn't loaded draws the dim plate the pages fade from
/// instead. The in-game pages' art (0x03000070/0x030000db) lives in the mission archives, so a
/// front-end `--page pause` shot (the front-end archive loaded) falls back to the plate.
fn tex_quad_or_plate(h: &mut Hud, textures: &[(u32, crate::scene::TextureImage)], tex: u32, x: f32, y: f32, w: f32, hh: f32, uv: [f32; 4], c: [f32; 4]) {
    if textures.iter().any(|(hash, _)| *hash == tex) {
        quad640(h, tex, x, y, w, hh, uv, c);
    } else {
        plate(h, x, y, w, hh, gs(0x1e1210, 0xd0));
    }
}

/// fix-pause-menu2: the SCORE tab's per-level TARGET column [L ACTION.ELF static data — the
/// `PTR_HT_Level_Driving_Paris_ST` pointer table 0x2bc658, 12 levels × 15 dwords, level row i's
/// pointer → 9 × 0x14-byte slots whose +0x00 the pause handler reads as the targets [M
/// PlrStats_SetupScoreTable 0x1990e0 + C_GCPAUSE_Handler 0x204408:464-498]]. Extracted from the
/// retail disc 2026-10-14 and cross-checked: campaign row 1 (07000005 The Exchange) reads
/// [2, 30, 3, 3, 30, 100, 100, 540, 7] = Bond Moves 2, Dispatched 30, Subdued 3, Surrendered 3,
/// —, Accuracy 100, Health 100, Time 09:00 (540 s), 007 Bonus 7 — the PCSX2 capture's numbers.
/// Row order = `CAMPAIGN` (sp_level) order [M Menu_GetLevelIndex 0x2023e8 walks sp_level[i]+0xc].
/// Slot layout: 0 Bond Moves (0x165), 1 Dispatched (0x15b), 2 Subdued (0x15c), 3 Surrendered
/// (0x16e), 4 unused by the pause display, 5 Accuracy (0x15e), 6 Health (0x15f; the game prints
/// the literal 100), 7 Time in seconds (0x160), 8 007 Bonus (0x161).
pub const SCORE_TARGETS: [[u32; 9]; 12] = [
    [7, 1, 0, 0, 0, 100, 60, 240, 4],   // 09000001 Paris Prelude
    [2, 30, 3, 3, 30, 100, 100, 540, 7], // 07000005 The Exchange
    [5, 1, 0, 0, 0, 100, 92, 250, 3],   // 09000005 Alpine Escape
    [3, 1, 0, 0, 0, 100, 90, 240, 5],   // 09000006 Enemies Vanquished
    [2, 45, 3, 3, 38, 100, 100, 480, 7], // 07000001 Double Cross
    [2, 8, 0, 0, 10, 100, 100, 270, 7], // 07000009 Night Shift
    [2, 45, 0, 0, 0, 100, 100, 1200, 7], // 0700000c Chain Reaction
    [2, 65, 0, 0, 0, 100, 100, 720, 7], // 07000011 Phoenix Fire
    [3, 1, 0, 0, 0, 100, 90, 420, 6],   // 09000002 Deep Descent
    [7, 1, 0, 0, 0, 100, 75, 609, 4],   // 09000003 Island Infiltration
    [1, 65, 0, 0, 68, 100, 100, 900, 7], // 07000014 Countdown
    [0, 15, 0, 0, 0, 100, 100, 255, 7], // 0700001b Equinox
];

/// The campaign row a level code plays [M Menu_GetLevelIndex 0x2023e8 — sp_level[i]+0x0c].
pub fn score_row_for_level(level: u32) -> Option<usize> {
    CAMPAIGN.iter().position(|c| c.3 == level)
}

/// The in-game pause page — menu-def page 0x4000004b, the mission archives' kind-8 entry 08000001
/// [L parsed 2026-10-14; MenuManager_Create(0x80000002, 0x4000004b) on Start [M Player_Update
/// 0x193600:73], handler C_GCPAUSE_Handler 0x204408]. Drawn over the paused frame.
fn pause_page(h: &mut Hud, fe: &FrontEnd, font: &nf_formats::hud::Font, lang: Option<&Lang>, textures: &[(u32, crate::scene::TextureImage)], cream: [f32; 4], bright: [f32; 4], dim: [f32; 4]) {
    let bind = |id: u32| -> String { lang.and_then(|l| l.bind(id)).unwrap_or("").to_string() };
    let lbl = |id: u32, fb: &str| -> String { let s = bind(id); if s.is_empty() { fb.into() } else { s } };
    // The backdrop plate (45,107,550×266) TEX 0x03000070 [L the page's 0x10000001 control].
    tex_quad_or_plate(h, textures, 0x0300_0070, 45.0, 107.0, 550.0, 266.0, [0.0, 0.0, 512.0, 256.0], gs(0x7f7f7f, 0xff));
    // THE FIGURE: control 0x10000184 draws texture 0x030000db, whose disc texels are only a placeholder (palette
    // blobs and a basketball player) — the game overwrites them every other paused frame with the burning woman
    // (`psiDecompressWoman` 0x1202a8, crate::woman: the archive's "woman" mask filled from the fire texture 0x030000da,
    // through 0x030000da's CLUT). The loader seeds the HUD texture with the first frame and the frame loop advances
    // it while the page shows. The control carries msg 0x6c = 0x100 [M P_PAUSE_Handler 0x203c28] and psiDrawSprites
    // draws flags 0x100 with ALPHA_1 = 0x48 = (Cs−0)·As + Cd [M 0x120e68:56-68]: additive, the black around her adds
    // nothing. PCSX2 (07000005 fail menu, 2026-10-15): the fiery woman with a pistol on the panel's right.
    {
        h.quad(Anchor::Centre, 0, 0x0300_00db, 1, mx(385.0), my(121.0), mx(202.0), my(218.0), [0.0, 0.0, 128.0, 128.0], (false, false), gs(0x808080, 0xff));
    }
    // The four tabs [L the page's four type-1 controls 0x10000028 at (84/199/339/465,120,107/132/118/90×16),
    // LBL 0x167 MISSION / 0x176 OBJECTIVES / 0x168 CONTROLS / 0x169 SCORE; base colour 0x7d6d59ff →
    // 0x645a49ff [M 0x21b908:238-243], the selected tab's 0x1c colour 0x201707ff].
    let tabs = [(0x167u32, 84.0, 107.0, "MISSION"), (0x176, 199.0, 132.0, "OBJECTIVES"), (0x168, 339.0, 118.0, "CONTROLS"), (0x169, 465.0, 90.0, "SCORE")];
    let tab = fe.pause_tab.min(3);
    for (i, &(id, x, w, fb)) in tabs.iter().enumerate() {
        let c = if i == tab { gs(0x201707, 0xff) } else { cream };
        h.text(Anchor::Centre, font, -2, &lbl(id, fb), mx(x + w / 2.0), my(120.0 + 8.0), false, c, false);
    }
    match tab {
        // fix-pause-menu2: the CONTROLS tab. The d1=0xa list (94,185,465×153) is the controller-
        // style page: the style name row ("Classic Bond", the wheel's up/down marker) then the
        // per-style action rows as "<glyph> <label>" pairs [M Menu_DisplayControllerStyleList
        // 0x1f44d8 case 0; the glyph strings are ACTION.ELF .data 0x30d1c8-0x30d228, the label ids
        // 0x8f/0x8c, 0x8b/0x8d, 0x84/0x89, 0x85/0x8a, 0x88/0x90, 0x87/0x86 bind from USATxt.dat;
        // row geometry from the d1=0xa control's 0x28 message 0x34 = 52-px rows [L]]. The style
        // name is the port's only style (the others' labels are the game's, unported) [S].
        2 => {
            // The Y-Axis toggle line (400,160) LBL 0x492 + the value 0x3bd "Normal" / 0x206
            // "Inverted" [L the page's d1=0x10/0xe controls; the toggle is the game's msg-0x5e arm
            // flipping PlayerSetting [M 0x204408 case 0x5e]; the port's setting is not-inverted →
            // the game's boot value 0 shows 0x3bd "Normal" [M 0x204408:512-517]].
            h.text(Anchor::Centre, font, -2, &lbl(0x492, "Y-Axis"), mx(400.0 + 30.0), my(160.0 + 8.0), false, cream, false);
            h.text(Anchor::Centre, font, -2, &lbl(0x3bd, "Normal"), mx(463.0 + 46.0), my(160.0 + 8.0), false, bright, false);
            // The style name + its up/down marker, then the two glyph columns.
            h.text(Anchor::Centre, font, -2, &lbl(0x2f5, "Classic Bond"), mx(94.0 + 232.0), my(192.0), false, bright, false);
            h.text(Anchor::Centre, font, -2, "▲▼", mx(94.0 + 232.0), my(205.0), false, gs(YELLOW, 0xff), false);
            let rows: [(&str, u32, &str, u32); 6] = [
                ("~A", 0x8f, "~B", 0x8c), ("~Y", 0x8b, "~X", 0x8d), ("~C", 0x84, "~D", 0x89),
                ("~L", 0x85, "~R", 0x8a), ("~F", 0x88, "~E", 0x90), ("~H", 0x87, "~V", 0x86)];
            for (i, (g0, l0, g1, l1)) in rows.iter().enumerate() {
                let y = 222.0 + i as f32 * 19.0;
                glyph_row(h, font, &format!("{g0} {}", lbl(*l0, "")), 112.0, y, cream);
                glyph_row(h, font, &format!("{g1} {}", lbl(*l1, "")), 330.0, y, cream);
            }
        }
        // fix-pause-menu2: the SCORE tab — the stats list box (98,147,445×190) [L the d1=8 control]
        // filled exactly as the game's msg-0x4e case-3 arm does [M 0x204408:447-498]: the
        // Category/Current/Target header (0x1000204/0x10001e1/0x10001e2), then Bond Moves, the
        // indented Opponents trio, Accuracy Rating, Health Remaining, Time, 007 Bonus. Current =
        // the port's tracked stats (time, accuracy); the other counters are the port's zeros
        // (reported). Targets = SCORE_TARGETS for the level's campaign row.
        3 => {
            plate(h, 98.0, 147.0, 445.0, 190.0, gs(0x1e1210, 0x60));
            h.text(Anchor::Centre, font, -2, &lbl(0x1000_0204, "Category"), mx(98.0 + 30.0), my(155.0), false, bright, false);
            h.text(Anchor::Centre, font, -2, &lbl(0x1000_01e1, "Current"), mx(98.0 + 280.0), my(155.0), false, bright, false);
            h.text(Anchor::Centre, font, -2, &lbl(0x1000_01e2, "Target"), mx(98.0 + 395.0), my(155.0), false, bright, false);
            let st = &fe.stats;
            let row = score_row_for_level(fe.flow.current_level).unwrap_or(0);
            let t = SCORE_TARGETS[row.min(11)];
            let time_s = |s: u32| format!("{:02}:{:02}", s / 60, s % 60);
            // (category label, indented?, current, target) — the game's row order [M :461-498].
            let rows: [(u32, bool, String, String); 8] = [
                (0x165, false, "0".into(), t[0].to_string()),                       // Bond Moves
                (0x15a, false, String::new(), String::new()),                       // Opponents (parent)
                (0x15b, true, "0".into(), t[1].to_string()),                        // Dispatched
                (0x15c, true, "0".into(), t[2].to_string()),                        // Subdued
                (0x16e, true, "0".into(), t[3].to_string()),                        // Surrendered
                (0x15e, false, format!("{}%", st.accuracy_pct), t[5].to_string()),  // Accuracy Rating
                (0x15f, false, "100".into(), "100".into()),                          // Health Remaining [M :436/:487-490 the literal 100]
                (0x161, false, "0".into(), t[8].to_string()),                       // 007 Bonus
            ];
            let mut y = 173.0;
            for (id, indent, cur, tgt) in rows.iter() {
                let x = if *indent { 98.0 + 40.0 } else { 98.0 + 30.0 };
                if !cur.is_empty() || !tgt.is_empty() {
                    h.text(Anchor::Centre, font, -2, &lbl(*id, ""), mx(x), my(y), false, cream, false);
                    h.text(Anchor::Centre, font, -2, cur, mx(98.0 + 280.0), my(y), false, bright, false);
                    h.text(Anchor::Centre, font, -2, tgt, mx(98.0 + 395.0), my(y), false, bright, false);
                }
                y += 19.0;
            }
            // The game's own Time row last [M :491-494]: current ST+0x94 seconds, target 0x8c×100.
            h.text(Anchor::Centre, font, -2, &lbl(0x160, "Time"), mx(98.0 + 30.0), my(y), false, cream, false);
            h.text(Anchor::Centre, font, -2, &time_s(st.time_s), mx(98.0 + 280.0), my(y), false, bright, false);
            h.text(Anchor::Centre, font, -2, &time_s(t[7]), mx(98.0 + 395.0), my(y), false, bright, false);
        }
        // fix-pause-menu2: the OBJECTIVES tab — one row per objective, a ~J (done, state 4) or
        // ~I (pending) marker then the objective's name [M C_GCPAUSE_Handler 0x204408:251-271 —
        // the rows are Mission_ObjectiveState + Txt_BindLabel(state label); the markers are
        // ACTION.ELF .data 0x30d2e0 "~J" / 0x30d2e8 "~I"; the specialchar 'J'/'I' glyphs draw
        // them [L]]. The host fills fe.pause_objectives from the sim's mission states.
        1 => {
            plate(h, 153.0, 183.0, 334.0, 120.0, gs(0x1e1210, 0x60));
            for (i, (label, done)) in fe.pause_objectives.iter().enumerate().take(5) {
                let y = 192.0 + i as f32 * 22.0;
                let marker = if *done { "~J" } else { "~I" };
                glyph_row(h, font, &format!("{marker} {}", lbl(*label, "(objective)")), 170.0, y, if *done { dim } else { bright });
            }
        }
        // The MISSION tab (default [L the page's initial state — the d1=6 list is the enabled one]).
        _ => {
            // The rows [L the d1=6 list (153,183,334×120) m0x11 labels 0x16a CONTINUE / 0x16b RESTART /
            // 0x16c QUIT; the row colour 0x7d6d5980 — the selected row at full alpha [S, the wheel's
            // pattern]].
            let rows = [0x16au32, 0x16b, 0x16c];
            let fallback = ["CONTINUE", "RESTART", "QUIT"];
            for (i, (&id, fb)) in rows.iter().zip(fallback).enumerate() {
                let c = if i == fe.cursor { bright } else { dim };
                h.text(Anchor::Centre, font, -2, &lbl(id, fb), mx(153.0 + 167.0), my(183.0 + 20.0 + i as f32 * 40.0), false, c, false);
            }
            // fix-pause-menu2: the yellow arrow cursor left of the selected row [L the main agent's
            // PCSX2 captures 2026-10-13 — the game's list cursor; the port had none]. Drawn as a
            // small yellow triangle at the list's cursor column (the port's stand-in sprite; the
            // disc's own cursor texture was not identified — live check).
            pause_cursor(h, 218.0, 183.0 + 20.0 + fe.cursor.min(2) as f32 * 40.0, gs(YELLOW, 0xff));
        }
    }
    // The YES/NO confirm box over the rows [L the d1=0xc text (187,188,266×16) and the d1=0xd list
    // (238,210,165×86) m0x11 rows 0x5df YES / 0x5e0 NO; the text is 0x1ff "RESTART: ARE YOU SURE?" or
    // 0x200 "QUIT: ARE YOU SURE?" [M 0x204408's msg-0x4b d1=6 cases]; the selection opens on NO
    // [L m0x1d:1; M :181/204 re-sends msg 0x1d=1].]
    if fe.confirming_restart || fe.confirming_quit {
        let text = if fe.confirming_quit { lbl(0x200, "QUIT: ARE YOU SURE?") } else { lbl(0x1ff, "RESTART: ARE YOU SURE?") };
        h.text(Anchor::Centre, font, -2, &text, mx(187.0 + 133.0), my(188.0 + 8.0), false, bright, false);
        let yn = [0x5dfu32, 0x5e0];
        for (i, &id) in yn.iter().enumerate() {
            let c = if i == fe.confirm_sel { bright } else { dim };
            h.text(Anchor::Centre, font, -2, &lbl(id, if i == 0 { "YES" } else { "NO" }), mx(238.0 + 82.0), my(210.0 + 12.0 + i as f32 * 36.0), false, c, false);
        }
    }
    // The hint bar (45,341,550×18) — the TAB's own label [M C_GCPAUSE_Handler 0x204408 swaps the
    // hint per tab on entry: MISSION 0x201 (:275), OBJECTIVES 0x202 (:239), CONTROLS 0x204 (:501),
    // SCORE 0x205 (:288); the confirm swaps it to 0x1000323 [M :160/175/198]]. The fallbacks are
    // the captures' footers [L the main agent's PCSX2 captures 2026-10-13].
    let hint_id = if fe.confirming_restart || fe.confirming_quit { 0x100_0323 } else {
        [0x201u32, 0x202, 0x204, 0x205][tab]
    };
    let hint_fb = match (tab, fe.confirming_restart || fe.confirming_quit) {
        (_, true) => "~A Select  ~B Back  ~V Scroll",
        (1, false) => "~A Hint  ~V Scroll  ~H Next",
        (2, false) => "~A Invert  ~V Scroll  ~H Next",
        _ => "~A Select  ~V Scroll  ~H Next",
    };
    hint_bar(h, font, &hint_wrap(lang, hint_id, hint_fb), 45.0, 341.0, cream);
}

/// fix-pause-menu2: a menu-space text row with the specialchar glyphs inline (the "~A Label" rows of
/// the CONTROLS/OBJECTIVES tabs). The game's list rows ARE menu text: `sprintf("%s %s", glyph, label)`
/// [M 0x1f44d8] rendered through the same text path as the hint bars — so the port reuses hint_bar's
/// tokenizer at a row pen (hint_bar already walks ~tokens with the SPECIALCHAR table).
fn glyph_row(h: &mut Hud, font: &nf_formats::hud::Font, text: &str, x640: f32, y640: f32, c: [f32; 4]) {
    hint_bar(h, font, text, x640, y640, c);
}

/// The MISSION tab's row cursor: a small filled triangle pointing right at (x, y centre) [L the
/// captures' yellow arrow; the disc's own cursor sprite was not identified — the port's stand-in,
/// colour the title's yellow 0xfec43e [M page 0x40000009's 0x1b colour]].
fn pause_cursor(h: &mut Hud, x: f32, y: f32, c: [f32; 4]) {
    let (a, b, d) = (h.ndc(Anchor::Centre, mx(x), my(y - 6.0)), h.ndc(Anchor::Centre, mx(x), my(y + 6.0)), h.ndc(Anchor::Centre, mx(x + 10.0), my(y)));
    let p = |q: [f32; 2]| HudVertex { p: q, uv: [0.03, 0.03], c };
    // One triangle: the first 3 verts; the second is degenerate (the frame wants 6).
    let v = [p(a), p(b), p(d), p(a), p(d), p(d)];
    h.ndc_quad(-3, v, c);
}

/// The mission-end pages. Outcome::Failed = the fail/death menu, menu-def page 0x40000042 [M
/// Mission_Update 0x182c60:78 creates it on death; handler P_ENDMISSION_Handler 0x216618].
/// Outcome::Complete = the results page 0x40000036 [M Mission_Update 0x182c60:74 sets +0x28;
/// P_NFRESULTS 0x208e78 fills it].
fn results_page(h: &mut Hud, fe: &FrontEnd, font: &nf_formats::hud::Font, lang: Option<&Lang>, textures: &[(u32, crate::scene::TextureImage)], cream: [f32; 4], bright: [f32; 4], dim: [f32; 4]) {
    let bind = |id: u32| -> String { lang.and_then(|l| l.bind(id)).unwrap_or("").to_string() };
    let lbl = |id: u32, fb: &str| -> String { let s = bind(id); if s.is_empty() { fb.into() } else { s } };
    match fe.outcome {
        Outcome::Failed => {
            // The fail menu's art [L page 0x40000042's controls: the plate 0x10000001 (45,107,550×266)
            // TEX 0x03000070, the Bond image 0x10000184 (385,121,202×218) TEX 0x030000db].
            tex_quad_or_plate(h, textures, 0x0300_0070, 45.0, 107.0, 550.0, 266.0, [0.0, 0.0, 512.0, 256.0], gs(0x7f7f7f, 0xff));
            // The burning woman, as on the pause page (crate::woman; flags 0x100 additive).
            h.quad(Anchor::Centre, 0, 0x0300_00db, 1, mx(385.0), my(121.0), mx(202.0), my(218.0), [0.0, 0.0, 128.0, 128.0], (false, false), gs(0x808080, 0xff));
            // The header (105,120,431×16) LBL 0x2fd "Mission Failed" [L the page's 0x10000001 m0x18].
            h.text(Anchor::Centre, font, -2, &lbl(0x2fd, "MISSION FAILED"), mx(105.0 + 215.0), my(120.0 + 8.0), false, bright, true);
            // The rows [L the 0x100001a4 list (197,186,246×115) m0x11 rows 0x5bc "R E P L A Y" /
            // 0x16c "Q U I T"; row 1 = 0x16b "R E S T A R T" [S — the pause page's own label; the
            // game fills the middle row at runtime]. The ACTIONS are the handler's arms [M
            // 0x216618:19-27].]
            let rows = [(0x5bcu32, "REPLAY"), (0x16b, "RESTART"), (0x16c, "QUIT")];
            for (i, (id, fb)) in rows.iter().enumerate() {
                let c = if i == fe.cursor { bright } else { dim };
                h.text(Anchor::Centre, font, -2, &lbl(*id, fb), mx(197.0 + 123.0), my(186.0 + 12.0 + i as f32 * 36.0), false, c, false);
            }
            // The hint (97,341,446×18) LBL 0x5c8 [L the page's 0x1000006f].
            hint_bar(h, font, &hint_wrap(lang, 0x5c8, "~A Select  ~V Scroll"), 97.0, 341.0, cream);
        }
        Outcome::Complete => {
            // The results page's layout [L page 0x40000036's controls: header 0x10000001 (50,40,376×17)
            // LBL 0x2c5 "Results"; the photo 0x10000166 (135,80,370×186) TEX 0x03000140; the medal
            // plate 0x100001d1 (49,264,541×37); "Total Score:" 0x10000001 (48,336,264×16) LBL 0x2cb;
            // the target line 0x1000002b (47,360,266×16) — its static LBL 0x2cd is REPLACED at enter
            // [M P_NFRESULTS 0x208e78:163-164 sends the medal's own target label 0x2cc..0x2cf]; the
            // score numbers 0x10000167/0x10000168 (320,336/360,248×16) [M :97/:165 fills them]; the
            // stamp 0x10000224 TEX 0x0300014c; the logo (434,38,156×43) TEX 0x0300016d].
            h.text(Anchor::Centre, font, -2, &lbl(0x2c5, "RESULTS"), mx(50.0 + 188.0), my(40.0 + 8.0), false, bright, true);
            if textures.iter().any(|(hash, _)| *hash == 0x0300_014c) { quad640(h, 0x0300_014c, 539.0, 393.0, 255.0, 255.0, [0.0, 0.0, 256.0, 256.0], gs(0x7f7f7f, 0xff)); }
            if textures.iter().any(|(hash, _)| *hash == 0x0300_016d) { quad640(h, 0x0300_016d, 434.0, 38.0, 156.0, 43.0, [0.0, 0.0, 256.0, 128.0], gs(0x7f7f7f, 0xff)); }
            // The fields [M 0x208e78:96-167]. The photo control's texture is the MEDAL EMBLEM at
            // runtime [M :166-167 msg 0x24 — 0x300013d/e/f, 0x300019a]; the page's static photo
            // 0x03000140 stands in until the enter built the fields.
            match &fe.results {
                Some(rf) => {
                    tex_quad_or_plate(h, textures, rf.emblem, 135.0, 80.0, 370.0, 186.0, [0.0, 0.0, 256.0, 128.0], gs(0x7f7f7f, 0xff));
                    // The medal plate 0x100001d1 [M :159-161]: the medal name / no-medal line.
                    plate(h, 49.0, 264.0, 541.0, 37.0, gs(0x1e1210, 0x60));
                    h.text(Anchor::Centre, font, -2, &rf.medal_text, mx(49.0 + 270.0), my(264.0 + 18.0), false, bright, false);
                    // "Total Score:" is the page's own static label [L]; the number 0x10000167 is
                    // SeparateNumber(+0x34) [M :96-97].
                    h.text(Anchor::Centre, font, -2, &lbl(0x2cb, "Total Score:"), mx(48.0 + 132.0), my(336.0 + 8.0), false, cream, false);
                    h.text(Anchor::Centre, font, -2, &separate_number(rf.score), mx(320.0 + 124.0), my(336.0 + 8.0), false, bright, false);
                    // The target line [M :163-165]: Txt(target_label) + SeparateNumber(threshold).
                    h.text(Anchor::Centre, font, -2, &lbl(rf.target_label, match rf.target_label {
                        0x2cc => "Platinum Medal Target:", 0x2cd => "Gold Medal Target:",
                        0x2ce => "Silver Medal Target:", _ => "Bronze Medal Target:" }),
                        mx(47.0 + 133.0), my(360.0 + 8.0), false, cream, false);
                    h.text(Anchor::Centre, font, -2, &separate_number(rf.target), mx(320.0 + 124.0), my(360.0 + 8.0), false, bright, false);
                }
                None => {
                    tex_quad_or_plate(h, textures, 0x0300_0140, 135.0, 80.0, 370.0, 186.0, [0.0, 0.0, 256.0, 128.0], gs(0x7f7f7f, 0xff));
                    h.text(Anchor::Centre, font, -2, &lbl(0x2cb, "Total Score:"), mx(48.0 + 132.0), my(336.0 + 8.0), false, cream, false);
                    h.text(Anchor::Centre, font, -2, "—", mx(320.0 + 124.0), my(336.0 + 8.0), false, bright, false);
                    h.text(Anchor::Centre, font, -2, &lbl(0x2cd, "Gold Medal Target:"), mx(47.0 + 133.0), my(360.0 + 8.0), false, cream, false);
                    h.text(Anchor::Centre, font, -2, "—", mx(320.0 + 124.0), my(360.0 + 8.0), false, bright, false);
                }
            }
            // The hint (50,419,540×21) LBL 0x2d0 [L the page's control #2] — the page's own action.
            hint_bar(h, font, &hint_wrap(lang, 0x2d0, "~A Continue  ~X Stats  ~Y Dossier"), 50.0, 419.0, cream);
        }
    }
}

/// `SeparateNumber` 0x1dec78 — the score texts' comma grouping: "%d" under 1000, "%d,%.3d" under
/// a million, "%d,%.3d,%.3d" above [M the sprintf formats at 0x30d098/0x30d090; the third arm's
/// dropped args are the decompiler's — the groups are thousands/hundreds-of-thousands/millions].
pub fn separate_number(v: u32) -> String {
    let thousands = (v / 1000) % 1000;
    if v / 1_000_000 == 0 {
        if thousands == 0 { format!("{}", v % 1000) }
        else { format!("{},{}", thousands, v % 1000) }
    } else {
        format!("{},{:03},{:03}", v / 1_000_000, thousands, v % 1000)
    }
}

/// `hint`-style fallback for the menu_overlay-external drawers (a bound hint or the literal).
fn hint_wrap(lang: Option<&Lang>, id: u32, fallback: &str) -> String {
    let s = lang.and_then(|l| l.bind(id)).unwrap_or("").to_string();
    if s.is_empty() { fallback.into() } else { s }
}

/// The five-slot wheel the list pages draw [M the rows' CONTROL records on pages 0x40000025/
/// 0x40000023/0x4000001c/0x4000002b (parsed 2026-10-14): slot 0 (323,123,317×17), slot 1 (333,152,307×17),
/// slot 2 (304,182,336×17), slot 3 (333,211,307×17), slot 4 (323,241,317×17); the static colours are
/// 0x7d6d59xx with alpha 0x40/0x80/0xff (slot 2 selected)]. `Label_Update` 0x218cb8:177-182 centres a
/// label's sprite at ctrl.x + ctrl.w/2 (the default label format DAT_003096c0 = ff 02 fe 03 — SerpLight
/// + centre [M Label_Init 0x218638:53]), and :202-206 centres it vertically.
const WHEEL_Y: [f32; 5] = [123.0, 152.0, 182.0, 211.0, 241.0];
const WHEEL_X: [f32; 5] = [323.0, 333.0, 304.0, 333.0, 323.0];
const WHEEL_W: [f32; 5] = [317.0, 307.0, 336.0, 307.0, 317.0];

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
        // Locked rows go grey [M Menu_UpdateWheel 0x1f8388:210-234 sends the locked rows 0x484848xx].
        if locked(fe, row) { c = gs(0x484848, 0x80); }
        let s = label(fe, row);
        // The label sprite centres in its control: x = ctrl.x + ctrl.w/2, y = ctrl.y + ctrl.h/2
        // [M Label_Update 0x218cb8:177-182 (x), :202-206 (y); the default format's fe 03].
        centred(h, font, -2, &s, mx(WHEEL_X[slot] + WHEEL_W[slot] / 2.0), my(WHEEL_Y[slot] + 8.5), c, true, scale);
    }
}

fn dimmed(mut c: [f32; 4], a: u32) -> [f32; 4] { c[3] = gs(0, a)[3]; c }

/// Centre one text row at 640-space x `xc` [M `__Font_DrawText` 0x1eee48:69-71 — the format's fe 03
/// flag 2 applies `x -= width * 0.5`; every menu label carries fe 03 (Label_Init 0x218638:53's
/// DAT_003096c0). The port's `text_scaled` pen starts at x, so the half-width shift lands here.]
fn centred(h: &mut Hud, font: &nf_formats::hud::Font, layer: i32, s: &str, xc: f32, y: f32, c: [f32; 4], shadow: bool, scale: f32) {
    // The drawn ink of one row: the glyph quads span [gx, gx + gw] (quad() maps both corners, so
    // the anchor is only an origin shift — the glyphs are left-edged at their pen x), giving
    // [p0.x, plast.x + gwlast] · scale. Its centre goes to `xc` — the game's fe 03 centring
    // [M 0x1eee48:69-71 applies x -= width*0.5 to the same measure the draw uses].
    let ink_centre = || -> f32 {
        let (placed, _) = font.layout(s, 0.0, 0.0);
        match (placed.first(), placed.last()) {
            (Some(p0), Some(pl)) => ((p0.x + pl.x + font.glyphs[pl.glyph].w as f32) / 2.0) * scale,
            _ => 0.0,
        }
    };
    let mid = ink_centre();
    h.text_scaled(Anchor::Centre, font, layer, s, xc - mid, y, false, c, shadow, scale);
}

/// `centred` for a font with its own atlas texture (the header face 0x03000005 — see page_header).
fn centred_tex(h: &mut Hud, font: &nf_formats::hud::Font, tex: u32, layer: i32, s: &str, xc: f32, y: f32, c: [f32; 4], shadow: bool, scale: f32) {
    let ink_centre = || -> f32 {
        let (placed, _) = font.layout(s, 0.0, 0.0);
        match (placed.first(), placed.last()) {
            (Some(p0), Some(pl)) => ((p0.x + pl.x + font.glyphs[pl.glyph].w as f32) / 2.0) * scale,
            _ => 0.0,
        }
    };
    let mid = ink_centre();
    h.text_scaled_tex(Anchor::Centre, font, layer, tex, s, xc - mid, y, false, c, shadow, scale);
}

/// The page header: the header control's own band (50,40,376×17) with the label centred in it
/// [M every list page's header control (parsed 2026-10-14); Label_Update 0x218cb8:177-182 centres at
/// ctrl.x + ctrl.w/2 = 238, :202-206 vertically at 48.5; the label colour is the control's own
/// msg-0x1b 0x7d6d59ff → 0x645a49ff (MenuManager_Create 0x21b908:238-243's rewrite)].
/// The font is the control's own msg-0x23 (page-def NAME tag −8) format override: the header
/// controls carry "\xff\x01" (font select 1 = FONTNFont2, the bold-italic face, texture
/// 0x03000005) — Label_Init 0x218638 stamps the SerpLight default (DAT_003096c0 [L ee_snapshot
/// 0x3096c0 = ff 02 fe 03 …]) on every label, then MenuManager_Create 0x21b908:255-274 sends the
/// page's 32-byte (UTF-16, compacted) NAME string as msg 0x23 [M Label_SendMessage 0x218890:100-104
/// sprintf's it over label+0x94 and points label+0xc0 at it]; Font_ParseFormat 0x1ee3e0 stops at
/// the first NUL, so the effective format is the \xff\x01 font select [L the page dump: header
/// NAME = ff 00 01 00 00 00 …]. FontTable[0] = {tex 0x03000005, FONTNFont2 0x305a58} [L
/// ee_snapshot 0x2dd8b0].]
fn page_header(h: &mut Hud, hfont: &nf_formats::hud::Font, s: &str) {
    centred_tex(h, hfont, 0x0300_0005, -1, s, mx(238.0), my(48.5), gs(CREAM, 0xff), true, 1.2);
}

/// The panel behind the rows [M pages 0x40000025/0x40000023/0x4000001c/0x4000002b control (50,53,590×295)
/// TEX 0x030000b3].
fn panel(h: &mut Hud) {
    quad640(h, 0x0300_00b3, 50.0, 53.0, 590.0, 295.0, [0.0, 0.0, 512.0, 256.0], gs(0x7f7f7f, 0xff));
}

/// The ring emblem stack: the page's ring-masked preview UNDER the gold ring, both (114,118,150×150)
/// [M the emblem controls on pages 0x40000025/0x40000023/0x4000001c/0x4000002b; the ring's f1 layer
/// (0x1c) is below the preview's (0x1d/0x1e) and lower layers draw on top [M hud.rs finish]]. The
/// preview is clipped to the ring's circle — the game masks it (the port's square preview poked out
/// of the ring as a visible box [L the main agent's 2026-10-13 render]).
fn emblem(h: &mut Hud, preview: u32, ring: u32) {
    emblem_col(h, preview, ring, gs(0x7f7f7f, 0xff), gs(0x7f7f7f, 0xff));
}

/// `emblem` with per-quad colours (the locked rows' preview dims to 0x60 [M 0x1f8388's 0x60606060]).
fn emblem_col(h: &mut Hud, preview: u32, ring: u32, c: [f32; 4], ring_c: [f32; 4]) {
    circle_clipped_quad(h, preview, 114.0, 118.0, 150.0, 150.0, [0.0, 0.0, 128.0, 128.0], c);
    quad640(h, ring, 114.0, 118.0, 150.0, 150.0, [0.0, 0.0, 128.0, 128.0], ring_c);
}

/// A textured quad clipped to the rect's inscribed circle: 24 vertical strips, each the chord of the
/// circle at that x range, UVs proportional. The game masks the preview to the ring (the emblem
/// draw order + the square-box render); a `HudFrame` has no per-pixel mask, so the port clips
/// geometrically — 24 strips' chord steps are under 0.1 px at the 150-px emblem.
fn circle_clipped_quad(h: &mut Hud, tex: u32, x: f32, y: f32, w: f32, hh: f32, uv: [f32; 4], c: [f32; 4]) {
    let n = 24;
    let (cx, cy, r) = (x + w / 2.0, y + hh / 2.0, w / 2.0);
    for k in 0..n {
        let sx0 = x + w * k as f32 / n as f32;
        let sx1 = x + w * (k + 1) as f32 / n as f32;
        // The chord at the strip's FAR edge: a rectangle's corners must stay inside the circle, so
        // the y range is clipped at the edge farther from the centre (the strip then sits a hair
        // inside the circle — under 0.1 px at the 150-px emblem — never poking out).
        let dx = (sx1 - cx).max(cx - sx0);
        let half = (r * r - dx * dx).max(0.0).sqrt();
        if half <= 0.0 { continue; }
        let (sy0, sy1) = (cy - half, cy + half);
        let u0 = uv[0] + (sx0 - x) / w * uv[2];
        let v0 = uv[1] + (sy0 - y) / hh * uv[3];
        quad_uv(h, tex, mx(sx0), my(sy0), mx(sx1 - sx0), my(sy1 - sy0), [u0, v0, (sx1 - sx0) / w * uv[2], (sy1 - sy0) / hh * uv[3]], c);
    }
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
    for (i, line) in wrap_px(text, font, 500.0).iter().take(4).enumerate() {
        centred(h, font, -1, line, mx(320.0), my(306.0 + i as f32 * 22.0), c, true, 1.0);
    }
}

/// The hint bar with the game's button glyphs [M the bars at (50,419,540×21); the strings carry ~A/~B/
/// ~X/~Y/~V/~H/~S placeholders and the menu text renderer draws them through the `specialchar` table
/// 0x2dd8d8 — texture 0x03000075, per-token (u, v, w, h, advance) [M the table's bytes]. The port's
/// earlier 64×64-atlas 0x03000042 tiles were a guess, and ~A (the ✕) fell back to the font's letter X
/// — both wrong against the PCSX2 captures. fix-menus-real: the bar's label CENTRES in the bar [M
/// Label_Update 0x218cb8:177-182 sets the sprite x to ctrl.x + ctrl.w/2 = 320 (the default label
/// format's fe 03) — the PCSX2 captures show the footer hints centred; the port's old left edge at
/// x=50 was the control's x, not the text's anchor].]
fn hint_bar(h: &mut Hud, font: &nf_formats::hud::Font, text: &str, x640: f32, y640: f32, c: [f32; 4]) {
    // One pass collects each item's ink span relative to the row's pen origin (text glyphs sit at
    // [p0.x − gw0/2, plast.x + gwlast/2] — the per-glyph quads centre at their pen x — and an icon's
    // quad centres at its pen x, so [−iw/2, +iw/2]); the row's ink centre then goes to the bar's
    // centre [M Label_Update 0x218cb8:177-182 puts the label sprite at ctrl.x + ctrl.w/2].
    enum Item { Text { x: f32, ink: (f32, f32) }, Icon { tex: u32, uv: [f32; 4], w: f32, hh: f32, ink: (f32, f32) } }
    let mut items: Vec<Item> = Vec::new();
    let mut adv_sum = 0.0f32;
    let mut rest = text;
    while let Some(t) = rest.find('~') {
        let (before, after) = rest.split_at(t);
        if !before.is_empty() {
            let (placed, nat) = font.layout(before, 0.0, 0.0);
            let (a, b) = match (placed.first(), placed.last()) {
                (Some(p0), Some(pl)) => (p0.x, pl.x + font.glyphs[pl.glyph].w as f32),
                _ => (0.0, nat),
            };
            items.push(Item::Text { x: adv_sum, ink: (adv_sum + a, adv_sum + b) });
            adv_sum += nat;
        }
        match after.chars().nth(1).and_then(|t| SPECIALCHAR.iter().find(|s| s.0 == t)) {
            Some(&(_, tex, u, v, w, hh, adv)) => {
                let (iw, ih) = (mx(w), my(hh));
                items.push(Item::Icon { tex, uv: [u, v, w, hh], w: iw, hh: ih, ink: (adv_sum - iw / 2.0, adv_sum + iw / 2.0) });
                adv_sum += mx(adv) + 2.0;
            }
            _ => { adv_sum += mx(16.0); }
        }
        rest = &after[2.min(after.len())..];
        if rest.starts_with(' ') { rest = &rest[1..]; adv_sum += 4.0; }
    }
    if !rest.is_empty() {
        let (placed, nat) = font.layout(rest, 0.0, 0.0);
        let (a, b) = match (placed.first(), placed.last()) {
            (Some(p0), Some(pl)) => (p0.x, pl.x + font.glyphs[pl.glyph].w as f32),
            _ => (0.0, nat),
        };
        items.push(Item::Text { x: adv_sum, ink: (adv_sum + a, adv_sum + b) });
    }
    let ink_lo = items.iter().map(|i| match i { Item::Text { ink, .. } | Item::Icon { ink, .. } => ink.0 }).fold(f32::MAX, f32::min);
    let ink_hi = items.iter().map(|i| match i { Item::Text { ink, .. } | Item::Icon { ink, .. } => ink.1 }).fold(f32::MIN, f32::max);
    // The bar's centre: ctrl.x + ctrl.w/2 = 50 + 540/2 = 320 in 640-space [L the parsed bars].
    let bar_c = mx(x640 + 270.0);
    let pen0 = bar_c - (ink_lo + ink_hi) / 2.0;
    if std::env::var_os("NF_DEBUG_HINT").is_some() { eprintln!("[hint] text={text:?} items={} ink=({ink_lo},{ink_hi}) bar_c={bar_c} pen0={pen0}", items.len()); }
    let y = my(y640);
    // Draw from the collected items: each text segment re-lays-out at its absolute pen (the ink
    // spans land where the measure put them), the icons at theirs.
    let mut rest = text;
    let mut ti = 0usize;
    let mut pen_at = |items: &Vec<Item>, i: usize| match &items[i] { Item::Text { x, .. } => *x, Item::Icon { .. } => 0.0 };
    let _ = &mut pen_at;
    let mut seg_pen = 0.0f32;
    while let Some(t) = rest.find('~') {
        let (before, after) = rest.split_at(t);
        if !before.is_empty() {
            let x = pen0 + seg_pen;
            h.text(Anchor::Centre, font, -2, before, x, y, false, c, true);
            seg_pen += font.layout(before, 0.0, 0.0).1;
            ti += 1;
        }
        match after.chars().nth(1).and_then(|t| SPECIALCHAR.iter().find(|s| s.0 == t)) {
            Some(&(_, tex, u, v, w, hh, adv)) => {
                if let Item::Icon { ink, .. } = &items[ti.min(items.len() - 1)] {
                    let (lo, hi) = (ink.0, ink.1);
                    let _ = (lo, hi);
                }
                let iw = mx(w);
                let ih = my(hh);
                // The icon's ink [pen − iw/2, pen + iw/2] — same anchor as the measure pass.
                h.quad(Anchor::Centre, -2, tex, 0, pen0 + seg_pen, y - ih + my(12.0), iw, ih, [u, v, w, hh], (false, false), c);
                seg_pen += mx(adv) + 2.0;
                ti += 1;
            }
            _ => { seg_pen += mx(16.0); }
        }
        rest = &after[2.min(after.len())..];
        if rest.starts_with(' ') { rest = &rest[1..]; seg_pen += 4.0; }
    }
    if !rest.is_empty() {
        h.text(Anchor::Centre, font, -2, rest, pen0 + seg_pen, y, false, c, true);
    }
}

/// A textured quad in the menu's 640 × 480 space. Anchor::Centre keeps every quad inside the centred
/// 4:3 picture: the game draws the 640×480 menu space as a centred 4:3 image with black bars at the
/// sides [L the main agent's PCSX2 captures 2026-10-13; the port's Anchor::Stretch quads filled the
/// whole 16:9 window]. At a 4:3 window Centre ≡ Stretch, so 4:3 renders are unchanged.
#[allow(clippy::too_many_arguments)]
fn quad640(h: &mut Hud, tex: u32, x: f32, y: f32, w: f32, hh: f32, uv: [f32; 4], c: [f32; 4]) {
    quad_uv(h, tex, mx(x), my(y), mx(w), my(hh), uv, c);
}

/// A textured quad at screen-pixel coords with the quad's own uv rect (the circle strips need it).
#[allow(clippy::too_many_arguments)]
fn quad_uv(h: &mut Hud, tex: u32, x: f32, y: f32, w: f32, hh: f32, uv: [f32; 4], c: [f32; 4]) {
    h.quad(Anchor::Centre, 0, tex, 0, x, y, w, hh, uv, (false, false), c);
}

/// A flat coloured plate (the menu backdrop / the confirm box), inside the centred 4:3 band — see
/// `quad640` [L the main agent's captures].
fn plate(h: &mut Hud, x: f32, y: f32, w: f32, hh: f32, c: [f32; 4]) {
    let (a, b) = (h.ndc(Anchor::Centre, x, y), h.ndc(Anchor::Centre, x + w, y + hh));
    let p = |q: [f32; 2]| HudVertex { p: q, uv: [0.03, 0.03], c };
    let v = [p([a[0], a[1]]), p([b[0], a[1]]), p([b[0], b[1]]), p([a[0], a[1]]), p([b[0], b[1]]), p([a[0], b[1]])];
    h.ndc_quad(-5, v, c);
}

/// Greedy word wrap to a pixel width, measured with the real font (the game's text renderer wraps
/// inside its control's width [M the plates; `Font::layout` measures one line]). The port's earlier
/// char-count wrap (`wrap`) overfilled the plates and its mission-description call even re-joined
/// the lines back into one unwrapped string.
fn wrap_px(s: &str, font: &nf_formats::hud::Font, max_w: f32) -> Vec<String> {
    let mut out = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        for w in para.split_whitespace() {
            let cand = if line.is_empty() { w.to_string() } else { format!("{line} {w}") };
            if !line.is_empty() && font.layout(&cand, 0.0, 0.0).1 > max_w {
                out.push(std::mem::take(&mut line));
                line = w.to_string();
            } else { line = cand; }
        }
        out.push(line);
    }
    out
}

/// `wrap_px` for the tests (the draw path keeps it private).
pub fn wrap_px_for_test(s: &str, font: &nf_formats::hud::Font, max_w: f32) -> Vec<String> { wrap_px(s, font, max_w) }

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
