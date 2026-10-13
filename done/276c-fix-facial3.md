## Your task: `facial3`, blinking and the face tracks driving the morphs

Read `docs/reports/fix-facial2.md` (merged: the morph target buffer is pinned and the vertex half is applied on the CPU
skinning path). Left:
1. **The idle/blink driver** `AnimFacialUpdate` 0x1be310 (state byte +0x99, timer +0x92, the male/female state tables
   already banked in anim.rs, its `Rand_Rand` draws in order), wired into the drones' anim loop. The draws move
   seed-pinned tests: re-pick them by the WORKLOG 0072 procedure against the same live expectations.
2. **The face tracks' consumer:** `script_pose_drone` poses BONES from the 22-channel tracks today; the decode says they're
   morph weights. Re-point them to the weights, keeping the 07000006 truck driver's immobility and the ai_traces tests
   green (the immobility must then come from the body track's root: check the data first).
3. **The GPU skinning path** (gpu-skinning is merged): apply the morphs in the vertex shader too, or upload the morphed
   positions for faces. Keep the GPU vs CPU palette oracle test green.

Tests: a guard's eyelid vertices close and reopen on the blink cycle; a dialogue face's mouth vertices move at frame 767 of
the 07000005 ledge track; sister assertions.

Files: `crates/nf-game/src/anim.rs`, `crates/nf-game/src/drone.rs` (the anim loop, minimal), `crates/nf-view/src/{scene,gpu}.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/facial3`.

---
claimed-by: sandbox5 at 2026-10-13T22:52Z

---
## Report (done, 2026-10-13T23:49Z)


---
## Report from the worker (done, 2026-10-13T23:54Z)

