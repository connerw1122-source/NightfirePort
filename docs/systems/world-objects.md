# Dynamic objects at level start (07000005): searchlights (WORKLOG 0072) and coronas (0073) ported; the rest not drawn yet

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp, [L] snapshot `work/ee_start_05.bin`, [S] suggested. Source: a
research pass (subagent, 2026-10-09). **Searchlights re-checked here 2026-10-10** (disassembly of 0x1cb4f8/0x1cb900
plus the snapshot; corrections below); **coronas re-checked 2026-10-10** (disassembly + snapshot + the `control_funcs`
table); the rest is the research pass, not yet re-checked.

## Framework [M, L]

- `parsemap_block_map_data_dynamic` (0x1d1b68) → `parsemap_create_dynamic_objects` (0x1d04f0) only for placements with
  `flags & 0xA000` = 0 (0x8000 cels and 0x2000 templates are not created). Placement +0x40 / +0x42 / +0x44: path
  count, rate, `m_paths` index.
- **Create call convention [M, disassembly of 0x1d04f0]**: the dispatcher builds two vectors on its stack from the
  placement record — `sp+48` = translation (+0x0C), `sp+64` = rotation angles (+0x18) — and calls every
  `*_Create` as `(a0 = &translation, a1 = &rotation, a2 = placement, a3 = celglist)`. The `level_tag*` in the mangled
  names is the placement record; parameters sit at placement `+0x2C + 4k` [M, also docs/formats/placement.md].
- Live list `DynamicObjList` 0x2705a0 (head +0x14, next `obj+0x14`); per type `control_funcs` 0x26f890 (update,
  collide, delete; 12 bytes). Snapshot: 523 objects (13 drones, 8 ScriptPlayers, 5 doors, 49 triggers, 29 searchlight
  parts, 4 pickups, 2 copters, …). `View_AddObjects` 0x1e62d8 skips `obj+0xF0 & 0x8010`; `obj+0xFC` ≠ 0 = drawn.
- **Cels [M, re-checked]**: `build_LinkToRoom` (0x1c9680) finds the cel from the object's bounding sphere
  (`Control_BuildWorldSph`, `build_FindCel`) and `control_link_object_to_cel` (0x1346b8) **prepends** the object to
  the cel's doubly-linked list (`cel+8` newest, next `obj+8`, prev `obj+0xC`, `obj+0x20` = cel). Searchlight part
  linking (below) walks this list, so it sees same-cel objects only, newest first.

## Missing, by priority

| What | Source | Creator / update |
|---|---|---|
| **Wine truck** (9 rigid parts + 2 headlight coronas) | script `06000A22` (type-7 entry, 1856 bytes) played by ScriptPlayer placement #37 (type 0xD9, p0 = script, p2 = 1) | `SP_CreateScriptPlayer` 0x1db9a8 / `SP_Update` 0x1dbda8 → `Script_AnimStart` 0x1d8f40 |
| Snow (4096 flakes) | 0xDF `SNOW_gen` ×2 + 0x32 `Effect_Snow_*` ×18 | docs/systems/render.md |
| Searchlights (7 mounts, 7 heads, 6 beams, 7 coronas, 2 spots) | 0xDE ×29 | `Searchlight_Create` 0x1cb4f8 / `Searchlight_Update` 0x1cb900 — **spec re-checked 2026-10-10, see below** |
| ~~Coronas~~ | 0xFC ×13 | `Corona_Create` 0x18da00 — **ported (WORKLOG 0073), see below**; `Corona_Update` 0x18da78 is a different, runtime class (0x45, U-042) |
| Doors (5), level-exit gates (2) | 0xDB, 0xE8 | `Door_Create` 0x136100 / `Door_Update` 0x1363c8; `Trigger_LoadLevelCreate` 0x1b2950 |
| Helicopter | 0x2F ×2 (+ ScriptPlayers `06000046` / `06000047`) | `Copter_Create` 0x17d340 / `Copter_Update` 0x17dbb8 |
| Ladder, zip wire, grapple pulleys, placed pickups | 0x22, 0x3E (rotation z negated), 0x3A, 0xF0 | `Ladder_Create` 0x1966c0, `Wire_Create` 0x1981d8, `Grapple_Create` 0x198298, `Pickup_Create` |

## Coronas [M disassembly + decomp, re-checked 2026-10-10; L snapshot]

Type-0xFC placements dispatch to `Corona_Create` (0x18da00) → `control_create_object(0, &translation, &rotation,
NULL)` → **class 1** (`obj+0xFF = 1`), `obj+0xD8 = glist`, `obj+0xF0 = glist+0x44 | (old & 0x30008000)
(`Control_SetGList` 0x134960 — the decompile's `+0xF0 = *(glist+0x44)` drops the preserved-bits OR),
`build_LinkToRoom`, no instance, **no RNG draw**. `control_funcs[1]` is **empty** (0x26f894, table read from the
ELF): **no update function ever runs** — the 13 live objects keep their create-time state forever (alpha
`+0x106 = 0xFF`, scale `+0xE8 = 1.0`, `+0xFA = 0`, `+0xF4 = 0`; the drawn byte `+0xFC` cycles 2→1→0 only
through the renderer's decrement pass at the tail of `control_movement_object_handler` 0x1336f8, reset to 2
by `View_AddObjects` when drawn).

Live [L]: 13 objects — `Corona_Glow2` ×5 (placements 14, 15, 16, 55, 260), `Window_corona` ×7 (231–237,
the castle windows), `Corona_test1` ×1 (245); `obj+0xF0` = 0x22D (Glow2, test1) / 0x229 (Window) — the
glist's +0x44, which is the object's own flags word. Bit 0x1 → `View_AddObjects` (0x1e62d8) **draw list 1**
with a mirrored (−1) distance sort key — the alpha list; `+0xDC = 0` so the distance-fade branch is skipped;
`+0xFA` has neither 0x8 (×50 scale) nor 0x10 (+2000 depth bias); each leaf's own GS blend state draws them.

`Corona_Update` (0x18da78) sits in **`control_funcs[69]`** (class 0x45, pointer read from the table at
0x26fbcc): a LOS alpha fade (±0x10 per `GameState+52` parity tick, `obj+0xF0 |= 0x2000` while clear) — a
*runtime* corona class. No class-0x45 object exists at level start and no creator was found in the static
disassembly (U-042) — the truck's headlight `Corona_2` script entities are class 0 (`Script_CreateEntity`'s
generic path, no class store). Nothing to port until a 0x45 creator shows up.

**Port status**: `crates/nf-game/src/corona.rs` (the placement filter + the spec) — the objects are static,
so the "simulation" is the placement transform; `nf-view` adds the 13 as World-layer instances (the alpha
list via the object-flags bit 0x1, each leaf's GS blend). Golden test `corona_live.rs` (names, flags,
the alpha-list bit; doors 0xDB as the discriminator).

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
  (88.8, 18.9, 101.7) frames 1800–2400, again at (57.9, 26.2, 96.8) 3361–3601, parked (29.9, 36.0, 48.0) from ~5400.
  **Correction (WORKLOG 0058):** not deleted at 5895 — the event at 5890 sets "keep objects", so the truck stays parked
  on the last frame (docs/systems/script-player.md).
- Streams 4 / 5: headlight coronas `Corona_2` 0x02000426 on bone 0, local (−0.7338, 1.0187, 1.4450) /
  (0.7836, 0.9992, 1.4383). Streams 6–8: touch triggers (channels 0x70, 0x71 on the bed, 0x5C → ScriptPlayer #38).
- Ported as the general script player (docs/systems/script-player.md, WORKLOG 0058), not as a special case.

## Searchlights [M disassembly + decomp 0x1cb4f8 / 0x1cb900, re-checked 2026-10-10; L snapshot]

Five part kinds (the spec's "searchlight objects" are per-part placements, type 0xDE). The placement's object-type
byte feeds `Searchlight_Create` as **parameter 0 = part kind** (u16 at +0x2C): **0 mount** (rotating base, yaw
follows the head), **1 head** (the controller; only parts of kind 1 run `Searchlight_Update`), **2 spot** (ground
circle; carries a dynamic light), **3 beam** (the visible cone), **4 corona** (billboard at the lamp). Snapshot: 7
mounts, 7 heads, 6 beams, 7 coronas, 2 spots = 29 objects, all class `obj+0xFF = 0x22`.

### Create (0x1cb4f8)

`Searchlight_Create(&translation, &rotation, placement, glist)` (convention above) → `control_create_object(0x40,
pos, rot, NULL)`; class byte `obj+0xFF = 0x22`; instance (malloc'd, `obj+0xE0`):

```
+0x00 u16 kind              <- placement p0 (+0x2C)
+0x04 f32 phase             <- Rand_Rand(480) as float (EVERY part; RNG order!)
+0x10 f32[4] rotation copy  <- *(&rotation) 16 bytes; +0x14 = placement rot.y = base yaw
+0x20..+0x30 obj* by kind   <- filled by the linking loop: instance + 0x20 + kind*4
+0x34 u8 on-channel         <- p1 (+0x30)   +0x35 u8 off-channel  <- p2 (+0x34)
+0x36 u8 alarm-channel      <- p3 (+0x38)
+0x38 f32 half-range (rad)  <- (hi_rad − lo_rad) · 0.5, with lo/hi = float(min/max(p4,p5)) · 0x3C8EFA36 (EE-chopped
                                muls; the constant is π/180 + 1 ulp, NOT π/180 — 0x1cb604) — the live heads' bits
                                reproduce 7/7; the old "(max−min)·π/360" was last-bit wrong on all of them
+0x3C f32 mid (rad)         <- (hi_rad + lo_rad) · 0.5
obj+0xF4 u16 state          <- 3 if p1 ≠ 0 else 0; obj+0xD8 = glist; obj+0xF0 = glist+0x44
```

**RNG order [M, critical]**: every part consumes `Rand_Rand(480)` at create (the phase), in placement order; kind-2
parts consume another `Rand_Rand(0)` as the light's flicker seed. Both advance the shared stream
(`Rand_Rand` steps Y then X; `Rand_FRand` steps X then Y — 0x1e3738/0x1e3780).

**Linking loop [M + L]**: after `build_LinkToRoom`, walk the cel list (`obj+0x20 → cel+8`, next `+8`): skip self,
skip `class ≠ 0x22`, skip `Vec_Dist3D ≥ 3.0`; if I am kind 1, link the other into my slot by ITS kind; else if the
other is kind 1, link me into ITS slot by my kind. **Both stores are unconditional** — the walk (newest first)
leaves the OLDEST match linked. If the head's `+0x34` (on-channel) ≠ 0, also `View_SetDrawInNoViews` on the newly
linked non-mount part (starts dark). **The kind-2 branch (the light node and the −10000 move) runs AFTER the
walk [M, 0x1cb79c]**: a spot created after its head links while still in place and then moves below the world
(spot 016e9020); a spot created before its head is moved away by its own create and the later head's walk never
sees it — spot `016e8ec0` (courtyard) is exactly such an orphan (no light ray, invisible). Portable as-is: create
placements in record order, link by the rule, then move.

**Kind-2 branch**: allocate a light node (free-list 0x2c74c0, 32 × 0x50; `Mem_Malloc(0xa00, 0x3704)` fills it):
`+0x3C = 500.0` (radius), `+0x40 = 1.0`, `+0x44..46 = FF FF FF`, `+0x47 = 2` (type), `+0x48 = 1`,
`+0x28 = Rand_Rand(0)` (flicker), `+0x2C = 0xFFFF`; position = obj pos; `psiLight_Create` → `+0x34` handle,
`+0x30` = owner obj; the spot's own `instance+0x28` = light. Then the object is moved `pos −= (10000, 10000, 10000)`
(until the first ray hit repositions it). Lights live on list 0x2c74cc.

### Update (0x1cb900), head parts only

Kill check (any state): if `+0x35` (off-channel) ≠ 0 and the channel is on → state 3, delete the light
(`LList_Remove` 0x2c74cc / `psiLight_Delete` / back to 0x2c74c0), flag spot/beam/corona objects `+0xFE |= 1`,
clear the links and both channel bytes, `Control_NextLOD`.

- **State 0 (sweep)**: `pitch = PS2Sinf(phase · 2π/480) · half + mid`; `yaw = PS2Sinf(phase · 2π/480 + π/2) · half +
  rot.y` (cos via the same approximation, checked in the disassembly; the step constant is 0x3C567751, the steer
  0x3E19999A). `phase += FRAME_RATE_MUL`; the `fmodf(phase, 480)` call's result is **discarded** (the store sits in
  the jal delay slot) — phase grows unbounded; `PS2Sinf`'s internal wrap keeps precision. The object's angles are
  then **normalised to (−π, π]** (the framework's matrix build): a head with mid > π stores its pitch 2π below the
  raw sum (016e03f0: 4.445 stored as −1.838) [L]. Verified on the snapshot: head `016dfe70` phase 317 → pitch
  0.04712 / yaw 0.62241, bit-exact in the port.
- **State 1 (tracking Bond)**: `switch_channels[alarm] = 1` held every tick. Targets: `yaw* = atan2f(dx, dz)`,
  `pitch* = −atan2f(dy − 1, √(dx²+dz²))` (player pos − head pos; **libm `atan2f`**, not `ATAN2_APPROX`). Steer
  `angle_difference(cur, target) · 0.15`, clamped to the sweep cone (yaw within `rot.y ± half`, pitch `mid ± half`).
  Mount yaw = head yaw.
- **Spot (kind 2, via the head's +0x28)**: `Vec_Spherical_2_Cartesian(100, yaw, −pitch)` from the head pos
  (`(r·sin(yaw)·cos(p), r·sin(p), r·cos(yaw)·cos(p))` — 0x1e3948, disassembly-checked) → `Collide_RayIntersect`
  (mask 0x18). Hit: spot pos = hit + normal·0.01, orientation from the normal (`ATAN2_APPROX(nz, √(nx²+nz²))`,
  `π/2 − that`, `ATAN2_APPROX(nx, nz)`), draw. No hit: hide. Then: `dist(spot, player) ≤ 2` → state 1 (tracking);
  while tracking the spot's `instance+0x04` counts ticks not within 2 units, back to state 0 past
  `FRAME_RATE · 15` (15 s).
- **Beam (kind 3)**: `Mat_Align2Dir(beam+0x90, dir = spherical(1, yaw, −pitch), up = player.pos − head.pos, up)`
  (0x1df2a8: rows = normalize(cross(up, z)), normalize(cross(z, row0)), z = normalize(dir); the 4th arg is the
  fallback when |up·z| > 0.999); `Mat_SetTrans(head.pos)`; flags `+0xFA |= 0x24`, draw.
- **Corona (kind 4)**: pos = head + `spherical(0.4, yaw, −pitch)`; `dot = (0.4·dir)·(Player_GetHeadPos − head.pos) /
  |Player_GetHeadPos − head.pos|`; ≤ 0.2 → hide; else draw, `+0xFA |= 0x20`, `alpha byte +0x106 = (dot−0.2)·4·255`,
  `scale +0xE8 = (dot−0.2)·16`; `Collide_LineOfSight(viewer, corona)` clear → `+0xF0 |= 0x2000`, scale × 1.5.
- **State 2 (broken)**: pitch += 0.05 per tick up to 0.8 (droop); while drawn, `(GameState+52 + obj+0xEC) % 7 == 0`
  → `Gas_Init` smoke (hash `0x2000039`, size 0.05/0.25, `Rand_FRand(2π)` per puff — consumes the shared stream).
  Entered from the bullet check; no part updates after.
- **State 3 (off)**: parts stay hidden; if `+0x34` (on-channel) fires → state 0 + `Sound_Play3D(… id 0xE5)` (the
  relay clunk; 0xE6 = glass/break).
- **Bullet check** (states 0/1/3, tail of the update): walk the object's child list `obj+0xD0` (next `+4`); an entry
  whose `+0x58` object has class 5 (bullet) and `+8 > 0` breaks the light: state 2, light deleted, parts freed,
  `Sound_Play3D(… 0xE6)`, `Control_NextLOD`.

**Alarm wiring [L]**: the two dark heads (states 3) wait on channels **97 (0x61)** and **164 (0xA4)** — the same
channels `DroneFunc_CheckAlarmRaised` posts (docs/systems/music.md); the alarm turns the castle searchlights on.
While a head tracks Bond it holds its own alarm channel (`p3`).

**Port status**: `crates/nf-game/src/searchlight.rs` (WORKLOG 0072) — create/link (the orphan rule), the RNG
order, the sweep (bit-exact on the five golden heads), tracking, the off/kill channels with sounds 0xE5/0xE6, the
spot ray and orientation, `Mat_Align2Dir` (cross order pinned against the live beam matrix: rows =
n(cross(up, z)), n(cross(z, row0)), z), the head/mount matrices (the framework's align against world up, pinned on
head 016dfe70), the corona dot/alpha/scale/LOS, mount yaw, breaking (state 2 droop) and the bullet stand-in
(segments vs the solid parts' boxes). nf-view draws the parts from the simulation each frame; the coronas are
camera billboards. Still open: the spot's dynamic light (illumination is renderer work), `Gas_Init` smoke (no
particle system; its RNG draws are not consumed — G-033), the corona's alpha byte is not wired to nf-view's
shader, and the bullet child-list path remains the U-041 stand-in.

## Open

ScriptPlayer #71 (`06000074`, a Loadable) at the start trigger; whether AnimObject flag 0x8000 selects the absolute
root; door opening; copter paths. ~~`Corona_Update`'s caller~~ answered (WORKLOG 0073): `control_funcs[1]` is empty —
the 13 level coronas never update; `Corona_Update` belongs to runtime class 0x45 (U-042, no creator found).

Searchlight remainder: the spot's dynamic light and its flicker (illumination is renderer work), smoke
(`Gas_Init` — needs a particle system; 6 RNG calls per puff measured at 0x1ab060–0x1ab248, not consumed), the
corona's alpha byte into nf-view's shader, the bullet child-list path (U-041; the box stand-in is in place). The
level-exit gates and objectives feed music events 3/7/8 (see HANDOFF §5).
