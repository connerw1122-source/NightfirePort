# CamSubject (the photograph target) — NO instance (0 bytes at obj+0x110); class 0x48

Proof levels: see [README.md](README.md). Instance size from `CamSubject_Create` 0x1bc5d8:
`control_create_object(0, …)` [M 001bc5d8:11] — zero-size; obj+0xE0 points one past the
allocation and no code reads through it. Placement type 0xFD. Level: none of the three available
snapshots (ee_start_05, ee_start_07000001, ee_snapshot) contains a class-0x48 object [L census],
so this struct is **[M] + [D-placement] only** — the live rows below are the census itself.

Readers/writers census: grep `CamSubject` over work/decomp/ACTION.ELF → only the creator, the
update, the dispatcher case 0xFD, index.tsv. `param_offsets.py obj_tag` lists exactly one extra
taker of the object: `CamSubject_Update` (obj+0xD0, +0xF6, +0xFE) — none other. The class's
control_funcs row: update = CamSubject_Update 0x1bc668, no collide, no delete (object.md).

## Layout (all offsets are obj_header offsets — there is no instance)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0xD8 | ptr | glist (the placement's target object — what the camera "photographs"; the contact geometry the collision pass registers) | [M* 001bc5d8:17] |
| +0xF0 | u32 | = glist+0x44 flags (set directly, not via Control_SetGList) | [M* 001bc5d8:18-20] |
| +0xF6 | u16 | **the channel the photograph raises** = placement param 0 (level_tag+0x2C, u16 load). Read by the update as a **signed** s16 index into switch_channels | [M* 001bc5d8:22, 001bc668:18] |
| +0xFF | u8 | class = 0x48 | [M* 001bc5d8:19] |
| +0xD0 | ptr | HITDATA contact list — walked every tick | [M* 001bc668:9] |
| +0xFE | u8 | bit 0 set by the update **after the first successful photograph — the object deletes itself** (one photograph per level load) | [M* 001bc668:19] |

`CamSubject_Update` 0x1bc668 in full [M 001bc668]: walk obj+0xD0 (entry next +4, other obj at
entry+0x58); for each other object of class 5 (bullet, `*(char*)(iVar1+0xff) == 5` :16) whose
**weapon id is the camera gadget** — `*(short *)(*(int *)(*(int *)(iVar1 + 0xe0) + 0x44) + 2) ==
0x54` :17, i.e. bullet instance+0x44 → weapon record, +2 = weapon id — raise
`switch_channels[obj+0xF6] = 1` (unconditional, even when +0xF6 is 0) and set `obj+0xFE |= 1`.
No position, damage, or LOS test: *any* camera-bullet contact photographs.

The bullet-side fields this leans on (shared with Destroy): bullet instance+0x44 = weapon record
ptr; record+2 = weapon id (0x54 camera here, 0x67 launcher in Destroy_CollisionHandler);
record+0x10 = the weapon's damage-type bitfield u16 (read by `Collide_FilterBullets` 001ece30:26).

## Param map (from the creator)

| level_tag param | read as | "Instance" offset |
|---|---|---|
| 0 (+0x2C) | u16 | obj+0xF6 (channel) |

Params 1+ are never read. The placement transform/glist carry everything else.

## Port mismatches (crates/nf-game/src/props.rs — report only)

1. **Channel width** — game stores the param 0 **u16** into obj+0xF6 and indexes switch_channels
   with the s16 (`*(undefined2 *)(iVar3 + 0xf6) = *(undefined2 *)(param_3 + 0x2c)` 001bc5d8:22;
   `switch_channels[*(short *)(param_1 + 0xf6)] = 1` 001bc668:18). Port:
   `channel: param(0) as u8` (props.rs:94). A placement with param 0 ≥ 0x100 would disagree
   (none known: 0x1c on 0700000c #414/#454, 0x1e on 0700000d #494/#495 [D]).
2. **Photograph count** — game: the first camera contact raises the channel and the object marks
   itself for deletion (`+0xFE |= 1`), so exactly one photograph per object per load, but the
   channel write happens on **every** contacting camera bullet until the delete lands (next
   movement pass). Port: `s.hit = true` latches after the first segment hit and stops testing
   (props.rs:121-126) — one channel write total. Equivalent outcomes; the game can double-write
   within the same tick.
3. **Contact model** — game: the collision pass fills obj+0xD0 for whatever overlaps the placed
   glist; the update tests only class + weapon. Port: bullet flight segment vs the target's
   transformed bounds box, weapon byte carried by the caller (props.rs:120-126) — the documented
   stand-in; a camera bullet that grazes the box edge can disagree with the glist contact.
4. **Channel 0** — with obj+0xF6 = 0 the game literally writes switch_channels[0] = 1; the port's
   `ch.set(0, …)` is a no-op (triggers.rs:82 `if c == 0 … return`). If any placement ever ships
   param 0 = 0 the game raises a channel the port ignores. No such placement known.

No disagreement on: class 0x48, zero-size instance, param 0 → channel, weapon-0x54 gate, the
one-shot self-delete semantics.

## [L-read] spot checks (all three snapshots)

No class-0x48 object exists in ee_start_05.bin (07000005), ee_start_07000001.bin (07000001,
569 objects) or ee_snapshot.bin (495 objects) [L census walk: head 0x2705a0 → +0x14 chain,
class byte obj+0xFF histogram — 0x48 absent everywhere]. The port's two live users are on
0700000c/0700000d placements [D], for which no snapshot exists in work/. Nothing to read live.

## Live checks (for the main agent)

Load 0700000c (jetpack display). Find the class-0x48 object: walk `DynamicObjList 0x2705a0 →
u32@head+0x14 → obj+0x14 …` until obj+0xFF == 0x48; its channel = u16 at obj+0xF6 (expect 0x1c).

1. **Photograph** — shoot the display with the camera gadget: `watch <obj>+0xfe` should flip bit 0
   (self-delete) the tick the shot lands, switch_channels+0x1c (`0x26fcac`) becomes 1, and the
   object disappears from the list after the next delete pass. A non-camera weapon must do
   nothing (weapon-id gate).
2. **One-shot** — a second camera shot after the delete must find no object: the channel never
   re-raises.
