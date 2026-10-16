# The driving missions' script data — `Rule` / `MSet` / `Trgr` and the mission state machine

Status: **first working port** (task `drive-mission-scripts`, 2026-10-16). The mission logic of
the driving levels (MIS01.VIV … RACE.VIV) lives in the track `.crp` inside each mission archive:
the `Map ` pool record's `Rule`/`MSet`/`Trgr` children. The headless state machine is
`crates/nf-game/src/driving/mission.rs`; the container walk is `nf_formats::crp`. Evidence tags:
[M] code (function + address, DRIVING.ELF, EE = file offset + 0x107080), [D] disc bytes,
[S] suggested. Nothing asserted from memory.

## 1. Where the data is

- The mission archive opens by level key (`paris_mis01` → `MIS01.VIV` [M FUN_00175a28
  0x175a28]); the track entry `data\track\<key>.crp` RefPack-decompresses to a flat record pool
  + chunk directory (docs/formats/crp.md).
- "Init World" `FUN_0022faf8` 0x22faf8 finds the **`Map `** record in the pool and resolves its
  window's children: `Wmap`, `Rule`, `MSet`, `Audi`, `Trgr`, `Envi` [M 0x22faf8:108-129,187-194;
  print "Resolving misc world elements" 0x39b628].
- Measured over MIS01.VIV / `paris_mis01.crp` [D]: `Rule` = 114 × 0x20-byte entries (3648 B),
  `MSet` = 760 B, `Trgr` = 754 × 0x40-byte records (48256 B), `Audi` = 0 B, `Wmap` = 160 B.

## 2. The `Rule` chunk — the mission rules

`FUN_00205220` 0x205220 walks the payload as **0x20-byte entries** and builds one runtime
object per entry via `FUN_00209390` 0x209390 — a **15-way type switch**. The base ctor
`FUN_00209318` 0x209318 maps: obj+0 = entry+0xc (name hash), obj+4/8/0xc = entry+4's bits
0/1/2, obj+0x10 = the per-type descriptor. The entry's group id (entry+8, i32) buckets the
object at mgr+0x24+g*4 (mgr = `DAT_0033a024`); g < 0 → the default list mgr+0x20 [M
0x205220:27-56].

| entry offset | meaning | proof |
|---|---|---|
| +0x00 | rule type 0..=14 (≥ 0xf → dropped, the switch `default`) | [M 0x209390] |
| +0x04 | 3 flag booleans (bits 0/1/2) | [M 0x209318:15-17] |
| +0x08 | group/channel id (i32; −1 = default bucket) | [M 0x205220:29-51] |
| +0x0c | name hash (`f3 02 6c 65`-shaped in MIS01) | [M 0x209318:14] |
| +0x10..0x20 | type payload | [M per-type ctors] |

The type names are the GCC typeinfo strings at 0x397c28.. [D]; sizes + ctors from the switch
[M 0x209390]:

| type | class | obj size | MIS01 count | payload notes |
|---|---|---|---|---|
| 0 | SRuleAlways | 0x14 | 2 | no payload |
| 1 | SRuleAmmo | 0x24 | 0 | |
| 2 | SRuleDamage | 0x24 | 4 | |
| 3 | SRuleDeath | 0x1c | 14 | |
| 4 | SRuleHealth | 0x28 | 4 | |
| 5 | SRulePIP | 0x20 | 0 | |
| 6 | SRuleProg | 0x18 | 41 | |
| 7 | SRuleProperty | 0x28 | 0 | |
| 8 | SRuleRange | 0x28 | 0 | |
| 9 | SRuleSpeed | 0x24 | 28 | |
| 10 | SRuleTimer | 0x24 | 14 | |
| 11 | SRuleDifficulty | 0x1c | 0 | |
| 12 | SRulePlayerDir | 0x24 | 0 | |
| 13 | SRuleCollision | 0x18 | 5 | |
| 14 | SRuleProgCounter | 0x20 | 2 | |

The type descriptors (obj+0x10) carry three function pointers each at +0xc/+0x14/+0x1c [D
0x397b40..0x397c08]. Reading them: the +0xc slot is a **typeinfo accessor** (base →
FUN_0020b120 = `FUN_0024e7c8(0x3c8358, 0x397c28 "12SMissionRule")`; SRuleAlways →
FUN_0020b1a0 = the 0x397c38 "11SRuleAlways" typeinfo) [M] — the descriptors are C++ RTTI
objects, not vtables, so the per-frame rule evaluation is dispatched elsewhere (the manager
walk / live check L-1) and stays undisassembled.

## 3. The `MSet` chunk — the mission set and the objective table

`FUN_00204d18` 0x204d18 reads the set name (MIS01: `"helicopter"` [D]); when the chunk is
absent the name defaults to one of `lotus`/`playervan`/`playerlimo`/`playertaxi`/`playersmart`
by `DAT_0033a010..020` (the player-car choice) [M]. `FUN_00205498` 0x205498 registers up to
0x20 objective channels:

| MSet offset | meaning | proof |
|---|---|---|
| +0x00..+0x2c | the name area | [M 0x204d18] |
| +0x2c+i | enabled byte per channel | [M 0x205498] |
| +0x4c+i*4 | target count (skipped when ≥ 0x2711) | [M 0x205498] |
| +0xcc+i*4 | param 2 | [M 0x205498 → 0x20d378 arg 4] |
| +0x14c+i*4 | param 3 | [M 0x205498 → 0x20d378 arg 5] |
| +0x27c/+0x280/+0x284/+0x288 | health/damage thresholds (0 → the 10000.0/1000.0 defaults) | [M 0x206ec0:12-21, the 0x461c4000/0x447a0000 stores] |
| +0x294 | difficulty-scaling gate | [M 0x206ec0:52] |
| +0x298..+0x2a8 | per-difficulty percent tables (÷100 scalers) | [M 0x206ec0's mgr+0x3b8 ∈ {1,2,else}] |

MIS01's enabled channels: **2 (target 30) and 26 (target 24)** [D]. The no-MSet fallback
registers fixed count-99 channels {0x15, 0x16, 3, 7} (or {0..2, 0xe, 10, 0xb, 3, 0xf} when the
PTP key matches string 0x397540) [M 0x205498:11-29].

The active objective is set through `FUN_0020d488` 0x20d488 (display+4 = index): the first
enabled non-zero index, else 0 [M 0x205498's iVar5 scan]. Channel entries are stride 0x54 off
display+0x10: +0x14 |= 1 (enabled), +0x18 += target, +0x1c/+0x20 = progress, +0x34 = param
[M 0x20d378].

## 4. The `Trgr` chunk — triggers (minimal decode)

754 records of 0x40 bytes in MIS01. The leading four f32s are finite positions + a fourth
(radius/height) [D]; +0x18 is a u32 whose high half is 0x656c (`le`) on most records [D]. The
consumer `FUN_0022cbe0` 0x22cbe0 (class `WTrigger` [D]) is not decomposed — the port stores
the records raw and exposes pos/hash only.

## 5. The ported state machine and its [S] boundaries

`Mission::load` mirrors `FUN_00205220` + `FUN_00205498` + `FUN_00206ec0` (buckets, channels,
thresholds, timers). `Mission::step(dt, SimSnapshot)` (fixed 60 Hz [M 0x2d6b90's table])
evaluates the active channel's rules against a scripted sim snapshot. Per-type firing
semantics are [S] except where a test pins them; the descriptor functions to decompose next
are listed in §7.

The mission end maps through the existing PTP machinery: complete → `DriveResult::Success` →
`drive_exit` writes result 1 / status 0 → `Mission_SetStatus(6)`; fail → result 0 / status 4 →
status 3 [M crates/nf-game/src/ptp.rs, 0x174bf0:315-323 + 0x1c8560:12-20].

**[S] boundaries (explicit):** (1) the per-type evaluation order inside the descriptor
functions FUN_0020b120/0020b1a0/…/0020b3f8; (2) which fired rule advances vs completes a
channel (the port: every fire bumps done, done ≥ target completes); (3) the mid-frame advance
semantics (the port: the frame evaluates the channel active at frame start); (4) the trigger
field semantics beyond the positions; (5) whether SRuleTimer expiry fails or merely fires (the
port: it fires its channel like any rule).

## 6. Verification

- 9 unit tests (`nf-game` lib, `driving::mission`): the entry/MSet layouts, a scripted 3-channel
  run to complete with exact event sequences, the fail path + PTP mapping (result/status 0/4 →
  state 3), the success mapping (1/0 → state 6 + `key_for_index`), a 2-second SRuleTimer firing
  at tick 120, the no-MSet fixed channels, death+difficulty matching, PIP hash requests.
- 5 live tests (`nf-game/tests/driving_mission_live.rs`, NF_DRIVING-gated): the measured
  MIS01 numbers (114×0x20 / 760 B / 754×0x40), the type histogram, channels [2,26] targets
  [30,24], all 8 mission archives parse (`MIS11`/`MIS13A/B/C`/`MIS3`/`MIS4`/`RACE` included),
  a headless scripted run over the real paris rules, trigger stability.

## 7. Live checks for the main agent

- **L-1**: breakpoint FUN_00209d98 0x209d98 (SRuleAlways's descriptor +0x14) during a paris
  drive — confirm it runs every frame and what it writes into the display entry (the +0x1c
  progress increment site would settle §5's boundary 2) [M expectation].
- **L-2**: watch the display channel entry for objective 2 (display+0x10 + 2*0x54 + 0x1c)
  during play — expect +1 per checkpoint/progress event, completion at 30 [D target 30].
- **L-3**: breakpoint FUN_0020d488 0x20d488 — record the arg when the second objective
  activates (expect 26) [D channels {2, 26}].
- **L-4**: breakpoint FUN_0022cbe0 0x22cbe0 — dump the parsed trigger fields against the raw
  0x40-byte records to settle the +0x10..0x40 semantics (the `01 01 07 04` bytes and the
  1.15-ish float at +0x14).
