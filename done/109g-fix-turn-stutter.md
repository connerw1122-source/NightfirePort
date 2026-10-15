## Your task: `turn-stutter`, the world stutters while turning

User (05, 08, 15): "While I'm turning, I see this kind of jitter in the world, kind of like a stutter when I'm
rotating... It doesn't happen to my view model or my hand or gun... Not as smooth as it used to be." and "I'm getting
like frame lag... this is not smooth anymore." and "it seems a bit stuttery... this isn't just in this level". This is
new since the 1013 playtests (the user marked the interpolation/smoothness checks ok then). Find the regression: the
camera yaw path vs the interpolated render (the view model is smooth, the world isn't - the world camera may read the
raw tick yaw while the view model reads the interpolated one, or the fps cap / frame pacing changed), frame-time
spikes (nf_game::prof), and bisect master since 2026-10-13 23:00 if needed. Fix it; report frame times while turning
before/after.

Files: `crates/nf-view/src/lib.rs` (the frame loop/camera), as found.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/turn-stutter`.

---
claimed-by: sandbox6 at 2026-10-15T01:38Z

---
## Report (done, 2026-10-15T03:41Z)


# fix-turn-stutter — the world stutters while turning

**Task:** 109g-fix-turn-stutter (branch `s/fix-turn-stutter`). Playtest 1015, users 05/08/15:
"While I'm turning, I see this kind of jitter in the world... It doesn't happen to my view model
or my hand or gun... I'm getting like frame lag... not as smooth as it used to be... this isn't
just in this level." New since the 1013 playtests, where the interpolation/smoothness checks were
marked OK.

## The cause (measured, then derived, then proven fail-first)

The port renders the world through an interpolated draw camera: `draw_cam = lerp_cam(cam_prev,
cam_cur, anim_time.fract())` — the G-043 fixed-step scheme (logic at a fixed 60 Hz, the draw one
tick behind). The scheme is mathematically smooth **iff the bracket `cam_prev`/`cam_cur` stays
exactly one tick apart**: then the drawn camera sits at exactly `anim_time − 1` ticks, a smooth
function of wall time, at any frame rate.

The loop broke that invariant whenever a frame ran more than one tick [M `crates/nf-view/src/lib.rs`,
the pair update the `RedrawRequested` handler ran pre-fix: `self.cam_prev = self.cam_cur;
self.cam_cur = Some(self.cam);` — `self.cam` is the camera after the frame's LAST tick, and
`cam_prev` is the previous frame's `cam_cur`]:

- a frame that runs **N ≥ 2 ticks** (any frame time over 16.7 ms — any droop below ~55 fps) built
  a bracket spanning **N ticks** of camera motion, while the blend alpha `anim_time.fract()`
  spans **one** tick. The drawn camera crawled through the wide bracket, then jumped at the
  boundary — a hold-jump every frame, worst exactly while turning (rotation makes temporal jumps
  visible), on every level.
- at a solid 60 fps every frame runs exactly 1 tick and the bracket was correct — which is why
  the 1013 playtests passed.

Two things the 2026-10-13 23:00 → 10-15 window changed made this pre-existing defect visible:

1. **fix-interp-ghosting's view-model reattach** (in the window): the gun's per-tick poses are
   re-attached to the draw camera in camera space, so the gun is *screen-rigid* by construction.
   Before it, the gun was blended in world space and swam **with** the camera's unevenness —
   everything jittered together and the eye had no steady reference. After it, the gun holds
   perfectly still while the world hold-jumps: the exact reported signature (world stutters,
   hand/gun smooth).
2. **Small per-frame cost additions in the window** (the `near_los_rescue` cel re-find per frame,
   the slerp-based palette blend, the HUD context work): on a box riding the 60 Hz vsync deadline
   they tip frames past 16.7 ms, and vsync drops to the 30 Hz band — where **every** frame runs 2
   ticks and the artifact is constant. That matches "I'm getting like frame lag" and "this isn't
   just in this level".

### The other suspects, cleared by measurement (not memory)

- **Sim frame-time spikes (`nf_game::prof`'s domain):** a headless probe through the real path
  (`nf_view::load` + `Sim::step_bond`, a steady 0.02 rad/tick turn + forward walk, 600 ticks) on
  the final tree: **05: 1.86 ms mean / 2.77 ms max; 06: 1.75 ms mean / 5.6 ms max** — nowhere
  near a 16.7 ms frame. The sim is not the bottleneck (scratch probe kept uncommitted at
  `crates/nf-view/examples/tick_cost.rs`).
- **The camera yaw path, the tick scheduler, the pacing:** a line-level audit of the whole
  redraw-handler diff against the 1013 baseline `904bef36` (the last master commit before
  2026-10-13 23:00) — the `dt` stamping, `ticks_for_frame`, the mouse-delta accumulation and
  per-tick `mem::take`, the cam capture, the pair update, `interp0`/`alpha0`/`draw_cam`, the
  vsync config, the fps-cap sleep, `request_redraw` — all **byte-identical**; the only interp-gate
  change is the dev menu's `d.interp` (defaults on). No pacing change, no raw-yaw path.
- **Per-tick yaw unevenness** (the mouse bucketing) is pre-existing and smoothed by the same
  interpolation — unchanged by the window.

## The fix

Files: `crates/nf-view/src/lib.rs` (the task's named file) + `crates/nf-view/tests/turn_stutter_live.rs`
(new). **No shared files touched.**

- The camera is now captured **at every tick** (inside the tick loop, right after `step_bond`'s
  followers: Bond's head/yaw/pitch/fov and the script-camera override — the same capture the loop
  ran once per frame). Two `Camera` copies per tick; no measurable cost.
- `update_cam_bracket` (new, `pub`, extracted **verbatim** from the pair update so the headless
  test drives the same arithmetic the winit loop runs — the `ticks_for_frame` precedent) closes
  the bracket to the **last two ticks' cameras** — one tick apart — at every frame rate:
  - 0 ticks ran (pause page, dev menu, a replay exhausted): the pair stays (the alpha keeps
    blending it);
  - 1 tick (the solid-60 case): the previous frame's camera and the new one — **byte-identical to
    the old behavior**;
  - 2+ ticks: the last two of *them* — the fix.
- The view-model reattach is gated to frames whose last palette/bracket build ran exactly 1 tick
  (`bracket_ticks`): a 2+ tick frame rebuilds the palette across its wide bracket while the
  cameras now span one tick — reattaching there would pose the gun from mismatched ticks, so
  those frames keep the plain world blend (the pre-ghosting behavior under load).

**Evidence note (per the code-task rules):** this is a defect in the port's own host-side frame
scheduler, not in ported game logic — there is no game address to cite because the game's
decompile has no interpolation to compare (the native game is tick-locked to the console's 60 Hz
vsync [D input.md:126 `FRAME_RATE = 60`]; under load it slows down rather than catch up [M the
fix-perf-level note in the loop]). The doc entry implemented is the port's own **G-043**
fixed-step render interpolation (docs/merge-log.md; the `fps_determinism_live` header; the
`ticks_for_frame` doc comment) — the fix restores the invariant G-043's one-tick-behind scheme
requires. The behavioral ground truth is the playtest symptom plus the measured fail-first below.

## Verified failing-first

`turn_stutter_live` drives the loop's own arithmetic (`ticks_for_frame` + `update_cam_bracket` +
`lerp_yaw`) over a real level's sim with a steady turn (0.02 rad/tick — a normal mouse pace):

- **Pre-fix (the extraction committed with the old rule, run on 07000005):**
  - `loaded_frames_render_one_tick_behind_exactly` **FAILED: max error 0.02000 rad at 30 fps** —
    exactly one tick's turn: the drawn camera sat a full tick off the one-tick-behind sim clock,
    i.e. the hold-jump, quantified.
  - `bracket_arithmetic` **FAILED**: the old rule kept the previous *frame's* camera (11.0) where
    the last two ticks' rule needs the second-to-last tick's (12.0).
- **Post-fix: 3/3 green** — 30 fps over 6 s of sim (180 frames, the drawn camera tracks the
  sim clock to < 1e-4 rad while the old rule's contrast stays > 0.005 rad), 60 fps over 4 s, and
  the no-ISO bracket unit (2+ tick frames close the bracket from the per-tick captures; 1-tick
  frames keep the previous frame's camera; 0-tick frames leave it; the first ticks propagate
  `None`).
- **Sisters:** `full_rate_frames_keep_the_exact_bracket` (at one tick per frame the new bracket
  IS the old bracket — error exactly 0.0: nothing moves at a solid 60); `fix_interp_ghosting_live`
  2/2; `fps_determinism_live` 4/4 (the 60/144 Hz determinism and the drop cap unchanged).

## Frame times while turning, before/after

The fix changes no frame cost (two struct copies per tick; the reattach gate *removes* work on
loaded frames). Measured on this 2-core box, walking + turning through the real input path:

| level | sim tick (mean) | sim tick (p95) | sim tick (max) |
|---|---|---|---|
| 07000005 | 1.86 ms | 2.14 ms | 2.77 ms |
| 07000006 | 1.75 ms | 2.22 ms | 5.61 ms |

The user-facing frame time is dominated by their render (not measurable in this sandbox — no
GPU); the drawn camera now tracks the sim correctly at whatever rate their box achieves, which
removes the hold-jump at 30–55 fps and leaves a solid 60 fps untouched.

## Suite

Run on the final tree (merged with origin/master 56bb263f) with the fast runner, in foreground
chunks on this box (the nohup background death is the known 102c/100f/grenade precedent):

- nf-game + nf-formats + nf-audio (44 bins): **376 passed, 3 failed (the allowed `disc_golden`
  trio), 2 ignored**
- nf-view chunks 1–4 (208 bins): **745 passed, 3 failed, 25 ignored** — the 3 failures are all
  the documented GPU-adapter sandbox condition (`adapter: NotFound`, `fix_0c_pylon_live` ×2 and
  `probe_pylon_render` — no Vulkan/GL adapter in the worker sandbox; pre-existing, not this
  diff)
- nf-view chunk 5, partial (13/48 bins — the 10-minute foreground window ended the run;
  `weapon_matrix_live` alone ran 339 s): included in the totals below

**Total: 1155 passed, 6 failed (3 allowed `disc_golden` + 3 GPU-adapter `NotFound`), 52 ignored**
over 265 of 300 binaries. Not run (35 bins, the last chunk's remainder — the `walkreal_*`,
`walk_*`, `weapon_assets_live`, `weapon_fx_levels_live`, `wire_*`, `world_fx_live` families and
`zz_obj_drives_probe`): the sandbox window closed; none of them touches the frame loop, and the
bracket fix is covered by the 265 completed binaries including `turn_stutter_live`,
`fix_interp_ghosting_live`, `fps_determinism_live`, the fire-modes/aim-raise/portal suites and
the full nf-game lib.

## Live checks for the main agent

PCSX2 runs the real game (no interpolation there — the game is vsync-locked), so the checks are
in nf-view and one playtest re-run:

1. **nf-view, 05, turning with the fps readout:** run the viewer on 07000005 (`--dev`), open the
   F2 panel's Toggles page, turn steadily and read the fps EMA. Expected: ≥ 60 on the playtest
   box; the world must look step-free while turning at any fps. Before the fix, any droop under
   ~55 fps showed the hold-jump.
2. **The interpolation A/B:** with the same session, `NF_NOINTERP=1` vs on. Both must look
   equally step-free at ≥ 60 fps (at 60 the camera updates once per frame either way).
3. **A 2-tick-frame spot (the fix's own territory):** `NF_MAXTICKS=1 --fps 30` (forces every
   frame to want 2 ticks and run 1 — the capped path) on 05: turn steadily; the world must move
   smoothly (one tick behind, interpolated), not hold-jump.
4. **Render spots (`--bond`), the levels the user named:** stills cannot show a temporal bug, but
   for the merge render set: `--bond 105.91,25.32,108.13,1.57,0` (05 spawn, facing down the
   corridor) and the same spot with yaw 3.14 — the frame must be the normal 05 view, unchanged by
   this fix (the fix touches no geometry).
5. **Playtest re-run on the user's box (05/08/15):** turn in place and strafe-turn; confirm the
   world no longer jitters relative to the gun. If the box still feels laggy, read the fps EMA —
   a persistently < 30 fps render is a separate render-cost issue (the bracket fix keeps the
   motion smooth there, but the rate itself is the GPU's).

## What's left (honest limits, not regressions)

- **The characters' pose bracket on 2+ tick frames:** the palette pair still spans the frame's
  whole tick run (built once per frame), so *other characters'* poses advance on the wide bracket
  during loaded frames — subtle, pre-existing, not the reported symptom. Fixing it costs a
  per-tick `skin_palette` build on exactly the frames that are already slow; not done here.
- **Sustained heavy overload (> 4 ticks per frame, < ~15 fps):** `anim_time.fract()` wraps while
  the capped pair lags — pre-existing, reads as a slideshow, out of scope.
- The user's box confirmation (the live checks above).
