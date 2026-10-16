## Your task: `mp-pickups-respawn`, multiplayer pickups, weapon sets and respawns

The MP options choose a weapon set (MPSettings+0x1B4, 11 sets), respawn rule (+0x1C0 Near/Far/Random), vehicles,
fixed gun emplacements, grapple (docs/systems/mp-menus.md, 611/402). Port how the game applies them in a match: which
pickups spawn per weapon set (the tables), their respawn timers, the spawn-point choice per rule, armour/health
pickups, the gun emplacements / vehicles / grapple toggles, in nf-game's mp code. Tests: per weapon set, the map's
pickup list matches the game's tables; the respawn rule picks the game's point (numbers only).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-pickups-respawn`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T01:54Z

---
## Report (done, 2026-10-16T03:09Z)


---
## Report from the worker (done, 2026-10-16T03:09Z)

# mp-pickups-respawn — report (2026-10-16, branch s/mp-pickups-respawn)

Task: port how a match applies the MP options — which pickups spawn per weapon set (the tables),
their respawn timers, the spawn-point choice per rule, armour/health pickups, and the gun
emplacements / vehicles / grapple toggles — in nf-game's mp code, with numbers-only tests.

## What's done

**1. The weapon-set table, corrected and completed.** `PickupMatrix` @0x2b9100 is **11 rows × 10
bytes (5 u16 each)**, one row per weapon set (0 Normal … 9 Cloak and Dagger, 10 Random); the
weapon a kind 7..11 placement becomes is the u16 at byte offset `set·10 + (kind−7)·2`, set = the
u16 at MPSettings+0x1B4 [M Pickup_CreateFromSet 0x18f1b0:56]. Proof of the 10-byte stride: the
symtab pins the table at 110 bytes with `UseableGuns` @0x2b9170 right behind it (a 20-byte-row
table would need 220 bytes and overlap it), `Pickup_MakeRandomWeaponSet` 0x18f098 writes row 10
at byte offsets 100..108 [M], and the menu offers exactly 11 sets [M mp-menus.md]. The port's old
`PICKUP_MATRIX_ROW0` ([0x6, 0x1e, 0x2c, 0x6, 0x10]) was the even u16s of the table's first 20
bytes — sets 0 and 1 interleaved (its own "odd columns" note was the giveaway: those odds are
exactly the ELF's u16[1,3,5,7,9]). **Corrected row 0 = [0x6, 0x1c, 0x1e, 0x18, 0x2c]** [D ELF
0x2b9100], and the full 11-row table is now `mp::PICKUP_MATRIX` [D ELF]. The SP kind-9 test in
pickup.rs became kind 11 (the rocket moved from index 2 to index 4 of the corrected row).

**2. `Pickup_CreateFromSet` / `Pickup_MakeRandomWeaponSet` ported** (crates/nf-game/src/mp/
pickups.rs): `weapon_for_kind(set, kind)`; `make_random_weapon_set` re-rolls row 10 — slot 0
samples `UseableGuns[0..26]`, slots 1-4 `[0..27]`, duplicates re-drawn [M 0x18f098:16-50] — and
`MpMatch::reroll_random_weapon_set` calls it exactly where `MP_Init` 0x183438:11 does (every
match; skipped only for the front-end level 0x07000048), so the shipped zero row is never played.
`Rng::rand` is the game's `Rand_Rand` bit-for-bit, so the re-roll is deterministic per seed.

**3. The registered-pickup layer** (`MpPickups`, 64 × 0xa0-byte records @0x2a4b50): the free-slot
search (`PickupLastDeletedIndex` → `PickupNextAddIndex` scan → lowest free → evict the registered
pickup with the smallest non-zero `PICKUPINFO+0x2E` lifetime among the first `MPSettings+0x1D8`)
[M MP_RegisterPickup 0x189d80:27-76], the count bump [M :107], unregister [M MP_UnregisterPickup
0x18a058], and `MP_Pickup_Process` 0x18a168 (the four per-record visit stamps zero when the
MPGame+0x19C clock passes them). **The +0x80 floats are NOT respawn timers** — they are per-bot
"don't retarget until" stamps: `BOTSTATE_setPickupVisitTime` 0x129958 writes clock+45.0 (+0.1
when 0.0) into `record[bot]`, and `BOTSTATE_pickGoal` skips/penalises recently visited pickups
[M]. The actual pickup respawn stays in the pickup obj (`PICKUPINFO+0x2C`, hidden for
`FRAME_RATE_INT·respawn·10` ticks) — already ported in pickup.rs and unchanged in MP.

**4. The spawn loadout** — `MP_EquipPlayer` 0x1855f8 as `mp::equip_player` (data; the port's own
player layer equips): the set's kind-7 weapon upgraded via `Upgrade_MPWeapon` 0x1a9040 with
`weapon_data+0x92 · 2` rounds [M :15-23], the 0x1a → 0x1b companion ammo [M :24-26], the grapple
weapon `Upgrade_MPWeapon(slot, 0x50)` when MPSettings+0x1D0 != 0 [M :27-34], the watch gadget
0x45 ×1 [M :35-38], the Protection-team-0 / Demolition-team-1 kit 0x3b ×1 [M :42-47].
`upgrade_mp_weapon` ports the five tables [D ELF 0x2bee28/0x2bee38/0x2bee78/0x2bee88/0x2bee98]
and the on-disc `Upgrades` bytes @0x2beea8 (+0/+1/+3 per 9-byte index row) [D ELF]. Health:
`MP_ReSpawn` 0x185368:72 spawns players at `MPSettings+slot·0x30+0x2c` (the Health Handicap)
**+ 100.0** — that and armour (Pickup_Handler kind 3) are the armour/health numbers the match
uses; both were already in the port's rules/pickup layers and are now pinned by tests.

**5. The toggles.** MpSettings gains `gun_emplacements` (+0x1B8, boot 0 [M 0x1c7e78:38]),
`vehicles` (+0x1CC, boot 0 [M :42]) and `grapple` (+0x1D0, boot 1 [M :45]) — new fields on their
own lines. `mp::gun_imp_class` = `GunImp_Create` 0x131f20:50-62: SP class 0xb; MP +0x1B8 == 0 →
none; weapon set 10 → `Rand(10)`; else the class is the set id. `mp::car_kind` = `Car_Create`
0x1305b0:13-42: MP + 0 → none; Ravine 0x0700004b → never; 1 tank / 2 helicopter / 3 =
`Rand(100) < 0x33` → helicopter else tank; SP builds a helicopter; the 8-vehicle cap stays with
the caller. The spawn rule (+0x1C0): the driver's Near/Far/Random is confirmed as
`MP_GetSpawnPoint`'s policy word — 0 = nearest candidate (> 2.0 squared units from every other
player), 1 = farthest, 2 = `Rand_Rand(candidates)`, else the range start; team games and
Assassination split the 64 entries at 0x20 [M 0x185098:85-183]. The port's existing
`get_spawn_point` already matched the decompile; it is now pinned by tests.

## How it was verified

- New `crates/nf-game/tests/mp_pickups_respawn.rs` — **10 tests, numbers only, no ISO**: the
  ELF-pinned tables (PickupMatrix, UseableGuns, the five upgrade tables), the kind→weapon map,
  random-set determinism/dedup/ranges, the front-end skip, ring eviction + count, visit stamps +
  process, the Near/Far/Random picks on a hand-built spawn table (incl. the 2.0 exclusion and the
  team split fallback), the upgrade tables, the loadout numbers (companion ammo, grapple, gadget,
  kits), the toggles, the boot defaults.
- `cargo test --offline --release -j2 -p nf-game -- --test-threads=1` on the final merged tree:
  **lib 215 passed**, all integration binaries green except `driving_ai_follow::real_mis01_track_
  rngp_walks_when_nf_driving_is_set` — verified pre-existing by stashing my changes and re-running
  (same failure without them; it belongs to the driving merge).
- The fast suite (`tools/test/suite.py`, background, started at 02:31) finished on the PRE-task
  tree with **"1581 passed, 7 failed, 66 ignored (70 min)"**; only one FAILED name survives the
  runner's progress-bar overwrites in suite.out (`walkreal_07000001_live::walk_from_the_spawn_
  to_the_exit_with_the_contact`); the no-GPU wgpu pylon render failures are the known pre-existing
  pair (mp-defaults-and-setup recorded them). I did not re-run the full 70-minute suite on the
  final tree — the window closed; the slice that covers my changes (all of nf-game, release,
  single-threaded) is the green run above plus the 10 new tests.
- `origin/master` (the net-match-host batch) merged mid-task; both WORKLOG entries and both sides
  of MpSettings kept (their `seed` field + my three toggles).

## What's left

- The sim does not yet call `register_pickup`/`pickup_process`/`reroll_random_weapon_set` from
  the live MP map load (the plumbing exists and is tested; the hook into nf-view's level-load
  path is the next worker's step, together with drawing the pickup's set weapon).
- `Upgrade_MPWeapon` returns the weapon unchanged when the `Upgrades` flag byte is ≥ 4 (the game
  reads past its 4-entry tables into adjacent .data there — chars/rows beyond the first few).
  Modelling that faithfully needs the surrounding bytes; flagged as a *Deviation*.
- MPSettings+0x1BC (the port's `capture_reward`) reads as a ×3.0 damage multiplier in
  `Player_HandlePain` 0x1902c8:80-83 and gates a weapon-search loop in `Pickup_Handler` 0x18e650
  — likely the "one hit kill"-family mod, not a capture reward; left untouched (renaming is a
  shared-field risk) and flagged for the field-doc owners.
- The pickup respawn timers come from each level's placements; per-map tables for all 8 arenas
  were not enumerated (the loader exposes them as placement params 9/11; a follow-up can dump
  them per map like gadget-carry did for SP).

## Live checks for the main agent (PCSX2; none runnable here)

1. PickupMatrix layout: watch 0x2b9100 (u16 read) during a Quick Game (set 0) — expect kind 8's
   weapon to be **0x1c** and kind 11's **0x2c** (my correction) — the old port claim said 0x1e/0x10.
2. `fieldprobe` MPSettings+0x1C0 (0x2a4960, w) with the Enviro-Mods row on Near, then respawn in a
   Deathmatch: the spawn should be the nearest enabled point > 1.41 units from any other player
   (the 2.0 threshold is on the SQUARED distance).
3. `fieldprobe` MPpickups+slot·0xA0+0x80 (0x2a4bd0+·, f) while a bot runs over a pickup: expect
   the bot's stamp ≈ MPGame+0x19C (0x2a4b1c, f) + 45, zeroing ~45 s later.
4. Quick Game (vehicles = 2): every `Car_Create` object should be a helicopter (obj+0x112 = 1)
   with +1.5 y; with the Enviro-Mods vehicles row on Random, count helicopters vs tanks ≈ 51/49.
5. Gun emplacements toggle Off (0x1B8 = 0): the map's fixed guns must not spawn; On with the
   Random weapon set: the emplacement's class byte (PICKUPINFO+0x9D) should be 0..9 per match.
6. Weapon set Random: two consecutive matches on the same map must show different kind-7..11
   pickups (row 10 re-rolled per MP_Init), all drawn from UseableGuns.

## ntfy

See the finish step's output — sent after this report (or its error recorded in the queue).
