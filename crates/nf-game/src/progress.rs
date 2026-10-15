//! The mission-progress layer: the end-of-mission tally, the rating (score + medals), the
//! difficulty global and the save fields. Implements `docs/fields/player-stats.md` (the verified
//! decompile doc) and `docs/systems/progress.md` (this port's entry).
//!
//! Ported globals ([M] addresses in ACTION.ELF, layouts proven against `work/ee_start_05.bin`):
//! `PlrMissionStats` 0x2bcc20 (8 × 56 B; SP always slot 0), `BondMoments` 0x2bcde0 (10 × s16),
//! `ScoringTable` 0x2bc640 (12 × 60 B) with its static row arrays (`HT_*_ST` 0x2bbda0…0x2bc640),
//! `ScoringTableDifMul` 0x2bc910 (3 f32), `RewardsTable` 0x2bc920 (12 × 64 B), `bonus` 0x2e1b88
//! (u64[21]; slots 0..3 are the player slots), `sp_level` 0x2df2e0 (12 × 24 B) and the difficulty
//! word `GameState+40` (DECIMAL — hex +0x28, 0x2a3790; boot default 2 [M `bootup` 0x1c7e78:25,
//! the `sw r8, 40(r17)` 0x1c7f88; every gameplay reader `lw`s 0x2a3790: InitWeapon 0x1a7ef8:
//! 0x1a7f64, Check_AutoAim 0x1a73f8, Player_HandlePain 0x1902c8, DroneWeap_DoBulletAccuracy
//! 0x179740] [L ee_start_05: [0x2a3790]=1, [0x2a37a8]=0]).
//!
//! The static tables below are the snapshot's bytes ([L] `work/ee_start_05.bin`, file offset =
//! EE address), not reconstructions: thresholds, penalties, the 9 score rows per level, the reward
//! descriptors and the sp_level map order all match `docs/fields/player-stats.md`.
//!
//! Determinism: the only time input is `PlarStat_LogUpdateElapsedTime`'s `psiGetTimeIn100ths`
//! [M 0x1995a0], which the port derives from the fixed 60 Hz frame counter
//! (`now_100ths = frame · 100 / 60`) — no wall clock anywhere.

/// The mission tally record (`PlrMissionStats`, 56 bytes at 0x2bcc20 + slot·0x38). SP passes slot 0
/// at every call site [M docs/fields/player-stats.md "The PlrMissionStats record"].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlrMissionStats {
    /// `+0x00` — times enemies detected the player (`PlrStat_LogEnemyDetectedPlayer` 0x199960).
    pub detected: u32,
    /// `+0x04` — shots fired minus the shots that only hit scenery (`PlrStat_LogShotFired`
    /// 0x1998d8; `PlrStat_LogShotHitScenery` 0x198408 decrements).
    pub shots_fired: u32,
    /// `+0x08` — shots that hit an enemy (`PlrStat_LogShotHitEnemy` 0x199840).
    pub shots_hit_enemy: u32,
    /// `+0x0C` — enemies dispatched (`PlrStat_LogEnemyDispatched` 0x1996a8).
    pub dispatched: u32,
    /// `+0x10` — enemies disabled, the KO (`PlrStat_LogEnemyDisabled` 0x199730).
    pub disabled: u32,
    /// `+0x14` — enemies surrendered (`PlrStat_LogEnemySurrender` 0x1997b8).
    pub surrendered: u32,
    /// `+0x18` — enemies spawned (`PlrStat_LogEnemySpawned` 0x199430).
    pub spawned: u32,
    /// `+0x1C` — the health snapshot (`PlrStat_LogHealth` 0x199620 ← `Player_SetHealth` 0x1960b8:14).
    pub health: f32,
    /// `+0x20` — Bond moments completed (`PlrStat_DoneBondMoment` 0x199448).
    pub bond_moments: u32,
    /// `+0x24` — Bond bonus pickups collected (`PlrStat_LogBondBonus` 0x1999e0 ← `Pickup_Handler`
    /// case 6 0x18e650:235).
    pub bonus_pickups: u32,
    /// `+0x2C` — the last update's `psiGetTimeIn100ths` (`PlarStat_LogUpdateElapsedTime` 0x1995a0).
    pub last_time: u32,
    /// `+0x30` — the elapsed play time in 1/100 s (0x1995a0:10-13, gated on +0x34 == 0).
    pub elapsed: u32,
    /// `+0x34` — 0 = the timer running (0x1995a0).
    pub timer_run: u32,
    // +0x28 is unused (zero) [M docs/fields/player-stats.md]; the port stores no padding.
}

impl PlrMissionStats {
    /// The Log family's SP gate [M every `PlrStat_Log*` 0x198408..0x1999e0]:
    /// `MPSettings._384_4_ == 0` (the port has no MP mode → always true) AND
    /// `Mission_Status() ∈ {0, 1}` (0 = idle, 1 = playing; the port's `Mission::status()`).
    fn gated(mission_status: u32) -> bool { mission_status < 2 }

    /// `PlrStat_LogEnemyDetectedPlayer` 0x199960: `+0x00 += 1` behind the gate.
    pub fn log_detected(&mut self, ms: u32) { if Self::gated(ms) { self.detected += 1; } }
    /// `PlrStat_LogShotFired` 0x1998d8: `+0x04 += 1` behind the gate (per bullet spawned,
    /// `Player_WeaponInitBullet` 0x1a4bf0:258).
    pub fn log_shot_fired(&mut self, ms: u32) { if Self::gated(ms) { self.shots_fired += 1; } }
    /// `PlrStat_LogShotHitScenery` 0x198408: `+0x04 -= 1` behind the gate (SP_Update 0x1dbda8:261,
    /// Destroy_Smash 0x12bb60:55 — the accuracy denominator nets to the enemy hits).
    pub fn log_shot_hit_scenery(&mut self, ms: u32) { if Self::gated(ms) { self.shots_fired = self.shots_fired.saturating_sub(1); } }
    /// `PlrStat_LogShotHitEnemy` 0x199840: `+0x08 += 1` behind the gate (NDrone2_HitDamage
    /// 0x145558:60,130, DroneFunc_HandleImpact 0x1469b8:316, NDrone2_ExplosiveImpact 0x146480:66,
    /// Copter_Update 0x17dbb8:315).
    pub fn log_shot_hit_enemy(&mut self, ms: u32) { if Self::gated(ms) { self.shots_hit_enemy += 1; } }
    /// `PlrStat_LogEnemyDispatched` 0x1996a8: `+0x0C += 1` behind the gate.
    pub fn log_enemy_dispatched(&mut self, ms: u32) { if Self::gated(ms) { self.dispatched += 1; } }
    /// `PlrStat_LogEnemyDisabled` 0x199730: `+0x10 += 1` behind the gate.
    pub fn log_enemy_disabled(&mut self, ms: u32) { if Self::gated(ms) { self.disabled += 1; } }
    /// `PlrStat_LogEnemySurrender` 0x1997b8: `+0x14 += 1` behind the gate.
    pub fn log_enemy_surrender(&mut self, ms: u32) { if Self::gated(ms) { self.surrendered += 1; } }
    /// `PlrStat_LogEnemySpawned` 0x199430: `+0x18 += 1` behind the gate (NDrone2_DefaultInit
    /// 0x14b300:609, per drone created).
    pub fn log_enemy_spawned(&mut self, ms: u32) { if Self::gated(ms) { self.spawned += 1; } }
    /// `PlrStat_LogHealth` 0x199620: `+0x1C = health` behind the gate (the snapshot GetScore row 6
    /// reads).
    pub fn log_health(&mut self, ms: u32, health: f32) { if Self::gated(ms) { self.health = health; } }
    /// `PlrStat_LogBondBonus` 0x1999e0: `+0x24 += 1` behind the gate.
    pub fn log_bond_bonus(&mut self, ms: u32) { if Self::gated(ms) { self.bonus_pickups += 1; } }
    /// `PlrStat_DoneBondMoment` 0x199448: behind the gate and only when the moment is new
    /// (`BondMoments[id−0x65] == 0`): set the flag, `+0x20 += 1`. (The game also flashes
    /// `glb_blokes[p]+0x948 = 200` — the HUD's moment flash, owned by the view.)
    pub fn done_bond_moment(&mut self, ms: u32, moments: &mut [i16; 10], id: u32) {
        if !Self::gated(ms) { return; }
        let idx = (id.wrapping_sub(0x65)) as usize;
        if idx >= 10 || moments[idx] != 0 { return; }
        moments[idx] = 1;
        self.bond_moments += 1;
    }
    /// `PlarStat_LogUpdateElapsedTime` 0x1995a0: while `+0x34 == 0` (the timer runs),
    /// `+0x30 += now − +0x2C` and `+0x2C = now`. NO mission gate on this one [M the decompile:
    /// only `slot < 8` and the timer flag]. `now` is the port's frame-derived 1/100 s clock.
    pub fn log_update_elapsed(&mut self, now: u32) {
        if self.timer_run != 0 { return; }
        self.elapsed = self.elapsed.wrapping_add(now.wrapping_sub(self.last_time));
        self.last_time = now;
    }
    /// `psiGetTimeIn100ths`' port: the fixed 60 Hz frame counter in 1/100 s (deterministic; the
    /// game reads its own platform timer [S — the value only matters through differences, and the
    /// Sim has no wall clock]).
    pub fn now_100ths(frame: u32) -> u32 { (frame as u64 * 100 / 60) as u32 }
}

/// One score row (20 bytes; the static `HT_*_ST` arrays): `+0x00` count/par and `+0x04` target are
/// the design data, `+0x08` value, `+0x0C` f32 ratio and `+0x10` f32 raw score are GetScore's
/// runtime fields [M docs/fields/player-stats.md "The ScoringTable entry"].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScoringRow {
    /// `+0x00`: the par (allowed detections, the accuracy %, the par seconds — row-identity table
    /// in the doc; row 7's count is in seconds, its value in 1/100 s).
    pub count: u32,
    /// `+0x04`: the row's target score.
    pub target: u32,
    /// `+0x08`: the runtime value GetScore computes from the tally.
    pub value: u32,
    /// `+0x0C`: the clamped ratio.
    pub ratio: f32,
    /// `+0x10`: the raw score this row contributed.
    pub raw: f32,
}

/// One `ScoringTable` entry (60 bytes at 0x2bc640 + i·60): the design data is static, the score
/// fields are GetScore's [M].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScoringEntry {
    /// `+0x00`: the map hash (the lookup key = `Mission_BaseMapHCode` 0x1830a0).
    pub hash: u32,
    /// `+0x04/+0x08/+0x0C/+0x10`: the bronze/silver/gold/platinum thresholds
    /// (P_NFRESULTS 0x208e78:119-146 compares `last +0x34`).
    pub thresholds: [u32; 4],
    /// `+0x14`: the f32 penalty per detection over par (row 4); the code default is 0.03.
    pub penalty: f32,
    /// `+0x18`: the 9 rows (the static `HT_*_ST` arrays; count/target static, the rest runtime).
    pub rows: [ScoringRow; 9],
    /// `+0x24`: the difficulty multiplier (int; 0/1/2 from `ScoringTableDifMul`).
    pub dif: u32,
    /// `+0x28`: the bonus score = base × difmult.
    pub bonus: u32,
    /// `+0x2C`: the base score (Σ raw).
    pub base: u32,
    /// `+0x30`: the best score (persisted; LS_MakeMission 0x1cd720:54-56).
    pub best: u32,
    /// `+0x34`: the last score.
    pub last: u32,
    /// `+0x38`: 0 if hash&0xFF000000 == 0x09 (the "NewScores" maps), else 1.
    pub new_scores: u8,
}

/// One `RewardsTable` descriptor: `+0 bit` / `+2 kind(0..4)` / `+4 param` [M the doc's layout].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RewardDesc {
    /// The bit index in the `bonus` bitmask (< 0x3F).
    pub bit: u16,
    /// 0 = other rewards (weapon upgrades), 1..4 = the medal counters, 4 = the gold-medal groups.
    pub kind: u16,
    /// The entry's param (upgrade ids / label refs; meaning inferred [S docs/fields/
    /// player-stats.md "Unknowns" 4]).
    pub param: u32,
}

/// One `RewardsTable` entry (64 bytes at 0x2bc920 + i·64).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RewardEntry {
    /// `+0x00`: the map hash.
    pub hash: u32,
    /// `+0x04..+0x18`: 5 descriptors.
    pub desc: [RewardDesc; 5],
}

/// The 12 `sp_level` map hashes, in table order ([L] ee_start_05.bin 0x2df2e0, +0xC of each
/// 24-byte record; identical to the `ScoringTable` order).
pub static SP_LEVEL_HASHES: [u32; 12] = [
    0x0900_0001, 0x0700_0005, 0x0900_0005, 0x0900_0006, 0x0700_0001, 0x0700_0009,
    0x0700_000C, 0x0700_0011, 0x0900_0002, 0x0900_0003, 0x0700_0014, 0x0700_001B,
];

const fn entry(hash: u32, thresholds: [u32; 4], penalty: f32, rows: [(u32, u32); 9]) -> ScoringEntry {
    let new_scores = if hash & 0xFF00_0000 == 0x0900_0000 { 0 } else { 1 };
    ScoringEntry {
        hash, thresholds, penalty,
        rows: {
            let mut r = [ScoringRow { count: 0, target: 0, value: 0, ratio: 0.0, raw: 0.0 }; 9];
            let mut i = 0;
            while i < 9 { r[i] = ScoringRow { count: rows[i].0, target: rows[i].1, value: 0, ratio: 0.0, raw: 0.0 }; i += 1; }
            r
        },
        dif: 0, bonus: 0, base: 0, best: 0, last: 0,
        new_scores,
    }
}

/// The 12 scoring entries with their 9 rows ([L] ee_start_05.bin 0x2bc640 + the row arrays at
/// 0x2bbda0…0x2bc640). Row identities (the doc's row table): 0 Bond moments, 1 dispatched,
/// 2 disabled, 3 surrendered, 4 undetected (count = the allowed detections), 5 accuracy (count
/// 100), 6 health (count 100), 7 time (count = the par seconds), 8 bonus pickups.
pub static SCORING_TABLE: [ScoringEntry; 12] = [
    entry(0x0900_0001, [50000, 125000, 200000, 1000000], 0.0,
        [(7, 100000), (1, 75000), (0, 0), (0, 0), (0, 0), (100, 50000), (60, 50000), (240, 50000), (4, 100000)]),
    entry(0x0700_0005, [50000, 125000, 200000, 1000000], 0.35,
        [(2, 50000), (30, 50000), (3, 25000), (3, 25000), (30, 25000), (100, 50000), (100, 50000), (540, 25000), (7, 150000)]),
    entry(0x0900_0005, [50000, 125000, 200000, 1000000], 0.0,
        [(5, 100000), (1, 75000), (0, 0), (0, 0), (0, 0), (100, 50000), (92, 50000), (250, 50000), (3, 100000)]),
    entry(0x0900_0006, [50000, 125000, 200000, 1000000], 0.0,
        [(3, 100000), (1, 75000), (0, 0), (0, 0), (0, 0), (100, 50000), (90, 50000), (240, 50000), (5, 100000)]),
    entry(0x0700_0001, [125000, 250000, 500000, 1000000], 0.1,
        [(2, 50000), (45, 50000), (3, 25000), (3, 25000), (38, 25000), (100, 50000), (100, 50000), (480, 25000), (7, 150000)]),
    entry(0x0700_0009, [125000, 250000, 500000, 1000000], 0.1,
        [(2, 50000), (8, 50000), (0, 0), (0, 0), (10, 50000), (100, 50000), (100, 50000), (270, 50000), (7, 150000)]),
    entry(0x0700_000C, [125000, 250000, 500000, 1000000], 0.1,
        [(2, 50000), (45, 75000), (0, 0), (0, 0), (0, 0), (100, 100000), (100, 50000), (1200, 25000), (7, 150000)]),
    entry(0x0700_0011, [125000, 250000, 500000, 1000000], 0.0,
        [(2, 50000), (65, 75000), (0, 0), (0, 0), (0, 0), (100, 75000), (100, 75000), (720, 25000), (7, 150000)]),
    entry(0x0900_0002, [250000, 500000, 750000, 1000000], 0.0,
        [(3, 100000), (1, 75000), (0, 0), (0, 0), (0, 0), (100, 50000), (90, 50000), (420, 50000), (6, 100000)]),
    entry(0x0900_0003, [250000, 500000, 750000, 1000000], 0.0,
        [(7, 100000), (1, 75000), (0, 0), (0, 0), (0, 0), (100, 50000), (75, 50000), (609, 50000), (4, 100000)]),
    entry(0x0700_0014, [250000, 500000, 750000, 1000000], 0.1,
        [(1, 50000), (65, 75000), (0, 0), (0, 0), (68, 25000), (100, 75000), (100, 50000), (900, 25000), (7, 150000)]),
    entry(0x0700_001B, [250000, 500000, 750000, 1000000], 0.0,
        [(0, 0), (15, 100000), (0, 0), (0, 0), (0, 0), (100, 100000), (100, 100000), (255, 0), (7, 150000)]),
];

const fn reward(hash: u32, desc: [(u16, u16, u32); 5]) -> RewardEntry {
    RewardEntry { hash, desc: {
        let mut d = [RewardDesc { bit: 0, kind: 0, param: 0 }; 5];
        let mut i = 0;
        while i < 5 { d[i] = RewardDesc { bit: desc[i].0, kind: desc[i].1, param: desc[i].2 }; i += 1; }
        d
    } }
}

/// The 12 reward/medal entries ([L] ee_start_05.bin 0x2bc920). Descriptors 1..4 are the
/// bronze..platinum counters; kind 4 descriptors with the same packed {bit, kind} word form a
/// gold-medal group (the rank within the group widens the bit) [M `PlrStarts_ProcessRewardCounter`
/// 0x199210].
pub static REWARDS_TABLE: [RewardEntry; 12] = [
    reward(0x0900_0001, [(0, 0, 0x1e), (30, 0, 0x1002b), (43, 1, 0x40018), (24, 4, 0x10034), (52, 1, 0)]),
    reward(0x0700_0005, [(0, 0, 0x1a), (26, 0, 0x10026), (38, 1, 0x40000), (0, 4, 0x1002d), (45, 1, 0)]),
    reward(0x0900_0005, [(0, 0, 0x1b), (27, 0, 0x20038), (56, 2, 0x40003), (3, 4, 0x10035), (53, 1, 0)]),
    reward(0x0900_0006, [(0, 0, 0x1c), (28, 0, 0x1002a), (42, 1, 0x40006), (6, 4, 0x10030), (48, 1, 0)]),
    reward(0x0700_0001, [(0, 0, 0x1d), (29, 0, 0x20039), (57, 2, 0x40009), (9, 4, 0x1002e), (46, 1, 0)]),
    reward(0x0700_0009, [(0, 0, 0x23), (35, 0, 0x2003a), (58, 2, 0x40000), (0, 4, 0x10032), (50, 1, 0)]),
    reward(0x0700_000C, [(0, 0, 0x1f), (31, 0, 0x10028), (40, 1, 0x4000c), (12, 4, 0x1002f), (47, 1, 0)]),
    reward(0x0700_0011, [(0, 0, 0x20), (32, 0, 0x2003b), (59, 2, 0x40000), (0, 4, 0x10036), (54, 1, 0)]),
    reward(0x0900_0002, [(0, 0, 0x21), (33, 0, 0x10029), (41, 1, 0x4000f), (15, 4, 0x10031), (49, 1, 0)]),
    reward(0x0900_0003, [(0, 0, 0x22), (34, 0, 0x2003c), (60, 2, 0x40012), (18, 4, 0x10027), (39, 1, 0)]),
    reward(0x0700_0014, [(0, 0, 0x24), (36, 0, 0x3003e), (62, 3, 0x40015), (21, 4, 0x10033), (51, 1, 0)]),
    reward(0x0700_001B, [(0, 0, 0x25), (37, 0, 0x2003d), (61, 2, 0x40009), (9, 4, 0x10037), (55, 1, 0)]),
];

/// The reward-text half of each `RewardsTable` entry — what `Menu_GetLevelBonuses` 0x1ffe98:21-31
/// reads for the bonus page 0x40000038: per tier N (1..4) the entry's label at +0x18+4·N and the
/// texture hash at +0x2c+4·N [L ee_start_05.bin 0x2bc920; the labels are `Txt_BindLabel` ids
/// (`bank<<24|index` [M 0x1cad60:13]), the textures the 0x03xxxxxx sprite hashes the handler
/// forwards with msg 0x24 to control 0x100001cf [M 0x209670:23]]. Indexed like
/// [`REWARDS_TABLE`] (the same table order); `[tier - 1]` = (label, texture).
pub static REWARD_TEXT: [[(u32, u32); 4]; 12] = [
    [(0x2d6, 0x0300_012c), (0x2ec, 0x0300_0111), (0x0100_01f9, 0x0300_0102), (0x2f2, 0x0300_0118)], // 0x09000001
    [(0x2d2, 0x0300_0134), (0x2e9, 0x0300_0116), (0x3a5, 0x0300_00f3), (0x2f1, 0x0300_0108)], // 0x07000005
    [(0x2d3, 0x0300_0131), (0x2de, 0x0300_00a1), (0x2df, 0x0300_00e2), (0x0100_01fc, 0x0300_0196)], // 0x09000005
    [(0x2d4, 0x0300_0133), (0x2ea, 0x0300_011a), (0x3a6, 0x0300_00df), (0x2ee, 0x0300_0113)], // 0x09000006
    [(0x2d5, 0x0300_012f), (0x2e0, 0x0300_00a4), (0x3a7, 0x0300_00fa), (0x2ed, 0x0300_011d)], // 0x07000001
    [(0x2db, 0x0300_012e), (0x2e1, 0x0300_00a3), (0x3a5, 0x0300_00f3), (0x2f0, 0x0300_010b)], // 0x07000009
    [(0x2d7, 0x0300_0130), (0x3a3, 0x0300_011c), (0x3a8, 0x0300_00e0), (0x2e8, 0x0300_0110)], // 0x0700000c
    [(0x2d8, 0x0300_012a), (0x2e3, 0x0300_009d), (0x3a5, 0x0300_00f3), (0x3a2, 0x0300_0148)], // 0x07000011
    [(0x2d9, 0x0300_0198), (0x3a4, 0x0300_0114), (0x3a9, 0x0300_00e1), (0x2ef, 0x0300_010f)], // 0x09000002
    [(0x2da, 0x0300_0132), (0x2e5, 0x0300_009e), (0x3aa, 0x0300_00e6), (0x3a1, 0x0300_010a)], // 0x09000003
    [(0x2dc, 0x0300_012d), (0x2e6, 0x0300_00c4), (0x3ab, 0x0300_00c4), (0x2eb, 0x0300_0117)], // 0x07000014
    [(0x0100_01ff, 0x0300_012b), (0x2e7, 0x0300_00a0), (0x3a7, 0x0300_00fa), (0x0100_01fb, 0x0300_0194)], // 0x0700001b
];

/// `ScoringTableDifMul` 0x2bc910 ([L]: [0.0, 1.0, 2.0]) — the difficulty bonus multiplier
/// (difficulty 1 → [0], 2 → [1], anything else → [2] [M `PlrStat_GetScore` 0x1984b0:294-300]).
pub static SCORING_DIF_MUL: [f32; 3] = [0.0, 1.0, 2.0];

/// A kind-4 descriptor's group lookup ([M 0x199210's kind-4 arm]): the entries whose FIRST
/// descriptor shares the packed {bit, kind} u32 word form the group; this entry's effective bit is
/// `bit + rank` within the group order. Returns `(bit, group_len, rank)`.
fn gold_group(entry_idx: usize) -> (u16, usize, usize) {
    let word = REWARDS_TABLE[entry_idx].desc[0].bit as u32 | ((REWARDS_TABLE[entry_idx].desc[0].kind as u32) << 16);
    let mut rank = 0;
    let mut len = 0;
    for (i, e) in REWARDS_TABLE.iter().enumerate() {
        let w = e.desc[0].bit as u32 | ((e.desc[0].kind as u32) << 16);
        if w == word { len += 1; if i < entry_idx { rank += 1; } }
    }
    (REWARDS_TABLE[entry_idx].desc[0].bit + rank as u16, len, rank)
}

/// The outcome of one `PlrStat_GetScore` commit — what the results screen and the save show.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Scored {
    /// `ScoringTable[i]+0x2C` — the base score (Σ raw).
    pub base: u32,
    /// `+0x28` — base × the difficulty multiplier.
    pub bonus: u32,
    /// `+0x34` — the last score (base + bonus).
    pub last: u32,
    /// `+0x30` — the best score after this run.
    pub best: u32,
    /// `PlrStats_DoneBetter` 0x199ac0's flag (only set when `commit`).
    pub done_better: bool,
    /// The medal the results screen awards (P_NFRESULTS 0x208e78:119-146: 1 bronze .. 4 platinum,
    /// 5 = the "target" none).
    pub medal: u32,
}

/// The level-end score hand-off (`fix-scores-handoff`, the G-045 remainder): the rows and totals
/// the Sim produces when a level exits — the plain data the end-of-level results screen reads.
/// No UI; the results page (`menu.rs results_enter`) re-runs the game's own scoring on entry.
///
/// The game's results path is `P_NFRESULTS_Handler` 0x208e78:77 `PlrStat_GetScore(0)` — commit
/// mode [M 0x1984b0:301-315] — with `Mission_Status() == 6` [M 0x182c60:85-122 sets state 6
/// before the front end loads]; the scored entry then carries what the page reads [M :80-146].
/// The same scoring runs at the DRIVING→ACTION hand-off (`Boot_LoadPTPData` 0x1c8560's
/// `PlrStat_GetScore(0)` before `ResetMap_LevelToLoad(0x7000048)`). This production runs
/// READ-mode (commit = false): the rows/totals are exactly what the page's entry would hold,
/// while `best`/`last` stay the game's — a mid-mission part exit (07000005's truck ride into
/// 07000006) must not stamp them; only the results page commits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FinishedLevel {
    /// The base map hash the rows were scored under (`Mission_BaseMapHCode`'s port,
    /// `mission.base_map`).
    pub base_map: u32,
    /// The results-path mission status the production runs under (6 — the game's step 6 keeps
    /// the scores only there [M 0x1984b0:316-333]).
    pub mission_status: u32,
    /// The entry's 9 rows after the build: count/target the static design data, value the run's
    /// stat, ratio (+0xC) and raw (+0x10) the formula's output [M 0x1984b0:100-293].
    pub rows: [ScoringRow; 9],
    /// `Σ raw` [M entry +0x2C].
    pub base: u32,
    /// The difficulty multiplier applied [M entry +0x24; `ScoringTableDifMul` 0x2bc910].
    pub dif: u32,
    /// `base × dif` [M entry +0x28].
    pub bonus: u32,
    /// `base + bonus` — the score the results page prints [M 0x208e78:96-98 reads entry +0x34].
    pub last: u32,
    /// The entry's best after the production (unchanged — the read-mode call; the results page's
    /// own `GetScore(0)` commits [M 0x1984b0:301-315]).
    pub best: u32,
    /// The medal the page's compare chain would award on `last` (1 bronze .. 4 platinum, 5 none)
    /// [M 0x208e78:98-146].
    pub medal: u32,
    /// The mission tally snapshot the rows were produced from (`PlrMissionStats[0]` as of the
    /// exit tick) — "the stats the run produced".
    pub stats: PlrMissionStats,
    /// The difficulty the production ran with (GameState+40 DECIMAL = +0x28, 0x2a3790's port).
    pub difficulty: u32,
}

/// The whole progress layer: the port's `PlrMissionStats` + `BondMoments` + `ScoringTable`
/// runtime + `bonus` + `sp_level` flags + the game-complete flag. Lives on `Drones` (the Sim);
/// the menus (180-fix-menus) read it through `Drones::progress`.
#[derive(Clone)]
pub struct Progress {
    /// `PlrMissionStats` 0x2bcc20 — SP uses slot 0 only.
    pub stats: [PlrMissionStats; 8],
    /// `BondMoments` 0x2bcde0: the done flags for moment ids 0x65..0x6E.
    pub bond_moments: [i16; 10],
    /// `ScoringTable` 0x2bc640 — the static tables with the runtime score fields.
    pub table: [ScoringEntry; 12],
    /// `bonus` 0x2e1b88 — u64[21]; the game only uses slots 0..3 (the player slots
    /// [M Menu_UnlockMPSettings 0x1ff698]) and the save carries slot 0 (LS_MakeBonus 0x1ce3b0).
    pub bonus: [u64; 21],
    /// `sp_level` 0x2df2e0's `+0x10` completed flags, one bit per the SP_LEVEL_HASHES order (the
    /// `Menu_GetNightfireStatus` 0x202360 packing the MSSN block carries).
    pub nightfire_status: u32,
    /// `GameState[0x52]` — the game-complete flag (finishing 0x0700001B sets it
    /// [M Menu_SetLevelBonus 0x1ffa10]).
    pub game_complete: bool,
    /// `NewScoresRef` 0x30cc6c's stand-in: the DRIVING (0x09-map) score rows come from here in
    /// the game — a 9×3 s32 array (`[value, count, target]` per row) whose pointer bootup sets
    /// to the zeroed bss buffer 0x31ca38 (the `PTPDATA` region — the ACTION↔DRIVING hand-off
    /// block lives at 0x31c090 [M symtab; docs/atlas/driving.md §2]) and no ACTION.ELF writer
    /// exists [M whole-decompile grep + the whole-ELF lui/addiu scan: only `bootup` 0x1c7e78:86
    /// `NewScoresRef = &DAT_0031ca38` and `PlrStat_GetScore` 0x1984b0:45-64 touch them]. [L
    /// ee_start_05] the pointer is 0x31ca38 and rows 0..8 are all zero. So the game's shipped
    /// behaviour for the 0x09 maps is NOT the NULL abort (that is dead code — bootup always sets
    /// the ref): it reads the zero rows, every row's count is 0, nothing scores. The port models
    /// that state (`Some` zeroed rows); a driving hand-off, if one ever lands, writes rows here.
    pub new_scores: Option<[[i32; 3]; 9]>,
    /// `GameState+40` (DECIMAL — hex +0x28, address 0x2a3790) — the difficulty (1 Agent / 2
    /// Secret Agent / 3 00 Agent / 4 the bonus double-damage mode [M Player_HandlePain
    /// 0x1902c8:29-48]). Boot default 2 [M bootup 0x1c7e78:25 = `sw r8, 40(r17)` 0x1c7f88 →
    /// [0x2a3790]; the decomp's `GameState._40_4_` names are decimal offsets, so the old
    /// "GameState+0x40 (0x2a37a8)" citation was the hex/decimal mixup — [L ee_start_05:
    /// [0x2a3790]=1 with difficulty 1 active, [0x2a37a8]=0]. Kept here (not on GameState —
    /// the port has no GameState struct) and mirrored on `Drones::difficulty` for the tick code.
    pub difficulty: u32,
    /// `DAT_002e1c2c` — "a new reward unlocked": Menu_SetLevelBonus 0x1ffa10 sets it when a medal
    /// counter's bit was not yet set [M :74-79], and P_NFRESULTS case 0x4b routes to the bonus
    /// page 0x40000038 while it stands [M 0x208e78:49-52]. Cleared at every SetLevelBonus entry
    /// [M :56].
    pub new_reward: bool,
    /// The last level-end production (`finish_level`) — the score rows the game would hand to
    /// the results screen, as plain data (fix-scores-handoff). Rebuilt on every exit tick while
    /// the level-exit latch stands; consumed by tests/the menus, never committed back.
    pub finished: Option<FinishedLevel>,
}

impl Default for Progress {
    /// A fresh profile: everything zero except the difficulty's boot default 2
    /// [M bootup 0x1c7e78:25]. (`Menu_SetNightfireStatus` 0x2023a0 defaults the loaded status to
    /// 0x3 when the save holds 0 — the LOAD path's default, applied in `load`, not here.)
    fn default() -> Self {
        Progress {
            stats: [PlrMissionStats::default(); 8],
            bond_moments: [0; 10],
            table: SCORING_TABLE,
            bonus: [0; 21],
            nightfire_status: 0,
            game_complete: false,
            // Bootup 0x1c7e78:86 sets `NewScoresRef = &DAT_0031ca38` (the zeroed PTPDATA tail);
            // the NULL GetScore abort is dead code in the shipped game.
            new_scores: Some([[0; 3]; 9]),
            difficulty: 2,
            new_reward: false,
            finished: None,
        }
    }
}

impl Progress {
    /// `PlrStat_ResetForMission` 0x1993a0: memset PlrMissionStats (0x1C0) and BondMoments (0x14) —
    /// called by `Mission_Init` 0x182270:65 and `Menu_MapDefaultCodename` 0x1f7fa8:60. (The
    /// ScoringTable's runtime fields are NOT reset here — that is `PlrStat_Init` 0x198318 /
    /// `PlrStats_ResetScoring` 0x199020, a profile-new operation.)
    pub fn reset_for_mission(&mut self) {
        self.stats = [PlrMissionStats::default(); 8];
        self.bond_moments = [0; 10];
    }

    fn stats_mut(&mut self) -> &mut PlrMissionStats { &mut self.stats[0] }
    fn stats(&self) -> &PlrMissionStats { &self.stats[0] }

    /// The Log wrappers (slot 0; the gate lives in `PlrMissionStats`).
    pub fn log_shot_fired(&mut self, ms: u32) { self.stats_mut().log_shot_fired(ms); }
    pub fn log_shot_hit_scenery(&mut self, ms: u32) { self.stats_mut().log_shot_hit_scenery(ms); }
    pub fn log_shot_hit_enemy(&mut self, ms: u32) { self.stats_mut().log_shot_hit_enemy(ms); }
    pub fn log_enemy_dispatched(&mut self, ms: u32) { self.stats_mut().log_enemy_dispatched(ms); }
    pub fn log_enemy_disabled(&mut self, ms: u32) { self.stats_mut().log_enemy_disabled(ms); }
    pub fn log_enemy_surrender(&mut self, ms: u32) { self.stats_mut().log_enemy_surrender(ms); }
    pub fn log_enemy_spawned(&mut self, ms: u32) { self.stats_mut().log_enemy_spawned(ms); }
    pub fn log_detected(&mut self, ms: u32) { self.stats_mut().log_detected(ms); }
    pub fn log_bond_bonus(&mut self, ms: u32) { self.stats_mut().log_bond_bonus(ms); }
    pub fn log_health(&mut self, ms: u32, health: f32) { self.stats_mut().log_health(ms, health); }
    pub fn log_update_elapsed(&mut self, now: u32) { self.stats_mut().log_update_elapsed(now); }
    pub fn done_bond_moment(&mut self, ms: u32, id: u32) {
        let mut m = self.bond_moments;
        self.stats_mut().done_bond_moment(ms, &mut m, id);
        self.bond_moments = m;
    }

    /// `PlrStat_GetScore` 0x1984b0. `base_map` = `Mission_BaseMapHCode()`, `ms` =
    /// `Mission_Status()`, `difficulty` = `GameState+0x40`, `commit` = the results path — the
    /// game's param_1 **0** (P_NFRESULTS 0x208e78:77 calls `GetScore(0)`); a non-zero param (the
    /// pause screen's read) leaves the entry's last/best alone.
    ///
    /// Row sources [M 0x1984b0:38-84 — the hash's top byte picks the branch]: the 0x07 maps
    /// (entry +0x38 == 1) build the rows from `PlrMissionStats[0]`; the 0x09 maps (+0x38 == 0)
    /// copy `NewScoresRef`'s 9×3 s32 rows (row 7's value ×100) and abort only when the pointer
    /// is NULL — dead in the game, whose bootup always sets it to the zeroed PTPDATA buffer
    /// [M 0x1c7e78:86; L ee_start_05], which the port models with `new_scores: Some(zeros)`. The
    /// doc maps in docs/systems/progress.md and docs/fields/player-stats.md step 2 carry the
    /// 0x07/0x09 labels swapped vs the decompile (the accuracy formula in the +0x38==1 branch
    /// settles it: accuracy is an on-foot stat) — flagged in the fix-progress-remainder report.
    pub fn get_score(&mut self, base_map: u32, ms: u32, difficulty: u32, commit: bool) -> Option<Scored> {
        let idx = self.table.iter().position(|e| e.hash == base_map)?;
        let s = *self.stats();
        // The row values, per source [M 0x1984b0:38-84].
        let mut rows = self.table[idx].rows;
        if self.table[idx].new_scores == 0 {
            let ns = self.new_scores.as_ref()?;
            // [value, count, target] per row; row 7's value is ×100 at the copy [M 0x1984b0:
            // 52-54 — the driving module's par seconds become the 1/100-s unit the row carries;
            // the row-7 step's /100 turns it back].
            for (r, (i, n)) in rows.iter_mut().zip(ns.iter().enumerate()) {
                r.value = if i == 7 { n[0].wrapping_mul(100) } else { n[0] } as u32;
                r.count = n[1] as u32;
                r.target = n[2] as u32;
            }
        } else {
            rows[0].value = s.bond_moments;
            rows[1].value = s.dispatched;
            rows[2].value = s.disabled;
            rows[3].value = s.surrendered;
            rows[4].value = s.detected;
            rows[5].value = if s.shots_fired > 0 {
                (rows[5].count as u64 * s.shots_hit_enemy as u64 / s.shots_fired as u64) as u32
            } else { 0 };
            rows[6].value = s.health as u32; // fptoui [M 0x1984b0 row 6]
            if ms == 3 { rows[6].value = 0; } // the failed-mission zero [M 0x1984b0]
            rows[7].value = s.elapsed;
            rows[8].value = s.bonus_pickups;
        }
        // Per-row raw [M 0x1984b0 doc step 3].
        let mut base: f32 = 0.0;
        for i in 0..rows.len() {
            let r = &mut rows[i];
            if r.count == 0 { continue; }
            match i {
                // Row 4, the undetected row: over par → the penalty ramp.
                4 if r.count < r.value => {
                    let pen = if self.table[idx].penalty == 0.0 { 0.03 } else { self.table[idx].penalty };
                    let f = (1.0 - (r.value - r.count) as f32 * pen).max(0.0);
                    r.raw = f * r.target as f32;
                    r.ratio = f.clamp(0.0, 1.0);
                }
                // Row 7, the time row: value in 1/100 s, count in seconds.
                7 => {
                    let t = r.value as f32 / 100.0;
                    if (r.count as f32) < t {
                        r.raw = r.target as f32 * r.count as f32 / t;
                        r.ratio = (r.count as f32 / t).clamp(0.0, 1.0);
                    } else {
                        r.raw = r.target as f32;
                        r.ratio = 1.0;
                    }
                }
                // Row 8, the bonus pickups: all-or-nothing.
                8 => {
                    if r.count <= r.value { r.raw = r.target as f32; r.ratio = 1.0; }
                    else { r.raw = 0.0; r.ratio = 0.0; }
                }
                // The normal rows: raw = target · min(count, value) / count.
                _ => {
                    let m = r.count.min(r.value);
                    r.raw = r.target as f32 * m as f32 / r.count as f32;
                    r.ratio = (((m as f32 / r.count as f32) * 100.0).round() * 0.01).clamp(0.0, 1.0);
                }
            }
            base += r.raw;
        }
        self.table[idx].rows = rows;
        // The difficulty multiplier [M 0x1984b0:294-300] — written to the entry before the commit
        // guard, like the game's puVar7[9]/[10]/[0xb] stores [M :292-300].
        let dif_idx = match difficulty { 1 => 0, 2 => 1, _ => 2 };
        let fmult = SCORING_DIF_MUL[dif_idx];
        let base_u = base as u32;
        let bonus_u = (base * fmult) as u32;
        let total = base_u + bonus_u;
        let mut done_better = false;
        {
            let e = &mut self.table[idx];
            e.dif = fmult as u32;
            e.base = base_u;
            e.bonus = bonus_u;
            // The commit [M 0x1984b0:301-315 — the param_1 == 0 block]: DoneBetter = best ≤ total
            // (best BEFORE the update, and only when total != 0 [M :303-307]); then last = total
            // and best = max(best, last) [M :309-315]. A read (commit == false) touches neither.
            if commit {
                if total != 0 { done_better = e.best <= total; }
                e.last = total;
                e.best = e.best.max(total);
            }
        }
        // Step 6 [M 0x1984b0:316-333]: scores only stick on the results path (Mission_Status 6) —
        // everything from step 3 except entry.best is zeroed otherwise, and DoneBetter clears.
        let e = &mut self.table[idx];
        if ms != 6 {
            for r in e.rows.iter_mut() { r.value = 0; r.ratio = 0.0; r.raw = 0.0; }
            e.base = 0;
            e.bonus = 0;
            e.last = 0;
            e.dif = 0;
            done_better = false;
        }
        let th = SCORING_TABLE[idx].thresholds;
        let medal = if e.last >= th[3] { 4 } else if e.last >= th[2] { 3 }
            else if e.last >= th[1] { 2 } else if e.last >= th[0] { 1 } else { 5 };
        Some(Scored { base: e.base, bonus: e.bonus, last: e.last, best: e.best, done_better, medal })
    }

    /// The level-end hand-off (`fix-scores-handoff`): at a level exit the Sim produces the rows
    /// the game would hand to the end-of-level results screen, as plain data (no UI). The
    /// game's scoring moment is the results page's `PlrStat_GetScore(0)` with
    /// `Mission_Status() == 6` [M P_NFRESULTS 0x208e78:77-146; the state-6 gate 0x182c60:85-122;
    /// the same call at the DRIVING→ACTION hand-off, `Boot_LoadPTPData` 0x1c8560], so the
    /// production runs the same formula at status 6 — but READ-mode (commit = false): the entry's
    /// `best`/`last` stay the game's (a mid-mission part exit must not stamp them; the results
    /// page's own commit, already ported in `menu.rs results_enter`, updates them). Returns the
    /// produced `FinishedLevel` (None when the hash is not a `ScoringTable` entry) and mirrors it
    /// on `Progress::finished` — rebuilt on every exit tick while the latch stands (the same
    /// inputs score the same rows; the last tick's data is the final hand-off).
    pub fn finish_level(&mut self, base_map: u32, difficulty: u32) -> Option<FinishedLevel> {
        // The results-path status: step 6 keeps the scores only at Mission_Status 6
        // [M 0x1984b0:316-333]; the commit flag false leaves the entry's best/last alone
        // [M :301-315 — the param_1 != 0 arm].
        self.get_score(base_map, 6, difficulty, false)?;
        let idx = self.table.iter().position(|e| e.hash == base_map)?;
        let e = self.table[idx];
        // The page reads `last` (entry +0x34) off the committed entry; the read-mode entry
        // carries this run's totals in base/bonus [M :294-300 — the unconditional stores], so
        // the run's total (the page's would-be +0x34) is base + bonus.
        let total = e.base.wrapping_add(e.bonus);
        // The medal compare chain on the run's total [M 0x208e78:98-146 — the +0x10/+0xc/+8/+4
        // threshold cascade].
        let medal = if total >= e.thresholds[3] { 4 } else if total >= e.thresholds[2] { 3 }
            else if total >= e.thresholds[1] { 2 } else if total >= e.thresholds[0] { 1 } else { 5 };
        let out = FinishedLevel {
            base_map,
            mission_status: 6,
            rows: e.rows,
            base: e.base,
            dif: e.dif,
            bonus: e.bonus,
            last: total,
            best: e.best,
            medal,
            stats: self.stats[0],
            difficulty,
        };
        self.finished = Some(out);
        Some(out)
    }

    /// `PlrStarts_ProcessRewardCounter` 0x199210: the counter c (1 bronze .. 4 platinum) of
    /// `hash`'s reward entry, read or set in `bonus[slot]`. Kind-4 descriptors widen their bit by
    /// the rank within the packed-word group. Returns whether the bit was already set (the get
    /// arm); `out.count` (the group's set-bit count) is folded into the bool return for the get
    /// arm's caller (`Menu_HasMedal` needs just the bit).
    pub fn process_reward_counter(&mut self, hash: u32, counter: u32, slot: usize, set: bool) -> bool {
        let Some(idx) = REWARDS_TABLE.iter().position(|e| e.hash == hash) else { return false };
        if counter == 0 || counter > 5 { return false; }
        let d = REWARDS_TABLE[idx].desc[counter as usize];
        let bit = if d.kind == 4 { gold_group(idx).0 } else { d.bit };
        if bit >= 0x3F { return false; }
        let m = &mut self.bonus[slot];
        if set { *m |= 1u64 << bit; }
        *m & (1u64 << bit) != 0
    }

    /// `Menu_HasMedal` — the kind-`kind` reward bit of `hash` in `bonus[slot]`
    /// (gold = kind 3 [M PlrStats_HasGoldMedal 0x199a40 → Menu_HasMedal(BaseMapHCode, 3, 0)]).
    pub fn has_medal(&self, hash: u32, kind: u32, slot: usize) -> bool {
        let Some(idx) = REWARDS_TABLE.iter().position(|e| e.hash == hash) else { return false };
        if kind > 5 { return false; }
        let d = REWARDS_TABLE[idx].desc[kind as usize];
        let bit = if d.kind == 4 { gold_group(idx).0 } else { d.bit };
        if bit >= 0x3F { return false; }
        self.bonus[slot] & (1u64 << bit) != 0
    }

    /// `Menu_SetLevelBonus` 0x1ffa10: award `medal` (1 bronze .. 4 platinum, 5 none) for `hash` —
    /// the medal loop runs only for counters 1..4 (the `param_2 < 5 && 0 < param_2` guard [M
    /// :70], so a no-medal run awards nothing), each counter sets only when its bit was clear and
    /// raises `new_reward` when it does [M :72-79]; it stamps the level completed in `sp_level`
    /// (+0x10 = 1) AND the next level's (+0x10 = 1); finishing 0x0700001B instead sets the game
    /// complete [M :62-68].
    pub fn set_level_bonus(&mut self, hash: u32, medal: u32, slot: usize) {
        self.new_reward = false; // [M 0x1ffa10:56]
        if (1..5).contains(&medal) {
            for counter in (1..=medal).rev() {
                if !self.process_reward_counter(hash, counter, slot, false) {
                    self.process_reward_counter(hash, counter, slot, true);
                    self.new_reward = true; // [M 0x1ffa10:76-79]
                }
            }
        }
        if let Some(i) = SP_LEVEL_HASHES.iter().position(|&h| h == hash) {
            self.nightfire_status |= 1 << i;
            if hash == 0x0700_001B {
                self.game_complete = true; // [M 0x1ffa10 — GameState[0x52] = 1]
            } else if i + 1 < SP_LEVEL_HASHES.len() {
                self.nightfire_status |= 1 << (i + 1);
            }
        }
    }

    /// `Menu_GetNightfireStatus` 0x202360's packing: bit i of the 12 `sp_level[i]+0x10` flags.
    pub fn nightfire_status(&self) -> u32 { self.nightfire_status }

    /// `Menu_SetNightfireStatus` 0x2023a0 (the MSSN load): the value loads directly; a 0 save
    /// defaults to 0x3 (the first two levels available) [M the doc's MSSN block].
    pub fn set_nightfire_status(&mut self, v: u32) {
        self.nightfire_status = if v == 0 { 0x3 } else { v };
    }

    // ── The save (docs/fields/player-stats.md "Save/load" + the LS dossier) ────────────────────

    /// The game's LSB-first bit packer (`BIN_PushBits` 0x1e8b88): value bit i → stream bit
    /// (pos+i), byte (pos+i)>>3, bit (pos+i)&7.
    fn push_bits(buf: &mut [u8], pos: &mut usize, value: u64, n: u32) {
        for i in 0..n {
            if (value >> i) & 1 == 1 {
                let k = *pos + i as usize;
                buf[k >> 3] |= 1 << (k & 7);
            }
        }
        *pos += n as usize;
    }

    /// `BIN_PullBits` 0x1e8db0, the mirror.
    fn pull_bits(buf: &[u8], pos: &mut usize, n: u32) -> u64 {
        let mut v = 0u64;
        for i in 0..n {
            let k = *pos + i as usize;
            if k >> 3 < buf.len() && buf[k >> 3] & (1 << (k & 7)) != 0 { v |= 1 << i; }
        }
        *pos += n as usize;
        v
    }

    /// `LS_MakeMission` 0x1cd720's MSSN payload, bit-exact: 32 b nightfire status, 8 b count,
    /// per entry 32 b best + 4 b rows[0]+8 (the moments value), 1 b game complete — 473 bits for
    /// the full table → 68-byte block [L the dossier's parse of the snapshot's save].
    pub(crate) fn make_mssn(&self) -> Vec<u8> {
        let payload_bits = 32 + 8 + self.table.len() * 36 + 1;
        let size = (64 + payload_bits + 7) / 8;
        let mut buf = vec![0u8; size];
        let mut pos = 64usize; // the header is bytes 0..8
        Self::push_bits(&mut buf, &mut pos, self.nightfire_status as u64, 32);
        Self::push_bits(&mut buf, &mut pos, self.table.len() as u64, 8);
        for e in &self.table {
            Self::push_bits(&mut buf, &mut pos, e.best as u64, 32);
            Self::push_bits(&mut buf, &mut pos, e.rows[0].value as u64, 4);
        }
        Self::push_bits(&mut buf, &mut pos, self.game_complete as u64, 1);
        buf[0..4].copy_from_slice(&0x4E53_534D_u32.to_le_bytes()); // 'MSSN'
        buf[4..8].copy_from_slice(&(size as u32).to_le_bytes());
        buf
    }

    /// `LS_LoadMission` 0x1cd880 → `PlrStats_SetupScoreTable` 0x1990e0: restore the best scores,
    /// the moments values and the nightfire status / game-complete flag. The runtime row fields
    /// (+0x1C..+0x30's scratch) stay as the static defaults — the game restores only what the
    /// block carries.
    pub(crate) fn load_mssn(&mut self, payload: &[u8], pos: &mut usize) {
        let status = Self::pull_bits(payload, pos, 32) as u32;
        let count = Self::pull_bits(payload, pos, 8) as usize;
        self.nightfire_status = if status == 0 { 0x3 } else { status }; // [M 0x2023a0's default]
        for i in 0..count.min(self.table.len()) {
            let best = Self::pull_bits(payload, pos, 32) as u32;
            let moments = Self::pull_bits(payload, pos, 4) as u32;
            self.table[i].best = best;
            self.table[i].rows[0].value = moments;
        }
        self.game_complete = Self::pull_bits(payload, pos, 1) != 0;
    }

    /// `LS_MakeBonus` 0x1ce3b0's BUNS payload: 64 bits = `bonus[0]` (the high dword pushed first
    /// — the bit packer's LSB-first order reproduces it: the u64's bit 0 goes to stream bit 0).
    pub(crate) fn make_buns(&self) -> Vec<u8> {
        let size = (64 + 64 + 7) / 8;
        let mut buf = vec![0u8; size];
        let mut pos = 64usize;
        Self::push_bits(&mut buf, &mut pos, self.bonus[0], 64);
        buf[0..4].copy_from_slice(&0x5355_4E42_u32.to_le_bytes()); // 'BUNS'
        buf[4..8].copy_from_slice(&(size as u32).to_le_bytes());
        buf
    }

    pub(crate) fn load_buns(&mut self, payload: &[u8], pos: &mut usize) {
        self.bonus[0] = Self::pull_bits(payload, pos, 64);
    }

    /// The local save file, in the game's data-file layout [M LS_GetSaveStuff 0x1cd0d8 +
    /// `psiSaveData` 0x110df0]: the blocks chained by their `size` headers, then the CRC-32 of the
    /// chain appended little-endian. The port models the two progress blocks (MSSN, BUNS — the
    /// game's SP saves always write mask 0xFF; the port's options blocks live with their owners).
    pub fn encode_save(&self) -> Vec<u8> {
        let mut buf = self.make_mssn();
        buf.extend_from_slice(&self.make_buns());
        let crc = nf_formats::png::crc32(0, &buf);
        buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }

    /// Parse the game's block chain (`LS_GetLoadStuff` 0x1cd2b0's walk: `{u32 id; u32 size;
    /// payload[size-8]}` until id 0 or past 0xFFF), verify the CRC trailer (`crc32buf` 0x120670 —
    /// the standard CRC-32 [L table[1] = 0x77073096]), and load the MSSN / BUNS payloads.
    pub fn decode_save(&mut self, file: &[u8]) -> Result<(), &'static str> {
        if file.len() < 4 { return Err("short"); }
        let (body, crc_bytes) = file.split_at(file.len() - 4);
        let crc = u32::from_le_bytes(crc_bytes.try_into().unwrap());
        if nf_formats::png::crc32(0, body) != crc { return Err("crc mismatch"); }
        let mut off = 0usize;
        let mut saw_mssn = false;
        let mut saw_buns = false;
        while off + 8 <= body.len() {
            let id = u32::from_le_bytes(body[off..off + 4].try_into().unwrap());
            let size = u32::from_le_bytes(body[off + 4..off + 8].try_into().unwrap()) as usize;
            if id == 0 || off + size > body.len() || size < 8 { break; }
            let payload = &body[off..off + size];
            let mut pos = 64usize;
            match id {
                0x4E53_534D => { self.load_mssn(payload, &mut pos); saw_mssn = true; }
                0x5355_4E42 => { self.load_buns(payload, &mut pos); saw_buns = true; }
                _ => {}
            }
            off += size;
        }
        if saw_mssn && saw_buns { Ok(()) } else { Err("missing MSSN/BUNS block") }
    }

    /// Write the save file (the same fields as the memory card, as a local file).
    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.encode_save())
    }

    /// Read the save file. Deterministic: nothing in the format carries a timestamp (the game's
    /// MC directory furniture is not part of the data file [M the dossier's container note]).
    pub fn load_from_file(&mut self, path: &std::path::Path) -> std::io::Result<Result<(), &'static str>> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(self.decode_save(&bytes)),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The MSSN block for a fresh table is bit-counted like the game's: 64 header bits + 32 + 8 +
    /// 12×36 + 1 = 537 payload bits → 68 bytes [L the dossier's parse].
    #[test]
    fn mssn_block_is_68_bytes() {
        let p = Progress::default();
        let b = p.make_mssn();
        assert_eq!(b.len(), 68, "LS_MakeMission's size for the 12-entry table");
        assert_eq!(u32::from_le_bytes(b[0..4].try_into().unwrap()), 0x4E53_534D);
        assert_eq!(u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize, 68);
    }

    /// The save round-trips: best scores, the moments value, the nightfire status, the game
    /// complete flag and the bonus bitmask survive encode → decode, and the CRC catches a flip.
    #[test]
    fn save_round_trips() {
        let mut p = Progress::default();
        p.table[1].best = 145_198; // [L] the snapshot's 0x07000005 best
        p.table[1].rows[0].value = 2;
        p.nightfire_status = 0xFFF;
        p.game_complete = true;
        p.bonus[0] = 0x4004_0000_0000; // [L] the snapshot's bonus[0]: bits 26 + 42
        let file = p.encode_save();
        assert_eq!(file.len(), 68 + 16 + 4, "MSSN + BUNS + the CRC trailer");
        let mut q = Progress::default();
        q.decode_save(&file).expect("parses");
        assert_eq!(q.table[1].best, 145_198);
        assert_eq!(q.table[1].rows[0].value, 2);
        assert_eq!(q.nightfire_status, 0xFFF);
        assert!(q.game_complete);
        assert_eq!(q.bonus[0], 0x4004_0000_0000);
        // Sister: a flipped payload bit fails the CRC.
        let mut bad = file.clone();
        bad[10] ^= 0x40;
        assert_eq!(q.decode_save(&bad), Err("crc mismatch"));
        // Sister: truncation is refused.
        assert_eq!(q.decode_save(&file[..40]), Err("crc mismatch"));
    }

    /// GetScore's arithmetic on 07000005 (a 0x07 map — the rows come from PlrMissionStats
    /// [M 0x1984b0:66-84]), then the same formula through a 0x09 entry's NewScoresRef copy
    /// [M :44-64].
    #[test]
    fn get_score_arithmetic() {
        // The 0x07 path: the mission tally rows [M 0x1984b0:66-84].
        let mut p = Progress::default();
        let s = p.stats_mut();
        s.bond_moments = 2;          // row 0: 2/2
        s.dispatched = 30;           // row 1: 30/30
        s.shots_fired = 10;
        s.shots_hit_enemy = 8;       // row 5: 80/100
        s.health = 46.0;             // row 6: 46/100
        s.elapsed = 54_000;          // row 7: 540.00 s == the 540 s par → full target
        s.bonus_pickups = 7;         // row 8: 7/7
        let sc = p.get_score(0x0700_0005, 6, 1, true).expect("the 0x07 entry scores");
        // Row raws: 50000·2/2 = 50000; 50000·30/30 = 50000; accuracy 50000·80/100 = 40000;
        // health 50000·46/100 = 23000; time full 25000; bonus 150000.
        // Σ = 50000+50000+40000+23000+25000+150000 = 338000.
        assert_eq!(sc.base, 338_000, "Σ raw");
        // Difficulty 1 → multiplier 0.0 → bonus 0, last = base.
        assert_eq!(sc.bonus, 0);
        assert_eq!(sc.last, 338_000);
        assert!(sc.done_better, "338000 ≥ the fresh best 0 [M 0x1984b0:303-307]");
        assert_eq!(sc.best, 338_000);
        // Medal vs [50000, 125000, 200000, 1000000]: 338000 ≥ 200000 → gold 3.
        assert_eq!(sc.medal, 3);
        // Sister: a read (the game's param_1 != 0 — the pause screen) leaves last/best alone.
        let before = (p.table[1].last, p.table[1].best);
        let sc_r = p.get_score(0x0700_0005, 6, 1, false).unwrap();
        assert_eq!(sc_r.last, before.0, "the read reports the entry's last");
        assert_eq!((p.table[1].last, p.table[1].best), before, "a read commits nothing [M 0x1984b0:301]");
        // Sister: the same tally at difficulty 3 doubles (multiplier 2.0).
        let mut p2 = Progress::default();
        std::mem::swap(p2.stats_mut(), p.stats_mut());
        let sc2 = p2.get_score(0x0700_0005, 6, 3, true).unwrap();
        assert_eq!(sc2.base, 338_000);
        assert_eq!(sc2.bonus, 676_000, "bonus = base × 2.0 [M 0x1984b0 step 4]");
        assert_eq!(sc2.last, 1_014_000, "last = base + bonus");
        // Sister: status != 6 zeroes the stickable fields but keeps best (step 6 zeroes everything
        // from step 3 EXCEPT entry.best [M 0x1984b0:316-333]). A fresh tally still earns the time
        // row's full target (elapsed 0 ≤ the 540 s par → 25000), so best = max(0, 25000).
        let mut p3 = Progress::default();
        let sc3 = p3.get_score(0x0700_0005, 1, 1, true).unwrap();
        assert_eq!(sc3.last, 0, "not the results path — last zeroed");
        assert_eq!(p3.table[1].best, 25_000, "best kept");
        assert!(!sc3.done_better, "DoneBetter clears off the results path [M 0x1984b0:332]");
        // The 0x09 path: NewScoresRef's [value, count, target] rows, row 7's value ×100
        // [M 0x1984b0:50-56] — the PTP stand-in supplies the driving tallies.
        let mut p4 = Progress::default();
        p4.new_scores = Some([
            [2, 5, 100_000], [1, 1, 75_000], [0, 0, 0], [0, 0, 0], [0, 0, 0],
            [80, 100, 50_000], [46, 92, 50_000], [250, 250, 50_000], [3, 3, 100_000],
        ]);
        let sc4 = p4.get_score(0x0900_0005, 6, 1, true).expect("the 0x09 entry scores from NewScoresRef");
        assert_eq!(sc4.base, 330_000, "40000+75000+40000+25000+50000+100000");
        assert_eq!(sc4.last, 330_000);
    }

    /// The medal bits: 07000005's bronze is bit 26 [L bonus[0] = 0x4004000000], the gold group of
    /// CastleExterior/TowerA/Tower2A spreads bits 0..2, and set_level_bonus stamps the sp_level
    /// flags and unlocks the next level.
    #[test]
    fn medal_bits_and_level_stamp() {
        let mut p = Progress::default();
        p.set_level_bonus(0x0700_0005, 4, 0);
        assert!(p.has_medal(0x0700_0005, 1, 0), "bronze bit 26");
        assert_eq!(p.bonus[0] & (1 << 26), 1 << 26);
        assert!(p.has_medal(0x0700_0005, 3, 0), "gold group rank 0 → bit 0");
        assert!(p.has_medal(0x0700_0005, 4, 0), "platinum bit 45");
        assert_eq!(p.nightfire_status & 0b110, 0b110, "07000005 (i 1) + the next 09000005 (i 2)");
        assert!(!p.game_complete);
        // Sister: the game-complete level.
        p.set_level_bonus(0x0700_001B, 1, 0);
        assert!(p.game_complete, "0x0700001B sets GameState[0x52]");
        // Sister: an un-earned medal reads false.
        let q = Progress::default();
        assert!(!q.has_medal(0x0700_0005, 1, 0));
    }

    /// The tally gates: the Log family only counts while Mission_Status ∈ {0,1}, the bond moment
    /// fires once, and the elapsed clock accumulates.
    #[test]
    fn tally_gates() {
        let mut p = Progress::default();
        p.log_shot_fired(1);
        p.log_enemy_dispatched(1);
        p.log_shot_hit_scenery(1);
        assert_eq!((p.stats().shots_fired, p.stats().dispatched), (0, 1), "fired 1, scenery −1");
        p.log_enemy_dispatched(2); // the failure path — gated off
        p.log_shot_fired(6);
        assert_eq!((p.stats().shots_fired, p.stats().dispatched), (0, 1));
        p.done_bond_moment(1, 0x65);
        p.done_bond_moment(1, 0x65); // the same moment again — the BondMoments gate
        assert_eq!(p.stats().bond_moments, 1);
        assert_eq!(p.bond_moments[0], 1);
        p.log_update_elapsed(10);
        p.log_update_elapsed(70);
        assert_eq!(p.stats().elapsed, 70, "100ths accumulate: 0→10 then 10→70");
        assert_eq!(p.stats().last_time, 70);
        // Sister: PlrStat_ResetForMission wipes the tally and the moments.
        p.reset_for_mission();
        assert_eq!(*p.stats(), PlrMissionStats::default());
        assert_eq!(p.bond_moments, [0; 10]);
    }

    /// GetScore's row source [M 0x1984b0:38-84]: the hash's top byte picks it — the 0x07 maps
    /// (`+0x38` = 1) build the rows from `PlrMissionStats`; the 0x09 maps (`+0x38` = 0) copy
    /// `NewScoresRef` (row 7 ×100 at the copy [M :52-54]). The NULL abort [M :44-48] is dead in
    /// the shipped game: bootup 0x1c7e78:86 points the ref at the zeroed PTPDATA tail 0x31ca38
    /// and no ACTION.ELF writer exists — the port's `Some(zeroed rows)` default models that.
    #[test]
    fn the_on_foot_mission_rows_come_from_the_mission_stats() {
        let mut p = Progress::default();
        let s = p.stats_mut();
        s.dispatched = 30; // row 1: 30/30 → the full 50000
        s.elapsed = 54_000; // row 7: 540.00 s == the 540 s par → the full 25000
        let sc = p.get_score(0x0700_0005, 6, 2, true).expect("the 0x07 map scores from PlrMissionStats");
        assert_eq!(sc.base, 75_000, "dispatched 50000 + time 25000");
        assert_eq!(sc.last, 150_000, "difficulty 2 → ×1.0 → last = base + bonus");
        assert_eq!(sc.medal, 2, "150000 ≥ the silver 125000, < the gold 200000");
        // Sister: the 0x09 maps keep the NewScoresRef row source — the bootup-set zeroed buffer
        // [M 0x1c7e78:86; L ee_start_05] scores nothing (every row's count is 0), no abort.
        let sc = p.get_score(0x0900_0005, 6, 2, true).expect("the 0x09 map reads the zeroed buffer");
        assert_eq!((sc.base, sc.last), (0, 0), "zero rows → nothing scores");
    }

    /// Menu_SetLevelBonus 0x1ffa10's medal loop runs only for counters 1..4 — a no-medal run
    /// (P_NFRESULTS's uVar11 = 5 [M 0x208e78:106]) awards NO medal bits (the `param_2 < 5` guard
    /// [M 0x1ffa10:70]) but still stamps the level complete + the next unlock [M :62-66].
    #[test]
    fn set_level_bonus_medal_none_awards_no_bits() {
        let mut p = Progress::default();
        p.set_level_bonus(0x0700_0005, 5, 0);
        for k in 1..=4 {
            assert!(!p.has_medal(0x0700_0005, k, 0), "medal 5 awards no counter {k}");
        }
        assert_eq!(p.nightfire_status() & 0b110, 0b110, "07000005 complete + 09000005 unlocked");
        // Sister: a real medal stamps every counter ≤ the medal.
        let mut p2 = Progress::default();
        p2.set_level_bonus(0x0700_0005, 2, 0);
        assert!(p2.has_medal(0x0700_0005, 1, 0) && p2.has_medal(0x0700_0005, 2, 0));
        assert!(!p2.has_medal(0x0700_0005, 3, 0), "the gold counter is above the silver medal");
    }

    /// The level-end hand-off (fix-scores-handoff): `finish_level` produces the rows the results
    /// screen would read, as plain data, and a run with no kills gives the zero rows (the task's
    /// sister). Read-mode: the entry's best/last are untouched (the commit belongs to the
    /// results page's own `GetScore(0)` [M 0x208e78:77, 0x1984b0:301-315]).
    #[test]
    fn finish_level_handoff_no_kills_zero_rows() {
        let mut p = Progress::default();
        // A run with no kills: nothing logged. The kill rows (1 dispatched / 2 disabled /
        // 3 surrendered), the detection row (4), the accuracy row (5, no shots) and the bonus
        // row (8) all score zero; the time row still scores full (elapsed 0 ≤ the 540 s par).
        let f = p.finish_level(0x0700_0005, 2).expect("07000005 is a ScoringTable entry");
        assert_eq!(f.base_map, 0x0700_0005);
        assert_eq!(f.mission_status, 6, "the production runs the results-path status");
        assert_eq!(f.stats, *p.stats(), "the snapshot is the run's tally");
        for i in [1usize, 2, 3, 4, 5, 8] {
            assert_eq!(f.rows[i].value, 0, "row {i}: the empty tally's stat");
            assert_eq!(f.rows[i].raw, 0.0, "row {i} scores zero on a no-kill run");
        }
        // The static design data survives in the hand-off rows.
        assert_eq!(f.rows[1].count, SCORING_TABLE[1].rows[1].count);
        assert_eq!(f.rows[1].target, SCORING_TABLE[1].rows[1].target);
        assert_eq!(f.rows[7].raw, SCORING_TABLE[1].rows[7].target as f32, "elapsed 0 → the full time target");
        // base = Σ raw = the time row only; difficulty 2 → ×1.0 → last = base + bonus.
        assert_eq!(f.base, 25_000);
        assert_eq!(f.dif, 1);
        assert_eq!(f.bonus, 25_000);
        assert_eq!(f.last, 50_000);
        assert_eq!(f.medal, 1, "50000 ≥ 07000005's bronze 50000, < the silver 125000");
        assert_eq!(f.best, 0, "the read-mode production commits nothing");
        assert_eq!((p.table[1].last, p.table[1].best), (0, 0), "the entry's last/best stay the game's");
        assert_eq!(p.finished, Some(f), "the hand-off mirrors on Progress::finished");
        // Sister: re-running while the latch stands is inert (the same inputs, the same rows).
        assert_eq!(p.finish_level(0x0700_0005, 2), Some(f));
        // Sister: an unknown hash produces nothing.
        assert_eq!(p.finish_level(0x0700_0048, 2), None, "the front end is not a table entry");
    }

    /// The hand-off's formula on a non-trivial tally: a kill run's dispatched row scores
    /// `target · min(count, value) / count` [M 0x1984b0:174-191], and the snapshot carries the
    /// stats the rows were produced from.
    #[test]
    fn finish_level_handoff_kill_run() {
        let mut p = Progress::default();
        let ms = 1; // the tally gate's playing status
        for _ in 0..7 { p.log_enemy_dispatched(ms); }
        p.log_health(ms, 80.0);
        p.log_update_elapsed(12_000); // 120.00 s of the 540 s par
        let f = p.finish_level(0x0700_0005, 3).expect("scores");
        assert_eq!(f.stats.dispatched, 7);
        assert_eq!(f.rows[1].value, 7);
        let m = f.rows[1].count.min(7);
        assert_eq!(f.rows[1].raw, f.rows[1].target as f32 * m as f32 / f.rows[1].count as f32,
            "the generic row's raw = target · min(count, value) / count [M 0x1984b0:186]");
        assert_eq!(f.rows[6].value, 80, "the health snapshot's fptoui [M 0x1984b0:80-81]");
        // Difficulty 3 → the multiplier 2.0 [M 0x1984b0:294-297].
        assert_eq!(f.dif, 2, "difficulty 3 doubles");
    }
}
