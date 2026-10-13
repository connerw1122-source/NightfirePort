# fix-anim-blend — animation frame blending and speed modes

Branch `s/fix-anim-blend`, cut from `origin/master` 13abef4. (The driver message names both
`s/fix-anim-blend` and `s/anim-blend`; `s/fix-anim-blend` is used everywhere — it is the branch
that exists.)

## What's done

1. **`Quat_Slerp_Acc` verbatim** [M `work/decomp/ACTION.ELF/001e4188_Quat_Slerp_Acc…c`, lines
   12-36] — `crates/nf-game/src/anim.rs::slerp`. The old port negated **q1** when `dot < 0`, gated
   the linear branch at `dot > 0.9995` (the game: `0.01 < 1 − dot`, i.e. dot ≥ 0.99), renormalised
   the result (the game does not — 0x1e4188:33-36 add w0·q0 + w1·q1 as is), and used host `sin`
   with `x/s` division (the game: `PS2Sinf` and a reciprocal-multiply, one division). The game
   needs no NaN guard for a rounded dot past 1.0: `1 − dot ≤ 0 < 0.01` sends it to the linear
   branch, so the clamp was dropped too. Doc entry: docs/systems/anim-engine.md, section
   "fix-anim-blend" (appended); game address 0x1e4188.
2. **Fractional-frame root motion for drones** [M `AnimFrameSet` 0x1c75a0, `AnimFrameBlend`
   0x1c2b80, `AnimSeqTick` 0x1c28d0] — the port already blended posed bones at fractional frames
   (`anim::sequence_pose`, gate 0.0002), but the root motion sampled the quintics at whole frames:
   `Drone::script_frame` returned `(hash, u32)` and `root_value` evaluated `s.value(k, f)` at that
   integer. The game's root delta is the blended pose buffer's root (AnimSeqTick stores seq.frame =
   script.time − start + 1.0 — the float clock +0x90, the port's `ftime` — and the pose buffer
   comes from AnimFrameSet's f/f+1 blend). Now `script_frame` returns `(hash, f32)`,
   `anim::root_translation_at` applies the AnimFrameSet gate + AnimFrameBlend translation clamp to
   the root channels, `root_value` and the outgoing-fade root sampler use it, and the wrap tick
   samples through `ftime` synced to the integer times it forces (the loop wrap keeps `ftime`'s
   fraction via `ftime_in`). At speed 1.0 (most walk danims) every frame is exactly the integer it
   was before — `frac == 0` hits the single-frame gate. At a fractional `Drone_AnimInfo +0x14`
   speed (lean-out 176 plays at 1.5 [L work/chase_trace.tsv], run 0xDC at 0.7 on 07000009/0A
   [M `DAnimFunc_Run`]) the root moves by the same blend the bones show, instead of stepping whole
   frames — the guard feet-slide at fractional speeds.
3. **Speed modes 1–3 machinery** [M `AnimScriptTick` 0x1c2630 (byte +0xB4),
   `AnimDistanceTableCreate` 0x1c4060, `AnimDistanceTableDistanceToFrame2` 0x1c4270] —
   `anim::DistanceTable::build` (cumulative |root Δ| per frame, axis-mask components, segment
   under 1e-05 → 0.01, looping = frames−1 segments), `distance_to_frame` (walked wrapped into
   [0, total) and stored back, estimate-then-walk segment search, frame = segment + into/length,
   speed wrapped in the stride's direction) and `speed_for_mode` (mode 0 ×FRAME_RATE_MUL = ×1.0 at
   the fixed 60 Hz; mode 2 synced to master +0x4C with the decompile's exact wrap; mode 3
   (len+1)·phase − time; no writer for mode 3 in ACTION.ELF, as in the game).
   **Machinery only this session**: nothing sets +0xB4 yet. The game's mode-1/2 pairs are built by
   `AnimSetUpdate` (0x1c0938) for the PLAYER's walk sets (master +0xB4=1, slave +0xB4=2,
   `sAnimSet` +0x24 = 0.37 idle accumulator, +0x28 = 0.5 stride factor [L docs/fields/animobject.md
   — players only]); the guards play mode-0 scripts at `Drone_AnimInfo +0x14`, which the port
   already did. Wiring the player walk sets is the remaining work (see below).

## Tests (failing-first, shown red before the fix)

- `anim::tests::slerp_negates_w0_and_never_renormalises` — RED against the old slerp body (the
  opposite-w case returns +1 where the game returns −1; the length-2 case shrinks when
  renormalised), GREEN on the exact model. Also run with the old body restored in the working tree
  to show it: `cargo test --offline --release -p nf-game --lib anim::` → `2 failed` (both new
  tests) with the old body, `3 passed` with the fix.
- `anim::tests::distance_table_maps_walked_distance_to_frames` — RED before (no table existed),
  GREEN after: builds a 5-frame looping walk, asserts frames−1 segments, total 4.0, the stored-back
  wrapped walked value, the forward wrap and the backward-stride negative speed [M 0x1c4060/0x1c4270].
- The pre-existing `slerp_and_blend_follow_the_weights` stays green under the exact model.
- The full four-crate suite is the regression net for the drone.rs change: the 17 `walkreal_*_live`
  levels and the cut-scene pose tests re-run it at speed 1.0 (where the change must be bit-identical).

## What's left

- **NF_ISO tests on real disc data** for the two new pieces (a `Level05`-style loader against
  `l.sequences`: `root_translation_at(seq, 10.5)` == the transcribed AnimFrameBlend of frames
  10/11 on a real walk sequence; `DistanceTable::build` on a real walk sequence then
  frame-vs-walked mapping + the monotone-root property). The loader plumbing was not done in this
  session's window; the unit tests carry the algorithm pinning.
- **The stance-foot test through Sim** (a walking guard's foot bone world position drift < 0.05
  while it's the stance foot): needs a guard at a fractional speed or the player walk sets; at
  speed 1.0 the port was already integer-consistent, so the scenario has to force the fractional
  path (e.g. danim 0xDC at 0.7 on 07000009/0A) or use the walk sets below.
- **The player walk sets** (`AnimSetUpdate` 0x1c0938: levels from |fwd|/max, master +0xB4=1 +
  slave +0xB4=2 pairs, 4-tick countdown, mask-0x200 fades over clock=length=8, the 0.37
  accumulator / 0.5 stride) — this is what drives Bond's walk anim on 0700000c and removes the
  remaining feet-slide the user sees; the machinery it needs is now in place.
- **The far-object blend path** (`AnimFrameBlend`'s `ScriptCam == 0` + min LOD byte ≥ 0xF1 →
  `Quat_Slerp` 0x1e4120 = nearest) — verified in the decompile this session, not yet ported; the
  port has no LOD gating on the blend yet.

## Live checks for the main agent

- PCSX2, 0700000c, walk Bond forward then stop: the stance foot should hold its ground position
  over a stride (the walk-set master is mode 1 — distance-driven). In nf-view:
  `--bond` spots along the 0c corridor at the ST-1 plate area, watch for feet sliding during
  acceleration (the walk-set wiring is NOT in yet, so some slide remains by design until the
  next session ports AnimSetUpdate).
- PCSX2, 07000001: a patrolling guard (walk danim, speed 1.0) — confirm his steps and pace match
  the current master build (the slerp/root changes are bit-identical at speed 1.0; any visible
  change there is a regression to report).
- nf-view `--bond` on 07000009/0A where a guard runs (danim 0xDC at 0.7): his run should now
  advance root motion smoothly (no per-frame stepping) — before this branch the root stepped in
  whole frames while the bones blended.

## Shared-file changes

- `crates/nf-game/src/drone.rs` — `script_frame` returns a fractional frame; `root_value` and the
  outgoing root sampler use `anim::root_translation_at`; the wrap tick syncs `ftime` to its forced
  integer times; `ftime_in` preserves the loop wrap's fraction. All inside the anim-advance path
  the task's blending work owns; no new fields, no renames, no match-arm changes.
- `crates/nf-game/src/anim.rs` — the named task file: exact slerp, `root_translation_at`,
  `DistanceTable`, `speed_for_mode`, the two new tests.
- `docs/systems/anim-engine.md`, `docs/WORKLOG.md` — appended sections only.

## Suite

`NF_ISO=<slim ISO> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
-p nf-audio -p nf-view -- --test-threads=1` (run in two foreground chunks — the sandbox killed
the backgrounded nohup runs; same command, same flags):

- **nf-view: all targets green** (the 17 `walkreal_*_live` levels, the cut-scene pose tests, the
  walk/anim suites — the regression net for the fractional root motion). **nf-audio: green.**
- **nf-game: all green except one PRE-EXISTING failure** — `ai_live::guard_takes_cover_like_the_game`
  ("UnderCoverAim lasted 90 (live 37)") fails on **unmodified master too** (verified by stashing
  this branch's two files and re-running: same 90/37). Not caused by this task.
- **nf-formats: the 3 permitted `disc_golden` failures** (slim ISO, as the driver allows) plus
  `pss_movies_live::audio_sample_count_matches_stream_length` ("expected ~20 audio-bearing
  movies, checked 0") — also pre-existing on master: the slim ISO has no PSS movies and the
  skip-on-empty guard exists only on the unmerged `s/fix-swim-vent4` branch (c463614). Not this
  task's file to change.
- This branch's own tests: `anim::tests` 3 passed / 0 failed with the fix; the two new tests
  shown RED against the old slerp body before it.
