//! MP bots: the data layer — per-bot config records, `default_bot_stats` presets,
//! the bot state table, and the `BotGlobal` dispatch skeleton.
//!
//! Groundwork for phase 4 (atlas §6 step 4). Everything here is data-first [M]/[L];
//! nothing in the sim calls it yet — the dispatch skeleton is behind the `mp-bots`
//! feature and compiled for tests only.
//!
//! Evidence tags (docs/fields/README.md): [M] = decompile in `work/decomp/ACTION.ELF/`
//! (function address = filename prefix; `:NN` = line), [L] = EE snapshot read
//! (`work/ee_snapshot.bin`, file offset == EE address), [S] = suggested.
//! Field map: `docs/fields/bot-vars.md`.

#![allow(dead_code)]

// ============================================================================
// 1. The per-bot config record (0x12 bytes each, array of 4 @ 0x2DEEAA)
// ============================================================================
// `MP_Start` 0x183a50:151 passes `0x2deeaa + i*0x12` to `BOT_init` as the MPBOTS*
// record and reads the team byte `0x2deeb9 + i*0x12` (= record+0x0F) into
// MPSettings+slot*0x30+0x20 [M]. `BOT_respawn` 0x124e38:74 rebuilds the same
// pointer on respawn [M]. `Menu_PrepareBots` 0x1f94a0 compacts enabled records
// (record+0x0E != 0) to the front, sets the count byte 0x2DEEA9 and
// MPSettings+0x432, and forces count 0 for map 0x700004b [M].
//
// Field layout ([M] BOT_init 0x1249e8 reads stats 0x00..0x0D and character
// +0x10; Menu_PrepareBots +0x0E; MP_Start +0x0F; atlas §4):
//
// | off  | type | meaning                                            |
// |------|------|----------------------------------------------------|
// | 0x00 | u8   | aggression (0..9)                                  |
// | 0x01 | u8   | (padding in every snapshot; unused [S])            |
// | 0x02 | u8   | aim (0..9)                                         |
// | 0x03 | u8   | (padding [S])                                      |
// | 0x04 | u16  | max HP (`BOT_setDroneStats` 0x126318 → `BOT_SetHealth`) |
// | 0x06 | u8   | speed level                                        |
// | 0x07 | u8   | recovery A                                         |
// | 0x08 | u8   | recovery B (stun frames = stats[8] x 2/3, atlas §4)|
// | 0x09 | u8   | (unused in the readers found)                      |
// | 0x0A | u8   | preferred weapon class                             |
// | 0x0B | u8   | trait 0..8 (`BOTSTATE_getPreferredTraitOpponentObjIndex` 0x128b50 switches on it) |
// | 0x0C | u8   | flags: bit3 0x08 = 360-degree vision, bit4 0x10 = HP regen (BOT_init 0x1249e8 tests `stats & 0x10` to arm the regen timer) |
// | 0x0D | u8   | trailing stats byte (part of the 0xE stats copy)   |
// | 0x0E | u8   | enabled (`Menu_PrepareBots` 0x1f94a0)              |
// | 0x0F | u8   | team (`MP_Start` 0x183a50:140)                     |
// | 0x10 | u8   | character index (`BOT_init` 0x1249e8: `MP_skins[rec[0x10]]`, stored to MPSettings+slot*0x30+0x24) |
// | 0x11 | u8   | dirty                                              |

/// EE address of the 4-record array (4 x 0x12 bytes).
pub const BOT_RECORDS_ADDR: u32 = 0x2DEEAA;
/// EE address of the bot-count byte just before the array (`Menu_PrepareBots`).
pub const BOT_COUNT_ADDR: u32 = 0x2DEEA9;

/// Size of one config record.
pub const BOT_RECORD_SIZE: usize = 0x12;
/// Number of config records (player slots 4..7).
pub const BOT_RECORD_COUNT: usize = 4;

/// One MP bot config record (0x12 bytes), byte-exact lift of the game layout.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BotRecord(pub [u8; BOT_RECORD_SIZE]);

impl BotRecord {
    pub fn aggression(&self) -> u8 { self.0[0x00] }
    pub fn aim(&self) -> u8 { self.0[0x02] }
    pub fn max_hp(&self) -> u16 { u16::from_le_bytes([self.0[0x04], self.0[0x05]]) }
    pub fn speed(&self) -> u8 { self.0[0x06] }
    pub fn recovery_a(&self) -> u8 { self.0[0x07] }
    pub fn recovery_b(&self) -> u8 { self.0[0x08] }
    pub fn preferred_weapon_class(&self) -> u8 { self.0[0x0A] }
    pub fn trait_id(&self) -> u8 { self.0[0x0B] }
    /// bit3 = 360-degree vision, bit4 = HP regen [M BOT_init 0x1249e8].
    pub fn flags(&self) -> u8 { self.0[0x0C] }
    pub fn has_360_vision(&self) -> bool { self.flags() & 0x08 != 0 }
    pub fn has_hp_regen(&self) -> bool { self.flags() & 0x10 != 0 }
    pub fn enabled(&self) -> bool { self.0[0x0E] != 0 }
    pub fn team(&self) -> u8 { self.0[0x0F] }
    pub fn character(&self) -> u8 { self.0[0x10] }
    pub fn dirty(&self) -> bool { self.0[0x11] != 0 }
    /// The 0xE stats subset copied into `BOT_vars+0xA0` by `BOT_init`
    /// (`amemcpy(0x26d700 + i*0x780, rec, 0xE)`, 0x26d700 = 0x26d660+0xA0) [M].
    pub fn stats(&self) -> &[u8; 14] {
        self.0[0x00..0x0E].try_into().expect("stats are the first 0xE bytes")
    }
}

/// The four records exactly as the disc's initial `.data` image has them
/// (identical bytes in all four EE snapshots at 0x2DEEAA) [L ee_snapshot].
/// Slot 4 = preset 0 with enabled=0 / team=0 / character=6; slots 5..7 are
/// empty with characters 7/8/9. `P_MPBOTSETUP_Handler` 0x211270 rewrites them
/// from the menu.
pub const INITIAL_BOT_RECORDS: [BotRecord; BOT_RECORD_COUNT] = [
    BotRecord([
        0x01, 0x00, 0x04, 0x00, 0x96, 0x00, 0x02, 0x96, 0x32, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x06, 0x00,
    ]), // slot 4
    BotRecord([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x07, 0x00,
    ]), // slot 5
    BotRecord([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x08, 0x00,
    ]), // slot 6
    BotRecord([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x09, 0x00,
    ]), // slot 7
];

// ============================================================================
// 2. `default_bot_stats` — 29 preset rows of 0xE bytes @ 0x26D2F0
// ============================================================================
// `BOT_getDefaultStats(i)` 0x1263a8 returns `default_bot_stats + i * 0xe`
// (ELF symbol: 0x26d2f0, size 406 = 29 x 14) [M-sym]. Rows 0..14 end with
// byte 0x0D == 1, rows 15..28 with 0 == 0 — two groups; `BOT_init` uses row 1
// for the null-record default [M 0x1249e8: `BOT_getDefaultStats(1)`].
// Byte layout = the record's stats subset (see `BotRecord::stats`).
// Values below are byte-identical across all four EE snapshots [L ee_snapshot].

/// EE address of the presets table.
pub const DEFAULT_BOT_STATS_ADDR: u32 = 0x26D2F0;
/// Row size (0xE) and row count (29) — `BOT_getDefaultStats` 0x1263a8 [M].
pub const DEFAULT_BOT_STATS_ROW: usize = 0x0E;
pub const DEFAULT_BOT_STATS_COUNT: usize = 29;

/// One `default_bot_stats` row (0xE bytes).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BotStats(pub [u8; DEFAULT_BOT_STATS_ROW]);

impl BotStats {
    pub fn aggression(&self) -> u8 { self.0[0x00] }
    pub fn aim(&self) -> u8 { self.0[0x02] }
    pub fn max_hp(&self) -> u16 { u16::from_le_bytes([self.0[0x04], self.0[0x05]]) }
    pub fn speed(&self) -> u8 { self.0[0x06] }
    pub fn recovery_a(&self) -> u8 { self.0[0x07] }
    pub fn recovery_b(&self) -> u8 { self.0[0x08] }
    pub fn weapon_byte9(&self) -> u8 { self.0[0x09] }
    pub fn preferred_weapon_class(&self) -> u8 { self.0[0x0A] }
    pub fn trait_id(&self) -> u8 { self.0[0x0B] }
    pub fn flags(&self) -> u8 { self.0[0x0C] }
    pub fn trailing(&self) -> u8 { self.0[0x0D] }
    /// The copy `BOT_setDroneStats` 0x126318 performs into the Drone_tag:
    /// HP → `BOT_SetHealth`, +0 → Drone+0xB4, +2 → +0xB5, +6 → +0xB6
    /// (+0xB7 := 3 hardcoded), +7 → +0xB8, +8 → +0xB9, +9 → +0xBA,
    /// and `(f32)aggression` → Drone+0xC0 [M].
    pub const fn drone_stat_bytes(&self) -> [u8; 7] {
        [self.0[0], self.0[2], self.0[6], 3, self.0[7], self.0[8], self.0[9]]
    }
}

/// The 29 preset rows, byte-exact from 0x26D2F0 [L ee_snapshot].
pub const DEFAULT_BOT_STATS: [BotStats; DEFAULT_BOT_STATS_COUNT] = [
    BotStats([0x01, 0x00, 0x04, 0x00, 0x96, 0x00, 0x02, 0x96, 0x32, 0x00, 0x00, 0x00, 0x00, 0x01]), // 0
    BotStats([0x05, 0x00, 0x02, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x01, 0x00, 0x06, 0x00, 0x01]), // 1
    BotStats([0x03, 0x00, 0x03, 0x00, 0x96, 0x00, 0x01, 0x64, 0x32, 0x01, 0x00, 0x07, 0x00, 0x01]), // 2
    BotStats([0x03, 0x00, 0x03, 0x00, 0x64, 0x00, 0x01, 0x96, 0x64, 0x01, 0x00, 0x00, 0x00, 0x01]), // 3
    BotStats([0x03, 0x00, 0x03, 0x00, 0x64, 0x00, 0x01, 0x7d, 0x64, 0x00, 0x00, 0x02, 0x00, 0x01]), // 4
    BotStats([0x05, 0x00, 0x02, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x00, 0x00, 0x03, 0x00, 0x01]), // 5
    BotStats([0x08, 0x00, 0x02, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x01, 0x00, 0x00, 0x00, 0x01]), // 6
    BotStats([0x03, 0x00, 0x03, 0x00, 0x7d, 0x00, 0x01, 0x7d, 0x4b, 0x01, 0x00, 0x00, 0x00, 0x01]), // 7
    BotStats([0x05, 0x00, 0x02, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x01, 0x00, 0x05, 0x00, 0x01]), // 8
    BotStats([0x03, 0x00, 0x03, 0x00, 0x96, 0x00, 0x01, 0x7d, 0x4b, 0x01, 0x00, 0x00, 0x00, 0x01]), // 9
    BotStats([0x03, 0x00, 0x02, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x01, 0x00, 0x00, 0x00, 0x01]), // 10
    BotStats([0x03, 0x00, 0x03, 0x00, 0xc8, 0x00, 0x02, 0x96, 0x32, 0x01, 0x00, 0x08, 0x00, 0x01]), // 11
    BotStats([0x01, 0x00, 0x04, 0x00, 0x96, 0x00, 0x02, 0x96, 0x32, 0x00, 0x00, 0x00, 0x00, 0x01]), // 12
    BotStats([0x05, 0x00, 0x02, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x01, 0x00, 0x06, 0x00, 0x01]), // 13
    BotStats([0x01, 0x00, 0x04, 0x00, 0x96, 0x00, 0x02, 0x96, 0x32, 0x00, 0x00, 0x00, 0x00, 0x01]), // 14
    BotStats([0x05, 0x00, 0x03, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x01, 0x01, 0x06, 0x00, 0x00]), // 15
    BotStats([0x03, 0x00, 0x04, 0x00, 0xfa, 0x00, 0x01, 0x64, 0x32, 0x01, 0x05, 0x07, 0x00, 0x00]), // 16
    BotStats([0x01, 0x00, 0x04, 0x00, 0x96, 0x00, 0x01, 0x96, 0x64, 0x01, 0x01, 0x08, 0x01, 0x00]), // 17
    BotStats([0x05, 0x00, 0x02, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x00, 0x00, 0x01, 0x00, 0x00]), // 18
    BotStats([0x05, 0x00, 0x02, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x00, 0x00, 0x03, 0x00, 0x00]), // 19
    BotStats([0x03, 0x00, 0x03, 0x00, 0xc8, 0x00, 0x02, 0xc8, 0x32, 0x00, 0x03, 0x04, 0x02, 0x00]), // 20
    BotStats([0x03, 0x00, 0x04, 0x00, 0xc8, 0x00, 0x01, 0x7d, 0x4b, 0x01, 0x02, 0x00, 0x02, 0x00]), // 21
    BotStats([0x03, 0x00, 0x03, 0x00, 0x96, 0x00, 0x02, 0xc8, 0x32, 0x01, 0x04, 0x08, 0x00, 0x00]), // 22
    BotStats([0x05, 0x00, 0x02, 0x00, 0x64, 0x00, 0x01, 0x64, 0x64, 0x01, 0x00, 0x00, 0x00, 0x00]), // 23
    BotStats([0x08, 0x00, 0x03, 0x00, 0x2c, 0x01, 0x00, 0x32, 0xc8, 0x01, 0x00, 0x00, 0x04, 0x00]), // 24
    BotStats([0x05, 0x00, 0x02, 0x00, 0x96, 0x00, 0x02, 0x96, 0x64, 0x01, 0x00, 0x00, 0x18, 0x00]), // 25
    BotStats([0x03, 0x00, 0x03, 0x00, 0x96, 0x00, 0x01, 0x64, 0x96, 0x01, 0x00, 0x00, 0x04, 0x00]), // 26
    BotStats([0x03, 0x00, 0x03, 0x00, 0x64, 0x00, 0x02, 0x7d, 0x4b, 0x01, 0x01, 0x00, 0x00, 0x00]), // 27
    BotStats([0x03, 0x00, 0x04, 0x00, 0x96, 0x00, 0x01, 0x7d, 0x4b, 0x01, 0x00, 0x06, 0x00, 0x00]), // 28
];

// ============================================================================
// 3. The bot state table — ids 0xC3..0xF9
// ============================================================================
// The id → function mapping is `NDrone2_StateFuncs` @ 0x29B300 (250 u32
// entries; read at ids 0xC3..0xF9 from the EE snapshot, byte-identical in all
// four) [L ee_snapshot]. The id → type mapping is a 0x37-byte u8 table at
// 0x26F460 — one past the end of `BOT_vars` — read by
// `BOTSTATE_getStateType` 0x1291f0 as `BOT_vars[state + 0x1D3D]` for
// `state - 0xC3 < 0x37` [M; bytes L ee_snapshot]. State types group the
// family: 1 init, 2 global, 3 trait, 4 attack, 12 cover, 5
// stuck/alert/goto-goal, 6 seen/heard, 7 impact, 10 death, 8 door,
// 13 guard-friend, 9 goal-idle.

/// First bot state id (`NDrone2_DSTATE_BotInit`).
pub const BOT_STATE_FIRST: u8 = 0xC3;
/// One past the last bot state id (0xF9 + 1).
pub const BOT_STATE_END: u8 = 0xFA;
/// Number of bot states (0xFA - 0xC3 = 55).
pub const BOT_STATE_COUNT: usize = (BOT_STATE_END - BOT_STATE_FIRST) as usize;
/// EE address of the state → type byte table (`BOTSTATE_getStateType` 0x1291f0).
pub const BOT_STATE_TYPES_ADDR: u32 = 0x26F460;
/// EE address of the game's state-function pointer table.
pub const BOT_STATE_FUNCS_ADDR: u32 = 0x29B300;

pub mod state {
    /// State ids 0xC3..0xF9, names from the decompile symbols (`action-symtab.tsv`,
    /// addresses cross-checked against `NDrone2_StateFuncs` @0x29B300 [L ee_snapshot]).
    pub const BOT_INIT: u8 = 0xC3;
    pub const BOT_RESPAWN: u8 = 0xC4;
    pub const BOT_GLOBAL: u8 = 0xC5;
    pub const BOT_COLLECTOR: u8 = 0xC6;
    pub const BOT_GUARDIAN: u8 = 0xC7;
    pub const BOT_TEAM_PLAYER: u8 = 0xC8;
    pub const BOT_BULLY: u8 = 0xC9;
    pub const BOT_BERSERKER: u8 = 0xCA;
    pub const BOT_GREEDY: u8 = 0xCB;
    pub const BOT_VENGEFUL: u8 = 0xCC;
    pub const BOT_JUDGE: u8 = 0xCD;
    pub const BOT_ASSASSIN: u8 = 0xCE;
    pub const BOT_ATTACK: u8 = 0xCF;
    pub const BOT_ATTACK_RUN: u8 = 0xD0;
    pub const BOT_ATTACK_NO_ROUTE: u8 = 0xD1;
    pub const BOT_ATTACK_FIRE: u8 = 0xD2;
    pub const BOT_ATTACK_NO_OPPONENT: u8 = 0xD3;
    pub const BOT_ATTACK_STRAFE_AIM_LEFT: u8 = 0xD4;
    pub const BOT_ATTACK_STRAFE_AIM_RIGHT: u8 = 0xD5;
    pub const BOT_ATTACK_RUN_CHANGE_POSITION: u8 = 0xD6;
    pub const BOT_ATTACK_BACKOFF: u8 = 0xD7;
    pub const BOT_ATTACK_CROUCH: u8 = 0xD8;
    pub const BOT_ATTACK_ROLL_LEFT_CROUCH: u8 = 0xD9;
    pub const BOT_ATTACK_ROLL_RIGHT_CROUCH: u8 = 0xDA;
    pub const BOT_ATTACK_STEP_AIM_LEFT: u8 = 0xDB;
    pub const BOT_ATTACK_STEP_AIM_RIGHT: u8 = 0xDC;
    pub const BOT_ATTACK_RELOAD: u8 = 0xDD;
    pub const BOT_ATTACK_CHANGE_WEAPON: u8 = 0xDE;
    pub const BOT_ATTACK_UNARMED: u8 = 0xDF;
    pub const BOT_COVER_RUN_TO: u8 = 0xE0;
    pub const BOT_COVER_INIT: u8 = 0xE1;
    pub const BOT_COVER_IDLE: u8 = 0xE2;
    pub const BOT_COVER_AIM: u8 = 0xE3;
    pub const BOT_COVER_FIRE: u8 = 0xE4;
    pub const BOT_COVER_RETURN: u8 = 0xE5;
    pub const BOT_COVER_TYPE_CHANGE: u8 = 0xE6;
    pub const BOT_COVER_LEAVE: u8 = 0xE7;
    pub const BOT_COVER_LEAVE_NOW: u8 = 0xE8;
    pub const BOT_STUCK: u8 = 0xE9;
    pub const BOT_ALERT_TO_POSITION: u8 = 0xEA;
    pub const BOT_GOTO_GOAL_POSITION: u8 = 0xEB;
    pub const BOT_SEEN_OPPONENT: u8 = 0xEC;
    pub const BOT_SEEN_DRONE_SHOT: u8 = 0xED;
    pub const BOT_HEARD_NOISE: u8 = 0xEE;
    pub const BOT_IMPACT_BULLET: u8 = 0xEF;
    pub const BOT_IMPACT_EXPLOSIVE: u8 = 0xF0;
    pub const BOT_IMPACT_PUNCH: u8 = 0xF1;
    pub const BOT_DEATH_ANIM: u8 = 0xF2;
    pub const BOT_DEATH_BY_EXPLOSION: u8 = 0xF3;
    pub const BOT_DEAD: u8 = 0xF4;
    pub const BOT_IMPACT_STUN_GRENADE: u8 = 0xF5;
    pub const BOT_DOOR_OPEN: u8 = 0xF6;
    pub const BOT_GUARD_FRIEND_IDLE: u8 = 0xF7;
    pub const BOT_GUARD_FRIEND_FOLLOW: u8 = 0xF8;
    pub const BOT_IDLE: u8 = 0xF9;
}

/// One row of the lifted state table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BotStateInfo {
    /// State id (0xC3..0xF9).
    pub id: u8,
    /// Function address in ACTION.ELF.
    pub addr: u32,
    /// Short name (the decompile symbol minus prefixes).
    pub name: &'static str,
    /// Classification for the phase-4 port (see the report): Full (real logic),
    /// Stub (ENTER → one SetState), Thin (a few shared-helper calls), Empty.
    pub kind: StateKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateKind {
    /// Real per-state logic (movement/anim/weapon decisions).
    Full,
    /// A handful of shared-helper calls around one decision.
    Thin,
    /// ENTER (msg 1) does one `Drone_SM_SetState` and nothing else.
    Stub,
    /// No code at all (falls through / returns a constant).
    Empty,
    /// The msg-id dispatcher (0xC5).
    Global,
}

use state::*;

/// The 55 bot states: id, address [L ee_snapshot @0x29B300], name, and the
/// phase-4 classification (kind from reading every .c in work/decomp/ACTION.ELF;
/// details in docs/reports/mp-bots-data.md).
pub const BOT_STATES: [BotStateInfo; BOT_STATE_COUNT] = [
    BotStateInfo { id: BOT_INIT, addr: 0x1748C8, name: "BotInit", kind: StateKind::Thin },
    BotStateInfo { id: BOT_RESPAWN, addr: 0x174938, name: "BotRespawn", kind: StateKind::Thin },
    BotStateInfo { id: BOT_GLOBAL, addr: 0x16E060, name: "BotGlobal", kind: StateKind::Global },
    BotStateInfo { id: BOT_COLLECTOR, addr: 0x174980, name: "BotCollector", kind: StateKind::Full },
    BotStateInfo { id: BOT_GUARDIAN, addr: 0x174A28, name: "BotGuardian", kind: StateKind::Stub },
    BotStateInfo { id: BOT_TEAM_PLAYER, addr: 0x174A70, name: "BotTeamPlayer", kind: StateKind::Stub },
    BotStateInfo { id: BOT_BULLY, addr: 0x174AB8, name: "BotBully", kind: StateKind::Stub },
    BotStateInfo { id: BOT_BERSERKER, addr: 0x174B00, name: "BotBerserker", kind: StateKind::Stub },
    BotStateInfo { id: BOT_GREEDY, addr: 0x174B48, name: "BotGreedy", kind: StateKind::Stub },
    BotStateInfo { id: BOT_VENGEFUL, addr: 0x174B90, name: "BotVengeful", kind: StateKind::Stub },
    BotStateInfo { id: BOT_JUDGE, addr: 0x174BD8, name: "BotJudge", kind: StateKind::Stub },
    BotStateInfo { id: BOT_ASSASSIN, addr: 0x174C20, name: "BotAssassin", kind: StateKind::Stub },
    BotStateInfo { id: BOT_ATTACK, addr: 0x16F198, name: "BotAttack", kind: StateKind::Full },
    BotStateInfo { id: BOT_ATTACK_RUN, addr: 0x16F390, name: "BotAttackRun", kind: StateKind::Full },
    BotStateInfo { id: BOT_ATTACK_NO_ROUTE, addr: 0x174C68, name: "BotAttackNoRoute", kind: StateKind::Thin },
    BotStateInfo { id: BOT_ATTACK_FIRE, addr: 0x16F5E8, name: "BotAttackFire", kind: StateKind::Full },
    BotStateInfo { id: BOT_ATTACK_NO_OPPONENT, addr: 0x174CD0, name: "BotAttackNoOpponent", kind: StateKind::Stub },
    BotStateInfo { id: BOT_ATTACK_STRAFE_AIM_LEFT, addr: 0x16F850, name: "BotAttackStrafeAimLeft", kind: StateKind::Full },
    BotStateInfo { id: BOT_ATTACK_STRAFE_AIM_RIGHT, addr: 0x16F9B8, name: "BotAttackStrafeAimRight", kind: StateKind::Full },
    BotStateInfo { id: BOT_ATTACK_RUN_CHANGE_POSITION, addr: 0x16FB20, name: "BotAttackRunChangePosition", kind: StateKind::Full },
    BotStateInfo { id: BOT_ATTACK_BACKOFF, addr: 0x16FC30, name: "BotAttackBackoff", kind: StateKind::Full },
    BotStateInfo { id: BOT_ATTACK_CROUCH, addr: 0x16FE80, name: "BotAttackCrouch", kind: StateKind::Full },
    BotStateInfo { id: BOT_ATTACK_ROLL_LEFT_CROUCH, addr: 0x174D18, name: "BotAttackRollLeftCrouch", kind: StateKind::Thin },
    BotStateInfo { id: BOT_ATTACK_ROLL_RIGHT_CROUCH, addr: 0x174DD0, name: "BotAttackRollRightCrouch", kind: StateKind::Thin },
    BotStateInfo { id: BOT_ATTACK_STEP_AIM_LEFT, addr: 0x174E88, name: "BotAttackStepAimLeft", kind: StateKind::Thin },
    BotStateInfo { id: BOT_ATTACK_STEP_AIM_RIGHT, addr: 0x174FA8, name: "BotAttackStepAimRight", kind: StateKind::Thin },
    BotStateInfo { id: BOT_ATTACK_RELOAD, addr: 0x170190, name: "BotAttackReload", kind: StateKind::Full },
    BotStateInfo { id: BOT_ATTACK_CHANGE_WEAPON, addr: 0x1750C8, name: "BotAttackChangeWeapon", kind: StateKind::Thin },
    BotStateInfo { id: BOT_ATTACK_UNARMED, addr: 0x175188, name: "BotAttackUnarmed", kind: StateKind::Thin },
    BotStateInfo { id: BOT_COVER_RUN_TO, addr: 0x1702E0, name: "BotCoverRunTo", kind: StateKind::Full },
    BotStateInfo { id: BOT_COVER_INIT, addr: 0x170448, name: "BotCoverInit", kind: StateKind::Full },
    BotStateInfo { id: BOT_COVER_IDLE, addr: 0x1705B0, name: "BotCoverIdle", kind: StateKind::Full },
    BotStateInfo { id: BOT_COVER_AIM, addr: 0x1708E8, name: "BotCoverAim", kind: StateKind::Full },
    BotStateInfo { id: BOT_COVER_FIRE, addr: 0x1709D0, name: "BotCoverFire", kind: StateKind::Full },
    BotStateInfo { id: BOT_COVER_RETURN, addr: 0x170B90, name: "BotCoverReturn", kind: StateKind::Full },
    BotStateInfo { id: BOT_COVER_TYPE_CHANGE, addr: 0x170D00, name: "BotCoverTypeChange", kind: StateKind::Full },
    BotStateInfo { id: BOT_COVER_LEAVE, addr: 0x170E20, name: "BotCoverLeave", kind: StateKind::Full },
    BotStateInfo { id: BOT_COVER_LEAVE_NOW, addr: 0x170F38, name: "BotCoverLeaveNow", kind: StateKind::Full },
    BotStateInfo { id: BOT_STUCK, addr: 0x175220, name: "BotStuck", kind: StateKind::Stub },
    BotStateInfo { id: BOT_ALERT_TO_POSITION, addr: 0x171048, name: "BotAlertToPosition", kind: StateKind::Full },
    BotStateInfo { id: BOT_GOTO_GOAL_POSITION, addr: 0x171240, name: "BotGotoGoalPosition", kind: StateKind::Full },
    BotStateInfo { id: BOT_SEEN_OPPONENT, addr: 0x175268, name: "BotSeenOpponent", kind: StateKind::Stub },
    BotStateInfo { id: BOT_SEEN_DRONE_SHOT, addr: 0x1752B0, name: "BotSeenDroneShot", kind: StateKind::Stub },
    BotStateInfo { id: BOT_HEARD_NOISE, addr: 0x1752F8, name: "BotHeardNoise", kind: StateKind::Empty },
    BotStateInfo { id: BOT_IMPACT_BULLET, addr: 0x1713F0, name: "BotImpactBullet", kind: StateKind::Full },
    BotStateInfo { id: BOT_IMPACT_EXPLOSIVE, addr: 0x175308, name: "BotImpactExplosive", kind: StateKind::Thin },
    BotStateInfo { id: BOT_IMPACT_PUNCH, addr: 0x175388, name: "BotImpactPunch", kind: StateKind::Thin },
    BotStateInfo { id: BOT_DEATH_ANIM, addr: 0x175508, name: "BotDeathAnim", kind: StateKind::Thin },
    BotStateInfo { id: BOT_DEATH_BY_EXPLOSION, addr: 0x1755A8, name: "BotDeathByExplosion", kind: StateKind::Thin },
    BotStateInfo { id: BOT_DEAD, addr: 0x171518, name: "BotDead", kind: StateKind::Full },
    BotStateInfo { id: BOT_IMPACT_STUN_GRENADE, addr: 0x175418, name: "BotImpactStunGrenade", kind: StateKind::Thin },
    BotStateInfo { id: BOT_DOOR_OPEN, addr: 0x1716D0, name: "BotDoorOpen", kind: StateKind::Full },
    BotStateInfo { id: BOT_GUARD_FRIEND_IDLE, addr: 0x175680, name: "BotGuardFriendIdle", kind: StateKind::Full },
    BotStateInfo { id: BOT_GUARD_FRIEND_FOLLOW, addr: 0x1718E0, name: "BotGuardFriendFollow", kind: StateKind::Full },
    BotStateInfo { id: BOT_IDLE, addr: 0x171B80, name: "BotIdle", kind: StateKind::Full },
];

/// The state → type table at 0x26F460, 0x37 bytes [L ee_snapshot; reader
/// `BOTSTATE_getStateType` 0x1291f0]. Index = state - 0xC3.
pub const BOT_STATE_TYPES: [u8; BOT_STATE_COUNT] = [
    0x01, 0x01, 0x02, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, // C3..CE
    0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, // CF..DA
    0x04, 0x04, 0x04, 0x04, 0x04, // DB..DF (17 attack entries total)
    0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, // E0..E8
    0x05, 0x05, 0x05, // E9..EB
    0x06, 0x06, 0x06, // EC..EE
    0x07, 0x07, 0x07, // EF..F1
    0x0A, 0x0A, 0x0A, // F2..F4
    0x07, // F5
    0x08, // F6
    0x0D, 0x0D, // F7..F8
    0x09, // F9
];

/// Port of `BOTSTATE_getStateType` 0x1291f0: the type byte for a bot state id,
/// or 0 for anything outside 0xC3..0xF9. [M 0x1291f0]
pub const fn state_type(state: u32) -> u8 {
    if state.wrapping_sub(BOT_STATE_FIRST as u32) < BOT_STATE_COUNT as u32 {
        BOT_STATE_TYPES[(state - BOT_STATE_FIRST as u32) as usize]
    } else {
        0
    }
}

// ============================================================================
// 4. The `BotGlobal` dispatch skeleton (state 0xC5, 0x16E060)
// ============================================================================
// `NDrone2_DSTATE_BotGlobal` 0x16E060 is the bots' message router — itself
// state id 0xC5 in `NDrone2_StateFuncs` [L ee_snapshot]. It is a switch on the
// MsgObject id (`*(int*)msg`), 759 decompiled lines. This skeleton mirrors its
// routing only: each arm records what the game does (which helper it calls /
// which state it switches to), so phase 4 turns the arms into real calls.
// Compiled only for tests or with the `mp-bots` feature — nothing calls it.

#[cfg(any(test, feature = "mp-bots"))]
/// A bot message id, the way `NDrone2_DSTATE_BotGlobal` switches on `*msg` [M 0x16E060].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BotMsg {
    pub id: u8,
    /// `msg[6]` — the payload word the goal/kill/weapon arms read (`puVar17[6]`).
    pub arg: u32,
    /// The sender/current-state word (`msg[1]` in the decompile: `*(int*)(iVar16+4)`).
    pub current_state: u32,
}

/// What the game does per message — the skeleton's outcome descriptors.
#[cfg(any(test, feature = "mp-bots"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BotGlobalOutcome {
    /// `return 0` without action (msg 0x00 and the 0x2F..0x39/0x3E/0x3F block).
    Ignore,
    /// Killed (msg 0x03): dead-drop. If the SMI current state is 0 → SetState(0xF9);
    /// else the type-10 (death) states keep, a dead/invalid opponent clears the
    /// MPGame slot status bits (+0x26) and SetState(0xF2) unless the type is 10.
    Killed { goto: u8 },
    /// Alert family (0x11..0x16, 0x1E): forward to `NDrone2_ReactToDroneAlertMsg`
    /// unless the current state's type is 0/1/2/4/10.
    Alert { react: bool },
    /// Msg 0x18 (impact/explosion alert?): current-state type != 10 → SetState(0xF5 stun).
    StunIfNotDeath { stun: bool },
    /// Msg 0x2E (state change): cover disown / +0x758 clear / alert-status set,
    /// `+0x766 = state_type(state)`; the skeleton reports the recorded fields.
    StateChange { clear_opponent: bool, state_type: u8 },
    /// Msg 0x3A: no current opponent → weight the objective (+1.0), pickGoal(1),
    /// goal found → SetState(0xEB goto goal position).
    SeekObjective { goto: u8 },
    /// Msg 0x3B: `BOTSTATE_setGoalComplete(slot = arg)`.
    GoalComplete { slot: u32 },
    /// Msg 0x3C: `BOTSTATE_uninitGoal(slot = arg)`.
    GoalUninit { slot: u32 },
    /// Msg 0x3D: `BOTSTATE_cancelGoalToObj(arg)`, then SetState(0xF9).
    GoalCancel { goto: u8 },
    /// Msgs 0x40/0x41 (demo/protect site): if my opponent == arg, re-target the
    /// mode's site object (`MP_getProtectionObj_NotExt` / `MP_getDemolitionObj_NotExt`).
    ObjectiveSite,
    /// Msg 0x42 (weapon announce): `BOT_vars+0x76A = arg byte`, `Drone+0x5A2 = current_state`.
    WeaponAnnounce { weapon: u8 },
    /// Msg 0x43 (kill): if the killed was my opponent clear `+0x758`; grudge slot
    /// `+0x76B` matches → clear; `cancelGoalToObj`; type-13 (guard-friend) state → SetState(0xF9).
    KillMsg { goto: u8 },
    /// Msg 0x44 (hit): only when I'm the target → `BOTSTATE_combatWeaponChangeChoice`.
    Hit,
    /// Msg 0x45 (weapon change done): weapon choice + default combat range;
    /// missile opponent → drop the active goal; `+0x771` set and state 0xF9 → SetState(0xCF attack).
    WeaponChanged { goto: u8 },
    /// Any other id: fall through to `return 1`.
    Pass,
}

/// The dispatch skeleton of `NDrone2_DSTATE_BotGlobal` 0x16E060 [M 0x16E060:37-759].
/// `scenario` is MPSettings+0x420 (the mode word — the 0x40/0x41 arms read it);
/// `has_opponent` mirrors `Drone+0x170 != 0`; `goal_slot_flag` mirrors the
/// `BOT_vars+0x771` re-entry flag.
#[cfg(any(test, feature = "mp-bots"))]
pub fn bot_global_dispatch(
    msg: BotMsg,
    scenario: u32,
    has_opponent: bool,
    goal_slot_flag: bool,
) -> BotGlobalOutcome {
    match msg.id {
        // case 0, 0x2F..0x39, 0x3E, 0x3F: the block falls to `return 0` [M :66-83]
        0x00 | 0x2F..=0x39 | 0x3E | 0x3F => BotGlobalOutcome::Ignore,

        // case 3 (killed) [M :84-114, 401-459]
        0x03 => {
            if msg.current_state == 0 {
                BotGlobalOutcome::Killed { goto: state::BOT_IDLE }
            } else {
                let ty = state_type(msg.current_state);
                // invalid opponent (the universal gate) → slot status cleared;
                // type 10 (death) states stay; anything else → 0xF2 death anim.
                if ty == 10 { BotGlobalOutcome::Killed { goto: msg.current_state as u8 } }
                else { BotGlobalOutcome::Killed { goto: state::BOT_DEATH_ANIM } }
            }
        }

        // 0x11..0x16, 0x1E: alert msgs — react unless state type ∈ {0,1,2,4,10} [M :547-565]
        0x11..=0x16 | 0x1E => {
            let ty = state_type(msg.current_state);
            BotGlobalOutcome::Alert { react: !matches!(ty, 0 | 1 | 2 | 4 | 10) }
        }

        // 0x18: not a death state → 0xF5 stun [M :566-573]
        0x18 => BotGlobalOutcome::StunIfNotDeath { stun: state_type(msg.current_state) != 10 },

        // 0x2E: state change [M :574-608]
        0x2E => {
            let ty = state_type(msg.current_state);
            BotGlobalOutcome::StateChange {
                // type != 0xD (impact) and state != 0xEB → clear the +0x758 opponent
                clear_opponent: ty != 13 && msg.current_state != state::BOT_GOTO_GOAL_POSITION as u32,
                state_type: ty,
            }
        }

        // 0x3A: seek an objective when no opponent [M :609-625]
        0x3A => {
            if has_opponent {
                BotGlobalOutcome::Pass
            } else {
                BotGlobalOutcome::SeekObjective { goto: state::BOT_GOTO_GOAL_POSITION }
            }
        }

        // 0x3B/0x3C/0x3D: the goal messages [M :626-634]
        0x3B => BotGlobalOutcome::GoalComplete { slot: msg.arg },
        0x3C => BotGlobalOutcome::GoalUninit { slot: msg.arg },
        0x3D => BotGlobalOutcome::GoalCancel { goto: state::BOT_IDLE },

        // 0x40/0x41: demo/protect site re-target [M :635-651]
        0x40 | 0x41 => {
            let _ = scenario; // the real arm reads MPSettings+0x420 to pick the site
            BotGlobalOutcome::ObjectiveSite
        }

        // 0x42: weapon announce → +0x76A [M :652-655]
        0x42 => BotGlobalOutcome::WeaponAnnounce { weapon: msg.arg as u8 },

        // 0x43: kill message [M :656-701]
        0x43 => {
            let ty = state_type(msg.current_state);
            BotGlobalOutcome::KillMsg {
                goto: if ty == 13 { state::BOT_IDLE } else { 0 },
            }
        }

        // 0x44: hit → weapon re-choice when I am the target [M :702-729]
        0x44 => BotGlobalOutcome::Hit,

        // 0x45: weapon-change done [M :730-757]
        0x45 => BotGlobalOutcome::WeaponChanged {
            goto: if goal_slot_flag { state::BOT_ATTACK } else { 0 },
        },

        _ => BotGlobalOutcome::Pass,
    }
}

// ============================================================================
// 5. Tests — the golden table checks and the dispatch test
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden: the 29 presets are byte-exact against the EE snapshot reads
    /// (spot rows + the structural invariants every row must satisfy).
    /// Bytes recorded from `work/ee_snapshot.bin` @0x26D2F0 [L]; identical in
    /// all four snapshots.
    #[test]
    fn golden_default_bot_stats_table() {
        assert_eq!(DEFAULT_BOT_STATS.len(), 29);
        // row 0: the boot default (slot 4's record and the BOT_init null-record
        // fallback both derive from preset rows)
        assert_eq!(DEFAULT_BOT_STATS[0].0, [0x01, 0x00, 0x04, 0x00, 0x96, 0x00, 0x02, 0x96, 0x32, 0x00, 0x00, 0x00, 0x00, 0x01]);
        // row 1: the BOT_init null-record default (agg 5, aim 2, HP 100, trait 6)
        assert_eq!(DEFAULT_BOT_STATS[1].aggression(), 5);
        assert_eq!(DEFAULT_BOT_STATS[1].aim(), 2);
        assert_eq!(DEFAULT_BOT_STATS[1].max_hp(), 100);
        assert_eq!(DEFAULT_BOT_STATS[1].trait_id(), 6);
        // row 16: the 250-HP preset
        assert_eq!(DEFAULT_BOT_STATS[16].max_hp(), 250);
        // row 24: 300 HP, speed 0 — the extreme row
        assert_eq!(DEFAULT_BOT_STATS[24].max_hp(), 300);
        assert_eq!(DEFAULT_BOT_STATS[24].speed(), 0);
        // last row
        assert_eq!(DEFAULT_BOT_STATS[28].trait_id(), 6);
        // structural invariants across all 29 rows
        for (i, r) in DEFAULT_BOT_STATS.iter().enumerate() {
            assert!(r.aggression() <= 9, "row {i} aggression out of range");
            assert!(r.aim() <= 9, "row {i} aim out of range");
            assert!(r.max_hp() >= 50 && r.max_hp() <= 300, "row {i} HP out of range");
            assert!(r.trait_id() <= 8, "row {i} trait out of range");
            assert!(r.speed() <= 2, "row {i} speed out of range");
        }
        // the two groups: rows 0..14 carry trailing byte 1, rows 15..28 carry 0
        for (i, r) in DEFAULT_BOT_STATS.iter().enumerate() {
            let want = if i < 15 { 1 } else { 0 };
            assert_eq!(r.trailing(), want, "row {i} trailing byte");
        }
        // rows 0, 12, 14 are the same preset (triplicated in the game data)
        assert_eq!(DEFAULT_BOT_STATS[0].0, DEFAULT_BOT_STATS[12].0);
        assert_eq!(DEFAULT_BOT_STATS[0].0, DEFAULT_BOT_STATS[14].0);
    }

    /// Golden: the boot `.data` image of the config records @0x2DEEAA [L
    /// ee_snapshot]: slot 4 = preset 0 disabled with character 6, slots 5..7
    /// empty with characters 7/8/9.
    #[test]
    fn golden_initial_bot_records() {
        let r0 = &INITIAL_BOT_RECORDS[0];
        assert_eq!(r0.stats(), &DEFAULT_BOT_STATS[0].0, "slot 4 = preset 0 stats");
        assert!(!r0.enabled());
        assert_eq!(r0.team(), 0);
        assert_eq!(r0.character(), 6);
        for (i, rec) in INITIAL_BOT_RECORDS.iter().enumerate() {
            assert_eq!(rec.character() as usize, 6 + i, "slot {} character", 4 + i);
            assert!(!rec.enabled(), "slot {} boots disabled", 4 + i);
            assert!(!rec.dirty());
        }
        for rec in INITIAL_BOT_RECORDS.iter().skip(1) {
            assert_eq!(rec.aggression(), 0);
            assert_eq!(rec.max_hp(), 0);
        }
        // Menu_PrepareBots compacts enabled records; with the boot image none are enabled.
        assert_eq!(INITIAL_BOT_RECORDS.iter().filter(|r| r.enabled()).count(), 0);
    }

    /// The state table: 55 ids 0xC3..0xF9, the addresses match the game's
    /// `NDrone2_StateFuncs` rows [L ee_snapshot @0x29B300], every id maps
    /// through `state_type`, and the type groups match the snapshot table.
    #[test]
    fn state_table_and_state_types() {
        assert_eq!(BOT_STATES.len(), 55);
        for (n, st) in BOT_STATES.iter().enumerate() {
            assert_eq!(st.id as usize, 0xC3 + n);
            assert!(st.addr >= 0x16E060 && st.addr < 0x180000, "{} addr", st.name);
        }
        // spot addresses from the snapshot pointer table
        let addr = |id: u8| BOT_STATES[(id - BOT_STATE_FIRST) as usize].addr;
        assert_eq!(addr(state::BOT_INIT), 0x1748C8);
        assert_eq!(addr(state::BOT_GLOBAL), 0x16E060);
        assert_eq!(addr(state::BOT_ATTACK), 0x16F198);
        assert_eq!(addr(state::BOT_GOTO_GOAL_POSITION), 0x171240);
        assert_eq!(addr(state::BOT_IDLE), 0x171B80);
        // the type table, spot rows [L 0x26F460]
        assert_eq!(state_type(state::BOT_INIT as u32), 1);
        assert_eq!(state_type(state::BOT_GLOBAL as u32), 2);
        assert_eq!(state_type(state::BOT_ASSASSIN as u32), 3);
        assert_eq!(state_type(state::BOT_ATTACK as u32), 4);
        assert_eq!(state_type(state::BOT_COVER_IDLE as u32), 12);
        assert_eq!(state_type(state::BOT_GOTO_GOAL_POSITION as u32), 5);
        assert_eq!(state_type(state::BOT_SEEN_OPPONENT as u32), 6);
        assert_eq!(state_type(state::BOT_IMPACT_BULLET as u32), 7);
        assert_eq!(state_type(state::BOT_DEATH_ANIM as u32), 10);
        assert_eq!(state_type(state::BOT_IMPACT_STUN_GRENADE as u32), 7);
        assert_eq!(state_type(state::BOT_DOOR_OPEN as u32), 8);
        assert_eq!(state_type(state::BOT_GUARD_FRIEND_IDLE as u32), 13);
        assert_eq!(state_type(state::BOT_IDLE as u32), 9);
        // out-of-range ids read 0, exactly like BOTSTATE_getStateType [M 0x1291f0]
        assert_eq!(state_type(0xC2), 0);
        assert_eq!(state_type(0xFA), 0);
        assert_eq!(state_type(0), 0);
    }

    /// The `BotGlobal` dispatch skeleton routes like the game's switch [M 0x16E060].
    #[test]
    fn bot_global_dispatch_routing() {
        let msg = |id: u8, arg: u32, cs: u32| BotMsg { id, arg, current_state: cs };
        // the ignore block
        assert_eq!(bot_global_dispatch(msg(0x00, 0, 0), 0, false, false), BotGlobalOutcome::Ignore);
        assert_eq!(bot_global_dispatch(msg(0x2F, 0, 0), 0, false, false), BotGlobalOutcome::Ignore);
        assert_eq!(bot_global_dispatch(msg(0x39, 0, 0), 0, false, false), BotGlobalOutcome::Ignore);
        assert_eq!(bot_global_dispatch(msg(0x3F, 0, 0), 0, false, false), BotGlobalOutcome::Ignore);
        // killed with a non-death current state → 0xF2; death state → stays
        assert_eq!(
            bot_global_dispatch(msg(0x03, 0, state::BOT_ATTACK as u32), 0, false, false),
            BotGlobalOutcome::Killed { goto: state::BOT_DEATH_ANIM }
        );
        assert_eq!(
            bot_global_dispatch(msg(0x03, 0, state::BOT_DEAD as u32), 0, false, false),
            BotGlobalOutcome::Killed { goto: state::BOT_DEAD }
        );
        // alert family: react unless the current state's type is exempt
        // (0xF9 BotIdle is type 9 → react; 0xCF BotAttack type 4 → no react)
        assert_eq!(
            bot_global_dispatch(msg(0x11, 0, state::BOT_IDLE as u32), 0, false, false),
            BotGlobalOutcome::Alert { react: true }
        );
        assert_eq!(
            bot_global_dispatch(msg(0x14, 0, state::BOT_ATTACK as u32), 0, false, false),
            BotGlobalOutcome::Alert { react: false }
        );
        // stun gate
        assert_eq!(
            bot_global_dispatch(msg(0x18, 0, state::BOT_ATTACK as u32), 0, false, false),
            BotGlobalOutcome::StunIfNotDeath { stun: true }
        );
        assert_eq!(
            bot_global_dispatch(msg(0x18, 0, state::BOT_DEATH_BY_EXPLOSION as u32), 0, false, false),
            BotGlobalOutcome::StunIfNotDeath { stun: false }
        );
        // goal messages carry the slot arg
        assert_eq!(
            bot_global_dispatch(msg(0x3B, 1, 0), 0, false, false),
            BotGlobalOutcome::GoalComplete { slot: 1 }
        );
        assert_eq!(
            bot_global_dispatch(msg(0x3C, 0, 0), 0, false, false),
            BotGlobalOutcome::GoalUninit { slot: 0 }
        );
        assert_eq!(
            bot_global_dispatch(msg(0x3D, 0, 0), 0, false, false),
            BotGlobalOutcome::GoalCancel { goto: state::BOT_IDLE }
        );
        // objective seek only without an opponent
        assert_eq!(
            bot_global_dispatch(msg(0x3A, 0, 0), 0, false, false),
            BotGlobalOutcome::SeekObjective { goto: state::BOT_GOTO_GOAL_POSITION }
        );
        assert_eq!(bot_global_dispatch(msg(0x3A, 0, 0), 0, true, false), BotGlobalOutcome::Pass);
        // weapon announce records the weapon byte
        assert_eq!(
            bot_global_dispatch(msg(0x42, 0x16, 0), 0, false, false),
            BotGlobalOutcome::WeaponAnnounce { weapon: 0x16 }
        );
        // demo/protect site arms
        assert_eq!(
            bot_global_dispatch(msg(0x40, 0, 0), 0x2000_0040, false, false),
            BotGlobalOutcome::ObjectiveSite
        );
        // weapon change done with the +0x771 re-entry flag → 0xCF attack
        assert_eq!(
            bot_global_dispatch(msg(0x45, 0, state::BOT_IDLE as u32), 0, false, true),
            BotGlobalOutcome::WeaponChanged { goto: state::BOT_ATTACK }
        );
        assert_eq!(
            bot_global_dispatch(msg(0x45, 0, state::BOT_IDLE as u32), 0, false, false),
            BotGlobalOutcome::WeaponChanged { goto: 0 }
        );
        // unhandled ids fall through
        assert_eq!(bot_global_dispatch(msg(0x7F, 0, 0), 0, false, false), BotGlobalOutcome::Pass);
    }
}
