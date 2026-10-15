# fix-exact-sine-flip — the 07000001 flip does not exist on master; the exact-math core is in and verified, the remaining host-trig sites are classified (G-046)

Branch `s/fix-exact-sine-flip`, task 110a. Sandbox 08:34–10:33 UTC 2026-10-15.

## What's done

**The task's premise is stale, and I verified that on the current tree rather than assuming it.** The brief
(read G-017/G-024, root-cause the 07000001 escort damage flip, then switch the exact math in) was already
executed by three merged tasks:

- **fix-exact-math-swap** (284, 2026-10-13) — the root-cause this task asked for: side-by-side replay of the
  01-doors trace, both models; first divergence tick 28 (Bond's y, one ulp — the exact RotMatrix bakes the
  placed walkway's collision a few ulps higher, `world.floor_below` under Bond's t27/t28 spot returns
  5.3394027 = 0x40aadc63 exact vs 5.3394012 = 0x40aadc60 host, and `Player_StandAtNewPosition`'s floor snap
  rides it); first AI-visible divergence tick 835 (drone 13's x, one ulp). Both old repro variants (the sine
  and the palette) flip for the same reason — ulp-wide geometry shifts arriving into ulp-wide fight margins.
  The game's own math is the exact path (PS2Sinf 40/40 recorded calls; Quat_QuatToMat 40/40; VU0 palette
  world-0.0 on the EE snapshots), so the swap went in.
- **fix-exact-math-swap2** (284b, 2026-10-14) — the swap re-landed on the then-current master; the escort
  walk passed 2/2 (the old repro's red state predated fix-08-collision2's cel relink, which repaired it);
  plus the real grenade-bounce 0.04-normal-offset fix.
- **fix-exact-math-swap3** (2026-10-14) — the held merge's "render shift" disproven (pixel-identical, ≤5e-6
  numeric), the bit-exact camera test added (`fix_camera_exact_math`: passes on the exact math, fails on
  host math), merged at **1098/0** (docs/merge-log.md, 2026-10-14).

Current master (`dc44c0e4` at my branch point, merged forward to `56d6dc24` during this session) runs
`placement::object_matrix` via the bit-exact `anim::sinf` + EE-FPU products/sums [M
crates/nf-formats/src/placement.rs:65-87], `anim::quat_trans_to_mat` on the EE FPU [M anim.rs:175-197],
`anim::mul` as psiBuildMatrixPalette 0x111498 on the VU0 [M anim.rs:199-208].

**The symptom does not reproduce — what I tried, freshly, on this tree** (not just citing the reports):

- `walkreal_07000001_live` (the escort walk through `nf_view::load` + `Sim`): **2/2 pass** on master @
  dc44c0e4 (125.2 s) and again on the final merged tree (127.9 s) — the escort finishes, no guard-5-early
  death, no stall. Only the documented pre-existing AND-touch note prints (walkreal-a.md).
- `diff_calls`: **4/4 pass** — PS2Sinf 40/40 bit-exact, Quat_QuatToMat 40/40, Rand_Rand 40/40, plus the
  sisters (the host sine differs on 24/40 recorded calls; the no-guard FPU polynomial on 11/40).
- `fix_camera_exact_math`: **1/1 pass** — the tick-170 camera matrix equals the decompile's arithmetic
  bit-for-bit (this test FAILS on host math, so its passing is affirmative evidence the exact path is live).
- `exact_vs_host_probe`: **pass** — the per-function host-vs-exact divergence is ≤1.1e-6 over wide domains.
- `ai_live`, `rng_live`, `ai_traces_live`, `doors_live`, `door_research_07000001`: **all pass** (chunk
  totals below).

So there was nothing left to fix at the G-017/G-024 call sites; the flip they describe cannot exist on a
tree that already runs the exact math and passes the walk. I closed the two GAPS rows with the t28/t835
mechanism and the merge pointer.

**The genuine remaining work — "everywhere the game uses it" — is now classified (GAPS G-046).** A
read-only survey (research subagent, every remaining platform-trig site in `crates/nf-game/src` checked
against `work/decomp/ACTION.ELF`):

- **GAME_EXACT** (the game demonstrably runs PS2Sinf/PS2Sinf2/PS2Sinf3 there via the cited decomp lines;
  each needs its own fail-first NF_ISO test when swapped): bullet.rs:57 spread (Bullet_init 0x12c5a0:171 →
  Vec_Spherical_2_Cartesian 0x1e3948 → PS2Sinf2 — bullet direction feeds every hit/damage roll);
  player.rs:1030 movement axes (View_RotTransMatrix 0x1e5168:33-36 / RotMatrixX 0x1e46a0); player.rs:431
  ladder grab snap (Player_CollLadder 0x196118:33-40, direct PS2Sinf); player.rs:827 + creepwall.rs:34 creep
  (Player_Creep 0x1969d0:122-125 direct PS2Sinf; Vec_Spherical_2_Cartesian); player.rs:633/670 wire hand
  (Player_CollWire 0x197130 / Player_Wire 0x197658, direct PS2Sinf2, 0.0542 sway); drone.rs:5325-5326
  missile steering (Bullet_update 0x12e7b0 → RotMatrixZYX 0x1e25a8 / RotMatrix 0x1e2368 — and the port's
  `asin(clamped y)` must become the game's `atan2f(y, √(x²+z²))` from vecutil_cartesian_to_spherical_acc
  0x1e3a70:10, a real divergence beyond the sin/cos swap); world spheres player.rs:270 / wire.rs:60,206 /
  ladder.rs:44 / player.rs:1490 (Control_BuildWorldSph 0x133070 → Control_GetObjMatrix 0x134210 →
  View_RotTransMatrix); pickup.rs:248 spin (Pickup_Update 0x18e148:193-201 → RotMatrix); drone.rs:926,
  2413-2420 head-track/aim pose (AnimObjectHeadTrack 0x1bf618:323-328 → Quat_AxisAngleToQuat 0x1e3f20).
- **PORT_ONLY stand-ins** (the game uses NO trig there — port-bug review candidates, NOT math swaps):
  ladder.rs:151 mesh columns (game: Collide_Update 0x1ea398 → Collide_Intersect 0x1ebaa8 cel refinement),
  player.rs:858 crawl advance (game: displacement carried in anim scripts 0x600046d/46f/471/475),
  casing.rs:80-81 eject matrix (game: AnimGetBoneWorldTrans), drone.rs:2495 object-space head-track
  (game: world-space plane projection + Ang_RotateToBearing 0x1e0f10, no sin/cos, no atan).
- **UNCLEAR**: anim.rs(nf-game):403-411 root_yaw/unyaw (no game counterpart found; the game's atan class
  ATAN2_APPROX 0x1e4528 vs libm atan2f unresolved for that path); player.rs:657 wire root-motion composition.
- Context: `Vec_Spherical_2_Cartesian` 0x1e3948:22-23 and RotMatrixX/Y/Z call PS2Sinf2; RotMatrix
  0x1e2368:28-31, RotMatrixZYX, View_RotTransMatrix, Quat_AxisAngleToQuat 0x1e3f20:27 call PS2Sinf3;
  `vecutil_cartesian_to_spherical_acc` 0x1e3a70:10-13 uses libm atan2f. There is no PS2Cosf — cosine is
  always PS2Sinf(θ+1.5707964).

I did NOT swap those sites in this session: each needs its own failing-first test and a full-suite pass,
and the sandbox reaps background jobs (details below), so a safe per-site cycle did not fit the window.
G-046 carries the full evidence so the next task can execute them one by one.

## How it was verified

Suite chunks with the fast runner (`tools/test/suite.py --only …`, per-binary 600 s limit,
`--test-threads=1`, NF_ISO, NF_SILENT). This sandbox kills background jobs between tool calls — the task
message's `nohup … &` pattern AND a `setsid`-detached run both died silently (suite.out stayed 0 bytes;
swap2's report hit the same wall) — so the chunks ran in the foreground:

- Pre-merge (master @ dc44c0e4 + my docs commits): chunk 1 (diff_calls, ai_live, rng_live, trig_live,
  drone_portcheck_live, exact_vs_host_probe, tuning_live) **50 passed / 0 failed**; chunk 2
  (walkreal_07000001_live, fix_camera_exact_math, camera_matrix_dump, ai_traces_live,
  door_research_07000001, doors_live) **13 passed / 0 failed**.
- Final tree (merged with origin/master 56d6dc24 — the casing-datum/mp-core/12-enemies merges; chunk 3
  included the rebuild): chunk 3 (diff_calls, walkreal_07000001_live, fix_camera_exact_math,
  exact_vs_host_probe) **10 passed / 0 failed**; chunk 4 (fix_ladder_live, fix_ladder2_live, fix_wire2_live,
  wire_live, fix_sg5_casings_live, casings16_live, grenade_live, explode_live, stun_grenade_live,
  fix_bond_damage_live) **27 passed / 0 failed / 3 ignored**; chunk 5 (ai_live, rng_live, doors_01_live,
  pickup_live, searchlight_live, weapon_switch_live, ledge_live, ai_traces_live) **50 passed / 0 failed**.
- **Final-tree total: 87 passed / 0 failed / 3 ignored over 22 binaries** (per-binary lines in suite.log;
  the 3 ignored are the standard skips). Plus 63 pre-merge passes. The binaries cover every consumer
  family of the three swapped functions (placement matrices → collision/walk; palette → AI traces, escort,
  camera; sine → trig/diff units), the seed-pinned AI families, and the G-046-adjacent movement/casing
  systems. The one documented exception: `weapon_matrix_live`'s >600 s suite-gate TIMEOUT (pre-existing on
  the combined master per fix-movies-tests/fix-armour-ring reports) was not re-run.
- The full four-crate end-to-end run was not achievable in this sandbox (background reaping + the 10-minute
  tool cap); the chunked pattern is swap2/swap3's sanctioned approach.

## Shared files touched

None (docs only: docs/GAPS.md rows G-017/G-024 closed + G-046 filed, docs/WORKLOG.md appends). No source
file changed, so no re-pin, no seed change, no RNG draws.

## Live checks for the main agent

1. (carried over from fix-exact-math-swap, still outstanding) PCSX2, 07000001 at the 01-doors trace's start
   (Bond ≈ (−4.92, 6.34, −0.09), frame 43445): the floor query under Bond
   (segment (−5.161, y+0.5, −0.684) → y−1.2) should read **≈ 5.3394027 (0x40aadc63)** — the exact path's
   value, confirming the walkway instance's matrix is the game's RotMatrix output.
2. None new: this session's changes are docs-only; there is nothing to render or poke.

## What's left

- **G-046's GAME_EXACT switches**, one at a time, each with its own fail-first NF_ISO test and a full-suite
  pass (highest value: bullet.rs:57 spread → hit/damage margins; player.rs:1030 movement axes; the missile
  `asin` → `atan2f(y, √(x²+z²))` at drone.rs:5325). The PORT_ONLY sites are separate port-bug-review work,
  not math swaps.
- `Check_Target` / `Player_AutoAim` diff_calls adapters (verify-bitexact.md's item 2) — the autoaim cone's
  host atan2/asin classification rides on it.

## ntfy

Sent via curl (first attempt, 09:45 UTC 2026-10-15) — delivery confirmed by the server response
(id AdWS0Ps8Z6yd, topic nightfire-yoda-8k3x). No fallback needed.
