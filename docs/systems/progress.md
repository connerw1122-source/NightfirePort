# The mission-progress layer (scoring, medals, difficulty, save)

**Port status: PORTED (fix-progress, 2026-10-13).** The evidence doc is
[fields/player-stats.md](../fields/player-stats.md) (the verified decompile doc); this entry is the
port map. Everything here cites the game address it implements. 180-fix-menus owns the screens and
calls this layer's API; this crate owns the data and the logic.

## Where the port keeps it

`crates/nf-game/src/progress.rs` — one `Progress` on `Drones` (the Sim):
`pub progress: crate::progress::Progress` and `pub difficulty: u32`
(drone.rs). The view's hooks: `Sim::data()` mirrors `difficulty` into `Data`
(scene.rs), `step_bond` runs the per-tick elapsed log, and `Drones::tick` folds the
tally outboxes.

| Game global | Address | Port |
|---|---|---|
| `PlrMissionStats` (8 × 56 B) | 0x2bcc20 | `Progress.stats[8]` (`PlrMissionStats`) |
| `BondMoments` (10 × s16) | 0x2bcde0 | `Progress.bond_moments[10]` |
| `ScoringTable` (12 × 60 B) | 0x2bc640 | `Progress.table[12]` (`ScoringEntry`; static data = [L] `work/ee_start_05.bin`) |
| the 9-row arrays (`HT_*_ST`) | 0x2bbda0…0x2bc640 | `ScoringEntry.rows[9]` (`count`/`target` static, `value`/`ratio`/`raw` runtime) |
| `ScoringTableDifMul` (3 f32) | 0x2bc910 | `SCORING_DIF_MUL` = [0.0, 1.0, 2.0] [L] |
| `RewardsTable` (12 × 64 B) | 0x2bc920 | `REWARDS_TABLE` ([L] snapshot) |
| `bonus` u64[21] | 0x2e1b88 | `Progress.bonus[21]` (slots 0..3 used [M 0x1ff698]) |
| `sp_level` (12 × 24 B) | 0x2df2e0 | `SP_LEVEL_HASHES` + `Progress.nightfire_status` (the +0x10 flags packed [M 0x202360]) |
| `GameState+0x40` (difficulty) | 0x2a37a8 | `Drones.difficulty` (boot default 2 [M bootup 0x1c7e78:25]; `set_difficulty` = the page's write [M C_SBNFDFCTY 0x216ca8:10-17]) |
| `GameState[0x52]` game complete | 0x2a37ba | `Progress.game_complete` |

## The tally (the `PlrStat_Log*` family, all SP slot 0)

Every Log gates on `Mission_Status() ∈ {0, 1}` [M each function; the MP flag is never set in the
port]. The writers and their port hooks:

| Counter | Game writer | Port hook |
|---|---|---|
| +0x00 detected | `PlrStat_LogEnemyDetectedPlayer` 0x199960 | `log_detected` via the tick fold ← the `DroneVision_LogSeenOpponent` 0x1780a8 rule run at the port's `HaveOpponentSight` ack (fix-progress-remainder): once per drone (`Drone+0x42`, no clearer in ACTION.ELF), team +0x44 ≠ 2, team 3 only on 0x07000009/0x0700000A, and on 0x07000007 only while Bond's class byte (`Drones.bond_weapon`, obj+0xdc → +0x62) is outside the `>1 && >=0x47` band minus 0x6a..0x71 |
| +0x04 fired | `PlrStat_LogShotFired` 0x1998d8 ← `Player_WeaponInitBullet` 0x1a4bf0:258 per bullet | `Drones::fire_player` per pellet |
| +0x04 −1 scenery | `PlrStat_LogShotHitScenery` 0x198408 ← SP_Update 0x1dbda8:261, Destroy_Smash 0x12bb60:55 | the bullet's world-hit arm (`PlayerImpact::World`) |
| +0x08 hit enemy | `PlrStat_LogShotHitEnemy` 0x199840 ← `NDrone2_HitDamage` 0x145558:57-60 (team 1 + hit by the player) | `Drone.hit_log` outbox → the tick fold |
| +0x0C dispatched | `PlrStat_LogEnemyDispatched` 0x1996a8 ← `OnInitDeath` 0x145bd0:57-59 | `dispatched_logged` outbox → the tick fold |
| +0x10 disabled | `PlrStat_LogEnemyDisabled` 0x199730 ← the KO enter 0x172ac8 | `disabled_logged` outbox → the tick fold |
| +0x14 surrendered | `PlrStat_LogEnemySurrender` 0x1997b8 ← `Surrender_Anim` ENTER 0x1626a0 | `surrender_logged` outbox → the tick fold |
| +0x18 spawned | `PlrStat_LogEnemySpawned` 0x199430 ← `NDrone2_DefaultInit` 0x14b300:609 | the `Mission_Init` latch (after the reset — the game creates drones after it [L] +0x18 = 13) and the spawner's Spawn arm |
| +0x1C health | `PlrStat_LogHealth` 0x199620 ← `Player_SetHealth` 0x1960b8:14 | the tick fold (the last snapshot wins in the game too) |
| +0x20 moments | `PlrStat_DoneBondMoment` 0x199448 ← `Trigger_Calc` 0x1b1900:12-17 (channels 0x65..0x6E) | the tick fold's per-tick channel scan (the `BondMoments` gate makes repeats inert) |
| +0x24 bonus pickups | `PlrStat_LogBondBonus` 0x1999e0 ← `Pickup_Handler` case 6 0x18e650:234-236 | the take arm of `Taken::Bonus` |
| +0x2C/+0x30 elapsed | `PlarStat_LogUpdateElapsedTime` 0x1995a0 ← `Player_Update` 0x193600:132 | `step_bond` after the tick; `now` = `frame·100/60` (deterministic; the game reads its platform timer [S]) |

Reset: `PlrStat_ResetForMission` 0x1993a0 runs in the `Mission_Init` latch when the level is the
mission's first part [M 0x182270:65].

## The score (`PlrStat_GetScore` 0x1984b0)

`Progress::get_score(base_map, mission_status, difficulty, commit)` implements the doc's 6 steps:
the row sources (the 0x09 maps read `NewScoresRef`; the 0x07 maps read `PlrMissionStats` — the
hash's top byte sets the entry's +0x38 flag 0 for 0x09 / 1 for 0x07 [M 0x1984b0:38-47]; the
accuracy formula in the flag-1 branch settles the mapping), the per-row raw (normal / the row-4
detection penalty `max(0, 1 − over·penalty)` with the 0.03 code default / the row-7 time ratio in
1/100 s / the row-8 all-or-nothing bonus), `base = Σ raw`, the difficulty multiplier
(`ScoringTableDifMul`, 1 → 0.0 / 2 → 1.0 / else 2.0), `last = base + bonus`, `best = max`,
`DoneBetter` on commit, and step 6: **scores only stick when `Mission_Status() == 6`** (everything
but `best` is zeroed otherwise). `NewScoresRef` is modelled as the bootup state — bootup
0x1c7e78:86 points it at the zeroed PTPDATA tail 0x31ca38 (`PTPDATA` 0x31c090, the ACTION↔DRIVING
hand-off region [M symtab]) and no ACTION.ELF writer exists — so the 0x09 maps score zero rows
(nothing scores) and the NULL abort is dead code; the port's `new_scores: Some(zeroed 9×3)` and
the copy's row-7 ×100 [M 0x1984b0:52-54] live in `progress.rs`.

Medals: P_NFRESULTS 0x208e78:119-146 compares `last` against the entry's thresholds
(≥ +0x10 platinum 4, ≥ +0xC gold 3, ≥ +8 silver 2 — the doc prints silver as medal 1 in the
compare chain; the port's `Scored.medal` follows the chain 1 bronze .. 4 platinum, 5 none).
`Progress::set_level_bonus` = `Menu_SetLevelBonus` 0x1ffa10: set every counter bit ≤ medal via
`PlrStarts_ProcessRewardCounter` 0x199210 (kind-4 descriptors spread their bit by the rank within
the packed-word group — CastleExterior/TowerA/Tower2A share gold bits 0..2, HendersonA/
SpaceStationD share 9..10 [L]), then stamp `sp_level +0x10` for the level and the next one;
0x0700001B instead sets game complete. `Progress::has_medal` = `Menu_HasMedal` (gold =
kind 3 [M `PlrStats_HasGoldMedal` 0x199a40]).

## Difficulty (GameState+0x40, 0x2a37a8)

The readers, per the decompile (all wired to `Drones::difficulty` / `Data::difficulty`):

| Reader | Address | Effect |
|---|---|---|
| `Player_HandlePain` | 0x1902c8:29-48 | the damage factor to Bond: `Plr_DMod_Easy/Normal/Hard` ([L] 07000005 band 0.6/0.7/1.0; d4 = ×2) — `bullet::damage_factor` |
| `NDrone2_HitDamage` | 0x145558:134-152 | Bond→guard: `DroneDamage_*` 1.5/1.0/0.8 — `damage::hit_damage` |
| `DroneWeap_DoBulletAccuracy` | 0x179740:81-95 | the hit chance × 0.8/0.9/1.3 — `drone::level_accuracy` |
| `Check_AutoAim` | 0x1a73f8:127-149 | the aim-assist multiplier 1.9/1.0/0.0 — `Drones::auto_aim` |
| `Player_AutoAim` | 0x1a7c98:0x1a7d30-64 | the scan's distance cap: `Autoaim_Range` 25.0 (`ReadTuningVars` 0x1d3a80:20, store 0x1d3bcc [L]), except the Q-Claw (0x50/0x51) seeded from its `weapon_data+0x1c` (50/100 [L ee_start_05]) — `Drones::auto_aim` (fix-progress-remainder) |
| `Player_InitWeapon` | 0x1a7ef8:24-39 | writes MAX_AUTO_AIM_DIST_MIDL/NEAR per difficulty (d2 14/8, d<3 or d>3 12/18 [L ee_start_05 with difficulty 1: 18/12], d3 0/0, MP 16/10) — **write-only in ACTION.ELF** (the whole-ELF gp/absolute scans + the full disasm of both auto-aim functions find no reader): NOT threaded; the live gate is `Autoaim_Range` above |
| `Drone_Create` | 0x136be0:49 | spawn gate: the level-tag int ≤ difficulty |
| `SoundTrigger/Hint_Create` | 0x1b2628 / 0x1b3248 | the min-difficulty tags (5/6/7 = exclusively d1/2/3) |
| `Lock_Create` | 0x1ae1f8:67 | doors auto-unlock on d1 — `lock.rs` |
| `NDrone2_DefaultInit` | 0x14b300:504-535 | captains by properties 9/5/8 — `captain_setup` |
| `PlrStat_GetScore` | 0x1984b0:294-300 | the score multiplier |

Pickups and objective lists have NO direct difficulty reader in the game [M `Pickup_Handler`
0x18e650, `Mission_Init` 0x182270 — no +0x40 access]; per-difficulty objective differences arrive
through level scripts/tags (070-fix-mission's data — not forked here).

## The save

`Progress::encode_save` / `decode_save` (and `save_to_file` / `load_from_file`) reproduce the
game's data-file layout for the progress blocks ([M] the LS dossier in
docs/reports/fix-progress.md): blocks `{u32 id; u32 size; payload}` chained (`LS_GetSaveStuff`
0x1cd0d8's walk), LSB-first bit packing (`BIN_PushBits` 0x1e8b88), then the CRC-32 of the chain
appended LE (`crc32buf` 0x120670, the zlib table — `nf_formats::png::crc32`).

- **MSSN** (`LS_MakeMission` 0x1cd720): 32 b nightfire status, 8 b count, per entry 32 b
  `ScoringTable[i]+0x30` best + 4 b `rows[0]+8` moments, 1 b game complete — 473 bits → 68 bytes
  [L the snapshot's own save parses to exactly this].
- **BUNS** (`LS_MakeBonus` 0x1ce3b0): 64 b = `bonus[0]`.

Load: `LS_LoadMission` 0x1cd880 / `LS_LoadBonus` 0x1ce488; the nightfire status defaults to 0x3
when the save holds 0 [M `Menu_SetNightfireStatus` 0x2023a0]. The port does not model PLRS/GSET/
CHET/MPSG (the options blocks — the menus' owners) or the MC directory furniture; the file is the
block chain + CRC, deterministic (no timestamps).

## The API for the menus (180-fix-menus)

`Drones::set_difficulty(u32)` · `drones.progress.stats[0]` (the live tally) ·
`Progress::get_score(base_map, status, difficulty, commit) -> Option<Scored>` (medal in
`Scored.medal`) · `Progress::set_level_bonus(hash, medal, slot)` · `Progress::has_medal(hash,
kind, slot)` · `Progress::save_to_file/load_from_file` · `Progress::nightfire_status()` /
`set_nightfire_status` · `Progress.game_complete`.

## Tests

`crates/nf-game/src/progress.rs` module tests: the formula (380k-class arithmetic on 0x09000005,
the ×2 difficulty bonus, the status-6 stick rule), the medal bits + level stamp, the tally gates,
the MSSN size and the save round-trip + CRC. `crates/nf-game/tests/progress_live.rs`: a guard's
SG5 damage to Bond through the real AI path — 1.925 at the default difficulty 2, 1.65 at
difficulty 1 (2.75 × the [L] factors). `crates/nf-view/tests/fix_progress_live.rs`: the scripted
07000005 run — 2 guard kills + 1 bonus pickup through `step_bond` give the decompile's tally, and
the save carries the run's fields.
