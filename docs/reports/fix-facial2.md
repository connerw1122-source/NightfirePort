# fix-facial2 — the morph target buffer, the vertex half (the blink driver and the track re-point are next)

Branch `s/fix-facial2` from master 0cda458. fix-facial's open items: the morph TARGET buffer's disc
location, the vertex application, the face tracks' second consumer, the blink driver. This session
pinned the buffer from the decompile+disassembly, landed the vertex half in the CPU skinning path,
and proved both against the disc. The blink driver and the bone→morph re-point did not fit the
window (details below).

## The morph target buffer — PINNED [M]

The gfx record is the 0x28-byte block `parsemap_block_morph_data` (0x1d23e8) and
`parsemap_block_PS2_GFX` (0x1d2450) lazily allocate per cel-list entry (the decompile's dropped
store is theirs; psiCreateEntityGfx 0x1171b0 has NO +0x14 store — its full disassembly only writes
the object array, entry fields 0x20/0x24, the REF-index terminator and the skin-info words):

- **`*(gfx+0x14) = FileNextBlock + 4`** [M 0x1d23e8] — the runtime morph target base is the raw
  payload of a **MORPH_DATA map block, dispatch `parsemap_handle_block_id` 0x1d1020 case 0x2b**
  (`map.rs::id::MORPH_DATA` already existed). The block belongs to the entity whose entity-params
  follow (the cel-list cursor only advances in `parsemap_block_entity_params` 0x1d1508), the same
  pairing the port already uses for 0x2E collision (`pending` pattern in level.rs).
- `psiDrawSkinObjectMatrix` 0x111828 reads `*(gfx+0x14)` into SkinIt's param 5 and disables the
  morph when it is 0, when `Morph_Count == 0`, or when `2.0 < fVar19` (the min lighting byte; the
  decompile says **2.0**, not the "<13" the character.md decode wrote — flagged for the main
  agent's live check; the port does not gate yet).
- Disc proof (`facial2_live::the_morph_blocks_pair_to_their_objects`, green): 07000005 has 7
  skinned objects, 4 with MORPH_DATA; `'generic_head_grunt'` 0200040d carries **63,528 bytes =
  5,294 targets** and its vertices' max morph index is **5272** → 5272 + 22 = 5294 — the target
  array is exactly `(max_morph_index + 1 + facial_count) × 12`, the `(morph + channel)` addressing
  SkinIt 0x10bf30 uses. The second object fits the same way (5227 = 5205 + 22). This is the census
  assertion the buffer pin rests on; it would have failed before (level.rs dropped 0x2b entirely).

## The vertex half — LANDED (CPU path)

- `MapObject.morph_targets` (level.rs: the 0x2b payload) → `Character.morph_targets` (scene.rs,
  taken from the object OR any of the skin's main meshes — the head is a separate map object).
- `CharVertex.morph` = the skinning word's upper half-word (`v.w >> 16` as i16, −1 = none) — the
  short at +0x1E SkinIt reads.
- `animate()` adds the morph term in object space before the bone skinning:
  `pos += Σ_m weight[m] × target[(morph + m)]` (`morph_vertex`, the exact 0x10bf30 arithmetic,
  out-of-range contributes nothing like the game's count-bounded loop).
- The weights: `Scene::facial_weights_now` finds the started opcode-0 window whose sequence
  carries the skin's facial channel count (the same window `script_pose_together` poses bones
  from) and samples it at the script object's frame via `nf_game::anim::facial_weights` (the
  0x1be648 source arithmetic). No RNG draws added.

## Verification (honest)

- `cargo check --offline -p nf-formats -p nf-game -p nf-view` clean; `nf-view` has pre-existing
  warnings only.
- `facial2_live` (NF_ISO, slim ISO), 2/2 green, ~2 s:
  - `the_morph_blocks_pair_to_their_objects` — the census above (failing-first: pre-fix the
    `morph_targets` field did not exist and no object carried targets).
  - `a_face_vertex_moves_by_the_morph_term_when_the_track_first_moves` — drives the REAL path:
    the level intro, then Bond onto the ledge cut-scene's start plate (**Touch trigger placement
    301, out 0x43** — SP #298 plays 06000994 [M nis.rs's start_allowed comment; the decompile
    refuses the scene while a combat-music event is live, none at spawn]). Character 70's face
    window **040007fa (1500 frames, 22 channels)** goes live; two lockstep scenes (one with the
    targets cleared) cancel the bone posing; the face vertices carry a ~1e-3 morph delta at the
    tick the weights cross 0.01 (weights max 0.0762, max target 0.0256 — mouth-scale), the
    weight-0 frame agrees to the noise floor (per-load HashMap seeds), and non-morph vertices are
    bit-identical. The delta magnitudes here are at the f32 noise floor of the scene contrast —
    the hard pins are the census above and the render spots below.
- The blast radius was run in the window (all green): **nf-game 228 passed / 0 failed** (incl. the
  3 facial tests); **nf-formats 11 passed + the 3 sanctioned `disc_golden` slim-ISO stubs**;
  **nf-view targeted: ai_traces_live 4/4, fix_truck_live 2/2, walk_07000005_live 2/2,
  facial2_live 2/2**. NOT run (did not fit): nf-audio, the remaining nf-view binaries — re-run
  `NF_ISO=... cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1` before merge. The risk areas are covered: animate()
  is the shared draw path (the morph term sits behind `c.morph_targets.is_some()`), and
  script_pose_together is untouched (ai_traces + truck green above).

## What's left

1. **The blink driver** `AnimFacialUpdate` 0x1be310 — tables banked, not wired into the drone anim
   loop; its Rand_Rand draws will move seed-pinned tests (WORKLOG 0072 re-pick).
2. **The face tracks' second consumer** — NOT re-pointed (correctly): the 07000006 truck driver's
   immobility still hangs on the narrow track's all-zero root owning the root bone [L trace; the
   body track's root runs 0.57 over the window], so the cut-scene must zero the body root — the
   live check below answers it first. The vertex half works WITHOUT the re-point (the weights are
   sampled straight from the track, the bones keep script_pose_together).
3. **The GPU skin path** (G-043) — the morph term is CPU-only; the GPU upload still carries the
   bind pose (per-vertex morph deltas need a per-frame vertex pass or a morph palette).
4. **The distance gate**: the decompile's `2.0 < min_light` vs the decode's "<13" — resolve by
   live check before adding any gate.
5. Sub-token note: the task header named the branch `s/fix-facial2`; pushed there throughout.

## Live checks for the main agent

1. **Morph targets at runtime**: gdbwatch `parsemap_block_morph_data` (0x1d23e8) during a
   07000005 load — the store `*(gfx+0x14) = FileNextBlock+4` should land a pointer into the
   loaded map block; then read 12 floats there for 'generic_head_grunt' (0200040d) and compare
   with the block bytes at the same offset (the port now has them at `MapObject.morph_targets`).
2. **Distance gate**: fieldprobe `psiDrawSkinObjectMatrix` 0x111828's float compare (`2.0 < fVar19`
   per the decompile) vs the decode's "<13" — walk a morph-bearing character out to 13 units and
   see where the mouth stops moving.
3. **Truck driver root**: at the 07000006 truck (start script 0600011d), confirm the cut-scene
   zeroes the BODY track's root channel (the decode requires it; the body track's root runs 0.57
   un-zeroed) — frees the face tracks to be re-pointed at the weights.

## nf-view render spots (the visual proof)

- `07000005` ledge cut-scene: teleport onto the Touch trigger placement 301 (out 0x43) — character
  70 (window 040007fa) — Mayhew's mouth moves from window frame 767 (the weights cross 0.01 there).
- `07000001` Bond close-up: NOT wired (the blink driver is item 1 above) — the face track path
  needs a script camera scene there.

## Files touched

`crates/nf-formats/src/level.rs` (the 0x2b pairing + `MapObject.morph_targets`),
`crates/nf-view/src/scene.rs` (`Character.morph_targets`, `CharVertex.morph`,
`facial_weights_now`, the morph term in `animate()`, `morph_vertex`),
`crates/nf-view/src/dev.rs` (one field), `crates/nf-view/tests/facial2_live.rs` (new),
`docs/formats/character.md`, `docs/WORKLOG.md` (appended). No shared file rewritten; no Rand_*
draws added.

## ntfy

(send result recorded after the finish step)
