# The mission objective record (0x18 bytes) and OBJ_STATE (the 0x10-byte query result)

Proof levels: see [README.md](README.md). Records live in the `MissionData` table (base 0x2a4354,
24 slots × 0x28) — slot +0x00 = level hash, +0x04 = order number, +0x08 = pointer to the slot's
objective-record array, +0x0C = record count [M `Mission_MonitorObjectives` 0x1827d8:24-30 walks
exactly these; `Mission_Init` 0x182270:60-66]. The port's map: crates/nf-game/src/mission.rs (its
field names are confirmed below). Level: ee_start_05 (07000005 at start).

## The 0x18-byte record

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | the objective's text label (`Txt_BindLabel(*puVar10)`) | [M* read 0x1827d8:43,58,72-73,80; writer: the level's mission data (disc)] |
| +0x04 | u32 | the fail label; copied to `FailLabel` (global) when the record reaches state 5; 0xffffffff = none in every shipped record | [M* read 0x1827d8:87-89; write FailLabel 0x1827d8:88] |
| +0x08 | u32 | hint / long-description label: copied to `OBJ_STATE[3]` by `Mission_ObjectiveState`, never read by any ACTION.ELF draw path | [M1 copy 0x182630:44 (`param_1[3] = puVar4[2]`)] |
| +0x0C | u8 | the condition channel: `switch_channels[+0x0C]`, polarity by +0x10 bit 1. NO zero guard — channel 0 is the raw `switch_channels[0]` | [M* read 0x1827d8:33-37; init 0x182270:100-105] |
| +0x0D | u8 | the value `Mission_Init` resets the condition channel to (also stamps `switch_channels_time` when non-zero) | [M* 0x182270:100-105 (`switch_channels[+0x0C] = +0x0D`)] |
| +0x0E | u8 | not read by any function found | [M1 scan param_offsets.py OBJ_STATE + grep] |
| +0x0F | u8 | the enable channel: 0 → state 0 at init, non-zero → state 1 (waits for this channel) | [M* 0x182270:96-99 (`cVar1 = +0x0F` → state 0/1); read 0x1827d8:56-57 (state 1's reveal)] |
| +0x10 | u8 | flags: **bit 1 = the channel being ON means FAIL** (inverts the +0x0C polarity); bit 0 not read | [M* 0x1827d8:32-36 (`(bit 1) ? ch==0 : ch!=0`)] |
| +0x11..+0x13 | — | not read | [M1] |
| +0x14 | u32 | runtime state: 0 = pending (announces when the record's level is reached), 1 = pending with an enable channel, 2 = announced fail-objective (channel ON → fail: `switch_channels[99]`, 0x170 text, → 5), 3 = announced (channel ON → complete: 0x2000053 text, → 4), 4 = complete, 5 = failed (records its fail label) | [M* write 0x182270:97-99, 0x1827d8:47-51,65,76,83; read (switch) 0x1827d8:40] |
| +0x18 | — | end (stride 0x18: `iVar3 = uVar5 * 0x18` 0x182630, `puVar10 = ptr + uVar11 * 0x18` 0x1827d8:30) | [M] |

## OBJ_STATE (0x10 bytes, the `Mission_ObjectiveState` result)

Filled by `Mission_ObjectiveState(OBJ_STATE*, short vis_index)` 0x182630: walks
`MissionData` slots with `BaseMap`/`ThisOrderNum` gating, counts only records with
state > 1 || state < 0 (2, 3, 4, 5 — active/complete/failed), and indexes them by the
**visible** objective number:

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | the record's runtime state (`*param_1 = puVar4[5]`) | [M 0x182630:46] |
| +0x04 | u32 | the state's text id: 2 → 0x2000001, 3 → 0x2000000, 4 → 0x16f, else 0x170 | [M 0x182630:29-40] |
| +0x08 | u32 | the record's text label (`param_1[2] = *puVar4`) | [M 0x182630:42] |
| +0x0C | u32 | the record's +0x08 word (`param_1[3] = puVar4[2]`) | [M 0x182630:44] |

**The wrong-record lesson** (docs/fields/README.md's motivating bug): the channel lives in the
0x18-byte RECORD (+0x0C), not in OBJ_STATE — a reader that takes the channel from the query
result reads garbage. The port reads channels from its `Objective` records [M mission.rs struct].

## Port check

`crates/nf-game/src/mission.rs`'s `Objective` struct matches every field above (+0x00 text,
+0x04 fail, +0x08 hint, +0x0C channel, +0x0D init, +0x0F enable, +0x10 flags bit 1, +0x14 state)
and the state machine matches the switch. **No mismatch found** — this doc formalises the port's
reading with the full reader list. `Mission_MonitorObjectives`' standing fail hooks (0x60/0x61 →
FailLabel + channels[99]; channels 0x7B/199 level-specific) are also ported [M mission.rs doc].

## [L-read] spot checks (work/ee_start_05.bin)

07000005's MissionData slot: the record array pointer and count read from the snapshot match the
walk; the level's three objective records carry text labels 0x0400xxxx (bank 4) and states 3/3/3
at start [L — addresses and raw values in the session transcript; the port's c07000005.rs module
encodes the same records from the disc data].

## Live checks

1. Find the records live: `MissionData 0x2a4354` → slot for the loaded level (word 0 == BaseMap
   global) → +0x08 pointer → the 0x18-byte records.
2. `watch` record+0x14 while completing an objective — it should step 3 → 4 exactly when the
   completion message appears.
3. `poke` record+0x0C's channel (via `switch_channels`, address 0x26fc90 + ch) to 1 on an active
   record — the objective-complete banner should appear (state 3); on a fail-objective (bit 1 of
   +0x10 set) it should instead fail the mission (state 2 → 5, channels[99]).

## Fixed (fix-party-mission, 2026-10-13)

The banner side of this record is now verified in play on the port branch: the objective's text
label binds through the language table and the banner CARRIES it (probe pinned "Breach the castle
walls" / "Find a way inside to the party" / "Rendezvous with undercover agents…" on 07000007,
"Escort Mayhew to bunker" on 07000001, the 07000008 set — playtest item 5's "it just says new
objective" was the user's pre-fix-mission build). The NIS hold-off is ported too: while
`Script_CameraStart` 0x1d9140:41-43 has the viewers' HUD gate off (viewer+0x2a = 0 [M
Camera_Enable 0x1b9920]), `Text_UpdateMsg` 0x1b11d8:16-18 runs no timers and the pane
(`HUD_UpdateStatusPane` 0x19c310 case 0) stays closed — a banner queued mid-cut-scene waits and
opens when the scene ends. Test: `fix_party_mission_live.rs::the_07000001_banner_survives_the_intro_cutscene`
(fail-first: the 07000001 banner expired at tick 300 of a 2120-tick NIS — "no banner came up") and
`no_status_pane_draws_during_the_07000002_intro_cutscene`.
