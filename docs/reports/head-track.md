# head-track — guards' heads turn the right way (the verification round)

Branch `s/head-track` (origin/master aa3bc1f5 + this task's commits). Playtests 1013-1758 and
1015-0046: "how all the models are looking kind of weird, where their heads are turning kind of
the wrong direction, wrong angle" and "how weird their head looks when they're turning".

## What was done

**The frame-stepped headless trace** (`crates/nf-view/tests/head_track_trace_live.rs`, new, green
4.3 s): drives the REAL input path (`Sim::step_bond`, no state pokes — Bond staged by placement)
through three poses of a tracked drone on 07000007 — beside it on side A (`cross(up, fwd)` off its
yaw), side B, and above (the fix_heads07 F12 balcony) — and asserts **every tick's stored head
yaw/pitch against the game's formula transcribed independently inside the test**:

- the ease: `Ang_RotateToBearing` 0x1e0f10, re-read from the decompile for this task (wrap both
  angles to [0, 2π), wrap the diff to [−π, π], clamp the step to ±max, move `cur + clamped·rate`,
  snap within 1e-4);
- the rates: 0.14 alerted (`DCVars+0x50c > 0.66`) else 0.07 [`NDrone2_DoHeadTracking` 0x1472c8,
  disasm-checked];
- the range gate: track only within 3.0 (`AnimObjectSetHeadTracking` 0x1c6748 stores
  0x40400000² = 9.0; `AnimObjectHeadTrack` 0x1bf618:191 skips both angle blocks beyond it, leaving
  the wants at 0) — the trace's ease-back phase asserts the stored angles decay to 0 beyond it;
- the clamps: yaw ±1.1 with the |yaw| > 2.6 → 0 rule, pitch [−1.0, +0.6] in the game's
  positive-down convention = [−0.6, +1.0] in the port's stored positive-up (0x1bf618:290-307) —
  the trace asserts the stored yaw never passes 1.101 in any pose;
- the target choice: the port's `los || seen` gate (the game's `DoHeadTrackingBehaviour`
  0x147578 +0x228 & 8 opponent path vs the fellow-guard scan) — the traced subject is an unarmed
  party guest (kind 0xD), since armed guards in the aim states leave the track branch by design
  (`AnimObjectHeadTrack` 0x1bf618:137-139 clears the 0x10 request under the 0x180 aim flags).

**The measurement re-verified against the disassembly** (the decompile's helper calls have
ambiguous argument order; the C is incomplete exactly where it matters):

- `Vec_UnitVector` 0x1e2d28 = unit(arg3 − arg2): called 0x1bf728 with (out, bone2 pos
  `param_4+0xb0`, head pos `+0x30`) → **neck = unit(head − bone2), pointing up**;
- `Vec_CrossNormalise` 0x1e3078 = unit(cross(arg1, arg2)) → arg3: called 0x1bf738 with
  (neck, head row2) → **side = unit(neck × fwd)** — the port's convention;
- the yaw sign via `Vec_ScalarTripleProduct`(fwd, to_target, neck) [0x1bfa48], the pitch sign via
  the side triple, the per-plane 0.25 gates, the pitch behind-skip, the one-plane-passed coupling
  (0x1bf618:185-289) — all match the port's `head_track_want` line for line;
- the ease sites: stored angles at sAnimObject+0x80/+0x88, the rate at +0x70 (patched by
  DoHeadTracking every call), `Ang_RotateToBearing` ×2 [0x1bfb64/0x1bfb88], the +0x96 start
  freeze (the stored angles held 6..20 frames at track start, SetHeadTracking's Rand(15)+6).

## Result — no convention flip found in the measured path

The trace: **side-b ran 90/90 tracked ticks with every stored angle on the game's ease path**
(photographed at tick 30/60/75: |yaw| 1.019/1.091/1.099 → the ±1.1 clamp engaged;
fwd→bond 0.394/0.333 rad — the head's world forward turns TOWARD Bond from 1.3 rad away;
upright ≥ 0.988 — no roll, the 1013-1758 symptom). Side-a exercised the no-target branch's
formula for its full trace; the away phase exercised the ease-back to 0. No left/right flip, no
axis-order flip, no rate or clamp error reproduces in the measured path — the port's head-track
matches the game's formula everywhere the trace reaches. **No production code change was needed or
made** (the two playtests likely ran builds predating the 2026-10-14/15 fixes: fix-head-look
merged in the 10-15 night batch, exact-sine-sites-2 at 06:57 MST 10-15 — 1015-0046 ran ~00:46 UTC
10-15, before both).

## Findings (documented, not "fixed" — each would shift the shared RNG or is cosmetic)

1. The game's `RotateToBearing` returns [0, 2π); the port stores the signed equivalent — equal
   mod 2π, and the palette consumes sin/cos, so the posed result is identical (pinned by the
   mod-2π comparison in the test).
2. The port lacks the game's +0x96 start freeze (the stored angles held for Rand(15)+6 frames at
   every (re)target) AND the Rand(15) draw `SetHeadTracking` makes on every retarget — an
   RNG-stream divergence vs the game. Porting it lands draws in the shared generator and re-pins
   seeds per the 0072 procedure; left to a dedicated task.
3. The port's aim/stop branches ease the head to 0 at a fixed 0.07; the game uses the stored
   alert-aware rate (0.14 alerted, sAnimObject+0x70 patched by DoHeadTracking's stop path
   0x1472c8). Minor feel difference when alerted; unported for the same RNG-freeze reason.
4. The composition (`q_yaw ⊗ q_pitch` premultiplied, rotation rows only) is 101c/fix-heads-07's
   and was NOT independently re-derived this session — see the live checks below.

## Verification

- `head_track_trace_live` 1/1 (NF_ISO, release); the touched crate's new test compiled and ran.
- The background fast runner on the base tree: **1779 passed / 6 failed / 72 ignored (42 min)** —
  the 3 `disc_golden` are the documented slim-ISO allowance; `probe_pylon_render` +
  `fix_0c_pylon_live::the_0c_pylon_fills_the_view_from_the_closeup_spot` +
  `the_0c_pylon_is_on_screen_at_the_user_spot` fail on **master's own code** (this task added no
  lib code — only the test file and docs), reported as a pre-existing master red for the driver.
- This task changed: `crates/nf-view/tests/head_track_trace_live.rs` (new), `docs/WORKLOG.md`
  (appended), this report. No production code.

## What's left

- The above-pose (balcony) phase skipped honestly this run: no guest wandered within the 3.0
  feet-range of any of the three fix_heads07 balcony spots in 900 ticks (the playtest run's
  saturated trackers arrived over ~25 s of a DIFFERENT bond path — the arrival depends on the
  guest wander, which the staged run perturbs). The pitch up-clamp (+1.0 port-side) is still
  pinned by `fix_heads07_live`'s measured head_pitch +1.000 on the same tree.
- The side-a placement resolved to a drone 2.42 away after the spawner splice (the trace honestly
  followed that drone's no-target branch); the staging loop re-resolves but the splice lands the
  same way each retry on this load.

## Live checks for the main agent

1. **The composition order (the one unverified piece).** In PCSX2, at the 07000007 balcony F12
   spot (a guest saturating at yaw ±1.1, pitch +1.0), watch the head bone's world matrix
   (gdbwatch on the drone's palette entry, bone 5 rows) for ~60 frames while it turns, and dump
   the sAnimObject +0x80/+0x88 stored angles per frame. Compute offline both candidate
   compositions — R(neck,yaw)·R(side,pitch)·M_anim (premultiplied, the port's) vs
   M_anim·R(side,pitch)·R(neck,yaw) (postmultiplied) — and compare which matches the recorded
   matrix. The port's premultiply is 101c's derivation; this settles it from the machine.
2. **The 1015-0046 build check**: confirm the playtest build's commit hash (the exe rebuilds are
   logged in merge-log) — if it predates the 10-15 night merges, the "wrong direction" report is
   the pre-fix code and this verification closes it.
3. **The pre-existing pylon reds**: `probe_pylon_render` and the two `fix_0c_pylon_live` tests
   fail on master (this tree's suite) — route to whoever owns the 0c pylon work.
