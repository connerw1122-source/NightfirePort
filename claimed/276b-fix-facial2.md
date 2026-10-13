## Your task: `facial2`, faces that move: the morph targets, the vertex half and the blink driver

Read `docs/reports/fix-facial.md` and `docs/formats/character.md` §Facial morphs first (fix-facial is merged: the 22-weight
blend `AnimFacialBlend` 0x1be648 and the face-track sampling are ported and tested). Left:

1. **The morph target buffer.** `SkinIt` 0x10bf30 adds `weight[m] × target(base + (vertex_index + Morph_Index[m]) × 12)`,
   with the base at the runtime gfx record's +0x14, whose writer is a store the decompiler dropped (in
   `psiCreateEntityGfx` 0x1171b0's area). Find it from the disassembly (not the decompile), or from the block layout: a
   morph-bearing gfx block (0x04000048: 356 morph vertices, indices up to 5272) implies a ≳63 KB target array inside the
   block. `crates/nf-formats/tests/facial_probe.rs` prints the candidates. If you can't pin it statically, write a
   precise live-check request in your report (the gdbwatch range and what value to expect), and the main agent will run it.
2. **The vertex half:** apply the weighted targets in the port's skinning (CPU path, and the GPU skin if 282 has landed),
   near objects only (animData +0x110..0x113 min < 13; no gate under a script camera) [M 0x1c4490 → 0x111828 → 0x10bf30].
   Test: a face vertex of the 07000005 ledge dialogue moves by the expected delta at the frame its weight first crosses
   0.01 (frame 767), with a sister on a weight-0 frame.
3. **The face tracks' second consumer:** `script_pose_drone` poses BONES from the 22-channel tracks today (pinned by the
   07000006 truck-driver immobility trace). If those channels are morph weights, the driver's immobility must come from the
   cut-scene zeroing the body track's root. Check that from the data first; only re-point the tracks when the
   truck-driver and ai_traces tests stay green.
4. **The idle/blink driver** `AnimFacialUpdate` 0x1be310 (state byte +0x99, timer +0x92, the male/female state tables
   already banked, its `Rand_Rand` draws in order) wired into the drones' anim loop. Its RNG draws will move seed-pinned
   tests: re-pick them by the WORKLOG 0072 procedure.

Files: `crates/nf-formats/src/character.rs` (+ the gfx block parse), `crates/nf-game/src/anim.rs`, the skinning in
`crates/nf-view/src/{scene,gpu}.rs`, `docs/formats/character.md`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/facial2`.

---
claimed-by: sandbox2 at 2026-10-13T19:59Z
