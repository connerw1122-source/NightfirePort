# `PlrStat_*` — the player statistics, scoring and medals

Proof levels: see [README.md](README.md). Evidence: [M] the decompile (file:line or `mipsdis`
addresses — note this ELF encodes `mul rd,rs,rt` as SPECIAL funct 0x18 with rd = destination, which
`mipsdis` prints as `mult`; Ghidra is correct); [L] `work/ee_start_05.bin` (the snapshot is ~8 s into
mission 0x07000001 — after `Mission_Init`). Port status up front: **the whole scoring system is unported**
— the port keeps per-drone stand-ins for the enemy counters (`crates/nf-game/src/drone.rs:650-654,1727-1737`,
documented as replacing `PlrStat_LogEnemy*`; `pickup.rs:90,375` notes LogBondBonus without implementing it;
`mission.rs:162-163` "the port's PlrStat counters live on the drones themselves"). No crate contains
ScoringTable, PlrMissionStats, RewardsTable, the bonus bitmask, GetScore or the medal logic.

## The globals

| Global | Addr | Size | Layout |
|---|---|---|---|
| **PlrMissionStats** | 0x2bcc20 | 448 = 8 × 56 | one record per player slot (the Log functions' arg is the slot 0..7; all SP call sites pass 0) |
| **BondMoments** | 0x2bcde0 | 20 = 10 × s16 | moment-done flags, ids 0x65..0x6E (101..110) [M 0x199448:20-23] |
| **ScoringTable** | 0x2bc640 | 720 = 12 × 60 | per-level score entries (the `SCORETABLE` type), indexed by map hash |
| HT_*_ST (12 named row arrays) | 0x2bbda0…0x2bc640 | 180 = 9 × 20 each | the score **rows**; `ScoringTable[i]+0x18` points at them (statically initialised) |
| **ScoringTableDifMul** | 0x2bc910 | 12 = 3 f32 | difficulty multipliers [0.0, 1.0, 2.0] [L] |
| **RewardsTable** | 0x2bc920 | 768 = 12 × 64 | reward/medal descriptors per level |
| **bonus** | 0x2e1b88 | 168 | u64[21] reward/medal bitmask (save data) [M 0x202580:31] |
| **NewScoresRef** | 0x30cc6c (bss) | 4 | pointer to a 9×3 s32 array set in bootup 0x1c7e78:86; target all zero in the snapshot, no writer found in the decomp (honesty) |
| **DoneBetter** | 0x30cc70 | 1 | set by GetScore, read by `PlrStats_DoneBetter` 0x199ac0 |
| BaseMap | 0x30cba0 (bss) | 4 | the current SP map hash (`Mission_BaseMapHCode` 0x1830a0) |
| sp_level | 0x2df2e0 | 288 = 12×24 | +0xC map id, +0x10 completed (`Menu_SetLevelBonus` 0x1ffa10:54-69) |
| DAT_002a373c | 0x2a373c | 4 | the all-weapons cheat flag (also gates the ammo seed and the owned bytes) |

[M symtab; layouts derived from the PlrStat_* decompiles and confirmed against the snapshot.]

## The PlrMissionStats record (56 bytes; base 0x2bcc20 + slot·0x38)

| Off | Type | Meaning | Writers | Readers |
|---|---|---|---|---|
| +0x00 | u32 | times enemies detected the player | `PlrStat_LogEnemyDetectedPlayer` 0x199960 ← DroneVision_LogSeenOpponent 0x1780a8:36 | GetScore row 4 0x1984b0:71 |
| +0x04 | u32 | shots fired (minus the shots that only hit scenery) | `PlrStat_LogShotFired` 0x1998d8 ← Player_WeaponInitBullet 0x1a4bf0:258, GT_Update 0x180498:523; −1 by `PlrStat_LogShotHitScenery` 0x198408 ← SP_Update 0x1dbda8:261, Destroy_Smash 0x12bb60:55 | GetScore row 5 denominator |
| +0x08 | u32 | shots that hit an enemy (≤ +0x04) | `PlrStat_LogShotHitEnemy` 0x199840 ← NDrone2_HitDamage 0x145558:60,130, DroneFunc_HandleImpact 0x1469b8:316, NDrone2_ExplosiveImpact 0x146480:66, Copter_Update 0x17dbb8:315 | GetScore row 5 numerator |
| +0x0C | u32 | enemies dispatched (killed) | `PlrStat_LogEnemyDispatched` 0x1996a8 ← NDrone2_LogDroneDeath 0x14ab00:29, DroneFunc_OnInitDeath 0x145bd0:59 | GetScore row 1 |
| +0x10 | u32 | enemies disabled (KO) | `PlrStat_LogEnemyDisabled` 0x199730 ← NDrone2_DSTATE_KnockedOut_Anim 0x172a30:25 | GetScore row 2 |
| +0x14 | u32 | enemies surrendered | `PlrStat_LogEnemySurrender` 0x1997b8 ← NDrone2_DSTATE_Surrender_Anim 0x1626a0:28 | GetScore row 3 |
| +0x18 | u32 | enemies spawned | `PlrStat_LogEnemySpawned` 0x199430 ← NDrone2_DefaultInit 0x14b300:609 | none found [S: informational] |
| +0x1C | f32 | the health snapshot | `PlrStat_LogHealth` 0x199620 ← Player_SetHealth 0x1960b8:14, BOT_SetHealth, BOT_handlePain | GetScore row 6 (fptoui) |
| +0x20 | u32 | Bond moments completed | `PlrStat_DoneBondMoment` 0x199448 (also sets BondMoments[id−0x65]=1 and glb_blokes[p]+0x948 = 200) ← Trigger_Calc 0x1b1900:16 | GetScore row 0; save 0x1cd720:58 |
| +0x24 | u32 | Bond bonus pickups collected | `PlrStat_LogBondBonus` 0x1999e0 ← Pickup_Handler case 6 0x18e650:235 | GetScore row 8 |
| +0x28 | — | unused (zero) | — | — |
| +0x2C | u32 | the last update time (`psiGetTimeIn100ths`) | `PlarStat_LogUpdateElapsedTime` 0x1995a0:12-13 ← Player_Update 0x193600:132 (slot = BL+0x94E) | 0x1995a0 |
| +0x30 | u32 | the elapsed play time in 1/100 s | 0x1995a0:10-13 (gated on +0x34 == 0) | GetScore row 7 |
| +0x34 | u32 | 0 = the timer running | — | — |

Every Log function gates on `slot < 8` and (in SP) `Mission_Status() ∈ {0,1}` — the same predicate as
`PlrStat_OkToUpdate` 0x1993e0 [M]. Reset: `PlrStat_ResetForMission` 0x1993a0 memsets PlrMissionStats
(0x1C0) and BondMoments (0x14) — called by Mission_Init 0x182270:65 and Menu_MapDefaultCodename
0x1f7fa8:60; `PlrStat_Init` 0x198318 = ResetForMission + zeroing every ScoringTable entry's runtime
fields. **[L]** p0 = `+0x18=13, +0x1C=100.0f, +0x2C=818, +0x30=77`; p1..p7 zero; BondMoments zero —
matches the layout exactly.

## The ScoringTable entry (60 bytes @0x2bc640 + i·60; rows 9 × 20 bytes)

| Off | Meaning | [L] sample (the 0x07000005 entry @0x2bc67c) |
|---|---|---|
| +0x00 | the map hash (the lookup key = Mission_BaseMapHCode) | 0x07000005 |
| +0x04/+0x08/+0x0C/+0x10 | medal thresholds: **bronze / silver / gold / platinum** (P_NFRESULTS 0x208e78:119-146 compares last score +0x34: ≥+0x10 → platinum "medal 4", ≥+0xC gold 3, ≥+8 silver 1, ≥+4 bronze; < +4 → "target" 0x10001FD, medal 5) | 125000 / 250000 / 500000 / 1000000 |
| +0x14 | f32 penalty per detection over par (row 4); code default 0.03 | 0.35 |
| +0x18 | ptr → the 9-row array (HT_Level_*_ST) | 0x2bbe58 |
| +0x1C/+0x20 | scratch: the row-7 value / count cached by GetScore | 16944 / 540 |
| +0x24 | the difficulty multiplier (int) | 0 |
| +0x28 | bonus score = base × difmult | 0 |
| +0x2C | base score (Σ raw) | 0 |
| +0x30 | **best score** (persisted, LS_MakeMission 0x1cd720:54-56) | 145198 |
| +0x34 | last score | 0 |
| +0x38 | u8: 0 if hash&0xFF000000 == 0x09, else 1 (the "NewScores" maps) | 1 |

Row (20 bytes): +0x00 count/par (static design data), +0x04 target score, +0x08 value (runtime),
+0x0C f32 ratio, +0x10 f32 raw score. `PlrStats_ResetScoring` 0x199020 zeroes only +8/+C/+10 of the rows
across all 12 entries — called only from the profile-name handlers (a new profile wipes scores).
Entry order ↔ the symtab row names: 0x09000001 Driving_Paris, 0x07000005 CastleExterior, 0x09000005
SnowMobile, 0x09000006 Alps, 0x07000001 HendersonA, 0x07000009 TowerA, 0x0700000C PowerStationA1,
0x07000011 Tower2A, 0x09000002 Underwater, 0x09000003 JungleA, 0x07000014 EvilBase, 0x0700001B SpaceStationD.
Row identities (GetScore sources + P_NFSTATS 0x209938:60-116 labels 0x165,0x15B,0x15C/0x16E/0x15D,0x15E,0x15F,0x160,0x161):
row 0 Bond moments, 1 dispatched, 2 disabled, 3 surrendered, 4 undetected (count = the allowed detections),
5 accuracy (count 100), 6 health (count 100), 7 time (count = the par seconds; value in 1/100 s),
8 bonus pickups (all-or-nothing).

## The score formula (`PlrStat_GetScore` 0x1984b0)

1. Find the entry for `Mission_BaseMapHCode()`; NULL if absent. Set the +0x38 flag (the 0x09-group test).
2. Flag ≠ 0 (the 0x07 maps): copy the rows from `NewScoresRef` (9×3 s32: value/count/target, ×100 for
   row 7); abort if NULL. Flag == 0 (the 0x09 maps): the rows come from PlrMissionStats[0] as the table
   above; row 5.value = count × hits / shots; row 6.value = (u32)health; Mission_Status() == 3 (failed)
   → row 6.value = 0.
3. Per row (skip count == 0): **normal rows (0,1,2,3,5,6)**: raw = target × min(count,value) / count;
   ratio = clamp(round(min/count × 100) × 0.01, 0..1). **Row 4 (undetected)**: count < value (detections
   over par) → raw = max(0, 1 − (value−count)·entry.penalty) × target (penalty 0.03 when +0x14 == 0);
   else raw = target, ratio 1.0. **Row 7 (time)**: t = value/100; count < t → raw = target·count/t; else
   target. **Row 8**: count ≤ value → target, else 0. entry.base += (u32)raw.
4. The difficulty multiplier (GameState+0x40: 1 → [0] 0.0, 2 → [1] 1.0, else [2] 2.0);
   entry.dif = (u32)fmult; entry.bonus = base × difmult.
5. param 0: DoneBetter = (base+bonus ≥ best) when non-zero; last = base+bonus; best = max(best, last).
6. **If `Mission_Status() != 6` everything from step 3 except entry.best is zeroed** — scores only stick
   on the results path (6 = MISSION_COMPLETE per Mission_Update 0x182c60:86; 2/3 are the failure paths;
   Mission_Init sets 1). The pause stats screen calls GetScore(0) and shows the counts live [S: verify the
   Menu_AddRow arg meaning before porting the display].

**Gold medal**: `PlrStats_HasGoldMedal` 0x199a40 = `Menu_HasMedal(BaseMapHCode(), 3, 0)` — the RewardsTable
entry's kind-3 bit in `bonus[0]`; bits set at mission end by `Menu_SetLevelBonus` (P_NFRESULTS
0x208e78:187 → 0x1ffa10:70-82; medals 1=bronze..4=platinum, 5=none). Finishing 0x0700001B instead sets
GameState[0x52] = 1 (game complete). The RewardsTable entry (64 B): {u32 mapHash; 5×{u16 bitIndex; u16
kind(0..4); u32 param}} then four 0x0300xxxx label ids at +0x30.. — kinds 1..4 = the medal bits, kind 0 =
other rewards (weapon upgrades: Menu_GetLevelBonuses 0x1ffe98; the params 0x1E, 0x00040018, 0, 0x2EC, 0x2F2
on lvl 0 [L]); `PlrStarts_ProcessRewardCounter` 0x199210 is the bit engine (kind < 5, bit < 0x3F).

**Save/load**: `LS_MakeMission` 0x1cd720 pushes the nightfire status, a count, then per level 0x20 bits of
entry+0x30 (best) + 4 bits of rows[0]+8 (moments); `LS_LoadMission` 0x1cd880 → `PlrStats_SetupScoreTable`
0x1990e0 restores rows +8/+C/+10 and entry +0x1C..+0x30. `PlrStats_GetScoreTable` 0x199aa8 returns the
table + count 12; `PlrStats_Get/SetDrivingData` 0x199ac8/0x199b18 move 12 row pointers + row-0 values
through the save.

## Unknowns / honesty

1. ammo_data +0 (see [ammo-data.md](ammo-data.md)) — the +0 u16 is never read.
2. The text label strings (0x0500xxxx ammo names, the row titles, the medal names) are not decoded here.
3. `NewScoresRef`'s target buffer (0x31ca38) has no writer in this ELF — likely PTP/external or always
   empty on retail; affects only the 0x07-group GetScore branch.
4. RewardsTable `param` semantics (upgrade ids? map hashes?) and the four 0x0300xxxx labels — layout
   proven, meaning inferred.
5. The MISSION_STATE enum is not fully enumerated (observed 0/1 playing, 2/3 failures, 5, 6 complete).
6. `Player_SetWatch` 0x1a69d0 also touches PlrMissionStats (the watch gadget's stats display) — not traced.
7. Row 7's signed/unsigned edge cases in the decompile are branch noise — copy the float math.

## Live checks for the main agent

1. **The accuracy row**: fire 10 rounds at a wall (scenery) on 07000005 — `+0x04` should net to the shots
   that hit enemies only (`PlrStat_LogShotHitScenery` decrements); P_NFSTATS should show 0% until a guard
   is hit.
2. **The detection penalty**: let a guard spot Bond, then finish — row 4's raw should drop by
   entry+0x14 (0.35) per detection over par on 07000005.

## The census to run

`gdbwatch 0x2bcc20 0x1c0 readwrite --seconds 10` on 07000005 during a fight (shots, a kill, a KO, a
surrender, a detection): the expected writers are the Log family (0x1998d8/0x199840/0x1996a8/0x199730/
0x1997b8/0x199960/0x199620/0x1995a0) and the readers GetScore (0x1984b0) + the save path.

## `Player_InitWeapon` 0x1a7ef8:10-27 — the per-difficulty autoaim distance writes (fix-progress-detected-hook)

`Player_InitWeapon` (called from `Player_Init` at every mission/warm load) writes the pair
`MAX_AUTO_AIM_DIST_*` at **0x30ccf0** (`$gp-0x7980`) / **0x30ccf4** (`$gp-0x797c`) keyed on the
difficulty runtime `GameState+40` (DECIMAL — hex +0x28, address 0x2a3790; see the progress.rs
correction — the decomp's `_40_4_` names are decimal offsets, and `[0x2a37a8]` is a different
field that reads 0 in every snapshot [L]):

| difficulty (`[0x2a3790]`) | store target 0x30ccf0 | store target 0x30ccf4 | disasm |
|---|---|---|---|
| MP (`[0x2a4920]` != 0) | 10.0 | 16.0 | `lui at,0x4120/0x4180` 0x1a7f44-0x1a7f50 |
| 2 | 8.0 | 14.0 | `lui at,0x4100/0x4160` 0x1a7f98-0x1a7fa4 |
| anything but 3 (SP) | 12.0 | 18.0 | `lui at,0x4140/0x4190` 0x1a7f80-0x1a7f8c |
| 3 | 0.0 | 0.0 | `sw zero` 0x1a7f7c / 0x1a7fb4 |

The decomp's symbolic names are **crossed against the addresses**: the C labels the 0x30ccf0
global `MAX_AUTO_AIM_DIST_MIDL` and the 0x30ccf4 one `NEAR`, but the stores put the SMALLER value
(NEAR) at 0x30ccf0 and the LARGER (MIDL) at 0x30ccf4 — [L] `ee_start_05.bin` with difficulty 1
(the 12.0/18.0 arm) reads [0x30ccf0]=12.0, [0x30ccf4]=18.0, matching the disasm store order
(`swc1 f0`, then `swc1 f1` in the 0x1a7fac branch's delay slot), not the C labels.

**The pair is write-only in ACTION.ELF** — the port does not thread it (that would add behaviour
the shipped game does not have):
- whole-decompile grep: only `Player_InitWeapon` touches the names [M];
- the only autoaim consumers, `Check_AutoAim` 0x1a73f8 and `Player_AutoAim` 0x1a7c98, touch
  `Autoaim_Range/Angle_H/Angle_V/Easy/Normal/Hard/LockOnMul` only [M decomp; fix-progress-
  remainder's gp-relative + symbolic lui/addiu scans found no other reader];
- fix-progress-remainder (110d) fixed the real gap instead: `Player_AutoAim` seeds the scan cap
  from `Autoaim_Range` (25.0) except the Q-Claw (0x50/0x51), whose cap is its `weapon_data+0x1c`
  (50.0/100.0 [L]).

The **effective** per-difficulty autoaim behaviour is the multiplier pick in `Check_AutoAim`
0x1a73f8:127-141 — `Autoaim_NormalMul` default, `Autoaim_EasyMul` iff d==1, `Autoaim_HardMul`
iff d==3 or 4, keyed on the same `[0x2a3790]` — which the port threads at `Drones::auto_aim`
(drone.rs, `match self.difficulty { 1 => Easy, 3 | 4 => Hard, _ => Normal }`).
