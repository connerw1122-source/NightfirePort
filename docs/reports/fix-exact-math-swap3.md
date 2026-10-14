# fix-exact-math-swap3 — the cut-scene camera: why the merge held, and what the game's arithmetic is

Task: the merge of `s/fix-exact-math-swap2` (+ master) was HELD because renders of master vs the branch
differed by "far more than float precision can explain" (the 07000002 intro camera sits further
right/closer at frames 140/170/200; the 07000007 hall guard leaves the frame with `--skip-cutscenes`).
The brief: dump the frame-170 NIS camera matrix on both trees element by element, find the first
function whose output differs by more than 1e-5, decide from the decompile (RotMatrix 0x1e2368,
Quat_QuatTransToMat 0x1e3fa8, psiBuildMatrixPalette 0x111498) which side is the game's, keep the
grenade-bounce fix.

## What was found

**1. The symptom does not reproduce on the current trees — the renders are pixel-identical.**

I rendered the hold's exact spots headlessly (lavapipe software Vulkan, 1280×720, the same
`--bond`/`--frame`/`--skip-cutscenes` arguments) on the branch (`s/fix-exact-math-swap3` = swap2 +
master @ 7785582) and on the same tree with master's host math swapped back into
`nf-formats/src/{anim,placement}.rs`:

| spot | branch vs master |
|---|---|
| `07000002 --bond -82.30,5.30,32.05,1.570,-0.100 --frame 140` | **0 of 3,686,400 bytes differ** |
| `07000002 ... --frame 170` | **0 bytes differ** |
| `07000002 ... --frame 200` | **0 bytes differ** |
| `07000007 --bond 42.59,44.10,-71.71,-1.228,0 --skip-cutscenes --frame 170` | **0 bytes differ** |

**2. The frame-170 camera matrices differ by a few ulps, not by "far more than precision".**

The dump test (`crates/nf-view/tests/camera_matrix_dump.rs`) prints the live `Obj::Camera` matrix at
the tick, through the real input path (`nf_view::load` + `Sim::step`). At 07000002 tick 170:

- branch: `bf7a6949 af300002 be54cc46 | 39b5ff95 3f7fffec bad62af6 | 3e54cc33 badaf298 bf7a6933 | c1da0412 3fb46dcb 420993c4`
- master: `bf7a694a af200000 be54cc45 | 39b5ff94 3f7fffe9 bad62af4 | 3e54cc31 badaf296 bf7a6932 | c1da040f 3fb46dc9 420993c2`

Largest absolute difference: **5.1e-6** (translation row, −27.251987 vs −27.251982). The derived yaw
is bit-identical on both trees (0x403baa18), pitch differs by 1 ulp, fov is identical
(0.49041873). Actor positions are identical (drone0 −24.3307/0.9510/31.9223, yaw −1.466483 on both;
the 07000007 hall guards at 44.32/44.0/−62.13 and 44.15/44.0/−61.10 on both, same drawn flags).
There is no function whose output differs by more than 1e-5 on this path.

**3. Per-function host-vs-exact divergence is ≤ 1.1e-6 over wide domains** (`exact_vs_host_probe.rs`):
`anim::mul` 7.2e-7 over 20000 random matrix pairs; `object_matrix` 1.1e-6 over the [−4π, 4π] euler
grid; `quat_trans_to_mat` 4.8e-7 over 20000 random quats. A bit-exact `sinf` differing ~1 ulp cannot
move the camera — confirmed: nothing on this path exceeds 1e-5.

**4. The decompile says the branch's arithmetic is the game's** — both sides have the same STRUCTURE,
the branch has the game's ARITHMETIC:

- `RotMatrix` 0x1e2368 (decomp lines 28-40): two `PS2Sinf3` calls with the caller-computed cosines
  (`*param_1 + 1.5707964` etc. — the π/2 add happens in the caller, on the FPU), then the nine
  products/sums — row-vector matrix, translation row 3 (`Mat_SetTrans` at the call site). The branch's
  `object_matrix` mirrors entry for entry: `m(cy, cz)`, `(sx·sy)·cz + cx·sz` in the decompile's
  left-associated order, `(−sx)·cy`, `cy·(−sz)`, `m[2][0] = sy` — with chop products and no-guard FPU
  adds. Master's host version has the same structure with IEEE-nearest host ops.
- `Quat_QuatTransToMat` 0x1e3fa8 (decomp lines 16-35): `y2 = y+y, z2 = z+z`, `xx2 = (x+x)·x`,
  `xw2 = (x+x)·w`; entries `m[0][0] = 1 − (y2·y + z2·z)`, `m[0][1] = y2·x + z2·w`, `m[0][2] = z2·x − y2·w`,
  `m[1][1] = 1 − (xx2 + z2·z)`, `m[1][2] = z2·y + xw2`, `m[2][0] = z2·x + y2·w`, `m[2][1] = z2·y − xw2`,
  `m[2][2] = 1 − (xx2 + y2·y)`, translation in row 3 (`param_3[0xc..e]`). The branch's
  `quat_trans_to_mat` is exactly this with EE arithmetic (chop products, no-guard adds); master's is
  exactly this with host arithmetic. No convention, sign or order difference on either side.
- `psiBuildMatrixPalette` 0x111498 (the VU block): the game composes each bone's rotation as a
  QUATERNION chain on the VU0 and applies translations as row-vectors — `t' = t_local·M_parent^{3×3} +
  t_parent` (`vmulabc/vmaddabc` over the parent matrix's rows), and the skin-to-world pass composes
  `v·M_world` the same way. So the composition convention is row-vector, local·base — the same order
  the port uses everywhere (`mul(&keys_matrix, &parent)`, `mul(&m, &self.matrix)` in nis.rs, case 4's
  camera `Concat` tail). The palette's quat-chain math is nf-game code that did NOT change between
  master and the branch (the diff is exactly anim.rs + placement.rs + grenade.rs + one test).
- The arithmetic model itself (chop products, guard-bit VU sums, no-guard FPU adds, PS2Sinf) is the
  one proven bit-exact against 3000 interpreter samples and the 40 calls recorded from the running
  game (docs/formats/animation.md "Arithmetic"; `crates/nf-game/tests/diff_calls.rs`). **The branch is
  the game's math; master's host math is the documented stand-in** (the same conclusion swap/swap2
  reached; the escort-walk divergence that originally blocked the swap was root-caused in
  fix-exact-math-swap).

**5. The failing-first test pins the camera to the decompile's arithmetic** (`fix_camera_exact_math.rs`):
it recomputes the tick-170 camera matrix from the stream's key inputs (a `Script_GetInterp` 0x1da158
mirror: the clock, key range and spline flag via the new `Scripts::camera_stream_inputs`, the key
lerp/slerp, then INLINED copies of the decompile's `Quat_QuatTransToMat` and matrix-product chain) and
asserts it equals the sim's `Obj::Camera` matrix:

- on the branch: **passes bit-for-bit** (`bf7a6949 af300002 ... c1da0412 3fb46dcb 420993c4` — both lines);
- on master's host math: **fails** (camera `bf7a694a af200000 ...` vs recompute `bf7a6946 af300000 ...`),
  i.e. master's camera is NOT the decompile's arithmetic — the test would have caught the hold;
- sister assertion: every entry within 2e-6 — passes on BOTH trees, pinning "the two trees differ
  only by float precision" so a future gross divergence fails loudly.

**6. Why the hold renders likely differed** [S]: the current trees cannot produce the held images
(pixel-identical, ≤5e-6 numeric). The hold note's own premise ("differs at every frame, so it isn't a
timing shift") fits a comparison made against an older tree or a stale build: master's 02-intro
set/lighting was re-verified the same morning (docs/merge-log.md @ 8295049: "fix-cutscene-brightness2
... The 02 intro actors draw in the set, lit to the room (render-verified at frame 170)"), and swap2's
report already recorded this pattern once (its walkreal red test predated fix-08-collision2, whose
merge repaired it). I tried: fresh builds of both trees, the exact hold arguments, all three frames,
the 07000007 skip spot, and a numeric element-by-element camera/actor dump — no visible difference
exists on the current trees.

**7. The grenade-bounce fix is kept**: `crates/nf-game/src/grenade.rs`'s 0.04 normal offset
(Bullet_CollisionHandler 0x12ce80) survived the merge; `grenade_live` 2/2 and `ai_live` green on the
final tree.

## How it was verified

- Pixel diffs of the four render pairs above (byte-level PNG compare, unfiltering-aware).
- The numeric camera/actor dumps on both trees (`camera_matrix_dump.rs`).
- The probe and the failing-first bit-exact test (numbers above).
- Targeted suite batches on the final tree (merged with origin/master 7785582):
  `placement,anim,exact_vs_host_probe,fix_camera_exact_math,camera_matrix_dump,grenade_live,ai_live,walkreal_07000001_live`
  → **45 passed, 0 failed**; `doors_live,doors_01_live,pickup_live,searchlight_live,trig_live,models_misc_live,fix_npc_doors_live,worldobj,copter`
  → **19 passed, 0 failed, 1 ignored**. The full four-crate suite did not fit this sandbox's window
  (background jobs are killed between tool calls here, so the fast runner ran in foreground chunks);
  the batches above cover every consumer family of the three swapped functions.

## Shared files touched

- `crates/nf-game/src/nis.rs`: the additive `Scripts::camera_stream_inputs()` accessor (12 lines) so
  the test can read the camera stream's key inputs without exposing `Stream`. No behaviour change.
- `crates/nf-view/tests/{fix_camera_exact_math,camera_matrix_dump}.rs`,
  `crates/nf-formats/tests/exact_vs_host_probe.rs`: new test files.

## Live checks for the main agent

1. Re-render the four pairs on fresh builds (the commands in the table above; the exact
   `nf-view <iso> 07000002 --bond -82.30,5.30,32.05,1.570,-0.100 --frame 170 --shot x.png` and
   `nf-view <iso> 07000007 --bond 42.59,44.10,-71.71,-1.228,0 --skip-cutscenes --frame 170`). Expected:
   master and branch identical. If they differ on your machine, the sandbox difference is in the build
   state, not the source — send me the two trees' `git log -1` and I will bisect.
2. PCSX2 confirmation of the exact-path values (gdbwatch on the running 07000002 intro's camera
   stream — the one `Script_CameraStart` opens; the port's `camera_stream_inputs()` identifies it):
   the camera matrix's translation should read
   ≈ (−27.2520, 1.4096, 34.3943) and forward row ≈ (0.2081, −0.0016704, −0.97765) — the branch's
   values; master's differ in the 6th decimal.
3. nf-view: `--bond -82.30,5.30,32.05,1.570,-0.100 --frame 140/170/200` — Bond centre-left, the bald
   man right of centre at all three frames (the hold's "Bond at the left edge" state is gone).

## What's left

- The full-suite total on the final tree (the driver's merge gate) — the targeted batches are green;
  run `tools/test/suite.py` once on the merge candidate.
- If the main agent's fresh renders DO differ (live check 1), the follow-up is a bisect over master
  commits between 8295049 and 7785582 with the render pairs as the test.
