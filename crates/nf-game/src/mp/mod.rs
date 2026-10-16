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
}

impl MpSettings {
    /// The phase-1 default for `--mp <arena>`: an Arena-1-style deathmatch. The menu's own defaults
    /// are not read here (no menu path in phase 1) — the values are the port's [S], the structure
    /// and the derivations are the game's.
    pub fn deathmatch(level_code: u32) -> MpSettings {
        MpSettings {
            players: std::array::from_fn(|_| MpPlayer { team: 0, skin: 0, armor_bonus: 0, radar_on: 1 }),
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
    /// frag-limit latch compares it against `MPSettings+0x19C`.
    pub high: i32,
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
        }
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
    /// :42-49]; 3 → 4 at once for non-0x10, 0x10 waits the 5 s debrief timer [M :54-61]; 4 → 5
    /// (the game loads the front-end map here — the port has no loader, *Deviation*) [M :62-78];
    /// 6 → the 5 s restart timer then `MP_RestartScenario` back to 0 [M :79-92]. `rng` is the
    /// shared bit-exact RNG — it is drawn only by the state-6 restart's site re-pick. The scenario
    /// objects are the caller's [`MpMatch::update_objects`] call (the game's `MP_ObjectUpdate` is
    /// driven by the object system the same way, and the KOTH hill volume is level data).
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
                    }
                } else {
                    self.game.state = 4;
                }
            }
            4 => {
                // GameState._28_4_ = 0x40000033, ResetMap_LevelToLoad(0x7000048) + the flow push
                // [M 0x184bd0:62-78] — the port has no loader; state 5 marks the wait.
                self.game.state = 5;
            }
            5 => {} // waiting for the front-end load [S]
            6 => {
                self.game.restart_timer += dt; // [M 0x184bd0:79-92]
                if self.game.restart_timer >= 5.0 {
                    self.game.state = 0;
                    self.restart_scenario(rng);
                }
            }
            _ => {}
        }
    }

    /// `MP_CheckForEndCondition` 0x184268 — the full rule set:
    ///
    /// - the clocks: the elapsed delta (+0x19A) adds always, the round clock (+0x190) when not
    ///   paused [M 0x184268:44-46,55,140] (the fixed tick stands in for the wall clock);
    /// - demo/prot: `limit > 0 && limit <= clock` latches the time-up switch and enters state 6
    ///   [M :57-60];
    /// - scenario 0x10: the out-count — humans (class 0x12) and bots (0x11) with
    ///   `frag_limit <= deaths`; `(total_players − 1) <= out` or `humans_out == player_count`
    ///   enters state 1 [M :85-135] (the all-humans-out bot-marking branch needs live bot
    ///   objects — the port's tests drive humans; noted in the doc);
    /// - the time-up latch (`switch_channels[0xfe]`) when `time_limit != −1 && limit <= clock`
    ///   [M :142-145];
    /// - the high score (+0x18C) = max(team 0, team 1), then max over the 8 slot scores for FFA
    ///   (the demo/prot branch takes both) [M :147-172];
    /// - the frag-limit latch (`switch_channels[0xfd]`) when `frag_limit != −1 && frag_limit <=
    ///   high` — skipped for 0x10 (lives only) [M :170-172];
    /// - final: state 1 on the frag latch, else state 2 on the time latch [M :173-184].
    fn check_for_end_condition(&mut self, dt: f32) {
        self.game.elapsed += dt; // [M 0x184268:44-46]
        let s = self.settings.scenario;
        if s == scenario::DEMOLITION || s == scenario::PROTECTION {
            self.game.clock += dt; // [M :55]
            if self.game.limit > 0.0 && !self.game.time_up && self.game.limit <= self.game.clock {
                self.game.time_up = true; // switch_channels[0xfe] [M :142-145]
                self.game.state = 6; // [M :58-60]
            }
            // the high score: max(team, team) then max over the slots [M :147-167]
            let t0 = self.game.team_scores[0] as i32;
            let t1 = self.game.team_scores[1] as i32;
            self.game.high = t0.max(t1);
            for slot in &self.game.slots {
                self.game.high = self.game.high.max(slot.score as i32);
            }
            return; // LAB_00184734's branch flows to the latch tail with the same result [S]
        }
        if s == scenario::DEATH_LIMIT_FFA {
            // the out-count [M :85-135]: humans class 0x12, bots 0x11 — the port counts its
            // `dead`-marked slots whose deaths reached the limit (the class bytes are the BLData
            // pipeline's; *Deviation* noted in the doc)
            let mut out = 0u32;
            let mut humans_out = 0u32;
            let limit = self.settings.frag_limit;
            for (i, slot) in self.game.slots.iter().enumerate() {
                if slot.pos.is_none() {
                    continue; // the obj != 0 gate [M :88]
                }
                let human = i < 4; // humans 0..3, bots 4..7 [M atlas §2]
                if limit <= slot.deaths {
                    out += 1;
                    if human {
                        humans_out += 1;
                    }
                }
            }
            if (self.settings.total_players as u32).wrapping_sub(1) <= out
                || humans_out == self.settings.player_count as u32
            {
                self.game.state = 1; // [M :103-135]
            }
        }
        self.game.clock += dt; // [M :55,140]
        if self.settings.time_limit != -1 && !self.game.time_up {
            self.game.time_up = self.game.limit <= self.game.clock; // [M :142-145]
        }
        // the high score [M :147-172]
        let t0 = self.game.team_scores[0] as i32;
        let t1 = self.game.team_scores[1] as i32;
        self.game.high = t0.max(t1);
        if self.settings.team_game == 0 {
            for slot in &self.game.slots {
                self.game.high = self.game.high.max(slot.score as i32);
            }
        }
        if s != scenario::DEATH_LIMIT_FFA && self.settings.frag_limit != -1 {
            self.game.frag_latched = self.settings.frag_limit <= self.game.high; // [M :170-172]
        }
        if self.game.frag_latched {
            self.game.state = 1; // [M :173-184]
        } else if self.game.time_up {
            self.game.state = 2;
        }
    }
}
