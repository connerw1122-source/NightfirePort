# fix-gpu-skinning — GAPS G-043: GPU skinning, the tick-keyed pose cache, 120 fps

Session 2026-10-13 21:18–22:55 UTC, branch `s/fix-gpu-skinning` from `origin/master` (7a614ba).

## What I found

G-043 was already ~90 % landed on master by earlier sessions (the WORKLOG entries at lines 4656/4786
record the groundwork; `docs/GAPS.md` still said "not reached" — stale). Specifically, all three
items were implemented:

1. **GPU skinning** — `CharVertex`'s bind pose + weights + two bone slots feed `Scene::skin_layout` /
   `Scene::skin_palette` (scene.rs 3882–4019); one frame's matrix palette + colour table go to
   read-only storage buffers (gpu.rs bindings 1/2) and the vertex shader skins
   `w·P[a]p + (1−w)·P[b]p` and poses the normal the same way (gpu.rs `SHADER.vs` 47–66). The CPU
   path (`Scene::animate` / `posed_vertices`) remains as the fallback (`--shot`, the bench).
2. **Tick-keyed pose cache** — the frame loop rebuilds the palette only when a sim tick ran
   (`lib.rs`: `if self.ticked || self.pal_cur.is_none()`, with `ticked = n_ticks > 0`, i.e. keyed on
   the sim having advanced — the GAPS "keyed on `sim.drones.frame`" condition); frames with no tick
   re-upload the cached palette (`pal_cur`) via `upload_skin`. There is no per-frame
   `update_vertices` CPU upload left in the loop.
3. **120 fps** — logic runs at fixed 60 Hz decoupled from the render rate; frames between two ticks
   draw one tick behind, interpolating the camera (`lerp_cam`) and the palettes (`lerp_palette`) by
   the fractional tick, both with the 4-unit snap for cuts/teleports and short-arc yaw. `--fps N`
   caps the frame rate with `PresentMode::AutoNoVsync`; the default is vsync at the display rate.

So I did not re-port them. What was missing was the **named deliverable — the fps-determinism
proof** — and the doc closure. I verified each item through tests instead (below), which is the
evidence this section of the task asked for.

## What I did

- **`lib.rs` (the only shared-file change, listed per the rules):** extracted the render loop's tick
  scheduler verbatim into `pub fn ticks_for_frame(anim_time: &mut f32, dt: f32, max_ticks: u32) ->
  u32` and the yaw half of `lerp_cam` into `pub fn lerp_yaw`. The winit loop now calls
  `ticks_for_frame` (a 3-line replacement, behaviour-identical; `before` for the HUD-title check
  stays at the call site). This gives the headless tests the exact arithmetic the real loop runs.
- **New `crates/nf-view/tests/fps_determinism_live.rs`** (4 tests):
  - `render_loop_rate_does_not_change_the_simulation` — two fresh loads of 07000006 (G-043's
    subject), one driven at 60 Hz frame times, one at 144 Hz, over 4.005 s of wall time (0.3 tick
    clear of the boundary; f32 drift is ~1e-4): identical sim states — 240 ticks run, Bond's head,
    health and `drones.frame` equal, every 60 Hz frame exactly 1 tick, no 144 Hz frame above 1,
    most 144 Hz frames 0 ticks (the cache's no-tick frames).
  - `slow_frames_drop_ticks_never_burst` — the 5 fps loop asks 12 ticks/frame and is dropped to the
    cap of 4: 80 ticks total, dropped-not-deferred (the fix-perf-level contract).
  - `scheduler_boundary_arithmetic` (headless sister) — the 1/60 / 1/120 boundary, the cap (the
    clock still advances by dt·60, only the run is capped), a negative dt runs nothing.
  - `lerp_yaw_takes_the_short_way_round` (headless sister) — 3.1→−3.1 sweeps ±π, not 0; t = 1.0
    lands on b's direction across the wrap.
- **Verified the oracle** (`gpu_skin_live`, untouched): `gpu_palette_matches_the_cpu_pose_on_
  07000005_06_07` green — worst position error **1.5258789e-5** (2⁻¹⁶, at the task's 1.5e-5 bar),
  **0 colour mismatches**, 712 164 / 783 629 / 836 908 vertices and 7 324–12 266 palette matrices
  on 05/06/07 at ticks 1 and 240; `sister_a_wrong_palette_is_caught` green.
- **`docs/GAPS.md`** — G-043 row closed with the evidence. **`docs/WORKLOG.md`** — outcome appended
  to my entry. Nothing else touched: scene.rs, gpu.rs, player.rs, drone.rs are byte-identical to
  master (checked via `git diff --stat origin/master`).

## How it was verified

- Full suite on the slim ISO (`NF_ISO` set, `--test-threads=1`, release):
  **nf-game + nf-formats + nf-audio 326 passed / 3 failed — the failures are exactly the sanctioned
  `disc_golden` trio (bigf_header_fields_behave_as_documented, bigf_tables_match_the_golden_manifest,
  directory_tree_matches_the_golden_manifest)**; **nf-view 482 passed / 0 failed** over all 152
  binaries (portal_live re-run to completion after a chunk timeout, +4). Total 808 passed / 3 failed.
- The fps-determinism tests fail on a tree without the extraction? They do not — they pin landed
  behaviour, not a fix; there was no bug to reproduce this session. The closest fail-first evidence
  I produced: before I fixed my own test's wrong expectation (the capped clock legitimately
  advances by dt·60; the wrap result is b's equivalent angle), the two headless sisters failed —
  the assertions now state the actual contract.
- Sandbox notes: no GPU adapter here, so the wgpu surface path (`--fps`'s present mode, the pacing
  sleep, the visual interpolation quality) could not be exercised — see the live checks. The oracle
  itself is CPU-side (it replicates the shader's arithmetic), so it runs headless. Also: the slim
  ISO was evicted mid-session by what looked like a sandbox disk cleaner and had to be re-downloaded
  (size verified 369 823 203 both times); the suite ran on the restored copy.

## What's left

- Nothing code-side for G-043's three items. Optional future work: a `--fps`-driven state-hash
  end-to-end check through the real winit loop (needs a window), and the bench numbers on real
  hardware (the WORKLOG's 13.2 ms → palette ~1.5 ms split) to confirm the 120 fps headroom on
  07000006.

## Live checks for the main agent

1. **nf-view render, 07000006 start, animated guard visible:**
   `--bond 149.5,46.0,-95.0,3.6,0` — the character should animate smoothly; with `NF_NOINTERP=1`
   the same camera should show a slight 60 Hz judder at 120+ Hz. Compare frames: interpolation must
   not smear (the palette lerp is per-matrix with the 4-unit snap; a teleporting/cut character must
   jump, not blend).
2. **`--fps` caps:** run `--fps 30`, `--fps 60`, `--fps 144` (uncapped display) for 10 s each at the
   same start; the title-bar health and a fixed walk input should reach the same spot (the sim
   state must not depend on the cap). `--fps 0` = uncapped.
3. **The pose cache:** on a 120+ Hz display, frame time while a character animates should show no
   `animate`/`posed_vertices` cost on no-tick frames (`NF_BENCH` on `--shot` still exercises the
   CPU path for reference).

## ntfy

Sent at the end of the session (see the WORKLOG); if the send failed, the exact error is in the
"ntfy failed" section below.
