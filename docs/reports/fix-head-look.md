# fix-head-look — the playtest 1013-1758 head-look bugs + the facial3 takeover

Branch `s/fix-head-look` (master 8144bd6 + `origin/s/fix-facial3` 9947360 merged at 16606c1).

## What the game's head-look actually is (the evidence base)

- `AnimObjectHeadTrack` 0x1bf618, called from `psiBuildMatrixPalette` 0x111498 on **bone 5** when
  the sAnimObject's flag word has the 0x30 bits (`AnimObjectIsHeadTracking` 0x1c6850), and on bone 2
  by `AnimObjectAimAt` 0x1bfe58 when 0x180. The palette entry is 0x40 bytes: rotation rows 0..0x30
  then the translation at +0x30; the per-bone quaternions are a separate array (both world-space,
  rebuilt from the anim every call).
- `sAnimObject_tag` = `*(obj+0xDC) + 0x70` (docs/fields/animobject.md header) — that resolves every
  offset in HeadTrack: flags 0x10 track-on / 0x20 settled / 0x40 object-target at +0xC8, target at
  +0x70, speed at +0xE0, **range² at +0xE8 ([L-read] drone 9.0)**, stored yaw/pitch at +0xF0/+0xF8,
  **the tracked bone at +0x104 ([L-read] drone = 5)**, the snap-delay counter at +0x106.
- The angles are GEOMETRIC, measured from the fresh anim pose's head axes: the yaw axis = the NECK
  axis `normalize(palette[5].pos − palette[2].pos)`, the pitch axis = `normalize(cross(neck, row2))`
  (row2 = the head's world +z row); each angle is a plane-projected acos signed by a scalar triple
  product (0x1bf618's two blocks). Clamps: yaw ±1.1, 0 beyond 2.6; pitch [−1.0, +0.6] in the game's
  positive-down convention. Eased by `Ang_RotateToBearing` at +0xE0 (0.07, or 0.14 when
  `DCVars+0x50c` alertness > 0.66 — `NDrone2_DoHeadTracking` 0x1472c8).
- The composition is `q_yaw ⊗ q_pitch ⊗ q_world_head` — a **world-frame premultiply** of the head
  bone's world rotation (Quat_QuatToMat writes bone 5's palette rows; the head has no children).
- The tracked point = the target object's **bone-5 world position** (`SetHeadTracking` 0x1c6748
  stores the bone id at +0x104; callers `NDrone2_DoHeadTracking` 0x1472c8 / `NDrone2_HeadTrackObj`
  0x14ada0 pass 3.0 as the range, and the re-decide timer `+0x358 = frame + (Rand(5)+2)·FR·FR_DIV`).
- **Script-cam gate**: `if (ScriptCam != 0) proceed; else proceed only if
  min(bytes at *(obj+0xDC)+0x110..0x113) < 0x21` — the game's HeadTrack RUNS during script cameras
  (the AI's flag bits decide, not the camera); the 4 bytes are inside the sAnimObject block
  (+0xA0..0xA3 = the post-resolve callback fn pointer per docs/fields/animobject.md) — semantics
  unresolved, listed under "open" below.

## What was wrong in the port (and the fixes)

All in `crates/nf-game/src/drone.rs` (plus the test files; no shared-file edits beyond drone.rs).

1. **The roll** ("heads turning the wrong direction, wrong angle"): the port post-multiplied
   model-axis rotations in the head bone's own animated local frame (`mul(w, rot)`), the game
   premultiplies world-frame rotations. Any tilt in the anim's head frame turned the yaw into a
   roll. **Fix**: the world-frame application `world5 · axis_angle(side, hp) · axis_angle(neck, −hy)`
   (row-vector transposes of the game's two column-form rotations; the product order matches
   M(q_yaw ⊗ q_pitch ⊗ q_bone)). The axes come from the pose itself (neck = head − bone 2, side =
   cross(neck, row2)) exactly as the game derives them from the palette.
2. **The double-counted pitch** ("the guest's head tips UP well past the camera"): the port eased
   toward `atan(dy/dist)` measured from the BODY axes over a fixed +1.55 stand-in — the anim's own
   head pitch was applied twice. **Fix**: `Drone::head_track_want` measures the want angles the way
   HeadTrack does (the two plane acos blocks, the same sign conventions and clamps) from the FRESH
   anim pose (`object_pose_inner(d, false)`), with the tracked point taken into object space
   (the port factors the placement into `palette()`'s object matrix).
3. **The range gate**: the game's angle block only runs within 3.0 of the tracked point
   (`SetHeadTracking`'s 0x40400000, range² 9.0 at +0xE8); beyond it the angles ease back to 0. The
   port tracked at any sighted distance. **Fix**: the tick's want is (0, 0) beyond `dr.dist <= 3.0`,
   keeping the game's draw structure identical (no RNG stream shift — the seed-pinned suites are
   unchanged and green).
4. **The target point**: the tracked point stands in the AI's player-eye snapshot (`player.eye`,
   the PlayerTarget V3) — the stand-in for the target's bone-5 world position (+0x104 = 5 [L]).

## The facial3 takeover (task 276c)

`origin/s/fix-facial3` merged at 16606c1 (two additive conflicts resolved: the Drone fields and the
WORKLOG — both sides kept). On the merged branch:

- `fix_cutscenes2_live::the_kimiko_scene_characters_play_their_face_tracks` — **pass** (5/5 file).
- `gpu_skin_live::gpu_palette_matches_the_cpu_pose_on_07000005_06_07` — **pass** (3/3 file).

(On master+facial3 these failed; on my merge both pass — the conflicts were field-appends only, so
the difference is master-side churn the merge picks up, not a semantic choice.)

## Tests

- New `crates/nf-view/tests/head_look_live.rs` (real input path, 07000007):
  - `tracking_head_stays_upright_and_points_at_bond` — a character tracking eye-level Bond 2.4
    units away: the posed head bone's world up stays within ~4° of vertical (0.9978 measured) and
    its forward points at Bond within 0.215 rad. **Post-fix green.**
  - `head_track_holds_still_beyond_the_game_range` — no head swivels at 6 units (beyond the game's
    3.0 gate). **Post-fix green.**
- `crates/nf-game/tests/fix_ai_remainder_live.rs` `head_tracks_bond_until_the_aim_takes_over`
  re-pinned to the game's semantics: the tracking phases stand Bond 2.4 units from the guard
  (inside the 3.0 gate, cited); the pose check (the posed head carries the tracked yaw — the delta
  between the tracked and the head-zeroed control pose) **passes, which pins the premultiply
  composition**; the body-relative signum check was replaced by a gap-closing check (the want now
  measures from the anim's head axes, so the body-relative offset can legitimately be 0.000); the
  torso-aim phase keeps the original far spot (at 2.4 units the body already faces Bond and the aim
  has nothing to wind up).
- Failing-first status, honestly: my headless tests do **not** reproduce the visible roll — both
  new tests pass on the pre-fix code too at the spots I found (the first enabled drones' anims hold
  the head frame near-identity, where the local-frame application coincides with the world-frame
  one; I sampled max-tracking ticks across every enabled drone on 07000007). The repro of the
  visible symptom needs the playtest spots' exact anim states — that is what the live checks below
  are for. The composition change itself is pinned by the fix_ai_remainder delta check and by the
  decompile analysis above (every constant and axis cited).
- Suites run (NF_ISO, release, `--test-threads=1`):
  - `nf-game`: **256 passed / 0 failed** over all binaries.
  - `nf-formats` + `nf-audio`: 85 passed / **3 failed — exactly the 3 `disc_golden` tests**
    (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
    `directory_tree_matches_the_golden_manifest`) — the documented slim-ISO allowance.
  - `nf-view` targeted: `head_look_live` 2/2, `gpu_skin_live` 3/3, `fix_cutscenes2_live` 5/5,
    `ai_traces_live` + `facial3_live` + `fix_drone3_live` + `fix_npcs_live` 16/16 (the seed-pinned
    body), `fix_ai_remainder_live` 2/2.
  - The full `tools/test/suite.py` could not run to completion in this sandbox window (the sandbox
    kills background jobs between tool calls, and the foreground window did not fit the whole run
    on 2 cores); the coverage above is every suite the head-look change can touch, plus the whole
    nf-game body. **The final tree was not re-run through suite.py — the next session should.**

## Open / left

1. The **ScriptCam==0 gate bytes** (`min(*(obj+0xDC)+0x110..0x113) < 0x21`, 0x1bf618:7-25) — the
   offset lands on sAnimObject+0xA0..0xA3 (the post-resolve callback fn pointer per
   docs/fields/animobject.md), which cannot be the real semantic; the decompile's param mapping
   needs a live look (the trace below).
2. The **snap-delay draw** (`SetHeadTracking` writes `Rand(FR/4)+FR/10` to +0x106 when its param_5
   is set — HeadTrack holds the stored angles while it counts down). The port does not implement
   the hold and does not draw it; adding the draw would shift the RNG stream against the seed-pinned
   tests (the facial3 lesson), so it is left out and documented here.
3. The headless roll repro (see Tests above) — needs the playtest spots' anim states.
4. The cut-scene gating: per the decompile the game's HeadTrack runs during ScriptCam whenever the
   AI's flag bits are set — scene characters whose AI never calls SetHeadTracking keep their anim
   pose (their 0x30 bits stay clear), and pre-existing tracking eases out (the bVar5==2 path clears
   0x10|0x20 once both eased angles are < 0.01). The port's structure matches this (the stored
   angles live on the drone, the tick's AI gates decide); nothing more was needed beyond the fixes
   above — the Kimiko scene test is green.

## Live checks for the main agent

PCSX2 (real game, 07000007), with the port's nf-view at the same `--bond` spots for comparison:
1. `--bond 42.59,44.10,-71.71,-1.228,-0.089` (the playtest's door-guard spot): the guard's head
   stays upright (no roll toward the camera) and only yaws toward Bond. In the port the head bone's
   world up should now read within ~4° of vertical.
2. `--bond 42.86,44.10,-77.92,2.484` (the tipping guest): the guest's head pitch should now point
   at Bond's eye, not past the camera.
3. `--bond 43.21,44.10,-61.98,1.309` (the over-looking guest): the head pitch follows Bond's actual
   eye height (the port's tracked point is the AI eye snapshot — crouch Bond and the guest's head
   should pitch down with him).
4. A guard with Bond 6+ units away: the guard's head stays at its anim pose in BOTH builds (the
   3.0 range gate) — watch a distant guard while Bond moves: no cross-room head swivel.
5. Trace request (gdbwatch on `*(drone_obj+0xDC)+0x110..0x113` during normal play and during a
   script camera): the access type and the values — to resolve the open gate-bytes item.
