# Characters: skinned meshes, skeletons, skins, and drone placement

Readers: `ps2gfx.rs` (skinned packets), `character.rs` (skeletons, skins), `level.rs` (`character_instances`).
CLI measurements: `nf-extract gfx-skin-words`, `skins`. Confidence: high for bind-pose geometry and the skin → mesh
link (*measured*, WORKLOG 0013); posing/animation not started (G-020).

## Skinned PS2_GFX objects (582 leaves on the disc)

- Footer word 2 (skin info) ≠ 0. Skin info: `+0` offset of the **source vertex buffer**, `+4` its size, `+8` offset
  of a 0xFFFFFFFF-terminated list of packet word indices, `+0xC` a further pointer (*measured*: `psiCreateEntityGfx`,
  `CacheSkin` 0x1112c0).
- Source vertex: 2 qwords — `qw0` = position (x, y, z, 1.0f); `qw1` = packed skinning data whose word 3 holds the
  no-kick flag in bit 15 (set on all 51,664 strip-start vertices; 25 % of the rest) and the morph index in the upper
  half-word (−1 = none, read by `SkinIt` as the short at +0x1E).
- At run time `CacheSkin` copies the packet and adds an output buffer's base to every REF tag named by the index list;
  `SkinIt` (0x10bf30) streams the source through the scratchpad and VU0 (microprograms at 0, 0xB8, 0x170), writing
  transformed (position, normal) pairs to the output buffer in the same layout. The REF tags then UNPACK those pairs
  (`STCYCL 3,2` / `2,1`) into the same VU1 slots static meshes use: position at B+1+3k, the second qword at B+3+3k;
  inline CNT data supplies the GIF tag, RGBA (B+2+3k) and UV (B+0xC7+k). One REF per batch starts at address −16, its
  first element landing in the header slot that the GIF tag then overwrites.
- **Bind pose** = the source buffer itself: decoding with REF data taken from it passes every VU-memory freshness
  check, and the glist box min/max equal the decoded positions' bounds **bit for bit in all 582 skinned leaves** — the
  source positions are model space (Y up, origin at the hips, e.g. y −1.16 … 0.75).
- Totals with skinned objects: 7,154,267 vertices, 4,697,239 triangles (golden `geometry.tsv`).

## Skeletons (type 6, `SKELnnnn`) — `AnimSkeletonProcess` 0x1bd328

`+0 u16` skeleton id (index into `pSkeletons`), `+2 u8` bone count, then per-bone data (not yet used).

## Skins (type 3, hashcode prefix 05) — `AnimProcessSkinData` 0x1bd500

```
+0x00 u32 hashcode   +0x04 f32[3]   +0x10 u8 main_count   +0x11 u8 extra_count   +0x12 u8   +0x13 u8 tail_count
+0x14 u8 skeleton id, then bone_count(skeleton) bytes
if extra_count: extra_count bytes, align 4, extra_count × u32 hashcode (−1 none)
if main_count:  align 4, main_count × u32 hashcode (−1 none; hash byte 2 bit 0x10 → AnimSleeveGetEntity substitute)
then per-bone quaternion+translation matrices, tail_count × 0x24 bytes
```
All 1,217 skins on the disc parse with their archive's skeletons. Main entries: 582 resolve to exactly the 582
skinned objects; 627 are sleeve substitutes.

## Placement

Characters are `oDrone` placements (flag 0x0F; also 8 animated `Tree`s) whose parameter 0 is a skin hashcode: 522
skins found in the same archive, 33 not (another archive, e.g. the common animation file; G-021). nf-view draws each
skin's main meshes at the placement transform; feet land on the floor (07000005 gatehouse), so the translation is at
foot level while the mesh origin is at the hips — consistent only if the bind-pose model's feet are at y ≈ −1.16 and
the placement y is 1.16 above the floor; not yet checked against the game (U-026).
