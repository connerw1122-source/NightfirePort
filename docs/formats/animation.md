# Skeletal animation: skeletons, skins, sequences, skinning

Reader: `crates/nf-formats/src/anim.rs`. Viewer: nf-view animates characters on the CPU. Sources: decomp of
`AnimSkeletonProcess` 0x1bd328, `AnimProcessSkinData` 0x1bd500, `AnimFrameCopy` 0x1c5668, `Quat_QuatTransToMat`
0x1e3fa8, `psiBuildMatrixPalette` 0x111498, `CacheSkin`/`SkinIt` (*measured*); whole-disc structure checks
(*measured*, WORKLOG 0015); field meanings first described by Raikaru/nightfire-engine `docs/formats.md`
(*sourced*, facts only) — each marked below with how it was confirmed.

## Skeleton (type 6, `SKELnnnn`; 45 skeletons, ids 0–44, every copy identical across archives)

```
+0x00 u16 id   +0x02 u8 bone_count   +0x03 u8 (unused)
+0x04 u32 mask[3]: bit i = bone i has translation channels in sequences
+0x10 f32[3] × bone_count: fixed local translation for bones without translation channels
size = 0x10 + 12·bones, padded to 32   (measured: holds for the files checked; Raikaru for the meanings)
```

## Skin (type 3, prefix 05; layout verified on all 1,217: size = computed end padded to 32)

```
+0x00 u32 hash  +0x04 f32 scale[3] (multiplies translations)  +0x10 u8 n_skinned  +0x11 u8 n_parts
+0x12 u8 facial_count  +0x13 u8 n_datums  +0x14 u8 skeleton id
+0x15 u8 bone[bone_count]: bit 7 = animated; low 7 bits = parent (0x7F = root) — measured: every parent precedes its
      child and there is exactly one root in all 1,217 skins
rigid parts: u8 bone[n_parts], align 4, u32 hash[n_parts]
skinned meshes: align 4, u32 hash[n_skinned] (byte 2 bit 0x10 = sleeve)
inverse bind (if n_skinned): per bone f32 t[3], f32 q[4]   (0x1C bytes, as AnimProcessSkinData steps)
datums: n_datums × 36 bytes
```

## Sequence (type 4, prefix 04; 18,244 + 8 empty 32-byte entries; in the 07F…/078… animation archives)

```
+0x00 u32 hash  +0x0C u8 flags  +0x38 u16 frame_count  +0x3A u16 channel_count  +0x3C u8 skeleton id
+0x40 channels: channel_count = 3·bones + 3·popcount(mask) in 17,235 (more for some skeletons)
```
Channel (*measured*, `AnimFrameCopy`): u32 words; word 0 = `byte_length | e0 << 16`; then e0's floats; then a new
word `[e1 | e2 << 16]`, e1's floats, e2's floats, and so on. Entry = `frames << 6 | mask`; mask bit k = coefficient
k stored (else 0). Segment k covers frames until its cumulative count; the value is
`c0 + c1·t + … + c5·t⁵` with **absolute** `t = frame − 1`. Checked on all 18,244: every channel consumes exactly its
byte length and its segments sum to exactly `frame_count − 1`.
Per bone: tx, ty, tz (only if the skeleton mask bit is set; × skin scale), then qx, qy, qz; w = +√(1−x²−y²−z²) if
qz < 2, else qz −= 4 and w = −√(…) (*measured*).

## Pose and skinning

- Local matrix = `Quat_QuatTransToMat(q, t)`: row-vector rotation (rows = rotated axes), translation in row 3
  (*measured*). Bones without translation channels use the skeleton offset × scale (Raikaru; matches the layout).
- `psiBuildMatrixPalette` processes animated bones only (bit 7): root world = local; child world rotation = parent ∘
  local, world translation = local t · parent rotation + parent translation (*measured*, VU0 macro code read as a
  quaternion product — composition order as for row vectors, to confirm visually / by trace).
- Skinning matrix = world rotation with translation `inv_bind_t · R + world_t` (Raikaru; equals identity in bind pose).
- Vertex (source buffer, *measured*): `qw0.w` = weight of the first matrix (all 845,738 in [0, 1]); qw1 word 0 / word 1
  low 15 bits = matrix slots; slot → bone via the 0xFF-terminated byte list at skin-info word 3 (≤ 55 entries; every
  used slot is inside it, all 582 objects). `p' = w·(p·S[slot0]) + (1−w)·(p·S[slot1])` (Raikaru).
- Playback: frames advance by speed × `FRAME_RATE_MUL` per update (60 Hz → 1 frame per update); `AnimFrameSet` blends
  frame f and f+1 (Raikaru, matches `AnimFrameCopy` evaluating integer frames).

Open: which sequence each drone plays (state machine / `AnimSet_*`, U-027); facial morphs; head tracking and aim
overrides in `psiBuildMatrixPalette`; root motion.
