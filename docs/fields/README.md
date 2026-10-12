# The field map

What every field of the game's runtime structs means, and **how sure we are**. Started 2026-10-12 after several
confident misreadings by worker sessions (a draw layer read as alpha, an objective channel read from the wrong record,
a team gate that the decompile contradicts), each of which cost a playtest round.

## Proof levels (every field row carries one)

| Tag | Meaning | Enough for the port to rely on? |
|---|---|---|
| **[S]** | suggested: a name or guess, from one context or another project | no |
| **[M1]** | one reader or writer in the decompile, read by hand | no, for anything visible or gameplay-relevant |
| **[M*]** | **every** reader and writer found (Ghidra cross-references on the offset, listed in the row) and they agree | yes, for internal state |
| **[L-read]** | the value read live in PCSX2 (or an EE snapshot) is what the reading predicts, in a known game state | yes, together with [M*] |
| **[L-verified]** | **poked or driven live and the predicted effect seen** (screenshot or state change), with the probe command recorded | yes: required for anything the player sees |

Rule: a field the port relies on for something visible or for gameplay must reach **[M*] + [L-verified]**. A
disagreement between readers is written in the row; it's never resolved silently.

## The live probe

`tools/trace/fieldprobe.py` (PINE; PCSX2 running with the level loaded, nobody playing, no other PINE client):
`dump`, `find`, `ptrs`, `watch`, `poke ... --shot` (pins the value during the screenshot, then restores it), `diff`, `shot`.
Every reader and writer, measured live: `tools/trace/gdbwatch.py` with the GDB-server PCSX2 (docs/tools/pcsx2-gdb.md).
Lessons so far:
- **Fields the game recomputes every frame can't be poked directly.** Poke their input instead. The HUD sprite layer
  (`+0x38`) is rewritten by `HUD_UpdateAmmoPane` each frame; poking the switch timer `BL+0x95E` that drives it works.
- **Timers count down under you.** Pin the value while the screenshot is taken (`poke` does this).
- **Get the address right.** Dump the target first. The probe prints the original value, and a surprise there means
  the wrong address (the first poke hit a neighbouring sprite).

## Coverage

| Struct | Doc | Fields mapped | [L-verified] | Notes |
|---|---|---|---|---|
| sprite (runtime HUD sprite) + HUD pane list | [sprite.md](sprite.md) | 14 | 3 | started 2026-10-12 |
| HUDPANE_tag + HUDPANECREATE_tag + all 26 `*SprInfo` tables | [hud-panes.md](hud-panes.md) | 15 + 26 tables | 2 | fields-player 2026-10; stride 0x2C everywhere, BSS fillers found |
| BLData (the player's 0x970-byte block) | [bldata.md](bldata.md) | ~110 | snapshot-cross-checked throughout | fields-player 2026-10; port fixes: the pain-type armour gate, the fall/drown pipeline, the clip regen |
| weapon_data record (0x10C × 115) | [weapon_data.md](weapon_data.md) | ~40 | 3 | fields-player 2026-10; +0x60 retyped u16, +0xCC/+0xD8 are script hashes |
| ammo_data (34 × 12) | [ammo-data.md](ammo-data.md) | 4/record + the weapon→ammo map | full dump | fields-player 2026-10 |
| PlrStat_* / ScoringTable / RewardsTable | [player-stats.md](player-stats.md) | 14 + 11 + row layout | 4 | fields-player 2026-10; the whole system is unported (documented) |
| obj_tag class-3 (the player) | [obj-tag-player.md](obj-tag-player.md) | ~25 | snapshot cross-checked | fields-player 2026-10; corrected the +0xCC root-height story; class 3 ↔ 0x12 |
| Drone_tag (the AI/drone record, obj+0x110, 0xD20 bytes) — the +0x4F8 bit census, +0x2B0/+0x2B4, the alertness block, +0x5A2/+0x5A4, the secondary offsets | [drone.md](drone.md) | 32 bits + 21 fields | 0 (live pass owed) | 044y-fields-actors 2026-10-13; drove the port fixes listed in the doc |
| DCVars_tag + StateMachineInfo_tag (Drone+0x108) + the SM message protocol | [drone-statemachine.md](drone-statemachine.md) | 4 + 11 + 20 msg ids | 0 (live pass owed) | 044y-fields-actors; obj+0xE0 = Drone_tag*, SMI embedded |
| DroneTypeSettings (0x29aee8, 85×12) + NDrone2_StateFuncs (0x29b300, 250 entries) | [dronetype-tables.md](dronetype-tables.md) | 4 row fields + 250 states | tables read from 4 snapshots [L] | 044y-fields-actors |
| AnimObject (obj+0xDC, 0x120 bytes): weapon bytes +0x62/+0x63/+0x64, LOD fade bytes +0x110..0x113 | [animobject.md](animobject.md) | 20 | 0 (live pass owed) | 044y-fields-actors; character alpha is obj+0x103..0x106, NOT the fade bytes |
| HITDATA_tag (0x60) + the spawner block (obj+0xE0) + the ally/mission-fail fields | [hitdata-spawner-ally.md](hitdata-spawner-ally.md) | 15 + 12 + 12 | spawner offsets [L ee_start_05] | 044y-fields-actors; the spawner p7 arm-vs-kill disagreement is written up with a decisive live check |

## Coverage — the world-object structs (fields-world, 2026-10-12)

Worker task `fields-world` (branch s/fields-world): the generic object header, the placement
parameter block, and every per-type instance struct. Proof levels as above; the [L-verified]
count is 0 everywhere — these docs are the static pass ([M]/[M1]/[M\*]), the main agent's live
pass (fieldprobe/gdbwatch, each doc's "Live checks" section) comes next.

| Struct | Doc | Instance | Notes |
|---|---|---|---|
| obj_tag (generic header) | [obj-tag.md](obj-tag.md) | 0x110 @ obj+0 | reader-list status per row; class-3 variant: obj-tag-player.md (s/044x-fields-player) |
| level_tag (placement params) | [level-tag.md](level-tag.md) | 0xB0 stack | +2 key count, +4 rate, +8 key-list ptr; per-creator param map for all 48 readers |
| trigger | [trigger.md](trigger.md) | 0x40, class 0x1D | port fixes: raw-0DC shift removed (3 live 0xDC on 0700000c) |
| door (DOORINFO) | [door.md](door.md) | 0x80, class 0x1C | atlas door.md corrections (Activate doesn't read +0x66) |
| pickup | [pickup.md](pickup.md) | 0x40, class 0x2F | respawn/state-2, kind 5, MP registry still open |
| switch | [switch.md](switch.md) | 0x1C, class 0x39 | port fixes: inhibit-gate polarity, busy one-shot latch |
| SS (SimpleScript) | [ss.md](ss.md) | 0x2C, class 0x25 | channel is an OUTPUT (SS_Operate); port gate doc wrong |
| fusebox | [fusebox.md](fusebox.md) | 0x60, class 0x3C | port fixes: hurt iff +0x56==0, +0x5C clear |
| script player + entities | [script-player.md](script-player.md) | 0x3C, class 0x1B | SSTREAM base erratum (+0xA8) for atlas/script-player.md |
| searchlight (4 part kinds) | [searchlight.md](searchlight.md) | class 0x22 | kill path deletes wrong parts in port; break gate; LOS origin |
| corona | [corona.md](corona.md) | 0 bytes, class 0x01 | no instance; runtime class-0x45 corona has no creator |
| CamSubject | [camsubject.md](camsubject.md) | class 0x48 | channel width u16 vs port u8 |
| Destroy | [destroy.md](destroy.md) | 0x18, class 0x21 | not modelled by the port at all (missing feature) |
| ladder | [ladder.md](ladder.md) | 4 bytes, class 0x2D | instance is 2 bytes of scratch; the game reads the obj header |
| wire | [wire.md](wire.md) | 8 bytes, class 0x2C | port agrees; ride channel +0x06 held while riding |
| objective record (0x18) + OBJ_STATE | [objective.md](objective.md) | MissionData @ 0x2a4354 | the channel lives in the RECORD +0x0C, never in OBJ_STATE |
| glist (celglist_tag) | [glist.md](glist.md) | 0x50 in pCelList | +0x44 → obj+0xF0 copy; +0x40 id (Destroy's swap watch) |
| leaf / obj colour bytes | [leaf.md](leaf.md) | obj+0x100..0x106 | deep GS walk stays in docs/atlas/gs-leaf.md |

**Port fixes this task** (all fail-first, see docs/reports/fields-world.md): fusebox hurt
polarity + +0x5C clear (fusebox.rs), switch inhibit-gate polarity + busy one-shot latch
(useobj.rs), raw-0DC trigger param shift (triggers.rs); tests in-crate plus
`crates/nf-game/tests/fields_live.rs` (NF_ISO).

Static passes for the remaining structs are queued as worker tasks (`docs/fields/<struct>.md`, [M*] level); the main
agent then runs the live pass for the fields the port relies on.
