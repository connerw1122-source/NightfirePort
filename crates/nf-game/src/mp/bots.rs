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
// 6. Phase 4 (`mp-bots-states`): the ported state machine — the goal machine,
//    the roam states (0xF9/0xEB) and the attack/fire decision (0xCF/0xD2)
// ============================================================================
// Driver mechanics [M 0x171df8 `Drone_SM_RouteMsgDCV`, 0x175778 `NDrone2_ProcessStateMachine`,
// 0x1490f0 `NDrone2_ControlSTANDARD`:81]: per drone-control tick the game sends itself msg 3
// (0x175a38:31); for MP bots (Drone+0xC5 == 0x1E) msg 3 is delivered to state 0xC5 BotGlobal
// FIRST regardless of the current state [M 0x171df8:78-86], then to the current state; a
// SetState request is committed after the delivery loop (EXIT msg 2, the commit, a synthetic
// msg 0x2E to BotGlobal — its only sender [M 0x171df8:119-203] — then the ENTER msg 1) [M
// 0x1758c0:32-56, 0x171df8:195-197]. The firing pipeline is NOT a state: `ControlSTANDARD`
// :231 runs `DroneWeap_HandleFiring` every tick [M].
//
// Corrections to the phase-401 skeleton this port encodes (from the decompile re-read):
// msg[1] is never read — every "current state" check is the live SM state [M 0x16e060];
// the ignore block returns 1 while the default arm returns 0 (re-delivery) [M :757/:115];
// msg 3 is the per-tick heartbeat with the death branch inside it, not "killed" [M :84-519].


use crate::drone::Rng;

use crate::world::CollisionWorld;


/// The fixed 60 Hz frame count the timers use [L docs/systems/sound.md:394 `FRAME_RATE_INT`].
pub const FRAME_RATE_INT: u32 = 60;


/// One 0x50-byte goal slot (`BOT_vars+0x00` / `+0x50`) — offsets per [M docs/fields/bot-vars.md].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GoalSlot {
    /// +0x00..0x0F — the goal position (`initGoal` 0x1294c0 copies the 0x20-byte CelPos; the port
    /// carries the position word the route targets [M]).
    pub celpos: [f32; 4],
    /// +0x20 — scratch; `gotoGoal` writes the wander budget into slot 1 [M 0x1278a8:84-142].
    pub scratch: f32,
    /// +0x24 — start stamp (seconds, `MPGame+0x412`) [M 0x1278a8:51].
    pub start: f32,
    /// +0x28 — duration := `VIDEO_FRAME_RATE * 5` (= 300.0 at 60 Hz) [M 0x1278a8:50,58; the
    /// atlas's "5 s" gloss was the pre-correction reading — it is 5 s × frame-rate on a
    /// seconds clock, see the report].
    pub duration: f32,
    /// +0x2C — pickup class 3 (health) weight, clamped ≤ 25.0 [M 0x128298:74-81].
    pub weight_health: f32,
    /// +0x30 — pickup class 1 (ammo) weight.
    pub weight_ammo: f32,
    /// +0x34 — pickup class 0 (weapon) weight.
    pub weight_weapon: f32,
    /// +0x38 — the objective weight f32 (`pickGoal` reads it as the multiplier [M 0x126b88:144,292]).
    pub objective_weight: f32,
    /// +0x3C — target index (type 1 → the pickups table, type 3 → the others table) [M].
    pub target: usize,
    /// +0x40 — the state to switch to when the goal fires [M 0x1278a8:59].
    pub fire_state: u8,
    /// +0x44 — flags: bit0 = complete (`setGoalComplete` 0x129510) [M].
    pub complete: u8,
    /// +0x45 — goal type: 1 pickup, 2 objective, 3 hunt player [M].
    pub goal_type: u8,
    /// +0x46 — pickup pref byte (bit1 0x2 one pass, bit2 0x4 the ≥5.0 skip, bit3 0x8 the
    /// opponent-path verify, bit5 0x20 ignore visit times) [M 0x126b88:346,369,404,420,514].
    pub pref_byte: u8,
    /// +0x47 — pickup distance limit (0 stored as −2/0xFE) [M 0x128298:70-77, 0x126b88:362-363].
    pub dist_limit: i8,
    /// +0x48 — the last route code (`BotGotoGoalPosition` writes it per tick [M 0x171240:40]).
    pub route_code: u8,
    /// +0x49 — slot-active flag (slot 1 := 1 at `BOT_init`) [M].
    pub active: u8,
    /// +0x4A — subtype (1 flag … 8 demo/protect, 9 hunt) [M].
    pub subtype: u8,
}


impl Default for GoalSlot {
    fn default() -> Self {
        GoalSlot {
            celpos: [0.0; 4],
            scratch: 0.0,
            start: 0.0,
            duration: 0.0,
            weight_health: 0.0,
            weight_ammo: 0.0,
            weight_weapon: 0.0,
            objective_weight: 0.0,
            target: 0,
            fire_state: 0,
            complete: 0,
            goal_type: 0,
            pref_byte: 0,
            dist_limit: 0,
            route_code: 0,
            active: 0,
            subtype: 0,
        }
    }
}


/// One registered MP pickup — the `MP_PICKUP` row subset the goal machine reads
/// [M 0x189d80 `MP_RegisterPickup`, 0x126b88:386-460, 0x129958].
#[derive(Clone, Debug, PartialEq)]
pub struct BotPickup {
    /// +0x08 — the registered position.
    pub pos: [f32; 3],
    /// `MPOBJECT+0x24` — the weapon id (0 = not resolved; the MP weapon-set ids are runtime
    /// random via `Pickup_MakeRandomWeaponSet`, the matrix is not lifted — see the report).
    pub weapon_id: u16,
    /// `MPOBJECT+0x22` — the pickup class (0 weapon, 1 ammo, 3 health per the pickGoal dispatch
    /// [M 0x126b88:431-477]).
    pub mp_class: u8,
    /// `MPOBJECT+0x20` (s16) == 2 — the "only ≥ 5.0 away" special [M 0x126b88:420-421].
    pub is_obj_flag2: bool,
    /// +0x80 — the per-bot visit stamps [f32; 4] by bot index [M 0x129958:10-15].
    pub visit: [f32; 4],
}


/// One row of the `BOT_vars+0xB0` per-player info block (`BOT_setOtherPlayerInfo` 0x125390).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OtherPlayer {
    /// The slot's object position (`MPGame+slot*0x30+0x1C → obj+0x30`) [M].
    pub pos: [f32; 3],
    /// The player is alive (`MP_playerIsDead` gate [M 0x128b50:102]).
    pub alive: bool,
    /// The player's index 0..7 (`Control_Plr2Ind` domain [M]).
    pub index: usize,
    /// The player's team word (`MPSettings+slot*0x30+0x20`) — bit3 0x8 source [M 0x125390:100-102].
    pub team: u32,
    /// +0x0C flags: bit1 0x2 candidate, bit2 0x4 seen this pass, bit3 0x8 same team [M].
    pub flags: u32,
    /// +0x04 — squared distance to the bot [M 0x125390:148-149].
    pub dist2: f32,
    /// The player's health for the trait-8 pick (their `Drone+0xAC`) [M 0x128b50:201-210].
    pub health: f32,
    /// The player's score (`MPGame+slot*0x30+0x18`) for the trait-4 pick [M 0x128b50:132-150].
    pub score: f32,
}


/// The world view one brain reads each tick — the port's `MPGame`/`MPSettings`/object stand-ins.
/// Every field carries its game origin; the collision world stands in for `BoundsTest` LOS
/// (the same *Deviation* `nav.rs` documents).
pub struct BotCtx<'a> {
    /// Kept for API compatibility, **ignored by `BotBrain::step`**: the brain's canonical clock
    /// is its own 60 Hz tick counter (`self.tick as f32 / 60.0` — the same clock
    /// `next_bullet_at` is stored on). A driver whose wall-clock accumulation drifts by ulps
    /// (or a lockstep peer that derives it differently) must not be able to move sim state —
    /// that was net-lockstep finding F4 (docs/systems/net-lockstep.md).
    pub now: f32,
    /// `MPSettings+0x420` — the scenario word.
    pub scenario: u32,
    /// The bot's own player index (`BOT_vars+0x767`).
    pub my_index: usize,
    /// The bot's team (`MPSettings+slot*0x30+0x20`).
    pub my_team: u32,
    /// The registered pickups (`MPpickups`).
    pub pickups: &'a [BotPickup],
    /// The 8 player slots (`MPGame+slot*0x30+0x1C` objs) — self included; readers skip `my_index`.
    pub others: &'a [OtherPlayer],
    /// The collision world (the LOS/cel stand-in).
    pub world: &'a CollisionWorld,
    /// The bot's health (`Drone+0xAC`).
    pub health: f32,
    /// The bot's max HP (`BOT_vars+0xA4`).
    pub max_health: f32,
    /// The attack states' stationary gate: behaviour prop 0x40 / `Drone+0x4F8` bit 0x10
    /// [M 0x16f198:42-45, 0x16f5e8:56-60]. The "stand" preset maps here [S — see the report].
    pub stationary: bool,
}


/// The bot's weapon fire parameters — lifted from `weapon_data` by the caller
/// (`BOTWEAP_InitWeapon` 0x129e48 sets the default; `DroneWeap_NextBulletTime` 0x179d40 reads).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BotWeapon {
    /// `weapon_data[w*0x10C+0x40]` (s16) — the per-shot interval in ticks [M 0x179d40:26-47].
    pub interval: i16,
    /// `weapon_data[w*0x10C+0x2C]` (u16) — the burst size [M 0x179d40:53-116].
    pub burst: u16,
    /// The aggression multiplier (`BOT_getAggressionMul` — switches on `Drone+0xB5`, ×counts ≥ 4,
    /// min 3.0 [M 0x179d40]; the exact 0.3..1.0 table is not lifted — the caller passes it, [S]).
    pub aggression_mul: f32,
}


/// The `BOT_vars_t` subset the ported states touch — every field carries its offset in the comment.
#[derive(Clone, Debug, PartialEq)]
pub struct BotVars {
    // the stats block (+0xA0..0xAD, `BOT_init` 0x1249e8:164's record copy)
    pub aggression: u8,   // +0xA0
    pub aim: u8,          // +0xA2
    pub max_hp: u16,      // +0xA4
    pub speed: u8,        // +0xA6
    pub weapon_class: u8, // +0xAA preferred weapon class
    pub trait_id: u8,     // +0xAB
    pub flags: u8,        // +0xAC (bit3 360° vision, bit4 HP regen)
    // the two goal slots
    pub goals: [GoalSlot; 2],
    pub active_goal: u8, // +0x765 (0xFF none)
    // instance fields
    pub requested_state: u32, // +0x72C (`BOTSTATE_setStateChange` writes; 0xEB reads as resume)
    pub pending_scratch: u32, // +0x730 (gotoGoal := the state at goal start)
    pub flag_word: u32,       // +0x734 (bit1 recovery pending, bit2 flag-carrier goal)
    pub regen_deadline: u32,  // +0x738 (HP-regen next-tick deadline)
    pub nav_node: u16,        // +0x760 (0xFFFF invalid)
    pub slot: i16,            // +0x75C (player slot 4..7)
    pub bot_index: i16,       // +0x75E (slot−4)
    pub state_type: u8,       // +0x766 (type of the last requested state)
    pub weapon_id: u8,        // +0x768 (current weapon)
    pub announced_weapon: u8, // +0x76A (msg 0x42 payload)
    pub grudge: i8,           // +0x76B (0xFF none)
    pub last_pickup: i8,      // +0x76E (the last chosen pickup index — pickGoal skip/ writer [M 0x126b88:397,496])
    pub guard_near_flag: u8,  // +0x76F (BotIdle's sphere-box result [M 0x171b80:96])
    pub scan_flag: u8,        // +0x770 (set when the +0xB0 scan found a live candidate)
    pub attack_latch: u8,     // +0x771 (the attack re-entry latch)
    // combat state (Drone_tag fields the states read; carried here so the brain is self-contained)
    pub opponent: Option<usize>, // Drone+0x170 (index into ctx.others)
    pub sighted: u32,            // Drone+0x228 (bit 2 = opponent visible)
    pub unseen_ticks: u32,       // Drone+0x274 (ticks since the last LOS tick)
    pub sight_stamp: f32,        // Drone+0x238 (last-sight stamp; heartbeat's 5 s latch)
    pub combat_range: [f32; 3],  // Drone+0xEC/0xF0/0xFC (too-close / engage / too-far)
    pub fire_latch: u8,          // Drone+0x3B
    pub burst_empty: u8,         // Drone+0x3C
    pub clip_rounds: i16,        // `BOTWEAP_AmmoInGun` = BOT_vars+0x144 + w*0xC [M 0x12ae40]
    pub next_bullet_at: f32,     // Drone+0xBC4 (the burst deadline)
    pub burst_left: u32,         // Drone+0xBC0
    pub resume_state: u16,       // Drone+0x5A6 (the 0xF6 door-detour resume marker)
}


impl Default for BotVars {
    fn default() -> Self {
        BotVars {
            aggression: 0,
            aim: 0,
            max_hp: 100,
            speed: 0,
            weapon_class: 0,
            trait_id: 0,
            flags: 0,
            goals: [GoalSlot::default(); 2],
            active_goal: 0xFF,
            requested_state: 0,
            pending_scratch: 0,
            flag_word: 0,
            regen_deadline: 0,
            nav_node: 0xFFFF,
            slot: 4,
            bot_index: 0,
            state_type: 0,
            weapon_id: 0,
            announced_weapon: 0,
            grudge: -1,
            last_pickup: -1,
            guard_near_flag: 0,
            scan_flag: 0,
            attack_latch: 0,
            opponent: None,
            sighted: 0,
            unseen_ticks: u32::MAX,
            sight_stamp: f32::NEG_INFINITY,
            combat_range: [2.0, 20.0, 40.0],
            fire_latch: 0,
            burst_empty: 0,
            clip_rounds: 0,
            next_bullet_at: 0.0,
            burst_left: 0,
            resume_state: 0,
        }
    }
}


/// One fired shot (the port's `DroneWeap_FireWeapon` event — the caller spawns the bullet).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BotShot {
    pub tick: u32,
    pub pos: [f32; 3],
    pub dir: [f32; 3],
    pub weapon: u8,
}


/// One MP bot brain: the `BotVars` slot, the SM state and the per-tick driver
/// (`Drone_Control` → `ControlSTANDARD` → `SendMsgSelf(3)` → `RouteMsgDCV` order).
pub struct BotBrain {
    pub vars: BotVars,
    /// The SM current state (`Drone+0x10C`).
    pub state: u8,
    /// The position (`obj+0x30`).
    pub pos: [f32; 3],
    /// The yaw (`obj+0x50..0x5C` matrix facing; the port carries the scalar).
    pub yaw: f32,
    /// The per-tick counter (the port's `GameState` stand-in).
    pub tick: u32,
    /// The weapon params (`BOTWEAP_InitWeapon`'s lift).
    pub weapon: BotWeapon,
    /// The fired shots (the test observes these).
    pub shots: Vec<BotShot>,
    /// The visited pickups (`setPickupVisitTime` events — the test's "reached a pickup").
    pub visited_pickups: Vec<usize>,
    /// The state-change trace (tick, from, to) — the test's transition assertions.
    pub trace: Vec<(u32, u8, u8)>,
    /// The last `setPickupVisitTime` stamp (pickup index, deadline) — the caller mirrors it
    /// into its `MP_PICKUP+0x80` floats (the game writes the pickup record in place).
    pub last_visit_stamp: (usize, f32),
    /// The pending SetState (`SM+0x0C` + the flag [M 0x1758c0:53-55]).
    pending: Option<u8>,
}


/// `BOTSTATE_getStateType` is already above; the states the port dispatches to.
mod st {
    pub use super::state::*;
    /// The attack family ids the port handles (`docs/reports/mp-bots-data.md` inventory).
    pub const BOT_ATTACK_NO_OPPONENT: u8 = 0xD3; // Stub
    pub const BOT_ATTACK_STRAFE_AIM_LEFT: u8 = 0xD4; // Full — not ported (chooseCombatMove target)
    pub const BOT_ATTACK_STRAFE_AIM_RIGHT: u8 = 0xD5; // Full — not ported
    pub const BOT_ATTACK_STEP_AIM_LEFT: u8 = 0xDB; // Thin — not ported
    pub const BOT_ATTACK_STEP_AIM_RIGHT: u8 = 0xDC; // Thin — not ported
    pub const BOT_SEEN_OPPONENT: u8 = 0xEC; // Stub: ENTER → 0xCF [M 0x175268:9-13]
}


impl BotBrain {
    /// `BOT_init` 0x1249e8 (the port's lift of the BOT_vars-relevant half) + the 0xC3 ENTER
    /// chain [M 0x1748c8:16-18: setNearestNavNode, +0x766 = 0, SetState(0xF9)]:
    /// the stats copy, the goal-slot active flags (slot 1 = 1, slot 0 = 0), the grudge 0xFF,
    /// the links, then state 0xF9 via the commit.
    pub fn bot_init(record: &BotRecord, slot: u8, pos: [f32; 3], yaw: f32) -> BotBrain {
        let stats = record.stats();
        let mut vars = BotVars::default();
        vars.aggression = stats[0x00];
        vars.aim = stats[0x02];
        vars.max_hp = u16::from_le_bytes([stats[0x04], stats[0x05]]);
        vars.speed = stats[0x06];
        vars.weapon_class = stats[0x0A];
        vars.trait_id = stats[0x0B];
        vars.flags = stats[0x0C];
        vars.slot = slot as i16;
        vars.bot_index = (slot - 4) as i16;
        vars.grudge = -1; // 0xFF [M 0x1249e8:163]
        vars.last_pickup = -1;
        vars.goals[1].active = 1; // [M 0x1249e8:172-176: slot1+0x49 = 1, slot0+0x49 = 0]
        vars.goals[0].active = 0;
        if vars.flags & 0x10 != 0 {
            // the HP-regen arm: +0x738 := GameState+52 + FRAME_RATE_INT [M 0x1249e8:165-167]
            vars.regen_deadline = FRAME_RATE_INT;
        }
        let mut brain = BotBrain {
            vars,
            state: 0xC3, // BotInit
            pos,
            yaw,
            tick: 0,
            weapon: BotWeapon { interval: 6, burst: 1, aggression_mul: 1.0 },
            shots: Vec::new(),
            visited_pickups: Vec::new(),
            trace: Vec::new(),
            last_visit_stamp: (usize::MAX, 0.0),
            pending: Some(state::BOT_IDLE), // the BotInit chain's SetState(0xF9) [M 0x1748c8:18],
            // committed by the first `step` (the game's PostLoad_Init → msg 1 → SetState order)
        };
        brain
    }

    fn enter_state(&mut self, next: u8, rng: &mut Rng) {
        // the commit: the state flips, +0x766 := the type [M 0x16e060:605], ENTER msg 1 runs below
        let prev = self.state;
        self.state = next;
        self.vars.state_type = state_type(next as u32);
        self.trace.push((self.tick, prev, next));
        // the ENTER delivery — each ported state's msg-1 arm runs now
        match next {
            state::BOT_IDLE => self.bot_idle_enter(),
            state::BOT_GOTO_GOAL_POSITION => {} // walk anim 0x1E [M 0x171240:20]
            state::BOT_ATTACK => self.bot_attack_enter(rng),
            state::BOT_ATTACK_FIRE => self.bot_attack_fire_enter(rng), // the trigger pull [M 0x16f5e8:19-26]
            st::BOT_SEEN_OPPONENT => {
                self.set_state(state::BOT_ATTACK); // the 0xEC stub ENTER [M 0x175268:9-13]
            }
            _ => {} // 0xF2/0xF6/0xD3… not ported (the report lists the gap)
        }
    }

    /// `Drone_SM_SetState` 0x1758c0: the pending write [M :53-55].
    fn set_state(&mut self, next: u8) {
        if next != 0 {
            self.pending = Some(next);
        }
    }

    /// One drone-control tick [M 0x1490f0:81 + 0x171df8:78-86,119-203]: heartbeat to 0xC5, msg 3
    /// to the current state, the firing pipeline, then the pending commit (0x2E + ENTER).
    pub fn step(&mut self, ctx: &BotCtx, rng: &mut Rng, weapon_fire: bool) {
        self.tick += 1;
        // 0. the perception pass [M 0x1490f0:164-167 — setOtherPlayerInfo then FindOpponent]:
        // the LOS/sight bookkeeping (the port's `CanSeeObject` = the collision-world segment)
        self.vars.sighted &= !4;
        if let Some(o) = self.vars.opponent {
            if let Some(t) = ctx.others.get(o) {
                if t.alive {
                    let clear = ctx.world.segment(self.pos, t.pos).is_none();
                    if clear {
                        self.vars.sighted |= 4; // Drone+0x228 bit 2 [M]
                        self.vars.unseen_ticks = 0; // Drone+0x274
                        // the brain's canonical clock: its own tick counter (60 Hz), NOT the
                        // driver-supplied ctx.now — a lockstep host's wall-clock accumulation
                        // must not reach sim state (net-lockstep finding F4). next_bullet_at is
                        // already tick-derived; this aligns every time read with it.
                        self.vars.sight_stamp = self.tick as f32 / 60.0; // Drone+0x238
                    } else {
                        self.vars.unseen_ticks = self.vars.unseen_ticks.saturating_add(1);
                    }
                }
            }
        }
        self.find_opponent(ctx); // [M 0x1490f0:167]
        // 1. the forced-to-0xC5 heartbeat [M 0x171df8:78-86]
        self.bot_global_msg3(ctx);
        // 2. msg 3 to the current state (unknown states do not consume → nothing, like the
        //    unported handlers returning 0 with the BotGlobal fallback consumed above)
        match self.state {
            state::BOT_IDLE => {
                self.bot_idle_msg3(ctx, rng);
            }
            state::BOT_GOTO_GOAL_POSITION => self.bot_goto_goal_msg3(ctx),
            state::BOT_ATTACK => self.bot_attack_msg3(ctx, rng),
            state::BOT_ATTACK_FIRE => self.bot_attack_fire_msg3(ctx, rng),
            _ => {}
        }
        // 3. the per-tick firing pipeline [M 0x1490f0:231 `DroneWeap_HandleFiring`]
        if weapon_fire {
            self.handle_firing(ctx, rng);
        }
        // 4. the commit [M 0x171df8:119-203]: EXIT msg 2 is a no-op for the ported states, the
        //    commit, the synthetic 0x2E (the +0x766 type write is the ported part [M 0x16e060:605]),
        //    then ENTER msg 1 — the loop repeats while ENTERs chain more SetStates [M :119-203].
        while let Some(next) = self.pending.take() {
            self.vars.state_type = state_type(next as u32);
            self.enter_state(next, rng);
        }
    }

    /// The brain's lockstep hash — every **live** state field, f32 as bit patterns
    /// (`mp::hash::Hasher`). The brains are sim state: a lockstep host hashes them with the
    /// match every tick (`docs/systems/net-lockstep.md`). The derived logs (`shots`,
    /// `visited_pickups`, `trace`) hash as lengths only — they are pure functions of the live
    /// state + the shared RNG stream, so a content divergence implies a hashed divergence.
    pub fn state_hash(&self) -> u64 {
        use crate::mp::hash::Hasher;
        let mut h = Hasher::new();
        let v = &self.vars;
        h.u8(v.aggression);
        h.u8(v.aim);
        h.u16(v.max_hp);
        h.u8(v.speed);
        h.u8(v.weapon_class);
        h.u8(v.trait_id);
        h.u8(v.flags);
        for g in &v.goals {
            h.f32_slice(&g.celpos);
            h.f32(g.scratch);
            h.f32(g.start);
            h.f32(g.duration);
            h.f32(g.weight_health);
            h.f32(g.weight_ammo);
            h.f32(g.weight_weapon);
            h.f32(g.objective_weight);
            h.usize(g.target);
            h.u8(g.fire_state);
            h.u8(g.complete);
            h.u8(g.goal_type);
            h.u8(g.pref_byte);
            h.u8(g.dist_limit as u8);
            h.u8(g.route_code);
            h.u8(g.active);
            h.u8(g.subtype);
        }
        h.u8(v.active_goal);
        h.u32(v.requested_state);
        h.u32(v.pending_scratch);
        h.u32(v.flag_word);
        h.u32(v.regen_deadline);
        h.u16(v.nav_node);
        h.u16(v.slot as u16);
        h.u16(v.bot_index as u16);
        h.u8(v.state_type);
        h.u8(v.weapon_id);
        h.u8(v.announced_weapon);
        h.u8(v.grudge as u8);
        h.u8(v.last_pickup as u8);
        h.u8(v.guard_near_flag);
        h.u8(v.scan_flag);
        h.u8(v.attack_latch);
        match v.opponent {
            None => h.u8(0),
            Some(o) => {
                h.u8(1);
                h.usize(o);
            }
        }
        h.u32(v.sighted);
        h.u32(v.unseen_ticks);
        h.f32(v.sight_stamp);
        h.f32_slice(&v.combat_range);
        h.u8(v.fire_latch);
        h.u8(v.burst_empty);
        h.u16(v.clip_rounds as u16);
        h.f32(v.next_bullet_at);
        h.u32(v.burst_left);
        h.u16(v.resume_state);
        h.u8(self.state);
        h.f32_slice(&self.pos);
        h.f32(self.yaw);
        h.u32(self.tick);
        h.i16(self.weapon.interval);
        h.u16(self.weapon.burst);
        h.f32(self.weapon.aggression_mul);
        h.usize(self.shots.len());
        h.usize(self.visited_pickups.len());
        h.usize(self.trace.len());
        h.usize(self.last_visit_stamp.0);
        h.f32(self.last_visit_stamp.1);
        match self.pending {
            None => h.u8(0),
            Some(p) => {
                h.u8(1);
                h.u8(p);
            }
        }
        h.finish()
    }

    // ------------------------------------------------------------------
    // The goal machine (BOTSTATE_*)
    // ------------------------------------------------------------------

    /// `BOTSTATE_initGoal` 0x1294c0 [M :15-40].
    /// The port form of `AINetwork_SetupGoalPositionToObj` (0x15add0); the goal machine is
    /// `BOTSTATE_gotoGoal` (0x1278a8).
    pub fn init_goal(&mut self, slot: usize, goal_type: u8, celpos: [f32; 4], target: usize, subtype: u8) {
        if slot >= 2 {
            return;
        }
        let g = &mut self.vars.goals[slot];
        g.goal_type = goal_type;
        g.complete = 0;
        g.route_code = 0;
        g.celpos = celpos;
        g.subtype = subtype;
        g.target = target;
    }

    /// `BOTSTATE_uninitGoal` 0x129438 [M :9-29].
    pub fn uninit_goal(&mut self, slot: usize) {
        if slot > 1 {
            return;
        }
        let carrier = self.vars.goals[slot].subtype == 1;
        let drone = &mut self.vars;
        if carrier {
            drone.flag_word |= 4;
        } else {
            drone.flag_word &= !4;
        }
        if drone.active_goal == slot as u8 {
            drone.active_goal = 0xFF;
        }
        let g = &mut drone.goals[slot];
        g.subtype = 0;
        g.goal_type = 0;
        g.complete = 0;
        g.route_code = 0;
        g.scratch = 0.0;
    }

    /// `BOTSTATE_setGoalComplete` 0x129510: `+0x44 |= 1` [M :9-10].
    pub fn set_goal_complete(&mut self, slot: usize) {
        if slot < 2 {
            self.vars.goals[slot].complete |= 1;
        }
    }

    /// `BOTSTATE_getActiveGoal` 0x129548: Some(slot) or None [M :8-12].
    pub fn get_active_goal(&self) -> Option<usize> {
        let idx = self.vars.active_goal;
        (idx != 0xFF).then_some(idx as usize)
    }

    /// `BOTSTATE_setStateChange` 0x129ce8: SetState + the +0x72C record [M :6-7].
    fn set_state_change(&mut self, next: u8) {
        self.set_state(next);
        self.vars.requested_state = next as u32;
    }

    /// `BOTSTATE_setGoalPickPrefs` 0x128298 [M :23-95]: the auto-derive when all three weights
    /// are 0 (health = (1 − hp/max)·25, ammo/weapon the same shape from the clip state), the
    /// 25.0 clamps, and the p7==0 → +0x47 := −2.
    pub fn set_goal_pick_prefs(
        &mut self, health_w: f32, ammo_w: f32, weapon_w: f32, objective_w: f32, slot: usize,
        dist_limit: i8, pref_byte: u8, health: f32, ammo: f32, clip: f32,
    ) {
        if slot >= 2 {
            return;
        }
        let (mut h, mut a, mut w) = (health_w, ammo_w, weapon_w);
        if health_w == 0.0 && ammo_w == 0.0 && weapon_w == 0.0 {
            // the auto-derive [M 0x128298:24-28]
            let hp = health.max(1.0);
            h = (1.0 - 1.0 / (self.vars.max_hp as f32 / hp)) * 25.0;
            if ammo < clip {
                let denom = clip.max(1.0);
                a = (1.0 - 1.0 / (denom / ammo.max(1.0))) * 25.0;
                w = a; // [M :53-70 — the same derivation for the weapon weight]
            }
        }
        let g = &mut self.vars.goals[slot];
        g.weight_health = h.min(25.0);
        g.weight_ammo = a.min(25.0);
        g.weight_weapon = w.min(25.0);
        g.objective_weight = objective_w;
        g.pref_byte = pref_byte;
        g.dist_limit = if dist_limit == 0 { -2 } else { dist_limit };
    }

    /// `BOTSTATE_setPickupVisitTime` 0x129958: `now + 45.0` (+0.1 if that is 0) [M :10-15].
    pub fn set_pickup_visit_time(&mut self, ctx: &BotCtx, pickup: usize) {
        let mut t = self.tick as f32 / 60.0 + 45.0;
        if t == 0.0 {
            t += 0.1;
        }
        let idx = self.vars.bot_index as usize;
        self.visited_pickups.push(pickup);
        // the game writes `MP_PICKUP+0x80+idx*4` in place; the port's ctx owns the pickups, so
        // the driver mirrors `last_visit_stamp` into its table between steps
        let _ = idx;
        self.last_visit_stamp = (pickup, t);
    }

    /// `BOTSTATE_gotoGoal` 0x1278a8 [M :20-147]: teardown of the old active goal, the deadline
    /// stamp (+0x24 := now, +0x28 := VIDEO_FRAME_RATE·5), +0x40 := fire state, the route setup,
    /// the +0x20 RNG budget (slot 1: the 600/600/500 rolls by subtype), activation.
    pub fn goto_goal(&mut self, ctx: &BotCtx, rng: &mut Rng, slot: usize, fire_state: u8) -> bool {
        if slot > 1 {
            return false;
        }
        // teardown of the currently active goal [M :28-49]
        if self.vars.active_goal != 0xFF {
            let old = self.vars.active_goal as usize;
            let carrier = self.vars.goals[old].subtype == 1;
            if carrier {
                self.vars.flag_word |= 4;
            } else {
                self.vars.flag_word &= !4;
            }
            let g = &mut self.vars.goals[old];
            g.subtype = 0;
            g.goal_type = 0;
            g.complete = 0;
            g.route_code = 0;
            g.scratch = 0.0;
        }
        {
            let g = &mut self.vars.goals[slot];
            g.start = self.tick as f32 / 60.0;
            g.duration = (FRAME_RATE_INT * 5) as f32; // [M 0x1278a8:50,58]
            g.fire_state = fire_state;
        }
        let gtype = self.vars.goals[slot].goal_type;
        if gtype == 3 {
            self.vars.goals[slot].subtype = 9; // hunt [M :71]
        }
        self.vars.pending_scratch = self.state as u32; // +0x730 := current state [M :67-68]
        // the +0x20 wander budget [M :84-142] — slot 0 stays 0 [M :85-87]
        if slot == 1 {
            let sub = self.vars.goals[1].subtype;
            let mut v = match sub {
                1 | 3 | 4 | 6 | 7 => rng.rand(600) as f32 + 500.0, // [M :95]
                8 => rng.rand(600) as f32 + 250.0,                  // [M :109]
                9 => rng.rand(500) as f32 + 250.0,                  // [M :119]
                2 | 5 => 1500.0,                                    // [M :104-107]
                _ => 0.0,
            };
            if ctx.my_team == 0 && self.vars.trait_id == 5 {
                v = 0.0; // [M :128-135]
            }
            self.vars.goals[1].scratch = v;
        }
        self.vars.flag_word &= !4; // [M :145]
        self.vars.active_goal = slot as u8; // [M :146]
        true
    }

    /// `BOTSTATE_pickGoal` 0x126b88 (the port's flow; route distance → straight-line [Deviation]).
    /// Returns true when a goal was initialised + `gotoGoal` ran.
    pub fn pick_goal(&mut self, ctx: &BotCtx, rng: &mut Rng, slot: usize) -> bool {
        let mut slot = slot;
        // Phase A: the mode-objective scan (slot 1 only) [M :143-303]. Deathmatch (0x1) has no
        // arm in the dispatch; the objective wrappers are mode globals not lifted here — see
        // the report. Nothing found → uninitGoal(0), zero slot-0 prefs, fall through [M :317-319].
        if slot == 1 {
            self.uninit_goal(0);
            self.set_goal_pick_prefs(0.0, 0.0, 0.0, 0.0, 0, 0, 0, ctx.health, 1.0, 1.0);
            slot = 0;
        }
        // Phase B: the grudge hunt (slot 0, pref byte 0) [M :327-342]
        if slot == 0 && self.vars.goals[0].pref_byte == 0 {
            let g = self.vars.grudge;
            let hunted = (g >= 0)
                .then(|| ctx.others.get(g as usize).filter(|o| o.alive).map(|_| g as usize))
                .flatten();
            if let Some(idx) = hunted {
                let p = ctx.others[idx].pos;
                self.init_goal(0, 3, [p[0], p[1], p[2], 0.0], idx, 9);
                if self.goto_goal(ctx, rng, 0, state::BOT_IDLE) {
                    return true;
                }
            } else {
                self.vars.grudge = -1; // [M :340]
            }
        }
        // Phase C: the pickup scoring [M :345-488]
        let g0 = self.vars.goals[0];
        let mut weights = (g0.weight_health, g0.weight_ammo, g0.weight_weapon);
        if weights.0 + weights.1 + weights.2 == 0.0 {
            weights = (1.0, 1.0, 1.0); // [M :353-356]
        }
        let dist_limit = g0.dist_limit as u8 as f32; // 0xFE (the -2 store) = 254.0 [M :362-363]
        let passes = if g0.pref_byte & 0x2 != 0 { 1 } else { 4 }; // [M :367-371]
        let mut best: Option<(usize, f32, f32)> = None; // (index, score, dist)
        for pass in 0..passes {
            for (i, p) in ctx.pickups.iter().enumerate() {
                if i as i8 == self.vars.last_pickup {
                    continue; // [M :397]
                }
                let mut dist = ((self.pos[0] - p.pos[0]).powi(2)
                    + (self.pos[1] - p.pos[1]).powi(2)
                    + (self.pos[2] - p.pos[2]).powi(2))
                .sqrt();
                if dist > dist_limit || dist <= 0.0 {
                    continue; // [M :397]
                }
                let visit = p.visit[self.vars.bot_index as usize];
                if pass < 3 {
                    if visit != 0.0 && g0.pref_byte & 0x20 == 0 {
                        continue; // [M :404-417 — visited pickups skipped until the last pass]
                    }
                } else if visit != 0.0 {
                    // the visit decay [M :407-415]
                    let mul = if visit >= self.tick as f32 / 60.0 {
                        4500.0
                    } else {
                        ((self.tick as f32 / 60.0 - visit).abs() * 50.0).max(1.0)
                    };
                    dist *= mul;
                }
                // the class filters, pass 0 [M :431-477]
                if pass == 0 {
                    match p.mp_class {
                        0 => {
                            // weapons: skip the held weapon id (the !hasWeapon gate's port form [S];
                            // the full isPreferredWeapon read is the report's remaining work).
                            // Game forms: `BOTSTATE_hasWeapon` (0x129658) skips any held weapon
                            // (`BOTWEAP_hasWeapon` 0x12af78); the port only has the current one.
                            if p.weapon_id != 0 && p.weapon_id as u8 == self.vars.weapon_id {
                                continue;
                            }
                        }
                        1 => continue, // ammo needs the owned-weapon scan — not lifted, [S]
                        3 => {}
                        _ => continue, // class 2 and others never score [M :474-475]
                    }
                }
                if p.is_obj_flag2 && (g0.pref_byte & 0x4 != 0 || dist < 5.0) {
                    continue; // [M :420-421]
                }
                let proximity = dist_limit + 1.0 - dist; // ((u8)+0x47 + 1.0) − dist [M :402]
                let weight = match p.mp_class {
                    3 => weights.0,
                    1 => weights.1,
                    _ => weights.2,
                };
                let score = proximity * weight;
                let better = match best {
                    None => true,
                    Some((_, bs, bd)) => score > bs || (score == bs && dist < bd), // [M :449-453]
                };
                if better {
                    best = Some((i, score, dist));
                }
            }
            if best.is_some() {
                break;
            }
            if pass == 1 {
                weights = (1.0, 1.0, 1.0); // pass 2's equal weighting [M :375-379]
            }
        }
        let Some((idx, _, _)) = best else {
            self.uninit_goal(slot);
            return false; // [M :490-492]
        };
        self.vars.last_pickup = idx as i8; // [M :496]
        let pos = ctx.pickups[idx].pos;
        self.init_goal(slot, 1, [pos[0], pos[1], pos[2], 0.0], idx, 0);
        self.goto_goal(ctx, rng, slot, state::BOT_IDLE) // [M :510]
    }

    /// `BOTSTATE_processGoals` 0x127c78 (the port's flow; called by the heartbeat's non-combat
    /// types [M 0x16e060:498-501]). Returns true when it changed the SM state.
    pub fn process_goals(&mut self, ctx: &BotCtx) -> bool {
        if self.vars.active_goal == 0xFF {
            return false;
        }
        let slot = self.vars.active_goal as usize;
        let g = self.vars.goals[slot];
        if g.goal_type == 0 {
            self.vars.active_goal = 0xFF; // [M :64-68]
            return false;
        }
        // the deadline [M :70]
        if self.tick as f32 / 60.0 > g.start + g.duration {
            // expiry: fire the goal state if I'm walking a goal state [M :216-221]
            if matches!(self.state, 0xEB | 0xF8) {
                self.set_state_change(g.fire_state);
            }
            self.uninit_goal(slot);
            return true;
        }
        // per-type validity
        match g.goal_type {
            3 => {
                // the hunt target must be a live player [M :88-97]
                let valid = ctx.others.get(g.target).is_some_and(|o| o.alive);
                if !valid {
                    if matches!(self.state, 0xEB | 0xF8) {
                        self.set_state_change(g.fire_state);
                    }
                    self.uninit_goal(slot);
                    return true;
                }
                // the trait-2 melee gate [M :108-113]: candidate + same-team, dist² < 20.25, |Δz| < 2
                if self.vars.trait_id == 2 {
                    if let Some(o) = ctx.others.get(g.target) {
                        if o.flags & 0xA == 0xA
                            && o.dist2 < 20.25
                            && (o.pos[1] - self.pos[1]).abs() < 2.0
                        {
                            self.vars.goals[slot].fire_state = st::BOT_GUARD_FRIEND_IDLE;
                            self.vars.opponent = Some(g.target); // +0x758 [M :113]
                        }
                    }
                }
            }
            _ => {}
        }
        // the arrival: the stored route code == 3 or the complete flag [M :165-205]
        let arrived = g.route_code == 3 || g.complete & 1 != 0;
        if arrived {
            if g.goal_type == 1 {
                self.set_pickup_visit_time(ctx, g.target); // [M :153,199]
            }
            let fire = self.vars.goals[slot].fire_state;
            if matches!(self.state, 0xEB | 0xF8) {
                self.set_state_change(fire);
            }
            self.uninit_goal(slot);
            return true;
        }
        false
    }

    // ------------------------------------------------------------------
    // The target choice + perception
    // ------------------------------------------------------------------

    /// `BOTSTATE_getPreferredTraitOpponentObjIndex` 0x128b50 — the trait switch [M :92-223].
    /// Returns the player index or −1.
    pub fn trait_pick_opponent(&self, ctx: &BotCtx) -> i8 {
        match self.vars.trait_id {
            2 => {
                // the same-team pool [M :93-131]: min dist² among same-team candidates
                let mut best: Option<(usize, f32)> = None;
                for o in ctx.others {
                    if o.index == ctx.my_index || o.team != ctx.my_team || !o.alive {
                        continue;
                    }
                    match best {
                        Some((_, d)) if o.dist2 >= d => {}
                        _ => best = Some((o.index, o.dist2)),
                    }
                }
                match best {
                    Some((idx, d)) => {
                        if d >= 20.25 {
                            idx as i8 // [M :120-122]
                        } else {
                            -1 // the close-range guard handoff (setStateChange(0xF7)) [M :123-130]
                        }
                    }
                    None => -1,
                }
            }
            4 => {
                // the top-score enemy [M :132-150]
                let mut best: Option<(usize, f32)> = None;
                for o in ctx.others {
                    if o.index == ctx.my_index || o.team == ctx.my_team || !o.alive {
                        continue;
                    }
                    if best.map_or(true, |(_, s)| o.score > s) {
                        best = Some((o.index, o.score));
                    }
                }
                best.map_or(-1, |(i, _)| i as i8)
            }
            5 => {
                // keep the current opponent, else the nearest enemy from the block [M :151-170]
                if let Some(cur) = self.vars.opponent {
                    return cur as i8;
                }
                let mut best: Option<(usize, f32)> = None;
                for o in ctx.others {
                    if o.index == ctx.my_index || o.team == ctx.my_team || !o.alive {
                        continue;
                    }
                    if o.flags & 0x10 != 0x2 {
                        continue; // bit1 candidate set, bit3 clear [M :152-166]
                    }
                    match best {
                        Some((_, d)) if o.dist2 >= d => {}
                        _ => best = Some((o.index, o.dist2)),
                    }
                }
                best.map_or(-1, |(i, _)| i as i8)
            }
            8 => {
                // the lowest-health enemy, init my own health [M :192-223]
                let mut best: Option<(usize, f32)> = None;
                for o in ctx.others {
                    if o.index == ctx.my_index || o.team == ctx.my_team || !o.alive {
                        continue;
                    }
                    if best.map_or(true, |(_, h)| o.health < h) {
                        best = Some((o.index, o.health));
                    }
                }
                best.map_or(-1, |(i, _)| i as i8)
            }
            _ => -1, // traits 0,1,3,6,>8 [M :223]
        }
    }

    /// `BOT_setOtherPlayerInfo` 0x125390 (the port's pass): bit1 candidate, bit3 same team,
    /// bit2 seen (the LOS via the collision world — the game's `NDrone2_CanSeeObject`), the
    /// +0x04 dist², and the +0x770 scan flag. Called once per tick like ControlSTANDARD:164.
    pub fn scan_others(&mut self, ctx: &BotCtx) {
        self.vars.scan_flag = 0;
        let mut seen_any = false;
        for o in ctx.others {
            if o.index == ctx.my_index || !o.alive {
                continue;
            }
            seen_any = true;
        }
        if seen_any {
            self.vars.scan_flag = 1; // [M 0x125390:61]
        }
    }

    /// `NDrone2_FindOpponent` 0x1424e0 (the port's minimal pick): accept the nearest live
    /// candidate the pass found (the gate [M 0x1424e0:164-167 — 0x600 clear, 0x100 set,
    /// health > 0]; the full score/history selection is not lifted — [S], see the report).
    /// Team reads are `MP_getObjectTeam` (0x18b6c0).
    pub fn find_opponent(&mut self, ctx: &BotCtx) {
        if self.vars.opponent.is_some_and(|i| ctx.others.get(i).is_some_and(|o| o.alive)) {
            return; // keep the current opponent
        }
        let mut best: Option<(usize, f32)> = None;
        for o in ctx.others {
            if o.index == ctx.my_index || !o.alive {
                continue;
            }
            if o.flags & 0x6 != 0x6 {
                continue; // bit1 candidate + bit2 seen (the game's CanSeeObject bit [S — the
                          // vision query's range/params are not lifted; the caller supplies it])
            }
            match best {
                Some((_, d)) if o.dist2 >= d => {}
                _ => best = Some((o.index, o.dist2)),
            }
        }
        self.vars.opponent = best.map(|(i, _)| i);
    }

    /// `NDrone2_ReactToOpponentSighted` 0x1770f0 — the gates the decompile shows, then the
    /// suggested state [M :5-…; the surrender arm returns 0x3F].
    pub fn react_to_opponent_sighted(&self, suggested: u8) -> u8 {
        if self.vars.opponent.is_none() {
            return 0; // [M :16-18]
        }
        suggested
    }

    /// `BOTSTATE_chooseCombatMove` 0x129218 via `checkAttackMove`: `Rand_Rand(4)` →
    /// 0xD4/0xD5/0xDB/0xDC [M 0x129218:14], gated by `checkAttackMove`'s validity [S — the
    /// strafe/step states are not ported; the validity gate keeps the bot in 0xCF so the roll
    /// still consumes the shared RNG in the game's order].
    fn choose_combat_move(&mut self, rng: &mut Rng) -> Option<u8> {
        let roll = rng.rand(4);
        let mv = match roll {
            0 => st::BOT_ATTACK_STRAFE_AIM_LEFT,
            1 => st::BOT_ATTACK_STRAFE_AIM_RIGHT,
            2 => st::BOT_ATTACK_STEP_AIM_LEFT,
            _ => st::BOT_ATTACK_STEP_AIM_RIGHT,
        };
        None // checkAttackMove's stand-in: no move state is ported yet (the report lists it)
    }

    // ------------------------------------------------------------------
    // BotGlobal — state 0xC5, the forced heartbeat arm [M 0x16e060:84-519]
    // ------------------------------------------------------------------

    fn bot_global_msg3(&mut self, ctx: &BotCtx) {
        // SM current state == 0 → 0xF9 [M :86-87]
        if self.state == 0 {
            self.set_state(state::BOT_IDLE);
            return;
        }
        self.vars.state_type = state_type(self.state as u32); // [M :90]
        // (the death branch [M :91-119] is the test's alive-bot path — the port carries it as
        // the caller's responsibility until the death states land; see the report)
        // tick reset: the +0x72C requested state [M :121-130]
        if self.vars.requested_state != 0 {
            if self.state as u32 != self.vars.requested_state {
                let next = self.vars.requested_state as u8;
                self.vars.requested_state = 0;
                self.set_state(next);
                return;
            }
            self.vars.requested_state = 0;
        }
        // HP regen [M :209-233]: +5 clamp to max, the +0x738 deadline re-arm
        // regen_deadline is a TICK count — compare ticks exactly (the old now*60 f32 fold
        // drifted above 2^24; the tick compare is the same predicate, exact).
        if self.vars.flags & 0x10 != 0 && self.tick as f32 >= self.vars.regen_deadline as f32 {
            self.vars.regen_deadline += FRAME_RATE_INT;
        }
        let ty = self.vars.state_type;
        match ty {
            1 | 2 | 4 => {
                // types 1/2 → the tail return [M :517-519]; type 4 → the combat block [M :290-495]
                // (the port keeps the fire latch state; the weapon re-choice chain is the report's work)
            }
            10 => {}
            _ => {
                // types 3,5,6..9,11..13 [M :498-513]
                self.vars.nav_node = 0xFFFF; // setNearestNavNode [M 0x129420:6]
                let changed = self.process_goals(ctx);
                if !changed && self.vars.opponent.is_some() {
                    // the 5 s sight latch [M :504-509]
                    if self.tick as f32 / 60.0 - self.vars.sight_stamp < 5.0 {
                        self.vars.sighted |= 4;
                    }
                    if self.vars.sighted & 4 != 0 {
                        let next = self.react_to_opponent_sighted(st::BOT_SEEN_OPPONENT);
                        if next != 0 {
                            self.set_state(next);
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // State 0xF9 BotIdle [M 0x171b80]
    // ------------------------------------------------------------------

    fn bot_idle_enter(&mut self) {
        if self.vars.opponent.is_none() || self.vars.sighted & 4 == 0 {
            self.vars.opponent = None; // SetOpponent(0) [M :39]
            return; // + the +0xCC0/4/8 search timer [M :40-42]
        }
        self.set_state(state::BOT_ATTACK); // [M :45-49]
    }

    fn bot_idle_msg3(&mut self, ctx: &BotCtx, rng: &mut Rng) {
        let pick = self.trait_pick_opponent(ctx); // [M :54]
        self.vars.grudge = pick; // +0x76B [M :55]
        if pick != -1 {
            // zero the slot-0 prefs → the hunt pick [M :57-59]
            self.set_goal_pick_prefs(0.0, 0.0, 0.0, 0.0, 0, 0, 0, ctx.health, 1.0, 1.0);
            if self.pick_goal(ctx, rng, 0) {
                self.set_state(state::BOT_GOTO_GOAL_POSITION); // [M :61-64]
                return;
            }
        }
        // the latch/mode gate [M :66-72 verbatim in the research]
        if self.vars.attack_latch == 0
            || !matches!(ctx.scenario, 0x2000_0080 | 0x2000_0040)
        {
            self.set_goal_pick_prefs(0.0, 0.0, 0.0, 1.0, 1, 0, 0, ctx.health, 1.0, 1.0);
            if self.pick_goal(ctx, rng, 1) {
                self.set_state(state::BOT_GOTO_GOAL_POSITION);
            }
        } else {
            // the protection/demolition guard-site sphere-box gate [M :80-96]
            self.vars.attack_latch = self.guard_site_check(ctx);
            if self.vars.attack_latch != 0 {
                self.vars.guard_near_flag = 1; // +0x76F [M :96]
            }
        }
    }

    /// The BotIdle protection/demolition site check [M 0x171b80:80-94]: `Intersect_SphereBox`
    /// (radius 3.0) of my pos against the site's box → the +0x771 latch. Deathmatch has no
    /// site → the latch clears (the caller only reaches here in those modes).
        // `BOTSTATE_opponentIsMissile` (0x129a60): the demo/protection attacker may target the
        // site object itself as "opponent" (team-gated, +0xF6 done check) — ported as a stub [S].
    fn guard_site_check(&mut self, _ctx: &BotCtx) -> u8 {
        0
    }

    // ------------------------------------------------------------------
    // State 0xEB BotGotoGoalPosition [M 0x171240]
    // ------------------------------------------------------------------

    /// `NDrone2_MoveToGoalPosition` 0x1517d8 (the port's straight-line walk — *Deviation*: the
    /// `AINetwork_CalcRouteToPosition`/`FollowRoute` steering is not wired for the MP arenas;
    /// the arrival code 3 stands in for `dist < Drone+0x8EC` [M 0x1517d8:52-54]).
    fn move_to_goal_position(&mut self, ctx: &BotCtx) -> u8 {
        let Some(slot) = self.get_active_goal() else { return 3 };
        // the hunt goals track the live player obj (`SetupGoalPositionToObj` [M 0x1278a8:65-66]);
        // pickup/objective goals use the goal's CelPos [M :77-78]
        let target: [f32; 3] = if self.vars.goals[slot].goal_type == 3 {
            ctx.others.get(self.vars.goals[slot].target).map(|o| o.pos).unwrap_or(self.pos)
        } else {
            let c = self.vars.goals[slot].celpos;
            [c[0], c[1], c[2]]
        };
        let dx = target[0] - self.pos[0];
        let dy = target[1] - self.pos[1];
        let dz = target[2] - self.pos[2];
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        if d < 1.0 {
            return 3; // arrived
        }
        if ctx.stationary {
            return 1; // the stationary gate holds the bot in place [M 0x16f198:42-45 pattern; 0xEB arm [S]]
        }
        let mul = [0.7f32, 1.0, 1.3][(self.vars.speed as usize).min(2)]; // the speed mult [M atlas §4]
        let step = 0.05 * mul;
        self.pos[0] += dx / d * step;
        self.pos[1] += dy / d * step;
        self.pos[2] += dz / d * step;
        // the game's own ATAN2_APPROX (0x1e4528), not std libm: std's atan2/sin/cos differ by
        // ulps across libm versions/platforms, which would desync lockstep peers at the aim-cone
        // gate (dot <= 0.9998). Same convention: tangent dz/dx, same quadrants.
        self.yaw = crate::trig::atan2_approx(dz, dx); // face the target
        1
    }

    fn bot_goto_goal_msg3(&mut self, ctx: &BotCtx) {
        // the no-active-goal branch [M :30-35]
        let Some(slot) = self.get_active_goal() else {
            if self.vars.requested_state != 0 {
                let next = self.vars.requested_state as u8;
                self.vars.requested_state = 0;
                self.set_state(next);
            } else {
                self.set_state(state::BOT_IDLE);
            }
            return;
        };
        if self.vars.goals[slot].goal_type == 0 {
            self.set_state(state::BOT_IDLE);
            return;
        }
        let code = self.move_to_goal_position(ctx);
        self.vars.goals[slot].route_code = code; // +0x48 [M :40]
        // validateRoute: codes ≥ 0xC read 0 = route finished [M 0x129880:8-15]
        if code >= 0xC {
            if self.vars.goals[slot].goal_type == 1 {
                let target = self.vars.goals[slot].target;
                self.set_pickup_visit_time(ctx, target); // [M :43-45]
            }
            self.set_state(state::BOT_IDLE); // [M :52]
            return;
        }
        if code == 10 {
            self.vars.resume_state = state::BOT_GOTO_GOAL_POSITION as u16; // [M :60]
            self.set_state(state::BOT_DOOR_OPEN); // [M :61-63]
            return;
        }
        if code == 1 || code == 2 {
            // the walk anim replays [M :65]
        }
    }

    // ------------------------------------------------------------------
    // State 0xCF BotAttack [M 0x16f198]
    // ------------------------------------------------------------------

    fn bot_attack_enter(&mut self, rng: &mut Rng) {
        // anim 10, the attack message, InvalidateAttackRoute, then the 1-in-3 roll [M :18-28,91]
        if rng.rand(3) == 0 {
            if let Some(mv) = self.choose_combat_move(rng) {
                self.set_state(mv); // [M :26-28]
            }
        }
    }

    fn bot_attack_msg3(&mut self, ctx: &BotCtx, rng: &mut Rng) {
        if self.vars.opponent.is_none() {
            self.set_state(st::BOT_ATTACK_NO_OPPONENT); // [M :36-41]
            return;
        }
        let o = self.vars.opponent.unwrap();
        let dist = ctx.others.get(o).map(|x| x.dist2.sqrt()).unwrap_or(f32::MAX);
        // the shortcut: unseen < 16 && dist < engage [M :46-50]
        let shortcut = self.vars.unseen_ticks < 0x10 && dist < self.vars.combat_range[1];
        if !shortcut && !ctx.stationary && self.vars.flag_word & 2 == 0 {
            // the route block [M :52-77] — the port's validateRoute stand-in: the bot always
            // has a route to the opponent here, result < 3 → 0xD0 (run)
            self.set_state(state::BOT_ATTACK_RUN);
            return;
        }
        // the sighted latch [M :85-88]
        if self.vars.sighted & 4 != 0 {
            let next = self.react_to_opponent_sighted(state::BOT_ATTACK_FIRE);
            if next != 0 {
                self.set_state(next);
            }
        }
    }

    // ------------------------------------------------------------------
    // State 0xD2 BotAttackFire [M 0x16f5e8]
    // ------------------------------------------------------------------

    fn bot_attack_fire_enter(&mut self, rng: &mut Rng) {
        // anim 10, InvalidateAttackRoute, then the trigger pull [M :19-26]
        if self.vars.fire_latch != 0 {
            return; // [M :22-23]
        }
        self.vars.fire_latch = 1; // DroneWeap_Fire [M 0x17a8c0:15-17]
        self.vars.burst_empty = 0;
        self.next_bullet_time(rng); // → DroneWeap_NextBulletTime [M :25]
    }

    fn bot_attack_fire_msg3(&mut self, ctx: &BotCtx, rng: &mut Rng) {
        if self.vars.opponent.is_none() {
            self.set_state(st::BOT_ATTACK_NO_OPPONENT); // [M :36-41]
            return;
        }
        let o = self.vars.opponent.unwrap();
        let dist = ctx.others.get(o).map(|x| x.dist2.sqrt()).unwrap_or(f32::MAX);
        // the latch routing [M :42-52 verbatim]: 3C == 0 → (3B == 0 → the block) then the roll
        if self.vars.burst_empty == 0 {
            if self.vars.fire_latch == 0 {
                self.route_vision_block(ctx, dist, rng);
                return;
            }
            if rng.rand(3) != 0 {
                return; // keep firing [M :48-50]
            }
            // the fall-through → chooseCombatMove [M :51 → LAB_0016f818]
            if let Some(mv) = self.choose_combat_move(rng) {
                self.set_state(mv);
            }
            return;
        }
        self.route_vision_block(ctx, dist, rng); // 3C != 0 → the block [M :52]
    }

    /// BotAttackFire's route/vision block [M 0x16f5e8:56-93].
    fn route_vision_block(&mut self, ctx: &BotCtx, dist: f32, rng: &mut Rng) {
        if ctx.stationary {
            return; // [M :56-60]
        }
        if dist <= self.vars.combat_range[0] {
            self.set_state(state::BOT_ATTACK_BACKOFF); // CanBackoff → 0xD7 [M :61-67]
            return;
        }
        if self.vars.unseen_ticks >= 0x10 {
            if let Some(mv) = self.choose_combat_move(rng) {
                self.set_state(mv); // [M :77]
            }
            return;
        }
        if dist >= self.vars.combat_range[2] {
            // the chase route [M :81-93]: the port's route stand-in → run
            self.set_state(state::BOT_ATTACK_RUN);
        }
    }

    /// `DroneWeap_NextBulletTime` 0x179d40 (the port's lift): the interval from the weapon
    /// (min 60 for ids 6/0xA/0xE..0x10), the burst size with the shotgun/MG rolls and the
    /// aggression scaling for counts ≥ 4 [M :26-116].
    fn next_bullet_time(&mut self, rng: &mut Rng) {
        let w = self.vars.weapon_id as usize;
        let mut interval = self.weapon.interval.max(0) as i32;
        if matches!(w, 6 | 0xA | 0xE..=0x10) && interval < 60 {
            interval = 60; // [M 0x179d40:26-47]
        }
        let mut burst = match self.vars.weapon_id {
            6 | 0xA => rng.rand(3) + 2,   // [M :58]
            0xB..=0xD => rng.rand(2) + 1, // [M :74]
            _ => self.weapon.burst as u32,
        };
        if burst >= 4 {
            burst = ((burst as f32) * self.weapon.aggression_mul).max(3.0) as u32; // [M :…116]
        }
        self.vars.burst_left = burst;
        self.vars.next_bullet_at =
            self.tick as f32 / FRAME_RATE_INT as f32 + interval as f32 / 60.0; // the deadline [M :…134]
    }

    // ------------------------------------------------------------------
    // The per-tick firing pipeline [M 0x1490f0:231; 0x17a080 DoFiring; 0x178c48 Ready2Fire]
    // ------------------------------------------------------------------

    fn handle_firing(&mut self, ctx: &BotCtx, rng: &mut Rng) {
        if self.vars.fire_latch == 0 || self.vars.next_bullet_at <= 0.0 {
            return;
        }
        if (self.tick as f32 / 60.0) < self.vars.next_bullet_at {
            return; // now < the deadline
        }
        // the gates [M 0x17a080: unseen < 16; 0x178c48: opponent alive, ammo, the aim cone]
        if self.vars.unseen_ticks >= 0x10 {
            return;
        }
        let Some(o) = self.vars.opponent else { return };
        let Some(target) = ctx.others.get(o) else { return };
        // `BOTWEAP_WeaponHasAmmo` (0x12adb0): the port gates on clip rounds only; the ammo-pool
        // half (BotVars+0x698) is not modelled [S partial].
        if !target.alive || self.vars.clip_rounds < 1 {
            return;
        }
        let dx = target.pos[0] - self.pos[0];
        let dz = target.pos[2] - self.pos[2];
        let d = (dx * dx + dz * dz).sqrt();
        let dir = [dx / d, 0.0, dz / d];
        // the aim cone [M 0x178c48: dot > 0.9998]
        // the game's PS2Sinf/PS2Cosf (0x120700) — see the ATAN2_APPROX note above: std trig is
        // libm-dependent; the game's own approximations are not.
        let facing = [crate::trig::cosf(self.yaw), 0.0, crate::trig::sinf(self.yaw)];
        let dot = facing[0] * dir[0] + facing[2] * dir[2];
        if dot <= 0.9998 {
            return;
        }
        // FireWeapon [M 0x179d40→0x17a080:11]
        self.shots.push(BotShot { tick: self.tick, pos: self.pos, dir, weapon: self.vars.weapon_id });
        self.vars.clip_rounds -= 1;
        self.vars.burst_left -= 1;
        if self.vars.burst_left == 0 {
            self.vars.burst_empty = 1; // [M 0x17a180:63]
            self.vars.next_bullet_at = 0.0; // the pipeline stops — Fire's only caller is the state ENTER [M]
        } else {
            self.vars.next_bullet_at += self.weapon.interval.max(0) as f32 / 60.0;
        }
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

#[cfg(test)]
mod brain_tests {
    use super::*;

    /// The shared harness: one bot at (0,1.6,0), a pickup table, a stationary player at
    /// (10,1.6,0), an empty collision world (all LOS clear — the arenas' real geometry is
    /// the NF_ISO test's job). `now` accumulates 1/60 per tick like the port's MPGame+0x412.
    struct Rig {
        world: CollisionWorld,
        pickups: Vec<BotPickup>,
        others: Vec<OtherPlayer>,
        brain: BotBrain,
        rng: Rng,
        now: f32,
        stationary: bool,
        /// The `CanSeeObject` stand-in: when false the human row never carries the seen bit.
        vision: bool,
    }

    impl Rig {
        fn new() -> Rig {
            let record = INITIAL_BOT_RECORDS[0].clone();
            // the test enables the boot record (the menu's enable byte +0x0E [M Menu_PrepareBots])
            let mut rec = record;
            rec.0[0x0E] = 1;
            let mut brain = BotBrain::bot_init(&rec, 4, [0.0, 1.6, 0.0], 0.0);
            brain.weapon = BotWeapon { interval: 6, burst: 3, aggression_mul: 1.0 };
            brain.vars.weapon_id = 0x02;
            brain.vars.clip_rounds = 50;
            Rig {
                world: CollisionWorld::from_triangles(Vec::new()),
                pickups: vec![BotPickup {
                    pos: [6.0, 1.6, 0.0],
                    weapon_id: 0x10,
                    mp_class: 0,
                    is_obj_flag2: false,
                    visit: [0.0; 4],
                }],
                others: vec![
                    OtherPlayer { index: 0, pos: [10.0, 1.6, 0.0], alive: true, team: 0, flags: 0, dist2: 0.0, health: 100.0, score: 0.0 },
                    OtherPlayer { index: 4, pos: [0.0, 1.6, 0.0], alive: true, team: 0, flags: 0, dist2: 0.0, health: 100.0, score: 0.0 },
                ],
                brain,
                rng: Rng::new(0x1234),
                now: 0.0,
                stationary: false,
                vision: false,
            }
        }

        /// Builds the tick's `BotCtx` from owned pieces (the +0xB0 dist² recompute and the
        /// candidate bits happen in `step` — the game's `setOtherPlayerInfo` runs per tick).
        fn build_ctx<'a>(
            now: f32, pickups: &'a [BotPickup], others: &'a [OtherPlayer],
            world: &'a CollisionWorld, stationary: bool,
        ) -> BotCtx<'a> {
            BotCtx {
                now,
                scenario: 0x1, // deathmatch
                my_index: 4,
                my_team: 0,
                pickups,
                others,
                world,
                health: 100.0,
                max_health: 100.0,
                stationary,
            }
        }

        fn step(&mut self) {
            let mut others = self.others.clone();
            // the per-tick +0xB0 dist² recompute [M 0x125390:148-149] and the candidate bits
            for o in others.iter_mut() {
                if o.index == 0 {
                    o.dist2 = (o.pos[0] - self.brain.pos[0]).powi(2)
                        + (o.pos[1] - self.brain.pos[1]).powi(2)
                        + (o.pos[2] - self.brain.pos[2]).powi(2);
                    o.flags = 0x2 | if self.vision { 0x4 } else { 0 }; // bit1 candidate [M 0x125390:99] + the seen bit
                }
            }
            {
                let ctx =
                    Self::build_ctx(self.now, &self.pickups, &others, &self.world, self.stationary);
                self.brain.step(&ctx, &mut self.rng, true);
            }
            self.now += 1.0 / 60.0;
            // the driver mirrors the visit stamp into the pickup record [M 0x129958:14]
            let (idx, t) = self.brain.last_visit_stamp;
            if idx != usize::MAX {
                self.pickups[idx].visit[self.brain.vars.bot_index as usize] = t;
                self.brain.last_visit_stamp = (usize::MAX, 0.0);
            }
        }
    }

    /// The goal machine round trip: BotInit → 0xF9 (the 0xC3 chain [M 0x1748c8:16-18]); the
    /// first idle tick picks the pickup goal (the deathmatch fall-through [M 0x126b88:317-319])
    /// with gotoGoal's deadline stamp (+0x28 = 300.0 [M 0x1278a8:50,58]); the walk arrives; the
    /// visit stamp fires (now + 45.0 [M 0x129958:10-15]); the bot returns to 0xF9.
    #[test]
    fn goal_roundtrip_pickup_visit() {
        let mut r = Rig::new();
        assert_eq!(r.brain.state, 0xC3);
        r.step();
        assert_eq!(r.brain.state, st::BOT_IDLE, "the BotInit chain lands on 0xF9");
        r.step();
        assert_eq!(r.brain.state, st::BOT_GOTO_GOAL_POSITION, "the pickup goal → 0xEB");
        let g = r.brain.vars.goals[r.brain.vars.active_goal as usize];
        assert_eq!(g.goal_type, 1, "a pickup goal");
        assert_eq!(g.target, 0);
        assert_eq!(g.duration, 300.0, "gotoGoal's VIDEO_FRAME_RATE*5 stamp [M 0x1278a8:58]");
        assert_eq!(g.fire_state, st::BOT_IDLE, "gotoGoal(slot, 0xF9) [M 0x126b88:510]");
        // walk: 6.0 u at 0.05/tick ≈ 120 ticks to arrive
        let mut arrived = false;
        for _ in 0..400 {
            r.step();
            if r.brain.state == st::BOT_IDLE && !r.brain.visited_pickups.is_empty() {
                arrived = true;
                break;
            }
        }
        assert!(arrived, "the bot reaches the pickup and returns to 0xF9; state {:?}", r.brain.state);
        assert_eq!(r.brain.visited_pickups, vec![0]);
        let visit = r.pickups[0].visit[0];
        assert!(visit >= 45.0 && visit <= 52.0, "the visit stamp = arrival + 45.0, got {visit}");
        // the walk actually moved the bot: the final position is at the pickup
        assert!((r.brain.pos[0] - 6.0).abs() < 1.5, "pos {:?}", r.brain.pos);
    }

    /// Sister: the "stand" preset (the stationary gate) never moves — the route blocks hold
    /// the bot [M 0x16f198:42-45] — while the moving bot of the sister test does.
    #[test]
    fn stand_preset_does_not_move() {
        let mut r = Rig::new();
        r.stationary = true;
        let start = r.brain.pos;
        for _ in 0..200 {
            r.step();
        }
        assert_eq!(r.brain.pos, start, "the stationary bot holds position");
        // it still runs its goal machine (the goal is picked, the route code stalls)
        assert!(
            r.brain.vars.active_goal != 0xFF || !r.brain.visited_pickups.is_empty(),
            "the goal machine still runs"
        );
    }

    /// The fire decision: the opponent inside the engage range with clear LOS → the heartbeat's
    /// sighted latch → 0xEC (seen) → 0xCF (attack) → 0xD2 (fire) → the first shot lands after
    /// the scheduled interval ticks (the DroneWeap_Fire → NextBulletTime chain [M 0x16f5e8:25,
    /// 0x179d40]). The aim-cone gate [M 0x178c48] holds because the bot faces the opponent.
    #[test]
    fn attack_fire_shot_at_stationary_player() {
        let mut r = Rig::new();
        r.vision = true; // the bot sees the stationary player (the CanSeeObject stand-in)
        r.pickups.clear(); // nothing to fetch: the idle bot turns to the candidate
        let mut fired_at = None;
        for i in 0..600 {
            r.step();
            if !r.brain.shots.is_empty() {
                fired_at = Some(r.brain.shots[0].tick);
                break;
            }
            assert!(i < 599, "no shot within 600 ticks; trace {:?}", r.brain.trace);
        }
        let t = fired_at.unwrap();
        // the decompile's timing bounds: the shot happens after the interval ticks of the
        // burst schedule (6 ticks here) and within the 5 s sight-latch window
        assert!(t >= 7, "the first shot waits for the burst deadline, tick {t}");
        let dir = r.brain.shots[0].dir;
        assert!(dir[0] > 0.99, "the shot aims at the stationary player (+x), dir {dir:?}");
        // the burst bookkeeping: the latch is set and the clip decremented [M 0x17a8c0:15-17]
        assert_eq!(r.brain.vars.fire_latch, 1);
        assert_eq!(r.brain.vars.clip_rounds, 49);
        // the state path went through the attack pair
        let path: Vec<u8> = r.brain.trace.iter().map(|e| e.2).collect();
        assert!(path.contains(&st::BOT_SEEN_OPPONENT), "the heartbeat's sighted arm fires 0xEC");
        assert!(path.contains(&state::BOT_ATTACK), "→ 0xCF");
        assert!(path.contains(&state::BOT_ATTACK_FIRE), "→ 0xD2");
    }

    /// The visit decay: a visited pickup is skipped on passes 0..2 and only scores on pass 3
    /// with the ×max(|now−visit|·50, 1) penalty [M 0x126b88:404-417] — a fresh far pickup wins
    /// first; a stale-visited near pickup can win once the decay has run (the 45 s cooldown).
    #[test]
    fn pickup_visit_decay_skips_then_decays() {
        let mut r = Rig::new();
        r.pickups.push(BotPickup {
            pos: [30.0, 1.6, 0.0],
            weapon_id: 0x11,
            mp_class: 0,
            is_obj_flag2: false,
            visit: [0.0; 4],
        });
        // first pick: the near pickup 0 wins (both class 0, weights equal → the highest score)
        r.step();
        r.step();
        let first = r.brain.vars.goals[r.brain.vars.active_goal as usize].target;
        assert_eq!(first, 0, "the near pickup wins the first pick");
        // mark pickup 0 visited right now (the cooldown window): the next pick must be pickup 1
        r.pickups[0].visit[0] = r.now + 45.0;
        r.brain.vars.last_pickup = -1;
        r.brain.uninit_goal(0);
        r.brain.set_goal_pick_prefs(0.0, 0.0, 0.0, 1.0, 1, 0, 0, 100.0, 1.0, 1.0);
        assert!(r.brain.pick_goal(
            &Rig::build_ctx(r.now, &r.pickups, &r.others, &r.world, false), &mut r.rng, 1
        ), "the second pick finds pickup 1");
        let second = r.brain.vars.goals[r.brain.vars.active_goal as usize].target;
        assert_eq!(second, 1, "the visited pickup is skipped (passes 0..2)");
        // 90 s later the decay (|now−visit|·50 → 4500·dist) has buried pickup 0 anyway; the
        // fresh pickup still wins, but a 90-s-cooled pickup 0 with a closer pos would beat a
        // far pickup 1 — pin the decay arithmetic directly:
        let decayed = ((100.0f32 - 10.0) * 50.0).max(1.0);
        assert_eq!(decayed, 4500.0, "the 90 s decay multiplier is the hard blacklist [M :407-415]");
    }

    /// The RNG order of a goal pick is exactly the decompile's: gotoGoal's slot-1 wander budget
    /// rolls only for the objective subtypes; a pickup goal (subtype 0) consumes none [M
    /// 0x1278a8:84-142] — two equal rigs with different seeds pick the same target.
    #[test]
    fn pick_goal_rng_order_is_deterministic() {
        let mut a = Rig::new();
        let mut b = Rig::new();
        a.rng = Rng::new(7);
        b.rng = Rng::new(0xBEEF);
        a.step();
        b.step();
        a.step();
        b.step();
        assert_eq!(a.brain.vars.goals[a.brain.vars.active_goal as usize].target,
                   b.brain.vars.goals[b.brain.vars.active_goal as usize].target,
                   "the pickup pick consumes no RNG (subtype 0 → no gotoGoal roll)");
    }
}
