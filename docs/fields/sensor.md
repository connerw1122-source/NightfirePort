# SENSOR — the security camera (`Sensor_Create` 0x1af010, `Sensor_Update` 0x1af618)

Placement type **0x28** (62 on disc), object class **0x37** (`control_funcs[0x37].update`,
`control_movement_object_handler` 0x1336f8's walk). The instance is `control_create_object(100, …)`
— 100 bytes at `obj+0xE0`; registered in the CameraList 0x319930 (`Sensor_Init` 0x1b0868,
node size 100) unless param 5 ≠ 0; `Sensor_ApplyDamage` 0x1b08a8 is an empty stub (sensors take
no damage). Port: `crates/nf-game/src/sensor.rs`.

Evidence levels: [M] decompile address, [L] EE snapshot / live read, [S] suggested.

## Detection is player-only

`Sensor_Update` loads `glb_players[0]` once per tick (disasm `lw $s4, -0x7720($v0)` @0x1af644,
global 0x2d88e0) and passes it as the test object to both `Sensor_InCone` calls (sweep
0x1af97c, track 0x1afae0) and to `Sensor_SetBeam` (0x1afc14). `Sensor_InCone` has no other
callers, and no `switch_channels` writer is NPC-proximity-driven — the "trip-by-anything"
summary in `docs/systems/script-census.md` (pre-2026-10-17) was not what the binary does.
`glb_viewer[4]+0x29` (0x2d88c0 → viewer 4's enable byte, the scripted-camera flag) set → the
cone test is skipped entirely (sensors go blind during cutscene cameras) [M 0x1af964-0x1af970].

## Placement params → instance (`level_tag` = the 0xB0 expansion, `docs/fields/level-tag.md`)

| level_tag | param | SENSOR off | meaning |
|---|---|---|---|
| +0x02 (key count) | — | +0x30 | sweep frame limit = count − 1 (0 when count 0) [M 0x1af010:82-88] |
| +0x04 (rate) | — | +0x2C | sweep step = (count−1)·REC_FRAME_RATE·(FRAME_RATE/rate) — the rate word is the sweep's whole duration in ticks [M 0x1af010:89-96] |
| +0x08 (key list) | — | +0x24 | the 0x19 framelist the sweep poses (`Spline_Interp` mode 2) [M 0x1af010:81] |
| +0x2C | p0 | (lens) | lens gfx entity hash; ≠ 0 creates the lens proxy and arms `Sensor_SetBeam`'s ray [M 0x1af010:97-108] |
| +0x30 | p1 | +0x34 | cone half-angle in degrees; +0x38 = `PS2Sinf(deg·0.008726647 + π/2)`, +0x3C = `PS2Sinf(deg·0.008726647)` [M 0x1af010:61-66] |
| +0x34 | p2 | +0x40 | range, (f32)(i32) [M 0x1af010:67-73] |
| +0x38 | p3 | +0x58 | OUT switch channel — 1 after 2 s of sight, 0 while sweeping/returning [M 0x1af618:154-157,169-171,227-229] |
| +0x3C | p4 | +0x5A | disable channel: set → state 3 (off), cleared → re-arm [M 0x1af618:107-117,259-263] |
| +0x40 | p5 | +0x5F | byte: when 0 the sensor joins the CameraList [M 0x1af010:135-137] |
| +0x44 | p6 | +0x5E | byte: monitor group id (`Monitor_Activate`/`Monitor_Update` match it) [M 0x1b0058] |
| +0x48 | p7 | +0x50 | mount gfx entity hash (visual) [M 0x1af010:109-119] |
| +0x4C | p8 | +0x5C | monitor-view channel, held 1 while a monitor watches [M `Monitor_Update` 0x1afcc8] |

Disc census of the 29 main-map sensors [D probe]: cone angles {5, 10, 15}°, ranges {5, 6, 7},
out channels on 0700000a/0b/12/14/46, p5 = 0 everywhere, lens 0x02000FE0 / mount 0x0200652 or 0.

## obj_tag fields

| off | meaning |
|---|---|
| +0x30 / +0xC0 | position / matrix translation — the cone apex [M 0x1af4f0] |
| +0x90 | the pose matrix; `Mat_GetDir` 0x1e3398 reads row 2 as the cone axis [M] |
| +0xD8 | the drawn glist (swapped to `SENSOR+0x10` = the destroyed glist in state 3) [M 0x1afc44] |
| +0xE0 | the SENSOR instance [M] |
| +0xF0 | draw flags; bit 0x10 = the monitor-view hide latch's effect [M 0x1af618:57-69] |
| +0xF4 | u16 state: 0 sweep, 1 tracking, 2 returning, 3 disabled [M] |
| +0xF6 | i16 consecutive seen/lost tick counter [M] |
| +0xFE→+0x60 | SENSOR+0x60 u8 monitor-view latch (1 = hide, 2 = unhide) [M] |

## SENSOR instance

| off | meaning |
|---|---|
| +0x08 | the owner obj_tag* (also the CameraList node payload) [M 0x1af010:94] |
| +0x0C / +0x10 | the placement glist / the destroyed glist (`celglist+0x40`) [M 0x1af010:129-134] |
| +0x14 | the aim quaternion — written by track (`Vec_Track` 0x1e20a0) and return (`Quat_Slerp_Acc` 0.1), NOT by the sweep [M] |
| +0x24 / +0x28 / +0x2C / +0x30 | the sweep: framelist, playhead, step, limit [M] |
| +0x34 / +0x38 / +0x3C / +0x40 | angle deg, cos half, sin half, range [M] |
| +0x44 / +0x48 | f32 detection timer (trip at ≥ 2.0; beep thresholds 2.0/3.0) / beep+servo timer [M 0x1af618:121-135] |
| +0x4C / +0x50 | the lens / mount proxy objects [M] |
| +0x54 | the 0x4AA hum sound handle [M 0x1af618:103-105] |
| +0x58 / +0x5A / +0x5C / +0x5E / +0x5F / +0x60 | out ch, disable ch, monitor-view ch, monitor id, skip-list byte, view latch [M] |

## State machine [M 0x1af618]

* **Preamble**: monitor latch hide/unhide; disable channel set → state 3 (+ hum stop).
* **0 sweep**: clear out ch; detect timer = 0; sweep pose + `t += step` (wrap past the limit,
  servo 0x4DD at the half and the wrap — unplayed in the port); `Intersect_ConeSphere` 0x1ee100
  (range gate `(range + target radius)²`, cone, then `Collide_LineOfSight` …, 10)): hit →
  `+0xF6++`, `prev > 0x1E` → state 1 (beep 0x4A9); miss → `+0xF6 = 0`.
* **1 tracking**: `Vec_Track(0.05, players[0]+0x80, …)`; miss 61 ticks (FRAME_RATE_INT) →
  state 2; else detect += REC, and at ≥ 2.0 with an out channel:
  **`switch_channels[out] = 1; switch_channels_time[out] = GameState+0x34`** (re-written every
  tick the sight holds).
* **2 returning**: clear out ch; slerp the aim quat toward the sweep pose at 0.1/tick; state 0
  when `Quat_IsEqual(0.01)`.
* **3 disabled**: destroyed-glist swap, lens hidden, hum stopped; re-arms (state 0) when the
  disable channel clears.

Port stand-ins: the beam-hit shortcut approximates `Sensor_SetBeam`'s object-identifying ray
with the static world + a sphere test [S]; the target radius is the port's documented 0.4
(obj+0x8C's live value is not in the snapshots) [S]; sounds are not played (ss.rs precedent).
