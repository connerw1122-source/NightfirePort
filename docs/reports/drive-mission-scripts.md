# Report: `drive-mission-scripts` (the driving missions' objectives and scripts)

Branch `s/drive-mission-scripts` (from origin/master 0f5f07f4), 2026-10-16. Sandbox window
01:46–03:45 UTC. Task: decode the driving missions' script data from the `.VIV` archives and
port the mission state machine (objectives + completion, fail states, timers, the PTP end)
into `nf_game::driving`, headless, with scripted-event tests.

## What's done

1. **The mission script chunks are decoded** — `docs/systems/drive-mission-scripts.md` is the
   protocol doc. The mission logic lives in the track `.crp` inside each mission `.VIV`: the
   `Map ` pool record's children `Rule`/`MSet`/`Trgr` [M FUN_0022faf8 0x22faf8:108-129,187-194].
   Measured MIS01/paris_mis01 [D]: `Rule` = 114 × 0x20-byte entries, `MSet` = 760 B
   ("helicopter"), `Trgr` = 754 × 0x40-byte records.
2. **The rule format is mapped** [M]: `FUN_00205220` 0x205220 walks 0x20-byte entries; the
   entry is `{type u32, flags u32 (3 bits), group i32, hash u32, payload 16 B}`; the group
   buckets at mgr+0x24+g*4. `FUN_00209390` 0x209390 is a 15-way type switch whose arms carry
   the object sizes and per-type ctors; the class names came from the GCC typeinfo strings at
   0x397c28.. [D]: SRuleAlways/Ammo/Damage/Death/Health/PIP/Prog/Property/Range/Speed/Timer/
   Difficulty/PlayerDir/Collision/ProgCounter. The MIS01 type histogram is measured in the
   live test (41 SRuleProg, 28 SRuleSpeed, 14 SRuleDeath, 14 SRuleTimer, …).
3. **The objective model is mapped** [M]: `FUN_00205498` 0x205498 registers ≤ 0x20 channels
   from the MSet table (enabled byte +0x2c+i, target +0x4c+i*4 with the 0x2711 gate, params
   +0xcc/+0x14c+i*4) and activates the first enabled non-zero one through `FUN_0020d488`
   0x20d488; channel entries are stride 0x54 (`FUN_0020d378` 0x20d378: +0x14 enabled bit,
   +0x18 target, +0x1c/+0x20 progress, +0x34 param). MIS01's channels: {2 (target 30),
   26 (target 24)} [D]. The no-MSet fallback's fixed count-99 channel sets are ported too
   [M 0x205498:11-29]. `FUN_00206ec0` 0x206ec0's damage/health thresholds and per-difficulty
   percent tables (MSet +0x27c..+0x2a8) are modelled.
4. **The state machine is ported** — `crates/nf-game/src/driving/mission.rs`:
   `MissionSet::parse`, `RuleEntry::parse`, the `RuleKind` enum (15 classes with the ctor
   sizes), `Trigger` (minimal), and `Mission` — bucketed rules, the channel tracker, timers,
   the fail path, and `drive_return()` mapping to the PTP machinery ([M] crates/nf-game/src/
   ptp.rs: Success → result 1/status 0 → `Mission_SetStatus(6)`; Fail → 0/4 → status 3).
   The per-type firing semantics are [S] (the descriptor functions FUN_0020b120…0020b3f8 are
   not decomposed — every boundary is listed in the doc §5). Supporting format additions:
   `crp::pool_find` + `crp::record_window` (the `Map ` child walk), additive only.
5. **Tests, all green**: 9 unit tests in the module (layout round trips, a scripted
   3-channel run to complete with exact event sequences, fail/success PTP mappings, a
   2-second SRuleTimer firing at tick 120, the no-MSet defaults, death/difficulty matching,
   PIP hash requests) + 5 NF_DRIVING-gated live tests over the real archives
   (`driving_mission_live.rs`): the pinned MIS01 numbers, the type histogram, channels
   {2, 26}/targets {30, 24}, all 8 missions' chunks parse (MIS01/MIS11/MIS13A/B/C/MIS3/
   MIS4/RACE), a headless scripted run over the real paris rules (complete reported; the
   fail path asserted), trigger stability.

## What ran (before the last push)

- `cargo test --offline --release -p nf-formats --lib -- --test-threads=1` → **79/79 passed**
  (the 3 crp unit tests included).
- `NF_ISO=… cargo test --offline --release -p nf-game --lib -- --test-threads=1` → **221/221
  passed** (the 9 new `driving::mission` tests included; nothing pre-existing broken).
- `NF_DRIVING=… cargo test --offline --release -p nf-formats --test driving_golden` → **9/9
  passed** (the crp pool/record additions against the byte-exact goldens of all 89 `.crp`s).
- `NF_DRIVING=… cargo test --offline --release -p nf-game --test driving_mission_live
  -- --test-threads=1` → **5/5 passed**.
- The full-suite fast runner (`tools/test/suite.py`) could **not** run: this sandbox reaps
  background jobs between tool calls (started once, dead on the next call), and the
  foreground window went to the research + implementation + the per-crate runs above. The
  3 slim-ISO `disc_golden` allowance does not apply here (nothing ISO-dependent changed).
- Touched crates: **nf-formats** (crp.rs additions — lib + goldens green), **nf-game**
  (driving/mission.rs + the live test — lib green). Nothing in nf-view/nf-audio was touched.

## What's left

- The per-type descriptor functions (§7 of the doc: FUN_0020b120/0020b1a0/0020b218/…) — the
  real firing semantics per rule class; the port's are [S] and the tests pin only the model.
- The `Trgr` consumer `FUN_0022cbe0` (0x40-byte record's +0x10..0x40 semantics).
- The within-frame rule order and the mid-frame advance semantics (doc §5 boundaries).
- Live checks L-1..L-4 (doc §7) — the main agent's PCSX2 pass would settle all of them.
- The mission-per-frame integration with `nf_game::driving`'s car step (the module is
  standalone-headless by design; the car health input comes from the physics layer).

## Live checks for the main agent

See docs/systems/drive-mission-scripts.md §7 (L-1..L-4: the SRuleAlways descriptor write,
the channel progress counter, the second objective activation arg = 26, the trigger parse).
