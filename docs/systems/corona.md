# Coronas (glow quads) — placement path ported, runtime class dead (U-042 closed)

Task `find-corona-creator` (2026-10-15): hunt the creator of the runtime class-0x45 coronas
(U-042, G-034). Result: **no creator exists on the disc's paths — `Corona_Update` is dead code.**
This page carries the full mechanism and the caller scan that proves it.

## The dispatch mechanics (how a class becomes an update)

- `control_create_object(class_size, pos, quat)` 0x1344e0 mallocs `class_size + 0x110` bytes,
  memsets them, runs `control_init_object` 0x134630 (`+0xF8 = 0x1f`, `+0xFC = 2`, alpha bytes
  `+0x100..+0x106 = 0xFF`, `+0xE8 = 1.0`), and links the object into `DynamicObjList`.
  **It never writes `obj+0xFF`** — the class byte must be stored by the creator afterwards.
  `Control_CreateObjEx` 0x1343c8 does not write it either (it sets `+0xE8/+0xFA/+0xD8/+0xF0/`
  `+0x103..05/+0x1C`). `control_init_object` does not touch it. [M 0x1344e0:11-38, 0x1343c8,
  0x134630]
- The class byte is **`obj+0xFF`**: `Control_DeleteAllObjectsOfType(Uc)` 0x1349e8:11 and
  `Control_ReturnNextObjectOfType(Uc, obj)` 0x134a38:15 compare `*(char*)(obj+0xFF)` against
  their type argument. [M]
- The per-tick dispatcher `control_movement_object_handler` 0x1336f8:62-64 walks the list and
  calls `control_funcs[class]`'s first word with the object, **stride 0xC** — the table is
  85 entries × 12 bytes (update / collide / third), not 255 pointers. `control_delete_object`
  0x133184-0x1331a0 reads entry+8 the same way (a destructor slot). [M]
- `control_funcs` @ 0x26f890 in .data (0x3fc bytes) is **read-only in code**: exactly two
  .text references exist (0x133184 delete path, 0x1337f0 dispatcher), both `lw` with
  `mult class, 12`. No runtime patching. [M table + raw scan]

## The corona table row

`control_funcs[0x45] = { update: Corona_Update 0x18da78, collide: NULL, third: NULL }`
[table dump @0x26f890 + 0x45·0xC = 0x26fbcc; M]. This is the link-time "slot assignment" the
task asked about — there is no other.

## Corona_Create 0x18da00 (the placement path — ported)

`parsemap_create_dynamic_objects` 0x1d04f0 case **0xFC** (0x1d04f0:525-527) is the ONLY caller
in ACTION.ELF (whole-decompile grep). It calls `Corona_Create(pos, quat, level, glist)` which
[M 0x18da00:11-21]:
1. `control_create_object(0, pos, quat, 0)` — malloc 0x110, zeroed;
2. `obj+0xD8 = glist`; `obj+0xFF = 1` (**class 1**); `obj+0xF0 = *(u32*)(glist+0x44)` (the
   glist flags word — bit 0x1 puts the quad in `View_AddObjects` 0x1e62d8 draw list 1, the
   alpha list);
3. `build_LinkToRoom(obj, 0, glb_world)`.

So the 13 type-0xFC placement coronas on the disc's levels are class-1 objects with
`control_funcs[1] = {NULL, NULL, NULL}` — **no update runs, no RNG is consumed**, and the
viewer's static-glow-quad port (`crates/nf-view/src/scene.rs`, `nf_game::corona::placements`)
is the complete behavior. (census: 5× `Corona_Glow2`, 7× `Window_corona`, 1× `Corona_test1`
on 07000005 [L ee_start_05 / WORKLOG 0073].)

## Corona_Update 0x18da78 (the unlaunched runtime fade — documented, not ported)

For the class-0x45 object that never exists, the update would do [M 0x18da78:12-35]:

- Guard: run only when `(GameState+0x34 & 1) != *(short*)(obj+0xF4)`. `obj+0xF4` is a generic
  per-object short (Bullet_update increments it; creates zero it). Nothing in the corona path
  writes it, so with the create-time 0 the body runs on **GameState-odd parity ticks** (every
  other tick).
- If `obj+0xFC == 0` → `obj+0x106 = 0` (the alpha byte is forced to invisible; the renderer's
  2→1→0 visible-state cycle keeps it out).
- Else run the occlusion ray `Collide_LineOfSight(glb_viewer+0xC0, obj+0x30,
  *(glb_viewer+4), obj, glb_players, 8)` — viewer eye → corona centre.
  - Occluded: `alpha = max(0, alpha − 0x10)`; `obj+0xF0 &= ~0x2000`.
  - Clear: `alpha = min(0xFF, alpha + 0x10)`; `obj+0xF0 |= 0x2000`.

Fade rate ±0x10 per parity tick (≈ ±0x10/2 per frame at 60 Hz → full 0↔0xFF sweep in ~16
ticks of clear sight), the alpha byte is `obj+0x106` (the same byte
`control_init_object` seeds 0xFF and the port uses as the quad's alpha), and bit 0x2000 of
the display-flags word tracks "line of sight clear". No RNG anywhere.

## The proof that class 0x45 is dead (the full caller scan)

Every way a class-0x45 object could come to exist, checked:

1. **Direct class stores.** A raw-opcode scan of ACTION.ELF's .text (322,249 words) finds
   exactly **110 `sb rt, 0xff(base)` stores**; no `sw …,0xFC`/`sh …,0xFE` stores on object
   bases (the 16 hits are unrelated structs — AI routes, menu labels, MPEG state; the only
   sq-class hits are VU0 `sdc2 …,0xF0($sp)` stack spills), and no `memcpy`/`amemcpy`
   targets an object's `+0xF0..0xFF` window. All 110 stores resolve to immediate class
   values via the decompile + register tracking — the three that read as parameter-fed in
   the disassembly are constants loaded into the arg register
   (`Hurt_Create` `addiu $a2,$zero,0x27` @0x17e93c; `SS_Create` `addiu $a2,$zero,0x25`
   @0x1a9d40; `SoundTrigger_Create` `addiu $a3,$zero,0x40` @0x1b2704). The full value set:
   `{0,1,2,3,5,7,0xC,0xD,0xE,0xF,0x10,0x11,0x15,0x1A,0x1B,0x1C,0x1D,0x20,0x21,0x22,0x23,`
   `0x24,0x25,0x26,0x27,0x28,0x2A,0x2B,0x2D,0x2E,0x30,0x31,0x33,0x35,0x36,0x37,0x38,0x39,`
   `0x3A,0x3B,0x3C,0x3D,0x3F,0x40,0x44,0x46,0x47,0x48,0x49,0x4A,0x4B,0x4D,0x4E,0x4F,0x50,`
   `0x51,0x52,0x54}`. **0x45 is not in it.** [M]
2. **`control_create_object` callers.** No caller passes a literal 0x45 anywhere (the first
   argument is the instance size anyway, not the class). [M whole-decompile grep]
3. **`control_funcs` slot assignment.** The table is static .data, read-only in code (see
   above); the only corona row is `[0x45]` itself. [M]
4. **`Script_CreateEntity` 0x1d8968.** Its kind switch creates `Break` (0x20), `Ladder`
   (0x2D), `Switch` (0x39), `Destroy` (0x21), `Trigger_TouchOnce/Touch` (0x1D), `Pickup`,
   `Drone_AIVolume` (0x2F), `Emitter_CreatePlist` (0x32) or a default class-0 object — never
   a corona, never 0x45; its reuse path only re-matches objects whose class equals the same
   mapped kinds. [M 0x1d8968:35-176]
5. **Searchlight / light builders.** `Searchlight_Create` 0x1cb4f8 → class 0x22. `Light_Create`
   (called from `GunImp_Update` etc.) writes no class byte — lights are not control objects.
   [M]
6. **Vehicle / explosion code.** `Car_Create` 0x28, `Sub_Create` 0x4A, `MiniSub_Create` 0x4D,
   `Copter_Create` 0x3B, `Destroy_Create` 0x21, `Debris_Create(Ex)` 0x10 — none 0x45. [M]
7. **Direct calls.** No `jal`/`j` to `Corona_Update` 0x18da78 anywhere in .text — the only
   entry is via the (dead) table row. [M raw scan]
8. **DRIVING.ELF** has no corona symbols at all. [M objdump -t]

## Verdict

U-042 **closed as dead code**: `Corona_Update`'s class-0x45 row is reachable only through
`control_funcs[0x45]`, and no code path on the disc (placement loader, script entity switch,
searchlight/light/vehicle/explosion creators, effects — the whole 110-store class catalog)
ever creates a class-0x45 object. The runtime LOS-fade corona never runs in the shipped game;
the ported static path is complete and nothing needs porting. If a live PCSX2 trace ever
shows `Corona_Update` executing, the creator must come through a path outside this scan's
coverage (e.g. a computed store from data we mis-attributed) — re-run the catalog first.
