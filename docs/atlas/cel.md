# The cel / room (`cel_tag`, `portal_tag`, `celglist_tag`)

Plan v2 §2 struct 12. Cels are the rooms of the world: each holds its object list, its portal list,
its collision reference, its draw resource and the ambient colour that lights the objects inside it.

## cel_tag — 0xb0 bytes [M]

Allocated by `build_alloc_cel(cel_tag*)` @0x1c93f8: `Mem_Malloc(0xb0, 0x104, 0)` [M]; heap stride
observed 0xc0 (0xb0 + 0x10 header) [L]. Each runtime cel keeps a back-pointer to its source record:
the level parse produces a **0x50-byte record array** (`pCelList` @0x30cfb0), and
`parsemap_block_portal_data` @0x1d15f8 matches `runtime_cel+0x38 == pCelList + idx*0x50` [M].

| off | type | name | meaning | evidence |
|---|---|---|---|---|
| +0x08 | ptr | obj_head | first object of the cel's object list (doubly-linked with the cel as anchor) | [M 0x1346b8] |
| +0x0c | u32 | anchor | cleared/maintained by the link helpers | [M 0x1346b8] |
| +0x14 | ptr | next | next cel; `world+0` = first cel (build_FindCel walks it) | [M 0x1c8cb0, 0x1c93f8] |
| +0x28 | ptr | portals | portal list head (`portal_tag` nodes) | [M 0x1d15f8, 0x1e0d58] |
| +0x38 | ptr | src | back-pointer to the 0x50-byte record in the pCelList array | [M 0x1d15f8] |
| +0x3c | u32 | flags | bit 0x40000 = include in the room cel collection (FindCel); snapshot value 0x0804c047 | [M 0x1c8cb0] [L] |
| +0x40..0x58 | 6×f32 | bbox | min XYZ (+0x40/+0x44/+0x48), max XYZ (+0x50/+0x54/+0x58); defaults ±1.0 | [M 0x1c8cb0] [L] |
| +0x90 | f32 | ? | default −500.0 | [M 0x1c93f8] |
| +0x94 | u32 | flags2 | View_AddCels tests bits 0x1 / 0x2000 / 0x40000 (meanings unverified) | [M 0x1e6ba0] |
| +0x9a | u8 | **ambient R** | default 0x50 | [M 0x1c93f8, 0x1cb178] |
| +0x9b | u8 | (2nd triple R) | a second, parallel RGB triple the lighting code ignores; loader-written on some cels | [L] |
| +0x9c | u8 | **ambient G** | default 0x50 | [M] |
| +0x9d | u8 | (2nd triple G) | malloc fill 0x98 proves untouched on unlit cels | [L] |
| +0x9e | u8 | **ambient B** | default 0x50 | [M] |
| +0x9f | u8 | (2nd triple B) | — | [L] |
| +0xa0 | u8 | ? | loader-written 0/1 | [L] |
| +0xa1 | u8 | ? | 0 | [M] |
| +0xa2 | u8 | **lighting zone** | `switch_channels[zone]` (table @0x26fc90) is a **byte offset** added to the cel pointer before every ambient read: `base = cel + switch_channels[cel+0xa2]` | [M 0x1cb178, 0x1346b8] |
| +0xa3 | u8 | ? | default 0x10 | [M 0x1c93f8] |

## The ambient colour (cel+0x9a/0x9c/0x9e) — who reads it

All access is per-byte (`lbu`/`sb`), always through the zone offset [M]:

- **`Lights_CalcAmbientLight(obj)` @0x1cb178** (called per moving object from
  `control_movement_object_handler` 0x1336f8): gate `obj+0x20 (cel) != 0 && obj+0xF0 & 0x800`;
  targets R/G/B = `base+0x9a/0x9c/0x9e`; ramps `obj+0x100/0x101/0x102` toward them with
  `ClrStep(…, 0.1)` (`0x3dcccccd`); if `obj+0xF0 & 0x10000` and `glb_blokes+0x962 != 0`, all three
  targets double (clamp 0xff) — the night-vision-ish boost. (The exact decomp was re-read this
  session — the earlier "0.025" note was a transcription error; the constant is 0.1.)
- **Snap-on-link**: `control_link_object_to_cel` 0x1346b8, `control_handle_cel_change` 0x133288 and
  `build_LinkToRoom` 0x1c9680 — when an object links into a cel with `obj+0x100..0x102 == 0xffffff`
  (the sentinel set by `control_init_object` 0x134630), the triple snaps to the cel's ambient [M].
- **No alpha is read anywhere**; **no sound-code reader found** — the SFXSetEnvironment/reverb use
  of cels (G-032) is a different code path, unconfirmed to share these bytes.

`obj+0x100..0x102` is the per-object ambient-light colour (see gs-leaf.md); `obj+0x103..0x106` is
the separate tint, default white.

## Portals (map block 0x21) [M]

- Dispatch: `parsemap_handle_block_id` @0x1d1020 `case 0x21` → **`parsemap_block_portal_data`**
  @0x1d15f8. File layout: count = u32 @FileNextBlock+4; records start @+8, **stride 56** —
  *measured* [D] on the disc (fix-perf-level): 07000006's main-map block is 2468 bytes with count 44
  = 4 + 44·56 exactly; the earlier "stride 68" reading was wrong (the audit note below flagged it).
  - +0x00..0x2f: four corner VECTORs (the door/portal quad);
  - +0x30: u16 **celA index**, +0x32: u16 **celB index** (into the pCelList record array);
  - +0x34: u16 flags (bit 0 → A-side alloc flag `(flag&2)==0`; bit 1 → B-side `(flag&1)^1`);
    +0x36: u16 pad. All 44 flags words on 07000006 are 0 [D].
  Two plane equations are built from the quad (`Plane_PlaneEq` @0x1df8d0: n = normalize(cross(v1−r,
  v2−v1)), d = −dot(n,r), called (out, ref=c0, v1=c1, v2=c2) forward and over the reversed corner
  copy for the B side) — one per side; *measured* [D/M]: the forward normal points into celA on
  137/137 resolvable records across 07000002/05/06/07 (one 07000002 record degenerate).
- The **pCelList record index is the placement's own `index` field**: `parsemap_block_map_data_static`
  @0x1d1998 computes `pCelList + placement.index * 0x50` for by-index placements [M] (the by-hash
  path resolves through the entity hashtable — untraced; portals naming those records are
  unresolvable in the port). *Measured* [D]: with `index`-keying, 0 of 37/21/44/34 portal endpoints
  drop on 07000002/05/06/07; an ordinal-based indexing dropped 22 on 07000002.
- Runtime **`portal_tag`** — `build_alloc_portal(plane_eq, next, dest_cel, flag)` @0x1c8928,
  malloc **0xa0**: +0x00 next portal; +0x04 **twin** (parser sets `P1↔P2`); +0x08 **destination
  cel**; +0x10 plane_equ (16B); +0x0C door object (0 until `build_LinkDoors2Portals`); +0x24 u16 =
  0xFFFF; +0x26 u8 seen flag; +0x27 u8 side flag; +0x30..0x6F the four corner VECTORs (forward
  order on the celB node, reversed on the celA node); +0x70 quad centre; +0x80/+0x8C bbox min−0.1 /
  max+0.1. The celB node carries the forward plane with side flag `(flags&2)==0`, the celA node the
  reversed plane with `(flags&1)^1` [M 0x1d15f8].
- Storage: each of the two cels gets its own portal node on **cel+0x28**; twins cross-link; each
  node's +0x8 points at the opposite cel.

## Visibility — the portal walk (Vision_* 0x1e7df8..0x1e89e0) [M] (fix-perf-level)

Per viewer (`View_CaptureScene` 0x1e6df0): clear `cel+0xa0` visited flags over the world cel chain;
`Vision_Init_Portal_Recurse` (0x1e89a8): `VisCelCount = 0`, `ViewPoints[0]` = the eye,
`Vision_Calculate_View_Fustrum` (0x1e7df8) builds the depth-0 planes: the near quad at 200 units
along the view axes (half-extents `tan(fov/2)·200` and `tan(fov/2)·aspect·200`), then **five**
`Plane_PlaneEq`s through the eye and consecutive corner pairs plus one through three corners (the
near plane) into `PortalPlanes[0]` (0x2d6a90, 48 depth slots × 10 planes); `NumPortalPlanes[0] = 5`;
the first four planes are copied to viewer+0x70 (what `Vision_InView` tests).
`Vision_Portal_Recurse` (0x1e86e8): add the viewer's cel to `VisCelList` (0x34b3b0, pairs of
{cel, u16 depth}, cap 0x40) and run **`Vision_Recurse` (0x1e82e0)**: for each portal of the current
cel's +0x28 list — skip when the side flag (+0x27) is 0, when `dot(plane, ViewPoints[0]=eye) >= 0`
(the portal faces away), or when `Door_IsClosed(portal+0x0C)` [M 0x1367b0]; copy the four corners
into the working polygon, clip it against `NumPortalPlanes[depth]` planes with
`ClipPortalWithPlane` (0x1e7fa0: keep `value >= 0`; returns 0 = fully inside → polygon unchanged;
the √1e-7 successive-duplicate compaction runs on every clipped result); a non-empty polygon marks
portal+twin seen (+0x26), adds the destination cel (+0x8) to `VisCelList` (dedup by scan; `cel+0xa0
= 1`), and — when the polygon keeps ≥ 3 corners — builds the next depth's planes: one
`Plane_PlaneEq` per polygon edge with **ref = the eye** (`ViewPoints[0]`), plus the portal's own
plane last; `NumPortalPlanes[depth+1] = corners+1`; recurse (budget: `VisionRecurseCount > 100`,
depth > 45 stop). Afterwards `VisCelList` is QuickSorted (stride 8, comp 3) and threaded into the
`GfxList` chain via `cel+0x20`; **`View_AddCels` (0x1e6ba0) adds ONLY these cels' glists** (+ the
cel's own objects; the +0x18 sibling chain still passes `Vision_InView`, the 4-plane viewer+0x70
test with radius slack [M 0x1e89e0]). The walk replaces fix-render3's box test as the visibility
authority (G-016's portal half); `switch_ForceDrawAll` bypasses it (0x1e6df0).
Port: `nf_view::scene::{parse_cel_graph, portal_visible_cels}` + the `Renderer` hookup; measured on
07000007: start view 4526 → **177** World draws, away 64 → 62, left 1379 → 81, right 148 → 81,
down 145 → 80.

## Object ↔ cel assignment [M]

- **`build_LinkToRoom(obj, Sc doStraddle, world)` @0x1c9680** — the second arg is **not a room id**
  (earlier reading): it is a bool "run `Controls_StraddleTest`" flag; all internal callers pass 0.
  The cel is *found*: builds a VECTOR from `obj+0x80` (world pos) and calls `build_FindCel`
  0x1c8cb0. Guards: `obj+0xF0` bit 0x20000000 = skip; bit 0x10000000 = defer (flags |=
  0x10008000, `LList_Add(0x270590, obj)` — the deferred-link list). On success:
  `control_unlink_object`, `control_link_object_to_cel`, ambient snap.
- **`control_link_object_to_cel(obj, cel)` @0x1346b8**: doubly-linked insert with the cel as
  sentinel — `obj+0x8` = prev object, `obj+0xc` = next object (head's next = the cel pointer
  itself), `cel+0x8` = first object, `obj+0x20` = cel. cel == 0 → falls back to `build_LinkToRoom`.
- **`control_handle_cel_change(obj)` @0x133288**, called per moving object from
  `control_movement_object_handler` 0x1336f8: if `obj+0x20 == 0` → build_LinkToRoom; if the position
  changed (`obj+0x70` vs `obj+0x80`): `Collide_StraddleCels` — ≥2 cels ⇒ defer to the ForcedList;
  else **`Cel_ObjectLeftCel(cel, from, to)` @0x1e0d58**: raycasts the move against the current cel's
  portal list (per-portal BoxBox pre-cull vs portal+0x20/+0x24, then two ray-triangle tests on the
  quad corners +0xc/+0x10/+0x14 and +0x14/+0x18/+0xc) and returns **portal+0x8 = destination cel**;
  hit ⇒ relink directly (no FindCel); miss ⇒ `build_point_in_bounding_box` (0x1c9500) — still inside
  ⇒ done; else full relink via build_LinkToRoom; if that fails, the class byte selects the fallout:
  0x2 → `Drone_FellOutMap`, 0xd → `View_SetDrawInNoViews`, 0x28 → `Car_Deactivate`, default → clamp
  into the old cel's bbox using radius `obj+0x8c`.

## celglist_tag (brief)

The per-model interned draw/gfx resource, resolved by hash: `hashtable_hashcode_to_celglist(u32)`
@0x1ca798. `Control_SetGList(obj, glist)` @0x134960: `obj+0xD8 = glist`; `glist+0x44` (u32 flags)
merges into `obj+0xF0` (the object keeps only its 0x30008000 bits); glist flags & 0x40000 clears
`obj+0xF8` bit 0x10. Consumers: `psiCreateEntityGfx` 0x1171b0, `psiDrawObjectMatrix` /
`psiDrawClonedObject` / `psiDrawSkinObjectMatrix`, `Break_Create`, `Car_Create`, `MP_CreateObject`.
Fields beyond +0x44 not yet mapped. (The wire.rs comment reading a glist hashcode out of wire
placement params was wrong — see wire-grapple.md.)

## Visibility (View_AddCels 0x1e6ba0) — summary with a caveat

Per viewer: from the world/gfx node list, iterate rooms/cels, skip flagged-out cels
(cel+0x44 bit 0x10, cel+0x94 bit 0x40000 unless `object_display_mask == 0x10`), frustum/bbox test
`Vision_InView(cel+0x8c, cel+0x80, viewer)`, push `{key = ±squaredist, cel}` into the viewer's ring
buffer and call `View_AddObjects` 0x1e62d8. **Caveat [S]**: the room-node vs cel-node field split
(+0x18/+0x20/+0x3c/+0x94) is a reconciliation of two decompile passes; the portal-graph traversal is
not visible in View_AddCels as decompiled and was not disassembled this pass — where the walk uses
cel+0x28 portals (View_AddObjects / View_DrawCelGlist) remains open (G-016).

## Snapshot check [L] (work/ee_start_05.bin)

| addr | raw | meaning |
|---|---|---|
| 0x30cfb0 | `0170af90` | pCelList (record array) |
| 0x30cfb4 | `0170e730` | pCurrCelList; delta = **0x37A0** = 0x50 × **178 cels** (the earlier "180/0x3780" was an arithmetic slip, caught by the audit) |
| 0x30d0bc | `0001` | VisCelCount = 1 (start view) |
| 0x30d0f8 → +0 | `01fd7de0` → `016f12a0` | glb_world → first runtime cel (matches build_FindCel's walk) |
| cel 0x016f12a0 +0x9a..0x9f | `1b 3c 28 3c 38 3c` | ambient = (0x1b, 0x28, 0x38) = (27,40,56) — dim blue: the night-outdoor sky ambient of 07000005; odd bytes 0x3c = the ignored second triple |
| cel 0x016f1360 +0x9a..0x9f | `50 98 50 98 50 98` | defaults 0x50 with malloc fill 0x98 between — this indoor cel's ambient was never overridden |
| cel 0x016f12a0 +0x40.. | (−1104.45, 71.95, −965.26) … (1262.62, 1162.30, 1428.59) | level-wide outdoor box |
| cel 0x016f1360 +0x40.. | (87.84, 18.51, 91.95) … | room-sized indoor box |
| cel 0x016f12a0 +0x28 / +0x38 / +0x3c | `016f0b10` / `0170e5a0` / `0804c047` | portal list head / src = pCelList + 172·0x50 (exact) / flags with 0x40000 |
| obj 0x0137b5b0 (class 0x3d) | +0x20 = `016f6e20`, +0x100..0x102 = `39 46 56` | a live object→cel link; its snapped ambient (57,70,86) — blue-ish, consistent with the snap mechanism |

## Not verified this pass

- View_AddCels' room/cel node split and the portal-graph traversal in rendering (see caveat).
- The odd-byte triple (0x9b/0x9d/0x9f), cel+0x90 (−500.0), +0xa0, +0xa3 meanings.
- `parsemap_handle_block_id` case 0x21 body beyond the decomp listing; the pCelList 0x50-byte record
  internals beyond the celA/celB indices; `hashtable_hashcode_to_celglist` internals.
- Where `VisCelList` (0x34b3b0) really stores (its snapshot slot holds a name/world pointer, not
  cel pointers).
- The block-0x21 record layout (stride 68, celA/celB at +0x30/+0x32, portal alloc 0xa0, twin at
  +0x04) rests on the original parse of `parsemap_block_portal_data`/`build_alloc_portal`; the
  independent audit re-verified only the runtime half (portal list head at cel+0x28, destination at
  portal+0x8) before running out of budget. Re-measure on contact.
- No sound-code reader of cel+0x9a..0x9e (SFXSetEnvironment relation unconfirmed).
