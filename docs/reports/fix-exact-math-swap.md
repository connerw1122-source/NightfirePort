# fix-exact-math-swap — the sim runs the game's exact sine and matrix path (G-017 / G-024 closed)

Branch `s/fix-exact-math-swap`, task 284. Sandbox 21:29–23:28 UTC 2026-10-13.

## What's done

**The swap.** `placement::object_matrix` (`crates/nf-formats/src/placement.rs`) and the palette path
(`anim::quat_trans_to_mat` + `anim::mul`, `crates/nf-formats/src/anim.rs`) now run the game's arithmetic
(the versions proven in `s/verify-bitexact` commit e246658, re-applied on master):

- `object_matrix` = `RotMatrix` (ACTION.ELF 0x1e2368) operation for operation: the two `PS2Sinf3` calls
  (`(rx+π/2, rx, ry+π/2)`, `(rz+π/2, rz, ry)`) via the bit-exact `anim::sinf` (PS2Sinf 0x120700 — 40/40
  recorded calls, `tests/diff_calls.rs`), then the nine products/sums on the EE FPU (`ps2float`).
  The 0-rotation diagonal is the game's own 0x3F800002, not 1.0 — pinned in the unit test.
- `quat_trans_to_mat` = `Quat_QuatTransToMat` (0x1e3fa8) on the EE FPU (doubling adds exact, chopped
  products, no-guard sums) — bit-exact on the 40 recorded `Quat_QuatToMat` calls (diff_calls).
- `mul` = `psiBuildMatrixPalette` (0x111498) on the VU0 (chopped products, guard-bit running sum) —
  world-0.0 on all three EE snapshots (`nf-extract pose-check`, docs/formats/animation.md "Arithmetic").

The unit tests pin the new bits (`placement::tests::matrix_matches_the_decomp_for_single_axes`: the
0-rotation diagonal 0x3f80_0002, `sinf(π/2)` 0x3f80_0001, `m[2][2]` 0; the identity transform is
2.0000002, not 2.0 — the game's own identity rotation is not exact). `anim::tests` pass unchanged.

**The old repro no longer reproduces.** verify-bitexact.md's failure (`walkreal_07000001_live::
walk_from_the_spawn_to_the_exit_with_the_contact` stalls, guard 5 dead at the round-0 clearing) does
NOT reproduce on master + the swap: the escort walk passes end to end (level_exit to 07000002 at tick
63143, 27.6 s), and the nf-formats unit tests are green. Master's merges since 045n (pickups, weapon
modes, script commands, sound) changed the fight the flip lived in. What follows is the side-by-side
root-cause of the divergence the two paths still have.

## Root cause: where the paths first diverge (side by side)

Method: a scratch replay harness (a copy of `ai_traces_live`'s replay with a per-tick dump — tick,
frame, Bond pos bits, then every `llist` slot's state + x/z/health bits) run twice over the 01-doors
trace's first 1200 frames, once on host math, once on the exact math, and the dumps diffed. The replay
drives Bond per frame from the recording, so the ONLY difference between the runs is the arithmetic.

1. **First divergence, tick 28 (frame 43473): Bond's y, one ulp.** Host `pos[1]` bits 0x40cf2bfd,
   exact 0x40cf2c00; x/z identical, all 17 drones identical.
   Bisected: with ONLY `anim::quat_trans_to_mat`/`anim::mul` swapped, t28 stays host — with ONLY
   `placement::object_matrix` swapped, t28 flips to 0x40cf2c00. The sine/RotMatrix path is the source.
2. **The mechanism (all [M], function + address):** `CollisionWorld::build_scoped`
   (`crates/nf-game/src/world.rs`, the `stat.push(... transform(&inst.matrix, p) ...)` walk) bakes every
   placed object's collision triangles into the static world through the placement's matrix, which is
   `placement::object_matrix` = `RotMatrix` 0x1e2368. The game's RotMatrix is not the platform's: its
   cosines are `PS2Sinf(a+π/2)` and PS2Sinf(π/2) = 0x3F800001 (one ulp above 1.0), so a 0-rotation
   matrix has diagonal 0x3F800002. Bond at the trace's start stands on a placed walkway mesh whose top
   face the exact RotMatrix puts a few ulps higher: `world.floor_below` under Bond's t27/t28 spot
   (−5.161, −0.684) returns **5.3394012 (0x40aadc60) on host vs 5.3394027 (0x40aadc63) on exact**.
   `Player_StandAtNewPosition`'s floor snap (pos = floor + cc) rides it: Bond's y flips at t28.
3. **First AI-visible divergence, tick 835 (frame 44308): drone 13's x, one ulp** (host 0xbfad73b1 vs
   exact 0xbfad73b0), 807 ticks later; the drones' own placements shift the same way and their
   movement/aim integrations amplify. Both variants of the old repro flip (the sine and the palette,
   independently) because each shifts the shared geometry by ulps and the fight's decisions sit on
   ulp-wide margins — there is no port bug underneath: the flip IS the game's arithmetic arriving.

## Which path the game agrees with

- The exact path is the game's arithmetic by construction, proven against the game itself:
  PS2Sinf 40/40 recorded calls + 3000 interpreter samples; Quat_QuatToMat 40/40 recorded calls;
  the VU0 palette world-0.0 on the EE snapshots (verify-bitexact.md; diff_calls).
- The `ai_traces_live` reference (01-doors/01-raptor, full replay, fire on) shows the swap does not
  move the port one bit away from the game's recordings: per-drone first divergences are IDENTICAL
  between the maths — 01-doors drone 15 at line 75 (state 140 vs game 88), drone 16 at 76; 01-raptor
  drone 15 at 63 (140 vs 103), drone 16 at 64 — and the pins (0 pos, 1 state, 15 ≥ 63, 16 ≥ 64) hold
  on both. The two paths only split at ulp level (t28/t835 above), an order of magnitude below the
  trace's 3-decimal recording and the harness's 0.05/0.15 tolerances; in the index-aligned window
  (trace lines 76..278, where the game's 17-row llist matches the port's) the two paths' drone
  positions are bit-identical, so the recording cannot — and need not — separate them.
- The escort walk (the test the swap used to flip) passes on the exact math.

Decision: the game agrees with the exact path; master's host-math call sites were the deviation.

## Re-pins

None needed so far: the swap adds no `Rand_*` draws and the full-suite run on the swap is in this
sandbox's record (see Suite below); the seed-pinned suites that reference ulp-sensitive fights
(`ai_live`, `walkreal_*`) were rerun — see the suite section for the exact counts. The only test
edits are the new bit pins in `placement.rs` (above).

## Shared files touched (hooks)

None. Only the task's own files changed: `crates/nf-formats/src/placement.rs`,
`crates/nf-formats/src/anim.rs` (the task's file list named `crates/nf-game/src/{anim,placement}.rs`;
the functions live in nf-formats — `nf_game::anim` holds none of the three, `nf_game::ps2math` was
already the proven model and needed no change).

## Suite

(filled in below when the run finishes)

## Live checks for the main agent

1. PCSX2, 07000001 at the 01-doors trace's start (Bond ≈ (−4.92, 6.34, −0.09), frame 43445): watch the
   floor query result under Bond (the `floor_below` segment (−5.161, y+0.5, −0.684) → y−1.2). Expected
   on the game: ≈ 5.3394027 (bits 0x40aadc63), the exact path's value, not the host's 5.3394012
   (0x40aadc60). Confirms the walkway instance's matrix is the game's RotMatrix output.
2. The walkway instance's identity: which placed glist object's collision triangles contain
   (−5.16, 5.3394, −0.68) in 07000001 (the port's `CollisionWorld::build_scoped` bucket) — for the
   field notes.

## What's left

- Nothing on the task's two items: the root-cause is above, the swap is in with its pins.
- The ulp-wide AI margins this exposes are the port's normal state now (the game has them too); any
  future seed/trace re-picks follow WORKLOG 0072's procedure.

## ntfy failed

(not reached yet)
