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

## Facial morphs (fix-facial)

- **`+0x12 u8` = the facial (morph) count** → runtime `AnimSkin+0x2e` [M `AnimProcessSkinData` 0x1bd500]. 22 on the
  dialogue skins (`05000007`, `050000ac`, `0500008f`, … — the count every archive's face-bearing skins share), which
  equals the cut-scene scripts' face tracks' 22 channels. `AnimObjectNew` 0x1be910 mallocs `count × f32` (zeroed) for
  the object's morph weight array (`animData+0xb0`; the sAnimObject's weights pointer reads `+0x40`).
- **Weights → face** [M `AnimFacialBlend` 0x1be648]: three facial script slots are looked up by marker
  (0xe0000000 idle / 0xe0000001 blink / 0xe0000002 talk) via `AnimListBuild(obj, 0x200, ~marker, 0, 1)`; each slot's
  playing sequence's **frame buffer (`sAnimSeq+0x84`, `AnimSeqNew` 0x1c7248's per-channel float evaluation — for a
  face track the 22 channels ARE the morph weights)** blends into the weight array: slot 0 copies straight; slots 1
  and 2 override a weight only when |w| > 0.01; the anim object's eye bytes write +0x97 → index 14 (value/255) and 13
  (1 − value/255), +0x98 → index 11 and 12, each guarded by the same 0.01. Under a script camera (`ScriptCam != 0`)
  only slot 2 applies, as a straight copy. Ported: `nf_game::anim::facial_blend` / `facial_weights`.
- **Idle/blink driver** [M `AnimFacialUpdate` 0x1be310]: per anim object, state byte +0x99 (0x7f mask) and timer
  +0x92 pick scripts from `Male_Facial_State_Anims` / `Female_Facial_State_Anims` (bit 10 of anim object +0x58 =
  the gender select) — [D ACTION.ELF .data 0x2c6ea8/0x2c6ec0: male `060000f8/060000f9/060000f5/060000f6/060000f3`,
  female `0600036d/0600036f/0600036a/0600036b/06000370`; table[0] = idle (0xe0000000), table[4] = blink
  (0xe0000001), with `Rand_Rand` draws for the blink cadence and a 50/52 % chance of a longer blink gap].
- **Weights → vertices** [M `AnimObjectDraw` 0x1c4490 → `psiDrawSkinObjectMatrix` 0x111828 → `SkinIt` 0x10bf30]:
  near objects only (the min of the four lighting bytes at animData+0x110..0x113 < 13; no gate under a script
  camera), `Morph_Count` = skin+0x2e, `Morph_Weight[i]` = the weight array's floats with `Morph_Index[i] = i`, then a
  (weight, index) selection sort (first min(count, 8) descending by weight, then the index order restored). Per
  vertex, the skinning word's upper half-word (the short at +0x1E, −1 = none) is the morph index; `SkinIt` fetches
  `target_base + (vertex_index + Morph_Index[m]) × 12` for each morph m and adds the weight-scaled VECTOR3. The
  target base is the runtime gfx record's +0x14 — **pinned (fix-facial2)**: the record is the 0x28-byte block
  `parsemap_block_morph_data` 0x1d23e8 / `parsemap_block_PS2_GFX` 0x1d2450 allocate per cel-list entry, and
  0x1d23e8 stores `FileNextBlock + 4` — the raw payload of a **MORPH_DATA map block (dispatch 0x1d1020 case
  0x2b)** — into `gfx+0x14` (psiCreateEntityGfx 0x1171b0 has no +0x14 store; its disassembly resolves only the
  object array and skin-info words). The block pairs with the entity whose params follow (the cel cursor advances
  only in `parsemap_block_entity_params` 0x1d1508). Disc proof [D 07000005]: 'generic_head_grunt' 0200040d
  carries 63,528 bytes = 5,294 targets and its vertices' max morph index is 5272 — the array is exactly
  `(max_morph_index + 1 + facial_count) × 12` (5227 = 5205 + 22 on 020002da), the `(morph + channel)` addressing
  SkinIt uses. Ported: `MapObject.morph_targets` (level.rs's `pending` pairing), `Character.morph_targets`,
  `CharVertex.morph` (the skinning word's upper half-word), and the morph term in the CPU skinning
  (`Scene::animate`, `morph_vertex` — `pos += Σ_m w[m] × target[morph + m]`), the weights sampled from the
  started face-track window (`Scene::facial_weights_now`). The GPU skin path (G-043) is not morphed yet.
- The face tracks ride the cut-scene scripts as the second opcode-0 window (docs/formats/animation.md "Scripts"):
  07000005's ledge pair 06000996/97 → face sequence 040007fa (frames 1..0x5dc), whose first weight crosses 0.01 at
  frame 767 [D, `facial_live`]. The port currently poses BONES from those channels
  (`nf_game::anim::script_pose_drone`'s narrow-track path, [L]-pinned by the 07000006 truck-driver trace) — re-pointing
  them at the morph weights is the next step and needs the truck-driver immobility re-explained (the body track's
  root motion must then be zeroed by the cut-scene itself).

## Placement

Characters are `oDrone` placements (flag 0x0F; also 8 animated `Tree`s) whose parameter 0 is a skin hashcode: 522
skins found in the same archive, 33 not (another archive, e.g. the common animation file; G-021). nf-view draws each
skin's main meshes at the placement transform; feet land on the floor (07000005 gatehouse), so the translation is at
foot level while the mesh origin is at the hips — consistent only if the bind-pose model's feet are at y ≈ −1.16 and
the placement y is 1.16 above the floor; not yet checked against the game (U-026).
