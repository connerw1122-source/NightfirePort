# PS2_GFX (map block 0x2D): static geometry

Reader: `crates/nf-formats/src/ps2gfx.rs`. Golden: `golden/geometry.tsv` (per archive digest). CLI:
`nf-extract gfx-list | gfx-digest | gfx-export`. Sources: decomp of `parsemap_block_entity_params` 0x1d1508,
`parsemap_block_PS2_GFX` 0x1d2450, `psiCreateEntityGfx` 0x1171b0, `RecurseAndDrawBoxes` 0x1233e8, `DrawThisBox`
0x1230d8 (*measured*); the disc bytes of all 2,230 maps (*measured*); GIF tag / VIF / DMA tag field layouts from the
PS2 hardware docs (*sourced*: standard EE/GS user's manual layouts as also used by PCSX2). Confidence: high for the
container and batch layout (invariants checked on every batch). The VU1 microcode itself has now been read
(docs/formats/vu1-microcode.md, task 110v): entry points 0 = clipped / 2 = unclipped are confirmed (the VU builds
the clipped output with a Sutherland–Hodgman pass; entry 0 sets the clip-enable flag), the UV handling is confirmed
(the VU multiplies the raw disc ST by 1/w and outputs Q = 1.0, so the port's direct-UV reading matches what the GS
interpolates), and no lighting terms exist in the microprogram (colors are pre-lit upstream). Still medium: the
signed V4-8 normal record's w byte ({0,−1}) — the interleaved pos/rgba/nrm layout matches the microcode's stride-3
walk and the VU's output-NLOOP counter proves per-vertex emit decisions, but the w byte's consumption point sits in
the not-yet-read handler tail (GAPS G-014, remaining ~40% of the program).

## Object

Each 0x2D block is preceded (in parse order) by an entity-params block 0x04: payload `+0 u32 hashcode`
(0xFFFFFFFF or the entity's hashcode, added to the hash table), `+4 u32`, `+8..+0x2C` values copied into the cel
glist, `+0x30` NUL-terminated name. The 0x2D block (offsets from the block start, i.e. including its 4-byte header):

```
+0x04 u32  offset of the footer G (0x10 = empty object, no footer)
G:    u32 box_count, u32 box_offset, u32 skin_info_offset (0 = static), u32 ?
box (0x38 bytes) × box_count:
  +0x00 f32[3] min, +0x0C f32[3] max   clip box for CheckObjectClipBox; bit-exact bounds of the leaf's vertices
                                       (all 24,537 leaves with vertices)
  +0x18 i32 child A, +0x1C i32 child B   (-1 = leaf; children are box indices)
  +0x20 u32 packet offset (0 = container box, nothing to draw)
  +0x24 u32 texture-ref list offset: u32 packet-relative positions of texture REF tags, 0xFFFFFFFF-terminated
  +0x28 u32 offset of the packet's RET tag, +0x2C u32 ?
  +0x30 u32 the game adds it to `Ps2NumVerts*` statistics; equals the decoded triangle count in 16,749 of 17,030
            static objects, larger in 281 (U-020)
  +0x34 u32 flags (box 0's is compared with `LastFlags` to decide whether to resend the matrix)
```

At load `psiCreateEntityGfx` turns the offsets into pointers and rewrites each texture REF's address word (which on
disc holds the map texture index) with the texture's upload chain address.

## Packet: a DMA source chain, called with `MSCALF` (entry 2 = unclipped, 0 = clipped)

Tags (*measured* over all leaves): `CNT` (id 1) — the tag's upper 64 bits are two VIF codes, then `qwc` quadwords of
VIF stream; `REF` (id 3) with zero VIF codes and `addr` = map texture index — texture change (exactly the positions
in the box's texture-ref list, in all 25,181 static leaves); `RET` (id 6) — end. In the 582 leaves of skinned objects
(G's skin info ≠ 0) further `REF` tags carry UNPACK codes whose data addresses are filled at run time by the skinning
code (`CacheSkin`/`SkinIt`); those are not decoded yet (GAPS G-015).

Stream (concatenating the tags' VIF words and CNT payloads):

```
STCYCL 4,4 ; UNPACK V4-32 ×1 → B           GIF tag: NLOOP = n (| EOP), PRE, PRIM, FLG = PACKED, NREG = 3, REGS = ST,RGBAQ,XYZ2
UNPACK V2-32 ×n → B + 0xC7                 UV (f32 s, t), one qword per vertex
STCYCL 1,3                                 (stride 3)
UNPACK V3-32 ×n → B + 1                    position (f32 x, y, z)
UNPACK V4-8 signed ×n → B + 3              nx, ny, nz (i8) and w: −1 (0xFF) = no kick, 0 = kick
UNPACK V4-8 unsigned ×n → B + 2            RGBA (alpha 0x80 = 1.0)
MSCNT                                      run the microprogram on this batch
```

B cycles through 0x2E1, 0x1D8, 0xCF (triple buffering). Texture changes (REF) apply to the batches after them; the
first CNT also sends GS state (`FLUSHE; DIRECT` A+D writes: ALPHA_1, TEST_1, …).

GS state: *measured over every DIRECT A+D write of all 27 on-foot levels* — only ALPHA_1 (0x42), TEST_1 (0x47) and
ZBUF_1 (0x4e) ever occur (no CLAMP/TEX1/TEX0: filtering and wrapping are global texture-upload state). The ALPHA_1
values and their port mapping (fields A,B,C,D per the ps2tek GS tables; equation ((A−B)·C >> 7) + D): 0x44 (0,1,0,1)
= (Cs−Cd)·As+Cd normal; 0x7F00000048 (0,2,0,1) = Cs·As+Cd additive; 0x7F00000042 (2,0,0,1) = Cd−Cs·As subtractive;
0x58 (0,2,1,1) = Cs·**Ad**+Cd — additive gated by the framebuffer's alpha (07000041's `EnvModTest_*` env-map overlays,
the only 159); 0x64 (0,1,2,1) with FIX = 0 = (Cs−Cd)·0+Cd — keep-destination (the mod batches, the other 159; the
GS still depth-tests/writes and alpha-tests). Nothing in the load/draw path patches the FIX field
(`psiCreateEntityGfx` 0x1171b0, `DrawThisBox` 0x1230d8, `CacheAndFillNewGlistTextures` 0x11f180 re-checked), so the
disc values are final. TEST_1: 0x5001b (ATE, GEQUAL 1), 0x507fd (GREATER 0x7f), 0x5000b (GEQUAL 0 = always pass);
ZBUF_1 ZMSK at bit 32. Confidence for the state semantics: high (*sourced* tables + *measured* values; the port
implements them in `scene.rs::Draw::from_gs`).

Triangles: the GIF tag's PRIM is a triangle strip (type 4); vertex `i ≥ 2` closes triangle `(i−2, i−1, i)` unless its
w is −1. The GS has no winding; the port's exporter alternates winding with `i`'s parity inside the batch.

Object ↔ entity pairing: each 0x2D block is followed (not preceded) by its 0x04 entity-params block in parse order —
`parsemap_block_PS2_GFX` stores the gfx in the current cel, `parsemap_block_entity_params` then builds it
(`psiCreateEntityGfx`) and advances `pCurrCelList`. The reader requires exactly one 0x04 after every 0x2D.

## Results and checks (WORKLOG 0009)

- All 17,612 objects parse; 17,030 static (1,620 empty: footer 0x10) and 582 skinned. Static leaves: 25,181, one
  header-only setup kick each; 5,670,766 vertices, 3,619,982 triangles; every batch passes the VU-memory invariants
  (GIF tag NREG 3 / REGS ST,RGBAQ,XYZ2 / PACKED; all read qwords freshly written; w ∈ {0, −1}).
- Box min/max equal the decoded vertex bounds bit for bit in all 24,537 leaves with vertices.
- **Cross-check** (`tools/crosscheck/nfre_geometry.py`, nightfire-re's own VIF reader and OBJ writer): all 15,410
  non-empty static objects accounted for — 6,227 unique objects identical in triangles (as sets of position-bit
  triples) and UV corners; the other 9,183 are copies of map entries shared between levels, whose geometry SHA-1s
  equal the compared copy's.
- 208 vertices in 2 objects (`Room-L_atrium_bottom` in two Tower maps) have NaN u coordinates on the disc (U-022).
