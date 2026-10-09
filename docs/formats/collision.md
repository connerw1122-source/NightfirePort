# Collision geometry (map block 0x2E)

Parser: `parsemap_block_Coll_Data_New` (ACTION.ELF 0x1d1810); ray tests `Intersect_RayGeom` (0x10a8c8) with
`ASH_RecurseBoxesRay` (0x10af30) / `ASH_Intersect_RayBox`; segment queries `Collide_LineOfSight` (0x1edf50).
Code: `crates/nf-formats/src/collision.rs`; world: `crates/nf-game/src/world.rs`. *Measured* unless tagged.

## Layout (payload = block bytes after the id/size word)

```
+0x00 u32 version (4 or 5; all 7,985 blocks on the disc are 5)
+0x04 u16 box count
+0x06 u16 triangle count
+0x08 u16 vertex-block count (0x40 bytes each)
+0x0C (v5: + 16 more header bytes; +0x0C u32 is summed into OldCollisionSize)
data at +0x0C (v4) / +0x1C (v5):
  vertex blocks: count × 0x40 bytes of s16 (vertices = 3 s16; planes = 3 s16 normal + i32 distance)
  boxes: count × 0x30
    +0x00 f32 min[3] (the loader subtracts 0.01)   +0x0C s16 left child / leaf: 0x8000 | vertex-block count
    +0x0E s16 right child / leaf: first triangle    +0x10 f32 max[3] (loader adds 0.01)
    +0x1C u16 leaf: end triangle                    +0x1E u16 leaf: first vertex block
    +0x20 f32 origin[3], f32 scale (2⁻¹⁴ in every sample): vertex = s16 · scale + origin
  triangles: count × 8: u16 v0, v1, v2 (halfword indices into the leaf's vertex blocks), u16 plane (halfword index)
```

## Checks on the whole disc (`nf-extract coll-check`)

7,985 blocks parse; leaves cover every triangle exactly once; all 2,245,313 triangle vertices lie inside their leaf
box (±0.02); every plane normal (s16 · 2⁻¹⁴) has unit length; 2,242,859 normals equal +cross(v1 − v0, v2 − v0)
(146 the opposite, 477 disagree, 1,831 degenerate triangles).

## Ownership

A 0x2E block precedes the PS2_GFX and entity params of the object it belongs to (map order `… 2e 2d 04 …`; the
loader attaches it to `pCurrCelList`). Its root box matches the following object's render bounds within 0.5 in
6,959 of 7,985 blocks, the preceding one's in 1,618 (collision is often a simpler shape). `Level` attaches it to the
next object (`MapObject::collision`).

## Floors

In 07000005 the collision floor under each idle drone (feet sphere height `Drone+0xA0` = 1.0, not yet updated)
explains the live object y within 0.003 (5 drones; `nf-extract coll-floor`, snapshot WORKLOG 0018). nf-game uses the
collision floor instead of the rendered one.

Open: the plane distance encoding (i32 at the plane's halfword +3, scale unverified); per-triangle material/flags
(surface type for footsteps, bullet decals) — not found in the 8-byte record, maybe in the plane words.
