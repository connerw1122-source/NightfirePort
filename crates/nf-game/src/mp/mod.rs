//! The MP match core — multiplayer phase 1 (`docs/atlas/multiplayer.md` §6 phase 1): `MP_Init`
//! 0x183438, `MP_Start` 0x183a50, `MP_Update` 0x184bd0, `MPSettings` @0x2a47a0, `MPGame` @0x2a4980,
//! the spawn-point table (`MP_RegisterSpawnPoint` 0x184f20, 64 × 0x30 bytes @0x2a7360) and the spawn
//! policy (`MP_GetSpawnPoint` 0x185098, the mode at `MPSettings+0x448`), plus the MP-object
//! registration (`MP_RegisterMPObject` 0x185990 — stored, not yet driven).
//!
//! Everything here follows the game's arithmetic; deviations are marked *Deviation* and are model
//! gaps, not silent rewrites: the round clock runs on the fixed 60 Hz tick (the game samples
//! `psiGetTimeIn100ths`, a wall clock — the determinism rule forbids that in the port's Sim; the
//! game's own state-3/6 timers in `MP_Update` use `REC_FRAME_RATE` the same way), and
//! `MP_CreateObject` (the immediate-create path) stores the record without building an object
//! (phase 1 drives no MP objects).
//!
//! Match end + debriefing (mp-debriefing-data): `player_killed` (the kill/score/team rules of
//! `MP_PlayerKilled` 0x188f70), the completed `MP_CheckForEndCondition` 0x184268 / `MP_Update`
//! 0x184bd0 state machine (frag/time latches → states 1..6 → the 0x40000033 handover) and
//! `debrief::MpMatch::debriefing` (the P_MPDEBRIEFING 0x212198 rows) — see
//! `docs/systems/mp-debriefing.md`.
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

/// The debriefing data builder (mp-debriefing-data): `MpMatch::get_mp_score` /
/// `MpMatch::debriefing` — the P_MPDEBRIEFING 0x212198 rows, ranks and result line.
/// See `docs/systems/mp-debriefing.md` §4.
pub mod debrief;

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
    /// +0x00: the codename string (`C_RBMPCNAME` 0x20d448 stores it; the debriefing's name
    /// column and the winner line read it — `MPSettings+s·0x30+0x00` [M 0x212198:169-171,
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
}

impl MpSettings {
    /// The phase-1 default for `--mp <arena>`: an Arena-1-style deathmatch. The menu's own defaults
    /// are not read here (no menu path in phase 1) — the values are the port's [S], the structure
    /// and the derivations are the game's.
    pub fn deathmatch(level_code: u32) -> MpSettings {
        MpSettings {
            players: std::array::from_fn(|_| MpPlayer { team: 0, skin: 0, armor_bonus: 0, codename: String::new() }),
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
    /// +0x20 (u16): the status init 0xfffe [M 0x183438:59].
    pub status: u16,
    /// +0x18 (f32): the score (the death-limit copy when scenario 0x10 [M 0x183438:60-62]).
    pub score: f32,
    /// +0x04 (i32): the kills — killer +1 unless a team-game friendly [M 0x188f70:157-163],
    /// suicide −1 [M :150-155]. The debriefing's Victories column [M 0x212198:186-192].
    pub kills: i32,
    /// +0x08 (i32): the deaths — victim +1 [M 0x188f70:92-94]. The Deaths column [M 0x212198:193-198].
    pub deaths: i32,
    /// +0x10 (i32): the kill streak (reset on death [M 0x188f70:152/161-162, MP_ReSpawn 0x185368:55]).
    pub streak: i32,
    /// The caller's out-of-play flag for the Top Agent elimination (`MP_CheckForEndCondition`
    /// 0x184268:86-119 counts slots whose obj state is 0x11/0x12 once deaths reach the lives
    /// limit — the port takes the state as this input; *Deviation*, see mp-debriefing.md §1).
    pub out: bool,
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
    /// +0x18C: the top-score scratch the frag latch compares against [M 0x184268:66-79,149-172].
    pub top_score: i32,
    /// `GameState+0x24 = 1` at state 3→4 [M 0x184bd0:59; disasm 0x184e6c].
    pub handover_flag: bool,
    /// The case-4 navigation request: `GameState+0x1C = 0x40000033` + `ResetMap_LevelToLoad`
    /// (0x7000048) + the fade [M 0x184bd0:62-78; disasm 0x184e70-0x184e94] — the host consumes
    /// it and opens the front end at the debriefing.
    pub request_debriefing: bool,
    /// The case-6 restart request: state 0 + `MP_RestartScenario` after 5 s (demo/prot)
    /// [M 0x184bd0:79-92] — the host re-runs the scenario init.
    pub restart_pending: bool,
    /// The end status `MP_SortOutWhoWon` 0x1848d0 computes for the status sprite (the game
    /// writes it straight into the string buffer; the port stores it for the HUD/debrief).
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
    /// The 0x10 fewest-deaths winner is pending a tie check (the sort ran, nobody qualified).
    /// *Not produced by the game* — placeholder so `None` can mean "not sorted yet".
    Pending,
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
                out: false,
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
            top_score: 0,
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
        MpMatch { settings, spawns: MpSpawnState::zeroed(), game, objects: MpObjects::default() }
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
    /// `MP_CheckForEndCondition` [M 0x184bd0:36-41], state 1 the win sort [M :51-53], state 2
    /// the time-up arm (0x10 falls through to the sort [M :42-53]), state 3 the 0x10 five-second
    /// gate + `GameState+0x24 = 1` [M :54-61], state 4 the debriefing navigation request
    /// [M :62-78], state 6 the demo/prot restart timer [M :79-92].
    /// `MP_Pickup_Process`/`BOT_update` are phase 2+ (no pickups are driven, no bots exist).
    pub fn update(&mut self, dt: f32, paused: bool) {
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
        match self.game.state {
            0 => self.check_for_end_condition(dt), // [M 0x184bd0:37-41]
            1 => self.sort_out_who_won(),          // [M 0x184bd0:51-53]
            2 => {
                // scenario 0x10 falls through to the sort; anything else goes straight on
                // [M 0x184bd0:42-53]
                if self.settings.scenario != scenario::DEATH_LIMIT_FFA {
                    self.game.state = 3;
                } else {
                    self.sort_out_who_won();
                }
            }
            3 => {
                // non-0x10 passes immediately; 0x10 waits for the debrief timer to reach 5.0 s
                // [M 0x184bd0:54-61]
                let done = self.settings.scenario != scenario::DEATH_LIMIT_FFA
                    || {
                        self.game.debrief_timer += dt;
                        self.game.debrief_timer >= 5.0
                    };
                if done {
                    self.game.state = 4;
                    self.game.handover_flag = true; // GameState+0x24 = 1 [M 0x184bd0:59]
                }
            }
            4 => {
                // GameState+0x1C = 0x40000033, state 5; the ResetMap(0x7000048) + fade + player
                // pauses are the host's [M 0x184bd0:62-78; disasm 0x184e70-0x184e94]
                self.game.request_debriefing = true;
                self.game.state = 5;
            }
            6 => {
                self.game.restart_timer += dt; // [M 0x184bd0:79-91]
                if self.game.restart_timer > 5.0 {
                    self.game.state = 0;
                    self.game.restart_pending = true; // MP_RestartScenario is the host's
                }
            }
            _ => {} // state 5 idles until the level change tears the match down
        }
    }

    /// `MP_CheckForEndCondition` 0x184268 — the full port (mp-debriefing.md §1). The clock runs on
    /// the fixed tick (the game samples `psiGetTimeIn100ths` — *Deviation*, the module doc). The
    /// elapsed word always accumulates unpaused [M 0x184268:44-46]; the round clock and the
    /// latches follow the scenario dispatch [M :50-172] and the final dispatch sets state 1/2
    /// [M :173-184].
    fn check_for_end_condition(&mut self, dt: f32) {
        self.game.elapsed += dt; // MPGame+0x19C [M 0x184268:44-46]
        let s = self.settings.scenario;
        if s == scenario::DEMOLITION || s == scenario::PROTECTION {
            // the shared demo/prot tail: the state jumps straight to 6 WITHOUT latching 0xfe
            // [M 0x184268:50-79]
            self.game.clock += dt; // [M :55]
            if self.game.limit > 0.0 && !self.game.time_up && self.game.limit <= self.game.clock {
                self.game.state = 6; // [M :59]
            }
            self.update_top_score(true); // [M :66-79]
        } else {
            if s == scenario::DEATH_LIMIT_FFA {
                self.top_agent_elimination(); // [M :85-135]
            }
            self.game.clock += dt; // [M :140]
            if self.settings.time_limit != -1 && !self.game.time_up {
                // the 0xfe latch — state is NOT set here [M :142-143]
                self.game.time_up = self.game.limit > 0.0 && self.game.limit <= self.game.clock;
            }
            self.update_top_score(false); // [M :149-167]
            if s != scenario::DEATH_LIMIT_FFA && self.settings.frag_limit != -1 {
                // the frag latch — 0x10 skips it (its +0x19C is the lives limit) [M :168-172]
                self.game.frag_latched = self.settings.frag_limit <= self.game.top_score;
            }
        }
        // the final dispatch [M :173-184]
        if self.game.frag_latched {
            self.game.state = 1;
        } else if self.game.time_up {
            self.game.state = 2;
        }
    }

    /// The top-score scratch `MPGame+0x18C` [M 0x184268:66-79 (demo/prot), :149-167 (others)]:
    /// the two team scores as ints, then — always for demo/prot, otherwise only for a non-team
    /// game — every slot's score as int. (The game's float→int casts; scores are small.)
    fn update_top_score(&mut self, demo_prot: bool) {
        let t0 = self.game.team_scores[0] as i32;
        let t1 = self.game.team_scores[1] as i32;
        let mut top = if t0 < t1 { t1 } else { t0 };
        if demo_prot || self.settings.team_game == 0 {
            for slot in &self.game.slots {
                let v = slot.score as i32;
                if top < v {
                    top = v;
                }
            }
        }
        self.game.top_score = top;
    }

    /// The Top Agent elimination [M 0x184268:85-135]: slots whose deaths reached the lives limit
    /// (`MPSettings+0x19C`) and whose caller-marked `out` state says they are out of play end the
    /// match (state 1) when only one player remains or every human is out; while bots remain the
    /// game force-flips one bot per call [M :124-135]. The game splits the outs by the player
    /// state byte (0x12 vs 0x11/0x02) — the port carries one flag (*Deviation*, mp-debriefing.md
    /// §1; both counters sum in the end test).
    fn top_agent_elimination(&mut self) {
        let lives = self.settings.frag_limit;
        let humans = self.settings.human_count as usize;
        let bots = self.settings.bot_count as usize;
        let total = (self.settings.total_players as usize).max(1);
        let outs = self.game.slots.iter().filter(|s| s.out && s.deaths >= lives).count();
        if total - 1 <= outs || outs == humans {
            self.game.state = 1; // [M 0x184268:120-123]
        }
        let bot_outs = self
            .game
            .slots
            .iter()
            .skip(humans)
            .take(bots)
            .filter(|s| s.out && s.deaths >= lives)
            .count();
        if humans <= outs && bots != 0 && bot_outs < bots - 1 {
            if let Some(i) = self.game.slots.iter().position(|s| !s.out) {
                self.game.slots[i].out = true; // the 0x11 flip [M :124-135]
            }
        }
    }

    /// `MP_SortOutWhoWon` 0x1848d0 — state 3 [M :20] + the end status [M :21-92]: 0x10 crowns the
    /// fewest deaths among live slots with deaths < the lives limit — the survivor count
    /// increments BEFORE the compare and a tie at the running minimum zeroes it, so the draw
    /// verdict is "the last qualifier tied the minimum and nothing followed" [M :32-52]; an FFA
    /// needs EXACTLY one slot at/over the frag limit [M :55-85]; a team game always reports the
    /// draw (the team winner is the debriefing's line) [M :87-92]. The status prefixes differ
    /// per branch ("Player %s Won" 0x2000022 for 0x10, "Game Over : …" 0x2000024/0x2000027/
    /// 0x2000023 otherwise).
    pub fn sort_out_who_won(&mut self) {
        self.game.state = 3; // [M 0x1848d0:20]
        let s = self.settings.scenario;
        if s == scenario::DEATH_LIMIT_FFA {
            let lives = self.settings.frag_limit;
            let mut count: i32 = 0; // iVar10 [M 0x1848d0:23-52]
            let mut best = 30000i32; // iVar2's start [M :23]
            let mut winner: Option<usize> = None;
            for (i, slot) in self.game.slots.iter().enumerate() {
                if slot.pos.is_none() {
                    continue; // the obj gate [M :29]
                }
                let d = slot.deaths;
                if d >= lives {
                    continue; // [M :31]
                }
                count += 1;
                if d < best {
                    best = d;
                    winner = Some(i);
                } else if d == best {
                    count = 0; // the tie quirk [M :38-40]
                }
            }
            self.game.end_status = match winner {
                Some(w) if count != 0 => MpEndStatus::Won(self.settings.players[w].codename.clone()),
                _ => MpEndStatus::Draw, // [M :49-52]
            };
        } else if self.settings.team_game == 0 {
            let limit = self.settings.frag_limit as f32;
            let mut count = 0usize;
            let mut winner: Option<usize> = None;
            for (i, slot) in self.game.slots.iter().enumerate() {
                if limit <= slot.score {
                    count += 1; // [M 0x1848d0:62-67]
                    winner = Some(i);
                }
            }
            self.game.end_status = match winner {
                Some(w) if count == 1 => MpEndStatus::Won(self.settings.players[w].codename.clone()),
                _ => MpEndStatus::Draw, // [M :79-84]
            };
        } else {
            self.game.end_status = MpEndStatus::Draw; // [M 0x1848d0:87-92]
        }
    }

    /// `MP_PlayerKilled` 0x188f70 — the victim's slot, the killer's slot (None = the HitByList
    /// walk resolved nothing [M :121-217]) and the +2.0 weapon-bonus arm [M :172-177 — the
    /// killer's weapon block +0xab == 7 with +0x76b == victim; the port takes it as a flag until
    /// the weapon state joins the MP core]. Deaths always +1 [M :92-94]; scenario 0x10 takes the
    /// score −1.0 [M :96-97]; the kill credit gates on the team rule [M :157-163]; the score
    /// delta lands only when `MPSettings+0x190 == 0` and the scenario is Arena/Team Arena/KOH
    /// [M :179-184]; the team scores move only in Team Arena [M :247-274]. The assassin's +5/+3
    /// arms need the target globals — phase 2 [M :223-245].
    pub fn player_killed(&mut self, victim: usize, killer: Option<usize>, double_bonus: bool) {
        if self.settings.master == 0 {
            return; // [M 0x188f70:84-86]
        }
        self.game.slots[victim].deaths += 1; // [M 0x188f70:92-94]
        if self.settings.scenario == scenario::DEATH_LIMIT_FFA {
            self.game.slots[victim].score -= 1.0; // [M :96-97]
        }
        let mut delta = 0.0f32;
        let mut teamkill = false;
        match killer {
            Some(k) if k == victim => {
                // suicide [M :150-155]
                self.game.slots[victim].kills -= 1;
                self.game.slots[victim].streak = 0;
                self.game.slots[victim].last_killer = 0xffff; // [M :154]
                delta = -1.0;
            }
            Some(k) => {
                let teams_differ = self.settings.team_game == 0
                    || self.settings.players[victim].team != self.settings.players[k].team;
                if teams_differ {
                    self.game.slots[k].kills += 1; // [M :157-163]
                    self.game.slots[k].streak += 1;
                } else {
                    teamkill = true;
                }
                self.game.slots[victim].last_killer = k as u16; // [M :164]
                delta = if double_bonus { 2.0 } else { 1.0 }; // [M :166-177]
            }
            None => {}
        }
        // the personal score delta [M :179-184]
        if delta != 0.0
            && self.settings.bit30 == 0
            && matches!(
                self.settings.scenario,
                scenario::DEATHMATCH | scenario::TEAM_DEATHMATCH | scenario::KOTH_FFA | scenario::KOTH_TEAM
            )
        {
            if let Some(k) = killer {
                self.game.slots[k].score += delta;
            }
        }
        // the team scores — Team Arena only [M :247-274]
        if self.settings.scenario == scenario::TEAM_DEATHMATCH {
            let vt = self.settings.players[victim].team.min(1) as usize;
            if teamkill || killer.is_none() || killer == Some(victim) {
                // a teamkill or a suicide: the victim's own team pays [M :249-252]
                self.game.team_scores[vt] -= delta.abs();
            } else {
                // a normal kill: the killer's team = the complement of the victim's [M :262-271]
                let kt = if vt != 0 { 0 } else { 1 };
                self.game.team_scores[kt] += delta.abs();
            }
        }
    }
}
