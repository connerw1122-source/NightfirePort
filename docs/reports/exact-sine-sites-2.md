# exact-sine-sites-2 — game-exact trig in missile steering and the drone aim pose (G-046 group 2)

Branch `s/exact-sine-sites-2` (from origin/master 0b91e999). Files: `crates/nf-game/src/trig.rs`,
`crates/nf-game/src/drone.rs` (the two named sites), `crates/nf-game/tests/trig_live.rs` (the
goldens), `crates/nf-formats/src/ps2float.rs` (+`div`), `crates/nf-formats/src/ee.rs` (+`set_gpr`),
`docs/GAPS.md` (G-046 markers). No other shared file touched.

## What the decompile says (the two sites)

**Site 1 — missile steering, `steer_missile` (drone.rs, was ~:5325-5326).** `Bullet_update`
0x12e7b0 substate 10 (guided camera): `Mat_GetDir` → `vecutil_cartesian_to_spherical_acc`
0x1e3a70 → `pitch' = pitch_delta − pitch`, `yaw' = yaw − yaw_delta` (0x12eb74 `sub.s f1,f1,f2`,
0x12eb7c `sub.s f0,f0,f3`) → `RotMatrixZYX(pitch', yaw', 0)` 0x1e25a8 → `RotMatrix((0,0,bank+yaw_δ))`
0x1e2368 → `MulMatrix0` → `Mat_GetDir` into the bullet's velocity. The decisive fact: vecutil's two
`jal`s land on **0x232e48 = the PS2 libm `atan2f`** (`__ieee754_atan2f` 0x2339f8), **not**
`ATAN2_APPROX` 0x1e4528 [M 0x1e3aa0, 0x1e3ab0] — so the port's `asin` had to become the libm
`atan2f(y, √(x²+z²))` [M 0x1e3a70:9], and the yaw `atan2f(x, z)` [M :11]. `RotMatrixZYX`/`RotMatrix`
get their sines from `PS2Sinf3` 0x11fc20 [M 0x1e25e8/0x1e2604 jal].

**Site 2 — head-track / aim pose (drone.rs `axis_angle_mat` ~:926, `object_pose_inner`
~:2440-2452).** `AnimObjectHeadTrack` 0x1bf618:323-328 builds the eased yaw/pitch quats with
`Quat_AxisAngleToQuat` 0x1e3f20 (in-place, input `{axis.xyz, angle}`), composes them with the head
bone quat (inlined VU0 MMI multiply), stores the composed quat back, and `Quat_QuatToMat` 0x1e3e80
writes the three rotation rows into the palette. AA2Q is disassembled end-to-end [M 0x1e3f38-0x1e3f90]:
`f12 = θ/2 + 1.5707964` (delay-slot `add.s`), `f13 = θ/2` into `PS2Sinf2` 0x11f968 with **a0 =
&out[3]** (out1) and **a1 = stack** (out2), so out[3] = sinf(θ/2 + 1.5707964) and xyz =
axis·sinf(θ/2) — the STANDARD axis-angle quaternion (my first transcription had the lanes swapped;
the interpreter golden caught it). `Quat_QuatToMat` writes 9 elements with EE doubling adds, chopped
products and no-guard sums [M 0x1e3e80:15-31]; it emits the **column-form transpose** (its +2zw
off-diagonals), so the port's Rodrigues-convention angle is handed to AA2Q as −θ — the port's [S]
bridge, pose-preserving (identity at 0 verified in the goldens).

## What was ported

- `trig::atanf_libm` = `atanf` 0x235310 (fdlibm s_atanf) operation-for-operation on the EE FPU:
  regions 0x50800000 / 0x3ee00000 / 0x3f300000 / 0x3f980000 / 0x401c0000, the tiny-|x| `huge+x>1`
  early-out, the two Horner chains p1 = z·(T0+s·(T1+…+s·T5)) and p2 = s·(C0+s·(…+s·C4)) — the game
  multiplies the T-chain by **z** (fVar6 = z at 0x235310:63; my first draft used w and the golden
  caught it) — and `atanhi[id] − ((w·(p1+p2) − atanlo[id]) − w)`.
- `trig::atan2f_libm` = `__ieee754_atan2f` 0x2339f8: the m = sign(y)|sign(x)<<1 quadrant tags, the
  x==1 → atanf(y), y==±0 (±π down 0x40490fda / ±0), x=±Inf and y=±Inf tables (π/4 0x3f490fdb,
  3π/4 0x4016cbe4, −0 0x80000000 …), the (|y|−|x|)>>23 exponent gates with the 0x3fc90fdc π/2-up
  cap, and the π−(z−1.50995788e-07) / (z−1.50995788e-07)−π sign adjustments. Every constant is from
  the EE snapshot [L atanhi 0x30ba08, atanlo 0x30ba18, poly 0x30ba28-0x30ba50, huge 0x30ba58,
  1.50995788e-07 0x30b530, −0 0x30b534] and the immediate loads in the disassembly [M 0x233aa8-0x233bf8].
- `trig::ee_add` / `ee_sub` (pub wrappers over the validated `add_noguard(…, true)` model).
- `drone::quat_axis_angle_to_quat` (0x1e3f20) and `drone::quat_quat_to_mat` (0x1e3e80), bit-exact;
  `axis_angle_mat` now routes through them; the aim-yaw rotation's sin/cos (drone.rs ~:2445-2452)
  are `trig::sinf`/`cosf`.
- `steer_missile`: pitch = `atan2f_libm(y, ps2sqrt(ee_add(mul(x,x), mul(z,z))))` (vecutil's formula
  on the normalized dir), yaw = `atan2f_libm(x, z) − mul(yaw_in, K)` (EE ops), rebuild
  `[mul(cp,sy), sp, mul(cp,cy)]` on PS2Sinf lanes. The port's angle-space ±0.995 pole gate and its
  pitch-input sign convention are kept (they are the port's validated interface, not decompile
  arithmetic — see the [S] note below).
- `ps2float::div` = nearest-rounded division, per the interpreter's own pinned model ([L] 739/750
  live ATAN2_APPROX results, ee.rs `div.s` note — the chop guess failed the golden).

## Verification (all green, run on this branch)

`cargo test --offline --release -p nf-game --test trig_live -- --test-threads=1`: **4/4** —
1. `libm_atanf_atan2f_match_the_game_bit_for_bit`: 1500 random x through the interpreter's `atanf`
   0x235310 and 1500 (y,x) through `atan2f` 0x232e48 — **0 diffs**; the vecutil pitch
   (0x1e3a70, f12/f13/f14 + a0=&out per the call site 0x12eb44-0x12eb50) — **0 diffs**;
   *fail-first sister*: the old site's `asin` misses the game's bits on **624/1500** inputs.
2. `quat_pose_functions_match_the_game_bit_for_bit`: AA2Q 0x1e3f20 (600 in-place calls) and
   Quat_QuatToMat 0x1e3e80 (all 12 written floats) — **0 diffs**; RotMatrixZYX 0x1e25a8's dir row
   vs a transcription of its decompile formula (rz=0, including the game's own sinf(π/2)=0x3f800001
   factor) — **0 diffs**; *fail-first sister*: the old host `sin_cos` rebuild misses the game's row
   bits on most inputs.
3. `probe_sinf3_lanes_vs_sinf`: PS2Sinf3 0x11fc20's three lanes vs `trig::sinf` — **0/900**.
4. the pre-existing ATAN2_APPROX/PS2Sinf/Vec_AngleDifference golden — still green.
`cargo test -p nf-game --lib`: **170/170** (includes the new atanf/atan2f unit tests + sisters:
the game core differs from host libm on 2000-sample sweeps). `cargo test -p nf-formats --lib`:
**64/64**.

## Suite (foreground chunks — this sandbox reaps background jobs)

**477 passed / 0 failed** on this branch: nf-game all test binaries **289/289** (the AI-fight and
drone live tests with the changed pose bits — none flipped) and nf-view **188/188** in foreground
chunks (this sandbox reaps background jobs): the alphabetical prefix through dump_drone3 (62, cut by
a chunk timeout, no failures), then targeted batches — dump_drone4/probe_drone4/fix_drone3_live/
fix_drone4_live/head_look_live/fix_aim_raise_live/fps_determinism_live (22); walk_07000005 (the
AT-420 level)/06/07 + fix_ai_remainder_live/fix_guard_misc_live (13); models_characters_live/
models_weapons_live (its re-pinned golden holds)/fix_guest_reactions_live/weapon_anims_live (20);
fix_weapon_fire_modes_live (the guided-cam weapons)/sentinel_camera_live/walkreal_07000005/
fix_weapons_all_live/walk_07000008-0a (21); the rest of the walk sweep 0b-4b (23+13), mp_core_live
included; walkreal_07000001-1b (5+6+3, realtime input walks). Not run: nf-view's remaining
portal/cutscene/probe tails (no plausible coupling to the two sites) and nf-audio (untouched);
nf-formats lib 64/64. Run the fast runner on the merged tree before merging per the standing rule.

## What's left / [S] notes
- **[S] steer rebuild structural factors**: the game's dir row carries `sinf(π/2)` = 0x3f800001
  (from RotMatrixZYX's rz=0 lanes) and the `RotMatrix`/`MulMatrix0` c²=0x3f800002 diagonal even at
  zero roll, and it is **not** re-normalized; the port's rebuild absorbs these (≤2 ulp per
  component, then normalizes). Also [S]: the game's `pitch' = pitch_delta − pitch` plus the matrix
  row's −sin(rx) means the game's dir-y is sin(pitch + input·K) for the raw slot-5 input; the port
  keeps its own (validated) caller convention. And the game's roll matrix mixes into the dir (the
  port's bank feeds only the HUD) — a pre-existing semantic choice, unchanged here.
- **[S] AA2Q bridge**: AA2Q is the standard quat and Quat_QuatToMat the column-form transpose; the
  port hands −θ. The game's own composition (q_yaw ⊗ q_pitch ⊗ q_bone, inlined MMI) vs the port's
  matrix-multiply composition remains a semantic equivalence, not bit-verified.

## Live checks for the main agent

- PCSX2, 05 (Air Combat) with the AT-420 guided camera: fire, steer hard up/down/left/right —
  expected: the missile still tracks the look input exactly as on master (the steering feel must not
  flip; the pitch sign convention note above is the thing to confirm).
- PCSX2, any level with turrets/drones aiming (e.g. 07000005 guard drones): watch head/aim poses at
  close range — expected: no visible change vs master (the swap is ulp-level), no head detachment.
- nf-view `--bond` spots: 05 near a sentry drone at ~8 units, yaw-only and pitch-only aim; compare
  against the master build screenshots — the head/aim orientation should be indistinguishable.
