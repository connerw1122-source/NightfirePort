# Level scripts (archive entry type 7)

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp / disassembly read here, [L] snapshot `work/ee_start_05.bin`, [D] bytes
on our disc (`nf-extract nis <iso> <level.bin> [hash]`). Reader: `crates/nf-formats/src/nis.rs`. Runtime:
docs/systems/script-player.md.

The timelines ScriptPlayers run (cut-scenes, the wine truck on 07000005, helicopters). Loaded by `Script_Load`
(0x1d8248) from the level archive's type-7 entries (hash `06xxxxxx`), cached per hash.

## Layout [M, D]

All little-endian, read with `BIN_GetWord` (u16) / `BIN_GetDWord` (u32) / `BIN_GetByte`.

```text
u16 magic 0x1C (anything else: the hash is marked bad)
u16 camera flag      ≠ 0 → SCRIPTINFO+0xAA8 |= 0x40 (cut-scene camera; ScriptCam)
u16 stream count     > 0x3E refused
u16 loop start       → +0xAAE: clock a looping script restarts at
u16 length           → +0xAB0: ticks
per stream:
  u16 first key, u16 last key  (absolute key indices; last < first = no keys)
  u32 size                     command bytes = size − 4, the next stream follows them
u32 key count  → +0x20A
u32 n, n bytes skipped
key count × 0x30-byte keys
```

Key (`Script_GetInterp` 0x1da158): `+0x00` vec4 position (w 1), `+0x10` quaternion (x, y, z, w), `+0x20` f32 scale,
`+0x24` f32 (×255 + 0.5 → `*param_8`, an alpha) or, on light streams, colour bytes `+0x24..+0x26`, `+0x28` f32,
`+0x2C` f32 time (ticks).

## Commands [M, D]

Each command is an opcode byte, its arguments, then one trailing byte (the paired opcode: 05 ↔ 06, others repeat
their own opcode — for stepping backwards). `Script_Run` (0x1d8548) skips the trailing byte after each handler.

| Op | Arguments | Handler |
|---|---|---|
| 04 | u16 time | `Script_StreamEnd` (0x1db420) |
| 05 | u16 time | wait: the stream's wait-until = time |
| 06 | u16 time | marker: past the script length → SCRIPTINFO flag 2 |
| 07 | u32 entity hash, u8 has parent, u8 parent stream, u8 spline, u8 object type, u8 n, n × u32 params | `Script_EntityStart` (0x1d8d78) |
| 09 | — | `Script_EntityEnd` (0x1dafe8) |
| 0A | u32 animation script, u32 skin, u32 start frame | `Script_AnimStart` (0x1d8f40) |
| 0C | — | `Script_AnimEnd` → `Script_KillStream` |
| 0D / 0F | camera | `Script_CameraStart` / `End` (not read yet) |
| 12 | u8 n, u8 event, n × u32 | `Script_EventHandler` (0x1d94c0) |
| 13 | fade | `Script_FadeStart` (not read yet) |
| 14 / 15 | sprite | not read yet |
| 16 | u32 sound, u8 2-D, u8 has object, u8 object stream | `Script_SoundStart` (0x1d8820) |
| 17 | — | `Script_SoundEnd` |
| 18 / 1A | light | not read yet |
| 1B / 1D | sub-script | not read yet |
| 1E | text | not read yet |

## 06000A22 (07000005, the wine truck) [D]

Length 5895, loop 0, 22 streams, 14 keys.

- Stream 0: `04` end at 5895 only.
- Stream 1: `0A` anim script 0x06000092 on skin 0x05000005 from frame 1 at t 0; `0C` at 5895.
- Stream 2: entity 0x0200040C, type 0xF5 (AI volume), parent stream 1, keys 2–3 (constant local offset).
- Stream 3: event 0x0C at 5890 (keep objects when streams end).
- Streams 4 / 5: entity 0x02000426, type 0x31 (plain object: the headlight corona), parent stream 1, keys 4–5 / 6–7.
- Stream 6: at 5384 entity 0x0200087F, type 0xEA (touch-once trigger), params (0x70, 8, 0…), keys 8–9 (world).
- Stream 7: entity 0x02000AC8, type 0xEB (touch trigger), params (0x71, 8, 0…), parent stream 1, keys 10–11.
- Stream 8: 1882–2312 entity 0x02000480, type 0xEA, params (0x5C, 8, 0…), keys 12–13 (world, at the gatehouse).
- Stream 9: event 0x0D at 5412: switch channel 0x73 = 1.
- Streams 10–21: 3-D sounds 0xFF–0x111 on stream 1's object over the drive.
