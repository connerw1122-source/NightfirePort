# obj_tag (the generic object header, 0x110 bytes; the instance follows at obj+0x110)

Proof levels: see [README.md](README.md). This doc upgrades docs/atlas/object.md's field table to
the field-map standard: every row now states its reader-list status, and the rows re-verified this
session carry their decompile lines. The class-3 (player) reading of the same header lives in
docs/fields/obj-tag-player.md (branch s/044x-fields-player). Allocation: `control_create_object`
0x1344e0 (`Mem_Malloc(instance_size + 0x110)`, memset 0); init `control_init_object` 0x134630;
lists, cel links and the update order are docs/atlas/object.md's §Lists/§Update order (still
authoritative). Level: ee_start_05 (07000005 at start, 523 objects).

## Field table (byte offsets)

| Off | Type | Meaning | Proof status |
|---|---|---|---|
| +0x00/+0x04 | ptr | ForcedList prev/next (membership when +0xF0 & 0x30000000) | [M*] `LList_Add` 0x1cca28 / `LList_Remove` 0x1ccac8 + `Controls_StraddleTest` 0x134778:40-56 (add/remove + flag sets — verified this session) |
| +0x08/+0x0C | ptr | cel-list next/prev (per-cel draw list) | [M*] `control_link_object_to_cel` 0x1346b8; `View_AddObjects` 0x1e62d8 walks +0x08 |
| +0x14/+0x18 | ptr | DynamicObjList next/prev (newest-first: add prepends) | [M*] 0x134698 / 0x133178; snapshot-verified walk of 523 [L] |
| +0x1C | ptr | proxy obj (share a cel with) | [M1] `Control_CreateObjEx` 0x1343c8 param 7; movement-handler relink 0x1336f8 |
| +0x20 | ptr | the cel this object sits in | [M*] link/relink fns; relink pass reads `(obj+0x1C)->+0x20` |
| +0x30 | vec3 | position | [M*] create arg, movement pass, `flags2 & 0x40` copy from +0xC0; every per-class update |
| +0x40 | vec3 | previous position | [M1] movement pass copies +0x30..+0x3F after a move; readers: collision deltas |
| +0x50 | vec3 | rotation x/y/z (rad), ±π-wrapped by the movement pass when flags2 & 0x20 | [M*] create, matrix rebuild, wrap 0x1336f8 |
| +0x60 | vec3 | prev rot / direction: copied from +0x50 when flags2 & 6 == 0, else `Mat_GetDir(+0x60, +0x90)` | [M1] movement pass 0x1336f8; per-class readers |
| +0x70..+0x7C | vec3+f32 | previous world sphere (centre + radius) | [M1] `Control_BuildWorldSph` 0x133070:19-22 copies +0x80..+0x8F first (verified this session) |
| +0x80 | vec3 | world sphere centre: glist local sphere +0x10 × matrix, or matrix row 3 (+0xC0) when glist-less, or `*(obj+0xDC)+0x80` when flags2 & 8 | [M*] 0x133070:24-40 (glist path verified this session); readers `View_AddObjects` (`Vision_InView`), triggers/collisions |
| +0x8C | f32 | world sphere radius: `glist+0x1C · abs(+0xE8)`, or `*(+0xDC)->+0x8C · abs`, or 0.1 (kept if flags2 & 0x100) | [M*] 0x133070 same block |
| +0x90 | MATRIX | 4×4: rotation rows 0-2, translation row 3 at +0xC0 | [M*] `View_RotTransMatrix` 0x1344e0 path, `Control_GetObjMatrix` in BuildWorldSph (0x133070:18 verified) |
| +0xC0 | vec4 | matrix translation row | [M1] matrix writers; flags2 & 0x40 teleport +0x30 ← +0xC0..+0xCC |
| +0xD0 | ptr | HITDATA list head (entries: next at +4, entry+0x58 may reference another object) | [M*] `Collide_Update` 0x1ea398 fills; `Collide_FreeHitList` 0x1ede00 frees; delete pass clears +0x58 refs; `Destroy_CollisionHandler` 0x12bd48:22-58 walks it (verified this session) |
| +0xD4 | ptr | class scratch (drone: instance+0x3D0) | [M1] `NDrone2_DefaultInit` 0x14b300 |
| +0xD8 | ptr | glist (`celglist_tag`) — docs/fields/glist.md; 0 = glist-less (renderer skips, 0.1-radius sphere) | [M*] `Control_SetGList` 0x134960:15 + creators (`Trigger` 0x1b1720:92, `FuseBox` 0x1b0448:25, `Wire` 0x1981d8:24 — verified); readers `View_AddObjects` 0x1e62d8:27, `BuildWorldSph` 0x133070:24, `Destroy_Update` 0x12c4f8:14,27 |
| +0xDC | ptr | AnimObject (draw object): +0x110..0x113 four LOD fade bytes, +0x62 class/weapon byte, +0x90 trans callback | [M1] `AnimObjectNew` DefaultInit; fade bytes memset 0xFF by the movement-handler tail when `GameState[0x40] != 0`; `Player_SSGrapple` reads +0x60 (verified this session) |
| +0xE0 | ptr | instance = obj+0x110 (in-allocation) | [M*] 0x1344e0 (`__s[0x38] = __s + 0x44`); EVERY instance access goes through it (all the per-struct docs) |
| +0xE4 | u32 | SP handle: second arg of `SP_RemoveObj(obj, +0xE4)` on delete; written by `Script_EntityStart` for script-owned entities, 0 on placement SPs | [M1] `SP_CreateScriptPlayer` 0x1db9a8 path; `control_delete_object` 0x133178:28 reads; [L] 0 on all 8 placement SPs (script-player.md) |
| +0xE8 | f32 | scale, 1.0 at init | [M1] 0x134630; sphere radius ×abs; searchlight corona parts animate it |
| +0xEC | u32 | frame/wobble: init `GameState[52]` [0x134630]; `Drone_Create` overwrites with `Rand_Rand(10000)` [0x136be0:70-71 — verified this session]; [L] reads 0x1A677 > 10000 on drones — an unidentified later writer exists (atlas note stands) | [M,L] |
| +0xF0 | u32 | draw/flags word: bit 0x1 alpha list, 0x10 not drawn, 0x20 disabled-pair, 0x400 cleared by drone init, 0x800 ambient-ramp gate, 0x2000 depth bias/LOS corona, 0x8000 skip draw / straddle mark, 0x10000000/0x20000000 ForcedList, 0x30008000 passthrough. **Copied from glist+0x44 at create** (masked `old & 0x30008000`) | [M*] `Control_SetGList` 0x134960:16-18 + every creator (`Trigger` 0x1b1720:95, `FuseBox` 0x1b0448:27, `Wire` 0x1981d8:27, `Destroy` via SetGList 0x12c318:23 — verified this session); `View_AddObjects` skips `& 0x8010` |
| +0xF4 | u16 | per-class state (door 2/4/8/0x10; drone: mirrored SM state) | [M1] per class; movement-handler world pass |
| +0xF6 | u16 | per-class scratch (drone init writes 1; CamSubject's photograph channel lives here [M 0x1bc5d8:22 — verified]; the ladder's kind scratch [M ladder.md]) | [M1 per class] |
| +0xF8 | u16 | display-mask bits, init 0x1F; bit 0x10 cleared when glist+0x44 has 0x40000 | [M*] 0x134630, `View_AddObjects` filter, `Control_SetGList` 0x134960:19-21 (verified) |
| +0xFA | u16 | flags2: 0x1 OnlyScaleZ, 0x2/0x4 rotation-changed, 0x8 sphere from AnimObject, 0x10 depth bias, 0x20 moved this tick, 0x40 teleport, 0x80 local sphere from AnimObject, 0x100 keep radius | [M1] `Control_CreateObjEx` param 9 OR-in, per-class updates, movement pass; `Trigger_Create` sets 0x4 when count != 0 (0x1b1720:89-91 verified) |
| +0xFC | u8 | draw counter: init 2, decremented per tick, set back to 2 by `View_AddObjects`; != 0 = drawn recently | [M1] 0x134630, 0x1e62d8, movement pass |
| +0xFD | u8 | per-class bit (door portal bit 3) | [S→M1] `Door_Link2Portal` 0x136788 `(>>3)&1` (door.md) |
| +0xFE | u8 | control flags: bit 0 = delete me (movement pass → `Effect_RemoveSObjs` + delete), bit 1 = rebuild sphere (init sets; BuildWorldSph + `Lights_CalcAmbientLight` then clear), bit 4 seen (drones) | [M*] 0x134630, delete pass 0x1336f8; `Searchlight_Update` kill path sets bit 0 on parts [M 0x1cb900 — searchlight.md] |
| +0xFF | u8 | class = `control_funcs` index | [M*] creators write it (verified this session: Trigger 0x1b1720:93 → 0x1D, Destroy 0x12c318:22 → 0x21, FuseBox 0x1b0448:23 → 0x3C, Wire 0x1981d8:26 → 0x2C, Corona 0x18da00:19 → 0x01, Door 0x136100:62 → 0x1C); movement pass + delete dispatch on it |
| +0x100..0x106 | 7×u8 | ambient RGB (0x100..0x102) + tint RGBA (0x103..0x106) | [M*] — full map in docs/fields/leaf.md |

## Reader-list status legend

[M*] = every reader/writer found and agreed (grep census + trace of the base pointer);
[M1] = the listed functions verified by hand, the tail is the atlas's (single-reader or
per-class-local — the field is only safely read within that class). The [M1] rows are exactly the
ones a live census (gdbwatch) should confirm before the port leans on them cross-class.

## Live checks

The header is per-object; the interesting live probes:
1. `+0xF0` bit 0x10 on a door glist (glist.md's check) — verifies the flags copy end-to-end.
2. `+0xFE` bit 0: `poke` it on any object — the object disappears next tick (the delete pass).
3. `+0xFC`: `watch` it on an in-view object — it should pin at 2 (renderer) and count down when
   the object leaves the view; the anim-early-end reads at 0 are the observable effect.
4. The census to run (gdbwatch) for the [M1] rows: walk DynamicObjList head 0x2705a0 → first
   object, byte range +0xF0..+0xFF, `readwrite`, 4 s of play on 07000005 — expected accessors:
   `View_AddObjects` (+0xF0 read, +0xFC write), the movement handler 0x1336f8 (+0x40/+0x60
   copies, +0xFE test, +0x50 wrap), `Control_BuildWorldSph` (+0x80/+0x8C writes), the per-class
   updates (+0xF4 writes). An accessor outside this list = a missed reader.
