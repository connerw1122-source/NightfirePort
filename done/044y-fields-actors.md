## Your task: `fields-actors`, the field map for the actor structs: every field, every reader, with proof

Scope: `Drone_tag` and `DCVars` (the drone instance and its control vars; the port reads many offsets, e.g. +0x4F8 flags, +0x2B4 killer, +0x504 alertness, +0x5A4 alt state, +0xCF8 scared radius), `DroneTypeSettings` (0x29aee8), the state-function table (0x29b300), `AnimObject` (`obj+0xDC`: +0x62/+0x63 weapon bytes, the LOD fade bytes), `HITDATA_tag`, and the spawner / ally / mission-fail fields `fix-actors` used.

**Why:** worker sessions have repeatedly read a field confidently and wrongly (a draw layer read as alpha, an objective
channel taken from the wrong record, a team gate the decompile contradicts). Each cost the user a playtest round. Read
`docs/fields/README.md` first: the proof levels, and the method the main agent used on the HUD sprite
(`docs/fields/sprite.md`, the worked example).

**The method, per struct:**
1. **Find every reader and writer of every offset.** Grep the whole decompile (`work/decomp/ACTION.ELF/*.c`) for the
   struct's access pattern (the base pointer's name varies, so search the offset with its typical forms: `+ 0x38)`,
   `[0xe]` and so on), then confirm each hit really is this struct (follow where the base pointer comes from). Where the C
   looks incomplete, check the disassembly. A field is **[M*]** only when every hit is listed in its row and they agree.
2. **Write disagreements down.** The same offset meaning different things for different object classes, a union, a
   reused byte: record each case and the condition that selects it. Never resolve a conflict silently.
3. **Cross-check with the EE snapshots** (`work/ee_start_*.bin`, file offset = EE address): find live instances and
   check the values make sense (**[L-read]**, with the address and the value).
4. **Check the port.** For every field the port already uses (grep `crates/` for the offset or the name), check the
   port's reading against your map. A mismatch is a bug: fix it with a test (fail first, with a sister assertion), or
   if it's big, describe it in the report.
5. **Write the live checks for the main agent.** For each field the port relies on for something visible or gameplay-
   relevant, give an exact probe the main agent can run in PCSX2:
   `python -I tools/trace/fieldprobe.py poke <addr expr> <width> <value> --shot out.png` (or `watch` / `diff`), how to
   find the instance live (the pointer chain from a global), and **what the screen or state should show** if your
   reading is right. If the game rewrites the field every frame, give the input that drives it instead (the
   sprite.md lesson). Put these in a "Live checks" section at the end of each doc.

6. **Ask for the live census.** The main agent can now record, in the running game, every instruction that reads or writes a field (`tools/trace/gdbwatch.py <start> <end> read|write|readwrite --seconds N`; worked example in `docs/fields/sprite.md`: `+0x38`'s three accessors). For each field the port relies on, give the census to run: how to find a live instance (the pointer chain from a global), the byte range, the access type, the game situation (the level and what the player does), and the accessor functions your static pass expects. A census that finds an accessor you didn't list means the static pass missed a reader; that's the point of it.

**Output:** `docs/fields/<struct>.md` per struct, in the format of `docs/fields/sprite.md` (offset, type, meaning,
proof with every reader/writer cited), and the coverage table in `docs/fields/README.md` updated with your structs.
No game bytes in the docs: offsets, values you read, names and addresses only.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fields-actors`.

---
claimed-by: sandbox1 at 2026-10-12T19:01Z

---
## Report (done, 2026-10-12T20:32Z)


---
## Report (done, 2026-10-12T20:32Z)

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
