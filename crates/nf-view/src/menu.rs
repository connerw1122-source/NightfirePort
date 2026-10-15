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
/// Codenames → "Edit Codename" [L 07000048 page 0x4000001b: header LBL 0x1f1, hint 0x20a; C_GOCODENAMES 0x216b38].
pub const PAGE_CN_SELECT: u32 = 0x4000_001b;
/// "Edit <name>": the codename options wheel [L page 0x4000001d; C_SBCNOPTIONS_Handler 0x213528, table 0x2dfc28].
pub const PAGE_CN_MENU: u32 = 0x4000_001d;
/// The codename-entry / Secret Unlocks page [M Handler_HandleMessage 0x2029e0:372-374 routes
/// 0x40000020 to P_CNNAME_Handler 0x213790; C_SBCNOPTIONS_Handler 0x213528 case 0 opens it].
/// Entered from the codename wheel (0x4000001d) it runs in cheat mode [M 0x213790:31-38] — the
/// Secret Unlocks letter grid; its model is [`SecretEntry`]. From Select Codename 0x40000025 /
/// Edit Codename 0x4000001b it is the PROFILE-name entry — the validator + create flow in the
/// `cn_*` API [M C_KEYBOARD_Handler 0x213a98:45-65].
/// Advanced Options [L page 0x4000002d; P_CNOPTIONS_Handler 0x213f08].
pub const PAGE_CN_NAME: u32 = 0x4000_0020;
/// Advanced Options [L page 0x4000002d; P_CNOPTIONS_Handler 0x213f08].
pub const PAGE_CN_ADVANCED: u32 = 0x4000_002d;
/// Multiplayer Options [L page 0x4000002e; P_CNMPOPTIONS_Handler 0x2144d8].
pub const PAGE_CN_MP: u32 = 0x4000_002e;
/// Screen Adjust — the picture-offset subscreen of AV Options [L page 0x40000047; C_SBCNOPTIONS
/// 0x213528's row 0x10000137 → Manager 0x44 0x40000047 [M 0x214818:117-126]]. Aux movie 0 [L PAGE record].
pub const PAGE_SCREEN_ADJUST: u32 = 0x4000_0047;
/// AV Options [L page 0x40000031; P_CNAVOPTIONS_Handler 0x214818].
pub const PAGE_CN_AV: u32 = 0x4000_0031;
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

// ── mp-menu-flow: the multiplayer front-end pages ─────────────────────────────────────────────
// All 13 PAGE records verified in the disc's menu-def (08000002 in 07000048.bin, parsed 2026-10-15 —
// every one kind 0 with the list pages' aux movie 0x07350048); the handlers are the decomp files
// named per constant.
/// Join Game [M C_GOMULTIPLAYER 0x216a78:18 — the main menu's Multiplayer row sends the delayed
/// msg 0x44 (page change) to 0x40000019; the page's handler is P_MPJOIN 0x20ca00].
pub const PAGE_MP_JOIN: u32 = 0x4000_0019;
/// Select Scenario [M C_RBMPSTART 0x20d048's ready arm → `Manager_SendMessage(0x44, 0x4000001a)`;
/// P_MPSCENARIO 0x217348. Also C_MPDBG 0x217860's post-debriefing target.]
pub const PAGE_MP_SCENARIO: u32 = 0x4000_001a;
/// Select Map [M C_SBMPSCEN 0x20c600's confirm: disasm 0x20c9a8-0x20c9b4 `lui 0x4000; ori 0x13` —
/// the decompiler dropped the constant; P_MPMAP 0x217468, its open checks source == 0x4000001a
/// (disasm 0x217494-0x217498)].
pub const PAGE_MP_MAP: u32 = 0x4000_0013;
/// The per-agent team/character page [M C_SBMPMAP 0x217538's confirm: disasm 0x217638-0x217644
/// `ori 0x51`; P_MPSETUP 0x20d7a8. P_MPOPTIONS 0x217678:12 checks it as the source.]
pub const PAGE_MP_SETUP: u32 = 0x4000_0051;
/// The MP options hub [M C_SBMPOPTIONS 0x20e818's row cases; P_MPBOTS 0x217790's open checks
/// source == 0x40000012].
pub const PAGE_MP_OPTIONS: u32 = 0x4000_0012;
/// Game Rules (Duration + Points/Lives) [M C_SBMPOPTIONS case 2 → 0x40000014; P_MPRULES 0x20f790].
pub const PAGE_MP_RULES: u32 = 0x4000_0014;
/// Player Mods [M C_SBMPOPTIONS case 3 → 0x40000017; P_MPPLAYERMODS 0x2104d0].
pub const PAGE_MP_PLAYERMODS: u32 = 0x4000_0017;
/// Enviro-Mods [M C_SBMPOPTIONS case 4 → 0x40000028; P_MPENVIROMODS 0x210030].
pub const PAGE_MP_ENVIROMODS: u32 = 0x4000_0028;
/// The AI Bots list [M C_SBMPOPTIONS case 1 → Menu_ChangePageCloseIris 0x40000027; P_MPBOTS
/// 0x217790 / C_SBBOTS 0x210a08].
pub const PAGE_MP_BOTS: u32 = 0x4000_0027;
/// The bot character wheel [M C_SBBOTS 0x210a08:23 → 0x4000003f; P_MPBOTCHOOSE 0x210cd8 /
/// C_SBMPBTCHOOSE 0x210f08].
pub const PAGE_MP_BOTCHOOSE: u32 = 0x4000_003f;
/// The per-bot stats page [M C_SBMPBTCHOOSE 0x210f08's confirm → 0x4000002c; P_MPBOTSETUP 0x211270].
pub const PAGE_MP_BOTSETUP: u32 = 0x4000_002c;
/// The match summary [M C_SBMPOPTIONS row 0 → 0x40000049 (disasm context :105); also Quick Game's
/// landing (C_SBMPSCEN disasm 0x20c978-0x20c984). P_MPCONFIRM 0x20ec00 starts the match.]
pub const PAGE_MP_CONFIRM: u32 = 0x4000_0049;
/// The post-match debriefing [M C_GCPAUSE 0x204408:146 — quitting a match (+0x180 != 0) sets
/// GameState+0x28 = 0x40000033; P_MPDEBRIEFING 0x212198 / C_MPDBG 0x217860].
pub const PAGE_MP_DEBRIEFING: u32 = 0x4000_0033;

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
        PAGE_START | PAGE_CODENAME | PAGE_DIFFICULTY | PAGE_MISSION_SELECT | PAGE_DOSSIER | PAGE_RESULTS
        | PAGE_CN_SELECT | PAGE_CN_MENU | PAGE_CN_ADVANCED | PAGE_CN_MP | PAGE_CN_AV => 0x0735_0048,
        // mp-menu-flow: every MP page's PAGE record carries the same list backdrop (the menu-def
        // dump 2026-10-15: all 13 MP pages aux 0x07350048 [L]).
        PAGE_MP_JOIN | PAGE_MP_SCENARIO | PAGE_MP_MAP | PAGE_MP_SETUP | PAGE_MP_OPTIONS | PAGE_MP_RULES
        | PAGE_MP_PLAYERMODS | PAGE_MP_ENVIROMODS | PAGE_MP_BOTS | PAGE_MP_BOTCHOOSE | PAGE_MP_BOTSETUP
        | PAGE_MP_CONFIRM | PAGE_MP_DEBRIEFING => 0x0735_0048,
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
pub enum Page { Start, Main, Codename, Difficulty, MissionSelect, Dossier, Records, Options, Pause, Results, CnSelect, CnMenu, CnName, CnAdvanced, CnMp, CnAv, ScreenAdjust,
    // mp-menu-flow: the multiplayer front end (page ids in the PAGE_MP_* constants below).
    MpJoin, MpScenario, MpMap, MpSetup, MpOptions, MpRules, MpPlayerMods, MpEnviroMods, MpBots, MpBotChoose, MpBotSetup, MpConfirm, MpDebriefing, Credits }

impl Page {
    pub fn id(self) -> u32 {
        match self {
            Page::Start => PAGE_START, Page::Main => PAGE_MAIN, Page::Codename => PAGE_CODENAME,
            Page::Difficulty => PAGE_DIFFICULTY, Page::MissionSelect => PAGE_MISSION_SELECT,
            Page::Dossier => PAGE_DOSSIER, Page::Records => PAGE_RECORDS,
            Page::Options => PAGE_OPTIONS, Page::Pause => PAGE_PAUSE, Page::Results => PAGE_RESULTS,
            Page::CnSelect => PAGE_CN_SELECT, Page::CnMenu => PAGE_CN_MENU, Page::CnName => PAGE_CN_NAME, Page::CnAdvanced => PAGE_CN_ADVANCED,
            Page::CnMp => PAGE_CN_MP, Page::CnAv => PAGE_CN_AV, Page::ScreenAdjust => PAGE_SCREEN_ADJUST,
            Page::MpJoin => PAGE_MP_JOIN, Page::MpScenario => PAGE_MP_SCENARIO, Page::MpMap => PAGE_MP_MAP,
            Page::MpSetup => PAGE_MP_SETUP, Page::MpOptions => PAGE_MP_OPTIONS, Page::MpRules => PAGE_MP_RULES,
            Page::MpPlayerMods => PAGE_MP_PLAYERMODS, Page::MpEnviroMods => PAGE_MP_ENVIROMODS,
            Page::MpBots => PAGE_MP_BOTS, Page::MpBotChoose => PAGE_MP_BOTCHOOSE, Page::MpBotSetup => PAGE_MP_BOTSETUP,
            Page::MpConfirm => PAGE_MP_CONFIRM, Page::MpDebriefing => PAGE_MP_DEBRIEFING,
            Page::Credits => crate::credits::PAGE_CREDITS,
        }
    }
    fn from_id(id: u32) -> Option<Page> {
        Some(match id {
            PAGE_START => Page::Start, PAGE_MAIN => Page::Main, PAGE_CODENAME => Page::Codename,
            PAGE_DIFFICULTY => Page::Difficulty, PAGE_MISSION_SELECT => Page::MissionSelect,
            PAGE_DOSSIER => Page::Dossier, PAGE_RECORDS => Page::Records,
            PAGE_OPTIONS => Page::Options, PAGE_PAUSE => Page::Pause, PAGE_RESULTS => Page::Results,
            PAGE_CN_SELECT => Page::CnSelect, PAGE_CN_MENU => Page::CnMenu, PAGE_CN_NAME => Page::CnName, PAGE_CN_ADVANCED => Page::CnAdvanced,
            PAGE_CN_MP => Page::CnMp, PAGE_CN_AV => Page::CnAv, PAGE_SCREEN_ADJUST => Page::ScreenAdjust,
            PAGE_MP_JOIN => Page::MpJoin, PAGE_MP_SCENARIO => Page::MpScenario, PAGE_MP_MAP => Page::MpMap,
            PAGE_MP_SETUP => Page::MpSetup, PAGE_MP_OPTIONS => Page::MpOptions, PAGE_MP_RULES => Page::MpRules,
            PAGE_MP_PLAYERMODS => Page::MpPlayerMods, PAGE_MP_ENVIROMODS => Page::MpEnviroMods,
            PAGE_MP_BOTS => Page::MpBots, PAGE_MP_BOTCHOOSE => Page::MpBotChoose, PAGE_MP_BOTSETUP => Page::MpBotSetup,
            PAGE_MP_CONFIRM => Page::MpConfirm, PAGE_MP_DEBRIEFING => Page::MpDebriefing,
            crate::credits::PAGE_CREDITS => Page::Credits,
            _ => return None,
        })
    }
}

/// The menu's per-frame input, mapped from mouse + keyboard by the caller (lib.rs). The game's own
/// engine routes cursor moves and confirms through `Menu_InputAction` / action 0x22 [M 0x1f1f40,
/// 0x21c3d8] and skips movies through action 0x19 [M 0x1c98c8 case 6].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { Up, Down, Confirm, Back, Side, Left, Right }

// ── menu-sfx: the front end's sounds ──────────────────────────────────────────────────────────
// The engine plays every menu sound through `Menu_PlaySound(MENU_SOUND)` 0x1f3278, which maps the
// enum to bank ids and gates four of them on `enabled_sound_flags` 0x30d184 [M 0x1f3278; L the
// boot value 0x0b — sounds 0/1/8 enabled, 2 disabled]:
//   0 → 0x1da (&1)   the ✕ select blip   [M Button_Update 0x1ef948:106, Label_Update 0x218cb8:236,
//                                             Checkbox_Update 0x1efef8:65, Combo_Update 0x1f0498:124,
//                                             Memo_Update 0x21e490:300, Scroll_Update 0x221b20:472]
//   1 → 0x1da (&2)   the □ side action    [M the 0x20-edge arms of the same widgets (msg 0x5d)]
//   2 → 0x1da (&4)   the ○ side action    [M the 0x21-edge arms (msg 0x5e)]
//   3 → 0x1d8        list/spinner up ◀ first step    [M Scroll_Update 0x221b20:204]
//   4 → 0x1d9        list/spinner down ▶ first step  [M Scroll_Update :188]
//   5 → 0x1d8        up, auto-repeat     [M Scroll_Update :228 — every 5th held frame, 2nd past 0x1fa]
//   6 → 0x1d9        down, auto-repeat   [M Scroll_Update :177]
//   7 → 0x1da        the cursor landed on a different control of the SAME page
//                                        [M Page_Update 0x21edc8:38-53]
//   8 → 0x1d9 (&8)   △ back with no 0x5b script on the focus: the buzz + the page-stack pop
//                                        [M MenuManager_Update 0x21c3d8:78-105 → msg 0x5f]
//   9 → silence      Scroll_Update's △ arm calls Menu_PlaySound(9), which falls into the switch
//                    default and starts nothing [M 0x1f3278 default → lVar2 = 0; 0x221b20:457]
// The twelve top-level page handlers play the open pair `SFXStart(0x1d7)` + `SFXStart(0x470)` on
// msg 0x4c, gated by the `menu_movie` static (set 1 after playing, reset by the intro/attract
// movie-end paths) [M P_START 0x20a408:12-22, P_MAIN 0x20a7b0:36-47, P_NFRESULTS 0x208e78:63-65,
// P_INTRO 0x209e88:23-58, and P_ESTHERO/P_ATTRACT/P_FMVPLAYER/P_NFMAP/P_MPDEBRIEFING/P_TRAILER/
// P_WINGAME/P_CREDITS]. The front end (07000048) loads bank hash 0xC or 0x38, flipping
// DAT_002e1b11 on every (re)load [M Sound_Ready 0x1dceb8:91-96]: SB_0.SFX (hash 12) carries
// {0x1d7, 0x1d8, 0x1d9, 0x1da, 0x1db} and SB_1.SFX (hash 56) carries {0x1d8, 0x1d9, 0x1da, 0x1db,
// 0x470} [D the disc's PS2/ENGLISH/SB_0/SB_{0,1}.SFX effect tables] — so exactly one half of the
// open pair sounds per visit, alternating. Mission banks duplicate 0x1d8/0x1d9/0x1da (not
// 0x1d7/0x470) [D SB_4.SFX/SB_5.SFX], which is why the pause page's blips play over a paused
// mission while the open pair cannot. A start whose id no loaded bank holds is a silent no-op
// [M the IOP lookup returns 1, docs/systems/sound.md §2].
/// The spinner/list ◀▲ step (MENU_SOUND 3/5) [D in the front-end and mission banks].
pub const SFX_MENU_VAL_DEC: u32 = 0x1d8;
/// The spinner/list ▶▼ step (MENU_SOUND 4/6) and the △ back buzz (MENU_SOUND 8) [D].
pub const SFX_MENU_VAL_INC: u32 = 0x1d9;
/// The select blip (MENU_SOUND 0/1/2) and the same-page cursor tick (MENU_SOUND 7) [D].
pub const SFX_MENU_TICK: u32 = 0x1da;
/// The page-open pair's bank-0xC half — only SB_0.SFX holds it [D].
pub const SFX_MENU_OPEN_MOVE: u32 = 0x1d7;
/// The page-open pair's bank-0x38 half — only SB_1.SFX holds it [D].
pub const SFX_MENU_OPEN_SELECT: u32 = 0x470;
/// The `menu_movie`-gated open pair's marker in [`MenuSfx::sound`] (not a MENU_SOUND enum value —
/// the pair goes through the handlers' own `SFXStart` calls, not `Menu_PlaySound`).
pub const MENU_SOUND_PAGE_OPEN: u8 = 0xff;
pub const MENU_SOUND_SELECT: u8 = 0;
pub const MENU_SOUND_SIDE_SQUARE: u8 = 1;
pub const MENU_SOUND_SIDE_CIRCLE: u8 = 2;
pub const MENU_SOUND_VAL_DEC: u8 = 3;
pub const MENU_SOUND_VAL_INC: u8 = 4;
pub const MENU_SOUND_FOCUS: u8 = 7;
pub const MENU_SOUND_BACK: u8 = 8;

/// One menu sound the game's front end starts. `sound` is the MENU_SOUND value the game passed to
/// `Menu_PlaySound` [M 0x1f3278] (or [`MENU_SOUND_PAGE_OPEN`] for the handlers' `SFXStart` pair);
/// `id` is the bank SFX id the sound engine receives. The host plays the drained events through
/// nf-audio; tests assert the numbers only and never open a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuSfx {
    /// The MENU_SOUND enum value [M 0x1f3278].
    pub sound: u8,
    /// The bank SFX id [M the mapping above; D the SB_*.SFX tables].
    pub id: u32,
}

/// objective-hints: one OBJECTIVES-tab row — the game's OBJ_STATE as the pause populate
/// (C_GCPAUSE_Handler 0x204408:251-271) uses it: the row text (record+0x00), the ✕ hint box's
/// label (record+0x08; 0xffffffff = none, the view substitutes 0x31d [M 0x205bd0-0x205bdc]) and
/// the ~J (done, state 4) / ~I (pending) marker [M the strings .rodata 0x30d2e0/0x30d2e8].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PauseObjective {
    /// The row's text label (bank 4).
    pub text: u32,
    /// The hint box's label (record+0x08), 0xffffffff = none.
    pub hint: u32,
    /// state 4 → the ~J done marker [M the populate's `if (state == 4)`].
    pub done: bool,
}

/// One cell of the Secret Unlocks letter grid. The character cells carry their char; the three
/// control cells use the ids the game's keyboard sends [M C_KEYBOARD_Handler 0x213a98: id 1000
/// (0x3e8) deletes the last char, id 1001 (0x3e9) appends the space, id 0x3ea submits].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretKey { Char(char), Space, Delete, Submit }

/// The grid's cells, digits then letters then the three control cells (0-9, A-Z, ← → ✓ [D the
/// page's cell set; the model order here is the port's — the drawn layout is the main agent's]).
pub static SECRET_GRID: [SecretKey; 39] = {
    let mut keys: [SecretKey; 39] = [SecretKey::Char('0'); 39];
    let mut i = 0;
    let mut c = b'0';
    while c <= b'9' { keys[i] = SecretKey::Char(c as char); i += 1; c += 1; }
    let mut c = b'A';
    while c <= b'Z' { keys[i] = SecretKey::Char(c as char); i += 1; c += 1; }
    keys[36] = SecretKey::Delete;
    keys[37] = SecretKey::Space;
    keys[38] = SecretKey::Submit;
    keys
};

/// The Secret Unlocks entry model: the grid cursor, the typed string (the game's buffer 0x34e028,
/// capped at 8 chars [M 0x213a98:29-31]), the feedback flag (the 0x100029d option box a valid code
/// opens [M 0x213a98:71-72]; an invalid one gives no feedback [M :68-70]) and the submit request
/// the host drains with [`FrontEnd::secret_submit`] (the action handler owns no Progress — the
/// same split as `results_enter`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecretEntry {
    /// The grid cell the cursor stands on (into [`SECRET_GRID`]).
    pub cursor: usize,
    /// The typed string so far.
    pub typed: String,
    /// The valid-code option box (label 0x100029d) is up.
    pub accepted: bool,
    /// The Submit cell was pressed; the host must run [`FrontEnd::secret_submit`].
    pub pending_submit: bool,
}

/// One option row of the codename option pages: the row control, its label, its y (640 space), the value items
/// (label id — 0 for a literal — the literal, the value), and its kind.
pub struct OptRow { pub ctrl: u32, pub label: u32, pub y: f32, pub items: &'static [(u32, &'static str, i32)], pub kind: OptKind }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OptKind { List, Slider, Action }

const ON_OFF: &[(u32, &str, i32)] = &[(0x1b7, "On", 1), (0x1b6, "Off", 0)];
const TOGGLE_HOLD: &[(u32, &str, i32)] = &[(0x2a2, "Toggle", 1), (0x2a3, "Hold", 0)];
/// Advanced Options [M P_CNOPTIONS_Handler 0x213f08:24-71 — each row's items (label, value) and the profile byte
/// (DAT_002a38d1/c9/d0/cc/cb/d2/d4/d3) it reads back with msg 0x35; L page 0x4000002d's controls for the y].
pub const ADV_ROWS: &[OptRow] = &[
    OptRow { ctrl: 0x1000_011a, label: 0x196, y: 104.0, items: ON_OFF, kind: OptKind::List },        // Vibration
    OptRow { ctrl: 0x1000_011b, label: 0x197, y: 136.0, items: ON_OFF, kind: OptKind::List },        // Auto Aim
    OptRow { ctrl: 0x1000_0124, label: 0x29c, y: 168.0, items: ON_OFF, kind: OptKind::List },        // Crosshairs
    OptRow { ctrl: 0x1000_0125, label: 0x032, y: 200.0, items: TOGGLE_HOLD, kind: OptKind::List },   // Crouch
    OptRow { ctrl: 0x1000_0126, label: 0x02a, y: 232.0, items: TOGGLE_HOLD, kind: OptKind::List },   // Manual Aim
    OptRow { ctrl: 0x1000_01cb, label: 0x399, y: 264.0, items: ON_OFF, kind: OptKind::List },        // Weapon Auto Switch
    OptRow { ctrl: 0x1000_0226, label: 0x5b7, y: 296.0, items: &[(0x1b7, "On", 2), (0x1b6, "Off", 0)], kind: OptKind::List },
    OptRow { ctrl: 0x1000_023b, label: 0x5bd, y: 328.0, items: &[(0x1b7, "On", 0), (0x1b6, "Off", 1)], kind: OptKind::List },
];
/// Multiplayer Options [M P_CNMPOPTIONS_Handler 0x2144d8:13-40 — Radar (MPSettings+0x40), Health Handicap
/// (MPSettings+0x44, the literals at 0x30d390..), Auto Aim (DAT_002a38ca); L page 0x4000002e].
pub const MP_ROWS: &[OptRow] = &[
    OptRow { ctrl: 0x1000_0128, label: 0x1b3, y: 175.0, items: ON_OFF, kind: OptKind::List },
    OptRow { ctrl: 0x1000_011c, label: 0x296, y: 207.0, items: &[(0, "-75", -75), (0, "-50", -50), (0, "-25", -25), (0, "0", 0), (0, "+25", 25), (0, "+50", 50), (0, "+75", 75), (0, "+100", 100)], kind: OptKind::List },
    OptRow { ctrl: 0x1000_0129, label: 0x197, y: 239.0, items: ON_OFF, kind: OptKind::List },
];
/// AV Options [M P_CNAVOPTIONS_Handler 0x214818:37-60, 130-134; L page 0x40000031]: the two volume sliders
/// (0..100, the controls' d3 0x64), Subtitles, Split Screen (0x2ba / 0x2b9), Speaker (0x393 / 0x394 / 0x395 =
/// 0 / 1 / 4), Widescreen, then four action rows.
pub const AV_ROWS: &[OptRow] = &[
    OptRow { ctrl: 0x1000_0135, label: 0x194, y: 86.0, items: &[], kind: OptKind::Slider },
    OptRow { ctrl: 0x1000_0134, label: 0x193, y: 114.0, items: &[], kind: OptKind::Slider },
    OptRow { ctrl: 0x1000_0136, label: 0x2b5, y: 142.0, items: ON_OFF, kind: OptKind::List },
    OptRow { ctrl: 0x1000_019a, label: 0x2b8, y: 170.0, items: &[(0x2ba, "Horizontal", 0), (0x2b9, "Vertical", 1)], kind: OptKind::List },
    OptRow { ctrl: 0x1000_0198, label: 0x195, y: 198.0, items: &[(0x393, "Mono", 0), (0x394, "Stereo", 1), (0x395, "Surround", 4)], kind: OptKind::List },   // 0x395 = "Surround" [D USATxt.dat]
    OptRow { ctrl: 0x1000_0223, label: 0x5b8, y: 227.0, items: ON_OFF, kind: OptKind::List },
    OptRow { ctrl: 0x1000_0137, label: 0x2b6, y: 256.0, items: &[], kind: OptKind::Action },
    OptRow { ctrl: 0x1000_01a3, label: 0x37e, y: 284.0, items: &[], kind: OptKind::Action },
    OptRow { ctrl: 0x1000_0138, label: 0x198, y: 312.0, items: &[], kind: OptKind::Action },
    OptRow { ctrl: 0x1000_0222, label: 0x550, y: 340.0, items: &[], kind: OptKind::Action },
];
/// The option pages' descriptions per (row control, current value) — i32::MIN matches any value, the first matching
/// record wins [L ee_snapshot 0x2e4610..0x2e4c28: 12-byte {control, value, label} records, 0xf0f0f0f0 = any; the main
/// menu rows, the MP rules / player-mods / enviro-mods / bot-setup rows, then the codename option pages'].
const OPT_DESC: &[(u32, i32, u32)] = &[
    (0x1000_0002, i32::MIN, 0x100_0000), (0x1000_0003, i32::MIN, 0x100_0001), (0x1000_0004, i32::MIN, 0x100_0002), (0x1000_0008, 0, 0x100_002f),
    (0x1000_00f6, 0, 0x100_002f), (0x1000_0081, 1, 0x100_0021), (0x1000_0081, 0, 0x100_0022), (0x1000_0084, 0, 0x100_0023),
    (0x1000_0084, 1, 0x100_0024), (0x1000_0084, 2, 0x100_0025), (0x1000_0084, 3, 0x100_0026), (0x1000_0084, 4, 0x100_0027),
    (0x1000_0084, 5, 0x100_0028), (0x1000_0084, 6, 0x100_0200), (0x1000_0084, 7, 0x100_0201), (0x1000_0084, 8, 0x100_0202),
    (0x1000_0084, 9, 0x100_0203), (0x1000_0084, 10, 0x100_0327), (0x1000_0080, 1, 0x100_002a), (0x1000_0080, 0, 0x100_0029),
    (0x1000_008c, 1, 0x100_002c), (0x1000_008c, 0, 0x100_002b), (0x1000_008d, 1, 0x100_002d), (0x1000_008d, 0, 0x100_002e),
    (0x1000_00a2, 0, 0x100_002f), (0x1000_00a2, 1, 0x100_0030), (0x1000_00a2, 2, 0x100_0031), (0x1000_009f, 1, 0x100_0033),
    (0x1000_009f, 0, 0x100_0032), (0x1000_019b, 0, 0x100_0034), (0x1000_019b, 1, 0x100_0035), (0x1000_019b, 16, 0x100_0295),
    (0x1000_0227, 1, 0x100_0037), (0x1000_0227, 0, 0x100_0036), (0x1000_0228, 0, 0x100_01dd), (0x1000_0228, 1, 0x100_0325),
    (0x1000_0228, 2, 0x100_0326), (0x1000_0228, 3, 0x100_01dc), (0x1000_0008, i32::MIN, 0x100_030d), (0x1000_00f6, i32::MIN, 0x100_030e),
    (0x1000_0116, 1, 0x100_0079), (0x1000_0116, 0, 0x100_007a), (0x1000_0115, 1, 0x100_007b), (0x1000_0115, 0, 0x100_007c),
    (0x1000_018c, 0, 0x100_007d), (0x1000_018c, 1, 0x100_007e), (0x1000_018c, 2, 0x100_007f), (0x1000_0195, 0, 0x100_02da),
    (0x1000_0195, 4, 0x100_0083), (0x1000_0195, 1, 0x100_0080), (0x1000_0195, 2, 0x100_0081), (0x1000_0195, 3, 0x100_0082),
    (0x1000_0195, 5, 0x100_0084), (0x1000_0195, 6, 0x100_0085), (0x1000_0195, 7, 0x100_0087), (0x1000_0195, 8, 0x100_0086),
    (0x1000_0185, 2, 0x100_0088), (0x1000_0185, 3, 0x100_0089), (0x1000_0185, 4, 0x100_008a), (0x1000_0186, 8, 0x100_008b),
    (0x1000_0186, 5, 0x100_008c), (0x1000_0186, 3, 0x100_008d), (0x1000_0186, 1, 0x100_008e), (0x1000_0187, 50, 0x100_008f),
    (0x1000_0187, 75, 0x100_0090), (0x1000_0187, 100, 0x100_0091), (0x1000_0187, 125, 0x100_0092), (0x1000_0187, 150, 0x100_0093),
    (0x1000_0187, 175, 0x100_0094), (0x1000_0187, 200, 0x100_0095), (0x1000_0187, 250, 0x100_0328), (0x1000_0187, 300, 0x100_0329),
    (0x1000_0188, 50, 0x100_0096), (0x1000_0188, 75, 0x100_0097), (0x1000_0188, 100, 0x100_0098), (0x1000_0188, 125, 0x100_0099),
    (0x1000_0188, 150, 0x100_009a), (0x1000_0188, 175, 0x100_009b), (0x1000_0188, 200, 0x100_009c), (0x1000_018a, 50, 0x100_009d),
    (0x1000_018a, 75, 0x100_009e), (0x1000_018a, 100, 0x100_009f), (0x1000_018a, 125, 0x100_00a0), (0x1000_018a, 150, 0x100_00a1),
    (0x1000_018a, 175, 0x100_00a2), (0x1000_018a, 200, 0x100_00a3),
    (0x1000_011a, 1, 0x100_030f), (0x1000_011a, 0, 0x100_0310), (0x1000_011b, 1, 0x100_0057), (0x1000_011b, 0, 0x100_0058),
    (0x1000_0124, 1, 0x100_0059), (0x1000_0124, 0, 0x100_005a), (0x1000_0125, 0, 0x100_005b), (0x1000_0125, 1, 0x100_005c),
    (0x1000_0126, 0, 0x100_005d), (0x1000_0126, 1, 0x100_005e), (0x1000_01cb, 1, 0x100_0061), (0x1000_01cb, 0, 0x100_0062),
    (0x1000_0226, 2, 0x100_0302), (0x1000_0226, 0, 0x100_0303), (0x1000_023b, 0, 0x100_0304), (0x1000_023b, 1, 0x100_0305),
    (0x1000_0128, 1, 0x100_005f), (0x1000_0128, 0, 0x100_0060),
    (0x1000_011c, -75, 0x100_006a), (0x1000_011c, -50, 0x100_0069), (0x1000_011c, -25, 0x100_0068), (0x1000_011c, 0, 0x100_0063),
    (0x1000_011c, 25, 0x100_0064), (0x1000_011c, 50, 0x100_0065), (0x1000_011c, 75, 0x100_0066), (0x1000_011c, 100, 0x100_0067),
    (0x1000_0129, 1, 0x100_006b), (0x1000_0129, 0, 0x100_006c),
    (0x1000_0135, i32::MIN, 0x100_0306), (0x1000_0134, i32::MIN, 0x100_0307), (0x1000_0136, 1, 0x100_0071), (0x1000_0136, 0, 0x100_0072),
    (0x1000_0198, 0, 0x100_006d), (0x1000_0198, 1, 0x100_006e), (0x1000_0198, 2, 0x100_030b), (0x1000_0198, 4, 0x100_030b),
    (0x1000_0223, 1, 0x100_0309), (0x1000_0223, 0, 0x100_030a), (0x1000_0137, i32::MIN, 0x100_030c), (0x1000_019a, 0, 0x100_006f),
    (0x1000_019a, 1, 0x100_0070), (0x1000_01a3, i32::MIN, 0x100_0073), (0x1000_0138, i32::MIN, 0x100_0074), (0x1000_0222, i32::MIN, 0x100_0308),
];
/// The description label of `ctrl` at `value` [L table 0x2e4a18].
pub fn opt_desc(ctrl: u32, value: i32) -> Option<u32> {
    OPT_DESC.iter().find(|r| r.0 == ctrl && (r.1 == value || r.1 == i32::MIN)).map(|r| r.2)
}
/// The default option values — the PCSX2 Default profile's (2026-10-15 captures m32/m38/m40) [L].
pub fn default_opts() -> std::collections::HashMap<u32, i32> {
    [(0x1000_011a, 1), (0x1000_011b, 1), (0x1000_0124, 1), (0x1000_0125, 1), (0x1000_0126, 0), (0x1000_01cb, 1), (0x1000_0226, 2),
     (0x1000_023b, 0), (0x1000_0128, 1), (0x1000_011c, 0), (0x1000_0129, 0), (0x1000_0135, 80), (0x1000_0134, 100), (0x1000_0136, 0),
     (0x1000_019a, 0), (0x1000_0198, 1), (0x1000_0223, 0)].into_iter().collect()
}
/// The "Edit <name>" wheel's rows {preview, label, description} [L ee_snapshot table 0x2dfc28].
pub const CN_MENU: [(u32, u32, u32); 7] = [
    (0x0300_013b, 0x100_02d4, 0x100_02d7), (0x0300_0121, 0x1f3, 0x100_0051), (0x0300_0121, 0x27e, 0x100_0052),
    (0x0300_0124, 0x37a, 0x100_0053), (0x0300_0123, 0x199, 0x100_0054), (0x0300_011f, 0x2bb, 0x100_0055), (0x0300_0125, 0x1f4, 0x100_0056),
];

// ── mp-menu-flow: the multiplayer wheels' own tables ──────────────────────────────────────────
// The game's wheel tables share the campaign's 0x18-byte record {+0x00 preview texture, +0x04 name
// label, +0x08 description label, +0x0C id, +0x10 unlocked flag, +0x14 locked-description} [M
// Menu_UpdateWheel 0x1f8388:94-234]. All four MP tables were dumped from the disc's ACTION.ELF
// .data (vaddr = file offset + 0x106100, the single PT_LOAD) 2026-10-15 and pinned by
// mp_menu_lists_live.rs against the ISO.
/// `mp_scenario` @0x2df508 (file 0x1d9408), 13 × 0x18 — the Select Scenario wheel's rows [M
/// C_SBMPSCEN 0x20c600:33-36 walks it; symtab name/size pinned]. `+0x0C` is the scenario word
/// MPSettings+0x1A4 gets (0 = Quick Game, the special row — see `FrontEnd::mp_quick_start`);
/// +0x10 is the SHIPPED unlock byte — `Menu_UnlockMPSettings` 0x1ff698:53-104 zeroes rows
/// 4/6/7/9/10/12 and re-derives them from the campaign rewards (0x38 Assassination, 0x39 Uplink,
/// 0x3a Team KOTH, 0x3b Demolition, 0x3c Protection, 0x3d GoldenEye Strike), so the port carries
/// those as the reward ids in the last field (0 = always open). Locked rows show +0x14
/// (0x010000ab "This scenario is locked.") and refuse Confirm [M C_SBMPSCEN:40-42].
pub const MP_SCENARIO_ROWS: [(u32, u32, u32, u32, u32); 13] = [
    // (preview, name, desc, scenario word, reward that unlocks; the shipped +0x10 bytes match)
    (0x0300_013a, 0x100_02dd, 0x100_02de, 0x0000_0000, 0), // Quick Game
    (0x0300_0099, 0x1cb, 0x1cc, 0x0000_0001, 0),          // Arena (Deathmatch)
    (0x0300_009a, 0x1cd, 0x1ce, 0x2000_0002, 0),          // Team Arena
    (0x0300_009b, 0x1cf, 0x1d0, 0x2000_0004, 0),          // Capture The Flag
    (0x0300_00a4, 0x1d1, 0x1d2, 0x6000_0008, 0x39),       // Uplink
    (0x0300_009c, 0x1d3, 0x1d4, 0x0000_0010, 0),          // Top Agent (the death-limit FFA)
    (0x0300_009d, 0x1d7, 0x1d8, 0x2000_0040, 0x3b),       // Demolition
    (0x0300_009e, 0x1d9, 0x1da, 0x2000_0080, 0x3c),       // Protection
    (0x0300_009f, 0x1db, 0x1dc, 0x2000_0100, 0),          // Industrial Espionage (BluePrint)
    (0x0300_00a0, 0x1dd, 0x1de, 0x2000_0200, 0x3d),       // GoldenEye Strike
    (0x0300_00a1, 0x1df, 0x1e0, 0x0000_0400, 0x38),       // Assassination
    (0x0300_00a2, 0x1e1, 0x1e2, 0x4000_0800, 0),          // King of the Hill
    (0x0300_00a3, 0x1e3, 0x1e4, 0x6000_1000, 0x3a),       // Team King of the Hill
];
/// The locked row's description [L mp_scenario +0x14 = 0x010000ab on every row; USATxt "This
/// scenario is locked."].
pub const MP_LOCKED_SCENARIO_DESC: u32 = 0x0100_00ab;
/// `mp_level` @0x2df400 (file 0x1d9300), 8 × 0x18 — the Select Map wheel [M C_SBMPMAP 0x217538:34-36;
/// symtab]. +0x0C is the arena's level code (MPSettings+0x1A8 gets it on Confirm [M disasm
/// 0x217654/0x21765c]); every shipped +0x10 flag is 1 and every +0x14 locked-desc is 0 — no map is
/// ever locked on retail [L the .data dump], the lock check is a no-op [M C_SBMPMAP:12-14].
/// The codes are exactly the on-foot ISO's arenas 0x07000023..0x29 + 0x0700004b (Ravine) [D
/// docs/atlas/multiplayer.md §1; 0x21/0x22/0x4c are absent from the table].
pub const MP_MAP_ROWS: [(u32, u32, u32, u32); 8] = [
    // (preview, name, desc, level code)
    (0x0300_0086, 0x0300_0024, 0x0300_0026, 0x0700_0024), // Skyrail
    (0x0300_008b, 0x0300_0025, 0x0300_0027, 0x0700_0027), // Fort Knox
    (0x0300_00ba, 0x0300_0028, 0x0300_0029, 0x0700_0029), // Snow Blind
    (0x0300_00b9, 0x0300_002a, 0x0300_002b, 0x0700_0026), // Phoenix Base
    (0x0300_00b8, 0x0300_002c, 0x0300_002d, 0x0700_0023), // Atlantis
    (0x0300_00b7, 0x0300_002e, 0x0300_002f, 0x0700_0028), // Missile Silo
    (0x0300_00b6, 0x0300_0030, 0x0300_0031, 0x0700_0025), // Sub Pen
    (0x0300_00b5, 0x0300_0032, 0x0300_0033, 0x0700_004b), // Ravine
];
/// `mp_options` @0x2dfbb0 (file 0x1d9ab0), 5 × 0x18 — the MP options hub's rows [M C_SBMPOPTIONS
/// 0x20e818:41-43; symtab]. +0x0C = the row index the confirm switches on. Row 1 (AI Bots)'s +0x10
/// byte is cleared at page open when the map is Ravine [M P_MPOPTIONS 0x217678:20-25 —
/// `MPSettings._424_4_ == 0x700004b → mp_options[0x28] = 0`], with locked-desc 0x010000ae "You
/// can't have bots play the Ravine map." [L].
pub const MP_OPTIONS_ROWS: [(u32, u32, u32, u32); 5] = [
    // (preview, name, desc, id)
    (0x0300_013a, 0x171, 0x0100_001c, 0), // Continue
    (0x0300_0136, 0x1ad, 0x0100_0020, 1), // AI Bots
    (0x0300_0139, 0x19b, 0x0100_001d, 2), // Game Rules
    (0x0300_0138, 0x19d, 0x0100_001e, 3), // Player Mods
    (0x0300_0137, 0x19e, 0x0100_001f, 4), // Enviro-Mods
];
/// `mp_bots` @0x2dfd30 (file 0x1d9c30), 5 × 0x18 — the AI Bots list's rows [M C_SBBOTS 0x210a08:61-62;
/// symtab; the .data names/descs; C_SBBOTS:40-59 rebuilds rows 1-4 at runtime with the "%s : %s"
/// Playing/Yes-No status (0x295/0x181/0x182) and the character's preview]. +0x0C is 0 (the rows are
/// addressed by index; Confirm row 0 = Continue/back, rows 1-4 pick the bot slot).
pub const MP_BOTS_ROWS: [(u32, u32, u32); 5] = [
    (0x0300_013a, 0x171, 0), // Continue
    (0x0300_0099, 0x284, 0), // Setup Bot 1
    (0x0300_0099, 0x285, 0), // Setup Bot 2
    (0x0300_0099, 0x286, 0), // Setup Bot 3
    (0x0300_0099, 0x287, 0), // Setup Bot 4
];
/// `mp_characters` @0x2df640 (file 0x1d9540), 29 × 0x18 — the bot/human character table [L the
/// .data dump 2026-10-15; symtab]. +0x0C == the row index == the `default_bot_stats` preset index
/// [M Menu_UnlockMPSkins 0x1ff2f8:113-187's reward map and P_MPBOTCHOOSE's selection]. The reward
/// column mirrors the shipped +0x10 bytes: rows 0-11 always open, 12-28 need the campaign reward
/// id shown (UnlockMPSkins's map; the .data flags are 0 for all of them). Locked rows show
/// 0x010000ad "This character is locked." [L every row's +0x14].
pub const MP_CHARACTERS: [(u32, u32, u32, u32); 29] = [
    // (preview, name, desc, reward; 0 = always unlocked)
    (0x0300_010a, 0x21a, 0x0100_0038, 0),      // Bond
    (0x0300_010e, 0x21b, 0x0100_0039, 0),      // Drake
    (0x0300_0119, 0x21c, 0x0100_003a, 0),      // Rook
    (0x0300_0112, 0x21d, 0x0100_003b, 0),      // Kiko
    (0x0300_0107, 0x21e, 0x0100_003c, 0),      // Alura
    (0x0300_010d, 0x21f, 0x0100_003d, 0),      // Dominique
    (0x0300_011b, 0x220, 0x0100_003e, 0),      // Snow Guard
    (0x0300_0109, 0x221, 0x0100_003f, 0),      // Black Ops
    (0x0300_011e, 0x222, 0x0100_0040, 0),      // Yakuza
    (0x0300_010c, 0x223, 0x0100_0041, 0),      // Phoenix Commando
    (0x0300_016c, 0x224, 0x0100_0042, 0),      // Phoenix Soldier
    (0x0300_0115, 0x225, 0x0100_0043, 0),      // Ninja
    (0x0300_0147, 0x39f, 0x0100_01f2, 0x27),   // Bond (tuxedo)
    (0x0300_0148, 0x3a0, 0x0100_01f3, 0x36),   // Drake (suit)
    (0x0300_0194, 0x100_01ee, 0x0100_01f0, 0x37), // Bond (space suit)
    (0x0300_0110, 0x226, 0x0100_0044, 0x2f),   // Goldfinger
    (0x0300_0118, 0x227, 0x0100_004e, 0x34),   // Renard
    (0x0300_011a, 0x228, 0x0100_0046, 0x28),   // Scaramanga
    (0x0300_0117, 0x229, 0x0100_0047, 0x33),   // Pussy Galore
    (0x0300_010b, 0x22a, 0x0100_004c, 0x32),   // Christmas Jones
    (0x0300_011c, 0x22b, 0x0100_01f4, 0x2d),   // Wai Lin
    (0x0300_011d, 0x22c, 0x0100_0049, 0x2e),   // Xenia Onatopp
    (0x0300_0113, 0x22d, 0x0100_004a, 0x31),   // May Day
    (0x0300_010f, 0x22e, 0x0100_004b, 0x30),   // Elektra King
    (0x0300_0111, 0x22f, 0x0100_0048, 0x2b),   // Jaws
    (0x0300_0108, 0x230, 0x0100_004d, 0x2a),   // Baron Samedi
    (0x0300_0116, 0x231, 0x0100_0045, 0x2c),   // Oddjob
    (0x0300_0114, 0x2ff, 0x0100_01f5, 0x29),   // Nick Nack
    (0x0300_0196, 0x100_01ef, 0x0100_01f1, 0x35), // Max Zorin
];
/// Each character's side, from `default_bot_stats` @0x26D2F0 (file 0x1671f0) byte +0x09 [M
/// `Menu_IsBotGood` 0x2027a8 = `BOT_getDefaultStats(id)[9] == 0`; the bot record's +0x0F team gets
/// 1 (MI6) for those and 0 (Phoenix) otherwise — C_SBMPBTCHOOSE 0x210f08:26-33]. Presets 15-28
/// (the reward characters) also have stats+0x0D = 0 — the Bot Setup page locks their stat rows
/// [M P_MPBOTSETUP 0x211270:23-48 `character >= 0x0F`].
pub const MP_CHARACTER_SIDE_MI6: [bool; 29] = [
    true, false, false, false, true, true, false, false, false, false, false, false,  // 0-11
    true, false, true, false, false, false, true, true, true, false, false, false,     // 12-23
    false, false, false, false, false,                                                 // 24-28
];
/// The weapon-set names, indexed by MPSettings+0x1B4 0..10 [M P_MPPLAYERMODS 0x2104d0:26-49's
/// label switch — the same list P_MPCONFIRM 0x20ec00's +0x1B4 switch binds].
pub const MP_WEAPON_SETS: [u32; 11] = [0x1a0, 0x1a2, 0x1a4, 0x1a6, 0x1a8, 0x24f, 0x3db, 0x3dc, 0x3dd, 0x3de, 0x1b2];
/// The P_MPSETUP per-agent prompt labels [M P_MPSETUP 0x20d7a8: state 2 (team) binds 0x37c, state
/// 3 (character) binds 0x37d; USATxt "Choose Team"/"Choose Character"].
pub const MP_SETUP_TEAM_PROMPT: u32 = 0x37c;
pub const MP_SETUP_CHAR_PROMPT: u32 = 0x37d;
/// The Bot Setup page's row titles in its own y order [L the menu-def's msg-0x18 b fields on page
/// 0x4000002c — see the row_label comment].
pub const MP_BOT_SETUP_LABELS: [u32; 8] = [0x295, 0x247, 0x232, 0x246, 0x15e, 0x1c6, 0x23a, 0x249];
/// The pages' headers [L the menu-def's header control (50,40,376×17) msg-0x18 b field per page;
/// the confirm 0x40000049 and debriefing 0x40000033 pages bind no static header].
pub const MP_JOIN_HEADER: u32 = 0x29e;
pub const MP_SCENARIO_HEADER: u32 = 0x2c3;
pub const MP_MAP_HEADER: u32 = 0x158;
pub const MP_SETUP_HEADER: u32 = 0x29e;
pub const MP_OPTIONS_HEADER: u32 = 0x2c4;
pub const MP_BOTS_HEADER: u32 = 0x294;
pub const MP_RULES_HEADER: u32 = 0x19b;
pub const MP_PLAYERMODS_HEADER: u32 = 0x19d;
pub const MP_ENVIROMODS_HEADER: u32 = 0x19e;
pub const MP_BOTSETUP_HEADER: u32 = 0x294;

/// One Join-Game panel's state [M mpjoin 0x2dee68 entry (4 × 0x10: +0x00 joined, +0x01 ready,
/// +0x04 team, +0x08 skin) and mp_stuff+0x360+slot·4 — the game's state values, kept verbatim:
/// 0 = "Press ~A to join" (0x1e8), 1 = "Choose Codename" (0x37b), 5 = joined (the codename
/// chosen; C_RBMPSTART then binds 0x39c "Player Ready")]. `codename` is the name the page puts
/// in MPSettings+slot·0x30 [M C_RBMPCNAME 0x20d448: the template default is "Player N" — Txt
/// 0x1c3 + sprintf "%s %d" @0x30d388 with slot+1 — and a saved row's codename otherwise].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MpJoinAgent {
    /// mp_stuff+0x360+slot·4: 0 / 1 / 5 (then P_MPSETUP's 2/3/4 continue the same cells).
    pub state: u32,
    /// mpjoin+slot·0x10 +0x00.
    pub joined: bool,
    /// mpjoin+slot·0x10 +0x01 — the "ready" flag [M C_RBMPSTART 0x20d048's state-5 arm].
    pub ready: bool,
    /// mpjoin+slot·0x10 +0x04 (0 Phoenix / 1 MI6; the game fills it on the setup page).
    pub team: u32,
    /// mpjoin+slot·0x10 +0x08 — the chosen character (mp_characters_small id).
    pub skin: u32,
    /// The codename row the ◀ ▶ list is on (the game's list control 0x1000022b's selection).
    pub codename_row: usize,
    /// The chosen codename (the MPSettings+slot·0x30 name).
    pub codename: String,
}
impl Default for MpJoinAgent {
    fn default() -> Self { MpJoinAgent { state: 0, joined: false, ready: false, team: 0, skin: 0, codename_row: 0, codename: String::new() } }
}

/// One bot's 0x12-byte config record — `mpbots` 0x2deea8's records at +2+slot·0x12 (= 0x2DEEAA in
/// the game) [M the record layout from mp-bots-data: +0x0E enabled, +0x0F team, +0x10 character,
/// +0x11 dirty; the stats the Bot Setup page edits are the record's +0x00..0xC view of
/// `default_bot_stats` 0x26D2F0]. Fresh records carry the game's msg-0x51 defaults (characters
/// 6/7/8/9 for slots 0-3 [M C_SBMPOPTIONS 0x20e818's init]) and enabled = 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MpBot {
    /// record+0x00: aggression ("Skill" — Poor→8 / Average→5 / Good→3 / Very Good→1 [M
    /// P_MPBOTSETUP 0x211270:49-59]).
    pub skill: i32,
    /// record+0x02 (u16): aim (Normal→2 / High→3 / Very High→4 [M :60-68]).
    pub aim: i32,
    /// record+0x04 (u16): hit points 50..300 [M :69-80].
    pub hit_points: i32,
    /// record+0x06: speed (Slow→0 / Normal→1 / Fast→2 [M :81-89]).
    pub speed: i32,
    /// record+0x07: recovery 1 (50..200) [M :117-126].
    pub recovery1: i32,
    /// record+0x08: recovery 2 (50..200) [M :127-135].
    pub recovery2: i32,
    /// record+0x0B: the trait (None→0; MI6: Collector 1/Guardian 2/Team Player 3/Judge 4; Phoenix:
    /// Berserker 5/Greedy 6/Vengeful 7/Assassin 8 [M :90-116]).
    pub trait_id: i32,
    /// record+0x0E: enabled.
    pub enabled: bool,
    /// record+0x0F: the team — `Menu_IsBotGood(char)` (the preset's +0x09 side byte == 0) [M
    /// C_SBMPBTCHOOSE 0x210f08:26-33].
    pub team_mi6: bool,
    /// record+0x10: the character (the mp_characters row id = the default_bot_stats preset).
    pub character: u32,
    /// record+0x11: dirty — the stats were hand-edited [M C_SBMPBTCHOOSE's nav resets them from
    /// the preset only when !dirty].
    pub dirty: bool,
}
impl MpBot {
    /// A fresh record: the game's defaults for `slot` — character 6/7/8/9, enabled = 0, stats from
    /// the character's preset [M the msg-0x51 init + BOT_getDefaultStats].
    pub fn fresh(slot: usize) -> Self {
        let character = 6 + slot as u32;   // [M DAT_002deeba=6, 0x2deecc=7, 0x2deede=8, 0x2deef0=9]
        let (skill, aim, hp, speed, r1, r2, trait_id) = Self::preset(character as usize);
        MpBot { skill, aim, hit_points: hp, speed, recovery1: r1, recovery2: r2, trait_id, enabled: false, team_mi6: !MP_CHARACTER_SIDE_MI6[character as usize], character, dirty: false }
    }
    /// `default_bot_stats` @0x26D2F0's stats for a preset — (aggr, aim, HP, speed, rec1, rec2,
    /// trait) [L the table dump 2026-10-15, verified by mp_menu_lists_live.rs; weapon class +0x0A
    /// is preset-only — no menu edits it (P_MPBOTSETUP has no row for it)]. The port needs the
    /// presets the SETUP page can reset to; only the side (+0x09, in MP_CHARACTER_SIDE_MI6) and
    /// these seven come through the menu.
    pub fn preset(character: usize) -> (i32, i32, i32, i32, i32, i32, i32) {
        //        skill aim hp  spd r1  r2  trait
        const T: [(i32, i32, i32, i32, i32, i32, i32); 29] = [
            (1, 4, 150, 2, 150, 50, 0),   // Bond
            (5, 2, 100, 1, 100, 100, 6),  // Drake
            (3, 3, 150, 1, 100, 50, 7),   // Rook
            (3, 3, 100, 1, 150, 100, 0),  // Kiko
            (3, 3, 100, 1, 125, 100, 2),  // Alura
            (5, 2, 100, 1, 100, 100, 3),  // Dominique
            (8, 2, 100, 1, 100, 100, 0),  // Snow Guard
            (3, 3, 125, 1, 125, 75, 0),   // Black Ops
            (5, 2, 100, 1, 100, 100, 5),  // Yakuza
            (3, 3, 150, 1, 125, 75, 0),   // Phoenix Commando
            (3, 2, 100, 1, 100, 100, 0),  // Phoenix Soldier
            (3, 3, 200, 2, 150, 50, 8),   // Ninja
            (1, 4, 150, 2, 150, 50, 0),   // Bond Tux
            (5, 2, 100, 1, 100, 100, 6),  // Drake Suit
            (1, 4, 150, 2, 150, 50, 0),   // Bond Spacesuit
            (5, 3, 100, 1, 100, 100, 6),  // Goldfinger
            (3, 4, 250, 1, 100, 50, 7),   // Renard
            (1, 4, 150, 1, 150, 100, 8),  // Scaramanga
            (5, 2, 100, 1, 100, 100, 1),  // Pussy Galore
            (5, 2, 100, 1, 100, 100, 3),  // Christmas Jones
            (3, 3, 200, 2, 200, 50, 4),   // Wai Lin
            (3, 4, 200, 1, 125, 75, 0),   // Xenia Onatopp
            (3, 3, 150, 2, 200, 50, 8),   // May Day
            (5, 2, 100, 1, 100, 100, 0),  // Elektra King
            (8, 3, 300, 0, 50, 200, 0),   // Jaws
            (5, 2, 150, 2, 150, 100, 0),  // Baron Samedi
            (3, 3, 150, 1, 100, 150, 0),  // Oddjob
            (3, 3, 100, 2, 125, 75, 0),   // Nick Nack
            (3, 4, 150, 1, 125, 75, 6),   // Max Zorin
        ];
        T[character.min(T.len() - 1)]
    }
    /// The presets' stats-editable flag (+0x0D = 1 for characters 0-14) [M P_MPBOTSETUP
    /// 0x211270:23-48 locks the stat rows for character >= 0x0F].
    pub fn stats_editable(character: u32) -> bool { character < 15 }
}


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
    /// objective-hints: one OBJECTIVES row — the game's OBJ_STATE as the populate uses it: the
    /// row text label (record+0x00), the ✕ hint box's label (record+0x08, 0xffffffff = none —
    /// the VIEW applies the 0x31d fallback [M C_GCPAUSE_Handler 0x204408 0x205bd0-0x205bdc]), and
    /// the ~J/~I marker (state 4 = done [M 0x204408:251-271; the strings .rodata 0x30d2e0/0x30d2e8]).
    pub pause_objectives: Vec<PauseObjective>,
    /// objective-hints: control 8's selected row — starts at the LAST row on every tab entry
    /// (the populate's msg 0x1d = numVis−1 [M 0x204408 0x204840-0x204850]) and moves with ~V.
    pub pause_objectives_sel: usize,
    /// objective-hints: the ✕ hint box — Some(label) while the modal ok-only option box is up
    /// (label 0x31d already applied) [M 0x204408 case 1 → Menu_CreateOptionBox 0x202218 with
    /// box_id 10, ok_only 1; the box's OK row / a cancel pop it [M C_LBMSGOPTIONS 0x2022a0] and
    /// the page handler re-shows the list on msg 0x50 [M P_PAUSE_Handler 0x203c28:19-24 —
    /// `option_return != 0` → control 8 0x2b(0)] — the port just clears the field].
    pub pause_hint: Option<u32>,
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
    /// The codename option values by row control (crate::menu::ADV_ROWS / MP_ROWS / AV_ROWS) [M the profile bytes
    /// the option handlers write — P_CNOPTIONS 0x213f08, P_CNMPOPTIONS 0x2144d8, P_CNAVOPTIONS 0x214818].
    pub opts: std::collections::HashMap<u32, i32>,
    /// The values on entering an option page: Back restores them, Confirm keeps the edits (the hint's ~A Confirm /
    /// ~B Back) [S].
    pub opts_saved: Option<std::collections::HashMap<u32, i32>>,
    /// The codename the "Edit <name>" wheel edits ("Default" for the default profile) [L PCSX2 m30 "Edit Default"].
    pub cn_name: String,
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
    // codename-entry (each field on its own line [WORKLOG lesson 4]).
    /// The name-entry page's typed buffer — the game's `buffer` at 0x34e028, shown by the draw
    /// code through `cn_typed_name` [M C_KEYBOARD_Handler 0x213a98 appends/erases it].
    pub cn_typed: String,
    /// The focused grid cell id — the game opens on the ✓ 1002 [M P_CNNAME_Handler 0x213790:47].
    pub cn_cell: u32,
    /// The page the name entry opened FROM — the 0x4c init's param_6 [M Manager_SendMessage
    /// 0x21c858:344-345]; 0x4000001d (the wheel) is the cheat-codename entry [M 0x213790:33].
    pub cn_from: u32,
    /// `del_when_keypressed` — the first press after open (or after a refused name) clears the
    /// proposal first, and the SPACE key's press is swallowed [M C_KEYBOARD 0x213a98:18-27].
    pub cn_clear_on_key: bool,
    /// The option box awaiting dismissal as `(box id, label id)`: (6, 0x2c0) reserved / (6, 0x3bc)
    /// empty are the validator's refusals [M C_KEYBOARD 0x213a98:47-51], (4, 0) the overwrite
    /// question the card flow asks on P_CNNAME's tick [M P_CNNAME 0x213790:62-76].
    pub cn_box: Option<(u32, u32)>,
    /// The Edit Codename list row awaiting its delete confirm — the game's ~X asks through the
    /// card flow, then mode 3 runs psiDeleteData [M C_SBCNSELECT 0x212f58 case 0x5e].
    pub cn_delete_row: Option<usize>,
    /// The reserved codenames the validator strstr's against — the def_codename labels 0x29b /
    /// 0x211 resolved through the language bank [M Menu_IsValidCodename 0x1f9930:63-76]. The host
    /// fills them (headless they are empty, so nothing is reserved).
    pub cn_reserved: Vec<String>,
    /// The page id the game changes to after an accepted name — DAT_002e1ad8 = 0x4000001d [M
    /// C_KEYBOARD 0x213a98:58]; the wheel's own init then forwards to Controls 0x40000022 [M
    /// P_CNMENU 0x213288:38-40, not ported].
    pub cn_exit: Option<u32>,
    /// The cheat entry's ✓ ran the special-codename check — Menu_SpecialCodenameCheck 0x200238
    /// (52 names at 0x308578, strcmp, unlock masks into sp_level+0x10) is not ported (reported);
    /// the port records that the check ran and creates nothing [M C_KEYBOARD 0x213a98:67-73].
    pub cn_cheat_checked: bool,
    /// The page's PROFILE-name mode — cn_open(from != 0x4000001d) runs the validator/create
    /// flow; the wheel's entry is the cheat grid (SecretEntry) [M 0x213790:33].
    pub cn_profile_mode: bool,
    /// secret-codes: the Secret Unlocks letter-grid model (page 0x40000020 in cheat mode). The
    /// grid cursor + typed string + the valid-code box flag [M C_KEYBOARD_Handler 0x213a98].
    pub secret: SecretEntry,
    /// The picture offset — `PS2ScreenXPos` 0x30c75c / `PS2ScreenYPos` 0x30c760 [M symtab], edited
    /// by the Screen Adjust page and saved in the global settings blob [M LS_MakeGlobalSettings
    /// 0x1cd9d0; docs/systems/screen-adjust.md §3/§5]. (Each field on its own line [WORKLOG lesson 4].)
    pub screen_x: i32,
    pub screen_y: i32,
    /// The Screen Adjust page's enter snapshot — `PS2OldX`/`PS2OldY` 0x30d578/0x30d57c, taken by
    /// msg 0x4c and restored by the msg 0x6b close [M P_SCREENADJUST 0x217a58].
    pub screen_saved_x: i32,
    pub screen_saved_y: i32,
    /// The AV page's own snapshot — `screen_adjust_x/y` 0x30d8e8/0x30d8ec [M P_CNAVOPTIONS
    /// 0x214818:29/135], restored by the page's Back (msg 0x4d) [M :79-85].
    pub screen_av_saved_x: i32,
    pub screen_av_saved_y: i32,
    // ── mp-menu-flow: the multiplayer front end (each new field on its own line [WORKLOG lesson 4]) ──
    /// The four Join-Game panels [M mpjoin 0x2dee68 (4 × 0x10: +0x00 joined, +0x01 ready,
    /// +0x04 team, +0x08 skin) and mp_stuff+0x360+slot·4 (the per-slot state: 0 = press ✕ to
    /// join, 1 = choose codename, 5 = joined) — the port keeps the game's own values]. Agents
    /// 2-4 stay unjoined: the game joins them through their own pads (msg 0x6a) [M P_MPJOIN
    /// 0x20ca00], and the port has no pads (keyboard only, agent 1 — the task's scope).
    pub mp_join: [MpJoinAgent; 4],
    /// MPSettings+0x1A4 — the selected scenario word (0 = none yet; Quick Game keeps 0 until its
    /// own quick-start writes Deathmatch) [M C_SBMPSCEN's confirm store].
    pub mp_scenario: u32,
    /// MPSettings+0x1A8 — the selected arena's level code [M C_SBMPMAP's confirm store].
    pub mp_level: u32,
    /// The scenario wheel's cursor [M C_SBMPSCEN 0x20c600's msg-0x40 selection over mp_scenario].
    pub mp_scenario_row: usize,
    /// The map wheel's cursor [M C_SBMPMAP 0x217538's selection over mp_level].
    pub mp_map_row: usize,
    /// The options hub's cursor [M C_SBMPOPTIONS 0x20e818's selection over mp_options].
    pub mp_options_row: usize,
    /// The AI Bots list's cursor [M C_SBBOTS 0x210a08's selection over mp_bots; opens on
    /// mp_stuff[0x37a]+1].
    pub mp_bots_row: usize,
    /// mp_stuff[0x37a] (byte @0x2df2da) — the bot slot (0..3) the Bots pages configure [M
    /// C_SBBOTS 0x210a08:20 `mp_stuff[0x37a] = sel-1`].
    pub mp_bot_slot: usize,
    /// The bot-choose wheel's cursor — the slot's current character on open [M P_MPBOTCHOOSE
    /// 0x210cd8: the selection = record+0x10].
    pub mp_bot_char: usize,
    /// The four bots' 0x12-byte config records [M mpbots 0x2deea8: [+1] = the count, the records
    /// at +2+slot·0x12 (0x2DEEAA in the game); the fields' game offsets are in MpBot]. Fresh
    /// records have the game's default characters 6/7/8/9 [M C_SBMPOPTIONS 0x20e818's msg-0x51
    /// init writes DAT_002deeba=6, 0x2deecc=7, 0x2deede=8, 0x2deef0=9] and enabled = 0.
    pub mp_bots: [MpBot; 4],
    /// P_MPSETUP's per-agent state for agent 1: 2 = Choose Team (team games), 3 = Choose
    /// Character (FFA), 4 = chosen [M P_MPSETUP 0x20d7a8 sets mp_stuff+0x360 to 2/3 from the
    /// scenario's 0x20000000 bit; C_RBMPSETUP 0x20de58:4b steps 2 → 3 and 3 → 4].
    pub mp_setup_state: u32,
    /// The setup page's agent-1 list selection: the team (0 Phoenix / 1 MI6) or the character id
    /// [M C_RBMPSETUP 0x20de58's msg-0x35 reads; the team list's items are Phoenix(0x1c7)/MI6(0x1c8)].
    pub mp_setup_sel: usize,
    /// MPSettings+0x1A0 — the time limit in minutes, −1 = Unlimited [M P_MPRULES 0x20f790:12; the
    /// game's confirm multiplies by 60 before handing the match over].
    pub mp_time_limit: i32,
    /// MPSettings+0x19C — the frag/points/lives limit [M P_MPRULES 0x20f790:13].
    pub mp_frag_limit: i32,
    /// MPSettings+0x198 — friendly fire [M P_MPPLAYERMODS 0x2104d0:12].
    pub mp_friendly_fire: u32,
    /// MPSettings+0x1B4 — the weapon set 0..10 [M P_MPPLAYERMODS 0x2104d0:13,26-49].
    pub mp_weapon_set: u32,
    /// MPSettings+0x1C4 — radar [M P_MPPLAYERMODS 0x2104d0:16].
    pub mp_radar: u32,
    /// The Player-Mods toggles beyond ff/weapon set/radar, in page order (+0x1BC, +0x1C8) [M
    /// P_MPPLAYERMODS 0x2104d0:14-15].
    pub mp_player_mods: [u32; 2],
    /// The Enviro-Mods rows in page order (+0x1C0 view distance, +0x1B8, +0x1D4, +0x1D0, +0x1CC)
    /// [M P_MPENVIROMODS 0x210030:12-16, 20-27, 35-61].
    pub mp_envmods: [u32; 5],
    /// The start checks' error-box label id, 0 = none [M C_SBMPOPTIONS 0x20e818:84-114 — 0x1000311
    /// (FFA with more than one "good" side), 0x389 (an empty team), 0x39e (one player total)].
    pub mp_error: u32,
    /// The match config the confirm page hands the host — the port's `MpSettings` mirror of
    /// P_MPCONFIRM 0x20ec00:53-94's writes. None until the confirm's Confirm runs.
    pub mp_match_settings: Option<nf_game::mp::MpSettings>,
    /// credits-data: the credits roll's state (crate::credits::CreditsRoll — the handler's own
    /// statics frame_518/name_520/index_519/fading_517/page_from_523/music_vol_524) [M
    /// P_CREDITS_Handler 0x2152b0]. The main agent draws `self.credits.visible()`.
    pub credits: crate::credits::CreditsRoll,
    /// credits-data: the page the credits were opened from, as a Page when it is a ported one
    /// (AV Options; the win page 0x40000053 is not ported and stays None) [M page_from_523].
    pub credits_from: Option<Page>,

    // menu-sfx (each new field on its own line [WORKLOG lesson 4]).
    /// The sounds this frame's input/page changes started — the host drains (`drain_sfx`) and
    /// plays them through nf-audio; the FrontEnd never touches the audio stack itself (tests
    /// stay silent). docs/systems/menu-sfx.md §3.
    pub sfx_out: Vec<MenuSfx>,
    /// `enabled_sound_flags` 0x30d184 — gates MENU_SOUND 0/1/2/8 [M 0x1f3278's case gates]; the
    /// boot value 0x0b leaves sound 2 (○) disabled [L ee_start @0x30d184 = 0x0b].
    pub enabled_sound_flags: u8,
    /// The handlers' `menu_movie` static: set once the page-open pair has played, cleared by the
    /// intro/attract movie ends [M P_START 0x20a408:22; P_INTRO 0x209e88:44,58].
    pub menu_movie: bool,
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
        let mut fe = FrontEnd { flow, page, cursor: 0, mission: 0, difficulty: 0, no_input_frames: 0, press_start_on: true, outcome: Outcome::Failed, stats: Stats::default(), requested_level: None, confirming_quit: false, confirming_default_profile: false, unlock_all: false, unlocked, closed: false, intro_playing: false, intro_index: 0,
            backdrop_live: false,
            // fix-pause-menu: the pause page's own state (each on its line [WORKLOG lesson 4]).
            confirming_restart: false,
            confirm_sel: 1,
            pause_tab: 0,
            // fix-pause-menu2 (each new field on its own line [WORKLOG lesson 4]).
            pause_objectives: Vec::new(),
            // objective-hints (each new field on its own line [WORKLOG lesson 4]).
            pause_objectives_sel: 0,
            pause_hint: None,
            // fix-save-profile (each new field on its own line [WORKLOG lesson 4]).
            profile_dir: None,
            profiles: Vec::new(),
            active_profile: None,
            profile: None,
            opts: default_opts(),
            opts_saved: None,
            cn_name: String::new(),
            // front-end-chain (each new field on its own line [WORKLOG lesson 4]).
            results: None,
            just_completed: false,
            new_reward: false,
            // codename-entry (each on its own line [WORKLOG lesson 4]).
            cn_typed: String::new(),
            cn_cell: 0,
            cn_from: 0,
            cn_clear_on_key: false,
            cn_box: None,
            cn_delete_row: None,
            cn_reserved: Vec::new(),
            cn_exit: None,
            cn_cheat_checked: false,
            cn_profile_mode: false,
            // secret-codes: the letter-grid model starts empty (the game's buffer 0x34e028 begins
            // empty on every page enter [M P_CNNAME 0x213790:46 re-binds the empty buffer]).
            secret: SecretEntry::default(),
            // screen-adjust-model (each new field on its own line [WORKLOG lesson 4]): the game
            // boots the offset at 0/0 (.sbss) [M psiRestoreScreenPos 0x10b940's reset value].
            screen_x: 0,
            screen_y: 0,
            screen_saved_x: 0,
            screen_saved_y: 0,
            screen_av_saved_x: 0,
            screen_av_saved_y: 0,

            // mp-menu-flow (each new field on its own line [WORKLOG lesson 4]).
            mp_join: std::array::from_fn(|_| MpJoinAgent::default()),
            mp_scenario: 0,
            mp_level: 0,
            mp_scenario_row: 0,
            mp_map_row: 0,
            mp_options_row: 0,
            mp_bots_row: 1,
            mp_bot_slot: 0,
            mp_bot_char: 0,
            mp_bots: std::array::from_fn(|slot| MpBot::fresh(slot)),
            mp_setup_state: 2,
            mp_setup_sel: 0,
            mp_time_limit: 10,
            mp_frag_limit: 10,
            mp_friendly_fire: 0,
            mp_weapon_set: 0,
            mp_radar: 1,
            mp_player_mods: [0, 0],
            mp_envmods: [1, 0, 0, 0, 0],
            mp_error: 0,
            mp_match_settings: None,
            // credits-data (each new field on its own line [WORKLOG lesson 4]): the credits roll's
            // state (crate::credits::CreditsRoll = the handler's own statics) and the opener.
            credits: crate::credits::CreditsRoll::new(crate::credits::PAGE_CN_AV, 100, false),
            credits_from: None,

            // menu-sfx (each new field on its own line [WORKLOG lesson 4]).
            /// The sounds this frame's input/page changes started — the host drains and plays them
            /// through nf-audio (never from inside the FrontEnd: tests stay silent).
            sfx_out: Vec::new(),
            /// `enabled_sound_flags` 0x30d184 — the boot value 0x0b gates MENU_SOUND 0/1/2/8
            /// [L ee_start 0x30d184 = 0x0b; M 0x1f3278's gates].
            enabled_sound_flags: 0x0b,
            /// The handlers' `menu_movie` static: set once the open pair has played, reset by the
            /// intro/attract movie ends [M P_START 0x20a408:22; P_INTRO 0x209e88:44,58].
            menu_movie: false };
        // The boot opens P_START: the pair plays with `menu_movie` clear [M P_START 0x20a408:12-22;
        // the game's boot path P_INTRO → P_START replays it the same way — the intro movie end
        // clears the static [M 0x209e88:44,58]].
        fe.open_pair_sfx();
        fe
    }

    /// The page-open sound pair [M the twelve handlers' msg-0x4c case — SFXRemove(0, 0x1d7/0x470)
    /// then SFXStart(0x1d7), SFXStart(0x470)], gated by `menu_movie`. The pair is emitted as two
    /// events; which one actually sounds depends on which front-end bank the game loaded
    /// ([D SB_0 vs SB_1], the alternation [M Sound_Ready 0x1dceb8:91-96] — the port's front-end
    /// bank state is the host's, so both events go out and the driver's lookup decides).
    fn open_pair_sfx(&mut self) {
        if self.menu_movie { return; }
        self.sfx_out.push(MenuSfx { sound: MENU_SOUND_PAGE_OPEN, id: SFX_MENU_OPEN_MOVE });
        self.sfx_out.push(MenuSfx { sound: MENU_SOUND_PAGE_OPEN, id: SFX_MENU_OPEN_SELECT });
        self.menu_movie = true;
    }

    /// Every front-end page change funnels through here so the open pair rides the transitions
    /// into the pair-playing pages (Start/Main/MissionSelect/Results/Credits/MpDebriefing — the
    /// port's stand-ins for the game's twelve handlers [M the SFXStart(0x1d7) caller list]).
    fn enter_page(&mut self, p: Page) {
        self.page = p;
        if matches!(p, Page::Start | Page::Main | Page::MissionSelect | Page::Results
                    | Page::Credits | Page::MpDebriefing) {
            self.open_pair_sfx();
        }
    }

    /// `Menu_PlaySound` [M 0x1f3278]: the gated sounds (0/1/2/8) drop when their
    /// `enabled_sound_flags` bit is clear; 3-7 play ungated; 9 (the switch default) starts nothing.
    fn sfx(&mut self, sound: u8, id: u32) {
        let bit = match sound { 0 => 1, 1 => 2, 2 => 4, 8 => 8, _ => 0 };
        if bit != 0 && self.enabled_sound_flags & bit == 0 { return; }
        self.sfx_out.push(MenuSfx { sound, id });
    }

    /// The cursor landed on a different control of the same page: MENU_SOUND 7 [M Page_Update
    /// 0x21edc8:38-53]. Returns true so the move arms stay one-liners.
    fn focus_moved(&mut self) -> bool {
        self.sfx(MENU_SOUND_FOCUS, SFX_MENU_TICK);
        true
    }

    /// Whether the cursor sits on a control the widget layer would blip for ✕/○: the rowed pages'
    /// row, the letter grids' cell. The Screen Adjust bars carry no select [L page 0x40000047's
    /// controls — two slider bars, the picture and the hint; S the no-blip reading].
    fn sfx_selectable(&self) -> bool {
        match self.page {
            Page::CnName => true,
            Page::ScreenAdjust => false,
            _ => self.cursor < self.rows(),
        }
    }

    /// The host's drain: everything the front end started since the last drain, in play order.
    /// The host plays each `id` through nf-audio's driver 2-D start; the events carry the game's
    /// own ids so the bank lookup decides audibility, exactly as the game's `SFXStart` /
    /// `Sound_Play` do [M 0x115a68, 0x1f3278].
    pub fn drain_sfx(&mut self) -> std::vec::Drain<'_, MenuSfx> {
        self.sfx_out.drain(..)
    }

    /// credits-data: opens the credits page 0x40000030 — the AV Options Credits row's page change
    /// and the win path's (P_WINGAME 0x215100:48 sends msg 0x44 with 0x40000030 after the
    /// 0x073f0048 movie). `from_page` = the opener's id (the game's page_from 0x523 param),
    /// `music_vol` = `SFXMusicGetVolume()` at enter, `bonus_flag` = GameState[0x52] [M the 0x4c
    /// case 0x2152b0:72-94]. Returns false when credits are already up.
    pub fn open_credits(&mut self, from_page: u32, music_vol: i32, bonus_flag: bool) -> bool {
        if self.page == Page::Credits { return false; }
        self.credits_from = Page::from_id(from_page);
        self.credits = crate::credits::CreditsRoll::new(from_page, music_vol, bonus_flag);
        self.enter_page(Page::Credits);
        self.cursor = 0;
        true
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
            Page::CnSelect => 2 + self.profiles.len(),
            Page::CnMenu => CN_MENU.len(),
            Page::CnName => 1, // the grid is not a row list; its cursor lives in `secret.cursor`
            Page::CnAdvanced => ADV_ROWS.len(),
            Page::CnMp => MP_ROWS.len(),
            Page::CnAv => AV_ROWS.len(),
            // credits-data: the credits page has no wheel rows (the two list controls are the
            // roll itself) [M P_CREDITS_Handler 0x2152b0 has no cursor code].
            Page::Credits => 0,
            // The Screen Adjust page's two sliders (the vertical and the horizontal bar) [L page
            // 0x40000047's two 0x100001eb controls, d1 0 = vertical / 1 = horizontal].
            Page::ScreenAdjust => 2,
            // The MISSION tab's rows [L page 0x4000004b's d1=6 list (153,183,334×120) binds labels
            // 0x16a CONTINUE / 0x16b RESTART / 0x16c QUIT via its m0x11 messages].
            Page::Pause => 3,
            // The FAILED results page is P_ENDMISSION's three rows [M 0x216618:19-27]; the
            // COMPLETED results page (P_NFRESULTS 0x208e78) has NO row list — the page's own
            // action is the hint's ~A Continue (Confirm anywhere).
            Page::Results => if self.outcome == Outcome::Complete { 1 } else { 2 },
            // ── mp-menu-flow ──
            // The four Join panels — the cursor walks the agents; agents 2-4 stay unjoined (no
            // pads in the port; the game joins them through their own controllers [M msg 0x6a]).
            Page::MpJoin => 4,
            Page::MpScenario => MP_SCENARIO_ROWS.len(),
            Page::MpMap => MP_MAP_ROWS.len(),
            // The setup page drives agent 1's single pick row (the game's per-agent lists live on
            // the four panel controls; keyboard = agent 1).
            Page::MpSetup => 1,
            Page::MpOptions => MP_OPTIONS_ROWS.len(),
            // Duration + Points/Lives [M P_MPRULES 0x20f790's two list controls 0x10000008/0x100000f6].
            Page::MpRules => 2,
            // Friendly fire, Weapon Set, +0x1BC, +0x1C8, Radar [M P_MPPLAYERMODS 0x2104d0:12-16].
            Page::MpPlayerMods => 5,
            // View distance, +0x1B8, +0x1D4, +0x1D0, vehicles [M P_MPENVIROMODS 0x210030:12-16].
            Page::MpEnviroMods => 5,
            Page::MpBots => MP_BOTS_ROWS.len(),
            Page::MpBotChoose => MP_CHARACTERS.len(),
            // Enabled, Skill, Aim, Hit points, Speed, Trait, Recovery 1, Recovery 2 [M P_MPBOTSETUP
            // 0x211270's eight list controls].
            Page::MpBotSetup => 8,
            Page::MpConfirm => 1,        // the summary page's own Confirm (the hint's ~A Start)
            Page::MpDebriefing => 1,
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
            // Edit Codename: New Codename, Default, then the card's codenames [L PCSX2 m29].
            Page::CnSelect if row >= 2 => self.profiles.get(row - 2).cloned().unwrap_or_default(),
            Page::CnSelect => bind([0x29b, 0x211][row.min(1)]),   // def_codename 0x2e1b18 [L docs/reports/fix-save-profile.md]
            Page::CnMenu => bind(CN_MENU[row.min(CN_MENU.len() - 1)].1),
            Page::CnName => String::new(), // the grid has no row labels (the model draws its own)
            Page::CnAdvanced | Page::CnMp | Page::CnAv => self.opt_rows().get(row).map_or(String::new(), |r| bind(r.label)),
            // credits-data: no rows — the roll's text lives in fe.credits.visible() [M].
            Page::Credits => String::new(),
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
            // The Screen Adjust page's bars carry no text — the game's page data has no label
            // controls on it, only the two slider bars, the picture and the hint [L page 0x40000047].
            Page::ScreenAdjust | Page::Results => String::new(),
            // ── mp-menu-flow: the wheels' own labels (the tables above) ──
            // The join panels: agent 1's states bind 0x1e8/0x37b/0x1e9 [M C_RBMPSTART 0x20d048's
            // 0x5c/0x4b arms — 0x1e8 "Press ~A to join", 0x37b "Choose Codename", 0x1e9 "Press ~A
            // when ready", 0x39c "Player Ready"]; agents 2-4 show the same 0x1e8 prompt (the
            // port's stand-in — the game's insert-controller text needs a pad [M
            // Menu_UpdateMPControllers 0x1f8d90]).
            Page::MpJoin => match self.mp_join.get(row).map_or(0, |a| a.state) {
                0 => bind(0x1e8),
                1 => bind(0x37b),
                5 => if self.mp_join[row.min(3)].ready { bind(0x39c) } else { bind(0x1e9) },
                _ => bind(0x1e8),
            },
            Page::MpScenario => {
                let r = row.min(MP_SCENARIO_ROWS.len() - 1);
                if self.mp_scenario_locked(r) { bind(MP_LOCKED_SCENARIO_DESC) } else { bind(MP_SCENARIO_ROWS[r].1) }
            }
            Page::MpMap => bind(MP_MAP_ROWS.get(row).map_or(0, |r| r.1)),
            // The setup page's one row is agent 1's pick: the team (the game's list items are
            // Phoenix 0x1c7 / MI6 0x1c8 [M P_MPSETUP 0x20d7a8]) or the character name.
            Page::MpSetup => {
                if self.mp_setup_state == 2 {
                    bind([0x1c7, 0x1c8][self.mp_setup_sel.min(1)])
                } else {
                    bind(MP_CHARACTERS[self.mp_setup_sel.min(MP_CHARACTERS.len() - 1)].1)
                }
            }
            Page::MpOptions => bind(MP_OPTIONS_ROWS.get(row).map_or(0, |r| r.1)),
            // The two list rows' static titles [L the menu-def's msg-0x18 b fields on page
            // 0x40000014: Duration 0x156, Points 0x24c]; the handler re-binds row 1 to 0x3d3
            // "Lives" for the death-limit scenario 0x10 [M P_MPRULES 0x20f790:78-95].
            Page::MpRules => bind([0x156, if self.mp_rules_lives() { 0x3d3 } else { 0x24c }][row.min(1)]),
            // The rows' static titles in page order [L the menu-def msg-0x18 b fields on page
            // 0x40000017: 0x1ae (Friendly Fire, +0x198), 0x24d (Weapon Set, +0x1B4), 0x1ac (+0x1BC),
            // 0x24e (+0x1C8), 0x1e5 (Radar, +0x1C4)].
            Page::MpPlayerMods => bind([0x1ae, 0x24d, 0x1ac, 0x24e, 0x1e5][row.min(4)]),
            // [L page 0x40000028's msg-0x18 b fields: 0x1af (View Distance, +0x1C0), 0x1ab (+0x1B8),
            // 0x1aa (+0x1D4 — the reward-locked row), 0x328 (+0x1D0), 0x5b9 (vehicles, +0x1CC)].
            Page::MpEnviroMods => bind([0x1af, 0x1ab, 0x1aa, 0x328, 0x5b9][row.min(4)]),
            Page::MpBots => {
                let r = row.min(MP_BOTS_ROWS.len() - 1);
                if r == 0 { bind(0x171) }
                else {
                    // The runtime row: "Setup Bot N : Playing Yes/No" — the game sprintf's it per
                    // scroll [M C_SBBOTS 0x210a08:40-59]; the port composes the same labels.
                    let b = &self.mp_bots[r - 1];
                    format!("{} : {} {}", bind(MP_BOTS_ROWS[r].1), bind(0x295), bind(if b.enabled { 0x181 } else { 0x182 }))
                }
            }
            Page::MpBotChoose => bind(MP_CHARACTERS.get(row).map_or(0, |r| r.1)),
            // The Bot Setup page's row titles in its own y order (85..309) [L the menu-def's
            // msg-0x18 b fields on page 0x4000002c: 0x295 (Enabled), 0x247 (Speed), 0x232 (Trait),
            // 0x246 (Aim), 0x15e (Skill), 0x1c6 (Hit points), 0x23a (Recovery 1), 0x249 (Recovery 2)].
            Page::MpBotSetup => bind(MP_BOT_SETUP_LABELS[row.min(7)]),
            Page::MpConfirm | Page::MpDebriefing => String::new(),
        }
    }

    /// The option page's rows (empty off the option pages).
    pub fn opt_rows(&self) -> &'static [OptRow] {
        match self.page { Page::CnAdvanced => ADV_ROWS, Page::CnMp => MP_ROWS, Page::CnAv => AV_ROWS, _ => &[] }
    }

    // --- The picture offset (AV Options → Screen Adjust) [M docs/systems/screen-adjust.md §3] ---

    /// `psiAdjustScreenPos(dx, dy)` 0x10b890: X += dx·12 clamped ±0x1b0 (±432), Y += dy·4 clamped
    /// ±0x30 (±48). `dx`/`dy` are the slider's −1..1 steps (C_SBSCREEN 0x217b08 msg 0x49 passes the
    /// slider value; d1 0 = vertical/Y, d1 1 = horizontal/X).
    pub fn screen_adjust(&mut self, dx: i32, dy: i32) {
        self.screen_y = (self.screen_y + dy * 4).clamp(-0x30, 0x30);
        self.screen_x = (self.screen_x + dx * 12).clamp(-0x1b0, 0x1b0);
    }

    /// P_SCREENADJUST's msg 0x4c (page enter): `psiGetScreenPos(&PS2OldX, &PS2OldY)` 0x217a58:8-10 —
    /// the offset the Cancel path restores [M; Cancel rides the manager's msg 0x6b close
    /// notification, which the ✕ path's 0x5f(1) skips — §2 of the doc].
    pub fn screen_adjust_open(&mut self) {
        self.screen_saved_x = self.screen_x;
        self.screen_saved_y = self.screen_y;
    }

    /// P_SCREENADJUST's msg 0x6b (closed without Confirm): `psiSetScreenPos(PS2OldX, PS2OldY)`
    /// 0x217a58:16-18 [M].
    pub fn screen_adjust_cancel(&mut self) {
        self.screen_x = self.screen_saved_x;
        self.screen_y = self.screen_saved_y;
    }

    /// `psiRestoreScreenPos` 0x10b940 — the Restore Defaults row's offset reset [M 0x214818:107].
    pub fn screen_restore_defaults(&mut self) {
        self.screen_x = 0;
        self.screen_y = 0;
    }

    /// P_CNAVOPTIONS' screen snapshot (`screen_adjust_x/y` 0x30d8e8/ec): taken at page enter and
    /// RE-taken on every re-enter (0x214818:29, 135) — so returning from Screen Adjust updates the
    /// baseline the page's Back restores [M].
    pub fn av_screen_snapshot(&mut self) {
        self.screen_av_saved_x = self.screen_x;
        self.screen_av_saved_y = self.screen_y;
    }

    /// P_CNAVOPTIONS' msg 0x4d Back: `psiSetScreenPos(screen_adjust_x, screen_adjust_y)` 0x214818:82 [M].
    pub fn av_screen_restore(&mut self) {
        self.screen_x = self.screen_av_saved_x;
        self.screen_y = self.screen_av_saved_y;
    }

    /// The picture offset in the menu's 640×480 authoring space: the visible line is 2560 X-units
    /// (MAGH 5 × 512) so 640-space x = X/4; 448 field lines make 480-space y = Y·480/448 = Y·15/14
    /// [M VBlankInt 0x124088 + sceGsSetDefDispEnv 0x226370; the derivation is §3 of the doc].
    pub fn screen_offset_640(&self) -> (f32, f32) {
        (self.screen_x as f32 / 4.0, self.screen_y as f32 * 15.0 / 14.0)
    }

    /// The picture offset as fractions of the visible picture: (X/2560, Y/448) [M §3]. The renderer
    /// multiplies by its own viewport size; the DIRECTION of the shift on the TV is the report's
    /// live check (DISPLAY's DX/DY select where the read starts).
    pub fn screen_offset_frac(&self) -> (f32, f32) {
        (self.screen_x as f32 / 2560.0, self.screen_y as f32 / 448.0)
    }

    /// The view aspect the game's camera writes at viewer+0x114: 16/9 (0x3fe38e39) with the
    /// widescreen option set, 4/3 (0x3faaaaab) otherwise [M Camera_CalcViewAngles 0x1baf70:28-32].
    /// Split-screen's ×0.5 / ×2 (two viewers) is not modelled — no split-screen view.
    pub fn view_aspect(&self) -> f32 {
        if self.opt_value(0x1000_0223) == 1 { 16.0 / 9.0 } else { 4.0 / 3.0 }
    }

    /// The current value of an option row.
    pub fn opt_value(&self, ctrl: u32) -> i32 { self.opts.get(&ctrl).copied().unwrap_or(0) }

    /// Steps a List row to its next (or previous) item, or moves a slider by 10 within 0..=100 [M the list
    /// controls' f2 3 cycle; the sliders' d3 0x64 range].
    fn opt_step(&mut self, row: &OptRow, dir: i32) {
        let v = self.opt_value(row.ctrl);
        let nv = match row.kind {
            OptKind::Slider => (v + 10 * dir).clamp(0, 100),
            OptKind::List if !row.items.is_empty() => {
                let i = row.items.iter().position(|it| it.2 == v).unwrap_or(0) as i32;
                let n = row.items.len() as i32;
                row.items[(i + dir).rem_euclid(n) as usize].2
            }
            _ => return,
        };
        self.opts.insert(row.ctrl, nv);
        if nv != v {
            // menu-sfx: the spinner's ◀/▶ step, first press — MENU_SOUND 3 (0x1d8) down-count,
            // 4 (0x1d9) up-count; no sound when the value didn't move [M Scroll_Update
            // 0x221b20:186-208 — the min/max guards skip the play; the game's wrap replays the
            // same first-step ids]. The held-repeat ids (5/6) are the host key-repeat's concern.
            self.sfx(if dir < 0 { MENU_SOUND_VAL_DEC } else { MENU_SOUND_VAL_INC },
                     if dir < 0 { SFX_MENU_VAL_DEC } else { SFX_MENU_VAL_INC });
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
            self.menu_movie = false; // the intro movie end clears menu_movie [M P_INTRO 0x209e88:44,58]
            self.intro_index = 0;
        }
        // credits-data: the credits page's 0x50 tick; the delayed 0x44/0x5f exit routes the page
        // [M P_CREDITS_Handler 0x2152b0:103-195 — ToMain = msg 0x44/0x40000002 after the win path,
        // PopBack = msg 0x5f back to the opener].
        if self.page == Page::Credits {
            if let Some(exit) = self.credits.tick() {
                match exit {
                    crate::credits::CreditsExit::ToMain => { self.enter_page(Page::Main); self.cursor = 0; }
                    crate::credits::CreditsExit::PopBack(from) => {
                        self.page = Page::from_id(from).unwrap_or(Page::Main);
                        self.cursor = 0;
                    }
                }
            }
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
            self.enter_page(Page::Pause);
            self.cursor = 0;
            self.pause_tab = 0;
            self.confirming_restart = false;
            self.confirming_quit = false;
            self.confirm_sel = 1;   // the YES/NO list opens on NO [L m0x1d:1; M 0x204408:181/204]
            // objective-hints: a fresh pause has no hint box up and the objectives list's
            // selection comes from the tab's populate (per-entry reset [M 0x204840-0x204850]).
            self.pause_hint = None;
            self.pause_objectives_sel = 0;
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
        self.enter_page(Page::Results);
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
        self.enter_page(Page::Difficulty);
        self.cursor = 1; // Agent [M the wheel starts centred on the middle row]
        true
    }

    /// codename-entry: the Edit Codename list's data load — the file read half of `load_codename`
    /// without the play flow's page change [M C_SBCNSELECT 0x212f58 case 0x4b sel ≥ 2: slot = sel,
    /// strcpy(0x2e1ae8, name), box mode 0 = LS_Load 0x1f7de8:15-17 — then the Edit wheel opens].
    fn load_profile_data(&mut self, row: usize) -> bool {
        let Some(name) = self.profiles.get(row.wrapping_sub(2)).cloned() else { return false };
        let Some(dir) = self.profile_dir.clone() else { return false };
        let path = nf_game::profile::Profile::path_in(&dir, &name);
        let Ok(profile) = nf_game::profile::Profile::load_file(&path) else { return false };
        let status = profile.progress.nightfire_status();
        for (i, u) in self.unlocked.iter_mut().enumerate() {
            *u = status & (1 << i) != 0;
        }
        self.active_profile = Some(name);
        self.profile = Some(profile);
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

    /// The Records page's medal for campaign row `row`: `(label, emblem texture)` or None when no medal is earned [M
    /// C_RBDSRECORDS 0x20bf00:58-110 — the level's four bonus flags k1..k4 (Menu_GetLevelBonuses +4/+0x10/+0x1c/+0x28):
    /// all four → 0x307/0x300013e, k1..k3 → 0x306/0x300013f, k1+k2 → 0x308/0x3000140, k1 → 0x309/0x300013d].
    pub fn record_medal(&self, row: usize) -> Option<(u32, u32)> {
        let profile = self.profile.as_ref()?;
        let hash = nf_game::progress::SP_LEVEL_HASHES.get(row).copied()?;
        let k = |n: u32| profile.progress.has_medal(hash, n, 0);
        if !k(1) { return None; }
        Some(if k(2) && k(3) && k(4) { (0x307, 0x0300_013e) } else if k(2) && k(3) { (0x306, 0x0300_013f) } else if k(2) { (0x308, 0x0300_0140) } else { (0x309, 0x0300_013d) })
    }

    /// The intro movie the P_INTRO run currently shows [M 0x209e88's movie_283 counter].
    pub fn intro_movie_code(&self) -> u32 { INTRO_MOVIES[self.intro_index.min(INTRO_MOVIES.len() - 1)] }

    // ── codename-entry: the name-entry page 0x40000020 (P_CNNAME_Handler 0x213790 + the grid's
    // C_KEYBOARD_Handler 0x213a98) ──────────────────────────────────────────────────────────────

    /// The grid's special keys [M C_KEYBOARD_Handler 0x213a98 keys on the pressed cell's id
    /// (ctrl+0x20): 1000 = remove the last typed character (:39-42), 1001 = the SPACE cell (the
    /// character at DAT_0030d380 = 0x20 [L ee_snapshot] :28-36), 1002 = the ✓ confirm (:45-65);
    /// P_CNNAME_Handler 0x213790:47 focuses 1002 on open].
    pub const CN_KEY_DELETE: u32 = 1000;
    pub const CN_KEY_SPACE: u32 = 1001;
    pub const CN_KEY_CONFIRM: u32 = 1002;
    /// The four cells P_CNNAME re-labels at init [M 0x213790:50-57 — `__Menu_SendEx(grid
    /// 0x10000074, cell, 0x18, string)` with the single bytes 0xc4/0xc2/0xc6/0xc0 written to
    /// 0x30d8d8/da/dc/de; which glyphs the menu font draws there is the draw code's question].
    pub const CN_GLYPH_CELLS: [u32; 4] = [100, 101, 102, 103];

    /// The name-entry grid as rows of cell ids. Digit/letter cells carry their ASCII code and
    /// type it [M 0x213a98:79-85 appends the first byte of the cell's text]; the last row is the
    /// four glyph cells + ← (delete) → (space) ✓ (confirm). The ROW PACKING is the port's [S] —
    /// the game's packing sits in page 0x40000020's grid data, which the port has not parsed;
    /// the ids and the three special keys are the game's own [M].
    pub const CN_GRID: [&[u32]; 5] = [
        &[0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39],       // 0-9
        &[0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49],             // A-I
        &[0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52],             // J-R
        &[0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a],                   // S-Z
        &[100, 101, 102, 103, Self::CN_KEY_DELETE, Self::CN_KEY_SPACE, Self::CN_KEY_CONFIRM],  // glyphs + ← → ✓
    ];

    /// The typed name's capacity [M C_KEYBOARD_Handler 0x213a98:75-77 — an append is refused once
    /// strlen(&buffer) > 7, so the buffer at 0x34e028 holds 8 characters + NUL].
    pub const CN_MAX_CHARS: usize = 8;

    /// Opens the name-entry page. `from` is the page it came from — the 0x4c init's param_6 [M
    /// Manager_SendMessage 0x21c858:344-345 passes the OLD page's id to the new page's init].
    /// 0x4000001d (the wheel) opens the cheat-codename entry (cheat_mode [M P_CNNAME 0x213790:33]).
    /// The typed buffer starts at the game's proposals: "BOND" for a profile name (DAT_0030d530
    /// [L ee_snapshot] — P_CNNAME 0x213790:43-44 copies it over `buffer`) or empty for the cheat
    /// entry (DAT_0030d2d8 [L] :36); the ✓ cell holds the focus [M :47-48]; the first press clears
    /// (del_when_keypressed [M 0x213a98:18-27]).
    pub fn cn_open(&mut self, from: u32) {
        self.enter_page(Page::CnName);
        // The mode flag: PROFILE-name entry (from Select Codename / Edit Codename) runs the
        // validator/create flow; the wheel's entry (0x4000001d) is the cheat/secret grid [M
        // P_CNNAME 0x213790:33 cheat_mode = param_6 == 0x4000001d].
        self.cn_profile_mode = from != PAGE_CN_MENU;
        self.cn_from = from;
        self.cn_cell = Self::CN_KEY_CONFIRM;
        self.cn_clear_on_key = true;
        self.cn_box = None;
        self.cn_delete_row = None;
        self.cn_exit = None;
        self.cn_cheat_checked = false;
        self.cn_typed = if from == PAGE_CN_MENU { String::new() } else { "BOND".into() };
    }

    /// The typed name (the game's `buffer` 0x34e028) for the draw code.
    pub fn cn_typed_name(&self) -> &str { &self.cn_typed }

    /// The focused grid cell id.
    pub fn cn_cursor_cell(&self) -> u32 { self.cn_cell }

    /// The (row, column) of a cell id in [`CN_GRID`].
    fn cn_find(cell: u32) -> Option<(usize, usize)> {
        Self::CN_GRID.iter().enumerate().find_map(|(r, row)| row.iter().position(|&c| c == cell).map(|c| (r, c)))
    }

    /// Moves the cursor by (dx, dy) within [`CN_GRID`], clamped at the edges.
    pub fn cn_move(&mut self, dx: i32, dy: i32) {
        let Some((r0, c0)) = Self::cn_find(self.cn_cell) else { return };
        let r = (r0 as i32 + dy).clamp(0, Self::CN_GRID.len() as i32 - 1) as usize;
        let c = (c0 as i32 + dx).clamp(0, Self::CN_GRID[r].len() as i32 - 1) as usize;
        let next = Self::CN_GRID[r][c];
        if next != self.cn_cell {
            // The focus moved between the grid's cells: MENU_SOUND 7 [M Page_Update 0x21edc8:38-53;
            // the game's clamped edge moves keep the focus, and Page_Update only sounds on a change].
            self.cn_cell = next;
            self.focus_moved();
        }
    }

    /// Types one character through the C_KEYBOARD append path [M 0x213a98:75-85 — the cell's
    /// text's first byte joins `buffer` unless strlen > 7]. Also the direct-keyboard path's
    /// core (the port's own addition [S]; the game's grid stays the reference).
    pub fn cn_type_char(&mut self, ch: char) -> bool {
        if self.cn_typed.chars().count() >= Self::CN_MAX_CHARS { return false; }
        self.cn_typed.push(ch);
        true
    }

    /// Presses a grid cell — C_KEYBOARD_Handler 0x213a98's msg-0x4b path. Returns true when the
    /// press was consumed.
    pub fn cn_press(&mut self, cell: u32) -> bool {
        if cell == Self::CN_KEY_CONFIRM {
            return self.cn_confirm();
        }
        // The first press after open (or after a refused name) clears the proposal first; the
        // SPACE key's press is swallowed by that clear [M del_when_keypressed 0x213a98:18-27 —
        // `buffer` resets to DAT_0030d2d8 and the 0x3e9 arm returns before typing].
        if self.cn_clear_on_key {
            self.cn_clear_on_key = false;
            self.cn_typed.clear();
            if cell == Self::CN_KEY_SPACE { return true; }
        }
        match cell {
            Self::CN_KEY_DELETE => { self.cn_typed.pop(); true }               // [M :39-42]
            Self::CN_KEY_SPACE => self.cn_type_char(' '),                      // [M :28-36]
            // The letter/digit cells type their character [M 0x213a98:79-85]; the four glyph
            // cells carry the menu font's slots 0xc4/0xc2/0xc6/0xc0 [M P_CNNAME 0x213790:50-57].
            100 => self.cn_type_char('\u{c4}'),
            101 => self.cn_type_char('\u{c2}'),
            102 => self.cn_type_char('\u{c6}'),
            103 => self.cn_type_char('\u{c0}'),
            id if (0x20..=0x7e).contains(&id) => self.cn_type_char(char::from_u32(id).unwrap_or('?')),
            _ => false,
        }
    }

    /// The ✓ confirm — C_KEYBOARD_Handler 0x213a98:45-65 + P_CNNAME_Handler 0x213790:59-79.
    fn cn_confirm(&mut self) -> bool {
        // The ✓ opens a NEW box (or none) — a refusal's box does not survive the next press
        // [M Menu_CreateOptionBox 0x202218 opens per-press; the game holds one active box].
        self.cn_box = None;
        if !self.cn_profile_mode {
            // cheat_mode: Menu_SpecialCodenameCheck 0x200238 (52 strcmp names from 0x308578,
            // unlock masks into sp_level+0x10, Menu_SetBonus) — not ported (reported). The
            // game creates nothing on a miss [M C_KEYBOARD 0x213a98:67-73] and neither do we.
            self.cn_cheat_checked = true;
            return true;
        }
        let reserved: Vec<String> = self.cn_reserved.clone();
        let reserved: Vec<&str> = reserved.iter().map(String::as_str).collect();
        match nf_game::profile::Profile::check_codename(&self.cn_typed, &reserved) {
            // The validator's refusals bind their message label onto box 6 [M 0x213a98:47-51 —
            // Menu_CreateOptionBox(player, Menu_IsValidCodename's return, 6, 1, 0)] and the
            // next press clears again [M :50 del_when_keypressed = 1].
            Err(nf_game::profile::CodenameError::Empty) => {
                self.cn_box = Some((6, 0x3bc)); self.cn_clear_on_key = true; false
            }
            Err(nf_game::profile::CodenameError::Reserved) => {
                self.cn_box = Some((6, 0x2c0)); self.cn_clear_on_key = true; false
            }
            Err(_) => false,
            Ok(normalized) => {
                // The validator uppercases + trims the buffer in place [M Menu_IsValidCodename
                // 0x1f9930:60-77 strcpy(param_1, acStack_a0)].
                self.cn_typed = normalized.clone();
                match self.create_codename(&normalized, &reserved) {
                    Ok(name) => {
                        // The accept [M 0x213a98:53-65]: the working name 0x2e1ae8, the save
                        // through the message box (mode 1 = LS_Save 0x1f7de8:21-29), then the
                        // wheel 0x4000001d (DAT_002e1ad8 :58) whose init forwards to Controls
                        // 0x40000022 [M P_CNMENU 0x213288:38-40 — not ported; the port lands
                        // on the wheel].
                        self.cn_name = name;
                        self.cn_exit = Some(PAGE_CN_MENU);
                        self.enter_page(Page::CnMenu);
                        self.cursor = 1;
                        true
                    }
                    Err(nf_game::profile::CodenameError::Exists) => {
                        // The card flow's overwrite question — P_CNNAME's 0x50 tick watches
                        // box 4 and saves on YES [M 0x213790:62-76]. The port refuses [M
                        // MemCard_ASK_OVERWRITE 0x10ee38, create_codename's own note]; the
                        // box stands for the host to answer.
                        self.cn_box = Some((4, 0));
                        false
                    }
                    Err(_) => false,
                }
            }
        }
    }

    /// Dismisses the pending option box (the game's Confirm/Back both close the validator's box;
    /// box 4's YES would re-save [M P_CNNAME 0x50:62-76] — the port only dismisses).
    pub fn cn_dismiss_box(&mut self) {
        self.cn_box = None;
    }

    /// The Edit Codename list's ~X Delete request on a saved row [M C_SBCNSELECT 0x212f58 case
    /// 0x5e — sel > 1, box mode 3 psiDeleteData 0x1f7de8:38-57 under the card-flow confirm;
    /// P_CNSELECT 0x212ad0 case 0x6e re-enumerates after]. Confirm = YES, Back = NO.
    pub fn cn_delete_begin(&mut self, row: usize) -> bool {
        if row < 2 || row >= 2 + self.profiles.len() { return false; }
        self.cn_delete_row = Some(row);
        true
    }

    /// The delete confirm's YES [M the mode-3 box → psiDeleteData(0x2e1ae8)].
    pub fn cn_delete_confirm(&mut self) -> bool {
        let Some(row) = self.cn_delete_row.take() else { return false };
        let Some(name) = self.profiles.get(row - 2).cloned() else { return false };
        let Some(dir) = self.profile_dir.clone() else { return false };
        let gone = nf_game::profile::Profile::delete_in(&dir, &name);
        self.reload_profiles();
        self.cursor = self.cursor.min(self.rows().saturating_sub(1));
        gone
    }

    /// The delete confirm's NO [M the card-flow box's cancel path].
    pub fn cn_delete_cancel(&mut self) { self.cn_delete_row = None; }

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
            self.menu_movie = false; // the intro movie end clears menu_movie [M P_INTRO 0x209e88:44,58]
                self.intro_index = 0;
                return true;
            }
            return false;
        }
        // ◀ / ▶ change values on the option pages and the MP value pages and move the Secret Unlocks
        // grid cursor; elsewhere they keep the old Back / Confirm meaning. The MP wheels
        // (scenario/map/options/bots) keep the remap.
        let a = match (a, matches!(self.page, Page::CnAdvanced | Page::CnMp | Page::CnAv | Page::CnName
            | Page::MpJoin | Page::MpSetup | Page::MpRules | Page::MpPlayerMods | Page::MpEnviroMods | Page::MpBotSetup | Page::ScreenAdjust)) {
            (Action::Left, false) => Action::Back,
            (Action::Right, false) => Action::Confirm,
            (a, _) => a,
        };
        // menu-sfx: the widget layer's select/back sounds fire with the input itself, before the
        // page handler decides what the input means [M Button_Update 0x1ef948:104-127 and the other
        // widget Updates play on the raw action edge; the △ buzz rides MenuManager_Update
        // 0x21c3d8:78-105]. Confirm = MENU_SOUND 0 (&1), Back = MENU_SOUND 8 (&8, the manager's △
        // path), Side = MENU_SOUND 2 (&4 — clear in the boot flags 0x0b [L 0x30d184], so ○ is
        // silent on the disc). Dead rows blip in the game too — the widget doesn't know the page
        // handler ignores the message — so the port emits on any cursor-able control [S the dead-
        // row reading; the ScreenAdjust bars select nothing]. The host drains `sfx_out`.
        match a {
            Action::Confirm if self.sfx_selectable() => self.sfx(MENU_SOUND_SELECT, SFX_MENU_TICK),
            Action::Back => self.sfx(MENU_SOUND_BACK, SFX_MENU_VAL_INC),
            Action::Side if self.sfx_selectable() => self.sfx(MENU_SOUND_SIDE_CIRCLE, SFX_MENU_TICK),
            _ => {}
        }
        match self.page {
            Page::CnSelect => match a {
                Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                Action::Down if self.cursor + 1 < self.rows() => { self.cursor += 1; return self.focus_moved(); }
                // The ~X Delete on a saved codename asks first [M C_SBCNSELECT 0x212f58 case 0x5e
                // — sel > 1 → box mode 3 psiDeleteData 0x1f7de8:38-57 under the card-flow
                // confirm; P_CNSELECT 0x212ad0 case 0x6e re-enumerates after].
                Action::Side if self.cursor >= 2 && self.cn_delete_row.is_none() => {
                    self.cn_delete_begin(self.cursor);
                    return true;
                }
                // The delete confirm's YES/NO [M the card-flow box].
                Action::Confirm if self.cn_delete_row.is_some() => {
                    self.cn_delete_confirm();
                    return true;
                }
                Action::Back if self.cn_delete_row.is_some() => {
                    self.cn_delete_cancel();
                    return true;
                }
                // Row 0 "New Codename" opens the name entry [M C_SBCNSELECT 0x212f58 case 0x4b
                // sel 0 — Menu_MapDefaultCodename(0,0,0xff) then page 0x40000020 with the
                // return-page send to 0x10000100].
                Action::Confirm if self.cursor == 0 => {
                    self.cn_open(PAGE_CN_SELECT);
                    return true;
                }
                // Edit: the default row or a codename opens its options wheel [M C_SBCNSELECT 0x212f58 → page
                // 0x4000001d]; the picked codename loads first [M case 0x4b sel ≥ 2 — box mode 0 =
                // LS_Load 0x1f7de8:15-17], so the wheel's Save Codename writes the loaded data.
                Action::Confirm if self.cursor >= 1 => {
                    self.cn_name = if self.cursor == 1 { "Default".into() } else { self.profiles.get(self.cursor - 2).cloned().unwrap_or_default() };
                    if self.cursor >= 2 { self.load_profile_data(self.cursor); }
                    self.enter_page(Page::CnMenu);
                    self.cursor = 1; // the wheel opens on Controller Setup [L PCSX2 m30]
                    return true;
                }
                Action::Back => { self.enter_page(Page::Main); self.cursor = 2; return true; }
                _ => return false,
            },
            Page::CnMenu => match a {
                Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                Action::Down if self.cursor + 1 < self.rows() => { self.cursor += 1; return self.focus_moved(); }
                // [M C_SBCNOPTIONS_Handler 0x213528:32-60: row 0 → page 0x40000020 (Secret
                // Unlocks / the codename entry); rows 3/4/5 → pages 0x4000002d/2e/31; the
                // LAST row (6) is Save Codename: a default re-maps and leaves [M case 6 —
                // Menu_UpdateDefaultCodename + Menu_ChangePageCloseIris], a saved codename saves
                // through box mode 1 = LS_Save(0x2e1ae8) [M 0x1f7de8:21-29] — the port writes the
                // active profile in place.]
                Action::Confirm => {
                    match self.cursor {
                        0 => { self.opts_saved = Some(self.opts.clone()); self.av_screen_snapshot(); self.cn_open(PAGE_CN_MENU); self.cursor = 0; return true; }
                        3 => { self.opts_saved = Some(self.opts.clone()); self.av_screen_snapshot(); self.enter_page(Page::CnAdvanced); self.cursor = 0; return true; }
                        4 => { self.opts_saved = Some(self.opts.clone()); self.av_screen_snapshot(); self.enter_page(Page::CnMp); self.cursor = 0; return true; }
                        5 => { self.opts_saved = Some(self.opts.clone()); self.av_screen_snapshot(); self.enter_page(Page::CnAv); self.cursor = 0; return true; }
                        6 => {
                            if self.profiles.iter().any(|p| *p == self.cn_name) {
                                self.save_profile();
                            }
                            self.enter_page(Page::CnSelect);
                            self.cursor = 1;
                            return true;
                        }
                        _ => return false,
                    }
                }
                Action::Back => { self.enter_page(Page::CnSelect); self.cursor = 1; return true; }
                _ => return false,
            },
            Page::CnName => {
                // codename-entry: the PROFILE-name mode (cn_open from 0x40000025 / 0x4000001b)
                // runs the validator/create flow [M C_KEYBOARD 0x213a98:45-65]; the cheat entry
                // (from the wheel) falls through to the Secret Unlocks grid below [M 0x213790:33].
                if self.cn_profile_mode {
                    match a {
                        Action::Left => { self.cn_move(-1, 0); return true; }
                        Action::Right => { self.cn_move(1, 0); return true; }
                        Action::Up => { self.cn_move(0, -1); return true; }
                        Action::Down => { self.cn_move(0, 1); return true; }
                        Action::Confirm => {
                            if self.cn_box.is_some() { self.cn_dismiss_box(); return true; }
                            let cell = self.cn_cell;
                            return self.cn_press(cell);
                        }
                        Action::Back if self.cn_box.is_some() => { self.cn_dismiss_box(); return true; }
                        Action::Back => {
                            self.enter_page(Page::from_id(self.cn_from).unwrap_or(Page::Main));
                            self.cursor = 0;
                            return true;
                        }
                        _ => return false,
                    }
                }
                // The letter grid [M C_KEYBOARD_Handler 0x213a98]: cells append (8-char cap),
                // Delete truncates, Space appends ' ', Submit validates through
                // `nf_game::secret_codes::submit` (Menu_SpecialCodenameCheck 0x200238's port);
                // a match opens the 0x100029d box, a miss is silent [M :67-73].
                match a {
                    Action::Left if self.secret.cursor > 0 => { self.secret.cursor -= 1; return self.focus_moved(); }
                    Action::Right if self.secret.cursor + 1 < SECRET_GRID.len() => { self.secret.cursor += 1; return self.focus_moved(); }
                    Action::Up if self.secret.cursor >= 8 => { self.secret.cursor -= 8; return self.focus_moved(); }
                    Action::Down if self.secret.cursor + 8 < SECRET_GRID.len() => { self.secret.cursor += 8; return self.focus_moved(); }
                    Action::Confirm => { self.secret_press(); return true; }
                    Action::Back => { self.secret = SecretEntry::default(); self.enter_page(Page::CnMenu); self.cursor = 1; return true; }
                    _ => return false,
                }
            }
            Page::CnAdvanced | Page::CnMp | Page::CnAv => {
                let rows = self.opt_rows();
                let back_row = match self.page { Page::CnAdvanced => 3, Page::CnMp => 4, _ => 5 };
                match a {
                    Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                    Action::Down if self.cursor + 1 < rows.len() => { self.cursor += 1; return self.focus_moved(); }
                    Action::Left => { let r = &rows[self.cursor]; self.opt_step(r, -1); return true; }
                    Action::Right => { let r = &rows[self.cursor]; self.opt_step(r, 1); return true; }
                    // The action rows [M P_CNAVOPTIONS 0x214818 msg 0x4b]: Screen Adjust (0x10000137)
                    // pushes page 0x40000047 (:117-126); Restore Defaults (0x100001a3) runs the full
                    // reset list (:96-110); Credits (0x10000138) opens the credits page 0x40000030 (credits-data); the
                    // trailer (0x10000222) opens a page not ported yet (:91-94, 112-115).
                    Action::Confirm if rows[self.cursor].kind == OptKind::Action => {
                        match rows[self.cursor].ctrl {
                            0x1000_0137 => {
                                self.screen_adjust_open();          // the subscreen's own 0x4c snapshot [M 0x217a58:8-10]
                                self.enter_page(Page::ScreenAdjust);
                                self.cursor = 0;                    // the vertical bar first [L page data's d1 0 arm]
                                return true;
                            }
                            0x1000_01a3 => {
                                // The game's own reset list [M 0x214818:97-110]: music slider 0xe →
                                // volume 70, sfx 0x14 → 100, subtitles per LanguageToLoad (NTSC-U's
                                // language 0 → Off), split 0, speaker 1, widescreen 0, and
                                // psiRestoreScreenPos. (Not default_opts — the Default profile's
                                // music 80 is a different thing.)
                                self.opts.insert(0x1000_0135, 70);
                                self.opts.insert(0x1000_0134, 100);
                                self.opts.insert(0x1000_0136, 0);
                                self.opts.insert(0x1000_019a, 0);
                                self.opts.insert(0x1000_0198, 1);
                                self.opts.insert(0x1000_0223, 0);
                                self.screen_restore_defaults();
                                return true;
                            }
                            0x1000_0138 => {
                                let vol = self.opts.get(&0x1000_0135).copied().unwrap_or(100);
                                return self.open_credits(crate::credits::PAGE_CN_AV, vol, false);
                            }
                            _ => return false,
                        }
                    }
                    // Any other row's ✕ commits: the option globals keep the edits and the offset's
                    // cancel baseline re-snapshots [M 0x214818:127-136 — psiGetScreenPos at :135],
                    // then the page closes back to the wheel (Manager 0x5f(0,0)).
                    Action::Confirm => {
                        self.opts_saved = None;
                        if self.page == Page::CnAv { self.av_screen_snapshot(); }
                        self.enter_page(Page::CnMenu); self.cursor = back_row; return true;
                    }
                    Action::Back => {
                        if let Some(o) = self.opts_saved.take() { self.opts = o; }
                        // The game's Back restores the offset snapshot too (msg 0x4d) — but only
                        // when no subscreen was opened; a subscreen trip re-snapshots on re-enter,
                        // so the stored baseline is always the right one [M 0x214818:79-85].
                        if self.page == Page::CnAv { self.av_screen_restore(); }
                        self.enter_page(Page::CnMenu); self.cursor = back_row; return true;
                    }
                    _ => return false,
                }
            }
            // The Screen Adjust page [M P_SCREENADJUST 0x217a58 + C_SBSCREEN 0x217b08]: two sliders
            // (row 0 vertical/Y, row 1 horizontal/X — the page data's d1 0/1); a slider step calls
            // psiAdjustScreenPos with the −1..1 value (X step 12, Y step 4); ✕ keeps (Manager 0x5f(1)
            // skips the restore notification [M 0x21c858 case 0x5f]) and ○/Back restores the enter
            // snapshot (the msg 0x6b path, the page hint's "Cancel" [L 0x67b]).
            Page::ScreenAdjust => {
                match a {
                    Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                    Action::Down if self.cursor + 1 < self.rows() => { self.cursor += 1; return self.focus_moved(); }
                    Action::Left => {
                        if self.cursor == 0 { self.screen_adjust(0, -1); } else { self.screen_adjust(-1, 0); }
                        return true;
                    }
                    Action::Right => {
                        if self.cursor == 0 { self.screen_adjust(0, 1); } else { self.screen_adjust(1, 0); }
                        return true;
                    }
                    Action::Confirm => {
                        self.av_screen_snapshot();   // the AV page's re-enter re-snapshots [M 0x214818:135]
                        self.enter_page(Page::CnAv);
                        self.cursor = 6;             // the Screen Adjust row [L page data y=256 is the 7th row]
                        return true;
                    }
                    Action::Back => {
                        self.screen_adjust_cancel();
                        self.av_screen_snapshot();
                        self.enter_page(Page::CnAv);
                        self.cursor = 6;
                        return true;
                    }
                    _ => return false,
                }
            }
            Page::Start => {
                if a == Action::Confirm {
                    // → the intro movie, then the main menu [M P_INTRO 0x209e88 plays 0x07370048
                    // then 0x07380048 (non-looping) and only then opens the next page; the driver's
                    // observed order Title → movie → Main]. The page flips at once (menus_flow's
                    // contract); the overlay shows the movie fullscreen until the state pops.
                    self.flow.play_movie(INTRO_TICKS);
                    self.intro_playing = true;
                    self.intro_index = 0;
                    self.enter_page(Page::Main);
                    self.cursor = 0;
                    return true;
                }
            }
            Page::Main => match (a, self.cursor) {
                (Action::Up, _) => { self.cursor = (self.cursor + self.rows() - 1) % self.rows(); return self.focus_moved(); }
                (Action::Down, _) => { self.cursor = (self.cursor + 1) % self.rows(); return self.focus_moved(); }
                // NightFire → Select Codename [M C_GONIGHTFIRE 0x2169b8:36-37 opens the next front-end
                // page; the driver's observed order NightFire → Select Codename → Select Difficulty →
                // Single Player].
                (Action::Confirm, 0) => { self.enter_page(Page::Codename); self.cursor = 0; return true; }
                // Multiplayer → the Join page [M C_GOMULTIPLAYER 0x216a78 sends the delayed
                // msg 0x44 → 0x40000019 after 0x13 frames; the port flips at once like the other
                // page changes].
                (Action::Confirm, 1) => { self.enter_page(Page::MpJoin); self.cursor = 0; return true; }
                // Codenames → Edit Codename, on the Default row [M C_GOCODENAMES 0x216b38:37 → page 0x4000001b; L PCSX2 m29].
                (Action::Confirm, 2) => { self.enter_page(Page::CnSelect); self.cursor = 1; return true; }
                (Action::Back, _) => { self.enter_page(Page::Start); self.cursor = 0; return true; }
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
                    if self.cursor == 0 {
                        // Row 0 "New Codename" opens the game's name entry [M C_SBNFCN
                        // 0x20b390:46-49 — Menu_MapDefaultCodename(0,0,0xff) then
                        // Menu_ChangePageCloseIris(0x40000020) with the return-page send].
                        self.cn_open(PAGE_CODENAME);
                        return true;
                    }
                    if self.cursor == 1 {
                        if !self.confirming_default_profile {
                            self.confirming_default_profile = true;
                        } else {
                            self.confirming_default_profile = false;
                            self.enter_page(Page::Difficulty);
                            self.cursor = 1; // Agent [M the game's default difficulty row is the middle one — the wheel starts centred]
                        }
                        return true;
                    }
                    return false;
                }
                Action::Back => {
                    if self.confirming_default_profile { self.confirming_default_profile = false; return true; }
                    self.enter_page(Page::Main); self.cursor = 0; return true;
                }
                _ => {}
            },
            Page::Difficulty => match a {
                // The difficulty wheel clamps too: the same Scroll_Update rule (0x2221b20:186-210,
                // wrap only under msg 0x69 [M 0x222810:136-139]) governs every wheel scroll, and the
                // front-end menu-def carries NO msg 0x69 on any control [L the tag-stream dump,
                // 891 messages, zero 0x69]; C_SBNFDFCTY 0x216ca8:25-28 sets this wheel's range
                // [0,2]. The port's old % wrap bounced Easy↔Little Leak on Up.
                Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                Action::Down if self.cursor + 1 < self.rows() => { self.cursor += 1; return self.focus_moved(); }
                Action::Up | Action::Down => return false,
                Action::Confirm => {
                    self.difficulty = self.cursor.min(DIFFICULTIES.len() - 1);
                    // → the Single Player list [M C_SBNFDFCTY 0x216ca8:10-17 Menu_ChangePageCloseIris
                    // 0x4000001c — the driver's observed order Select Difficulty → Single Player].
                    self.enter_page(Page::MissionSelect);
                    self.cursor = self.mission.min(self.first_unlocked());
                    return true;
                }
                Action::Back => { self.enter_page(Page::Codename); self.cursor = 0; return true; }
                _ => {}
            },
            Page::MissionSelect => match a {
                Action::Left | Action::Right => {}   // remapped to Back / Confirm before the page match
                Action::Up => { self.cursor = self.step(-1); return self.focus_moved(); }
                Action::Down => { self.cursor = self.step(1); return self.focus_moved(); }
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
                Action::Side => { self.enter_page(Page::Dossier); self.cursor = 0; return true; }   // □ Dossier
                Action::Back => { self.enter_page(Page::Difficulty); self.cursor = self.difficulty; return true; }
            },
            Page::Dossier => match a {
                // The wheel CLAMPS, it never wraps [M Scroll_Update 0x2221b20:186-194 (Down at the
                // max) / :202-210 (Up at the min) wrap only when the scroll's +0x144 flag is set,
                // and that flag is set only by msg 0x69 (Scroll_SendMessage 0x222810:136-139), which
                // nothing sends to 0x1000010c; msg 0x2e rejects out-of-range indices]. The port's
                // old % wrap teleported Records → Weapons on Up.
                Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                Action::Down if self.cursor + 1 < self.rows() => { self.cursor += 1; return self.focus_moved(); }
                Action::Up | Action::Down => return false,
                Action::Confirm => {
                    // The wheel's row opens its page [M C_SBDOSSIER 0x216eb0:12-31 — index 0 →
                    // 0x4000003a Records, 1 → 0x4000003b Rewards, 2 → 0x4000003c Gadgets,
                    // 3 → 0x4000003d Weapons; the port draws Records, the rest are reported].
                    if self.cursor == 0 { self.enter_page(Page::Records); self.cursor = self.mission.min(CAMPAIGN.len() - 1); return true; }
                    return false;
                }
                Action::Back | Action::Side => { self.enter_page(Page::MissionSelect); return true; }
                _ => {}
            },
            Page::Records => match a {
                Action::Left | Action::Right => {}   // remapped to Back / Confirm before the page match
                // The mission list walks the campaign [M the list control 0x10000174 is filled from
                // sp_level by Menu_AddItemsToControl (P_DSRECORDS 0x217248:11-13); a row's selection
                // drives C_RBDSRECORDS 0x20bf00's binds].
                Action::Up => { self.cursor = (self.cursor + CAMPAIGN.len() - 1) % CAMPAIGN.len(); return self.focus_moved(); }
                Action::Down => { self.cursor = (self.cursor + 1) % CAMPAIGN.len(); return self.focus_moved(); }
                Action::Back | Action::Confirm | Action::Side => { self.enter_page(Page::Dossier); self.cursor = 0; return true; }
            },
            Page::Options => match a {
                Action::Up => { self.cursor = (self.cursor + 2) % 3; return true; }
                Action::Down => { self.cursor = (self.cursor + 1) % 3; return true; }
                Action::Back | Action::Confirm if self.cursor == 2 => { self.enter_page(Page::Main); self.cursor = 2; return true; }
                _ => {}
            },
            Page::Pause => {
                // The YES/NO confirm list runs while either confirm is armed [M 0x204408's msg-0x4b
                // d1=0xd case reads the list's row (msg 0x34) and 0 = YES executes].
                if self.confirming_restart || self.confirming_quit {
                    match a {
                        Action::Left | Action::Right => {}   // remapped to Back / Confirm before the page match
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
                                // The page closes and play resumes on the reloaded map (the menu manager is deleted
                                // with the restart [M 0x204408:139-141]); leaving Page::Main up kept the main menu
                                // over the reloaded level.
                                self.closed = true;
                                return true;
                            }
                            // YES on QUIT [M 0x204408:142-152 — +0x28 = 0x4000001c (the SP quit page),
                            // ResetMap_LevelToLoad(0x7000048, 0, 0), the fade push and the delete].
                            self.flow.page = PAGE_RESULTS_QUIT_TARGET;
                            self.requested_level = Some(FRONTEND_LEVEL);
                            self.flow.reset_map_level_to_load(FRONTEND_LEVEL, false, true);
                            self.enter_page(Page::Main);
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
                // objective-hints: the ✕ hint box is modal (option-box page 0x4000002f pushed
                // over the pause [M Menu_CreateOptionBox 0x1f9608:45]). Its OK row (0x4b) or a
                // cancel (0x6d) pops it [M C_LBMSGOPTIONS_Handler 0x2022a0]; the page handler
                // then re-shows the list on msg 0x50 [M P_PAUSE_Handler 0x203c28:19-24 —
                // option_return != 0 → control 8 0x2b(0); the ok-only box returns 3 on OK and 4
                // on cancel, both non-zero] — the port just clears the field. Everything else
                // idles.
                if self.pause_hint.is_some() {
                    match a {
                        Action::Confirm | Action::Back => { self.pause_hint = None; return true; }
                        _ => return false,
                    }
                }
                // objective-hints: the OBJECTIVES tab's own list — ~V scrolls ITS selection and
                // ✕ shows the selected objective's hint (control 1's 0x4b activation [M
                // 0x204408 0x205b98-0x205c34]; the MISSION rows' cursor is idle here).
                if self.pause_tab == 1 {
                    let n = self.pause_objectives.len();
                    match a {
                        Action::Confirm => {
                            if n == 0 { return false; }
                            let sel = self.pause_objectives_sel.min(n - 1);
                            let hint = self.pause_objectives[sel].hint;
                            // No-hint records (−1 = 0xffffffff) show label 0x31d instead
                            // [M 0x204408 `if (iStack_304 == -1) iStack_304 = 0x31d`].
                            self.pause_hint = Some(if hint == 0xffff_ffff { 0x31d } else { hint });
                            return true;
                        }
                        Action::Up if n > 0 => { if self.pause_objectives_sel > 0 { self.pause_objectives_sel -= 1; } return true; }
                        Action::Down if n > 0 => { if self.pause_objectives_sel + 1 < n { self.pause_objectives_sel += 1; } return true; }
                        _ => {}
                    }
                }
                match a {
                    Action::Left | Action::Right => {}   // remapped to Back / Confirm before the page match
                    // ~H Next: the tab cycle [L the page hint 0x201 "~A Select ~V Scroll ~H Next";
                    // the four tabs are the page's type-1 controls]. Every entry re-populates the
                    // tab — the OBJECTIVES list re-selects its LAST row (msg 0x1d = numVis−1
                    // [M 0x204408 0x204840-0x204850]).
                    Action::Side => {
                        self.pause_tab = (self.pause_tab + 1) % 4;
                        if self.pause_tab == 1 {
                            self.pause_objectives_sel = self.pause_objectives.len().saturating_sub(1);
                        }
                        return true;
                    }
                    Action::Up => { self.cursor = (self.cursor + 2) % 3; return self.focus_moved(); }
                    Action::Down => { self.cursor = (self.cursor + 1) % 3; return self.focus_moved(); }
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
                Action::Left | Action::Right => {}   // remapped to Back / Confirm before the page match
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
                    self.enter_page(Page::MissionSelect);
                    self.flow.page = PAGE_MISSION_SELECT;
                    return true;
                }
            },
            Page::Results => match a {
                Action::Left | Action::Right => {}   // remapped to Back / Confirm before the page match
                Action::Side => {}
                Action::Up | Action::Down => { self.cursor = (self.cursor + 1) % 2; return self.focus_moved(); }
                Action::Confirm => {
                    // The list's item VALUES [L page 0x40000042: REPLAY = 0, QUIT = 2; M 0x216618 msg 0x35].
                    match [0usize, 2][self.cursor.min(1)] {
                        // The arms in row order [M P_ENDMISSION_Handler 0x216618:19-27]:
                        // row 0 → `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` + `Player_RamLoad(Bond, 1)`
                        // — the CURRENT level, WARM, on the entry checkpoint (fix-restart-target; the old
                        // row 0 sent arm 1). Row 1 → `ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)`
                        // — BaseMap, COLD. Row 2 → `GameState+0x28 = 0x4000001c` + 0x7000048.
                        0 => {
                            let level = self.flow.current_level;
                            self.requested_level = Some(level);
                            self.flow.reset_map_level_to_load(level, true, true);
                            self.enter_page(Page::Main);
                            return true;
                        }
                        1 => {
                            // The mission's base map: the campaign row's level code (fix-menus2's `sp_level`
                            // table; `Mission_BaseMapHCode`).
                            let level = CAMPAIGN[self.mission].3;
                            self.requested_level = Some(level);
                            self.flow.reset_map_level_to_load(level, false, true);
                            self.enter_page(Page::Main);
                            return true;
                        }
                        2 => {
                            self.flow.page = PAGE_RESULTS_QUIT_TARGET;
                            self.requested_level = Some(FRONTEND_LEVEL);
                            self.flow.reset_map_level_to_load(FRONTEND_LEVEL, false, true);
                            self.enter_page(Page::Main);
                            self.cursor = 0;
                            return true;
                        }
                        _ => {}
                    }
                }
                Action::Back => { self.cursor = 1; return self.focus_moved(); }
            },
            // ── mp-menu-flow: the multiplayer front end ──
            // Join Game [M P_MPJOIN 0x20ca00 + C_RBMPSTART 0x20d048 + C_RBMPCNAME 0x20d448].
            // Keyboard = agent 1 only: the game joins agents 2-4 through their own pads (msg 0x6a
            // with the controller id), which the port has none of. The panel cursor picks the
            // agent; Confirm is that agent's ✕. The port folds the game's two ✕ presses after the
            // codename (C_RBMPCNAME's 0x4b then C_RBMPSTART's 0x4b on state 5) into: Confirm on
            // state 1 chooses the codename (state 5), Confirm again readies (mpjoin+1).
            Page::MpJoin => {
                let slot = self.cursor.min(3);
                let can_join = slot == 0;   // keyboard = agent 1 [S — the task's scope; see mp_join doc]
                match a {
                    Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                    Action::Down if self.cursor + 1 < 4 => { self.cursor += 1; return self.focus_moved(); }
                    Action::Left if can_join && self.mp_join[0].state == 1 => {
                        // ◀ ▶ move the codename list [M C_RBMPCNAME's list control 0x1000022b; the
                        // rows are the two defaults ("Player N" via Menu_MapDefaultCodename) then
                        // the card's codenames [M Menu_PutCodenamesIntoControl 0x1f79c0]].
                        self.mp_join[0].codename_row = self.mp_join[0].codename_row.saturating_sub(1);
                        return true;
                    }
                    Action::Right if can_join && self.mp_join[0].state == 1 => {
                        self.mp_join[0].codename_row = (self.mp_join[0].codename_row + 1).min(self.mp_codename_rows() - 1);
                        return true;
                    }
                    Action::Confirm if can_join => {
                        match self.mp_join[0].state {
                            // ✕ on the idle panel: join, prompt "Choose Codename" (0x37b) [M
                            // C_RBMPSTART 0x20d048's 0x4b state-0 arm: mpjoin[slot·0x10] = 1,
                            // mp_stuff+0x360 = 1].
                            0 => { self.mp_join[0].state = 1; self.mp_join[0].joined = true; return true; }
                            // ✕ on the codename list: pick it [M C_RBMPCNAME 0x20d448's 0x4b: the
                            // template rows 0/1 map to the default codename, the name lands in
                            // MPSettings+slot·0x30, mp_stuff+0x360 = 5]. The port's rows beyond
                            // the two defaults would open the game's keyboard page (not ported —
                            // the join flow's defaults are what Quick Game itself uses).
                            1 => {
                                self.mp_join[0].codename = self.mp_codename_at(self.mp_join[0].codename_row);
                                self.mp_join[0].state = 5;
                                return true;
                            }
                            // ✕ once joined: "Player Ready" [M C_RBMPSTART's 0x4b state-5 arm sets
                            // mpjoin+1 = 1 and fades; P_MPJOIN's msg-0x50 arm checks
                            // Menu_MPAreWeReady → page 0x4000001a].
                            5 if !self.mp_join[0].ready => { self.mp_join[0].ready = true; self.enter_page(Page::MpScenario); self.cursor = 0; return true; }
                            _ => {}
                        }
                    }
                    // △ leaves [M C_RBMPSTART's 0x6b arm un-joins state-1 agents; C_RBMPCNAME's
                    // 0x6b un-chooses codenames].
                    Action::Back if can_join && self.mp_join[0].state != 0 => {
                        self.mp_join[0] = MpJoinAgent::default();
                        return true;
                    }
                    _ => {}
                }
            }
            // Select Scenario [M P_MPSCENARIO 0x217348 + C_SBMPSCEN 0x20c600].
            Page::MpScenario => {
                match a {
                    Action::Up if self.mp_scenario_row > 0 => { self.mp_scenario_row -= 1; return true; }
                    Action::Down if self.mp_scenario_row + 1 < MP_SCENARIO_ROWS.len() => { self.mp_scenario_row += 1; return true; }
                    Action::Back => { self.enter_page(Page::Main); self.cursor = 1; return true; }  // [M P_MPSCENARIO:29 → 0x40000002]
                    Action::Confirm => {
                        let row = self.mp_scenario_row.min(MP_SCENARIO_ROWS.len() - 1);
                        // Locked rows refuse [M C_SBMPSCEN:40-42 — the +0x10 gate, unless
                        // menu_unlock_everything].
                        if self.mp_scenario_locked(row) { return false; }
                        // Row 0 = Quick Game: the fixed quick-start config then straight to the
                        // confirm page [M C_SBMPSCEN:48-145].
                        if MP_SCENARIO_ROWS[row].3 == 0 { self.mp_quick_start(); return true; }
                        // Confirm stores the scenario word [M disasm 0x20c9c8 `sw 0x4944($t0)`]
                        // and opens the map page [M 0x20c9a8-0x20c9b4 → 0x40000013].
                        self.mp_scenario = MP_SCENARIO_ROWS[row].3;
                        self.enter_page(Page::MpMap);
                        self.mp_map_row = self.mp_map_row.min(MP_MAP_ROWS.len() - 1);
                        return true;
                    }
                    _ => {}
                }
            }
            // Select Map [M P_MPMAP 0x217468 + C_SBMPMAP 0x217538].
            Page::MpMap => {
                match a {
                    Action::Up if self.mp_map_row > 0 => { self.mp_map_row -= 1; return true; }
                    Action::Down if self.mp_map_row + 1 < MP_MAP_ROWS.len() => { self.mp_map_row += 1; return true; }
                    Action::Back => { self.enter_page(Page::MpScenario); return true; }   // [M the wheel's back — C_SBMPSCEN's page again; the game's own back is msg 0x63]
                    Action::Confirm => {
                        let row = self.mp_map_row.min(MP_MAP_ROWS.len() - 1);
                        // [M C_SBMPMAP:12-14's lock gate — a no-op on retail data (all +0x10 = 1).]
                        // Confirm writes the level code to MPSettings+0x1A8 AND GameState+0x08
                        // [M disasm 0x217654/0x21765c] and opens the setup page [M
                        // 0x217638-0x217644 → 0x40000051].
                        self.mp_level = MP_MAP_ROWS[row].3;
                        self.enter_page(Page::MpSetup);
                        // P_MPSETUP picks the prompt from the scenario's team bit [M 0x20d7a8].
                        self.mp_setup_state = if self.mp_scenario & 0x2000_0000 != 0 { 2 } else { 3 };
                        self.mp_setup_sel = 0;
                        return true;
                    }
                    _ => {}
                }
            }
            // The per-agent team/character page [M P_MPSETUP 0x20d7a8 + C_RBMPSETUP 0x20de58].
            // Keyboard = agent 1: ◀ ▶ move the pick, ✕ confirms (state 3 → 4 team, 2 → 3 character
            // — the game's C_RBMPSETUP:0x4b steps 2 → 3 and 3 → 4).
            Page::MpSetup => {
                let n = if self.mp_setup_state == 2 { 2usize } else { MP_CHARACTERS.len() };
                match a {
                    Action::Left if self.mp_setup_sel > 0 => { self.mp_setup_sel -= 1; return true; }
                    Action::Right if self.mp_setup_sel + 1 < n => { self.mp_setup_sel += 1; return true; }
                    Action::Confirm => {
                        if self.mp_setup_state == 2 {
                            // The team list: the selection IS the team [M P_MPSETUP's items
                            // Phoenix(0x1c7) = 0 / MI6(0x1c8) = 1 → mpjoin+4].
                            self.mp_join[0].team = self.mp_setup_sel as u32;
                            self.mp_setup_state = 3;
                            self.mp_setup_sel = 0;
                            return true;
                        }
                        // The character list [M C_RBMPSETUP's msg-0x35 → mp_characters_small row;
                        // mpjoin+8 = the id]. All picked → the options hub (P_MPOPTIONS expects
                        // 0x40000051 as its source [M 0x217678:12]).
                        self.mp_join[0].skin = self.mp_setup_sel as u32;
                        self.mp_setup_state = 4;
                        self.enter_page(Page::MpOptions);
                        self.cursor = 0;
                        self.mp_options_row = 0;
                        return true;
                    }
                    Action::Back => { self.enter_page(Page::MpMap); return true; }
                    _ => {}
                }
            }
            // The options hub [M P_MPOPTIONS 0x217678 + C_SBMPOPTIONS 0x20e818].
            Page::MpOptions => {
                match a {
                    Action::Up if self.mp_options_row > 0 => { self.mp_options_row -= 1; return true; }
                    Action::Down if self.mp_options_row + 1 < MP_OPTIONS_ROWS.len() => { self.mp_options_row += 1; return true; }
                    Action::Back => { self.enter_page(Page::MpSetup); return true; }
                    Action::Confirm => {
                        let row = self.mp_options_row.min(MP_OPTIONS_ROWS.len() - 1);
                        // AI Bots is locked on Ravine [M P_MPOPTIONS 0x217678:20-25].
                        if row == 1 && !self.unlock_all && self.mp_level == 0x0700_004b { return false; }
                        match row {
                            // Continue: the start checks then the confirm page [M C_SBMPOPTIONS
                            // 0x20e818:50-130 — the team/FFA checks, then 0x40000049 or the
                            // error box].
                            0 => { return self.mp_start_checks(); }
                            1 => { self.enter_page(Page::MpBots); self.mp_bots_row = self.mp_bot_slot + 1; return true; }  // [M :116 → 0x40000027; the cursor = mp_stuff[0x37a]+1]
                            2 => { self.enter_page(Page::MpRules); return true; }    // [M :119 → 0x40000014]
                            3 => { self.enter_page(Page::MpPlayerMods); return true; }  // [M :122 → 0x40000017]
                            _ => { self.enter_page(Page::MpEnviroMods); return true; }  // [M :125 → 0x40000028]
                        }
                    }
                    _ => {}
                }
            }
            // Game Rules [M P_MPRULES 0x20f790: two list rows; Confirm reads both and closes].
            Page::MpRules => {
                match a {
                    Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                    Action::Down if self.cursor + 1 < 2 => { self.cursor += 1; return self.focus_moved(); }
                    Action::Left => { self.mp_rules_step(-1); return true; }
                    Action::Right => { self.mp_rules_step(1); return true; }
                    // Confirm AND Back close the page (the game's confirm reads both rows; the
                    // 0x5f close is the same manager pop either way [M :14 → Manager 0x5f]).
                    Action::Confirm | Action::Back => { self.enter_page(Page::MpOptions); self.cursor = 0; return true; }
                    _ => {}
                }
            }
            // Player Mods [M P_MPPLAYERMODS 0x2104d0: five list rows, Confirm reads all five].
            Page::MpPlayerMods => {
                match a {
                    Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                    Action::Down if self.cursor + 1 < 5 => { self.cursor += 1; return self.focus_moved(); }
                    Action::Left => { self.mp_playermods_step(-1); return true; }
                    Action::Right => { self.mp_playermods_step(1); return true; }
                    Action::Confirm | Action::Back => { self.enter_page(Page::MpOptions); self.cursor = 0; return true; }
                    _ => {}
                }
            }
            // Enviro-Mods [M P_MPENVIROMODS 0x210030: five list rows].
            Page::MpEnviroMods => {
                match a {
                    Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                    Action::Down if self.cursor + 1 < 5 => { self.cursor += 1; return self.focus_moved(); }
                    Action::Left => { self.mp_envmods_step(-1); return true; }
                    Action::Right => { self.mp_envmods_step(1); return true; }
                    Action::Confirm | Action::Back => { self.enter_page(Page::MpOptions); self.cursor = 0; return true; }
                    _ => {}
                }
            }
            // The AI Bots list [M P_MPBOTS 0x217790 + C_SBBOTS 0x210a08].
            Page::MpBots => {
                match a {
                    Action::Up if self.mp_bots_row > 0 => { self.mp_bots_row -= 1; return true; }
                    Action::Down if self.mp_bots_row + 1 < MP_BOTS_ROWS.len() => { self.mp_bots_row += 1; return true; }
                    Action::Back => { self.enter_page(Page::MpOptions); self.cursor = 1; return true; }  // row 0 = Continue [M C_SBBOTS:15 → Manager 0x5f]
                    Action::Confirm => {
                        // Row 0 closes [M C_SBBOTS:15]; rows 1-4 pick the slot [M :20 `mp_stuff[0x37a]
                        // = sel-1`] and open the bot choose wheel [M :23 → 0x4000003f].
                        if self.mp_bots_row == 0 { self.enter_page(Page::MpOptions); self.cursor = 1; return true; }
                        self.mp_bot_slot = self.mp_bots_row - 1;
                        self.mp_bot_char = self.mp_bots[self.mp_bot_slot].character as usize;
                        self.enter_page(Page::MpBotChoose);
                        return true;
                    }
                    _ => {}
                }
            }
            // The bot character wheel [M P_MPBOTCHOOSE 0x210cd8 + C_SBMPBTCHOOSE 0x210f08].
            Page::MpBotChoose => {
                match a {
                    Action::Up if self.mp_bot_char > 0 => { self.mp_bot_char -= 1; return true; }
                    Action::Down if self.mp_bot_char + 1 < MP_CHARACTERS.len() => { self.mp_bot_char += 1; return true; }
                    Action::Back => { self.enter_page(Page::MpBots); self.mp_bots_row = self.mp_bot_slot + 1; return true; }
                    Action::Confirm => {
                        let ch = self.mp_bot_char.min(MP_CHARACTERS.len() - 1);
                        // Locked characters refuse [M C_SBMPBTCHOOSE:18-20's unlock gate].
                        if self.mp_character_locked(ch) { return false; }
                        // [M C_SBMPBTCHOOSE:26-43 — record+0x10 = the character, +0x0F team =
                        // Menu_IsBotGood (the preset's side), the name into MPSettings+slot·0x30+0xC0,
                        // +0x0E enabled = 1; then page 0x4000002c.]
                        let b = &mut self.mp_bots[self.mp_bot_slot.min(3)];
                        b.character = ch as u32;
                        b.team_mi6 = MP_CHARACTER_SIDE_MI6[ch];
                        b.enabled = true;
                        b.dirty = false;
                        let (skill, aim, hp, speed, r1, r2, trait_id) = MpBot::preset(ch);
                        (b.skill, b.aim, b.hit_points, b.speed, b.recovery1, b.recovery2, b.trait_id) = (skill, aim, hp, speed, r1, r2, trait_id);
                        self.enter_page(Page::MpBotSetup);
                        self.cursor = 0;
                        return true;
                    }
                    _ => {}
                }
            }
            // The per-bot stats [M P_MPBOTSETUP 0x211270: eight list rows; the reward characters
            // (>= 15) lock all seven stat rows [M :23-48] and Confirm still closes [M :138-169]].
            Page::MpBotSetup => {
                match a {
                    Action::Up if self.cursor > 0 => { self.cursor -= 1; return self.focus_moved(); }
                    Action::Down if self.cursor + 1 < 8 => { self.cursor += 1; return self.focus_moved(); }
                    Action::Left => { self.mp_botsetup_step(-1); return true; }
                    Action::Right => { self.mp_botsetup_step(1); return true; }
                    // [M :138-169 — the values store back, +0x11 dirty = 1, Manager 0x5f twice =
                    // back to the Bots list.]
                    Action::Confirm | Action::Back => {
                        self.mp_bots[self.mp_bot_slot.min(3)].dirty = true;
                        self.enter_page(Page::MpBots);
                        self.mp_bots_row = self.mp_bot_slot + 1;
                        self.cursor = 0;
                        return true;
                    }
                    _ => {}
                }
            }
            // The match summary [M P_MPCONFIRM 0x20ec00: the enter builds the lines, Confirm
            // starts the match].
            Page::MpConfirm => {
                if a == Action::Confirm { return self.mp_confirm_start(); }
                if a == Action::Back { self.enter_page(Page::MpOptions); self.cursor = 0; return true; }
            }
            // The debriefing [M P_MPDEBRIEFING 0x212198 + C_MPDBG 0x217860: Confirm restores the
            // MPSettings backup and returns to the scenario page; the rematch arm (msg 0x5d)
            // reloads the level — the port has no in-match host loop for it yet (reported)].
            Page::MpDebriefing => {
                if a == Action::Confirm { self.enter_page(Page::MpScenario); self.mp_scenario_row = 0; return true; }
            }
            // credits-data: the credits take no input — the handler has no input cases at all
            // (the roll cannot be skipped) [M P_CREDITS_Handler 0x2152b0's message cases].
            Page::Credits => {}
        }
        false
    }

    /// secret-codes: press the grid cell the cursor stands on. Characters append (the 8-char cap
    /// [M 0x213a98:29-31]), Delete truncates [M :39-43], Space appends ' ' [M :28-36/:75-85],
    /// Submit flags the drain [M :67-74 — the handler calls Menu_SpecialCodenameCheck; the port
    /// splits it so the host supplies the Progress, like `results_enter`].
    pub fn secret_press(&mut self) -> bool {
        self.secret.accepted = false;
        match SECRET_GRID[self.secret.cursor.min(SECRET_GRID.len() - 1)] {
            SecretKey::Char(c) => {
                if self.secret.typed.chars().count() >= 8 { return false; }
                self.secret.typed.push(c);
                true
            }
            SecretKey::Space => {
                if self.secret.typed.chars().count() >= 8 { return false; }
                self.secret.typed.push(' ');
                true
            }
            SecretKey::Delete => { self.secret.typed.pop(); true }
            SecretKey::Submit => { self.secret.pending_submit = true; true }
        }
    }

    /// secret-codes: the host drains a pending Submit here — `Menu_SpecialCodenameCheck`
    /// 0x200238's port applies the unlock to the live progress; `true` opens the 0x100029d box
    /// [M 0x213a98:67-73], `false` is the game's silent miss.
    pub fn secret_submit(&mut self, progress: &mut nf_game::progress::Progress) -> bool {
        if !self.secret.pending_submit { return false; }
        self.secret.pending_submit = false;
        let hit = nf_game::secret_codes::submit(progress, &self.secret.typed);
        self.secret.accepted = hit;
        hit
    }

    /// The menu-def page the screen is: `Page::Results` is TWO pages — the fail menu 0x40000042 (a mission
    /// archive's own menu-def, no aux movie: it draws over the paused, red-faded frame [L 07000005 08000001]) and the
    /// results page 0x40000036 (the front-end's, aux 0x07350048).
    pub fn page_id(&self) -> u32 {
        if self.page == Page::Results && self.outcome == Outcome::Failed { PAGE_FAIL_MENU } else { self.page.id() }
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

    // ── mp-menu-flow: the multiplayer front end's helpers ────────────────────────────────────

    /// mp_scenario[row]+0x10 with `menu_unlock_everything`: the shipped flag is 1 for the
    /// always-open rows and 0 for the reward rows, which `Menu_UnlockMPSettings` 0x1ff698:53-104
    /// re-derives from the campaign rewards (the port carries the reward ids in the rows' last
    /// field and gates on the profile's rewards — None here means a fresh profile [S the reward
    /// tracking is the results page's job, not yet wired to MP]).
    pub fn mp_scenario_locked(&self, row: usize) -> bool {
        let (_, _, _, _, reward) = MP_SCENARIO_ROWS[row.min(MP_SCENARIO_ROWS.len() - 1)];
        reward != 0 && !self.unlock_all
    }

    /// mp_characters[row]+0x10 with the same unlock [M Menu_UnlockMPSkins 0x1ff2f8:113-187].
    pub fn mp_character_locked(&self, row: usize) -> bool {
        MP_CHARACTERS[row.min(MP_CHARACTERS.len() - 1)].3 != 0 && !self.unlock_all
    }

    /// The Join page's codename list length: the two defaults then the card's rows [M
    /// Menu_PutCodenamesIntoControl 0x1f79c0:15-23 — def_codename's two rows, then the saves].
    pub fn mp_codename_rows(&self) -> usize { 2 + self.profiles.len() }

    /// The codename at `row` [M C_RBMPCNAME 0x20d448: the template rows map through
    /// Menu_MapDefaultCodename and the name becomes "Player N" — Txt 0x1c3 + slot+1 through
    /// sprintf "%s %d" @0x30d388; a card row is the codename itself].
    pub fn mp_codename_at(&self, row: usize) -> String {
        match row {
            0 | 1 => format!("Player {}", self.cursor.min(3) + 1),
            r => self.profiles.get(r - 2).cloned().unwrap_or_else(|| format!("Player {}", self.cursor.min(3) + 1)),
        }
    }

    /// P_MPRULES's row-1 unit: "Lives" for the death-limit scenario (Top Agent 0x10), "Points"
    /// otherwise [M P_MPRULES 0x20f790:78-95].
    pub fn mp_rules_lives(&self) -> bool { self.mp_scenario == 0x10 }

    /// P_MPRULES's Duration list: 1..10, 15, 20, 30, 45, 60 minutes then Unlimited (-1) [M
    /// 0x20f790:20-42 — the static strings "1".."60" at 0x30d3f8, then 0x3ca "Unlimited" at
    /// index -1]; the Points list goes to 120 [M :65-76]. Steps by `dir`.
    fn mp_rules_step(&mut self, dir: i32) {
        const TIMES: [i32; 13] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 20, 30];
        if self.cursor.min(1) == 0 {
            // Duration: the list walks 1..60 then Unlimited [M the strings' order].
            let unlimited = self.mp_time_limit == -1;
            let idx = TIMES.iter().position(|&t| t == self.mp_time_limit);
            self.mp_time_limit = match (idx, dir) {
                (None, -1) => 30,                       // from Unlimited back to 30
                (None, _) => 1,                          // fresh: 1 minute
                (Some(i), 1) if i + 1 < TIMES.len() => TIMES[i + 1],
                (Some(12), 1) => -1,                     // 30 → 45 → 60 → Unlimited
                (Some(i), -1) => TIMES[i.saturating_sub(1)],
                (_, _) => self.mp_time_limit,
            };
            if unlimited && dir == -1 { self.mp_time_limit = 30; }
        } else {
            // Points/Lives: 1..120, no Unlimited [M :65-76].
            self.mp_frag_limit = (self.mp_frag_limit + dir).clamp(1, 120);
        }
    }

    /// P_MPPLAYERMODS's rows: friendly fire (On/Off), weapon set (0..10), +0x1BC, +0x1C8 (On/Off),
    /// radar (On/Off) [M 0x2104d0:12-16, 20-61].
    fn mp_playermods_step(&mut self, dir: i32) {
        match self.cursor.min(4) {
            0 => { self.mp_friendly_fire = (self.mp_friendly_fire as i32 ^ 1) as u32; }   // the two-item list wraps
            1 => { self.mp_weapon_set = (self.mp_weapon_set as i32 + dir).rem_euclid(11) as u32; }
            2 => { self.mp_player_mods[0] = (self.mp_player_mods[0] as i32 ^ 1) as u32; }
            3 => { self.mp_player_mods[1] = (self.mp_player_mods[1] as i32 ^ 1) as u32; }
            _ => { self.mp_radar = (self.mp_radar as i32 ^ 1) as u32; }
        }
    }

    /// P_MPENVIROMODS's rows: view distance (Near 0/Far 1/Random 2), +0x1B8, +0x1D4 (the
    /// reward-locked row), +0x1D0 (On/Off), vehicles (Off 0/Tanks 1/Helicopters 2/Random 3) [M
    /// 0x210030:12-16, 20-27, 35-61].
    fn mp_envmods_step(&mut self, dir: i32) {
        match self.cursor.min(4) {
            0 => { self.mp_envmods[0] = (self.mp_envmods[0] as i32 + dir).rem_euclid(3) as u32; }
            1 => { self.mp_envmods[1] = (self.mp_envmods[1] as i32 ^ 1) as u32; }
            2 => { self.mp_envmods[2] = (self.mp_envmods[2] as i32 ^ 1) as u32; }
            3 => { self.mp_envmods[3] = (self.mp_envmods[3] as i32 ^ 1) as u32; }
            _ => { self.mp_envmods[4] = (self.mp_envmods[4] as i32 + dir).rem_euclid(4) as u32; }
        }
    }

    /// P_MPBOTSETUP's rows in page order (the y order of MP_BOT_SETUP_LABELS): Enabled, Speed,
    /// Trait, Aim, Skill, Hit points, Recovery 1, Recovery 2 — the value lists of P_MPBOTSETUP
    /// 0x211270:17-135.
    fn mp_botsetup_step(&mut self, dir: i32) {
        let b = &mut self.mp_bots[self.mp_bot_slot.min(3)];
        let (list, len): (&[i32], i32) = match self.cursor.min(7) {
            0 => return,                                              // Enabled is not a stat row here (the game's own list Yes/No)
            1 => (&[0, 1, 2], 3),                                     // Speed
            2 => (&[0, 1, 2, 3, 4, 5, 6, 7, 8], 9),                   // Trait 0..8
            3 => (&[2, 3, 4], 3),                                     // Aim
            4 => (&[1, 3, 5, 8], 4),                                  // Skill (Very Good..Poor)
            5 => (&[50, 75, 100, 125, 150, 175, 200, 250, 300], 9),   // Hit points
            6 => (&[50, 75, 100, 125, 150, 175, 200], 7),             // Recovery 1
            _ => (&[50, 75, 100, 125, 150, 175, 200], 7),             // Recovery 2
        };
        let _ = len;
        let cur = match self.cursor.min(7) {
            1 => b.speed, 2 => b.trait_id, 3 => b.aim, 4 => b.skill,
            5 => b.hit_points, 6 => b.recovery1, _ => b.recovery2,
        };
        let i = list.iter().position(|&v| v == cur).unwrap_or(0);
        let nv = list[(i as i32 + dir).rem_euclid(list.len() as i32) as usize];
        match self.cursor.min(7) {
            1 => b.speed = nv, 2 => b.trait_id = nv, 3 => b.aim = nv, 4 => b.skill = nv,
            5 => b.hit_points = nv, 6 => b.recovery1 = nv, _ => b.recovery2 = nv,
        }
    }

    /// C_SBMPOPTIONS row 0's start checks [M 0x20e818:50-130]. Returns whether the state changed:
    /// on success the page becomes the confirm page; on failure the error-box label lands in
    /// mp_error (the game pops Menu_CreateOptionBox with it).
    pub fn mp_start_checks(&mut self) -> bool {
        self.mp_error = 0;
        let humans = self.mp_join.iter().filter(|a| a.joined).count();
        let bots = self.mp_bots.iter().filter(|b| b.enabled).count();
        let team = self.mp_scenario & 0x2000_0000 != 0;
        if !team {
            // FFA: the game refuses more than one "good" (MI6-side) player [M the uVar1 > 1 arm,
            // Txt 0x1000311; the decomp's counting is thin — the string settles the intent].
            let good = self.mp_join.iter().filter(|a| a.joined && a.team == 1).count();
            if good > 1 { self.mp_error = 0x0100_0311; return false; }
        } else {
            // Team: both sides need someone [M the 0x389 arm with 0x1c7/0x1c8].
            let blue = self.mp_join.iter().filter(|a| a.joined && a.team == 1).count()
                + self.mp_bots.iter().filter(|b| b.enabled && b.team_mi6).count();
            let red = self.mp_join.iter().filter(|a| a.joined && a.team == 0).count()
                + self.mp_bots.iter().filter(|b| b.enabled && !b.team_mi6).count();
            if blue == 0 || red == 0 { self.mp_error = 0x389; return false; }
        }
        // At least two players overall [M the 0x39e arm].
        if humans + bots < 2 { self.mp_error = 0x39e; return false; }
        self.enter_page(Page::MpConfirm);
        self.cursor = 0;
        true
    }

    /// The Quick Game row: the fixed quick-start config then straight to the confirm page [M
    /// C_SBMPSCEN 0x20c600:48-145 + the disassembly's stores at 0x20c72c-0x20c864: time 10,
    /// frags 10, Deathmatch, radar on, no friendly fire, weapon set 0 (Normal), FFA, 3 bots
    /// (Drake/Kiko/Rook, all Phoenix, default stats), the map sampled from mp_level rows 0..6,
    /// the joined players' teams/skins fixed, humans = joined count].
    pub fn mp_quick_start(&mut self) {
        self.mp_time_limit = 10;
        self.mp_frag_limit = 10;
        self.mp_scenario = 1;                 // Deathmatch [M +0x1A4 = 1]
        self.mp_friendly_fire = 0;            // [M +0x198 = 0]
        self.mp_weapon_set = 0;               // [M +0x1B4 = 0]
        self.mp_radar = 1;                    // [M +0x1C4 = 1]
        self.mp_player_mods = [0, 0];         // [M +0x1D4 = 0, +0x1CC = 0]
        self.mp_envmods[0] = 1;               // [M +0x1D0 = 0, +0x1C0 = Far(1)? — the quick start writes 2 at +0x1CC; view distance keeps the Far default]
        self.mp_envmods[1] = 0;
        self.mp_envmods[3] = 0;
        self.mp_envmods[4] = 0;
        // The map sample: `Rand_Random() % 7` over mp_level rows 0..6 — never Ravine [M
        // 0x20c76c-0x20c7b4]. Headless determinism: the shared RNG is not wired here, so the port
        // takes row 0 (Skyrail) and records the gap [S — the game's sample is random].
        self.mp_level = MP_MAP_ROWS[0].3;
        // The joined players: slot 0 → team MI6 (1) / skin Bond (0); the unjoined slots carry the
        // game's placeholder skins 6/7/8 [M disasm 0x20c7f8-0x20c834].
        self.mp_join[0].team = 1;
        self.mp_join[0].skin = 0;
        self.mp_join[0].joined = true;
        self.mp_join[0].state = 5;
        self.mp_join[0].codename = "Player 1".into();
        // The bots: count byte 0x2DEEA9 = 3, MPSettings+0x1B0 = 3; characters 1/3/2 (Drake, Kiko,
        // Rook), teams Phoenix, enabled, fresh stats [M the quick start's bot setup].
        for (slot, ch) in [1usize, 3, 2].iter().enumerate() {
            let b = &mut self.mp_bots[slot];
            b.character = *ch as u32;
            b.team_mi6 = MP_CHARACTER_SIDE_MI6[*ch];
            b.enabled = true;
            b.dirty = false;
            let (skill, aim, hp, speed, r1, r2, trait_id) = MpBot::preset(*ch);
            (b.skill, b.aim, b.hit_points, b.speed, b.recovery1, b.recovery2, b.trait_id) = (skill, aim, hp, speed, r1, r2, trait_id);
        }
        for b in self.mp_bots.iter_mut().skip(3) { b.enabled = false; }
        self.enter_page(Page::MpConfirm);          // [M 0x20c978-0x20c984 → 0x40000049]
        self.cursor = 0;
    }

    /// The joined humans [M P_MPCONFIRM's compaction counts mpjoin != 0].
    pub fn mp_human_count(&self) -> usize { self.mp_join.iter().filter(|a| a.joined).count() }
    /// The enabled bots [M Menu_PrepareBots 0x1f94a0 compacts the enabled records; MPSettings
    /// +0x1B0 = the count, or 0 on Ravine [M :85-88]].
    pub fn mp_bot_count(&self) -> usize {
        if self.mp_level == 0x0700_004b { return 0; }   // [M P_MPOPTIONS's gate + Menu_PrepareBots' zeroing]
        self.mp_bots.iter().filter(|b| b.enabled).count()
    }

    /// The confirm page's Confirm — P_MPCONFIRM 0x20ec00:53-94: `Menu_StoreMPSettings` (the port
    /// has no MP card — the backup/restore pair is the debriefing's own [S]); the joined players'
    /// records compact; `+0x1A0 != -1 → ×60` (minutes → seconds); `+0x1D4 &= 1`; `+0x180 = 1`;
    /// `GameState+0x08 = +0x1A8`; `+0x1AC = joined`; `ResetMap_LevelToLoad(+0x1A8, 0, 0)` +
    /// `GameFlow_PushState(80.0f, 7, 0xff)` + `SFXFadeDown` + `MenuManager_Delete`. The port's
    /// handover: the level request rides the existing `requested_level`/load machinery, the
    /// match config lands in `mp_match_settings` for the host's MP core.
    pub fn mp_confirm_start(&mut self) -> bool {
        let humans = self.mp_human_count();
        if humans == 0 { return false; }
        let time = if self.mp_time_limit == -1 { -1 } else { self.mp_time_limit * 60 };   // [M the ×0x3c]
        let bots = self.mp_bot_count() as u32;
        let mut s = nf_game::mp::MpSettings::deathmatch(self.mp_level);
        s.scenario = self.mp_scenario;
        s.gametype_mask = self.mp_scenario;       // [M the same word — see the atlas correction]
        s.level_code = self.mp_level;
        s.frag_limit = self.mp_frag_limit;
        s.time_limit = time;
        s.human_count = humans as u32;
        s.player_count = humans as u16;           // [M +0x1AC = the joined count]
        s.bot_count = bots;
        s.weapon_set = self.mp_weapon_set;
        s.radar = self.mp_radar;
        s.master = 1;                             // [M +0x180 = 1]
        s.total_players = humans as u32 + bots;   // [M MP_Start 0x183a50:164]
        s.team_game = (self.mp_scenario >> 29) & 1;   // [M MP_Init 0x183438:41 — the core re-derives]
        for (i, slot) in self.mp_join.iter().enumerate().take(4) {
            if !slot.joined { continue; }
            s.players[i] = nf_game::mp::MpPlayer { team: slot.team, skin: slot.skin, armor_bonus: 0 };
        }
        self.mp_match_settings = Some(s);
        // [M the match-start calls] — the level request + the fade push; MenuManager_Delete is
        // the host closing the overlay once the load leaves the front-end map.
        self.requested_level = Some(self.mp_level);
        self.flow.reset_map_level_to_load(self.mp_level, false, false);
        self.flow.push_state(ST_FADE_HOLD);
        true
    }

    /// A summary line's value text for the confirm page [M P_MPDEBRIEFING — no: P_MPCONFIRM's
    /// enter builds the lines with sprintf "%s : %s" @0x30d3e8: Map 0x3d4, Scenario 0x3d5,
    /// Weapon Set 0x24d, the limit line 0x3d8/0x3d7 (0x3ca Unlimited), Duration 0x3d6 (0x3ca),
    /// Friendly Fire 0x3da (0x1b7/0x1b6)].
    pub fn mp_confirm_lines(&self, lang: Option<&Lang>) -> [(u32, String); 6] {
        let bind = |id: u32| lang.and_then(|l| l.bind(id)).map(str::to_string).unwrap_or_else(|| format!("<{id:#x}>"));
        let pair = |a: u32, b: String| (a, b);
        let map_row = MP_MAP_ROWS.iter().find(|r| r.3 == self.mp_level);
        let scen_row = MP_SCENARIO_ROWS.iter().find(|r| r.3 == self.mp_scenario);
        let limit = if self.mp_rules_lives() { 0x3d7 } else { 0x3d8 };
        [
            pair(0x3d4, map_row.map_or(String::new(), |r| bind(r.1))),
            pair(0x3d5, scen_row.map_or(String::new(), |r| bind(r.1))),
            pair(0x24d, bind(MP_WEAPON_SETS[self.mp_weapon_set.min(10) as usize])),
            pair(limit, if self.mp_frag_limit >= 0 { self.mp_frag_limit.to_string() } else { bind(0x3ca) }),
            pair(0x3d6, if self.mp_time_limit == -1 { bind(0x3ca) } else { self.mp_time_limit.to_string() }),
            pair(0x3da, bind(if self.mp_friendly_fire != 0 { 0x1b7 } else { 0x1b6 })),
        ]
    }

    /// The wheel rows' preview texture for the draw code (the game's msg-0x24 binds) — the
    /// scenario/map/options/bots/character wheels' per-row previews.
    pub fn mp_row_preview(&self, row: usize) -> u32 {
        match self.page {
            Page::MpScenario => MP_SCENARIO_ROWS.get(row).map_or(0, |r| r.0),
            Page::MpMap => MP_MAP_ROWS.get(row).map_or(0, |r| r.0),
            Page::MpOptions => MP_OPTIONS_ROWS.get(row).map_or(0, |r| r.0),
            Page::MpBots if row == 0 => MP_BOTS_ROWS[0].0,
            Page::MpBots => self.mp_bots.get(row - 1).map_or(0, |b| MP_CHARACTERS.get(b.character as usize).map_or(0, |c| c.0)),
            Page::MpBotChoose => MP_CHARACTERS.get(row).map_or(0, |r| r.0),
            _ => 0,
        }
    }
    /// The wheel rows' description label (the game's msg-0x18 bind of +0x08, or +0x14 when
    /// locked [M Menu_UpdateWheel 0x1f8388:114-147]; the bots list binds none [M C_SBBOTS's
    /// UpdateWheel call passes 0]).
    pub fn mp_row_desc(&self, row: usize) -> u32 {
        match self.page {
            Page::MpScenario => if self.mp_scenario_locked(row) { MP_LOCKED_SCENARIO_DESC } else { MP_SCENARIO_ROWS.get(row).map_or(0, |r| r.2) },
            Page::MpMap => MP_MAP_ROWS.get(row).map_or(0, |r| r.2),
            Page::MpOptions if row == 1 && !self.mp_options_ai_bots_open() => 0x0100_00ae,
            Page::MpOptions => MP_OPTIONS_ROWS.get(row).map_or(0, |r| r.2),
            Page::MpBotChoose => if self.mp_character_locked(row) { 0x0100_00ad } else { MP_CHARACTERS.get(row).map_or(0, |r| r.2) },
            _ => 0,
        }
    }
    /// The AI Bots row's enable byte [M P_MPOPTIONS 0x217678:20-25 — 0 on Ravine].
    pub fn mp_options_ai_bots_open(&self) -> bool { !(self.mp_level == 0x0700_004b && !self.unlock_all) }
    /// The per-row locked flag for the draw code's dimming [M Menu_UpdateWheel's locked colour
    /// 0x60606060 vs 0x808080ff].
    pub fn mp_row_locked(&self, row: usize) -> bool {
        match self.page {
            Page::MpScenario => self.mp_scenario_locked(row),
            Page::MpOptions => row == 1 && !self.mp_options_ai_bots_open(),
            Page::MpBotChoose => self.mp_character_locked(row),
            _ => false,
        }
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
pub const MENU_TEXTURES: [u32; 41] = [0x0300_0042, 0x0300_0074, 0x0300_00b3, 0x0300_00bf, 0x0300_0084, 0x0300_00b6, 0x0300_00c4, 0x0300_00a5, 0x0300_016d, 0x0300_014c,
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
    0x0300_0005,
    // The difficulty wheel's per-row previews [L ee_snapshot table 0x2df4c0 +0x00] and the Records medals
    // [M C_RBDSRECORDS 0x20bf00:58-110].
    0x0300_00a6, 0x0300_00a7, 0x0300_013e, 0x0300_013f,
    // The codename options wheel's previews [L ee_snapshot table 0x2dfc28].
    0x0300_0121, 0x0300_0124, 0x0300_0123, 0x0300_011f, 0x0300_0125];

/// The menu text renderer's special-character table `specialchar` (0x2dd8d8, 17 × 0x18 bytes: the token
/// char, the texture hash 0x03000075, u0, v0, w, h, advance, height) [M ACTION.ELF symtab + the table's
/// bytes; the hint strings' ~A/~B/... tokens draw through it — the game's own button glyphs, NOT the
/// 64×64 atlas 0x03000042 the port guessed before]. (token, u, v, w, h, advance)
/// The button icons draw ~1.25× their atlas size in the hint bars (PCSX2 2026-10-13/15 captures: the 19-px ✕ and
/// D-pad span ~24 of 640).
const ICON_SCALE: f32 = 1.25;

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
    let aux = aux_movie_for(fe.page_id());
    // fix-pause-menu: the IN-GAME pages (pause 0x4000004b, fail menu 0x40000042) draw OVER THE PAUSED
    // FRAME — the game dims nothing full-screen; their own backdrop plate (45,107,550×266, TEX 0x03000070
    // [L the pages' 0x10000001 control]) carries the menu.
    if !(aux != 0 && fe.backdrop_live) && !matches!(fe.page, Page::Pause | Page::Results) {
        plate(&mut h, 0.0, 0.0, MW, MH, gs(0x180c08, 0xff));
    }
    match fe.page {
        // mp-menu-flow: the multiplayer pages draw from the same getters the other wheels use
        // (mp_row_preview/mp_row_desc/mp_row_locked + row_label); the drawing itself is the main
        // agent's task (no drawing in this task's scope) — the overlay leaves them empty here.
        // Select Scenario / Select Map [L pages 0x4000001a (header 0x2c3) / 0x40000013 (header 0x158): the codename
        // wheel's layout — panel, emblem, five wheel rows, the description plate, hint 0x218; PCSX2 m45-m48].
        Page::MpScenario | Page::MpMap => {
            let (hdr, n, cur) = if fe.page == Page::MpScenario { (0x2c3, MP_SCENARIO_ROWS.len(), fe.mp_scenario_row) } else { (0x158, MP_MAP_ROWS.len(), fe.mp_map_row) };
            let cur = cur.min(n - 1);
            page_header(&mut h, hfont, &bind(hdr));
            panel(&mut h);
            let c = if fe.mp_row_locked(cur) { gs(0x7f7f7f, 0x60) } else { gs(0x7f7f7f, 0xff) };
            emblem_col(&mut h, fe.mp_row_preview(cur), 0x0300_00bf, c, c);
            // Locked rows still show their name, greyed (PCSX2 m48: "Uplink", "Demolition"); the locked text is the description.
            let name = |r: usize| -> String { if fe.page == Page::MpScenario { bind(MP_SCENARIO_ROWS[r].1) } else { fe.row_label(lang, r) } };
            wheel_at(&mut h, font, n, cur, name, |r| fe.mp_row_locked(r), cream);
            desc_block(&mut h, font, &bind(fe.mp_row_desc(cur)), bright);
            hint_bar(&mut h, font, &hint(0x218, "~A Select  ~V Scroll  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        // Join Game [L page 0x40000019 (parsed 2026-10-15): header 0x29e, hint 0x49b; four panels TEX 0x03000199
        // (50/328, 103/253, 262×134) titled 0x5f0..0x5f3 at (70/346, 105/255); each panel's message box
        // (55/332, 135/285, 253×70); agent 1's codename prompt 0x37b (70,132), its emblem (117,150,128×64) and the
        // codename spinner 0x1000022b (70,215,226×16). PCSX2 m42-m44.]
        Page::MpJoin => {
            page_header(&mut h, hfont, &bind(0x29e));
            let panels = [(50.0f32, 103.0f32), (328.0, 103.0), (50.0, 253.0), (328.0, 253.0)];
            for (i, &(px, py)) in panels.iter().enumerate() {
                quad640(&mut h, 0x0300_0199, px, py, 262.0, 134.0, [0.0, 0.0, 256.0, 128.0], gs(0x7f7f7f, 0xff));
                label(&mut h, font, &bind(0x5f0 + i as u32), px + 20.0, py + 2.0, 226.0, 16.0, bright, true, 1.0);
                let a = &fe.mp_join[i];
                let (bx, by) = (px + 5.0, py + 32.0);
                // The message lines, centred in the box at pitch 19 (PCSX2: middles 152/171/190 in a box from 135).
                let lines_at = |h: &mut Hud, text: &str, mid: f32| {
                    let lines: Vec<String> = text.split('\n').flat_map(|p| wrap_px(p, font, mx(240.0))).collect();
                    let n = lines.len() as f32;
                    // A text with a button glyph spaces its lines wider (PCSX2 m44: "Press ✕" / "when ready" 29 apart; the
                    // plain wrapped lines 19).
                    let pitch = if text.contains('~') && n > 1.0 { 29.0 } else { 19.0 };
                    for (k, l) in lines.iter().enumerate() {
                        let m = mid + (k as f32 - (n - 1.0) / 2.0) * pitch;
                        hint_row(h, font, l, bx + 126.5 - 270.0, m - 8.5, cream, None);
                    }
                };
                // hint_row centres on its x640 + 270 and puts the words' middle at y640 + 8.5.
                let _ = by;
                match (i, a.state) {
                    (0, 1) => {
                        label(&mut h, font, &bind(0x37b), 70.0, 132.0, 226.0, 16.0, cream, true, 1.0);
                        // [M C_RBMPSTART 0x20d048:51-53: the image control gets TEX 0x030001a0, uv (0,0)-(127,63)]
                        quad640(&mut h, 0x0300_01a0, 117.0, 150.0, 128.0, 64.0, [0.0, 0.0, 128.0, 64.0], gs(0x7f7f7f, 0xff));
                        label(&mut h, font, &fe.mp_codename_at(a.codename_row), 70.0, 215.0, 226.0, 16.0, cream, true, 1.0);
                        let yellow = gs(0xfec43e, 0xff);
                        for (cx, left) in [(80.0f32, true), (284.0, false)] {
                            let cy = 223.0;
                            let (p0, p1, p2) = if left { ((cx + 6.0, cy - 6.0), (cx + 6.0, cy + 6.0), (cx - 6.0, cy)) } else { ((cx - 6.0, cy - 6.0), (cx - 6.0, cy + 6.0), (cx + 6.0, cy)) };
                            let q = |p: (f32, f32), h: &Hud| HudVertex { p: h.ndc(Anchor::Centre, mx(p.0), my(p.1)), uv: [0.0, 0.0], c: yellow };
                            let v = [q(p0, &h), q(p1, &h), q(p2, &h), q(p0, &h), q(p2, &h), q(p2, &h)];
                            h.ndc_solid(-3, v, yellow);
                        }
                    }
                    (0, _) => { let t = fe.row_label(lang, 0); lines_at(&mut h, &t, py + 32.0 + 43.0); }
                    // Agents 2-4: the game's insert-controller texts (the port has one input) [L USATxt 0x3c0 / 0x4a2;
                    // PCSX2 m42: port 2 for agent 2, the multitap line for 3 and 4].
                    (1, _) => { let t = bind(0x3c0).replace("%s", "2"); lines_at(&mut h, &t, py + 32.0 + 36.0); }
                    _ => { let t = bind(0x4a2); lines_at(&mut h, t.trim_end(), py + 32.0 + 36.0); }
                }
            }
            hint_bar(&mut h, font, &hint(0x49b, "~A Confirm  ~V Change  ~B Back"), 50.0, 419.0, cream);
            quad640(&mut h, 0x0300_016d, 434.0, 38.0, 156.0, 43.0, [0.0, 0.0, 256.0, 128.0], gs(0x7f7f7f, 0xff));
        }
        // The MP wheels with the codename wheel's layout [L pages 0x40000012 (header 0x2c4 Scenario Options),
        // 0x40000027 / 0x4000003f (header 0x294 Multiplayer Bot Setup); hint 0x218; PCSX2 m55, m56, m57, m61, m63].
        Page::MpOptions | Page::MpBots | Page::MpBotChoose => {
            let (hdr, n, cur) = match fe.page {
                Page::MpOptions => (0x2c4, MP_OPTIONS_ROWS.len(), fe.mp_options_row),
                Page::MpBots => (0x294, MP_BOTS_ROWS.len(), fe.mp_bots_row),
                _ => (0x294, MP_CHARACTERS.len(), fe.mp_bot_char),
            };
            let cur = cur.min(n - 1);
            page_header(&mut h, hfont, &bind(hdr));
            panel(&mut h);
            let c = if fe.mp_row_locked(cur) { gs(0x7f7f7f, 0x60) } else { gs(0x7f7f7f, 0xff) };
            emblem_col(&mut h, fe.mp_row_preview(cur), 0x0300_00bf, c, c);
            let page = fe.page;
            // The wheels' rows: the bots list's rows 1-4 read "Setup Bot N" (PCSX2 m57); the others the getters' labels.
            let name = |r: usize| -> String {
                match page {
                    Page::MpOptions => bind(MP_OPTIONS_ROWS[r].1),
                    Page::MpBotChoose => bind(MP_CHARACTERS[r].1),
                    _ => bind(MP_BOTS_ROWS[r].1),
                }
            };
            wheel_at(&mut h, font, n, cur, name, |r| fe.mp_row_locked(r), cream);
            let desc = match fe.page {
                // [PCSX2 m57/m63: "Playing : No" / "Playing : Yes" for the slot's enabled byte]
                Page::MpBots if cur > 0 => format!("{} : {}", bind(0x295), bind(if fe.mp_bots[cur - 1].enabled { 0x181 } else { 0x182 })),
                Page::MpBots => String::new(),
                // [PCSX2 m61: the character's description, then "Team : Phoenix" (the preset's side)]
                Page::MpBotChoose => format!("{}\n{} : {}", bind(fe.mp_row_desc(cur)), "Team", bind(if MP_CHARACTER_SIDE_MI6[cur] { 0x1c8 } else { 0x1c7 })),
                _ => bind(fe.mp_row_desc(cur)),
            };
            let lines: Vec<String> = desc.split('\n').flat_map(|p| wrap_px(p, font, mx(505.0))).collect();
            let nl = lines.len().min(4) as f32;
            for (i, line) in lines.iter().take(4).enumerate() {
                // The description plate's lines at pitch 28 (PCSX2 m56/m61, like the other wheel pages' desc_block).
                let mid = 366.0 + (i as f32 - (nl - 1.0) / 2.0) * 28.0;
                centred(&mut h, font, -2, line, mx(320.0), my(mid + 6.0), bright, true, 1.0);
            }
            hint_bar(&mut h, font, &hint(0x218, "~A Select  ~V Scroll  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        // The MP value-list pages [L pages 0x40000014 Game Rules (header 0x19b), 0x40000017 Player Mods (0x19d),
        // 0x40000028 Enviro-Mods (0x19e), 0x4000002c Bot Setup (0x294); hint 0x217; labels at x 54, values centred on
        // the value controls, the description plate (50,355,540×40); the value labels per row from the handlers
        // (P_MPRULES 0x20f790, P_MPPLAYERMODS 0x2104d0, P_MPENVIROMODS 0x210030, P_MPBOTSETUP 0x211270); PCSX2
        // m58-m60, m62].
        Page::MpRules | Page::MpPlayerMods | Page::MpEnviroMods | Page::MpBotSetup => {
            let onoff = |v: bool| bind(if v { 0x1b7 } else { 0x1b6 });
            let num = |v: i32| v.to_string();
            // (label, value text, row y, control, value)
            let rows: Vec<(u32, String, f32, u32, i32)> = match fe.page {
                Page::MpRules => {
                    let t = fe.mp_time_limit;
                    vec![(0x156, if t < 0 { bind(0x3ca) } else { num(t) }, 191.0, 0x1000_0008, t),
                         (if fe.mp_rules_lives() { 0x3d3 } else { 0x24c }, num(fe.mp_frag_limit), 223.0, 0x1000_00f6, fe.mp_frag_limit)]
                }
                Page::MpPlayerMods => {
                    const SETS: [u32; 11] = [0x1a0, 0x1a2, 0x1a4, 0x1a6, 0x1a8, 0x24f, 0x3db, 0x3dc, 0x3dd, 0x3de, 0x1b2];
                    vec![(0x1ae, onoff(fe.mp_friendly_fire != 0), 143.0, 0x1000_0081, fe.mp_friendly_fire as i32),
                         (0x24d, bind(SETS[(fe.mp_weapon_set as usize).min(10)]), 175.0, 0x1000_0084, fe.mp_weapon_set as i32),
                         (0x1ac, onoff(fe.mp_player_mods[0] != 0), 207.0, 0x1000_0080, fe.mp_player_mods[0] as i32),
                         (0x24e, onoff(fe.mp_player_mods[1] != 0), 239.0, 0x1000_008c, fe.mp_player_mods[1] as i32),
                         (0x1e5, onoff(fe.mp_radar != 0), 271.0, 0x1000_008d, fe.mp_radar as i32)]
                }
                Page::MpEnviroMods => {
                    let e = fe.mp_envmods;
                    vec![(0x1af, bind([0x1b0, 0x1b1, 0x1b2][(e[0] as usize).min(2)]), 143.0, 0x1000_00a2, e[0] as i32),
                         (0x1ab, onoff(e[1] != 0), 177.0, 0x1000_009f, e[1] as i32),
                         (0x1aa, if e[2] == 0x10 { bind(0x100_0076) } else { onoff(e[2] != 0) }, 209.0, 0x1000_019b, e[2] as i32),
                         (0x328, onoff(e[3] != 0), 241.0, 0x1000_0227, e[3] as i32),
                         (0x5b9, bind([0x1b6, 0x100_02d8, 0x100_02d9, 0x1b2][(e[4] as usize).min(3)]), 273.0, 0x1000_0228, e[4] as i32)]
                }
                _ => {
                    let b = &fe.mp_bots[fe.mp_bot_slot.min(3)];
                    let pick = |v: i32, t: &[(i32, u32)]| t.iter().find(|x| x.0 == v).map_or(num(v), |x| bind(x.1));
                    vec![(0x295, bind(if b.enabled { 0x181 } else { 0x182 }), 85.0, 0x1000_0116, b.enabled as i32),
                         (0x247, pick(b.speed, &[(0, 0x23c), (1, 0x23d), (2, 0x23e)]), 117.0, 0x1000_018c, b.speed),
                         (0x232, pick(b.trait_id, &[(0, 0x100_02d0), (1, 0x233), (2, 0x234), (3, 0x235), (4, 0x24a), (5, 0x237), (6, 0x238), (7, 0x239), (8, 0x24b)]), 149.0, 0x1000_0195, b.trait_id),
                         (0x246, pick(b.aim, &[(2, 0x3bd), (3, 0x31b), (4, 0x31c)]), 181.0, 0x1000_0185, b.aim),
                         (0x15e, pick(b.skill, &[(8, 0x241), (5, 0x242), (3, 0x243), (1, 0x244)]), 213.0, 0x1000_0186, b.skill),
                         (0x1c6, num(b.hit_points), 245.0, 0x1000_0187, b.hit_points),
                         (0x23a, num(b.recovery1), 277.0, 0x1000_0188, b.recovery1),
                         (0x249, num(b.recovery2), 309.0, 0x1000_018a, b.recovery2)]
                }
            };
            let hdr = match fe.page { Page::MpRules => 0x19b, Page::MpPlayerMods => 0x19d, Page::MpEnviroMods => 0x19e, _ => 0x294 };
            page_header(&mut h, hfont, &bind(hdr));
            let on = gs(CREAM, 0xff);
            let off = gs(0x73330f, 0xc0);
            let yellow = gs(0xfec43e, 0xff);
            let cur = fe.cursor.min(rows.len() - 1);
            for (i, (lbl, val, y, _, _)) in rows.iter().enumerate() {
                let c = if i == cur { on } else { off };
                let mid = y + 8.0;
                left_text(&mut h, font, &bind(*lbl), mx(54.0), my(mid + 6.0), c, true, 1.3);
                centred(&mut h, font, -2, val, mx(457.0), my(mid + 6.0), c, true, 1.3);
                if i == cur {
                    for (cx, left) in [(357.0f32, true), (555.0, false)] {
                        let (p0, p1, p2) = if left { ((cx + 7.0, mid - 7.0), (cx + 7.0, mid + 7.0), (cx - 7.0, mid)) } else { ((cx - 7.0, mid - 7.0), (cx - 7.0, mid + 7.0), (cx + 7.0, mid)) };
                        let q = |p: (f32, f32), h: &Hud| HudVertex { p: h.ndc(Anchor::Centre, mx(p.0), my(p.1)), uv: [0.0, 0.0], c: yellow };
                        let v = [q(p0, &h), q(p1, &h), q(p2, &h), q(p0, &h), q(p2, &h), q(p2, &h)];
                        h.ndc_solid(-3, v, yellow);
                    }
                }
            }
            if let Some(&(_, _, _, ctrl, v)) = rows.get(cur) {
                if let Some(dl) = opt_desc(ctrl, v) {
                    let plate_mid = if fe.page == Page::MpBotSetup { 375.0 } else { 375.0 };
                    centred(&mut h, font, -2, &bind(dl), mx(320.0), my(plate_mid + 6.0), gs(CREAM, 0xff), true, 1.0);
                }
            }
            hint_bar(&mut h, font, &hint(0x217, "~A Confirm  ~V Scroll  ~H Change  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        // Not drawn yet: the per-agent setup step (PCSX2 m52-m54 — the Join Game panels with Choose Team / Choose
        // Character / Health Handicap), the Start summary (m65) and the debriefing.
        Page::MpSetup | Page::MpConfirm | Page::MpDebriefing => {}
        Page::Start => {
            // The title [M menu-def page 0x40000009: the art control 0x10000196 (0,0,640,480) TEX
            // 0x03000074; the prompt control 0x100000fa (131,309,379×16) LBL 0x1c9, colours 0x7f621fff;
            // the legal plate 0x10000197 (50,332,540×108). The handler binds 0x10001eb to the prompt
            // [M 0x20a408] and 0x375 to the plate, flashing on the 240-frame cycle.]
            quad640(&mut h, 0x0300_0074, 0.0, 0.0, MW, MH, [0.0, 0.0, 512.0, 512.0], gs(0x7f7f7f, 0xff));
            if fe.press_start_on {
                centred(&mut h, font, -2, &bind(0x100_01eb), mx(320.5), my(326.0), gs(YELLOW, 0xff), true, 1.25); // PCSX2 m00: 266 units wide, middle y 320
            }
            let legal = bind(0x375);
            if !legal.is_empty() {
                // The legal text wraps inside its plate (50,332,540×108) and each line is centred in it
                // [L the main agent's PCSX2 captures 2026-10-13; the port's raw 92-column run went off
                // the right edge]. Wrap to the plate's pixel width, centre at the plate's centre x=320.
                // PCSX2 m00 (2026-10-15): lines up to ~482 units, middles from y 345 at pitch 18, full cream with shadow.
                for (i, line) in wrap_px(&legal, font, mx(490.0)).iter().take(6).enumerate() {
                    centred(&mut h, font, -3, line, mx(320.0), my(351.0 + i as f32 * 18.0), gs(CREAM, 0xff), true, 1.0);
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
                // The rows draw in the header face, centred in their controls (PCSX2 07000048 main, 2026-10-15:
                // "NightFire" centred on x 320, middles at 279/311/343).
                let c = if i == fe.cursor { cream } else { gs(0x73330f, 0x80) };
                centred_tex(&mut h, hfont, 0x0300_0005, -2, &bind(lbl), mx(167.0 + 153.0), my(y + 14.0 + 3.0 + 6.0), c, true, 1.0);
            }
            // Wrapped inside its plate, the lines centred on y 379 at pitch 16 (PCSX2 m03/m05: one line at 379, two at
            // 371/387 breaking after "saved").
            let lines = wrap_px(&bind(MAIN_DESCS[fe.cursor.min(2)]), font, mx(380.0));
            let n = lines.len().min(3) as f32;
            for (i, line) in lines.iter().take(3).enumerate() {
                let mid = 379.0 + (i as f32 - (n - 1.0) / 2.0) * 16.0;
                centred(&mut h, font, -2, line, mx(320.0), my(mid + 6.0), cream, true, 1.0);
            }
            hint_bar(&mut h, font, &hint(0x54e, "~V Scroll  ~A Select"), 50.0, 419.0, cream);
            // No logo or stamp: the page's only art is its aux FMV.
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
            desc_block(&mut h, font, &desc, bright);
            // The confirm box: the game pops it for "Play without saving" [M C_SBNFCN 0x20b390:19-21
            // Menu_CreateOptionBox(0x10000bb, 9, 1, 1); the plate 0x100001f2].
            if fe.confirming_default_profile {
                confirm_box(&mut h, font, &bind(0x100_00bb), &hint(0x100_01d6, "Press ~A to continue or ~B to cancel."), bright);
            }
            // The box takes the footer's place (PCSX2 m07: no hint while it shows).
            if !fe.confirming_default_profile {
                hint_bar(&mut h, font, &hint(0x216, "~A Select  ~Y Create  ~V Scroll  ~B Back"), 50.0, 419.0, cream);
            }
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
            // The preview is per row [M C_SBNFDFCTY_Handler 0x216ca8 → Menu_UpdateWheel with the table 0x2df4c0; L
            // ee_snapshot: +0x00 = 0x030000a5/a6/a7].
            emblem(&mut h, [0x0300_00a5, 0x0300_00a6, 0x0300_00a7][fe.cursor.min(2)], 0x0300_00bf);
            wheel(&mut h, fe, font, lang, |_fe, row| bind(DIFFICULTIES[row.min(2)]), cream);
            // The description wraps inside its plate (50,301,540×113) [L the main agent's captures:
            // the port's single line ran off the right edge].
            desc_block(&mut h, font, &bind(DIFFICULTY_DESCS[fe.cursor.min(2)]), bright);
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
            desc_block(&mut h, font, &desc, bright);
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
            desc_block(&mut h, font, &bind(DOSSIER_ROWS[row].2), bright);
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
            // The three framed boxes [L the b0=1 frame controls (68,120,505×34), (68,162,505×113), (68,285,505×60)]:
            // translucent black with a thin dark border (PCSX2 07000048 Records, 2026-10-13). No ring art.
            for (x, y, w, hh) in [(68.0f32, 120.0f32, 505.0f32, 34.0f32), (68.0, 162.0, 505.0, 113.0), (68.0, 285.0, 505.0, 60.0)] {
                frame640(&mut h, x, y, w, hh);
            }
            let m = fe.cursor.min(CAMPAIGN.len() - 1);
            // The spinner 0x10000174 (132,124,376×16): the mission name centred, a yellow ◀ ▶ at its ends (PCSX2:
            // x 144 / 497) [M P_DSRECORDS 0x217248:11-13 fills it from sp_level].
            const REC_SCALE: f32 = 1.25; // the Records labels measure 1.25× the port's base face (PCSX2: "High Score" 120 units)
            label(&mut h, font, &bind(CAMPAIGN[m].1), 132.0, 124.0, 376.0, 16.0, bright, true, REC_SCALE);
            for (cx, left) in [(144.0f32, true), (497.0, false)] {
                let yellow = gs(0xfec43e, 0xff);
                let (a, b, d) = if left { ((cx + 8.0, 126.0), (cx + 8.0, 140.0), (cx - 8.0, 133.0)) } else { ((cx - 8.0, 126.0), (cx - 8.0, 140.0), (cx + 8.0, 133.0)) };
                let q = |p: (f32, f32)| HudVertex { p: h.ndc(Anchor::Centre, mx(p.0), my(p.1)), uv: [0.0, 0.0], c: yellow };
                let v = [q(a), q(b), q(d), q(a), q(d), q(d)];
                h.ndc_solid(-3, v, yellow);
            }
            // The rows [L LBL 0x302/0x165/0x305 at (83,169/202/235)] left-aligned, the values [L 0x10000175/177/178
            // (375,·,185×16)] right-aligned to x 560 (PCSX2 558).
            let (best, moments, bonuses) = fe.record_row(m);
            for ((id, y), v) in [(0x302u32, 169.0f32), (0x165, 202.0), (0x305, 235.0)].into_iter().zip([best, moments, bonuses]) {
                let yc = my(y + 12.5 + 6.0);
                h.text_scaled(Anchor::Centre, font, -2, &bind(id), mx(83.0), yc, false, cream, true, REC_SCALE);
                h.text_scaled(Anchor::Centre, font, -2, &v.to_string(), mx(560.0), yc, true, cream, true, REC_SCALE);
            }
            // The medal: shown only once earned [M C_RBDSRECORDS 0x20bf00:58-110 — the level's four bonus flags pick
            // 0x307/0x300013e, 0x306/0x300013f, 0x308/0x3000140 or 0x309/0x300013d; none → both controls hidden].
            if let Some((lbl_id, tex)) = fe.record_medal(m) {
                quad640(&mut h, tex, 435.0, 282.0, 128.0, 64.0, [0.0, 0.0, 255.0, 127.0], gs(0x7f7f7f, 0xff));
                label(&mut h, font, &bind(lbl_id), 76.0, 306.0, 240.0, 16.0, cream, true, REC_SCALE);
            }
            hint_bar(&mut h, font, &hint(0x49e, "~V Change  ~B Back"), 50.0, 419.0, cream);
            // Only the logo [L 0x10000001 (434,38,156×43) TEX 0x0300016d] — this page has no ring stamp.
            quad640(&mut h, 0x0300_016d, 434.0, 38.0, 156.0, 43.0, [0.0, 0.0, 256.0, 128.0], gs(0x7f7f7f, 0xff));
        }
        Page::Options => { page_rows(&mut h, fe, font, "OPTIONS", dim, bright); }
        Page::CnSelect => {
            // Edit Codename [L page 0x4000001b: header LBL 0x1f1, the codename wheel 0x100000ec, the emblem
            // 0x100001cd (preview) under 0x10000108 (ring), hint 0x20a; PCSX2 m29].
            page_header(&mut h, hfont, &bind(0x1f1));
            panel(&mut h);
            emblem(&mut h, if fe.cursor == 0 { 0x0300_013b } else { 0x0300_0135 }, 0x0300_00bf);
            wheel(&mut h, fe, font, lang, |fe, row| fe.row_label(lang, row), cream);
            // [M Menu_UpdateCodenameWheel's descriptions: create 0x1000004, default 0x1000003, a codename 0x1000005]
            desc_block(&mut h, font, &bind([0x100_0004, 0x100_0003, 0x100_0005][fe.cursor.min(2)]), bright);
            // ~X Delete only offers on a saved codename (PCSX2 m29: Default shows Edit / Create / Scroll / Back).
            let mut hs = hint(0x20a, "~A Edit  ~Y Create  ~X Delete  ~V Scroll  ~B Back");
            if fe.cursor < 2 { if let Some(i) = hs.find("~X") { let j = hs[i..].find("  ").map_or(hs.len(), |k| i + k + 2); hs.replace_range(i..j, ""); } }
            hint_bar(&mut h, font, &hs, 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::CnMenu => {
            // "Edit <name>" [L page 0x4000001d: header 0x100001f9 runtime-filled (PCSX2 m30 "Edit Default"), the
            // wheel 0x100000ff, the preview 0x100001a5 under the ring 0x10000109, hint 0x5cb; table 0x2dfc28].
            page_header(&mut h, hfont, &format!("Edit {}", if fe.cn_name.is_empty() { "Default" } else { &fe.cn_name }));
            panel(&mut h);
            emblem(&mut h, CN_MENU[fe.cursor.min(CN_MENU.len() - 1)].0, 0x0300_00bf);
            // The default profile's last row reads "Codenames" (PCSX2 m37) — it has nothing to save.
            let default = fe.cn_name.is_empty() || fe.cn_name == "Default";
            wheel(&mut h, fe, font, lang, |fe, row| if row == 6 && default { bind(MAIN_ITEMS[2]) } else { fe.row_label(lang, row) }, cream);
            desc_block(&mut h, font, &bind(CN_MENU[fe.cursor.min(CN_MENU.len() - 1)].2), bright);
            hint_bar(&mut h, font, &hint(0x5cb, "~A Select  ~V Scroll  ~B Cancel"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::CnName => {
            // The Secret Unlocks letter grid (the model side; the drawn layout is the stub until
            // the drawing task lands — the main agent owns the page's look).
            page_header(&mut h, hfont, &bind(0x1000_02d4)); // cheat_mode's prompt label [M 0x213790:37]
            panel(&mut h);
            let mut grid = String::new();
            for (i, k) in SECRET_GRID.iter().enumerate() {
                let ch = match k { SecretKey::Char(c) => *c, SecretKey::Space => '_', SecretKey::Delete => '<', SecretKey::Submit => '+' };
                if i == fe.secret.cursor { grid.push('['); }
                grid.push(ch);
                if i == fe.secret.cursor { grid.push(']'); }
                grid.push(' ');
            }
            h.text_scaled_tex(Anchor::Centre, font, -1, 0x0300_0005, &fe.secret.typed, mx(320.0), my(150.0), false, cream, true, 1.3);
            h.text_scaled_tex(Anchor::Centre, font, -1, 0x0300_0005, &grid, mx(320.0), my(190.0), false, bright, true, 0.9);
            if fe.secret.accepted {
                h.text_scaled_tex(Anchor::Centre, font, -1, 0x0300_0005, "CODE ACCEPTED", mx(320.0), my(300.0), false, bright, true, 1.1); // label 0x100029d's text [S]
            }
            hint_bar(&mut h, font, &hint(0x5cb, "~A Select  ~H Move  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::CnAdvanced | Page::CnMp | Page::CnAv => {
            let header = match fe.page { Page::CnAdvanced => 0x37a, Page::CnMp => 0x199, _ => 0x2bb };
            page_header(&mut h, hfont, &bind(header));
            opt_list(&mut h, fe, font, lang);
            hint_bar(&mut h, font, &hint(0x217, "~A Confirm  ~V Scroll  ~H Change  ~B Back"), 50.0, 419.0, cream);
            logo_stamp(&mut h);
        }
        Page::ScreenAdjust => {
            // The Screen Adjust page's own controls [L page 0x40000047 walk 2026-10-15]: the
            // fullscreen reference picture TEX 0x3000074 (the msg-0x24 control (0,0,640,480)); the
            // two slider bars — vertical (304,176,32,128) d1 0, horizontal (256,224,128,32) d1 1,
            // both named 0x100001eb; the hint bar (50,419) LBL 0x67b. The main agent compares this
            // against PCSX2 — the bars here are the page data's plates with the focused one bright.
            quad640(&mut h, 0x0300_0074, 0.0, 0.0, MW, MH, [0.0, 0.0, 512.0, 512.0], gs(0x7f7f7f, 0xff));
            let (bar_v, bar_h) = (gs(0x73330f, 0xc0), gs(0x73330f, 0xc0));
            let hot = cream;
            plate(&mut h, 304.0, 176.0, 32.0, 128.0, if fe.cursor == 0 { hot } else { bar_v });
            plate(&mut h, 256.0, 224.0, 128.0, 32.0, if fe.cursor == 1 { hot } else { bar_h });
            hint_bar(&mut h, font, &hint(0x67b, "~A Confirm  ~B Cancel"), 50.0, 419.0, cream);
        }
        Page::Pause => pause_page(&mut h, fe, font, lang, textures, cream, bright, dim),
        Page::Results => results_page(&mut h, fe, font, lang, textures, cream, bright, dim),
        // credits-data: the main agent draws the roll from fe.credits.visible() (the game's
        // own line list + y positions) — nothing here yet.
        Page::Credits => {}
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
        h.quad(Anchor::Centre, 0, 0x0300_00db, 1, mx(385.0), my(121.0), mx(202.0), my(218.0), [0.0, 0.0, 128.0, 128.0], (false, false), gs(0x7f7f7f, 0x50));
    }
    // The four tabs [L the page's four type-1 controls 0x10000028 at (84/199/339/465,120,107/132/118/90×16),
    // LBL 0x167 MISSION / 0x176 OBJECTIVES / 0x168 CONTROLS / 0x169 SCORE; base colour 0x7d6d59ff →
    // 0x645a49ff [M 0x21b908:238-243], the selected tab's 0x1c colour 0x201707ff].
    let tabs = [(0x167u32, 84.0, 107.0, "MISSION"), (0x176, 199.0, 132.0, "OBJECTIVES"), (0x168, 339.0, 118.0, "CONTROLS"), (0x169, 465.0, 90.0, "SCORE")];
    let tab = fe.pause_tab.min(3);
    // The selected tab is the LIGHT one and the rest dark brown (PCSX2 07000005 pause, MISSION/OBJECTIVES tabs) — the
    // controls' 0x1b colour 0x7d6d59 vs 0x1c 0x201707; each centred in its control.
    for (i, &(id, x, w, fb)) in tabs.iter().enumerate() {
        let c = if i == tab { gs(0x7d6d59, 0xff) } else { gs(0x201707, 0xff) };
        label(h, font, &lbl(id, fb), x, 120.0, w, 16.0, c, false, 1.0);
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
            // The CONTROLS tab, from `Menu_DisplayControllerStyleList` 0x1f44d8 for the selected style. PCSX2's
            // default (and the port's: mouse+keyboard has no pad styles) is style 7 "Classic Bond" 0x100019a [M
            // P_PAUSE_Handler 0x203c28 binds the 8 style names 0x82/0x91/0xa0/0xaf/0xbe/0xcd/0x100018b/0x100019a].
            // Style 7's rows [M 0x1f44d8 case 7]: each row is "%s %s" of a fixed button token (.data 0x30d1c8..
            // 0x30d228: ~A ~B / ~Y ~X / ~C ~D / ~L ~R / ~F ~E / ~H ~V / ~T) and that style's label.
            // The header band (88,156,465×27) — a solid texel of 0x03000042 in 0x07020280 [L the control's m0x2d uv
            // (19,38,1×1)]; the style spinner (84,152,24×34); the style name in the list (113,156,286×28); the Y-Axis
            // label (400,160) and its value (463,160) — 0x3bd "Normal" in 0x694646 (PlayerSetting 0) [M 0x203c28].
            {
                let (a, b) = (h.ndc(Anchor::Centre, mx(88.0), my(156.0)), h.ndc(Anchor::Centre, mx(88.0 + 465.0), my(156.0 + 27.0)));
                let q = |p: [f32; 2]| HudVertex { p, uv: [0.0, 0.0], c: [0.0; 4] };
                let v = [q([a[0], a[1]]), q([b[0], a[1]]), q([b[0], b[1]]), q([a[0], a[1]]), q([b[0], b[1]]), q([a[0], b[1]])];
                // Behind the labels (a higher layer draws first); a translucent dark band in the capture.
                h.ndc_solid(-1, v, [0.06, 0.02, 0.02, 0.5]);
            }
            let yellow = gs(0xfec43e, 0xff);
            for (dy, up) in [(-8.0f32, true), (8.0, false)] {
                let (cx, cy) = (96.0f32, 169.0 + dy);
                let (a, b, d) = if up { ((cx - 9.0, cy + 6.0), (cx + 9.0, cy + 6.0), (cx, cy - 6.0)) } else { ((cx - 9.0, cy - 6.0), (cx + 9.0, cy - 6.0), (cx, cy + 6.0)) };
                let q = |p: (f32, f32)| HudVertex { p: h.ndc(Anchor::Centre, mx(p.0), my(p.1)), uv: [0.03, 0.03], c: yellow };
                let v = [q(a), q(b), q(d), q(a), q(d), q(d)];
                h.ndc_solid(-3, v, yellow);
            }
            let left_label = |h: &mut Hud, s: &str, x: f32, yc: f32, c: [f32; 4]| h.text(Anchor::Centre, font, -2, s, mx(x), my(yc + 6.0), false, c, false);
            left_label(h, &lbl(0x100_019a, "Classic Bond"), 114.0, 169.0, bright);
            left_label(h, &lbl(0x492, "Y-Axis"), 404.0, 168.0, cream);
            left_label(h, &lbl(0x3bd, "Normal"), 466.0, 168.0, gs(0x694646, 0xff));
            // The rows: PCSX2 07000005 (2026-10-15) — row centres 199 + 21·i, the left column's icons at x 98, the
            // right column's at 329.
            let rows: [(&str, u32, &str, u32); 6] = [
                ("~A", 0x100_01a7, "~B", 0x100_01a4), ("~Y", 0x100_01a3, "~X", 0x100_01a5), ("~C", 0x100_019c, "~D", 0x100_01a1),
                ("~L", 0x100_019d, "~R", 0x100_01a2), ("~F", 0x100_01a0, "~E", 0x100_01a8), ("~H", 0x100_019f, "~V", 0x100_019e)];
            for (i, (g0, l0, g1, l1)) in rows.iter().enumerate() {
                let y = 199.0 + i as f32 * 21.0 - 9.0;
                glyph_row(h, font, &format!("{g0} {}", lbl(*l0, "")), 98.0, y, cream);
                glyph_row(h, font, &format!("{g1} {}", lbl(*l1, "")), 329.0, y, cream);
            }
            glyph_row(h, font, &format!("~T {}", lbl(0x100_01ce, "Vision Mode")), 98.0, 199.0 + 6.0 * 21.0 - 9.0, cream);
        }
        // fix-pause-menu2: the SCORE tab — the stats list box (98,147,445×190) [L the d1=8 control]
        // filled exactly as the game's msg-0x4e case-3 arm does [M 0x204408:447-498]: the
        // Category/Current/Target header (0x1000204/0x10001e1/0x10001e2), then Bond Moves, the
        // indented Opponents trio, Accuracy Rating, Health Remaining, Time, 007 Bonus. Current =
        // the port's tracked stats (time, accuracy); the other counters are the port's zeros
        // (reported). Targets = SCORE_TARGETS for the level's campaign row.
        3 => {
            // The d1=8 list (98,147,445×190): two right-aligned value columns [M 0x204408:447-498 — msg 0x59 (1,2) and
            // (2,2) right-align columns 1 and 2]. PCSX2 07000005 (2026-10-15): the Current column ends at x 421, the
            // Target column at 548; rows from y 157 at a 16.7 pitch — header, a blank row (0x30d2d8), Bond Moves,
            // Opponents (no values), the indented Dispatched/Subdued/Surrendered ("  %s" 0x30d2f8), Accuracy Rating,
            // Health Remaining (the literal 100), Time ("%s" of Timer_Seconds2String), 007 Bonus. Values print "%d"
            // (0x30d2f0) — no % sign. No backing plate (the page plate shows through).
            let st = &fe.stats;
            let row = score_row_for_level(fe.flow.current_level).unwrap_or(fe.mission);
            let t = SCORE_TARGETS[row.min(11)];
            let time_s = |s: u32| format!("{:02}:{:02}", s / 60, s % 60);
            let (cx1, cx2) = (421.0f32, 548.0f32);
            let row_y = |k: usize| 157.0 + k as f32 * 16.7;
            let put = |h: &mut Hud, k: usize, name: &str, cur: &str, tgt: &str, c: [f32; 4]| {
                let y = my(row_y(k) + 6.0);
                h.text(Anchor::Centre, font, -2, name, mx(100.0), y, false, c, false);
                if !cur.is_empty() { h.text(Anchor::Centre, font, -2, cur, mx(cx1), y, true, c, false); }
                if !tgt.is_empty() { h.text(Anchor::Centre, font, -2, tgt, mx(cx2), y, true, c, false); }
            };
            put(h, 0, &lbl(0x1000_0204, "Category"), &lbl(0x1000_01e1, "Current"), &lbl(0x1000_01e2, "Target"), cream);
            let rows: [(u32, bool, String, String); 9] = [
                (0x165, false, "0".into(), t[0].to_string()),                    // Bond Moves
                (0x15a, false, String::new(), String::new()),                    // Opponents (no values)
                (0x15b, true, "0".into(), t[1].to_string()),                     // Dispatched
                (0x15c, true, "0".into(), t[2].to_string()),                     // Subdued
                (0x16e, true, "0".into(), t[3].to_string()),                     // Surrendered
                (0x15e, false, st.accuracy_pct.to_string(), t[5].to_string()),   // Accuracy Rating
                (0x15f, false, "100".into(), "100".into()),                      // Health Remaining [M the literal 100]
                (0x160, false, time_s(st.time_s), time_s(t[7])),                 // Time
                (0x161, false, "0".into(), t[8].to_string()),                    // 007 Bonus
            ];
            for (k, (id, indent, cur, tgt)) in rows.iter().enumerate() {
                let name = if *indent { format!("  {}", lbl(*id, "")) } else { lbl(*id, "") };
                put(h, k + 2, &name, cur, tgt, cream);
            }
        }
        // fix-pause-menu2: the OBJECTIVES tab — one row per objective, a ~J (done, state 4) or
        // ~I (pending) marker then the objective's name [M C_GCPAUSE_Handler 0x204408:251-271 —
        // the rows are Mission_ObjectiveState + Txt_BindLabel(state label); the markers are
        // ACTION.ELF .data 0x30d2e0 "~J" / 0x30d2e8 "~I"; the specialchar 'J'/'I' glyphs draw
        // them [L]]. The host fills fe.pause_objectives from the sim's mission states.
        1 => {
            // The list (98,147,445×190) [L page 0x4000004b d1=8]: rows from its top, the marker column then the label
            // (PCSX2 m22, 2026-10-15: "• Breach the castle walls", the bullet at x 137, the text at 149, middle y 159);
            // the list's selection starts on the LAST row [M :271 msg 0x1d = count−1] and moves with ~V — the
            // yellow arrow follows fe.pause_objectives_sel (objective-hints).
            let n = fe.pause_objectives.len().min(8);
            for (i, o) in fe.pause_objectives.iter().enumerate().take(8) {
                let y = 159.0 + i as f32 * 22.0;
                let marker = if o.done { "~J" } else { "~I" };
                glyph_row(h, font, &format!("{marker} {}", lbl(o.text, "(objective)")), 135.0, y - 9.0, bright);
            }
            let sel = fe.pause_objectives_sel.min(n.saturating_sub(1));
            if n > 0 { pause_cursor(h, 107.0, 159.0 + sel as f32 * 22.0, gs(YELLOW, 0xff)); }
        }
        // The MISSION tab (default [L the page's initial state — the d1=6 list is the enabled one]).
        _ => {
            // The rows [L the d1=6 list (153,183,334×120) m0x11 labels 0x16a CONTINUE / 0x16b RESTART /
            // 0x16c QUIT; the row colour 0x7d6d5980 — the selected row at full alpha [S, the wheel's
            // pattern]].
            let rows = [0x16au32, 0x16b, 0x16c];
            let fallback = ["CONTINUE", "RESTART", "QUIT"];
            // Centred on the list; PCSX2 07000005: row centres at y 209 / 242 / 276, the arrow at the list's x + 8
            // (the fail list's arrow sits at its x + 8 too).
            // The confirm hides the list (PCSX2 m27: only the question and YES/NO show).
            let confirming = fe.confirming_restart || fe.confirming_quit;
            for (i, (&id, fb)) in rows.iter().zip(fallback).enumerate().filter(|_| !confirming) {
                let c = if i == fe.cursor { bright } else { dim };
                label(h, font, &lbl(id, fb), 153.0, 209.0 - 8.0 + i as f32 * 33.5, 334.0, 16.0, c, false, 1.0);
            }
            // fix-pause-menu2: the yellow arrow cursor left of the selected row [L the main agent's
            // PCSX2 captures 2026-10-13 — the game's list cursor; the port had none]. Drawn as a
            // small yellow triangle at the list's cursor column (the port's stand-in sprite; the
            // disc's own cursor texture was not identified — live check).
            if !confirming { pause_cursor(h, 153.0 + 9.0, 209.0 + fe.cursor.min(2) as f32 * 33.5, gs(YELLOW, 0xff)); }
        }
    }
    // The YES/NO confirm box over the rows [L the d1=0xc text (187,188,266×16) and the d1=0xd list
    // (238,210,165×86) m0x11 rows 0x5df YES / 0x5e0 NO; the text is 0x1ff "RESTART: ARE YOU SURE?" or
    // 0x200 "QUIT: ARE YOU SURE?" [M 0x204408's msg-0x4b d1=6 cases]; the selection opens on NO
    // [L m0x1d:1; M :181/204 re-sends msg 0x1d=1].]
    if fe.confirming_restart || fe.confirming_quit {
        let text = if fe.confirming_quit { lbl(0x200, "QUIT: ARE YOU SURE?") } else { lbl(0x1ff, "RESTART: ARE YOU SURE?") };
        // Each centred in its control (PCSX2 m27, 2026-10-15: the question's middle y 197, YES 237, NO 270, the
        // arrow at the list's x + 9 like the MISSION list).
        label(h, font, &text, 187.0, 188.0, 266.0, 16.0, bright, false, 1.0);
        let yn = [0x5dfu32, 0x5e0];
        for (i, &id) in yn.iter().enumerate() {
            let c = if i == fe.confirm_sel { bright } else { dim };
            label(h, font, &lbl(id, if i == 0 { "YES" } else { "NO" }), 238.0, 237.0 - 8.0 + i as f32 * 33.5, 165.0, 16.0, c, false, 1.0);
        }
        pause_cursor(h, 238.0 + 9.0, 237.0 + fe.confirm_sel.min(1) as f32 * 33.5, gs(YELLOW, 0xff));
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
    // Dark words without a shadow on the bright bar (PCSX2 m21–m24, 2026-10-15).
    hint_row_s(h, font, &hint_wrap(lang, hint_id, hint_fb), 45.0, 341.0 + 2.5, gs(0x0d0904, 0xff), None, false); // middle 352 (m21–m27)
}

/// fix-pause-menu2: a menu-space text row with the specialchar glyphs inline (the "~A Label" rows of
/// the CONTROLS/OBJECTIVES tabs). The game's list rows ARE menu text: `sprintf("%s %s", glyph, label)`
/// [M 0x1f44d8] rendered through the same text path as the hint bars — so the port reuses hint_bar's
/// tokenizer at a row pen (hint_bar already walks ~tokens with the SPECIALCHAR table).
fn glyph_row(h: &mut Hud, font: &nf_formats::hud::Font, text: &str, x640: f32, y640: f32, c: [f32; 4]) {
    hint_row(h, font, text, x640, y640, c, Some(x640));
}

/// The MISSION tab's row cursor: a small filled triangle pointing right at (x, y centre) [L the
/// captures' yellow arrow; the disc's own cursor sprite was not identified — the port's stand-in,
/// colour the title's yellow 0xfec43e [M page 0x40000009's 0x1b colour]].
fn pause_cursor(h: &mut Hud, x: f32, y: f32, c: [f32; 4]) {
    // PCSX2's arrow is ~15 wide × 16 tall (07000005 pause/fail, 2026-10-15).
    let (a, b, d) = (h.ndc(Anchor::Centre, mx(x), my(y - 8.0)), h.ndc(Anchor::Centre, mx(x), my(y + 8.0)), h.ndc(Anchor::Centre, mx(x + 15.0), my(y)));
    let p = |q: [f32; 2]| HudVertex { p: q, uv: [0.03, 0.03], c };
    // One triangle: the first 3 verts; the second is degenerate (the frame wants 6).
    let v = [p(a), p(b), p(d), p(a), p(d), p(d)];
    h.ndc_solid(-3, v, c);
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
            // The burning woman, as on the pause page (crate::woman; flags 0x100 additive) at the control's colour
            // 0x7f7f7f50 [L the 0x10000184 control's m0x1b/0x1c] — dimmer than full.
            h.quad(Anchor::Centre, 0, 0x0300_00db, 1, mx(385.0), my(121.0), mx(202.0), my(218.0), [0.0, 0.0, 128.0, 128.0], (false, false), gs(0x7f7f7f, 0x50));
            // The header (105,120,431×16) LBL 0x2fd "Mission Failed", colour 0x7d6d59 [L the page's 0x10000001 control
            // m0x18/0x1b], centred in the control (Label_Update puts the label at ctrl.x + w/2 [M 0x218cb8:177-182])
            // in the same face size as the hint bar (PCSX2 07000005: the header's glyphs match the hint's).
            centred(h, font, -2, &lbl(0x2fd, "MISSION FAILED"), mx(105.0 + 215.5), my(134.0), gs(0x7d6d59, 0xff), true, 1.25);
            // The rows: the 0x100001a4 list (197,186,246×115) holds exactly TWO items — m0x11 0x5bc "R E P L A Y"
            // (value 0) and 0x16c "Q U I T" (value 2) [L the mission menu-def, page 0x40000042]; the handler reads
            // the chosen item's VALUE (msg 0x35) [M P_ENDMISSION_Handler 0x216618:12-26], so the BaseMap arm 1 is
            // unreachable from this page. PCSX2 (07000005, 2026-10-15): REPLAY / QUIT, the yellow arrow left of the
            // selected row. Row pitch 40 like the pause list (the capture's two rows).
            let rows = [(0x5bcu32, "REPLAY"), (0x16c, "QUIT")];
            for (i, (id, fb)) in rows.iter().enumerate() {
                // The unselected row stays bright cream here (PCSX2 07000005 fail: REPLAY #dfe3e4, QUIT #e1dcc1), unlike
                // the pause list's dimmed rows.
                let c = if i == fe.cursor { bright } else { gs(0x7d6d59, 0xff) };
                // Centred on the list (Label_Update ctrl.x + w/2 [M 0x218cb8:177-182]); PCSX2 07000005: the row centres
                // at y 216 / 254 of 480, the arrow at x 205.
                centred(h, font, -2, &lbl(*id, fb), mx(197.0 + 123.0), my(223.0 + i as f32 * 38.0), c, false, 1.0);
            }
            pause_cursor(h, 205.0, 216.0 + fe.cursor.min(1) as f32 * 38.0, gs(YELLOW, 0xff));
            // The hint (97,341,446×18) LBL 0x5c8 [L the page's 0x1000006f].
            // The hint control (97,341,446×18) is centred on x = 320 like the 540-wide bars hint_bar assumes [L]:
            // hint_bar takes the 540-bar's left edge.
            // Dark words on the bright bar like the pause page's, middle y 352 (PCSX2 07000005 fail, 2026-10-15).
            hint_row_s(h, font, &hint_wrap(lang, 0x5c8, "~A Select  ~V Scroll"), 320.0 - 270.0, 341.0 + 2.5, gs(0x0d0904, 0xff), None, false);
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

/// The multiplayer wheels' preview textures, loaded after MENU_TEXTURES so the fixed indices stay put [L the MP
/// row tables' +0x00 hashes].
pub fn mp_textures() -> &'static [u32] {
    static T: std::sync::OnceLock<Vec<u32>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let mut v: Vec<u32> = MP_SCENARIO_ROWS.iter().map(|r| r.0).chain(MP_MAP_ROWS.iter().map(|r| r.0))
            .chain(MP_CHARACTERS.iter().map(|r| r.0)).chain(MP_OPTIONS_ROWS.iter().map(|r| r.0)).chain(MP_BOTS_ROWS.iter().map(|r| r.0))
            .chain([0x0300_0199u32, 0x0300_0159, 0x0300_015c, 0x0300_0169, 0x0300_01a0])   // the Join Game panels + slot images [L page 0x40000019]
            .filter(|h| *h != 0 && !MENU_TEXTURES.contains(h)).collect();
        v.sort_unstable(); v.dedup(); v
    })
}

/// A wheel over `n` rows centred on `cursor` (the MP pages keep their own row counters).
#[allow(clippy::too_many_arguments)]
fn wheel_at(h: &mut Hud, font: &nf_formats::hud::Font, n: usize, cursor: usize, label: impl Fn(usize) -> String, locked: impl Fn(usize) -> bool, base: [f32; 4]) {
    for d in -2i32..=2 {
        let r = cursor as i32 + d;
        if r < 0 || r >= n as i32 { continue; }
        let row = r as usize;
        let slot = (d + 2) as usize;
        let (mut c, scale) = match d { 0 => (base, 1.25), -1 | 1 => (dimmed(base, 0x80), 1.0), _ => (dimmed(base, 0x40), 1.0) };
        if locked(row) { c = gs(0x484848, match d { 0 => 0x80, -1 | 1 => 0x40, _ => 0x20 }); }
        left_text(h, font, &label(row), mx(WHEEL_X[slot]), my(WHEEL_Y[slot] + 8.5 + 6.0), c, true, scale);
    }
}

fn wheel_dim(h: &mut Hud, fe: &FrontEnd, font: &nf_formats::hud::Font, _lang: Option<&Lang>, label: impl Fn(&FrontEnd, usize) -> String, locked: impl Fn(&FrontEnd, usize) -> bool, base: [f32; 4]) {
    for (row, d) in fe.wheel() {
        let slot = (d + 2) as usize;
        let (mut c, scale) = match d {
            0 => (base, 1.25),
            -1 | 1 => (dimmed(base, 0x80), 1.0),
            _ => (dimmed(base, 0x40), 1.0),
        };
        // Locked rows go grey at half the slot's alpha [M Menu_UpdateWheel 0x1f8388:210-234: 0x48484880 selected,
        // 0x48484840 the neighbours, 0x48484820 the outer rows].
        if locked(fe, row) { c = gs(0x484848, match d { 0 => 0x80, -1 | 1 => 0x40, _ => 0x20 }); }
        let s = label(fe, row);
        // The label sprite centres in its control: x = ctrl.x + ctrl.w/2, y = ctrl.y + ctrl.h/2
        // [M Label_Update 0x218cb8:177-182 (x), :202-206 (y); the default format's fe 03].
        // Left-aligned at the control's x, vertically centred (PCSX2 07000048 codename/difficulty/missions,
        // 2026-10-15: the selected row's ink starts at x 303, the neighbours' at 331, middles at 162/193/221).
        let _ = WHEEL_W;
        left_text(h, font, &s, mx(WHEEL_X[slot]), my(WHEEL_Y[slot] + 8.5 + 6.0), c, true, scale);
    }
}

fn dimmed(mut c: [f32; 4], a: u32) -> [f32; 4] { c[3] = gs(0, a)[3]; c }

/// A wheel page's description, wrapped to 500 units and centred in its plate (50,301,540×113): the lines' middles at
/// pitch 28 around y 366 (PCSX2 07000048 codename/difficulty/missions, 2026-10-15: two lines at 352/380, one at 366).
fn desc_block(h: &mut Hud, font: &nf_formats::hud::Font, text: &str, c: [f32; 4]) {
    // The wrap width is 505 640-space units: PCSX2 keeps the 502-unit codename line ("…not be able to") and breaks the
    // 509-unit Skyrail line before "deserted" (m06, m47).
    let lines = wrap_px(text, font, mx(505.0));
    let n = lines.len().min(4);
    for (i, line) in lines.iter().take(4).enumerate() {
        let mid = 366.0 + (i as f32 - (n as f32 - 1.0) / 2.0) * 28.0;
        centred(h, font, -2, line, mx(320.0), my(mid + 6.0), c, true, 1.0);
    }
}

/// The codename option pages' rows [L pages 0x4000002d / 2e / 31: the label controls at x 54-57 (301 wide, 16
/// tall), the value controls at x 348-351 (219 wide; the sliders' bars (365,86/114,187×16)), the description plate
/// (50,355,540×40); the selected row 0x7d6d59ff, the others 0x73330fc0 (the controls' msg 0x1b / 0x1c)]. PCSX2
/// m32/m38/m40 (2026-10-15): the ◀ ▶ arrows at x 357 / 555 around the selected row's value.
fn opt_list(h: &mut Hud, fe: &FrontEnd, font: &nf_formats::hud::Font, lang: Option<&Lang>) {
    let bind = |id: u32| -> String { lang.and_then(|l| l.bind(id)).unwrap_or("").to_string() };
    let rows = fe.opt_rows();
    let (lx, sc) = match fe.page { Page::CnAv => (56.0, 1.22), _ => (54.0, 1.3) };
    let on = gs(CREAM, 0xff);
    let off = gs(0x73330f, 0xc0);
    let yellow = gs(0xfec43e, 0xff);
    let arrow = |h: &mut Hud, cx: f32, cy: f32, left: bool| {
        let (a, b, d) = if left { ((cx + 7.0, cy - 7.0), (cx + 7.0, cy + 7.0), (cx - 7.0, cy)) } else { ((cx - 7.0, cy - 7.0), (cx - 7.0, cy + 7.0), (cx + 7.0, cy)) };
        let q = |p: (f32, f32), h: &Hud| HudVertex { p: h.ndc(Anchor::Centre, mx(p.0), my(p.1)), uv: [0.0, 0.0], c: yellow };
        let v = [q(a, h), q(b, h), q(d, h), q(a, h), q(d, h), q(d, h)];
        h.ndc_solid(-3, v, yellow);
    };
    let rect = |h: &mut Hud, x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 4]| {
        let (a, b) = (h.ndc(Anchor::Centre, mx(x0), my(y0)), h.ndc(Anchor::Centre, mx(x1), my(y1)));
        let q = |p: [f32; 2]| HudVertex { p, uv: [0.0, 0.0], c };
        h.ndc_solid(-2, [q([a[0], a[1]]), q([b[0], a[1]]), q([b[0], b[1]]), q([a[0], a[1]]), q([b[0], b[1]]), q([a[0], b[1]])], c);
    };
    for (i, r) in rows.iter().enumerate() {
        let sel = i == fe.cursor;
        let c = if sel { on } else { off };
        let mid = r.y + 8.0;
        left_text(h, font, &bind(r.label), mx(lx), my(mid + 6.0), c, true, sc);
        let v = fe.opt_value(r.ctrl);
        match r.kind {
            OptKind::List => {
                let it = r.items.iter().find(|it| it.2 == v).or(r.items.first());
                if let Some(it) = it {
                    let txt = if it.0 != 0 { bind(it.0) } else { it.1.to_string() };
                    centred(h, font, -2, &txt, mx(457.0), my(mid + 6.0), c, true, sc);
                }
                if sel { arrow(h, 357.0, mid, true); arrow(h, 555.0, mid, false); }
            }
            OptKind::Slider => {
                // The bar: a pale track with the yellow fill to the value (PCSX2 m38).
                let (x0, x1) = (365.0, 552.0);
                // PCSX2 m38: track #c8c8c8, fill #d4952b.
                rect(h, x0, mid - 7.0, x1, mid + 7.0, gs(0x646464, 0xff));
                rect(h, x0, mid - 7.0, x0 + (x1 - x0) * v as f32 / 100.0, mid + 7.0, gs(0x6a4a15, 0xff));
                if sel { arrow(h, 357.0, mid, true); arrow(h, 560.0, mid, false); }
            }
            OptKind::Action => {}
        }
    }
    // The description of the selected row at its current value [L table 0x2e4a18], centred in the plate.
    if let Some(r) = rows.get(fe.cursor) {
        if let Some(d) = opt_desc(r.ctrl, fe.opt_value(r.ctrl)) {
            centred(h, font, -2, &bind(d), mx(320.0), my(375.0 + 6.0), gs(CREAM, 0xff), true, 1.0);
        }
    }
}

/// Left-aligned text: the first glyph's ink starts at screen x `x`.
fn left_text(h: &mut Hud, font: &nf_formats::hud::Font, s: &str, x: f32, y: f32, c: [f32; 4], shadow: bool, scale: f32) {
    let x0 = font.layout(s, 0.0, 0.0).0.first().map(|p| p.x).unwrap_or(0.0) * scale;
    h.text_scaled(Anchor::Centre, font, -2, s, x - x0, y, false, c, shadow, scale);
}

/// Centre one text row at 640-space x `xc` [M `__Font_DrawText` 0x1eee48:69-71 — the format's fe 03
/// flag 2 applies `x -= width * 0.5`; every menu label carries fe 03 (Label_Init 0x218638:53's
/// DAT_003096c0). The port's `text_scaled` pen starts at x, so the half-width shift lands here.]
/// A label centred in its control rect (640-space x, y, w, h) like `Label_Update` 0x218cb8 — horizontally at
/// x + w/2 (:177-182) and vertically on y + h/2 (:202-206). `centred`'s pen y sits ~6 units above the glyphs'
/// middle (measured against PCSX2's pause/fail captures, 2026-10-15).
fn label(h: &mut Hud, font: &nf_formats::hud::Font, s: &str, x: f32, y: f32, w: f32, hh: f32, c: [f32; 4], shadow: bool, scale: f32) {
    centred(h, font, -2, s, mx(x + w / 2.0), my(y + hh / 2.0 + 6.0), c, shadow, scale);
}

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
    // Left-aligned at the control's x 50 at the face's own size, middle on y 48.5 (PCSX2 07000048: every header's ink
    // starts at x 51; "Select Codename" measures 261 units).
    let x0 = hfont.layout(s, 0.0, 0.0).0.first().map(|p| p.x).unwrap_or(0.0);
    h.text_scaled_tex(Anchor::Centre, hfont, -1, 0x0300_0005, s, mx(50.0) - x0, my(48.5 + 6.0), false, gs(CREAM, 0xff), true, 1.0);
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
    // The ring texture 0x030000bf only masks the preview to the disc: PCSX2 shows the preview filling the 150-unit
    // circle with no rim or square of its own (07000048 codename/difficulty/missions, 2026-10-15).
    let _ = (ring, ring_c);
    circle_clipped_quad(h, preview, 114.0, 118.0, 150.0, 150.0, [0.0, 0.0, 128.0, 128.0], c);
}

/// A textured quad clipped to the rect's inscribed circle: 24 vertical strips, each the chord of the
/// circle at that x range, UVs proportional. The game masks the preview to the ring (the emblem
/// draw order + the square-box render); a `HudFrame` has no per-pixel mask, so the port clips
/// geometrically — 24 strips' chord steps are under 0.1 px at the 150-px emblem.
fn circle_clipped_quad(h: &mut Hud, tex: u32, x: f32, y: f32, w: f32, hh: f32, uv: [f32; 4], c: [f32; 4]) {
    let n = 96;
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
    quad640(h, 0x0300_016d, 434.0, 38.0, 156.0, 43.0, [0.0, 0.0, 256.0, 128.0], gs(0x7f7f7f, 0xff));
}

/// The "Play without saving" confirm box [M the pages' plate controls (50,301,540×113) LBL
/// runtime-filled; the text Txt 0x10000bb].
fn confirm_box(h: &mut Hud, font: &nf_formats::hud::Font, text: &str, opt: &str, c: [f32; 4]) {
    // PCSX2 m07 (2026-10-15): a translucent grey box (51..589 × 146..333) centred on y 240 over the page, the lines
    // centred on x 320 at pitch 19 (blank lines kept), wrapped near 470 units; then the option line (~A/~B glyphs
    // inline) 37 below the last [M Menu_CreateOptionBox 0x1f9608: ok-only box, option 0x10001d6; height = memo +
    // options·0x15 + 0x36, centred vertically: (448 − h)/2].
    let lines = wrap_px(text, font, mx(470.0));
    let n = lines.len().min(9) as f32;
    let hh = n * 19.0 + 18.0 + 55.0;
    let top = 240.0 - hh / 2.0;
    let (a, b) = (h.ndc(Anchor::Centre, mx(51.0), my(top)), h.ndc(Anchor::Centre, mx(589.0), my(top + hh)));
    let gc = [0.5, 0.5, 0.5, 0.45];
    let q = |p: [f32; 2]| HudVertex { p, uv: [0.0, 0.0], c: gc };
    h.ndc_solid(-1, [q([a[0], a[1]]), q([b[0], a[1]]), q([b[0], b[1]]), q([a[0], a[1]]), q([b[0], b[1]]), q([a[0], b[1]])], gc);
    for (i, line) in lines.iter().take(9).enumerate() {
        let mid = top + 22.0 + i as f32 * 19.0;
        hint_row(h, font, line, 50.0, mid - 9.0, c, None);
    }
    hint_row(h, font, opt, 50.0, top + 22.0 + (n - 1.0) * 19.0 + 37.0 - 9.0, c, None);
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
    // The words' middle sits at the bar's y + h/2 + 2.5 (PCSX2 m03/m05/m06/m10/m12/m15: 432 for the 21-tall bars at 419).
    hint_row(h, font, text, x640, y640 + 4.5, c, None);
}

/// One "~X label" row: centred on the 540-wide bar at `x640` (hint bars), or left-aligned with its first ink at
/// `left` (640-space; the controls list's columns).
fn hint_row(h: &mut Hud, font: &nf_formats::hud::Font, text: &str, x640: f32, y640: f32, c: [f32; 4], left: Option<f32>) {
    hint_row_s(h, font, text, x640, y640, c, left, true);
}

/// `hint_row` with the text shadow switchable (the pause bar's dark words have none).
#[allow(clippy::too_many_arguments)]
fn hint_row_s(h: &mut Hud, font: &nf_formats::hud::Font, text: &str, x640: f32, y640: f32, c: [f32; 4], left: Option<f32>, shadow: bool) {
    // One pass collects each item's ink span relative to the row's pen origin (text glyphs sit at
    // [p0.x − gw0/2, plast.x + gwlast/2] — the per-glyph quads centre at their pen x — and an icon's
    // quad centres at its pen x, so [−iw/2, +iw/2]); the row's ink centre then goes to the bar's
    // centre [M Label_Update 0x218cb8:177-182 puts the label sprite at ctrl.x + ctrl.w/2].
    enum Item { Text { x: f32, ink: (f32, f32) }, Icon { tex: u32, uv: [f32; 4], w: f32, hh: f32, ink: (f32, f32) } }
    let mut items: Vec<Item> = Vec::new();
    let mut adv_sum = 0.0f32;
    // A word that follows an icon starts ~6 units after the icon's ink (PCSX2 m03/m06/m23, 2026-10-15: 6–8 units).
    let mut after_icon: Option<f32> = None;
    let mut rest = text;
    while let Some(t) = rest.find('~') {
        let (before, after) = rest.split_at(t);
        if !before.is_empty() {
            let (placed, nat) = font.layout(before, 0.0, 0.0);
            let (a, b) = match (placed.first(), placed.last()) {
                (Some(p0), Some(pl)) => (p0.x, pl.x + font.glyphs[pl.glyph].w as f32),
                _ => (0.0, nat),
            };
            if let Some(e) = after_icon.take() { adv_sum = e + mx(6.0) - a; }
            items.push(Item::Text { x: adv_sum, ink: (adv_sum + a, adv_sum + b) });
            // A word's trailing spaces don't advance the pen far enough before the next icon: PCSX2 leaves ~10 units
            // from the word's last ink to the icon (07000005 fail menu, 2026-10-15).
            adv_sum = (adv_sum + nat).max(adv_sum + b + mx(10.0));
        }
        match after.chars().nth(1).and_then(|t| SPECIALCHAR.iter().find(|s| s.0 == t)) {
            Some(&(_, tex, u, v, w, hh, adv)) => {
                let (iw, ih) = (mx(w * ICON_SCALE), my(hh * ICON_SCALE));
                // The icon occupies [pen, pen + iw] and its word follows ~6 units after (PCSX2 07000005 fail
                // menu 2026-10-15: "✕ Select" — a 6-unit gap; the table's advance alone overlapped the next word).
                let _ = adv;
                items.push(Item::Icon { tex, uv: [u, v, w, hh], w: iw, hh: ih, ink: (adv_sum, adv_sum + iw) });
                after_icon = Some(adv_sum + iw);
                adv_sum += iw + mx(6.0);
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
        if let Some(e) = after_icon.take() { adv_sum = e + mx(6.0) - a; }
        items.push(Item::Text { x: adv_sum, ink: (adv_sum + a, adv_sum + b) });
    }
    let ink_lo = items.iter().map(|i| match i { Item::Text { ink, .. } | Item::Icon { ink, .. } => ink.0 }).fold(f32::MAX, f32::min);
    let ink_hi = items.iter().map(|i| match i { Item::Text { ink, .. } | Item::Icon { ink, .. } => ink.1 }).fold(f32::MIN, f32::max);
    // The bar's centre: ctrl.x + ctrl.w/2 = 50 + 540/2 = 320 in 640-space [L the parsed bars].
    let bar_c = mx(x640 + 270.0);
    let pen0 = match left { Some(lx) => mx(lx) - ink_lo, None => bar_c - (ink_lo + ink_hi) / 2.0 };
    if std::env::var_os("NF_DEBUG_HINT").is_some() { eprintln!("[hint] text={text:?} items={} ink=({ink_lo},{ink_hi}) bar_c={bar_c} pen0={pen0}", items.len()); }
    // Label_Update centres the label vertically in its 18-px control [M 0x218cb8:202-206]: PCSX2 puts every hint
    // row's centre ~y + 9 (main menu 430, pause/fail 350 of 480 — 2026-10-13/15 captures); the port's text pen
    // drew it ~16 higher.
    let y = my(y640 + 15.0);
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
            if let Item::Text { x, .. } = &items[ti.min(items.len() - 1)] { seg_pen = *x; }
            let x = pen0 + seg_pen;
            h.text(Anchor::Centre, font, -2, before, x, y, false, c, shadow);
            let (placed, nat) = font.layout(before, 0.0, 0.0);
            let ink_end = placed.last().map_or(nat, |pl| pl.x + font.glyphs[pl.glyph].w as f32);
            seg_pen = (seg_pen + nat).max(seg_pen + ink_end + mx(10.0));
            ti += 1;
        }
        match after.chars().nth(1).and_then(|t| SPECIALCHAR.iter().find(|s| s.0 == t)) {
            Some(&(_, tex, u, v, w, hh, adv)) => {
                if let Item::Icon { ink, .. } = &items[ti.min(items.len() - 1)] {
                    let (lo, hi) = (ink.0, ink.1);
                    let _ = (lo, hi);
                }
                let iw = mx(w * ICON_SCALE);
                let ih = my(hh * ICON_SCALE);
                // The icon's ink [pen − iw/2, pen + iw/2] — same anchor as the measure pass.
                // Level with the text's middle (PCSX2: the ✕/scroll icons sit level with their words; drawn from
                // y − ih + 12 they hung ~8 low).
                // The icons keep their own colours when the words are dark (the pause bar).
                let ic = if shadow { c } else { gs(0x7f7f7f, 0xff) };
                // Centred on the words' middle (the pen y sits ~6.5 below it): the 20-unit buttons land where they did, the small
                // objective bullets level with their text (PCSX2 m22).
                h.quad(Anchor::Centre, -2, tex, 0, pen0 + seg_pen, y - my(6.5) - ih / 2.0, iw, ih, [u, v, w, hh], (false, false), ic);
                let _ = adv;
                seg_pen += iw + mx(6.0);
                ti += 1;
            }
            _ => { seg_pen += mx(16.0); }
        }
        rest = &after[2.min(after.len())..];
        if rest.starts_with(' ') { rest = &rest[1..]; seg_pen += 4.0; }
    }
    if !rest.is_empty() {
        if let Some(Item::Text { x, .. }) = items.last() { seg_pen = *x; }
        h.text(Anchor::Centre, font, -2, rest, pen0 + seg_pen, y, false, c, shadow);
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
/// A 640-space framed box: a translucent black fill and a thin dark border (the b0=1 frame controls, PCSX2 07000048
/// Records 2026-10-13).
fn frame640(h: &mut Hud, x: f32, y: f32, w: f32, hh: f32) {
    let rect = |h: &mut Hud, x0: f32, y0: f32, x1: f32, y1: f32, layer: i32, c: [f32; 4]| {
        let (a, b) = (h.ndc(Anchor::Centre, mx(x0), my(y0)), h.ndc(Anchor::Centre, mx(x1), my(y1)));
        let q = |p: [f32; 2]| HudVertex { p, uv: [0.0, 0.0], c };
        let v = [q([a[0], a[1]]), q([b[0], a[1]]), q([b[0], b[1]]), q([a[0], a[1]]), q([b[0], b[1]]), q([a[0], b[1]])];
        h.ndc_solid(layer, v, c);
    };
    rect(h, x, y, x + w, y + hh, -1, [0.0, 0.0, 0.0, 0.55]);
    let e = [0.04, 0.02, 0.01, 0.9];
    rect(h, x, y, x + w, y + 1.5, -1, e);
    rect(h, x, y + hh - 1.5, x + w, y + hh, -1, e);
    rect(h, x, y, x + 1.5, y + hh, -1, e);
    rect(h, x + w - 1.5, y, x + w, y + hh, -1, e);
}

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
        KeyCode::Enter | KeyCode::Space => Action::Confirm,
        // ◀ / ▶: the option pages change values with them; every other page reads them as Back / Confirm (action()).
        KeyCode::ArrowRight | KeyCode::KeyD => Action::Right,
        KeyCode::ArrowLeft | KeyCode::KeyA => Action::Left,
        KeyCode::Escape | KeyCode::Backspace => Action::Back,
        KeyCode::Tab | KeyCode::KeyE => Action::Side,
        _ => return None,
    })
}

/// The level loading screen's text and dots over its picture (the picture itself goes on the movie plane) [M
/// `ResetMap_GenLoadScreen` 0x1d5de0 — see crate::loading]. Positions measured on PCSX2 07000005 (m18, 2026-10-15)
/// agree with the game's 512×448 sprite coordinates: LOADING centred on x 310 (248·1.25), middle y 339; OBJECTIVE
/// from x 35 (28·1.25), middle y 371; the hint 20 below at pitch 16, wrapped to 570 (456·1.25); six dots from x 69
/// (55·1.25) at pitch 20, middle y 339.
pub fn loading_overlay(font: &nf_formats::hud::Font, lang: Option<&Lang>, textures: &[(u32, crate::scene::TextureImage)], aspect: f32, level: u32, hint: Option<u32>, lit: u32) -> HudFrame {
    let mut h = Hud { quads: Vec::new(), aspect, textures };
    let bind = |id: u32| -> String { lang.and_then(|l| l.bind(id)).unwrap_or("").to_string() };
    let cream = gs(CREAM, 0xff);
    let gold = gs(0x785a14, 0xff);
    centred(&mut h, font, -2, &bind(0x3c1), mx(310.0), my(339.0 + 6.0), cream, false, 1.0);
    // "\xBA label \xBA text": the label in gold, a space, the text in cream; wrapped lines continue at x 35.
    let mut y = 371.0f32;
    let mut line = |h: &mut Hud, label: &str, text: &str, y: &mut f32| {
        let first = format!("{label} {text}");
        // PCSX2 draws these lines ~5% wider than the port's face at 1.0 ("OBJECTIVE Breach the castle walls" 311 vs
        // 295 units) and breaks the level-05 hint after "objective,".
        const SC: f32 = 1.05;
        for (i, l) in wrap_px(&first, font, mx(545.0) / SC).iter().enumerate() {
            let pen = my(*y + 6.0);
            if i == 0 && l.starts_with(label) {
                h.text_scaled(Anchor::Centre, font, -2, label, mx(35.0), pen, false, gold, false, SC);
                let lw = font.layout(&format!("{label} "), 0.0, 0.0).1 * SC;
                h.text_scaled(Anchor::Centre, font, -2, l[label.len()..].trim_start(), mx(35.0) + lw, pen, false, cream, false, SC);
            } else {
                h.text_scaled(Anchor::Centre, font, -2, l, mx(35.0), pen, false, cream, false, SC);
            }
            *y += 16.0;
        }
    };
    if let Some(o) = crate::loading::objective_for(level) {
        line(&mut h, &bind(0x200_0054), &bind(o), &mut y);
        y += 4.0;
    }
    if let Some(hi) = hint {
        let s = bind(hi);
        line(&mut h, &bind(0x200_0055), crate::loading::strip_hint(&s), &mut y);
    }
    // The dots: the disc texture tinted (0xee,0xa0,0x29) for the first `lit`, (0x5c,0x31,0x0c) for the rest [M
    // `CreateLoadingBlob` 0x11bf70]; drawn as discs the colours PCSX2 shows (m18: lit #cfcf50, dim #924c10, ~12 units
    // across, centres from x 77 at pitch 20).
    for i in 0..6u32 {
        let c = if i < lit { gs(0x676728, 0xff) } else { gs(0x492608, 0xff) };
        let (cx, cy, r) = (77.0 + 20.0 * i as f32, 338.5, 6.0);
        let centre = h.ndc(Anchor::Centre, mx(cx), my(cy));
        let n = 16;
        for k in 0..n {
            let a0 = k as f32 / n as f32 * std::f32::consts::TAU;
            let a1 = (k + 1) as f32 / n as f32 * std::f32::consts::TAU;
            let p = |a: f32, h: &Hud| h.ndc(Anchor::Centre, mx(cx + r * a.cos()), my(cy + r * a.sin()));
            let q = |p: [f32; 2]| HudVertex { p, uv: [0.0, 0.0], c };
            let (p0, p1) = (p(a0, &h), p(a1, &h));
            h.ndc_solid(-2, [q(centre), q(p0), q(p1), q(centre), q(p1), q(p1)], c);
        }
    }
    h.finish()
}

#[cfg(test)]
mod secret_grid_tests {
    use super::*;

    /// The grid flow: type BOWLER through the cells, submit, and the unlock lands in the
    /// progress's bonus[0] bit 38 (the sister: a miss leaves it zero [M 0x213a98:67-73]).
    #[test]
    fn grid_types_and_submits_a_code() {
        let mut fe = FrontEnd::new();
        fe.page = Page::CnName;
        let mut p = nf_game::progress::Progress::default();
        // The cursor walks to B (cell 10), presses, and the letter appends.
        for (want, cell) in [('B', 11), ('O', 24), ('W', 32), ('L', 21), ('E', 14), ('R', 27)] {
            fe.secret.cursor = cell;
            assert!(matches!(SECRET_GRID[cell], SecretKey::Char(c) if c == want));
            assert!(fe.secret_press());
        }
        assert_eq!(fe.secret.typed, "BOWLER");
        fe.secret.cursor = 38; // the Submit cell [M 0x213a98:45-74]
        assert!(fe.secret_press());
        assert!(fe.secret.pending_submit);
        assert!(fe.secret_submit(&mut p));
        assert!(fe.secret.accepted);
        assert_eq!(p.bonus[0], 1 << 38);
        // The miss path: a wrong string drains silently and unlocks nothing.
        let mut q = nf_game::progress::Progress::default();
        fe.secret = SecretEntry::default();
        fe.page = Page::CnName;
        fe.secret.cursor = 10; // 'B'
        fe.secret_press();
        fe.secret.cursor = 38;
        fe.secret_press();
        assert!(fe.secret_submit(&mut q) == false);
        assert!(!fe.secret.accepted);
        assert_eq!(q.bonus[0], 0);
    }

    /// The 8-char cap [M 0x213a98:29-31] and the Delete cell [M :39-43].
    #[test]
    fn grid_caps_and_deletes() {
        let mut fe = FrontEnd::new();
        fe.secret.cursor = 10;
        for _ in 0..9 { fe.secret_press(); }
        assert_eq!(fe.secret.typed.chars().count(), 8);
        fe.secret.cursor = 36; // Delete
        assert!(fe.secret_press());
        assert_eq!(fe.secret.typed.chars().count(), 7);
        fe.secret.cursor = 37; // Space
        assert!(fe.secret_press());
        assert_eq!(fe.secret.typed.chars().count(), 8);
    }

    /// The wheel routes row 0 to the grid page [M C_SBCNOPTIONS 0x213528 case 0] and Back
    /// returns to it.
    #[test]
    fn cnmenu_row0_opens_the_grid() {
        let mut fe = FrontEnd::new();
        fe.page = Page::CnMenu;
        fe.cursor = 0;
        assert!(fe.action(Action::Confirm));
        assert_eq!(fe.page, Page::CnName);
        assert_eq!(fe.page_id(), PAGE_CN_NAME);
        assert!(fe.action(Action::Back));
        assert_eq!(fe.page, Page::CnMenu);
    }
}
