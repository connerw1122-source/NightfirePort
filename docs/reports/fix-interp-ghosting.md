# fix-interp-ghosting — the gun and arm smear while moving (120 fps interpolation)

The playtest (2026-10-13, 07000005) reported "ghosting … when I move around … with my arm and
gun": it shows only in motion, never in a screenshot. It came with the 120 fps interpolation
(G-043, fix-gpu-skinning): between two sim ticks the draw loop blends the camera
(`lerp_cam`, by angle) and the skin palette (`lerp_palette`). The palette was blended
component-wise in WORLD space, but the view model (the hands and gun) is posed in world space
from each tick's own camera ([M `Player_PositionGun` 0x1a3b40] via `Sim::gun_matrix`), so
mid-turn its lerped matrices do not match the angle-lerped camera — the gun swims against the
view — and a component-wise lerp of a rotation is not a rotation: it shears and shrinks mid-way
(rows of two 30°-apart rotations average to length cos 15° ≈ 0.966). `NF_NOINTERP=1` hid it.

## What's done (crates/nf-view/src/lib.rs only)

1. **World matrices blend as rotation.** `lerp_palette` now blends each entry through
   `blend_pose`: slerp of the rotation part — the game's own `Quat_Slerp_Acc` [M
   work/decomp/ACTION.ELF/001e4188, ported as `nf_game::anim::slerp`] on a Shepperd-extracted
   quaternion (`rot_quat`/`quat_rows` for the port's row-vector matrices) — plus a component lerp
   of the translation. Entries whose 3×3 is not a **proper** rotation (the collapse/FAR hides,
   the scaled corona billboards; the det + 1 gate rejects reflections too) keep the old
   component lerp. The existing 4-unit snap for teleports/cuts is untouched.
2. **The view model blends in CAMERA space.** `reattach_view_model` (new, called on the upload
   path only when interpolating) takes the active view model's palette entries — its object
   matrix, its bones and its attachment slots (`view_model_entries`, from
   `scene.view_models[sim.gun.weapon]` + the skin layout) — converts both ticks' world matrices
   camera-relative with each tick's own camera (C = T(−eye)·Rᵀ; R rows = (left, up, fwd), the
   exact basis `Player_PositionGun` 0x1a3b40 builds the gun from — gl = up×gf = the viewer LEFT,
   gu = gf×gl; det + 1, identity at yaw 0), blends the relative poses at `alpha`, and re-attaches
   to the interpolated `draw_cam` (C⁻¹ = R·T(eye)) — the same scheme `lerp_cam` uses for the
   view. A relative-pose jump over 4 units snaps to the newer tick (draw/holster switches, the
   scoped hide), mirroring the world-space snap. Non-rotation entries (the collapsed muzzle-flash
   slot, hides) keep the plain world blend — nothing is drawn for them either way. The whole
   reattach is skipped while a script camera is live (then the tick cameras are not Bond's).
3. Draw-loop wiring: the interp upload branch blends + reattaches; `NF_NOINTERP`, pause, noclip
   and the non-interp path are byte-for-byte unchanged. `Camera`, `lerp_palette` and
   `reattach_view_model` are pub so the headless tests drive the exact production path.

The math the fix relies on: the gun's world matrix decomposes as local pose · C_tick⁻¹, so the
camera factor cancels exactly in camera space — a static local pose gives identical
camera-space matrices at both ticks, and re-attaching the blend to `draw_cam` locks the gun to
the interpolated view with no swim, by construction.

## How it was verified

`crates/nf-view/tests/fix_interp_ghosting_live.rs` (new, NF_ISO, 07000005 — the playtest level):

- `view_model_stays_glued_in_camera_space_across_a_turn` — the real input path (`nf_view::load`
  + `Sim::step_bond`, one real 30° `Input.turn` tick), palettes through `Scene::skin_palette`
  the way the loop builds them, then the loop's blend path (`lerp_palette` +
  `reattach_view_model`) at alpha ½. Every visible view-model entry's camera-space matrix must
  equal the average of its two ticks' camera-space matrices. **Shown failing before the fix**
  (verified against the pre-fix tree with the final test shape): gun-root entry 1889 translation
  off by 0.0307 (3.1 cm of swim) and rotation by 0.034 (the cos 15° shear). After the fix: ≤
  8e-6 translation, 0.0 rotation. Collapsed/hidden entries (the muzzle-flash slot between shots)
  are exempt — nothing draws for them.
- `interpolated_world_matrices_stay_orthonormal` (sister, no ISO) — two 30°-apart rotations
  blend orthonormal and land halfway (15°) at α = ½; the > 4-unit snap still snaps. **Shown
  failing before the fix**: row length 0.9659258 = cos 15° exactly. Green after.

Full suite on the final tree (this branch merged with origin/master 7b3bd80), the standard
runner (`tools/test/suite.py`, 209 binaries, run as three `--only` chunks of the same runner):

    869 passed, 3 failed, 26 ignored  (12 min)

The 3 failures are the allowed slim-ISO `disc_golden` tests; 0 TIMEOUT, 0 CRASH. The
fps-determinism proof (`fps_determinism_live`) stays green — the blend is a pure function of the
two ticks' state and alpha, no wall clock, no new RNG draws.

## What's left

Nothing code-side for this task. Two notes for future eyes: the collapsed muzzle-flash slot's
degenerate matrix keeps the world blend (it draws nothing, so the residual chord-shrink there is
invisible); and a firing flash's random roll [M `player_flash_matrix`] spins per tick, so if
anyone ever wants the flash itself mid-blend that needs its own look (not exercised here — the
test never fires).

## Live checks for the main agent

nf-view (the ghosting only shows in motion — static renders can't catch it):

- Interactive 07000005 at 120 fps without `NF_NOINTERP`: grab the mouse and turn fast. Expected:
  the arm and gun sit locked in the view while the world turns (before the fix they swam/smeared
  against it). Also walk forward while turning (the bob path blends in camera space too).
- `--bond 105.9,25.3,108.1,0,0` (07000005 spawn): the P2K raised bottom-right, no artifacts.
- `--bond 105.9,25.3,108.1,0.5,0.15`: turned view — the gun stays glued bottom-right at the same
  view offset (a regression would slide it toward the screen centre or squash it).
- Scoped zoom (right mouse on a scoped weapon): the view model hides; mid-blend no garbage flash.
- PCSX2 reference: the game's own first-person gun during a fast flick — the port should now
  match that locked-to-view feel at any frame rate.

## Files

`crates/nf-view/src/lib.rs` (the task's named file — slerp blend, camera-space reattach, wiring;
`Camera`/`lerp_palette`/`reattach_view_model` pub for the tests), `crates/nf-view/tests/
fix_interp_ghosting_live.rs` (new), `docs/WORKLOG.md` (my entry only), this report. `scene.rs`
turned out not to be needed — the palettes stay world-space per tick and only the draw-time blend
changed. No shared-file hooks beyond the named `lib.rs`.
