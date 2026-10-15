# fix-hud-zoom-tilt (110z) — the view-model tilt with zoom and aspect (G-044)

## What I found first

**G-044's fix was already on master.** `fix-viewmodel-zoom` (commit `b12a1b77`, merged 2026-10-13)
appended `/ self.zoom` to both gun-tilt terms in `scene.rs` `gun_matrix` and carried the 4/3 aspect,
with `fix_viewmodel_zoom_live` as its test. This task message's premise ("a fixed 4/3 and no zoom
divide") was stale against the tree I was given. So this task became: verify the fix against the
decompile independently, pin the exact terms the task specifies (the merged test only checks the
composed angle to a loose 10% at an uncontrolled zoom), and close the gap.

## The decompile chain (all re-derived this session)

- `Player_WeaponRecoil` 0x1a3de8:52-57 [M]: the gun rot (`BLData+0x7e8` object) gets
  `+0x50 = BL+0x124 · (−π/6) / viewer+0x108` and
  `+0x54 = viewer+0x114 · −(BL+0x120 · π/6 / viewer+0x108) + BL+0x8e4`; `+0x58 = 0`.
- `viewer+0x108` is the zoom: `Player_PositionCamera` 0x18f5d8 writes `viewer+0x108 = BL+0x8D0`
  every frame in cam modes 0/0xe, and pins it to 1.0 in mode 1 [M 0x18f5d8 case 0/0xe/case 1].
  The port's `Sim::zoom` is `BLData+0x8D0` (Player_Zoom 0x1a1a70 ramps it).
- `viewer+0x114` is the **display aspect**: `Camera_CalcViewAngles` 0x1baf70:29-41 writes
  0x3faaaaab (4/3), or 0x3fe38e39 (16/9) when `DAT_002a37c4` is set — that DAT is the game's
  widescreen option (`switch_CONFIG_WIDESCREEN`, C_CHCHWS_Handler 0x216168:10) — then ×0.5 or ×2
  for a two-viewer (split-screen) pair per `DrawInfo` [M].
- [L ee_start_05] viewer 0x1fc6c10 (re-read from the snapshot myself): +0x108 = 1.0, +0x114 =
  0x3faaaaab (4/3), +0x118 = π/3 (the FOV), +0x10c/+0x110 = 512/448. The port models the 4:3
  single view, so its `4.0 / 3.0` literal is the game's value bit-for-bit (asserted).

So the answer to the task's question: **+0x114 holds the display aspect** — 4/3 by default
(16/9 in the widescreen mode the port doesn't model; ×0.5/×2 in split screen). The port's fixed
4/3 is correct for the modelled display; the missing piece was only the zoom divide, which
b12a1b77's diff supplies (and nothing else — verified via `git show b12a1b77`).

## What this task did

1. **`crates/nf-view/tests/fix_hud_zoom_tilt_live.rs`** (new) — the task's exact test properties:
   - `the_tilt_terms_follow_the_game_formula`: extracts both tilt terms from `gun_matrix()` (gp =
     asin(up·gf), gy = atan2(−left·gf, fwd·gf) through the orthonormal view frame) and asserts
     at zoom 1 they equal the no-divide formula (the sister: **zoom 1 unchanged from master** —
     the merged diff only appends `/ self.zoom` and IEEE ÷1.0 is exact, so the zoom-1 matrix is
     bit-for-bit master's), at zoom 2 **each term is half its zoom-1 value** (IEEE ÷2 exact), and
     both match the formula with the 4/3 constant. Also pins `4.0/3.0 == f32::from_bits(0x3faaaaab)`.
   - `the_zoomed_tilt_tracks_the_ramp_through_the_real_input_path`: aim + hold zoom on the Ronin
     (0x32, zoom 3.5 [L ee_start_05 weapon_data+0x88]) — at every tick of the `Player_Zoom` ramp
     the terms equal the formula at that tick's zoom (fail at the first ramp tick before the fix).
   - `the_aspect_constant_is_the_games_bits` (pure): the constant's bits.
   - No scoped weapon zooms to exactly 2.0 (the table's zooms are 3.5/4/6/10/16 [L ee_start_05]),
     so the exact-zoom-2 leg sets `Sim::zoom` = 2.0 directly — BL+0x8D0 is the value
     `Player_PositionCamera` feeds viewer+0x108 verbatim each frame, and 2.0 is a mid-ramp state
     the real path passes through.
2. **Fail-first shown**: with the divide reverted in `gun_matrix` (the pre-b12a1b77 lines), the
   zoom-2 leg fails ("zoom-2 yaw term 0.174533 vs half the zoom-1 0.174533" — the term kept the
   zoom-1 value) and the ramp leg fails at tick 0 (zoom 1.0609756, term still the zoom-1 value);
   the zoom-1 assertions pass in both trees, as the sister predicts. Restored, 3/3 pass.
   `scene.rs` is byte-identical to master after the round-trip (`git diff` empty).
3. **docs/GAPS.md**: G-044 → Closed, with the full address chain and the one remaining nuance.
4. **docs/WORKLOG.md**: the `fix-hud-zoom-tilt` entry.

**Shared files touched: none.** The task named `scene.rs`, but the fix being already merged, the
correct change was zero — my revert/restore demo left it byte-identical. Only the new test file
and the two docs changed. `fix_viewmodel_zoom_live` still passes (3/3 re-run this session).

## Verification

- `fix_hud_zoom_tilt_live` 3/3 on the final tree; `fix_viewmodel_zoom_live` 3/3 (same session).
- Fail-first: shown on the reverted divide (above).
- Full suite (fast runner, `tools/test/suite.py`, slim ISO) on the final tree: the window fit a
  **partial — 670 passed, 0 failed, 58 ignored over 211 of 337 test binaries** (killed at the
  16:29 stop-new-work mark; `fix_hud_zoom_tilt_live` 3/3 and `fix_viewmodel_zoom_live` 3/3 are
  inside the partial; zero FAILED/TIMEOUT/CRASH lines). On the slim ISO only the 3 `disc_golden`
  tests (and `pss_movies_live` without the retail image) may fail — none reached, none failed.
  After the partial, master moved and was merged (394ef4a7: mp-bots-states + fix-progress-detected-
  hook + fix-viewmodel-zoom's tree); both zoom test files re-run 3/3 on the merged tree (their
  delta doesn't touch the gun axes).

## What's left

- **The recoil kick term**: the game's rot+0x54 also adds `BL+0x8e4` (decayed ×0.75 each tick in
  Player_WeaponRecoil 0x1a3de8:43-44; zeroed at Player_InitWeapon 0x1a7ef8:70-71). The port
  doesn't model BL+0x8e0/0x8e4 at all, and the C decompile shows no increment into them (only the
  decay/init/read) — a decompiler-dropped store is suspected; the disassembly of the fire path
  (Player_WeaponFiring 0x1a5460 / DroneWeap_FireWeapon 0x178f68) should be checked for the
  increment. Noted inside the G-044 closure; it needs its own task with a PCSX2 trace.
- The widescreen (16/9) and split-screen (×0.5/×2) branches of viewer+0x114 are not modelled —
  correct for the port's 4:3 single-view target; a future widescreen mode would need
  `Camera_CalcViewAngles`' branch ported.

## Live checks for the main agent

nf-view (no GPU here; render-check the tilt, not the reticle). Spot: `nf-view 07000005 --bond
105.91,25.32,108.13,0,0` (the level's default start), Ronin (0x32) or sniper selected:
- Zoom 1, crosshair held right-down (aim + mouse to one side): the gun yaws toward the crosshair
  and pitches down slightly (terms gy 0.131 / gp −0.105 rad at cross 0.25, −0.2).
- Same spot, aim + hold zoom (right mouse) so the zoom ramps: as the zoom rises the gun's turn
  visibly halves (at zoom 2 the terms are gy 0.065 / gp −0.052) — the gun stays much closer to
  the view forward than at zoom 1. Pre-b12a1b77 it kept the zoom-1 turn at every zoom.
- Zoom 1 vs zoom 1 again: bit-identical pose (the sister).

PCSX2 (optional, only if the kick term gets its task): gdbwatch `BL+0x8e0`/`BL+0x8e4` (BL = the
player's BLData) during sustained fire on 07000005 — expected: non-zero spikes on shots, ×0.75
decay between them, and the view-model yaw kicking with them (Player_WeaponRecoil 0x1a3de8:54-57).
