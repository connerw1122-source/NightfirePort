# cutscene-faces — why no cut-scene face moves, and the sim/data side

Branch `s/cutscene-faces` (master e416e5f0 + merge ac1fba78). The playtest symptom
("Mouths aren't moving in these cutscenes") is NOT a sim bug: the sim already produces the right
per-actor, per-tick facial data on every scene probed. It is the draw side — the GPU skinning
path (G-043, the default renderer) never applies the morph weights. That half sits on the held
branch `s/fix-gpu-facial-morphs` (108c), whose own report shows the driver's byte-identical
Kimiko renders at frames 600/1400 — exactly the playtest's still faces. The main agent wires the
draw from 108c; this task delivers the verified sim/data side, the game's full mechanism, and
the tests that pin it on the two requested scenes.

## The game's mechanism — the three task questions, from the decompile

1. **Does the face track's window start?** Yes. Each cut-scene actor's own anim script (the
   "face-track hash per actor" is the per-actor `Script_AnimStart` 0x1d8f40 script hash —
   06000503/06000504 on 07000002's intro, 060004f1/060004f2/06000225 in the Kimiko scene) starts
   its opcode-0 windows through `AnimProcessScriptCmds` 0x1c1b38; a facial window is classified
   by the SEQUENCE's flags bit 0x8 [M 0x1c1b38:165 `*(int *)(lVar6 + 0xc) >> 3 & 1`] and plays in
   the script's +0x54 slot (the body window in +0x50) [M `AnimSeqSet` 0x1c7460:
   `param_3 + param_7*4 + 0x50`]. The 0x80000 script flag that would skip a facial start has no
   setter anywhere in the decompile — the face track always starts.
2. **Do its channels reach the face bones?** Never. The link the earlier work was missing:
   `AnimScriptInit` 0x1c1600:52-61 marks EVERY script started while `ScriptCam != 0` on a
   facial-bearing skin with animScript+0x80 = **0xe0000002 (the talk slot)**, so
   `AnimFacialBlend` 0x1be648's ScriptCam path finds the cut-scene's own script by its marker
   (`AnimListBuild` 0x1c0fc0 predicate bit 0x200 = marker == value) and copies its +0x54 frame
   buffer — the 22 sampled channel floats — STRAIGHT into the morph weight array (animData+0xb0).
   The gameplay path instead reads the driver's idle/blink slots (0xe0000000/0xe0000001 [M
   `AnimFacialUpdate` 0x1be310]) and the speech talk slot [M `AnimFacialAppend` 0x1c6490, called
   from `NDrone2_SpeechSFX` 0x141b00] with the 0.01 threshold, then writes the +0x97/+0x98 eye
   bytes into indices 11-14. Under a script camera none of that applies — straight copy only.
   `SkinIt` 0x10bf30 consumes the weights as MORPH TARGET displacements (fix-facial2's vertex
   half), never as a bone pose; the body track (flags 0x00, 3·bones+3 channels) owns every bone.
3. **Does the game also use morph targets?** Yes — the weights do nothing by themselves. The
   MORPH_DATA blocks pair to the head gfx objects (fix-facial2's pin: the target array is
   `(max_morph_index + 1 + facial_count) × 12`), the CPU skinning path applies them, and the GPU
   path (the default) does not yet — 108c carries the GPU morph term + the CPU un-gate. Read
   108c's reasons in docs/merge-log.md: it was held only because its full-suite run didn't finish
   before its sandbox rolled over; its targeted suites and audit were green.

**The neutral face is also the game's own data**: `AnimScriptNew` 0x1c1398 allocates the +0x54
facial slot for every facial-bearing skin and `AnimSeqNew` 0x1c7248 zeroes its 22-float buffer —
a facial-bearing actor whose script has no face window (e.g. the Drake fly-through's guests) reads
a zero weight array under ScriptCam. The port's getter returns None for those, which renders the
same.

The window classifier census (new, pinned by a test): all 18,244 kind-4 sequences on the disc
carry flags {0x00 × 17,363, 0x01 × 10, 0x08 × 871} and every flags-0x08 one has exactly 22
channels; a body track is 3·bones + 3·popcount(mask) — always ≡ 0 (mod 3) — so the game's flags
bit and the old channel-count probe can never disagree [D].

## What landed

- `crates/nf-view/src/scene.rs` — `facial_weights_at`'s face-window picker now uses the game's
  own classifier (the sequence flags bit 0x8, the same rule `script_pose_together` poses bones
  by) instead of fix-facial2's channel-count stand-in; behavior-identical disc-wide per the
  census. This was fix-cutscene-faces2's open item 2. `facial_weights` keeps the facial-count
  guard (the game copies `skin.facial` floats from the slot buffer whatever the track carries —
  the guard is the port's stand-in for the game's over-read).
- `crates/nf-view/tests/cutscene_faces_live.rs` (new, 4 tests, all green):
  - `the_02_intro_actors_face_tracks_start_and_move_the_morph_weights` — 07000002's intro
    (07800002.bin scene 0x06000084, SP placement 400): Bond (05000009 / 06000504) and the desk
    man (0500004f / 06000503), facial 22, morph targets 63,528 bytes, the flags-0x8 window +
    the mod-3 body window per actor; the live getter equals the direct track read at the
    object's frame on every sampled tick; the weights MOVE between the dialogue beats (tick 170
    vs 400 vectors differ for both actors); the desk man's mouth closes between lines
    (all-zero at ticks 10/700) and opens on them (track max 1.02 @ frame 1399); the sweep pins
    ≥ 10 channels over 0.5 and ≥ 5 non-eye (jaw/lip) channels over 0.3 per track (the game names
    only indices 11-14 — the eye bytes' targets [M 0x1be648]; everything else is the mouth group).
  - `the_kimiko_guests_face_weights_move_over_the_dialogue` — the three guests 060004f1
    (0500003b), 060004f2 (05000015), 06000225 (0500000d): same window pair, track sweeps to ~1.0,
    the live getter matches the direct read at ticks 400/900/1400, the speaking guests' live
    weights move between beats, and the quiet guest (060004f1) sits closed at the early beat
    (all-zero at frame 399) — a closed mouth is zeros, never garbage.
  - `a_facial_bearing_guest_with_no_face_track_keeps_a_neutral_face` — the fly-through's guests
    (060001fd..060003b8): 11 facial-bearing guests (facial 22, morph targets present) whose
    scripts carry NO facial window all return weights None and their joined pose equals the
    body-only pose — the strong sister, matching the game's zeroed facial slot.
  - `every_facial_flagged_sequence_carries_the_facial_channel_count` — the disc-wide census pin.
- `docs/systems/anim-engine.md` — new section "The facial weights": the script +0x80 marker
  field, the three 0xe000000x slots, `AnimScriptInit`'s ScriptCam rule, `AnimFacialAppend`'s
  speech talk, the zeroed-slot neutral face, the classifier census.

## What's left

1. **The draw (the main agent wires it)**: merge/port `s/fix-gpu-facial-morphs` (108c) — the GPU
   skin path needs the `SkinIt` 0x10bf30 morph term (weights + MORPH_DATA targets, before the
   palette blend) and `animate()`'s CPU facial vector needs its `c.script.is_some()` gate dropped
   (fix-facial3's stand-in; 108c does both together — the un-gate alone breaks `gpu_skin_live`'s
   CPU-vs-GPU oracle). My flags-bit picker change touches the same function 108c's
   `skin_morph_weights` wraps — small merge, the comments overlap.
2. The gameplay facial driver (idle/blink) is ported (`facial_update`, fix-facial3) but not wired
   into the sim tick, and the gameplay speech talk slot (`NDrone2_SpeechSFX` →
   `AnimFacialAppend`) has no port counterpart — gameplay guard speech would not move mouths
   (cut-scene faces are unaffected: the ScriptCam path reads the scene's own script).
3. The game gates the gameplay facial blend on draw distance (min of animData+0x110..0x113 < 13
   [M AnimFrameResolve 0x1c35f8]; ScriptCam skips the gate). The port has no such gate — a
   superset, invisible at cut-scene camera distances; port it only if a far-guard render ever
   disagrees.

## Live checks for the main agent

1. PCSX2 07000002 intro (~frame 170) and 07000007 Kimiko (~frames 900/1400) close-ups: mouths
   move during the lines; the desk man's mouth is shut between his lines (~intro frame 10/700).
2. gdbwatch the guest's morph weight array (`*(obj+0xDC)+0xb0`, 22 floats) during the Kimiko
   scene: expect values in [0, 1] oscillating with the dialogue (the game's ScriptCam talk read
   of the face-track buffer), and the palette's face bones holding the body-track values.
3. Optional: confirm the intro's scripts carry marker 0xe0000002 (animScript+0x80) once the
   scene runs — the AnimScriptInit rule's live proof.

## Render spots for the main agent (after the 108c draw lands)

- `07000002` intro: `--bond` facing the desk man during a line — his jaw/lips move; between
  lines his mouth is shut.
- `07000007` Kimiko scene: `--bond 94.2,44.6,-45.9,0,0 --frame 900` and `--frame 1400` — the
  guests' mouths move during the dialogue (the CPU path already renders it; the GPU path is the
  one to check).

## What I ran

- `cargo test --offline --release -p nf-view --test cutscene_faces_live` — 4/4.
- Blast radius on the changed getter: fix_cutscenes2_live 5/5, facial2_live 2/2, facial3_live
  3/3, cutscene_draw_live 2/2, cutscene_models_live 4/4, gpu_skin_live 3/3, nf-game
  `--test facial_live` 3/3, nf-game `--lib` 237/0.
- Full suite (`tools/test/suite.py`, the final tree merged with origin/master 1883f83a's base
  ac1fba78), chunked over the tool-call limit — total line below.

## ntfy

(pending)
