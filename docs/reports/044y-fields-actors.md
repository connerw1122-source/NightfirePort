# 044y-fields-actors — report

Branch `s/044y-fields-actors` (from origin/master a99dd44). Done, with the live pass owed to the main agent.

## What's done

**The field maps** (docs/fields/, all readers/writers cited with function + address; disagreements written down,
none resolved silently; README.md's coverage table updated):

- **drone.md — `Drone_tag`** (obj+0x110, 0xD20 bytes; obj+0xE0 points at it): the +0x4F8 flag-word census —
  534 unique statements over 244 files, every bit named (32 bits + the multi-bit masks); +0x2B0 = the
  hostage↔killer partner pointer (not the opponent); +0x2B4 = last-hit-by (obj_tag*); the alertness block
  +0x500 base / **+0x50C live** / +0x504 alt-profile; +0x5A2 start/return vs +0x5A4 one-shot alt; ~20 secondary
  offsets (+0x14C hit-effect multiplier, +0x19 captain, +0x135/+0x13A channels/tag, +0x4D8/+0x4E8 behaviour
  blocks, +0xAC drone health, +0xBE0 special-death spot, +0xC48 selector, +0xD1C bot ptr = struct top).
- **drone-statemachine.md — `DCVars_tag` + `StateMachineInfo_tag` + the msg protocol**: DCVars is a 0x10-byte
  stack temp {obj, drone, obj+0x20 copy (no reader), SMI*}; SMI is embedded at Drone+0x108 (instance u16 counter
  at 0x299090 [L], current/prev/queued +0x04/+0x08/+0x0C, impact resume +0x10, entered +0x14, pending +0x18,
  channel arg +0x24, fn ptr +0x28 = 0x175778); 20 message ids pinned (msg+0x04 is a state filter that DROPS —
  [M-dis 0x171e28-44]).
- **dronetype-tables.md — `DroneTypeSettings` (0x29aee8, 85×12) + `NDrone2_StateFuncs` (0x29b300, 250 entries)**:
  row layout start(+0, u16 → Drone+0x5A2) / alt(+2 → +0x5A4) / init fn(+4) / control fn(+8); rows 0/3/11/20/26
  start −1 (row 3 is −1, not 0x56 — actors.md corrected); rows 30..84 the MP-bot identity block; all 250 state
  entries resolved to named decompiles, all byte-identical across the four snapshots [L]; 88 have port
  constants, every value matches.
- **animobject.md — `AnimObject`** (obj+0xDC, 0x120 bytes): +0x62 held weapon / +0x63 queued / +0x64 previous,
  the per-viewer LOD fade bytes +0x110..0x113, +0xC8 flag word, head/aim blocks. **Character alpha is
  obj+0x103..0x106, NOT the fade bytes** — flagged against the sprite-0x38-style misread. No port mismatches.
- **hitdata-spawner-ally.md — `HITDATA_tag` (0x60) + spawner + ally/mission-fail fields**: full HITDATA map with
  the bullet→drone trace; the spawner block (all nine params pinned [L ee_start_05]); the ally-route fields
  (+0x854/+0x9F2/+0x970, +0x12C wait word, +0x2B8 armed-peer) and props 0x53/0x54 behind +0x4D8
  (behaviour_util_getProperty 0x1c7d98). **Disagreement kept open**: spawner param 7 — fix-actors' playtest says
  ARM, the Control 0x138014 read says the +0x20 latch routes to the cleanup block; offsets agree, semantics get
  the decisive live check written in the doc.

**The port fixes** (each fail-first-tested; doc entries updated with the commit):

1. `ally.rs::on_init_death` corner-exact to `DroneFunc_OnInitDeath` 0x145bd0: the prop-0x54 + non-drone-kill arm
   re-checks prop 0x53 (unset → NO fail [M LAB_00145cc8 / LAB_00145c3c]); prop 0x53 + a drone killer → reason 2
   (team 1) / 1 (team 2/3) [M the bVar7 gotos]; the KO check opens the team-1 arm only [M 0x145bd0:20-22] (the
   caller passes `knocked_out`).
2. Every state change clears BOTH message timers and resets the hit-effect multiplier +0x14C to 1.0 [M-dis
   0x17207c-88] — the port left timer D armed across transitions (drone.rs `send`).
3. `AllyLeadMissionWait` latches its wait channel from `Drone+0x12C` (0 → 0x99) [M 0x15efc8:17-20], not from the
   second switch channel +0x135 (new `mission_wait_channel` field models +0x12C).
4. Exposition corrections from the census: the alertness block slots, in-air 0x4000 (no setter anywhere — the
   port's flag documented as its own model), cover 0x800/0x1000, HostageIdle 0x0d vs AllyLeadInit 0x1c, the
   0x86/0x88 names, GoToGoal's return slot +0x5A2 (was "+0x5A6").

## How it was verified

- Static: greps over all 3038 decompile files (offset + statement extraction, de-duplicated, classified
  SET/CLR/TEST/READ), every hit's base pointer followed to its source; [M-dis] raw-MIPS checks where the C looked
  incomplete; [L] reads from the four 32MB EE snapshots (the two tables, the instance counter, the spawner
  block, live drone values).
- Tests: `ally::tests::oninitdeath_corners_match_the_decompile` and
  `fields_actors_live::a_state_change_clears_both_message_timers` were both shown failing BEFORE the fixes
  (exact assert lines in the log), then passing after; the 4 fix_actors_live NF_ISO regressions (the protected
  guest, the spawner channels, the escort cower, the assassin) stayed green.
- Full suite: `NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio
  -p nf-view -- --test-threads=1` — every binary green except the 3 allowed bigf/disc_golden failures on the
  slim ISO.

## What's left

- **The live pass** (main agent, PCSX2): each doc ends with a "Live checks for the main agent" section —
  fieldprobe/gdbwatch recipes with pointer chains, byte ranges, situations and expected screens. The two
  highest-value ones: (1) the **decisive spawner-p7 check** (07000005: fire ch 0x43 after ch 0x22's wave — arm
  vs kill settles fix-actors' playtest against the Control read); (2) the **missing-setter censuses** for
  `0x4F8` bits 0x1 and 0x4000 (a census hit outside the listed accessors = the static pass missed a reader).
- Documented, not fixed (need bigger changes, listed in the docs): the msg state-filter and the monotonic
  instance counter in `drone.rs::send`; the death-flag bytes +0x3B/+0x3D (the port's `firing=false` covers the
  visible part); the roll/dodge writers of +0x14C (those states are not ported); self-msgs deferred vs the
  game's nested dispatch.
- The byte-0x304 hook of DroneTypeSettings has no writer in ACTION.ELF (unresolved in dronetype-tables.md);
  Drone+0x12C has no writer either (modeled as init-0; if a live census finds one, the MissionWait model needs
  the writer's source).
- Cross-lane note: docs/systems/actors.md §1 has two wrong rows (row 3 start −1, not 0x56; the GetDTYPENEW
  "→ 0x10" gloss) — left for actors.md's owner, corrected here in dronetype-tables.md.

Shared-file touches: `crates/nf-game/src/drone.rs` (the state-change fixups, one field + comments), `docs/WORKLOG.md`
(append-only entry), `docs/fields/README.md` (coverage rows) — minimal, listed per the rules.
