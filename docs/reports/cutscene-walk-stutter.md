# cutscene-walk-stutter — report

Task 686, branch `s/cutscene-walk-stutter`. Playtest 1015-1738 (07000007, the Kimiko cut-scene):
"the one in the red dress, stuttered a little bit while she was walking towards the other two".

## What I traced (the repro, through the real paths)

`crates/nf-view/tests/cutscene_walk_stutter_live.rs` drives the scene exactly like the render
compare harness does: Bond into TouchOnce #412's volume → SP #420 plays the scene script
0x06000073 → its op-2 commands start the guests' pack anim scripts (the packs 07800007.bin/07900007.bin
hold the scene's sequences). Per tick it samples every `Obj::Anim` and, for the walker — skin
**0x0500003b** playing anim script **0x060004f1** — her root world position (the body sequence
**0x04000332**'s world-space root, the same reading the render's `characters_script_pos` makes).

Findings, every link checked in the decomp:

1. **No frame repeat or skip.** The frame advances exactly 1 per tick for the whole 4326-tick
   scene. [M] `Script_Update` 0x1da998 advances the script clock and every stream clock by
   FRAME_RATE_MUL (1.0 at the fixed 60 Hz); `Script_Interp` case 2 (0x1d9a08:157-166) writes
   `clock − start_clock + start_frame` clamped to [1, anim-script length] into the anim time and
   zeroes the advance; `AnimScriptTick` 0x1c2630 skips the advance while `AnimSeqSet` 0x1c7460's
   start bit (0x20000000) is up — it is set at the window start and never cleared; `AnimSeqTick`
   0x1c28d0:62-63 maps the anim time to the sequence frame 1:1 (`time − window_first + 1`). Both
   op-0 windows of 060004f1 run from frame 1 to the scene end ([1..0x10e6] face 04000467, [1..0x10e7]
   body 04000332) — there is no mid-walk window hand-off.
2. **The root discontinuity is in the disc's own data.** Her walk moves in 0.005-0.027/tick steps;
   the trace shows exactly three one-tick jumps — at scene ticks ≈846/2645/3876, i.e. sequence
   frames **936, 2735, 3966** of 04000332, with |Δ| **0.279 / 0.371 / 0.880** — each an isolated
   lunge from a standstill inside a walk. Dumping the root channels' polynomial segments shows each
   jump is a 1-2-frame LINEAR bridging segment in the disc's own data (e.g. seg49 covers t ∈
   [935..936] with slope −0.227/frame and is C0-continuous with its neighbours at both ends; the
   2735 slide follows a 97-frame constant). The game evaluates the same polynomials at the same
   integer frames: `AnimFrameCopy` 0x1c5668 walks the packed segments exactly like the port's
   `Sequence::value` (bit-exact per WORKLOG 0017's PCSX2 measurements). So the port is not adding a
   discontinuity — **the three slides are the disc data playing**, and a sim-side "fix" would
   deviate from the game.
3. **No root-motion double application.** `Script_Interp` case 2 applies no stream keys to the anim
   object (no `Script_GetInterp`/`Concat` in its arm, unlike cases 1/4/7) and `Script_AnimStart`
   creates the object at the origin with the sequence root carrying it — the render-verified
   reading. The port matches arm for arm.

## The lock (what the tests assert)

- `walkers_root_introduces_no_discontinuity_beyond_the_disc_data` — the whole-scene trace: the
  frame advances 1/tick; every per-tick root delta is ≤ 0.05 unless it is exactly one of the disc's
  three slides (frame 936 |Δ| 0.279, frame 2735 |Δ| 0.371, frame 3966 |Δ| 0.880); all three slides
  verified present (the trace samples the disc data, not something else); the walk's 99th-percentile
  delta is small-step.
- Sister `a_doubled_frame_trips_the_walk_check` — a deliberately doubled frame (the tick advances,
  the frame and the root freeze) trips the frame-repeat arm; a synthetic root jump that is not a
  disc slide trips the discontinuity arm; a disc slide passes.

## Render side (for the main agent)

Nothing to fix: the view poses cut-scene characters at the sim's integer frame (`script_anim` →
`script_pose_together`; `Sequence::value`/`local_pose` — the game's own `AnimFrameCopy` evaluation),
and the game's `AnimFrameSet`/`AnimFrameBlend` fractional blend is a no-op at integer frames. There
is no render-side interpolation of the character root in the port, and the game has none to match.

## What I ran

- `cargo test --offline --release -p nf-view --test cutscene_walk_stutter_live` — 2/2 passed (the
  whole-scene run included).
- No library code changed in this task (the evidence says the sim already matches the game), so the
  full suite state carries over; `tools/test/suite.py` ran on this branch (started 11:20 UTC,
  finished 12:03 UTC) — total line: **`1805 passed, 6 failed, 72 ignored  (43 min)`**. The 6
  failures are the 3 `disc_golden` tests (allowed on the slim ISO) plus `probe_pylon_render` and
  2 `fix_0c_pylon_live` render tests from task 07000005's pylon work — pre-existing, GPU-render
  probes that fail headless and untouched by this task (this branch adds only a test file and
  docs).

## What's left / Live checks for the main agent

1. **The three slides in the real game** — freeze PCSX2 during the Kimiko scene and read the
   red-dress guest's position at scene ticks ~936/2735/3966 (the anim object of 060004f1, its
   sAnimScript +0x90 time and the drawn root). Predicted: the game's guest makes the same
   0.28/0.37/0.88 one-frame moves (the disc data plays). If the game is instead smooth there, the
   port's sampling has a gap my decomp pass did not find — the trace's whitelist should then be
   dropped and the discontinuity chased from that measurement.
2. **The tail garbage of 040003f9 and 06000225's sequence** — both tracks' root channels carry
   ±40000-unit values from frame ≈4270 to 4327 (the port's trace shows the two standing guests
   teleport to the void at t≈4178). The game must be faded/cut away from those actors when those
   frames play. Check the scene's camera at ticks 4178-4326: if any guest is on screen, the game
   clamps those frames somewhere I did not find (`AnimFrameCopy`'s sAnimSeq +0xa2/+0xa4 clamp is
   the suspect — the port clamps at the script length only).
