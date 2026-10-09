# Dynamic objects at level start (07000005): what the port does not draw yet

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp, [L] snapshot `work/ee_start_05.bin`, [S] suggested. Source: a
research pass (subagent, 2026-10-09); not yet re-checked here. **Status: spec only** (the objects / scripts subsystem
is planned after animation and rendering).

## Framework [M, L]

- `parsemap_block_map_data_dynamic` (0x1d1b68) → `parsemap_create_dynamic_objects` (0x1d04f0) only for placements with
  `flags & 0xA000` = 0 (0x8000 cels and 0x2000 templates are not created). Placement +0x40 / +0x42 / +0x44: path
  count, rate, `m_paths` index.
- Live list `DynamicObjList` 0x2705a0 (head +0x14, next `obj+0x14`); per type `control_funcs` 0x26f890 (update,
  collide, delete; 12 bytes). Snapshot: 523 objects (13 drones, 8 ScriptPlayers, 5 doors, 49 triggers, 29 searchlight
  parts, 4 pickups, 2 copters, …). `View_AddObjects` 0x1e62d8 skips `obj+0xF0 & 0x8010`; `obj+0xFC` ≠ 0 = drawn.

## Missing, by priority

| What | Source | Creator / update |
|---|---|---|
| **Wine truck** (9 rigid parts + 2 headlight coronas) | script `06000A22` (type-7 entry, 1856 bytes) played by ScriptPlayer placement #37 (type 0xD9, p0 = script, p2 = 1) | `SP_CreateScriptPlayer` 0x1db9a8 / `SP_Update` 0x1dbda8 → `Script_AnimStart` 0x1d8f40 |
| Snow (4096 flakes) | 0xDF `SNOW_gen` ×2 + 0x32 `Effect_Snow_*` ×18 | docs/systems/render.md |
| Searchlights (7 mounts, 7 heads, 6 beams, 7 coronas, 2 spots) | 0xDE ×29 | `Searchlight_Create` 0x1cb4f8 / `Searchlight_Update` 0x1cb900 |
| Coronas | 0xFC ×13 | `Corona_Create` 0x18da00, `Corona_Update` 0x18da78 |
| Doors (5), level-exit gates (2) | 0xDB, 0xE8 | `Door_Create` 0x136100 / `Door_Update` 0x1363c8; `Trigger_LoadLevelCreate` 0x1b2950 |
| Helicopter | 0x2F ×2 (+ ScriptPlayers `06000046` / `06000047`) | `Copter_Create` 0x17d340 / `Copter_Update` 0x17dbb8 |
| Ladder, zip wire, grapple pulleys, placed pickups | 0x22, 0x3E (rotation z negated), 0x3A, 0xF0 | `Ladder_Create` 0x1966c0, `Wire_Create` 0x1981d8, `Grapple_Create` 0x198298, `Pickup_Create` |

## The truck [M, L]

- Type-7 script format (`Script_Load` 0x1d8248, `Script_Run` 0x1d8548): header u16 magic 0x1C, flag, stream count,
  loop start, length; per stream u16 first / last key, u32 size, command bytes (opcode, args, trailing reverse
  opcode: 05 / 06 wait, 04 end, 07 EntityStart {u32 hash, u8 hasParent, u8 parent stream, u8 flag, u8 objType, u8 n,
  n × u32}, 0A AnimStart {u32 anim script, u32 skin, u32 start frame}, 09 EntityEnd, 0C AnimEnd, 0D camera, 12 event,
  16 / 17 sound); then u32 key count, u32 n + n bytes, keys of 0x30 bytes {vec4 pos, quat, scale, 3 × f32, time};
  interpolation linear + slerp / spline (`Script_GetInterp` 0x1da158). Clock + `FRAME_RATE_MUL` per tick.
- Stream 1: AnimStart 0x06000092 (sequence 0x04000009 in 07F00005.bin, 5896 frames, skeleton 11) on skin 0x05000005
  (9 bones, each with one rigid mesh: 020003BD body, 020003C4 / C2 / C3 wheels, 020003BE truck, 020003BF hatch,
  020003B7 windows, 020003C0 / C1 wipers). **The root translation is world space** (frame 1 root = (154.05, 15.80,
  93.95) = part 0's live matrix); frame = t − t0 + 1, clamped. Path: down the valley road, stops at the gatehouse
  (88.8, 18.9, 101.7) frames 1800–2400, again at (57.9, 26.2, 96.8) 3361–3601, parked (29.9, 36.0, 48.0) from ~5400;
  deleted at t = 5895 (98 s).
- Streams 4 / 5: headlight coronas `Corona_2` 0x02000426 on bone 0, local (−0.7338, 1.0187, 1.4450) /
  (0.7836, 0.9992, 1.4383). Streams 6–8: touch triggers (channels 0x70, 0x71 on the bed, 0x5C → ScriptPlayer #38).
- Port recipe: a character with skin 0x05000005, attachments per `extra` (bone, mesh), pose 0x04000009 at the clock's
  frame keeping the root translation, billboards for the headlights, hidden from t ≥ 5895.

## Searchlights [M]

Controller (p0 = 1) fields: on / off / alarm channels (p1–p3), sweep mid / half-range from p4 / p5 degrees, base yaw,
random phase 0..479. Sweep: pitch = sin(t·2π/480)·half + mid, yaw = cos(t·2π/480)·half + base. Beam aligned to the
direction; spot = 100-unit ray hit (+0.01 normal); corona at head + 0.4·dir, shown when the view dot > 0.2 (alpha
(d − 0.2)·4·255, scale (d − 0.2)·16, ×1.5 with line of sight). Spot within 2 of Bond → tracks him (gain 0.15), alarm
channel; shot → droops, `OBJECT_SearchLightBroken`.

## Open

ScriptPlayer #71 (`06000074`, a Loadable) at the start trigger; whether AnimObject flag 0x8000 selects the absolute
root; `Corona_Update`'s caller; door opening; copter paths.
