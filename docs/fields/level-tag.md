# level_tag (the per-placement parameter block, 0xB0 bytes, on the dispatcher's stack)

Proof levels: see [README.md](README.md). `level_tag` is NOT the map-file placement record
(`TARG23_PLACEMENT`): `parsemap_block_map_data_dynamic` 0x1d1b68 expands each placement into a
0xB0-byte `level_tag` on the stack (`memset(auStack_130, 0, 0xb0)` 0x1d1b68:64) and passes it to
the creator (`parsemap_create_dynamic_objects(placement, level_tag, flags, cel, path1, path2, u8)`
0x1d1b68:95-96). Every creator then reads it as its `param_2`/`param_3` (`P9level_tag` in the
mangled signature). Created per placement, never stored — the struct lives only for the create
call. No game bytes below; offsets and values only.

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u16 | never written by the parser (memset 0), no reader found — padding | [M1 0x1d1b68:64,77] |
| +0x02 | u16 | **key count** = `placement+0x40` (u16): the 0x19 key-list entry count for animated objects; creators read it as a count (`SS_Create` gates the whole object on it, `Trigger_Create` reads it as a trigger count, `Hurt_Create`/`Sub_Create`/`ThirdCam_Create`/`Sensor_Create` as `count−1`) | [M* 0x1d1b68:91 (`uStack_12e = puVar8[0x20]`); SS 0x1a9c58:15; Trigger 0x1b1720:62-66; Hurt 0x17e870:26-27; Sub 0x1ad6c0:25; ThirdCam 0x1bc510:15; Sensor 0xaf010:82-86] |
| +0x04 | u16 | **rate** = `placement+0x42` (u16): the key-playback divisor; `SS_Create` divides the key count by it (default 0x4B = 75 when 0 — it WRITES the default back: the only writer outside the parser) and `Trigger_Create`/`Hurt_Create` compute `count · REC_FRAME_RATE · (FRAME_RATE / rate)` | [M* 0x1d1b68:90; SS 0x1a9c58:23-24,51; Trigger 0x1b1720:85-86; Hurt 0x17e870:50] |
| +0x08 | u32 | **key-list pointer** = `m_paths[placement+0x44]` (the 0x19 framelist record): the animation key data; `SS_Create` stores it (`puVar2[3]`), `Wire_Create` reads it as +0x08, `Sub_Create`/`ThirdCam_Create`/`Sensor_Create`/`Hurt_Create` copy it into their instance | [M* 0x1d1b68:88-89 (`uStack_128 = *(undefined4 *)(puVar8[0x22] * 4 + m_paths)`); SS 0x1a9c58:27; Wire 0x1981d8 — reads it via the scan; Sub 0x1ad6c0:24; ThirdCam 0x1bc510:16; Sensor 0x1af010:81; Hurt 0x17e870:25] |
| +0x0C..+0x2B | — | never written by the parser (the translation/rotation the creators receive as params 1/2 point at the DISPATCHER's stack copies `uStack_c0`/`uStack_b0`, not here) | [M1 0x1d04f0:50, 0x1b1720:97] |
| +0x2C+4k | u32 | **param k** — the sparse key/value pairs expanded: each pair is 8 bytes in the placement at +0x4C+8k (key i32 → slot index, value i32), `auStack_104[key] = value` (0x1d1b68:69-75); readers use `+0x2C+4k` as u16/u8/u32/f32 per creator | [M 0x1d1b68:74] |

Size 0xB0 = 0x2C + 33 params [M memset 0xb0 0x1d1b68:64; Drone_Create amemcpys the whole block
`param_3 + 0x2c, 0x84` into its DIVars at +0xC34 — 0x84 = 0xB0−0x2C, 0x136be0:59,81].

## Per-creator param map

`python3 tools/fields/param_offsets.py level_tag` lists every direct reader (48 functions); the
scan misses creators that copy the pointer to a local first (e.g. `Door_Create` hands it to
`Door_SetupSwing` 0x135440). pK = level_tag+0x2C+4K. "—" = the creator reads no params (it works
from the placement transform and its glist).

| Placement type | Creator | level_tag reads |
|---|---|---|
| 0x0F | Drone_Create 0x136be0 | **p2 = the minimum-difficulty tag** (skip unless `p2 <= GameState[0x40]`, 0x136be0:49 — the atlas's "placement+0x34" is this level_tag offset; the old "spawn frame" name here was a misread — the gate is the selected difficulty, verified live on 07000011: rosters 20/33/37 at d1/2/3); the whole block is copied to DIVars+0xC34 for the drone AI |
| 0x20 | Break_Create 0x12bfd8 | p0, p1, p2 |
| 0x21 | Ripples_Create 0x1acc90 | p0, p1 (shorts) |
| 0x22 | Ladder_Create 0x1966c0 | p1 → inst+0x02 (no reader found — docs/fields/ladder.md) |
| 0x27 | GT_Create 0x17ffa0 | p0..p5 |
| 0x28 | Sensor_Create 0x1af010 | header +2/+4/+8, p0..p8 |
| 0x29 | Switch_Create 0x1adcc0 | p0→+0x12, p1→SP script, p2→+0x10, p3→+0x16, p4→+0x19, p5→+0x14, p6→SSysItems gate (docs/fields/switch.md) |
| 0x2B | LeafGen_Create 0x1acbd0 | — |
| 0x2C | Lightning_Create 0x18cde8 | p0 (u32), p1 (u16), p2 (u16) |
| 0x2E | Lock_Create 0x1ae1f8 | header +2/+4/+8, p0..p5 |
| 0x2F | Copter_Create 0x17d340 | header +2/+4/+8, p0..p11 |
| 0x31 | FuseBox_Create 0x1b0448 | p0 = SP script hash, p1→+0x54, p2→+0x56, p3→+0x5C, p4→+0x5E (docs/fields/fusebox.md) |
| 0x33 | Hint_Create 0x1b3248 | p0..p3 |
| 0x34 | GunImp_Create 0x131f20 | p? (reads via a local; quaternion placement +0x24 per atlas) |
| 0x35 | PCQWorm_Create 0x1b07a8 | p0 (u16), p1 (u16), p2 (u8), p3 (u8) |
| 0x36 | Sub_Create 0x1ad6c0 | header +2/+4/+8, p0 |
| 0x37 | MiniSub_Create 0x182130 | p0 |
| 0x38/0x39 | Create_SpaceLaser 0x1aa8b8 / SpaceMissile 0x1a9ef0 | p0..p7 / p0..p11 |
| 0x3A | Grapple_Create 0x198298 | — |
| 0x3B | DynamicObject_Create 0x18dbf8 | p0 (short), p1 (short), p2 (short), p3 (u8) |
| 0x3E | Wire_Create 0x1981d8 | p0→inst+0x02, p1→inst+0x00, p2→inst+0x04, p3→inst+0x06 (docs/fields/wire.md); header +8 = the wire's anim is NOT used here (the 0x19 list is the grapple's swing path? [S]) |
| 0x3F | CreepWall_Create 0x196fc8 | p0..p3 (u16s) |
| 0x41 | InverterTrigger_Create 0x1b3040 | p0, p1 |
| 0x4A | Apocalypse_Create 0x18dcc0 | p0 (u16), p1 (u32 ·60.0), p2 (u16), p3 (u8), p4 (u8) |
| 0x71 | rotor_init (see object.md dispatch) | — |
| 0xD2 | Shooter_Create 0x17d030 | p0→obj+0xF4, p1→obj+0xF6 |
| 0xD4 | Cloud_Create 0x18d620 | p0 (f32), p1 (u32 ·0.001), p2 (u32 ·0.001), p3 (f32), p4 (u16) |
| 0xD5 | RainBox_Create 0x18d840 | — |
| 0xD8 | RB_Create 0x1d6bf0 | p0, p2, p3, p4 |
| 0xDC | Trigger_Create 0x1b1720 | header +2/+4, p0 = kind, p1..p11 (docs/fields/trigger.md) |
| 0xDE | Searchlight_Create 0x1cb4f8 | p0 (u16), p1 (u8), p2 (u8), p3 (u8), p4/p5 (u32 min/max pair) |
| 0xE1 | Destroy_Create 0x12c318 | p0..p6 (docs/fields/destroy.md) |
| 0xE2 | SS_Create 0x1a9c58 | header +2/+4 (writes the default rate back!), +8, p0..p9 (docs/fields/ss.md) |
| 0xE3 | Flicker_Create 0x1acb00 | p0 (u16), p1 (u32) |
| 0xE4 | Hurt_Create 0x17e870 | header +2/+4/+8, p0..p5 |
| 0xE7 | ThirdCam_Create 0x1bc510 | header +2/+8, p0 (u16), p1 (u8) |
| 0xE8 | Trigger_LoadLevelCreate 0x1b2950 | (tail-calls Trigger_Create; LOAD_LEVEL reads p0/p2/p3/p4 — docs/fields/trigger.md) |
| 0xEA-0xEF | Trigger_Touch… 0x1b2aa8..0x1b2ed8 | (tail-calls Trigger_Create with a forced kind byte; same params) |
| 0xF0 | Pickup_Create 0x18dda0 | params arrive as SCALAR ARGS (type, id, respawn, model, colour, …) — the level_tag is not read |
| 0xF2 | Emitter_CreatePlist 0x17b848 | p0..p5 |
| 0xF3 | Env_Tree_Create 0x17c200 | p0, p1 |
| 0xF9 | SoundTrigger_Create 0x1b2628 | p0..p8 |
| 0xFA | ThirdIcon_Create 0x197070 | p0 (u32), p1 (short), p2 (u16) |
| 0xFB | MusicTrigger_Create 0x1b30e0 | p0, p1, p2, p3 |
| 0xFD | CamSubject_Create 0x1bc5d8 | p0 → obj+0xF6 (the photograph channel, u16 — docs/fields/camsubject.md) |
| 0xFE | Mine_Create 0x17ce28 | p0..p5 |
| 0x24/0x2D | Player_AddNewStartPos 0x195c00 | (MP start positions; level_tag passed through) |
| 0x25/0x26 | MP_RegisterSpawnPoint / MP_RegisterMPObject 0x185990 | p0..p5 (switches p0/p3 word pairs at 0x185990:133-135) |
| 0xDF | Env_Create 0x17b968 | p0..p11 |
| 0xE0 | Creature_Create 0x134ab8 | p0..p4 |
| 0xD9 | SP_CreateScriptPlayer 0x1db9a8 | (script stream, not params) |
| 0xDB | Door_Create 0x136100 | via `Door_SetupSwing` 0x135440 (docs/fields/door.md: p0→obj+0xF6 kind, p1→group, p2→open ch, p3→lock ch, p4→portal bit, p5-7 sounds, p8\|p9→flags, p10/p11 bytes) |
| 0xFC | Corona_Create 0x18da00 | — (param_3 never dereferenced [M 0x18da00]) |

## Reader census summary

- Header +0x02/+0x04/+0x08 readers: SS, Trigger, Hurt, Sub, ThirdCam, Sensor, Copter, Lock,
  Wire (+8 only) [M listed above]. Every other creator leaves them alone.
- The only writer outside the parser is `SS_Create` (+0x04 ← 0x4B when 0) [M 0x1a9c58:23-24].
- No reader of +0x00 was found in any of the 48 direct-reader functions [M scan
  tools/fields/param_offsets.py level_tag].

## Port check

`crates/nf-formats/src/placement.rs` models the placement record (params as key/value pairs);
`Level` exposes `p.params` — the port's creators read `param(k)` = level_tag+0x2C+4k, matching
the expansion [M per-creator docs above]. The port's `Placement.extra` (used by pickup_live and
ss.rs as `extra[7]`/`extra[8]`) is the RAW placement view (+0x40/+0x44 words), consistent with
key count / path id [M 0x1d1b68:88-91]. No mismatch found in the header handling; the per-creator
param meanings are cross-checked in each struct's draft.

## Live checks

`level_tag` is a stack temporary — not probed live. The thing to verify live is the EXPANSION:
1. Break on `parsemap_block_map_data_dynamic` (0x1d1b68) at level load with a debugger and dump
   `auStack_130` (0xB0 bytes) for a known placement — +0x02/+0x04/+0x08 must equal the placement's
   +0x40/+0x42 words and `m_paths[id]`.
2. Poke-free check: a `--bond` view of an SS object (docs/fields/ss.md's live checks)
   exercises +0x02/+0x04/+0x08 end-to-end (key count, rate, key-list pointer → the posed model).
