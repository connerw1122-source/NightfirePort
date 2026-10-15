# fix-gpu-facial-morphs — the facial morphs on the GPU skinning path

Branch `s/fix-gpu-facial-morphs` (origin/master c9ada8c8 + merge 9d8918d4). fix-cutscene-faces
landed the weights (`AnimFacialBlend` 0x1be648) on the CPU skinning path only; the GPU path
(G-043, the default) drew still faces — the driver's byte-identical Kimiko renders at frames
600/1400. This is fix-facial3's open item 3.

## The game's order (the evidence)

- [M `SkinIt` 0x10bf30:88-118 (decompile)]: per vertex whose morph index (the short at +6) is not
  −1, `pos += Σ_m weight[m] × target(base + (morph + Morph_Index[m])·12)` — the 12-byte reads as
  three f32 — BEFORE the VU1 skinning macro. The morph is a source-position displacement, the
  blend happens after.
- [M `AnimObjectDraw` 0x1c4490:141-200]: marshals `Morph_Weight[i] = animobj+0xb0[i]` (the
  `AnimFacialBlend` array; count = the skin's +0x2e facial count), `Morph_Index[i] = i` (identity,
  line 150), a descending weight sort with the top-8 selection passes, a |w| < 1e-6 trim into
  `Morph_Count`, and a final ascending re-sort of the kept channels — i.e. the effective sum is
  channel-order over the non-epsilon weights, which the port's full channel-order loop
  (`morph_vertex`) reproduces to the oracle tolerance.
- [M `psiDrawSkinObjectMatrix` 0x111828:148-155]: the per-draw LOD gates (Morph_Count 0, distance
  > 2.0) — the game skips morphs for far draws. NOT ported (the port's `animate` never gated
  either); listed under live checks.

## The fix

- `crates/nf-view/src/scene.rs`: `SkinAttr` gains the morph plumbing the shader needs (all static
  per scene): `mtb` (the vertex's target element base = char target base + morph·3, −1 = none —
  the game's +6 gate), `mwb` (the character's weight-block base, vec4s), `cnt` (the skin's facial
  count = Morph_Count), `tend` (the character's target block end — the per-channel bound of the
  game's count-bounded loop). `SkinLayout` gains `wt_stride` (the widest facial count in vec4s)
  and `n_tgt` (the concatenated target elements). `skin_layout()` fills them; new
  `morph_target_blob()` (the static MORPH_DATA bytes, character order) and `skin_morph_weights()`
  (one frame's per-character weight blocks from `facial_weights_at` — drone-backed characters
  included). **The CPU un-gate**: `animate`'s facial vector drops its `c.script.is_some()` clause
  — fix-facial3's stand-in gate, obsolete now that the shader applies the same term from the same
  weights source (the gate existed only because the GPU half was missing).
- `crates/nf-view/src/gpu.rs`: the shader gets the two storage buffers (bindings 3 = the weight
  blocks, 4 = the target elements) and `morph_pos()` — the `SkinIt` term, accumulated in the
  game's channel order onto the source position BEFORE the palette blend. The skin-attr vertex
  layout carries the new fields (attrs 7/8/9). `Renderer::new` creates the weight buffer
  (per-frame) and the target buffer (static, `morph_target_blob`), binds both in the group-0 bind
  group; `update_skin` uploads the weights each call (the `--shot` path), new
  `upload_morph_weights` for the live loop.
- `crates/nf-view/src/lib.rs` (shared, additive hook): one call —
  `g.renderer.upload_morph_weights(&g.queue, &self.scene)` after the live path's `upload_skin`,
  so in-play faces move at the tick rate like the palette.
- `crates/nf-view/tests/gpu_skin_live.rs`: the G-043 oracle's `apply` gains the morph term (read
  from the same buffers the shader gets). Required the moment the CPU un-gate lands — the
  07000006 driver's face window is live from tick 0 and the oracle would otherwise see the
  0.0033-style disagreement fix-facial3 reported.

## Reproduction (failing first)

- `crates/nf-view/tests/facial_gpu_live.rs` `the_drivers_face_moves_in_play_not_only_in_scripts`
  (07000006 driver, drone-backed, weights live from tick 0): **on the pre-fix src it FAILED** —
  `frozen 0.000008` (the gate freezes him: `animate` == the morph-less palette arithmetic).
  Post-fix: `frozen 0.001709` (the CPU pose carries the term), `morphed 0.000008` (the term alone
  explains the pose).
- `the_kimiko_face_delta_is_beyond_the_gpu_oracle_tolerance` (07000007, Bond at the TouchOnce
  #412 volume — fix_cutscenes2_live's spot — the guest, char 133, live at max |w| 1.000): the
  pre-fix GPU arithmetic trails the CPU pose by **0.004147** on the face vertices (the render
  delta the byte-identical frames showed); with the morph term the CPU pose is reproduced to
  **0.000000**.
- `the_gpu_morph_buffers_reproduce_the_cpu_pose_mid_blend` (the new plumbing, both scenes):
  GPU+weights vs CPU **0.00000000** (07000007 char 133) / **0.0000076** (07000006 char 14), zeroed
  weight upload caught by the sister (**0.004147 / 0.002316** — the frozen-face regression test).
- `gpu_skin_live` (the G-043 oracle, 05/06/07 at ticks 1+240, 713k–838k vertices): worst position
  error 3.05e-5 / 1.5e-5, 0 colour mismatches — the palette+attrs+weights reproduce the CPU pose
  with the un-gate live.

## Verification

- `facial_gpu_live` 3/3, `gpu_skin_live` 3/3, `facial2_live` 2/2, `facial3_live` 3/3,
  `fix_cutscenes2_live` 5/5 on the merged tree (9d8918d4). Full suite totals: see the WORKLOG
  entry (tools/test/suite.py on the final tree).
- Failing-first: the pre-fix src run is quoted above (stash of `crates/nf-view/src`, the
  pre-fix-API tests only — the full test file binds to the new APIs by design).

## Shared files touched

- `crates/nf-view/src/lib.rs` — the ONE `upload_morph_weights` call after the live path's
  `upload_skin` (additive, 3 lines with the comment).
- `crates/nf-view/src/scene.rs`, `crates/nf-view/src/gpu.rs` — the task's named files.
- `docs/WORKLOG.md` — appended only (the merge with 9d8918d4 kept both sides' entries).
- `drone.rs`, `player.rs` untouched.

## Left / open

1. The game's per-draw morph LOD gates (`psiDrawSkinObjectMatrix` 0x111828: Morph_Count == 0,
   distance > 2.0) are not ported — the port morphs at any distance. Only visible on far-away
   talking characters; the CPU path never had them either.
2. `AnimObjectDraw`'s weight sort/trim (top-8 selection, |w| < 1e-6 cut) is not mirrored
   bit-for-bit — the port applies all channels in channel order. Within the oracle tolerance on
   every measured scene; a bit-exact mirror would touch `anim.rs` (not this task's file).
3. The live window path uploads weights once per tick (no sub-tick interpolation) — the game's
   own rate; the palette interp (fix-interp-ghosting) does not extend to the weight vector.

## Live checks for the main agent

nf-view renders (the mouth/eyes must MOVE across frames — pre-fix they were byte-identical):
1. `nf-view $NF_ISO 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 600 --shot` vs `--frame 700` and
   `--frame 800` — Kimiko's close-up: the mouth/jaw vertices must differ frame to frame
   (fix-cutscene-faces' spot, frames 600/700/800; 399 is a legitimate closed frame).
2. `--frame 1200` — Dominique foreground, Bond/Kimiko dancing right; her face must track the
   dialogue.
3. `07000006` driver: any cab close-up across his face window (his weights are live from tick 0)
   — pre-fix the GPU renders froze his face; now they must move like the CPU path's.
4. Regression: a distant talking NPC (the morph LOD gate is not ported — if the game freezes far
   faces, the port will not; only a PCSX2-vs-nf-view far-shot comparison would show it).

PCSX2 (gdbwatch, optional): during the 07000007 scene, watch `Morph_Count` (the byte the skin
render marshals) and the `Morph_Weight` array against the port's `facial_weights_at` output at
the same frame — expect the same [0,1] oscillation; `Morph_Index` identity before the sort.
