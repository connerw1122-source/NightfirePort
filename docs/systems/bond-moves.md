# Bond moves — the 007 bonus

The "Bond moves" (the 007 bonus: the HUD 007 icon + the chime, counted in Mission Stats' Bond
Moves row and the 007 Bonus score) are awarded per level for specific actions. Evidence tags:
[M] ACTION.ELF decompile (address), [D] the retail disc data, [L] a live PCSX2/port measurement.

## The award path (code)

- **`Trigger_Calc` 0x1b1900:12-20** [M]: every tick, for switch channels **0x65..=0x6E
  (101..110)** that are non-zero, calls `PlrStat_DoneBondMoment(0, ch)`. The bond-move channels
  ARE switch channels 101-110; anything that sets one awards.
- **`PlrStat_DoneBondMoment(slot, id)` 0x199448** [M]: `slot < 8`; in SP the mission gate
  (`Mission_Status() ∈ {0,1}`, MP exempt via `MPSettings+0x180`); then, only when
  `BondMoments[id−0x65] == 0` (0x2bcde0, 10 × s16): flag = 1, `PlrMissionStats+0x20 += 1`
  (0x2bcc20 + slot·0x38), and **`glb_blokes[slot]+0x948 = 200`** — the HUD flash timer.
- **`PlrStat_LogBondBonus` 0x1999e0** [M]: `PlrMissionStats+0x24 += 1` (the 007 Bonus pickups,
  `Pickup_Handler` case 6 0x18e650:235) — a separate counter, not the moments.
- **The HUD flash**: `HUD_UpdateHealthPane` 0x19b9c8:232-260 [M] counts `BL+0x948` down by
  `FRAME_RATE_MUL` (1.0 at the port's fixed 60 Hz — GS_SetRefreshRate 0x17d2b8:15 = 60/FRAME_RATE;
  [L 0x30d0d8] = 1.0 in both EE snapshots) per pane update. While the timer is up the icon sprite
  is drawn (its +0x38 draw layer = 0 — the sin-bounce scale runs 200→101, a static ~last scale
  below 101; docs/fields/sprite.md:30 — +0x38 is the draw layer, 0xFF = not drawn, set only at
  timer < 1). Nothing else reads or writes +0x948.
- **The chime is level data, not code**: a **SoundTrigger** placement (type 0xF9,
  `SoundTrigger_Create` 0x1b2628 / `SoundTrigger_Update` 0x1b2790 [M]) whose ON channel is the
  bond channel plays the level's sting the tick the channel rises.
- **Mission Stats' Bond Moves row** = `PlrMissionStats+0x20` (GetScore row 0 0x1984b0 [M];
  `docs/fields/player-stats.md` has the record layout and the pause handler's read
  `C_GCPAUSE_Handler` 0x204408:447-498).
- Reset: `PlrStat_ResetForMission` 0x1993a0 memsets both the record and BondMoments [M].

Port side: `crates/nf-game/src/progress.rs` (`PlrMissionStats.done_bond_moment`, `Progress`),
the per-tick award fold `crates/nf-game/src/drone.rs` (`Trigger_Calc`'s scan + the +0x948 write),
`crates/nf-game/src/sound.rs` `SoundTriggers` (0xF9 placements), and `crates/nf-game/src/bullet.rs`
`PlayerHealth.bond_moment_flash` (BL+0x948, decayed 1/tick next to the pane's other timers).

## The sound ids (the chime family)

ACTION.ELF name→id table [D, the `{name*, 0, id, 0}` entries ~0x249c48]:
`SFX_BOND_MOMENT_SHORT_01` = 0x1dc (476), `MEDIUM_01` = 0xfd (253), `LONG_01` = 0x116 (278),
`LONG_02` = 0x115 (277), `BM_TENSE` = 0x117 (279), `BM_CAS_BEAM` = 0x14b (331), `BM_CAS_TRUCK` =
0x14c (332), `BM_CAS_WALL` = 0x14d (333), `BM_CAS_ZIP` = 0x14e (334), `BM_STAB3` = 0x14a (330).
The SoundTriggers placed on bond channels use the CAS_* family on the early levels and 65535
(no object) where a level awards silently.

## Every story level's bond moves (census [D])

Setter key: which placement/script raises the bond channel. `AND`/`SEQ` = trigger type 0xEC/0xED
(out = raw param 0), `Touch`/`TouchOnce` = 0xEA/0xEB (out = raw param 0; raw p1 = the contact
mask). SoundTrigger params: (gate, one-shot, sound id, 2-D?, off-channel, global). The census
tool: `cargo run -p nf-view --example census_bond_moves` (NF_ISO), scratch output.

| Level | Bond ch | Raised by [D] | Chime SoundTrigger [D] |
|---|---|---|---|
| 07000002 | 101 | SEQ_AND map 51 #396 (in 61, 62) | #435 gate 101 → 333 |
| 07000002 | 102 | AND map 51 #388 (in 126, 127, 128); TouchOnce #389 in-list | #436 gate 102 → 333 |
| 07000002 | 103 | SEQ_AND map 51 #405 (in 139, 140) | #437 gate 103 → 335 |
| 07000002 | 106 | SEQ_AND map 51 #403 (in 137, 138) | #434 gate 106 → 333 |
| 07000003 | 104 | SEQ_AND map 50 #472 (in 131, 132) | #490 gate 104 → 333 |
| 07000003 | 105 | SEQ_AND map 50 #474 (in 133, 134) | #491 gate 105 → 333; #81 gate 105 → 332 |
| 07000005 | 101 | AND map 50 #316 (in 112, 113, 114 — the truck-ride park: plates 112/113 script-created by 06000a22 streams 6/7, 114 drone-only Touch #318) | #82 gate 101 → **331** (BM_CAS_BEAM) |
| 07000005 | 105 | AND map 50 #80 (in 127, 128, 129, 133 — the four courtyard plates #311-#314) | #81 gate 105 → **332** (BM_CAS_TRUCK) |
| 07000007 | 102 | AND map 47 #430 (in 76, 77, 78) | #447 gate 105 → 0xffff (none); gate-102 trigger has no 0xF9 on the census (silent or script-side) |
| 07000008 | 104 | AND map 51 #102 (in 90, 91, 92) | #106 gate 104 → 333 |
| 0700000a | 103 | SEQ_AND map 37 #755 (in 126, 127, 128) | #756 gate 103 → 333 |
| 0700000b | 101 | TouchOnce map 50 #357 (mask 8, lock 13) | #288 gate 101 → 333 |
| 0700000c | 101 | Touch map 45 #478 (mask 8) | #541 gate 101 → (id on the census line) |
| 0700000c | 102 | Touch map 45 #476 (mask 8) | #563 gate 102 → (id on the census line) |
| 0700000d | 103 | script 060009ef stream 2 Entity Touch out 103 (mask 8) | #516 map 49 gate 103 → (id on the census line) |
| 07000011 | 102 | Touch map 43 #235 (mask 8) | #286 gate 102 → (id on the census line) |
| 07000014 | 101 | AND map 54 #324 (in 209, 34, 111) | #325 gate 101 → (id on the census line) |
| 0700000a | 102 | AND map 47 #430's family (see census) | #447-style (see census) |

Levels with NO bond channel in their archives [D census]: 07000001, 07000003 part-maps beyond
the two above, 07000004, 07000006, 07000009, 07000012, 07000013, 07000015, 07000016, 0700001b —
plus every bond SoundTrigger the census lists under the level's other maps. The full raw census
(per map, per placement, ids included) prints from the census example; the table above is the
per-level summary. MP archives (07800007/07900007/07F00007) were not censused (SP-only counters
gate on the mission status; DoneBondMoment's MP arm exists but the MP UI is out of scope).

## Live checks for the main agent

1. On 07000005 (the truck ride or any award), `gdbwatch` on `BondMoments` 0x2bcde0 (expect the
   awarding id's halfword 1) and `PlrMissionStats+0x20` 0x2bcc40 (expect +1), and on Bond's
   BLData+0x948 (expect 200 at the award tick, then −1 per frame).
2. The award moment on PCSX2 should flash the 007 icon by the health pane and play the CAS
   family sting; the port's draw for the icon is NOT ported (no drawing in worker tasks) — the
   state (`PlayerHealth.bond_moment_flash`) and its countdown are.

## What remains

- The HUD draw of the 007 icon (reads `sim.drones.health.bond_moment_flash`; the game's sprite
  animation is `HUD_UpdateHealthPane` 0x19b9c8:239-257: scale `(200−t)·0.01`, the sin bounce on
  the y-scale, hidden below 101).
- The pause SCORE tab's shots/accuracy/time still fill 0 (pre-existing gap, unchanged here).
- The per-level "what the player does" table (the chains behind channels 61/62/112/113/127-129…)
  is one more hop of census + playtests.
