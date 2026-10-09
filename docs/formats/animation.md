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

## Script (type 5, prefix 06; 18,231, all in level archives)

Objects play scripts, not sequences (`AnimScriptAdd(obj, 0x06……)`). Layout from `AnimScriptInit` (0x1c1600),
`AnimProcessScriptData` (0x1bd3d8) and `AnimScriptPeekSeq` (0x1c6c10), *measured*:

```
+0x00 u32 hashcode
+0x04 u16 length (script frames)
+0x06 u16 bits 0–8 = command count; bit 15 = script flag 0x80000000 (loop: AnimScriptEnd wraps instead of stopping);
          bits 9–14 → script +0xB3
+0x08 commands: u8 opcode, u8 n, then (n + 3) u16 for opcode 0, (n + 2) for opcodes 1–4, n otherwise
```

Opcode 0 plays a sequence: `u16 first script frame, u16 last script frame, u16 sequence low half` (hashcode
`0x04000000 |` it; `AnimScriptPeekSeq` reads it at +0xE). Opcodes 1 and 4 not decoded (events: footsteps, sounds).
On the disc (`nf-extract scripts`, golden `scripts.tsv`): every script's commands fit inside its entry; 1,983 loop;
opcodes 0 / 1 / 4 = 18,296 / 3,698 / 28,285; 18,104 of the 18,296 sequences are in the matching 07F archive (192 are
elsewhere, G-021); 8 opcode-0 ranges exceed the script length.

Time (`AnimScriptTick` 0x1c2630, `AnimScriptEnd` 0x1c1930): starts at 1, += speed × `FRAME_RATE_MUL` per update;
past the length a looping script subtracts the length (clamped to ≥ 1), others hold the last frame and end. The
sequence frame is script time − first + 1 (`AnimSeqTick`).

## Root motion and height (*measured* from the decomp; not yet confirmed live, U-028)

- `AnimFrameCopy` keeps the root translation of the first evaluated frame in the sequence state (+0x20) and the
  current minus first (+0x30). `AnimSeqTick` (0x1c28d0) stores the root's change since the previous tick as the
  frame's root motion (script flags 0x01000000/0x02000000/0x04000000 zero single components — y for 0x02000000; the
  decompile of the others is garbled; no motion on the tick a script starts or wraps) and then **zeroes the root
  bone's translation** in the pose buffer.
- `AnimFrameResolve` (0x1c35f8) blends root motion across playing scripts, rotates it by the object matrix (passing it
  to the object's pre-translation callback first) and adds it to the object position (`obj+0x30`); it stores the
  absolute root height + `sAnimObject+0x60` as the hip height (`sAnimObject+0x5C` = `anim+0xCC`).
- `psiBuildMatrixPalette` then places the root bone at the object matrix (`RotTransMat`): the object position is the hip.
- Drones: `DroneAnim_PreTransCallback` (0x13b490) sets `Drone+0xA0` = hip height − 0.02 (≥ 0.2; starts at 1.0) and
  replaces the vertical root motion by the change of `Drone+0xA0`; `NDrone2_Collision` (0x140e90) puts the bottom of
  the feet sphere `Drone+0xA0` below the object, and `NDrone2_FeetPos` = position.y − (hip height − 0.4).
- Root heights measured (`nf-extract seq-roots`): standing sequences 1.03–1.15 with the lowest joint 0.03–0.04 above
  y = 0 — the sequence floor is y = 0. Bind meshes have the hip at the origin (lowest vertex −1.16). In 07000005 the
  drone placements sit 1.0845 above the rendered floor (10 of 16; others on steps/slopes).

nf-view: the root is zeroed as the game does; horizontal root motion is dropped (no AI or collision yet), and the drone
object is put `Drone+0xA0` above the highest World triangle below the placement — where the game's collision leaves
it, feet 0.02 into the floor. Which script a drone plays: docs/systems/drone-animation.md.

Open: frame blending (f/f+1); facial morphs; head tracking and aim overrides in `psiBuildMatrixPalette`;
`sAnimObject+0x60`; opcodes 1 and 4.
