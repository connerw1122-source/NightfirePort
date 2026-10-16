//! The MP match core — multiplayer phase 1 (`docs/atlas/multiplayer.md` §6 phase 1): `MP_Init`
//! 0x183438, `MP_Start` 0x183a50, `MP_Update` 0x184bd0, `MPSettings` @0x2a47a0, `MPGame` @0x2a4980,
//! the spawn-point table (`MP_RegisterSpawnPoint` 0x184f20, 64 × 0x30 bytes @0x2a7360) and the spawn
//! policy (`MP_GetSpawnPoint` 0x185098, the mode at `MPSettings+0x448`), plus the MP-object
//! registration (`MP_RegisterMPObject` 0x185990 — stored, not yet driven).
//!
//! The per-scenario rules layer (scoring, win conditions, flag/uplink/objective logic, respawn,
//! friendly fire) lives in [`rules`] — see `docs/systems/mp-match-rules.md` for the tagged rule
//! list. Offset note: the field comments below carry the decompiler's **decimal** prints
//! (`MPSettings._412_4_` = hex +0x19C); the game's own hex offsets are in the systems doc.
//!
//! Everything here follows the game's arithmetic; deviations are marked *Deviation* and are model
//! gaps, not silent rewrites: the round clock runs on the fixed 60 Hz tick (the game samples
//! `psiGetTimeIn100ths`, a wall clock — the determinism rule forbids that in the port's Sim; the
//! game's own state-3/6 timers in `MP_Update` use `REC_FRAME_RATE` the same way), and
//! `MP_CreateObject` (the immediate-create path) stores the record without building an object
//! (phase 1 drives no MP objects).
//!
//! Match end + debriefing (mp-debriefing-data2, on top of 402's machine): the full
//! `MP_CheckForEndCondition` 0x184268 latch/dispatch, the `MP_Update` 0x184bd0 end states with the
//! handover/debriefing/restart requests, `MP_SortOutWhoWon` 0x1848d0's end status, and
//! [`debrief`]'s board (`P_MPDEBRIEFING` 0x212198) — see `docs/systems/mp-debriefing.md` and the
//! reconciliation table in `docs/reports/mp-debriefing-data2.md`.
//!
//! Placement wiring ([M] `parsemap_create_dynamic_objects` 0x1d04f0 case 0x25/0x26 ←
//! `parsemap_block_map_data_dynamic` 0x1d1b68): the dispatch passes the placement's translation,
//! rotation and its 0xb0 params block (values written by key: value[key] lands at block+0x2c+key·4)
//! — so the **team** (0x25) and the **MP object type** (0x26) are the placement's **param 0**
//! (u16/u32 at block+0x2c), and the stored "tag" pointer is the placement's cel record
//! (`pCelList + idx·0x50`). Both registrations gate on the MP master enable (`MPSettings+0x384`).

use crate::drone::Rng;
use crate::world::CollisionWorld;

/// Bots groundwork (401 mp-bots-data): the per-bot config records, the `default_bot_stats` presets, the bot
/// state table and the `BotGlobal` dispatch skeleton — see `bots.rs` and `docs/fields/bot-vars.md`. Nothing in
/// the sim calls it yet; the dispatch is behind the `mp-bots` feature.
pub mod bots;

/// The debriefing data builder (mp-debriefing-data2, ported from 621): `MpMatch::get_mp_score` /
/// `MpMatch::debriefing` — the `P_MPDEBRIEFING_Handler` 0x212198 msg-0x4c rows, ranks and result
/// line. See `docs/systems/mp-debriefing.md` §4.
pub mod debrief;

/// The lockstep state hash (`state_hash(&MpMatch) -> u64`, FNV-1a 64 over the state's bit
/// patterns) — the desync detector the future `nf-net` UDP lockstep exchanges per tick. See
/// `docs/systems/net-lockstep.md`.
pub mod hash;

/// The recorded-input replay harness for the MP core: settings + seed + per-tick events in, the
/// same sim calls the live host makes, per-tick hashes out.
pub mod replay;

/// The per-scenario rules (402 mp-match-rules): kill scoring, the end-condition latches and the
/// state machine, the who-won decision, respawn rules, friendly fire, and every scenario's
/// objective update (CTF flags, Uplink terminals, KOTH hill, Demolition/Protection sites,
/// GoldenEye items, the blueprint) — see `docs/systems/mp-match-rules.md`.
pub mod rules;
pub mod hud;

pub use rules::{MpObjState, MpObjectsRt, Winner};

/// The scenario words (`MPSettings+0x420`, atlas §3) — phase 1 only constructs Deathmatch; the rest
/// name the ids the registration gates on.
pub mod scenario {
    pub const DEATHMATCH: u32 = 0x1;
    pub const TEAM_DEATHMATCH: u32 = 0x2000_0002;
    pub const CAPTURE_THE_FLAG: u32 = 0x2000_0004;
    pub const DEATH_LIMIT_FFA: u32 = 0x10;
    pub const DEMOLITION: u32 = 0x2000_0040;
    pub const PROTECTION: u32 = 0x2000_0080;
    pub const BLUEPRINT: u32 = 0x2000_0100;
    pub const GOLDENEYE: u32 = 0x2000_0200;
    pub const ASSASSIN: u32 = 0x400;
    pub const KOTH_FFA: u32 = 0x4000_0800;
    pub const KOTH_TEAM: u32 = 0x6000_1000;
    pub const UPLINK: u32 = 0x6000_0008;
}

/// One of the 8 per-player records at `MPSettings+0x000..0x17F` (0x30 stride): humans 0..3, bots
/// 4..7. +0x00 name, +0x20 team (u32), +0x24 skin, +0x28 saved-flag bit, +0x2c armor bonus
/// (health = +100 at respawn) [M atlas §2].
#[derive(Clone, Debug)]
pub struct MpPlayer {
    /// +0x20 (u32): the team; the is-target stand-in in Assassin.
    pub team: u32,
    /// +0x24: the skin index (`MP_skins[skin]` at `Player_Init` 0x1910f8).
    pub skin: u32,
    /// +0x2c: the armour bonus.
    pub armor_bonus: u32,
    /// +0x28 (u32): THIS player's radar toggle — 0 = the radar returns nothing for them and
    /// their ring sprite hides [M MP_GetRadarObjects 0x189840:22-24 (0 → an empty list),
    /// HUD_RadarUpdate 0x19f6e8:102-105 (the ring's layer 0xff)]. New field on its own line
    /// [WORKLOG lesson 4]; the game's record is 0x30 wide, the port's compact struct skipped
    /// +0x28 (its armor_bonus is +0x2c).
    pub radar_on: u32,
    /// +0x00: the codename string (`C_RBMPCNAME` 0x20d448 stores it; the debriefing's name column
    /// and the winner line read `MPSettings+s·0x30+0x00` [M 0x212198:169-171,
    /// 0x212a64-0x212a7c]). The boot default is "Player N" [M bootup_bootup 0x1c7e78:50-67].
    pub codename: String,
}

/// `MPSettings` @0x2a47a0 — the match config. Only the fields the phase reads/writes are carried;
/// every one carries its game offset. Saved/loaded by `LS_MakeMPSettings` 0x1ce508 (the first 0x1dc
/// bytes are the menu-persisted part [M]).
#[derive(Clone, Debug)]
pub struct MpSettings {
    /// +0x000..0x17F: the 8 per-player records.
    pub players: [MpPlayer; 8],
    /// +0x188 (u8): the started flag — set by `MP_Start` 0x183a50's tail.
    pub started: bool,
    /// +0x1A4: the gametype bitmask (the bot goal scoring).
    pub gametype_mask: u32,
    /// +0x1AC (u16): the player count (= the viewports).
    pub player_count: u16,
    /// +0x384: the MP master enable — `MP_Start`/`MP_Update` and both placement registrations
    /// return when it is 0 [M 0x183a50:72-74, 0x1d04f0 case 0x25].
    pub master: u32,
    /// +0x388: `MP_Start` copies the master enable here [M 0x183a50:70].
    pub master_copy: u32,
    /// +0x396: the team game — derived by `MP_Init` as `(scenario >> 29) & 1` [M 0x183438:41].
    pub team_game: u32,
    /// +0x400: the bit30 flag — `(scenario >> 30) & 1` [M 0x183438:42]; gates kill-scoring.
    pub bit30: u32,
    /// +0x404 (settings): the total player count, written by `MP_Start` [M 0x183a50:164].
    pub total_players: u32,
    /// +0x412: the frag/score limit, 1..120.
    pub frag_limit: i32,
    /// +0x416: the time limit, 1..60, −1 = none (units: minutes [S — the live check owns the
    /// conversion; the port carries the value unchanged, as `MP_Init` does into `MPGame+0x404`]).
    pub time_limit: i32,
    /// +0x420: the scenario word (the mode id).
    pub scenario: u32,
    /// +0x424: the level code (the selected arena).
    pub level_code: u32,
    /// +0x428: the human count — `MP_Start` inits this many players [M 0x183a50:116-128].
    pub human_count: u32,
    /// +0x432: the bot count, clamped ≤ 4 [M 0x183a50:133-135] (phase 1: 0 — no bots).
    pub bot_count: u32,
    /// +0x436: the weapon set 0..10 (the PickupMatrix row).
    pub weapon_set: u32,
    /// +0x444: the capture-weapon reward.
    pub capture_reward: u32,
    /// +0x448: the spawn selection policy — see `MpMatch::get_spawn_point` for what the game's
    /// arithmetic actually does per value.
    pub spawn_policy: u32,
    /// +0x452: the radar on.
    pub radar: u32,
    /// +0x472 (u16): the registered pickup count.
    pub registered_pickups: u16,
    /// +0x198 hex (Ghidra `_408_`): the friendly-fire enable in team games — 0 blocks same-team
    /// bullet hits in `MP_RegisterBulletHit` 0x1857c0:43-57 [M]. The menu's Player Mods row 0x1ae
    /// writes it [M mp-menus.md].
    pub friendly_fire: u32,
    /// Port-only ([S], no game field): the deterministic lockstep seed — every machine in an
    /// online match builds the level from the same seed (net-match-host). The game's local flow
    /// keeps the level-build constant the port used before (0x1234_5678 equivalent: seed 0 keeps
    /// `Rng::new`'s mixing for local play — see `net-match-host.md`'s local-compat note).
    pub seed: u32,
}

impl MpSettings {
    /// The phase-1 default for `--mp <arena>`: an Arena-1-style deathmatch. The menu's own defaults
    /// are not read here (no menu path in phase 1) — the values are the port's [S], the structure
    /// and the derivations are the game's.
    pub fn deathmatch(level_code: u32) -> MpSettings {
        MpSettings {
            players: std::array::from_fn(|_| MpPlayer { team: 0, skin: 0, armor_bonus: 0, radar_on: 1, codename: String::new() }),
            started: false,
            gametype_mask: 0,
            player_count: 1,
            master: 1,
            master_copy: 0,
            team_game: 0,
            bit30: 0,
            total_players: 0,
            frag_limit: 20,
            time_limit: 10,
            scenario: scenario::DEATHMATCH,
            level_code,
            human_count: 1,
            bot_count: 0,
            weapon_set: 0,
            capture_reward: 0,
            spawn_policy: 0,
            radar: 1,
            registered_pickups: 0,
            friendly_fire: 0,
            seed: 0,
        }
    }

    /// The spawn-table split for `team` — `MP_RegisterSpawnPoint` 0x184f20:51-67 and
    /// `MP_GetSpawnPoint` 0x185098:85-98: team games (and Assassin) split the 64-entry table
    /// [0,0x20)/[0x20,0x40); FFA uses the whole range. `team == 2` is rejected outright [M
    /// 0x184f20:51-53]; team > 1 in a team game is the game's undefined range (the decompile leaves
    /// the low bound unwritten) — the port treats it as team 1's range and the report notes it.
    fn team_range(&self, team: u16) -> (u16, u16) {
        if team == 2 {
            return (0, 0); // the caller rejects; here: an empty range
        }
        if self.team_game != 0 || self.scenario == scenario::ASSASSIN {
            if team == 0 { (0, 0x20) } else { (0x20, 0x40) }
        } else {
            (0, 0x40)
        }
    }
}

/// One 0x30-byte `SpawnPoint` of the 64-entry table (0xc00 bytes @0x2a7360): +0x00 pos vec4 (the
/// floor-snapped placement, y = floor + 1.6 — the eye), +0x10 facing vec4, +0x20 the used flag [M
/// 0x184f20:73-94].
#[derive(Clone, Copy, Debug, Default)]
pub struct SpawnPoint {
    /// +0x00: pos vec4 — [x, floor + 1.6, z, w].
    pub pos: [f32; 4],
    /// +0x10: facing vec4 (the placement's rotation).
    pub facing: [f32; 4],
    /// +0x20 (u16): the used flag.
    pub used: u16,
}

/// The spawn globals: `SpawnPoints` (64 × 0x30 @0x2a7360), `SpawnPntTeamCount` (2 × u32), and
/// `SpawnPntCount` [M 0x184f20:81-95; `MP_Init` 0x183438:12-14 zeroes them].
#[derive(Clone, Debug, Default)]
pub struct MpSpawnState {
    /// The 64-entry table.
    pub points: Vec<SpawnPoint>,
    /// `SpawnPntTeamCount` (8 bytes = 2 × u32): the per-team registered counts.
    pub team_count: [u32; 2],
    /// `SpawnPntCount`.
    pub count: u32,
}

impl MpSpawnState {
    fn zeroed() -> MpSpawnState {
        MpSpawnState { points: vec![SpawnPoint::default(); 64], team_count: [0; 2], count: 0 }
    }
}

/// `MP_RegisterMPObject`'s stored record — the "places" record is 0x60 = the 0x40 matrix + 0x1c
/// config (copied from the params block+0x2c — the type word first) + the 0x4 cel-record pointer
/// [M 0x185990:64-69]. Phase 1 stores; nothing drives it.
#[derive(Clone, Debug)]
pub struct MpObjectRecord {
    /// The placement matrix (`RotMatrix(rotation)` + `Mat_SetTrans(translation)`).
    pub matrix: [[f32; 4]; 4],
    /// The 0x1c config copied from the placement's params block+0x2c (the type word first).
    pub config: [u32; 7],
    /// The placement's cel-record pointer (the game stores the raw `level_tag*`).
    pub cel: u32,
}

/// The per-type stores `MP_RegisterMPObject` fills (the phase-1 stand-ins for the game's globals):
/// the caps are the game's (8 per array [M 0x185990:55-63,63-71,88-96,102-110,117-125,131-143]).
#[derive(Clone, Debug, Default)]
pub struct MpObjects {
    /// The immediate-create path (`MP_CreateObject` 0x185c88): CTF flag/base (types 0/1, scenario
    /// 0x20000004), blueprint base (4, 0x20000100), KOTH hill (9, 0x40000800/0x60001000), uplink
    /// terminal (2, 0x60000008) — *Deviation*: stored here, no object is built (phase 1).
    pub created: Vec<MpObjectRecord>,
    /// Type 3 demolition sites (scenario 0x20000040).
    pub demolition: Vec<MpObjectRecord>,
    /// Type 5 blueprint items (scenario 0x20000100).
    pub blueprint_items: Vec<MpObjectRecord>,
    /// Type 6/7 GoldenEye item sets (scenario 0x20000200) — two separate counters in the game
    /// (`GoldenEyeSpawnCounts` / `DAT_0030d76a`, 8 each).
    pub goldeneye: [Vec<MpObjectRecord>; 2],
    /// Type 8 protection sites (scenario 0x20000080; the game swaps the config's +0x34 ↔ +0x40
    /// words first [M 0x185990:133-135]).
    pub protection: Vec<MpObjectRecord>,
}

/// One `MPGame` slot — the port's stand-in for the obj pointer at `MPGame + slot·0x30 + 0x1c`: the
/// game reads the live `obj+0x30` position per comparison [M 0x185098:123-129]; the port carries the
/// live position, updated by the sim each tick.
#[derive(Clone, Copy, Debug, Default)]
pub struct MpSlot {
    /// The slot's live position (the port's `obj+0x30`); `None` = the null pointer.
    pub pos: Option<[f32; 3]>,
    /// +0x22 / +0x24 (u16): the two per-slot timers `MP_Update` decrements [M 0x184bd0:25-31].
    pub timers: [u16; 2],
    /// +0x28 (u16): the last-killer slot id (0xffff = none, `MP_Init` 0x183438:58).
    pub last_killer: u16,
    /// +0x20 (u16): the **last-shooter** word — init 0xfffe [M 0x183438:59], written per allowed
    /// bullet hit [M 0x1857c0:34], reset 0xffff on a round restart [M 0x183fc0:81] and 0xfffe on a
    /// respawn [M 0x185368:56]. (Not a status bitfield — that is +0x26 below.)
    pub status: u16,
    /// +0x18 (f32): the score (the death-limit copy when scenario 0x10 [M 0x183438:60-62]).
    pub score: f32,
    /// +0x04 (i32): the kill count [M 0x188f70:157-163].
    pub kills: i32,
    /// +0x08 (i32): the death count — incremented for every death in every scenario [M
    /// 0x188f70:92-94]; the 0x10 lives gate compares it against the frag limit.
    pub deaths: i32,
    /// +0x10 (i32): the kill streak [M 0x188f70:151-163].
    pub streak: i32,
    /// +0x26 (u16): the status bits — bit0 carrying-flag [M 0x18a518:88-96], bit0x10 on-hill [M
    /// 0x186ae0:95]. The port sets/clears the bits its rules drive.
    pub status_bits: u16,
    /// The port's alive marker — its stand-in for the player `obj+0xF4` ∈ {1,2,3} state the game's
    /// filters read [M 0x1830e0:66-75] (*Deviation*: the port's players have no BLData pipeline
    /// yet, so the death path is a flag).
    pub dead: bool,
}

/// `MPGame` @0x2a4980 (0x1d0 bytes) — the live match state [M atlas §2; the writes in
/// `MP_Init`/`MP_Start`/`MP_Update`/`MP_CheckForEndCondition`].
#[derive(Clone, Debug)]
pub struct MpGame {
    /// The 8 slots (0x30 stride; the port carries the fields the phase touches).
    pub slots: [MpSlot; 8],
    /// +0x180 / +0x184: the team scores.
    pub team_scores: [f32; 2],
    /// +0x392: the state machine (`MP_Update`'s switch; 0 = playing).
    pub state: u32,
    /// +0x400: the round clock (seconds; the game accumulates the `psiGetTimeIn100ths` delta × 0.01
    /// [M 0x184268:35-55] — the port accumulates the fixed tick, see the module doc).
    pub clock: f32,
    /// +0x404: the limit (a float — compared `<=` and rendered ×100 [M 0x184268:57-62]); `MP_Init`
    /// copies `(f32)MPSettings+0x416` [M 0x183438:39], defaulting 60.0 for demo/prot with a negative
    /// limit [M 0x183438:51-54].
    pub limit: f32,
    /// +0x408: the state-6 restart timer [M 0x184bd0:79-91].
    pub restart_timer: f32,
    /// +0x412: the elapsed match time (the same delta, not gated on the mode) [M 0x184268:44-46].
    pub elapsed: f32,
    /// +0x416: the state-3 debrief timer [M 0x184bd0:55-60].
    pub debrief_timer: f32,
    /// +0x420: the wall-clock stamp (−1.0 from `MP_Start` 0x183a50:206; `MP_CheckForEndCondition`
    /// stamps the first frame and diffs per frame [M 0x184268:21-41]) — the port keeps the −1.0
    /// sentinel and models the delta as the fixed tick (*Deviation*, the module doc).
    pub pause_stamp: f32,
    /// `switch_channels[0xfe]`: the time-up latch [M 0x184268:57,142-145].
    pub time_up: bool,
    /// `switch_channels[0xfd]`: the frag-limit latch [M 0x184268:170-172].
    pub frag_latched: bool,
    /// +0x18C (i32, Ghidra `_396_4_`): the running high score — max(team 0, team 1), then max over
    /// the 8 slot scores for FFA (the demo/prot branch takes both) [M 0x184268:147-172]. The
    /// frag-limit latch compares it against `MPSettings+0x19C`. (621's `top_score` — the merge
    /// reuses this one field; there is no second +0x18C.)
    pub high: i32,
    /// `GameState+0x24 = 1` at state 3→4 [M 0x184bd0:59; disasm 0x184e6c `sw $v1,0x24($a1)`].
    pub handover_flag: bool,
    /// The case-4 navigation request: `GameState+0x1C = 0x40000033` + `ResetMap_LevelToLoad`
    /// (0x7000048) + the fade [M 0x184bd0:62-78; disasm 0x184e70-0x184e94] — the host consumes it
    /// and opens the front end at the debriefing (`FrontEnd::open_mp_debriefing`).
    pub request_debriefing: bool,
    /// The case-6 restart request: state 0 + `MP_RestartScenario` after 5 s (demo/prot)
    /// [M 0x184bd0:79-92] — the host re-runs the scenario init. The restart itself already runs
    /// here ([`MpMatch::restart_scenario`], the game's own call site); the flag is the host's
    /// observable.
    pub restart_pending: bool,
    /// The end status `MP_SortOutWhoWon` 0x1848d0 computes for the status sprite (the game writes
    /// it straight into the string buffer; the port stores it for the HUD/debrief).
    pub end_status: MpEndStatus,
}

/// The status line `MP_SortOutWhoWon` 0x1848d0 builds [M :21-92] — ids/texts per the USATxt bank.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum MpEndStatus {
    /// Still playing (no sort has run).
    #[default]
    None,
    /// "Player %s Won" 0x2000022 / "Game Over : %s Won" (0x2000024 + 0x2000027) — the codename.
    Won(String),
    /// "A Draw" 0x2000023 / "Game Over : A Draw" (0x2000024 + 0x2000023).
    Draw,
}

impl MpGame {
    fn zeroed() -> MpGame {
        MpGame {
            slots: std::array::from_fn(|_| MpSlot {
                pos: None,
                timers: [0; 2],
                last_killer: 0xffff,
                status: 0xfffe,
                score: 0.0,
                kills: 0,
                deaths: 0,
                streak: 0,
                status_bits: 0,
                dead: false,
            }),
            team_scores: [0.0; 2],
            state: 0,
            clock: 0.0,
            limit: 0.0,
            restart_timer: 0.0,
            elapsed: 0.0,
            debrief_timer: 0.0,
            pause_stamp: -1.0,
            time_up: false,
            frag_latched: false,
            high: 0,
            handover_flag: false,
            request_debriefing: false,
            restart_pending: false,
            end_status: MpEndStatus::None,
        }
    }
}

/// The phase-1 match: the settings, the spawn globals, the live state and the stored MP objects —
/// the port's `MP_Init`/`MP_Start`/`MP_Update` driver.
pub struct MpMatch {
    /// `MPSettings` @0x2a47a0.
    pub settings: MpSettings,
    /// The spawn globals.
    pub spawns: MpSpawnState,
    /// `MPGame` @0x2a4980.
    pub game: MpGame,
    /// The registered MP objects (0x26 placements; stored, not driven).
    pub objects: MpObjects,
    /// The scenario objects' live state — the port's `Flags`/`Bases`/`Uplinks`/`Hill`/
    /// `Demolition`/`GoldenEye`/`BluePrint` globals [M 0x185de8:133-178]. Built by
    /// [`MpMatch::create_mode_objects`], driven by [`MpMatch::update_objects`].
    pub objects_rt: MpObjectsRt,
    /// The assassin slot (global `DAT_0030d774`, scenario 0x400) [M 0x18b538:6].
    pub assassin: Option<usize>,
    /// The target slot (global `Assasin`, scenario 0x400) [M 0x18b558:6].
    pub assassin_target: Option<usize>,
    /// The bots (slots 4..4+bot_count). `MP_Start` 0x183a50:134-160 creates one bot per enabled
    /// `MPBOTS` record — `BOT_init(slot, spawn pos, …, 0x2deeaa + i*0x12, 0)` — after every
    /// human's `Player_Init`, in slot order [M]. mp-bots-in-match: the entities the sim drives
    /// (the ported `BotBrain` = the game's BOT_update family); creation is the caller's
    /// [`MpMatch::create_bots`] right after the humans' spawn picks, like the game's order.
    pub bots: Vec<MpBot>,
}

/// One MP bot: the ported brain (`BOT_vars` stand-in) plus the per-slot bookkeeping the match
/// reads. The game's bot is a drone-classed obj with a player slot (0x2a499c + slot·0x30); the
/// port carries the slot state in `MpGame::slots[slot]` and the AI in `brain`.
pub struct MpBot {
    /// The player slot (4..8) [M 0x183a50:138 `iVar4 = uVar9 + 4`].
    pub slot: usize,
    /// The bot's team word (`MPSettings + slot·0x30 + 0x20` = the record's +0x0F byte) [M 0x183a50:143-145].
    pub team: u16,
    /// The brain: `BOT_init` 0x1249e8's port (`bots::BotBrain::bot_init`) — the 0xC3 chain into
    /// state 0xF9, the stats copy from the record, the goal slots [M].
    pub brain: crate::mp::bots::BotBrain,
    /// The bot's health (`Drone+0xAC`; `BOT_SetHealth` 0x126758 clamps to the stats' max HP).
    pub health: f32,
    pub max_health: f32,
    /// The bot's weapon id (the record's stats `+0x0A` weapon class maps to the match's weapon
    /// — the Quick Start presets; the brain's firing interval/burst lift `weapon_data`).
    pub weapon: u8,
    /// Bookkeeping for the match table: shots fired, deaths, respawns.
    pub shots: u32,
    pub deaths: u32,
    pub respawns: u32,
    /// Dead flag + the tick the respawn may run (`BOT_respawn` 0x124e38 re-inits the obj at a
    /// spawn point; the death handler's delay — the port stands in 180 ticks / 3 s [S]).
    pub dead: bool,
    pub respawn_at: u32,
}

fn sq_dist3d(a: [f32; 4], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

impl MpMatch {
    /// `MP_Init` 0x183438: zeroes every MP global, derives `+0x396`/`+0x400` from the scenario
    /// word, copies the time limit into `MPGame+0x404` (f32), inits the per-slot id words and the
    /// 0x10-mode death-limit copy [M 0x183438:10-65]. The game skips the whole body for level
    /// 0x07000048 (the front-end map) — the port's caller never inits there.
    pub fn init(mut settings: MpSettings) -> MpMatch {
        settings.started = false;
        settings.master_copy = 0;
        settings.registered_pickups = 0;
        let scenario = settings.scenario;
        settings.team_game = (scenario >> 29) & 1;
        settings.bit30 = (scenario >> 30) & 1;
        let mut game = MpGame::zeroed();
        game.limit = settings.time_limit as f32;
        if (scenario == scenario::DEMOLITION || scenario == scenario::PROTECTION) && game.limit < 0.0 {
            game.limit = 60.0; // 0x42700000 [M 0x183438:53]
        }
        MpMatch {
            settings,
            spawns: MpSpawnState::zeroed(),
            game,
            objects: MpObjects::default(),
            objects_rt: MpObjectsRt::default(),
            assassin: None,
            assassin_target: None,
            bots: Vec::new(),
        }
    }

    /// The bots' half of `MP_Start` 0x183a50:134-160: the count from the prepared records
    /// (clamped ≤ 4 [M 0x183a50:138-140]), then per bot — the slot `4 + i`, the team byte from
    /// the record's +0x0F into `MPSettings + slot·0x30 + 0x20` [M 0x183a50:143-145], a spawn
    /// point pick for that team (`MP_GetSpawnPoint(team, 0)` [M 0x183a50:147-148]) and
    /// `BOT_init(slot, pos, …, record, 0)` [M 0x183a50:156-160]. The port: the brains init from
    /// the enabled records in order (the Quick Start's Drake/Kiko/Rook — the first `count`
    /// records, the menu's compacted order [M Menu_PrepareBots 0x1f94a0]), the slot team words
    /// ride the settings, and the caller re-picks the spawn through the port's
    /// [`MpMatch::get_spawn_point`] (the same policy the humans use). Draw order: the game picks
    /// the humans FIRST, then the bots — the caller keeps that order (create_bots runs after the
    /// humans' picks in the load).
    pub fn create_bots(&mut self, rng: &mut Rng) {
        use crate::mp::bots::{BotRecord, INITIAL_BOT_RECORDS};
        let count = (self.settings.bot_count as usize).min(4);
        if count == 0 {
            return;
        }
        // Menu_PrepareBots 0x1f94a0 compacts the enabled records (record+0x0E != 0) to the front
        // [M] — the port's menu writes the count only, so the enabled set is the first `count`
        // records here ([S]: the Quick Start's defaults are the disc's INITIAL records).
        let records: Vec<BotRecord> = INITIAL_BOT_RECORDS.iter().take(count).cloned().collect();
        for (i, rec) in records.iter().enumerate() {
            let slot = 4 + i;
            let team = rec.team() as u16; // +0x0F, the team byte [M 0x183a50:143-145]
            self.settings.players[slot].team = team as u32;
            let sp = self.get_spawn_point(team, slot, rng) as usize;
            let p4 = self.spawns.points[sp].pos;
            let pos = [p4[0], p4[1], p4[2]];
            let yaw = self.spawns.points[sp].facing[1];
            let max_hp = rec.max_hp() as f32;
            let brain = crate::mp::bots::BotBrain::bot_init(rec, slot as u8, pos, yaw);
            let weapon = rec.preferred_weapon_class();
            self.game.slots[slot].pos = Some(pos);
            self.bots.push(MpBot {
                slot,
                team,
                brain,
                health: max_hp,
                max_health: max_hp,
                weapon,
                shots: 0,
                deaths: 0,
                respawns: 0,
                dead: false,
                respawn_at: 0,
            });
        }
    }

    /// The bot slots' shared view for the brain's `BotCtx` others table — the 8 player slots'
    /// positions/teams/scores with the bots' own words filled from [`MpBot`] (the game reads
    /// `MPGame + slot·0x30` and the objs [M 0x125390]) and the humans' from the slots the caller
    /// keeps (the port's humans ride `slots[].pos`).
    pub fn bot_slot_team(&self, slot: usize) -> u32 {
        self.settings.players[slot].team
    }

    /// `MP_RegisterSpawnPoint` 0x184f20 (the placement 0x25 hook): the team gate, the FFA/team
    /// range, the first unused slot, the floor snap (`build_PointOnFloor` 0x1c95a8 with reach 3.0 —
    /// a straight-down ray from the placement pos; on a hit the pos = the hit point) and the
    /// +1.6 eye [M 0x184f20:79-96].
    pub fn register_spawn_point(&mut self, pos: [f32; 3], facing: [f32; 3], team: u16, world: &CollisionWorld) {
        if self.settings.master == 0 {
            return; // the dispatch's own gate [M 0x1d04f0 case 0x25]
        }
        let team = team & 0xffff;
        if team == 2 {
            return; // [M 0x184f20:51-53]
        }
        let (lo, hi) = self.settings.team_range(team);
        for i in lo..hi {
            if self.spawns.points[i as usize].used == 0 {
                // build_PointOnFloor(3.0, cel, 0, &pos, 0): the ray [x,y,z] → [x,y−3,z]; on a hit
                // the pos becomes the hit point [M 0x1c95a8:39-58]. The port's `segment` returns
                // the hit fraction.
                let mut p = pos;
                if let Some(h) = world.segment([p[0], p[1], p[2]], [p[0], p[1] - 3.0, p[2]]) {
                    p[1] -= 3.0 * h;
                }
                self.spawns.points[i as usize] = SpawnPoint {
                    pos: [p[0], p[1] + 1.6, p[2], 0.0],
                    facing: [facing[0], facing[1], facing[2], 0.0],
                    used: 1,
                };
                self.spawns.team_count[team as usize] += 1;
                self.spawns.count += 1;
                return;
            }
        }
    }

    /// `MP_RegisterMPObject` 0x185990 (the placement 0x26 hook): the dispatch on the placement's
    /// param 0 (the MP object type 0..9), each type gated on its scenario and capped at 8; the
    /// protection site swaps the config's +0x34 ↔ +0x40 words first [M 0x185990:133-135]. The
    /// config = the 0x1c bytes at the params block+0x2c (the type word first) — the port carries it
    /// as the first 7 param-block words (type at word 0).
    pub fn register_mp_object(&mut self, pos: [f32; 3], facing: [f32; 3], mp_type: u32, config: [u32; 7], cel: u32) {
        if self.settings.master == 0 {
            return; // the dispatch's own gate [M 0x1d04f0 case 0x26]
        }
        // RotMatrix(facing) + Mat_SetTrans(pos): the placement matrix. The port stores the
        // translation and facing words (the full matrix build is the renderer's; phase 1 stores).
        let matrix = [
            [facing[0], facing[1], facing[2], 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [pos[0], pos[1], pos[2], 1.0],
        ];
        let rec = MpObjectRecord { matrix, config, cel };
        let s = self.settings.scenario;
        match mp_type {
            0 | 1 if s == scenario::CAPTURE_THE_FLAG => self.objects.created.push(rec),
            2 if s == scenario::UPLINK && self.objects.created.len() < 8 => self.objects.created.push(rec),
            3 if s == scenario::DEMOLITION && self.objects.demolition.len() < 8 => self.objects.demolition.push(rec),
            4 if s == scenario::BLUEPRINT => self.objects.created.push(rec),
            5 if s == scenario::BLUEPRINT && self.objects.blueprint_items.len() < 8 => self.objects.blueprint_items.push(rec),
            6 if s == scenario::GOLDENEYE && self.objects.goldeneye[0].len() < 8 => self.objects.goldeneye[0].push(rec),
            7 if s == scenario::GOLDENEYE && self.objects.goldeneye[1].len() < 8 => self.objects.goldeneye[1].push(rec),
            8 if s == scenario::PROTECTION && self.objects.protection.len() < 8 => self.objects.protection.push(rec),
            9 if s == scenario::KOTH_FFA || s == scenario::KOTH_TEAM => self.objects.created.push(rec),
            _ => {} // the scenario gates [M 0x185990:52,60,74-78,85-86,99-102,114-118,128-132,146-148]
        }
    }

    /// `MP_GetSpawnPoint` 0x185098 (the spawn policy): scan the team range for used points whose
    /// nearest-enemy squared distance exceeds 2.0 (the candidate gate [M 0x185098:106,137]);
    /// `uStack_d8` tracks the argmax (the FARTHEST from enemies) and `uStack_d4` the argmin among
    /// the candidates [M 0x185098:136-154 — the keep-old/take-new updates are a running min and
    /// max] — the atlas's "2nd-farthest" gloss for policy 0 does not match this arithmetic. Policy
    /// (`MPSettings+0x448`): 1 → the farthest, 0 → the least-far candidate, 2 → a random candidate
    /// (`cands[Rand_Rand(n)]` [M 0x185098:177-180]), anything else → the range start [M
    /// 0x185098:181-183]; each falls back to the range start when its pick is unset
    /// (−1) [M 0x185098:165-176]. `me` = the calling slot index (the game excludes its own obj by
    /// pointer [M 0x185098:124]). Policy 2 with no candidates reads the game's uninitialized
    /// buffer — the port returns the range start (*Deviation*).
    pub fn get_spawn_point(&self, team: u16, me: usize, rng: &mut Rng) -> u32 {
        let (lo, hi) = self.settings.team_range(team);
        if lo == hi {
            return 0; // [M 0x185098:100-102]
        }
        let mut best1 = 0xffff_ffffu32; // uStack_d8: the argmax
        let mut best2 = 0xffff_ffffu32; // uStack_d4: the argmin
        let mut max_so_far = -9999.0f32; // fVar13
        let mut min_so_far = 9999.0f32; // fVar10
        let mut cands = [0u16; 64];
        let mut n_cands = 0usize;
        for i in lo..hi {
            let p = &self.spawns.points[i as usize];
            if p.used == 0 {
                continue;
            }
            // The nearest other player's squared distance [M 0x185098:116-135].
            let mut nearest = 9999.0f32;
            for (slot, ent) in self.game.slots.iter().enumerate() {
                if let Some(pos) = ent.pos {
                    if slot != me && nearest > 0.0 {
                        let d = sq_dist3d(p.pos, pos);
                        if d < nearest {
                            nearest = d;
                        }
                    }
                }
            }
            if nearest > 2.0 {
                // The running-min tracker (uStack_d4/fVar10) [M 0x185098:141-146].
                if !(min_so_far <= nearest) {
                    best2 = i as u32;
                    min_so_far = nearest;
                }
                // The running-max tracker (uStack_d8/fVar13) [M 0x185098:147-154].
                if !(nearest <= max_so_far) {
                    best1 = i as u32;
                    max_so_far = nearest;
                }
                cands[n_cands] = i as u16;
                n_cands += 1;
            }
        }
        match self.settings.spawn_policy {
            1 if best1 != 0xffff_ffff => best1 & 0xffff,
            0 if best2 != 0xffff_ffff => best2 & 0xffff,
            2 if n_cands > 0 => cands[rng.rand(n_cands as u32) as usize] as u32, // Rand_Rand [M 0x1e3738]
            _ => lo as u32, // the fallbacks [M 0x185098:165-176,181-183]
        }
    }

    /// `MP_Start` 0x183a50 (the port's part — no bots, no sprites in phase 1): the master gate, the
    /// mode-object creation draws (Deathmatch has none — the 0x20000040/0x20000080/0x20000100/
    /// 0x20000200 arms are the only draw paths [M 0x183a50:77-113]), `MPGame+0x420 = −1.0` [M
    /// 0x183a50:206] and the started flag [M 0x183a50:69,207]. The player creation is the caller's
    /// (`Player_Init`'s port is `Bond::stand_at` + `set_slot`).
    pub fn start(&mut self, rng: &mut Rng) {
        self.settings.started = false;
        self.settings.master_copy = self.settings.master;
        if self.settings.master == 0 {
            return; // [M 0x183a50:72-74]
        }
        // The mode-object creation draws: only demo/prot/blueprint/GE consume Rand_Rand here; a
        // Deathmatch start draws nothing [M 0x183a50:77-113] — phase 1 re-registers nothing either
        // (the records were stored at registration; the game re-picks a "place" per start).
        let _ = rng;
        self.game.pause_stamp = -1.0; // 0xbf800000 [M 0x183a50:206]
        self.settings.started = true; // [M 0x183a50:207]
    }

    /// Registers a player's slot (the port's `Player_Init` link: `MPGame + slot·0x30 + 0x1c = obj`
    /// [M 0x183a50:127] — the port stores the live position the spawn policy and later phases read).
    pub fn set_slot(&mut self, slot: usize, pos: [f32; 3]) {
        self.game.slots[slot].pos = Some(pos);
    }

    /// `MP_Update` 0x184bd0 (per tick, the port's fixed 60 Hz): the master + state gates are the
    /// caller's (the sim only ticks in gameplay and never pauses); the per-slot timer decrement
    /// [M 0x184bd0:25-35], then the state machine — state 0 (playing) runs
    /// `MP_CheckForEndCondition` [M 0x184bd0:36-41] plus the scenario-object updates (the game's
    /// `MP_ObjectUpdate` runs the same frame). `MP_Pickup_Process`/`BOT_update` are phase 2+ (no
    /// pickups are driven, no bots exist).
    ///
    /// The end states: 1 → `MP_SortOutWhoWon` [M :51-52]; 2 → the 0x10 fall-through into state 1's
    /// body, every other scenario straight to 3 (the time-up text is skipped — numbers only) [M
    /// :42-49]; 3 → 4 (setting `GameState+0x24 = 1`, the port's `handover_flag`) at once for
    /// non-0x10, 0x10 waits the 5 s debrief timer — the gate compare is `5.0 <=` [M :54-61, decomp
    /// line 56]; 4 → 5 with the debriefing request (`GameState+0x1C = 0x40000033` + the
    /// `ResetMap_LevelToLoad(0x7000048)`/fade/`GS_PausePlayer` calls — the port exposes the
    /// request, the host drives the rest, *Deviation*) [M :62-78]; 6 → the restart timer (the
    /// 2.5 s status text is the renderer's) then at `5.0 <` timer — STRICTLY greater [M :85, decomp
    /// line 85] — state 0 + `MP_RestartScenario` + `restart_pending`. `rng` is the shared bit-exact
    /// RNG — it is drawn only by the state-6 restart's site re-pick. The scenario objects are the
    /// caller's [`MpMatch::update_objects`] call (the game's `MP_ObjectUpdate` is driven by the
    /// object system the same way, and the KOTH hill volume is level data).
    pub fn update(&mut self, dt: f32, paused: bool, rng: &mut Rng) {
        if self.settings.master == 0 || !self.settings.started {
            return; // [M 0x184bd0:13-16]
        }
        for slot in &mut self.game.slots {
            for t in &mut slot.timers {
                if *t != 0 {
                    *t -= 1;
                }
            }
        }
        if paused {
            return; // the game's GS_IsPaused branch only stamps the wall clock [M 0x184bd0:95-98]
        }
        if self.game.state == 0 {
            self.check_for_end_condition(dt);
            return;
        }
        match self.game.state {
            1 => {
                self.sort_out_who_won(); // sets state 3 [M 0x1848d0:20]
            }
            2 => {
                if self.settings.scenario == scenario::DEATH_LIMIT_FFA {
                    self.sort_out_who_won(); // the C switch falls through for 0x10 [M 0x184bd0:42-52]
                } else {
                    self.game.state = 3; // the time-up text is the renderer's
                }
            }
            3 => {
                if self.settings.scenario == scenario::DEATH_LIMIT_FFA {
                    self.game.debrief_timer += dt; // REC_FRAME_RATE per frame [M 0x184bd0:54-61]
                    if self.game.debrief_timer >= 5.0 {
                        self.game.state = 4;
                        self.game.handover_flag = true; // GameState+0x24 = 1 [M 0x184bd0:59]
                    }
                } else {
                    self.game.state = 4;
                    self.game.handover_flag = true; // GameState+0x24 = 1 [M 0x184bd0:59]
                }
            }
            4 => {
                // GameState._28_4_ = 0x40000033, ResetMap_LevelToLoad(0x7000048) + the flow push +
                // the per-slot Player_SetCamMode/GS_PausePlayer [M 0x184bd0:62-78] — the port has
                // no loader and no player pipeline; it exposes the request (state 5 marks the wait).
                self.game.request_debriefing = true;
                self.game.state = 5;
            }
            5 => {} // waiting for the front-end load [S]
            6 => {
                self.game.restart_timer += dt; // [M 0x184bd0:79-92]
                if self.game.restart_timer > 5.0 {
                    // 5.0 < timer, STRICTLY [M 0x184bd0:85] — the state-3 debrief gate is the
                    // inclusive 5.0 <=, this one is not.
                    self.game.state = 0;
                    self.game.restart_pending = true; // the host's observable [M :86-90]
                    self.restart_scenario(rng); // the game's own call site [M :87]
                }
            }
            _ => {}
        }
    }

    /// `MP_CheckForEndCondition` 0x184268 — the full rule set (reconciled against the decomp in
    /// mp-debriefing-data2; the reconciliation table is in the report):
    ///
    /// - the clocks: the elapsed delta (+0x19C) adds always, the round clock (+0x190) when not
    ///   paused [M 0x184268:44-46,55,140] (the fixed tick stands in for the wall clock);
    /// - demo/prot (decomp lines 50-80): inside the gate `0.0 < limit && switch_channels[0xfe]
    ///   clear`, `limit <= clock` enters **state 6 and does NOT latch 0xfe** [M :57-60] — the
    ///   general 0xfe latch at :142 belongs to the OTHER branch and never runs for demo/prot;
    /// - scenario 0x10 (decomp lines 85-135): the out-count — every obj-live slot whose deaths
    ///   reached the lives limit counts, split by the `obj+0xff` state byte (0x12 humans vs
    ///   0x11/0x02 bots; a 0x02 slot showing the fully-alive signature is exempt) into two
    ///   counters that sum. The port counts `dead`-marked slots (its state-byte stand-in) — the
    ///   exemption is unobservable (any slot with deaths ≥ 1 is `dead`), the one-per-call
    ///   0x02→0x11 flip only lifts that same exemption, so neither is modeled (*Deviation*, the
    ///   systems doc §1). `(total_players − 1) <= outs` or `humans_out == human_count` enters
    ///   state 1 [M :120-123];
    /// - the time-up latch (decomp lines 142-143, the non-demo/prot branch): `time_limit != −1
    ///   && 0xfe clear` → `0xfe = limit <= clock` — **no `limit > 0` gate here** (that gate is
    ///   the demo/prot state-6 arm's alone); state is NOT set;
    /// - the high score (+0x18C) = max(team 0, team 1), then max over the 8 slot scores — always
    ///   for demo/prot, otherwise for a non-team game [M :66-79,149-167];
    /// - the frag latch (decomp lines 170-172) is **shared by demo/prot and the general branch**
    ///   (only 0x10 jumps past it — its +0x19C is the lives limit): `frag_limit != −1` → `0xfd =
    ///   frag_limit <= high` [M :168-172];
    /// - final dispatch (LAB_00184884, decomp lines 173-184): `0xfd` set → state 1, else `0xfe`
    ///   set → state 2, else the state stays as the scenario arms left it (a demo/prot state 6
    ///   or a 0x10 elimination state 1 both survive when the latches are clear).
    fn check_for_end_condition(&mut self, dt: f32) {
        self.game.elapsed += dt; // [M 0x184268:44-46]
        let s = self.settings.scenario;
        if s == scenario::DEMOLITION || s == scenario::PROTECTION {
            self.game.clock += dt; // [M :55]
            if self.game.limit > 0.0 && !self.game.time_up && self.game.limit <= self.game.clock {
                self.game.state = 6; // [M :58-60] — 0xfe is NOT written here (decomp lines 57-59)
            }
            self.update_high(true); // [M :66-79]
        } else {
            if s == scenario::DEATH_LIMIT_FFA {
                self.top_agent_elimination(); // [M :85-135]
            }
            self.game.clock += dt; // [M :140]
            if self.settings.time_limit != -1 && !self.game.time_up {
                // the 0xfe latch — no limit>0 gate (that gate is the demo/prot arm's) [M :142-143]
                self.game.time_up = self.game.limit <= self.game.clock;
            }
            self.update_high(false); // [M :149-167]
        }
        // the shared frag latch + final dispatch [M :168-184]: reached by demo/prot AND the
        // general branch; 0x10 jumps past the latch (LAB_00184884 at decomp line 168).
        if s != scenario::DEATH_LIMIT_FFA && self.settings.frag_limit != -1 {
            self.game.frag_latched = self.settings.frag_limit <= self.game.high; // [M :170-172]
        }
        if self.game.frag_latched {
            self.game.state = 1; // [M :173-184]
        } else if self.game.time_up {
            self.game.state = 2;
        }
    }

    /// The top-score scratch `MPGame+0x18C` (the port's `high`) [M 0x184268:66-79 (demo/prot),
    /// :149-167 (others)]: the two team scores as ints, then — always for demo/prot, otherwise
    /// only for a non-team game — every slot's score as int. (The game's float→int casts; scores
    /// are small. 621 named it `top_score`; the merge keeps 402's single `high`.)
    fn update_high(&mut self, demo_prot: bool) {
        let t0 = self.game.team_scores[0] as i32;
        let t1 = self.game.team_scores[1] as i32;
        let mut top = t0.max(t1);
        if demo_prot || self.settings.team_game == 0 {
            for slot in &self.game.slots {
                top = top.max(slot.score as i32);
            }
        }
        self.game.high = top;
    }

    /// The Top Agent elimination [M 0x184268:85-135]: every obj-live slot whose deaths reached
    /// the lives limit counts as out (the game splits the count by the `obj+0xff` state byte —
    /// 0x12 humans / 0x11+non-alive 0x02 bots — into two counters that sum; the port's `dead`
    /// flag, set by the death path, stands in for the byte). `(total − 1) <= outs` or all humans
    /// out enters state 1 [M :120-123]. The game's follow-up flips one state-0x02 bot to 0x11
    /// per call when the humans are all out [M :124-135] — it only lifts the alive-exemption
    /// this port cannot observe, so it is not modeled (*Deviation*, the systems doc §1).
    fn top_agent_elimination(&mut self) {
        let limit = self.settings.frag_limit;
        let mut outs = 0u32;
        let mut humans_out = 0u32;
        for (i, slot) in self.game.slots.iter().enumerate() {
            if slot.pos.is_none() || !slot.dead {
                continue; // the obj != 0 gate [M :91]; `dead` = the state-byte stand-in
            }
            if limit <= slot.deaths {
                outs += 1;
                if i < 4 {
                    humans_out += 1; // the 0x12 counter — humans 0..3, bots 4..7 [M atlas §2]
                }
            }
        }
        if (self.settings.total_players as u32).wrapping_sub(1) <= outs
            || humans_out == self.settings.human_count
        {
            self.game.state = 1; // [M :120-123]
        }
    }
}
