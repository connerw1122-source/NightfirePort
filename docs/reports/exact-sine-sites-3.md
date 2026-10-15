# exact-sine-sites-3 — the four player-motion trig sites run the game's exact sine (G-046, third group)

Branch `s/exact-sine-sites-3` (from origin/master d38e945d). Session 11:52–13:51 UTC 2026-10-15.

## What's done

All four G-046 third-group sites swapped from host trig to the game's exact math
(`trig::sinf`/`trig::cosf` = bit-exact `PS2Sinf`/`PS2Sinf2`/`PS2Sinf3`, the EE FPU chop/noguard
helpers of `nf_formats::ps2float`), each matching the decompile's arithmetic order first-hand:

1. **Movement axes** (`player.rs`, `Player_SSWalk`'s per-tick move): the game builds the object matrix
   with `RotMatrix` 0x1e2368 / `View_RotTransMatrix` 0x1e5168:33-36 — lane pair
   `PS2Sinf3(yaw + π/2, yaw, …)`, so the port now takes `sinf(yaw)` / `sinf(yaw + π/2)` (`trig::cosf`);
   the swim look-dir is `RotMatrixX` 0x1e46a0's `PS2Sinf2` (same two sines). The adds/muls after the
   sines stay the port's — the game's per-tick `pos += R·local` runs in VU code outside these
   functions (`Player_Move` 0x191dd0 only writes the local speeds; the matrix transform happens in the
   control pass), so there is no decompile order to match beyond the sines.
2. **Ladder grab snap** (`player.rs`, `Player_CollLadder` 0x196118:32-40): the game adds π to the
   LADDER's yaw on the EE FPU and SUBTRACTS: `pos = centre − (PS2Sinf(yaw+π), PS2Sinf(yaw+π+π/2))·0.5`.
   The port's old `centre + (sin yaw, cos yaw)·0.5` was algebraically equal but not bit-equal.
3. **Creep snap** (`player.rs`, `Player_Creep` 0x1969d0:118-125) + **creepwall endpoints**
   (`creepwall.rs:34`, `Vec_Spherical_2_Cartesian` 0x1e3948:22-26): the snap is
   `closest + (PS2Sinf(yaw+π), PS2Sinf(yaw+π+π/2))·0.275` with the yaw add on the EE FPU; the
   endpoints are the S2C pair with θ = `fadd(yaw, π/2)`: `d.x = (len·sin θ)·cos φ`,
   `d.z = (len·cos θ)·cos φ`, cos θ = `sinf(θ + π/2)` (a SECOND noguard add) and the game's own
   `sinf(π/2)` as the explicit cos-φ factor, the ± endpoints EE FPU adds/subs.
4. **Wire hand** (`player.rs`): the grab reach (`Player_CollWire` 0x1975b0-0x197618) is
   `PS2Sinf2(−yaw + π/2, −yaw)` → (cf, sf), `wx = dx·cf − dz·sf`, `wz = dz·cf + dx·sf` on the EE FPU,
   then the VU `pos −= (wx, dy, wz)` (guard-bit subs — `pf::add`); the sway arms
   (`Player_Wire` 0x197658) are `pos = closest − (cf, sf)·(±0.0542)`, the port's `sway_sign` mapping to
   the game's multiplier as `m = −sway_sign·0.0542` (verified equal to the old host form's signs),
   `pos.y = closest.y − hang` the same FPU sub.

`trig.rs` (shared file, small additive hooks): `add`/`sub` are now `pub`, plus the
`HOST_TRIG: AtomicBool` switch (default false = the exact math; only the side-by-side walk test sets
it) — listed here per the shared-files rule. No other file touched: `look()`/`right()` (player.rs:270)
are the world-spheres group's sites, left for that task; the wire root-motion composition (player.rs:657)
stays as G-046's UNCLEAR item.

## How it was verified

- **Fail-first shown on the pre-swap tree** (the site changes stashed, tests + trig hooks kept):
  `exact_sine_sites3_live` ran **3 failed / 3 passed** — the wire reach differed by 1 ulp at wire yaw
  −3.2 (`1092456848` vs `1092456847`), the creepwall endpoints bit-test failed, and the walk's
  live-check ("the two runs must differ somewhere") failed with 0/600 differing ticks. On the fixed
  tree: **6/6 pass** (7.8 s + the level loads).
- **The fail-first basis, quantified** (`site_formulas_where_host_and_exact_diverge`, 1500-yaw sweep):
  every site's host-vs-exact formula pair diverges — the sensitivity counts and max |Δ| print in the
  test output (movement sin/cos, ladder snap, creep snap, wire sway, wire reach, creepwall endpoints
  each have yaw ranges where host and exact disagree at the bit level).
- **Bit-for-bit through the real path**: 64 synthetic-yaw walk-in ladder grabs (`walk_tick` →
  `Player_CollLadder` on the real 07000002 world), 64 synthetic-yaw wire grab reaches (`grab_wire`),
  40 synthetic-yaw idle sway adds (the real wire state machine + the disc's anim tracks) plus the disc
  cable's own 5ec arm through the real jump-grab, the vent creep snap + endpoints on 07000005's real
  wall — every one asserts the decompile formula bit-for-bit.
- **The 600-tick side-by-side walk** (07000005 through `Sim::step_bond`, fwd 1.0 turn 0.003, exact vs
  `HOST_TRIG` runs): **203/600 ticks differ; tick-120 gap 0; end-to-end 4.8e-3; max gap 8.2e-2** —
  the swap's direct effect is ≤ 1 ulp per sine evaluation and stays ≤ 1e-5 while the path is open; the
  gap grows only when the path brushes the spawn courtyard's gate (the floor-snap/wall feedback
  amplifying ulps — the fix-exact-math-swap t28/t835 mechanism) and partially re-converges at the
  wall. The test pins the 120-tick horizon at 1e-3 (measured 0) and the imperceptibility of the full
  walk at 1e-2 (measured 4.8e-3 = five millimetres after ten seconds); the largest divergence is
  reported here as required.
- **Suite chunks** (the fast runner, foreground — this sandbox kills background jobs between tool
  calls, as fix-exact-sine-flip already recorded):
  - chunk A (diff_calls, walkreal_07000001/02/05_live, fix_camera_exact_math, ai_traces_live):
    **11 passed / 1 failed / 2 ignored**. The one failure,
    `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` (the escort contact
    settles at (−18.8, 6.3, 14.3) instead of the plate), is **pre-existing on master, not this
    branch's**: re-running the same test with this task's source changes stashed fails identically at
    the same spot (the rebuild is proven by the fail-first runs in the same stashed state). Some merge
    since the morning's all-green run (09:16–10:42, fix-start-loadouts's session) broke it; it needs
    its owner.
  - chunk B (exact_sine_sites3_live, fix_ladder_live, fix_ladder2_live, wire_live, fix_wire2_live,
    fix_swim_vent5_live, diff_calls, trig_live): **25 passed / 0 failed / 3 ignored**.
  - The full four-crate run did not fit the window (the 329-binary build alone is ~5 min; both chunks
    plus the pre-swap verification consumed it) — the standard allowed-set note applies: only the
    3 `disc_golden` tests may fail on the slim ISO, and the pre-existing walkreal_01 failure above is
    documented for its owner.

## Shared files touched

- `crates/nf-game/src/player.rs` (the task's four sites), `crates/nf-game/src/creepwall.rs` (endpoints),
  `crates/nf-game/src/trig.rs` (additive: `pub add/sub`, the `HOST_TRIG` switch),
  `crates/nf-view/tests/exact_sine_sites3_live.rs` (new), `docs/GAPS.md` (G-046 update), `docs/WORKLOG.md`
  (appends). No RNG draws, no seed changes, no re-pins.

## Live checks for the main agent

1. PCSX2, 07000002, the bunker exit ladder: walk into the ladder face and pause on the grab. Bond's
   xz should read `centre − (sin(yaw+π), sin(yaw+π+π/2))·0.5` — for the bunker ladder's yaw the exact
   and host values agree to 1 ulp, so this is a ≤ 1 ulp check (a `gdbwatch.py` watch on BL+0x30/+0x38
   across the grab tick, old vs new PCSX2 build not needed — the game IS the exact path; this just
   confirms the port matches).
2. nf-view `--bond` spots (render checks, no visual change expected — ulp-level):
   - `--bond -23.58,34.0,2.65,0,0` in 07000005, jump at the cable: the hang position and the first
     idle sway (±0.0542 along the perpendicular) should be pixel-identical to master except ≤ 1 ulp
     of camera jitter.
   - `--bond 104.95,25.12,108.83,1.5,0` in 07000005 walking forward: the creep vent and the ladder
     snaps unchanged; the walk path may drift ~mm over long walks (the exact sine is the game's).
3. The escort walk break (pre-existing, needs its owner): 07000001, `walkreal_07000001_live`'s route —
   the contact stops at (−18.8, 6.3, 14.3), route_idx Some((25, 37)), armed 6.7, bond d 9.3.

## What's left

- `walkreal_07000001_live`'s pre-existing break (not this task's — bisecting master's day of merges
  would have eaten the window; the failing state reproduces on d38e945d with this task's changes
  stashed).
- G-046's remaining groups (bullet.rs spread, the drone missile `asin`→`atan2f`, the world spheres
  incl. player.rs:270 look()/right(), pickup.rs spin, drone head-track) and the PORT_ONLY review
  candidates — the other sessions' scope.

## ntfy

FAILED — all three fallbacks. curl (13:50 UTC): `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`. python urllib retry (clean ASCII payload): `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`. wget `--post-data` printed nothing and the pipeline's exit status read 0 (through `tail`, so wget's own status is unresolved — the server-side 429 on the other two attempts says the quota was already spent, so this attempt most likely did not deliver either).
