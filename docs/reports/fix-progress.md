# fix-progress — the mission-progress layer

Branch `s/fix-progress`. The task: the mission tally, the rating (score + medals + unlocks), the
difficulty effects and the save. Evidence doc: `docs/fields/player-stats.md` (the verified
decompile doc — every address below traces there); port map: `docs/systems/progress.md` (new).

## What's done

1. **The tally** (`crates/nf-game/src/progress.rs`, new): `PlrMissionStats` 0x2bcc20 (8 × 56 B) as
   `Progress.stats[8]` with the `PlrStat_Log*` family — each with the game's gate
   (`Mission_Status() ∈ {0,1}`, SP slot 0) and hooked at the game's own call sites:
   - fired per bullet: `Drones::fire_player` (the port's `Player_WeaponInitBullet` 0x1a4bf0:258);
   - hit-enemy: `Drone::hit`'s outbox, the 0x145558:57-60 gate (team 1 + hit by the player), folded
     in `Drones::tick`;
   - hit-scenery: the bullet's world-hit arm (`PlrStat_LogShotHitScenery` 0x198408 ← SP_Update
     0x1dbda8:261; Destroy_Smash's breakables fold into the same path in the port);
   - dispatched / disabled / surrendered: the existing per-drone outboxes (`dispatched_logged`,
     `disabled_logged`, new `surrender_logged` at the `Surrender_Anim` ENTER 0x1626a0) folded in
     the tick (0x145bd0:57-59 / 0x172ac8 / 0x1997b8);
   - spawned per created drone (0x14b300:609): counted in the `Mission_Init` latch AFTER
     `PlrStat_ResetForMission` 0x1993a0 (the game creates drones after Mission_Init — [L]
     ee_start_05.bin shows +0x18 = 13 at a 07000001 start) and at the spawner's Spawn arm;
   - Bond bonus in the take arm (`Pickup_Handler` case 6 0x18e650:234-236);
   - Bond moments: the tick's per-tick scan of channels 0x65..0x6E (`Trigger_Calc` 0x1b1900:12-17;
     the `BondMoments` gate makes repeats inert);
   - health snapshot (`PlrStat_LogHealth` 0x199620 ← `Player_SetHealth` 0x1960b8:14) in the tick;
   - elapsed (`PlarStat_LogUpdateElapsedTime` 0x1995a0 ← `Player_Update` 0x193600:132) in
     `step_bond`, with `now = frame·100/60` — deterministic, no wall clock.
2. **The rating**: the 12 `ScoringTable` entries with thresholds, penalties and the 12 × 9 row
   arrays read byte-for-byte from `work/ee_start_05.bin` ([L]; they match the doc's samples), and
   `PlrStat_GetScore` 0x1984b0's 6 steps: row sources (the 0x09 maps from PlrMissionStats; the
   0x07 maps from `NewScoresRef` — the port's `new_scores: None` = the game's NULL, so on-foot
   maps abort exactly as the game does, see below), the per-row raw maths (normal, the row-4
   detection penalty `max(0, 1 − over·penalty)` with the 0.03 code default, the row-7 time ratio,
   the row-8 all-or-nothing), the difficulty multiplier (`ScoringTableDifMul` 0x2bc910 = [0.0,
   1.0, 2.0] [L]), last/best/DoneBetter, and step 6 (scores stick only at `Mission_Status() == 6`).
   Medals: `Scored.medal` per P_NFRESULTS 0x208e78:119-146's compare chain;
   `set_level_bonus` = `Menu_SetLevelBonus` 0x1ffa10 (bits ≤ medal, `sp_level +0x10` for the level
   and the next, 0x0700001B → game complete); `process_reward_counter` = 0x199210 including the
   kind-4 group ranks (CastleExterior/TowerA/Tower2A share gold bits 0..2, HendersonA/
   SpaceStationD share 9..10 [L]); `has_medal` = `Menu_HasMedal` (gold = kind 3, 0x199a40).
3. **Difficulty**: `Drones::difficulty` replaces the hard-coded `DIFFICULTY` const (which was also
   mislabelled `GameState+0x28` — the scored member is **+0x40** = 0x2a37a8 [M
   `PlrStat_GetScore` 0x1984b0:295]). Boot default 2 [M `bootup` 0x1c7e78:25]; `set_difficulty` =
   the difficulty page's write [M `C_SBNFDFCTY_Handler` 0x216ca8:10-17]. Threaded through `Data`
   to the wired readers: the damage factor (`Player_HandlePain` 0x1902c8:29-48 via
   `bullet::damage_factor`), Bond→guard damage (`NDrone2_HitDamage` 0x145558:134-152), the hit
   chance (`DroneWeap_DoBulletAccuracy` 0x179740:81-95 via `level_accuracy`), the autoaim
   multiplier (`Check_AutoAim` 0x1a73f8:127-149), captains (`NDrone2_DefaultInit` 0x14b300:504-535),
   the door auto-unlock (`Lock_Create` 0x1ae1f8:67 — `lock.rs` already documented it), the fusebox
   and grenade pain paths. `Pickup_Handler` 0x18e650 and `Mission_Init` 0x182270 have **no**
   +0x40 access [M — checked], so armour/health pickups and the objective lists are
   difficulty-neutral at the code level; per-difficulty objective differences arrive through level
   scripts/tags (070-fix-mission's data, not forked).
4. **The save**: `Progress::encode_save/decode_save/save_to_file/load_from_file` — the game's
   data-file layout for the progress blocks: `{u32 id; u32 size; payload}` chained
   (`LS_GetSaveStuff` 0x1cd0d8's walk), LSB-first bit packing (`BIN_PushBits` 0x1e8b88), the CRC-32
   trailer (`crc32buf` 0x120670, zlib — `nf_formats::png::crc32`). MSSN = `LS_MakeMission`
   0x1cd720 (32 b nightfire status, 8 b count, per entry 32 b best + 4 b moments, 1 b game
   complete — 473 bits → 68 bytes, matching the snapshot's own save [L]); BUNS = `LS_MakeBonus`
   0x1ce3b0 (64 b `bonus[0]`). Deterministic: no timestamps anywhere in the format. PLRS/GSET/
   CHET/MPSG (the options blocks) are out of scope here — they belong with the menus/options work.

## How it was verified

- **Module tests** (progress.rs): the GetScore arithmetic on 0x09000005 (Σ raw 370000 from the
  [L] rows, the ×2 difficulty bonus, last = base + bonus), the status-6 stick rule (best kept,
  last zeroed), the medal bits + level stamp + game-complete, the tally gates (fire − scenery,
  the moment-once gate, the elapsed accumulation, the reset), the MSSN block size (68 [L]) and
  the save round-trip with CRC rejection of a flipped bit and a truncation.
- **`crates/nf-game/tests/progress_live.rs`** (NF_ISO): a guard's SG5 damage to Bond through the
  real AI path (`Drones::tick`) — 1.925 at the default difficulty 2 and 1.65 at Agent
  (2.75 × the [L] ReadTuningVars factors 0.7/0.6), sister ratio 0.6/0.7. Shown as the decompile's
  numbers before any code change? No — the factor functions pre-existed (bullet.rs) and were
  already correct; the NEW thing the test pins is that the difficulty is runtime (`sim.difficulty`
  + `set_difficulty` change the outcome) and that the default boots at 2 (the old const was 1 —
  that was the bug this task fixes).
- **`crates/nf-view/tests/fix_progress_live.rs`** (NF_ISO, the real input path): load 07000005,
  warm ticks, kill 2 guards through `step_bond`'s trigger (the P2K's semi-auto press pulse) and
  take a kind-6 bonus pickup → `dispatched == 2`, `shots_fired == 2`, `shots_hit_enemy == 2`,
  `bonus_pickups == 1`, `spawned >= 13` ([L] the live start count), `elapsed > 0`; the sister is a
  fresh reload with the zero tally. The save test writes/reads a real file.
- Full suite: `NF_ISO=$NF_ISO cargo test --offline --release -j2 -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1` — see the "Suite" line at the bottom (run before the
  final push).

## What's left (honest list)

1. **The detected counter has no live hook yet** (`PlrStat_LogEnemyDetectedPlayer` 0x199960 ←
   `DroneVision_LogSeenOpponent` 0x1780a8:36): the Log function and the counter are ported; the
   port's vision code has no single "logged a detection" site to hook (the game logs per
   detection event in the vision pass). Next task: find the port's detection event (the sight
   pass's "seen" transition) and call `log_detected` there; the tally test's detections row stays
   unexercised until then.
2. **`Player_InitWeapon`'s per-difficulty autoaim distances** (0x1a7ef8:24-39: 12/18, 8/14, 0/0)
   are not threaded — the port's auto-aim has no distance gate.
3. **The `NewScoresRef` PTP hand-off**: the on-foot (0x07-map) score rows come from a buffer
   inside the PTP region (0x31ca38) with no writer in ACTION.ELF — the doc's honesty note says
   "likely PTP/external or always empty on retail". The port models the NULL and aborts (the
   faithful retail behaviour if empty); if the main agent can confirm DRIVING.ELF fills it, the
   port needs that block's producer.
4. **The screens**: the results page (P_NFRESULTS), the pause stats (P_NFSTATS) and the medal
   banners are 180-fix-menus'; this branch gives them the API (`get_score` returns `Scored` with
   the medal; `progress.stats[0]` is the live tally; `has_medal`/`set_level_bonus`/
   `nightfire_status`/`save_to_file`).

## Shared files touched (minimal, per the rules)

- `crates/nf-game/src/drone.rs`: the `Drones.progress`/`Drones.difficulty` fields (own lines),
  the `Data.difficulty` field, the tick folds, the fire/hit/surrender/spawn/bonus hooks, and the
  DIFFICULTY-const → runtime replacement (its uses in pain/accuracy/captain/autoaim and
  `grenade.rs`'s `explode_propagate` parameter). No arms renamed, no constants repurposed.
- `crates/nf-view/src/scene.rs`: the `data()` difficulty mirror, the per-tick elapsed log, and
  the two `pain_generic` difficulty args (the const → field). Load-time `Data` uses the boot
  default 2 (the menu sets the real value before launch — 180-fix-menus threads it).
- `crates/nf-game/src/mission.rs`: one stale comment ("nothing to reset here") updated — the
  reset now exists.
- 12 test files: one `difficulty: 1` token per `Data { ... }` literal (the old const's value —
  no behaviour change; forced by the new field).

## Live checks for the main agent

1. **The accuracy row on 07000005**: fire 10 rounds at a wall, then at a guard; the pause stats
   screen (P_NFSTATS) should show the hits-only percentage (`PlrStat_LogShotHitScenery`
   0x198408's decrement). `gdbwatch 0x2bcc20 0x38 readwrite --seconds 20` during that run: the
   writers should be 0x1998d8/0x198408/0x199840 (and the reader 0x1984b0 at the results).
2. **The `NewScoresRef` question**: does DRIVING.ELF write the 9×3 s32 score rows at
   0x31ca38 (inside the PTP buffer 0x31c090..0x31d350) during the boot hand-off? If yes, dump the
   108 bytes after a boot — that decides whether the port's on-foot GetScore should read a PTP
   stand-in instead of aborting.
3. **The sp_level flags' meaning**: ee_start_05.bin shows all 12 `sp_level[i]+0x10` = 1 ([L]) on a
   snapshot taken ~8 s into 07000001 — is that a post-playthrough card (flags = completed) or does
   +0x10 mean "unlocked/available"? Menu_SetLevelBonus 0x1ffa10 sets the level's AND the next
   level's flag on completion, which reads more like completed+unlocked; a fresh-card boot
   (`Menu_SetNightfireStatus` default 0x3) would settle it.

## Render spots (no GPU here)

The results screen is 180-fix-menus' work; nothing in this branch draws. For the eventual results
page: the tallies are `progress.stats[0]` (rows 0-8 of `get_score` for the percentage rows), the
medal from `Scored.medal`, per P_NFRESULTS 0x208e78.

## Suite

`NF_ISO=$NF_ISO cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
-p nf-audio -p nf-view -- --test-threads=1` (2026-10-13 ~16:00 UTC): green except the 3 allowed
`disc_golden` failures on the slim ISO. Two mid-run breakages were found and fixed: the ai_live
harness now pins difficulty 1 (its measured columns came from the user's difficulty-1 sessions —
the runtime default is the game's 2), and fix_player4_live's outbox-flag assertions read the folded
counters (the tick's tally fold consumes the outboxes it exists to fold).

## ntfy

(sent after the push; see the finish steps)
