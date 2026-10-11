# Atlas: the object/control list entry (`obj_tag`)

Plan v2 §2 struct 1. Build: ACTION.ELF `e7ffdcb3…` (SHA-1 verified on the copy used here).
Sources: decomp + disassembly [M]; snapshot `work/ee_start_05.bin` (level 07000005 at start) [L].
Port: `crates/nf-game/src/world.rs`, `levels.rs`; the per-class handlers map to `control_funcs`.

## Name, size, allocation site

- **`obj_tag`** — the object header, **0x110 bytes** [M `control_create_object` 0x1344e0:
  `Mem_Malloc(instance_size + 0x110, 0x404, 0)`, then memset 0]. The per-class instance follows
  in the **same allocation** at `obj+0x110`; `obj+0xE0` holds the instance pointer
  (`__s[0x38] = __s + 0x44` [M 0x1344e0], i.e. instance = obj+0x110).
- Instance sizes: drone 0xD20 [M `NDrone2_CreateObj` 0x14d7c8 passes 0xD20], DOORINFO 0x80
  [M `Door_Create` 0x136100 via `control_create_object(0x80,…)` — docs/systems/world-objects.md],
  searchlight part per `Searchlight_Create` 0x1cb4f8.
- Creation: `control_create_object(u32 instance_size, VECTOR* pos, VECTOR* rot, quaternion* q)`
  0x1344e0. Variant `Control_CreateObjEx` 0x1343c8 (scale, size, matrix, glist, parent obj, …)
  for script/MP creations. Placement drones: `Drone_Create` 0x136be0 → `NDrone2_CreateObj`
  0x14d7c8. Deletion: `control_delete_object` 0x133178 (runs `control_funcs[class].delete`,
  frees the HITDATA list, unlinks both lists, `Mem_Free`s the whole block).
- Init: `control_init_object` 0x134630 sets `+0xF8 = 0x1F`, `+0xFE |= 2` (sphere rebuild
  pending), `+0x102 = 0xFF`, `+0xEC = GameState[52]` (frame), `+0xFC = 2`, `+0xE8 = 1.0f`,
  `+0x100..0x106 = 0xFF`.

## The lists

- **`DynamicObjList` @ 0x2705a0** — the live object list head, itself an obj-shaped 0x110-byte
  struct [M symtab size 272; `control_init_object_lists` 0x134388 zeroes head+0x14/+0x18].
  **First object at head+0x14, last at head+0x18** [M disasm of `control_first_object` 0x134300:
  `lw $v0, 0x5b4($v1)` with `lui 0x27` = load from 0x2705b4]. Beware: the decompiler prints this
  load as `DynamicObjList._20_4_` — its data type for the global is skewed; the disassembly
  above is the ground truth. `control_add_object_to_list` 0x134698 **prepends**: `obj+0x18 =
  head`, `obj+0x14 = old first`, `head+0x14 = obj`, `old_first+0x18 = obj` — so **next = obj+0x14,
  prev = obj+0x18** [M]. Update order is therefore newest-first.
- **`DynamicObjCount` @ 0x30ca98** [M symtab] — incremented by create, decremented by delete
  [M 0x1344e0 / 0x133178]. [L] equals the walked list length (523).
- **ForcedList @ 0x270590** — an `LLISTINFO` (12 bytes: first +0, last +4, u16 count +8, u16
  node size +0xA) initialised `LList_Init(0x270590, 0x110, 0)` [M 0x134388]. Objects whose
  `+0xF0 & 0x30000000` are members; the links are the object's first two words (`+0x00` prev,
  `+0x04` next — why `control_create_object` zeroes `*obj` and `obj[1]` after linking [M
  0x1344e0]). `control_delete_object` removes from it when `+0xF0 & 0x30000000` [M 0x133178].
- **The drone LList @ 0x298e14** (unnamed) — instances as nodes, node links at instance+0x00
  (prev) / +0x04 (next); `LList_Add` 0x1cca28 **appends at the tail** [M]. [L] head first =
  0x016eda50 (drone with sm_id 1, the first created), last = 0x01382cd0 (sm_id 13, last
  created), count = 13. Membership is what the port's `present`/`in llist` stands for: a
  spawner template has no object at all until `DroneSpawner_SpawnDrone` 0x137cb0 calls
  `NDrone2_CreateObj` on the template's pre-built DIVars [M].
- **The cel list** — per-cel doubly-linked list: cel+0x8 = first, cel+0xC = its prev-slot
  (cleared on prepend); object links `next = obj+0x8`, `prev = obj+0xC`, cel pointer
  `obj+0x20` [M `control_link_object_to_cel` 0x1346b8 — same insert pattern as the relink in
  the movement handler]. `build_LinkToRoom` 0x1c9680 picks the cel from the world sphere when
  there is no explicit target [M].

## Field table (header, offsets in bytes)

| Off | Type | Name | Meaning | Writers | Readers | Tag |
|---|---|---|---|---|---|---|
| +0x00 | ptr | ForcedList prev | ForcedList link (see lists) | `LList_Add` 0x1cca28 / `LList_Remove` 0x1ccac8; zeroed at create | ForcedList walkers | [M] |
| +0x04 | ptr | ForcedList next | | same | same | [M] |
| +0x08 | ptr | cel next | next object in the same cel | `control_link_object_to_cel` 0x1346b8; movement-handler relink; cleared by delete | cel-list walkers (`View_AddObjects` 0x1e62d8 iterates `obj+0x8`) | [M] |
| +0x0C | ptr | cel prev | | same | same | [M] |
| +0x14 | ptr | list next | next object in DynamicObjList | `control_add_object_to_list` 0x134698, delete 0x133178 | `control_movement_object_handler` 0x1336f8 and every list walk | [M] |
| +0x18 | ptr | list prev | | same | same | [M] |
| +0x1C | ptr | proxy obj | another object this one must share a cel with | `Control_CreateObjEx` 0x1343c8 param 7 | movement handler cel-follow pass (relink to `(obj+0x1C)->+0x20`'s cel) | [M] |
| +0x20 | ptr | cel | the `cel_tag` this object sits in | link/relink fns | relink pass, `Lights_*`, BuildWorldSph ambient copy | [M] |
| +0x30 | vec3 | pos | position (the .w word +0x3C rides along) | `control_create_object` 0x1344e0 (arg), DefaultInits, movement pass, flags2&0x40 copy from +0xC0 | everywhere | [M] |
| +0x40 | vec3 | prev pos | previous position | movement pass copies +0x30→+0x40..+0x4F after a move (incl. +0x48=+0x38, +0x4C=+0x3C) | collision/render deltas | [M] |
| +0x50 | vec3 | rot | rotation angles x/y/z (rad; ±π-wrapped by the movement pass) | create, per-class updates | matrix rebuild | [M] |
| +0x60 | vec3 | prev rot / dir | copied from +0x50 when flags2&6 == 0; else `Mat_GetDir(+0x60, +0x90)` recomputes the matrix's direction into it | movement pass 0x1336f8 | per-class | [M] |
| +0x70 | vec3+f32 | prev world sphere | previous sphere centre (+0x70..+0x7B) and radius (+0x7C) | `Control_BuildWorldSph` 0x133070 copies +0x80..+0x8F here first | collision sweeps (vs. old sphere) | [M] |
| +0x80 | vec3 | world sphere centre | world-space bounding-sphere centre | `Control_BuildWorldSph` 0x133070 (glist local sphere +0x10 transformed by the matrix; glist-less objects use matrix row 3 at +0xC0; flags2&8 objects use `*(obj+0xDC)+0x80` instead) | `View_AddObjects` 0x1e62d8 (`Vision_InView(radius, centre, viewer)`), trigger/collision tests | [M] |
| +0x8C | f32 | world sphere radius | | same fn: `glist+0x1C * abs(scale +0xE8)`, or `*(obj+0xDC)+0x8C * abs(scale)`; 0.1 for glist-less objects (kept if flags2&0x100) | same | [M] |
| +0x90 | MATRIX | matrix | 4×4: rows 0-2 rotation, **row 3 = translation at +0xC0** | `View_RotTransMatrix(rot, pos, +0x90)` [M 0x1344e0], `Quat_QuatToMat` path, `Mat_Copy` in CreateObjEx, `Control_GetObjMatrix` in BuildWorldSph | renderer, sphere build, flags2&0x40 position copy | [M] |
| +0xC0 | vec4 | matrix row 3 | the matrix's translation | matrix writers | flags2&0x40 teleport: +0x30 ← +0xC0..+0xCC [M 0x1336f8] | [M] |
| +0xD0 | ptr | HITDATA list | collision-hit list head; entries: next at +4, and entry+0x58 can reference another object | `Collide_Update` 0x1ea398, `Collide_FreeHitList` 0x1ede00 (delete path) | `control_movement_object_handler` delete pass (clears entry+0x58 refs to a dying object) | [M] |
| +0xD4 | ptr | class scratch | e.g. drone: `instance+0x3D0` [M `NDrone2_DefaultInit` 0x14b300] | per class | per class | [M] |
| +0xD8 | ptr | glist | the `celglist_tag` (geometry list): +0x10 sphere centre, +0x1C radius, +0x44 flags word | `Control_SetGList` 0x134960, `Control_CreateObjEx` | BuildWorldSph, `View_AddObjects` (glist-less skip) | [M] |
| +0xDC | ptr | AnimObject | the draw/model object: +0x90 trans callback, +0x62 a byte (weapon/class), **+0x110..0x113 four LOD fade bytes** | `AnimObjectNew` [M DefaultInit]; fade bytes written by `View_AddObjects` (dist·0.25/viewer+0x108, clamped 0..255) and memset 0xFF by the movement handler's tail when `GameState[0x40] != 0` | renderer | [M] |
| +0xE0 | ptr | instance | = obj+0x110 (in-allocation) | `control_create_object` 0x1344e0 | every instance access | [M] |
| +0xE4 | u32 | SP handle | second arg of `SP_RemoveObj(obj, +0xE4)` on delete (script-player binding) | `SP_CreateScriptPlayer` 0x1db9a8 path | `control_delete_object` 0x133178 | [M] |
| +0xE8 | f32 | scale | 1.0 at init [M 0x134630]; CreateObjEx param 1 | init/creates | sphere radius (× abs), model scaling | [M] |
| +0xEC | u32 | frame/wobble | init `GameState[52]` [M 0x134630]; drones overwrite with `Rand_Rand(10000)` [M `Drone_Create` 0x136be0]; [L] later reads 0x1A677 > 10000 — something rewrites it after creation (unidentified writer; the port's "miss-wobble phase" story needs a re-check) | see left | bullet wobble etc. | [M,L] |
| +0xF0 | u32 | flags | bit 0x1 = alpha/mirror draw list (sort key −dist); bit 0x10 = **not drawn** (disabled); bit 0x20 = disabled/no-vision pair (see `NDrone2_Enable`); bit 0x400 = cleared by drone init; bit 0x800 = seen on drones [L]; bit 0x2000 = +2000 depth-bias draw; bit 0x8000 = skip draw; bit 0x30008000 = glist-flag passthrough preserved by SetGList; bit 0x10000000/0x20000000 = ForcedList membership | `Control_SetGList` 0x134960 (`+0xF0 = glist->0x44 \| (old & 0x30008000)`), `NDrone2_Enable` 0x149dd0 (0x10/0x20), DefaultInit (&~0x20, &~0x40, &~0x400), movement pass | `View_AddObjects` 0x1e62d8 (skips `& 0x8010`), movement pass | [M] |
| +0xF4 | u16 | state | per-class state (door 2/4/8/0x10; drone: mirrored current state id) | per class, `NDrone2_DefaultInit` mirrors the SM | movement-handler world pass, docs | [M,L] |
| +0xF6 | u16 | | drone init writes 1 [M 0x14b300] | | | [M] |
| +0xF8 | u16 | display mask bits | init 0x1F [M 0x134630]; `View_AddObjects` tests `+0xF8 & object_display_mask` (a global u16 filter) | init, filter UI | `View_AddObjects` 0x1e62d8 | [M] |
| +0xFA | u16 | flags2 | bit 0x1 = OnlyScaleZ (draw); bit 0x2/0x4 = rotation-changed (drives +0x60 dir recompute and suppresses prev-rot copy); bit 0x8 = sphere source is `+0xDC` (AnimObject) [L drones: set, radius from AnimObject]; bit 0x10 = +2000 depth bias; bit 0x20 = moved this tick (angle wrap + sphere/cel/ambient + pos→prev-pos); bit 0x40 = teleport/position-from-matrix-row3; bit 0x80 = (BuildWorldSph: local sphere from +0xDC); bit 0x100 = keep radius; OR-ed by CreateObjEx param 9 | `Control_CreateObjEx` 0x1343c8, per-class updates, `NDrone2_DefaultInit` (nothing — set by collision/move code) | movement pass, BuildWorldSph, View_AddObjects | [M,L] |
| +0xFC | u8 | draw counter | init 2; decremented per tick at the movement-handler tail; set back to 2 by `View_AddObjects` when drawn; `!= 0` = drawn recently (the port's `drawn`) | `View_AddObjects` 0x1e62d8, movement pass | anim-early-end checks (`obj+0xFC = 0` skip), port | [M] |
| +0xFD | u8 | | door proxy bit 3 (docs/systems/world-objects.md) [S] | | | [S] |
| +0xFE | u8 | control flags | bit 0 = delete me (movement pass: `Effect_RemoveSObjs` + `control_delete_object`); bit 1 = rebuild sphere (BuildWorldSph + `Lights_CalcAmbientLight` then clear); bit 4 = seen set on drones [L 0x12] | `control_init_object` (bit1), creators, per-class | movement pass | [M,L] |
| +0xFF | u8 | class | `control_funcs` index | creators (`NDrone2_CreateObj`: 2 placement / 0x4D script path [M disasm `sb $v0, 0xff($s0)` with $v0=2 / 0x4D]; Door 0x1C, searchlight part 0x22, corona 1, runtime corona 0x45) | movement pass, delete | [M] |
| +0x100..0x102 | u8×3 | ambient RGB | init 0xFF; cel-relink copies `cel+0x9A/0x9C/0x9E` via `switch_channels[cel+0xA2]` when the old value is still 0xFFFFFF [M 0x1346b8]; `Lights_CalcAmbientLight` recomputes | relink, Lights | renderer | [M] |
| +0x103..0x106 | u8×4 | tint/alpha | init 0xFF; CreateObjEx params 11-13 write +0x103..+0x105; corona alpha at +0x106 | init, creates, per class | renderer | [M] |

## `control_funcs` @ 0x26f890 (85 entries × 12 bytes: update, collide, delete)

Decoded from the ELF bytes [M] with symtab names. Empty (000000×3) for classes not listed.

| Class | Update | Collide | Delete |
|---|---|---|---|
| 2 | `Drone_Control` 0x139268 | `Drone_CollisionHandler` 0x140968 | `Drone_Delete` 0x136ce8 |
| 3 | `Player_Update` 0x193600 | `Player_CollisionHandler` 0x1949a8 | — |
| 5 | `Bullet_update` 0x12e7b0 | `Bullet_CollisionHandler` 0x12ce80 | `Bullet_Delete` 0x12e660 |
| 6 | `Bubble_Update` 0x1aca48 | | |
| 7 | `rotor_update` 0x18f568 | | |
| 0xC | `Particles_Update` 0x1cf528 | | |
| 0xD | `Bullet_casing_update` 0x12faa0 | | |
| 0xE | `gases_smoke` 0x1ab2d0 | | |
| 0xF | `Explode_Update` 0x17c350 | `Explode_CollisionHandler` 0x17cde0 | `Explode_Delete` 0x17cdc0 |
| 0x10 | `Debris_Update` 0x1aac00 | | |
| 0x11 | `Drone_Control` 0x139268 | | `Drone_Delete` 0x136ce8 |
| 0x12 | `Player_Update` 0x193600 | `Player_CollisionHandler` 0x1949a8 | |
| 0x15 | `control_AnimUpdate` 0x1341f0 | | |
| 0x19 | `Sunreflection_Update` 0x18db40 | | |
| 0x1A | `RB_Update` 0x1d6e38 | `RB_CollisionHandler` 0x1d7a80 | |
| 0x1B | `SP_Update` 0x1dbda8 | | `SP_Delete` 0x1dc8b0 |
| 0x1C | `Door_Update` 0x1363c8 | `Door_ReactToHit` 0x135980 | |
| 0x1D | `Trigger_Update` 0x1b1a78 | | |
| 0x1F | `control_AnimUpdate` 0x1341f0 | | |
| 0x20 | `Break_Update` 0x12bfd8 | | |
| 0x21 | `Destroy_Update` 0x12c4f8 | `Destroy_CollisionHandler` 0x12bd48 | |
| 0x22 | `Searchlight_Update` 0x1cb900 | | |
| 0x23 | `Creature_Update` 0x135268 | | `Creature_Delete` 0x135230 |
| 0x24 | `cloud_update` 0x18d768 | | |
| 0x25 | `SS_Update` 0x1a9948 | | |
| 0x26 | `Flicker_Update` 0x1ab960 | | |
| 0x27 | `Hurt_Update` 0x17e5d0 | | |
| 0x28 | `Car_Update` 0x1307d8 | `Car_CollisionHandler` 0x131838 | `Car_Delete` 0x132e78 |
| 0x2A | `Occlude_Update` 0x17eba0 | | |
| 0x2E | `Leaf_Update` 0x1abee0 | | |
| 0x2F | `Pickup_Update` 0x18e148 | | |
| 0x30 | `DroneSpawner_Control` 0x137e68 | | `DroneSpawner_Delete` 0x13a338 |
| 0x31 | `LeafGen_Update` 0x1acc48 | | |
| 0x32 | `Emitter_Update` 0x17b080 | | `Emitter_Kill` 0x17b960 |
| 0x33 | `Lightning_Update` 0x18cf68 | | |
| 0x34 | `Ripples_Update` 0x1ac028 | | |
| 0x35 | `MP_ObjectUpdate` 0x18b198 | | `MP_ObjectDelete` 0x18b2c0 |
| 0x36 | `GT_Update` 0x180498 | | `GT_Delete` 0x181678 |
| 0x37 | `Sensor_Update` 0x1af618 | | |
| 0x38 | `Monitor_Update` 0x1afcc8 | | |
| 0x39 | `Switch_Update` 0x1adea8 | | |
| 0x3A | `Lock_Update` 0x1ae3d8 | | |
| 0x3B | `Copter_Update` 0x17dbb8 | | `Copter_Delete` 0x17e568 |
| 0x3C | `FuseBox_Update` 0x1adfe8 | | |
| 0x3D | — | — | `Discrete_Delete` 0x1c64d8 |
| 0x3E | — | — | `Drone_AIVolume_Delete` 0x13a1d0 |
| 0x3F | `Env_Tree_Update` 0x17c328 | | |
| 0x40 | `SoundTrigger_Update` 0x1b2790 | | |
| 0x41 | `Swoosh_Update` 0x18d200 | | |
| 0x42 | — | `ThirdIcon_Update` 0x196e80 (collide slot) | |
| 0x43 | `Hint_Update` 0x1b3380 | | |
| 0x44 | `MusicTrigger_Update` 0x1b31a0 | | |
| 0x45 | `Corona_Update` 0x18da78 | | |
| 0x46 | `BodyGlow_Update` 0x1ac2c8 | | |
| 0x47 | `GunImp_Update` 0x1320d8 | | |
| 0x48 | `CamSubject_Update` 0x1bc668 | | |
| 0x49 | `PCQWorm_Update` 0x1aee20 | | |
| 0x4A | `Sub_Update` 0x1ad200 | `Sub_Collide` 0x1ad868 | `Sub_Delete` 0x1ad870 |
| 0x4B | `Mine_Update` 0x17c930 | | |
| 0x4C | `OneSided_Update` 0x1e7d58 | | |
| 0x4D | `MiniSub_Update` 0x181f40 | `MiniSub_Collide` 0x182128 | `MiniSub_Delete` 0x182268 |
| 0x4E | `Shooter_Update` 0x17d0c0 | | |
| 0x4F | `SpaceMissile_Update` 0x1aa118 | | |
| 0x50 | `SpaceLaser_Update` 0x1aa3f8 | | |
| 0x52 | `DynamicObject_Update` 0x18d3b8 | | |
| 0x53 | `InverterTrigger_Update` 0x1b30a8 | | |
| 0x54 | `Apocalypse_Update` 0x18d4f0 | | |

Note: the script-drone path's initial class 0x4D shares the MiniSub row; the port should not
rely on class 0x4D ever reaching the update loop as a drone (script entities re-class; no
class-0x4D object exists in the 07000005 start snapshot [L]).

## Update order [M]

`Game_Run` 0x1c9790 per tick: `psiPreGame_Run` → `Input_Update` → if not frozen:
`Sound_UpdateListeners` → if !sloflag: `MenuManager_Update/Monitor`, `Mission_Update`,
`MP_Update`, `Text_Update(2Line)`, `UpdateAllShards`, `Env_Update` → if `!GS_IsPaused(-1)`:
`SSys_Monitor`, `Light_Update`, **`control_movement_object_handler(0)`** → `Camera_UpdateAll` →
`psiPostGame_Run`. (`ScriptCam` reroutes the Text update; a paused GS runs
`psiDecompressWoman` instead of the sim block.)

`control_movement_object_handler` 0x1336f8 (1936 bytes), in order:

1. `Drone_InitComms` 0x139520 (per-frame drone global pass: delayed messages, vision
   scheduling), `MP_DebugDraw`, `Doors_Calc`, `Trigger_Calc`.
2. **World-object switch-channel pass** — walks `glb_world` 0x30d0f8 (a different list: the
   static `world_tag` objects, switch channel byte at `+0xA2`, ambient bytes at
   `+0x9A/0x9C/0x9E`, path ptr +0x38, halfword +0x3C compared against 0xC047) and refreshes
   `+0x38` when `switch_channels[ch] != switch_channels_hold[ch]`.
3. **Channel history shift**: `amemcpy(switch_channels_prev 0x26fd90, switch_channels_hold
   0x26fe90, 0x100)` then `amemcpy(switch_channels_hold, switch_channels 0x26fc90, 0x100)`
   [M; names from symtab].
4. **Update pass** — for each object (DynamicObjList, newest first): if flags2 & 6,
   `Mat_GetDir(obj+0x60, obj+0x90)`; then `control_funcs[obj+0xFF].update(obj)` (first of the
   three pointers).
5. **Delete pass** (loops until no deletion requested): `obj+0xFE & 1` →
   `Effect_RemoveSObjs(obj)` + `control_delete_object(obj)`; classes 5 and 0xF get their
   back-references cleared first (class 5 instance+0x30/+0x40, class 0xF instance+0x24), then
   every other object's HITDATA list `obj+0xD0` (entry next +4) has `entry+0x58 == obj`
   cleared. `obj+0xFE & 1` clear + `& 2` → `Control_BuildWorldSph` +
   `Lights_CalcAmbientLight` + clear bit.
6. **Movement pass** (for each object): flags2 & 0x20 → wrap `+0x50/+0x54/+0x58` into
   (−π, π]; `Control_BuildWorldSph`; `control_handle_cel_change` 0x133288;
   `Lights_CalcAmbientLight`; `+0x40..+0x4F = +0x30..+0x3F` (prev pos); if !(flags2 & 6)
   `+0x60..+0x6F = +0x50..+0x5F` (prev rot). flags2 & 0x40 → `+0x30..+0x3C = +0xC0..+0xCC`
   (position from the matrix's translation row).
7. **`Collide_Update` 0x1ea398(first object)** — collision over the list (feeds the HITDATA
   lists).
8. **Cel-relink pass** — per object: `obj+0xFC != 0` → decrement; `obj+0xDC != 0` and
   `GameState[0x40] != 0` → memset the AnimObject's four fade bytes `+0x110..0x113` to 0xFF;
   if `obj+0x1C` (proxy) is set and its `+0x20` cel differs from `obj+0x20` → unlink from the
   cel list and prepend into the proxy's cel (or `build_LinkToRoom(obj,0,glb_world)` when the
   proxy has none); after a relink, refresh the ambient bytes from the cel and
   `Control_BuildWorldSph`.

## Placement → creator dispatch [M `parsemap_create_dynamic_objects` 0x1d04f0]

The dispatcher builds `sp+0x30` = translation (placement+0x0C..0x14) and `sp+0x48` = rotation
(placement+0x18..0x20) on its stack and calls every creator as `(a0=&trans, a1=&rot, …)`.
Params live at `placement+0x2C+4k` (param k); other placement fields used directly: +0x24
quaternion (GT 0x27, GunImp 0x34, Emitter 0xF2), +0x2C/+0x30/+0x34/+0x38 raw words
(Pickup 0xF0, View_AddSkyObj 0x2A, Light 0x4E), +0x34 spawn-frame gate for drones
(`Drone_Create`: skip unless `placement+0x34 <= GameState[0x40]`). Cels (0x8000) and
templates (0x2000) are filtered earlier (`parsemap_block_map_data_dynamic` 0x1d1b68).

| Type | Creator | | Type | Creator |
|---|---|---|---|---|
| 0x0F | `Drone_Create` 0x136be0 | | 0xDC | `Trigger_Create` |
| 0x20 | `Break_Create` (+psiCreateBreakableData) | | 0xDE | `Searchlight_Create` 0x1cb4f8 |
| 0x21 | `Ripples_Create` | | 0xDF | `Env_Create` (snow etc.) |
| 0x22 | `Ladder_Create` 0x1966c0 | | 0xE0 | `Creature_Create` |
| 0x24/0x2D | `Player_AddNewStartPos` (MP-gated) | | 0xE1 | `Destroy_Create` |
| 0x25/0x26 | `MP_RegisterSpawnPoint`/`MP_RegisterMPObject` | | 0xE2 | `SS_Create` |
| 0x27 | `GT_Create` | | 0xE3 | `Flicker_Create` |
| 0x28 | `Sensor_Create` | | 0xE4 | `Hurt_Create` |
| 0x29 | `Switch_Create` | | 0xE5/0xE6 | `Drone_CoverCornerNode`/`CoverLowNode` |
| 0x2A | `View_AddSkyObj` | | 0xE7 | `ThirdCam_Create` |
| 0x2B | `LeafGen_Create` | | 0xE8 | `Trigger_LoadLevelCreate` 0x1b2950 |
| 0x2C | `Lightning_Create` | | 0xE9 | `Drone_AIPoint` |
| 0x2E | `Lock_Create` | | 0xEA-0xEF | `Trigger_TouchOnce`/`Touch`/`MultiplexIn`/`MultiplexSIn`/`MultiplexOut`/`MultiplexOrIn` |
| 0x2F | `Copter_Create` 0x17d340 | | 0xF0 | `Pickup_Create` |
| 0x30 | `Monitor_Create` | | 0xF1 | `DroneSpawner_Create` 0x1378d0 |
| 0x31 | `FuseBox_Create` | | 0xF2 | `Emitter_CreatePlist` |
| 0x32 | `RainBox_Create` | | 0xF3 | `Env_Tree_Create` |
| 0x33 | `Hint_Create` | | 0xF4 | `Trigger_MoviePlayer` |
| 0x34 | `GunImp_Create` | | 0xF5/0xF8 | `Drone_AIVolume_Create` 0x1376c0 |
| 0x35 | `PCQWorm_Create` | | 0xF9 | `SoundTrigger_Create` |
| 0x36 | `Sub_Create` | | 0xFA | `ThirdIcon_Create` |
| 0x37 | `MiniSub_Create` | | 0xFB | `MusicTrigger_Create` |
| 0x38/0x39 | `Create_SpaceLaser`/`SpaceMissile` | | 0xFC | `Corona_Create` 0x18da00 |
| 0x3A | `Grapple_Create` 0x198298 | | 0xFD | `CamSubject_Create` |
| 0x3B | `DynamicObject_Create` | | 0xFE | `Mine_Create` |
| 0x3E | `Wire_Create` 0x1981d8 | | | |
| 0x3F | `CreepWall_Create` | | | |
| 0x41 | `InverterTrigger_Create` | | | |
| 0x4A | `Apocalypse_Create` | | | |
| 0x4E | `Light_Create` | | | |
| 0x71 | `rotor_init` | | | |
| 0xD2 | `Shooter_Create` | | | |
| 0xD4 | `Cloud_Create` | | | |
| 0xD5 | `OneSided_Create` | | | |
| 0xD8 | `RB_Create` | | | |
| 0xD9 | `SP_CreateScriptPlayer` 0x1db9a8 | | | |
| 0xDA | `Car_Create` | | | |
| 0xDB | `Door_Create` 0x136100 | | | |

## Snapshot check [L `work/ee_start_05.bin`, level 07000005 at start]

- Head: `u32 0x2705b4` (head+0x14) = `0x01319e30` (first object), `u32 0x2705b8` =
  `0x01fc9900` (last), and `u32(0x01fc9900+0x14)` = 0 — a consistent doubly-linked list.
- `DynamicObjCount` `u32 0x30ca98` = **523**, and walking `+0x14` from the head visits exactly
  523 objects.
- Class histogram (class byte +0xFF): 0×61, 1×275, 2×13 (drones), 3×1, 0x1B×8 (ScriptPlayers),
  0x1C×5 (doors), 0x1D×49 (triggers), 0x22×29 (searchlight parts), 0x27×13, 0x2B×2, 0x2C×1,
  0x2D×1, 0x2F×4 (pickups), 0x30×3 (spawners), 0x3B×2 (copters), 0x3D×15, 0x3E×9 (AI
  volumes), 0x40×5, 0x42×9, 0x46×14, 0x51×2, 0x52×2.
- Object `0x01382bc0` (drone, class 2): pos `+0x30` = (62.20, 34.82, 104.15); sphere `+0x80` =
  (62.28, 34.65, 104.21), radius `+0x8C` = 1.02 (a guard-sized sphere, offset from the origin
  because flags2 `+0xFA` = 0x0008 takes the local sphere from the AnimObject); `+0xF0` =
  0x00000880; `+0xF4` = 0x0029 (the SniperIdle state, mirrored from the drone's state machine);
  `+0xFC` = 2 (drawn last frame); `+0xFF` = 2; `+0xEC` = 0x0001A677.
- Door-class object check: 5 doors (class 0x1C) matches docs/systems/world-objects.md's live
  count; the searchlight count (29) matches its 7 mounts + 7 heads + 6 beams + 7 coronas + 2
  spots breakdown.
