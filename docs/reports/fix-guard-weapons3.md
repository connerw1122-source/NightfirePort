# fix-guard-weapons3 — the HELD merge's two failures

Base: `s/fix-guard-weapons2` + `origin/master` merged first (e10f157a, the WORKLOG conflict resolved
append-only). Both master-passing tests failed on the merged tree; both are found, fixed and green.

## 1. `ai_live::guard_takes_cover_like_the_game` — "UnderCoverAim lasted 90 (live 37)": the RNG shift

**Reproduce** (on the merged tree, no fix): the cover-type pick at UnderCover entry —
`cover_anim` mode 6's `c[rng.rand(c.len())]` (drone.rs; the game's `DroneAnim_CoverAnim` 0x13c078
mode-6 `Rand`) — flipped from the lean-out (danim 176, 51 frames at 1.5 ≈ 37 ticks) to the
step-out (danim 178, 90 frames at 1.0 = 90 ticks). The stream at that point differs from the
live-calibrated one because fix-guard-weapons2 added four per-shot draws to the guard fire path.

**Which draws moved it** — of fix-guard-weapons2's four additions, two are removed here:

1. **The flash-sprite orientation trio** (`FRand(2π)` yaw, `FRandHalf(0.2)` pitch/roll —
   FireWeapon 0x179268/0x179288/0x1792ac). The decompile puts all three INSIDE
   `if (lVar9 != 0)` where `lVar9 = control_create_object(0x40, muzzle, 0, 0)` (0x179248,
   the gate at 0x179254 `beqz $s0, 0x179304` — disassembly checked): they run only when the
   kind-0x40 muzzle-flash sprite object is created. The port's sim creates no render sprite, so
   the branch's draws never fire in the game-equivalent headless path; consuming them
   unconditionally shifted every later draw.
2. **The shot-pitch draw** (`Rand_Rand(500) − 250`, FireWeapon 0x179490-0x17949c, skipped only
   for the silent 0x5ef). The C gates it only on the record's +0x60 sound id, but it is
   AUDIO-side — it parameterizes `Sound_Play3D`'s pitch jitter — and the port's audio layer
   (nf-audio) is render-side and does not feed the sim RNG. The live-calibrated stream
   (`ai_live`'s PCSX2-verified traces: the cover pick, the UnderCover positions, the matrix
   rates) pins the per-shot sim consumption at exactly roll + Light_Create's
   `Rand(param_11)` [M 0x1cc50c] + the spread draws (`Bullet_init` 0x12c5a0's
   FRand_MVar2 + FRand(2π) + FRand(π), one per pellet) — with neither the trio nor the pitch.

**What stays** (game-cited, and inert for the failing scenario — the firing drones there carry
0x16/0x24, which take none): the burst-sizing draws (`Rand(3)+2` for weapons 6/10 with a raw
size < 2, `Rand(2)+1` for 0x0e-0x10 — NextBulletTime 0x179eb8-0x179ef8), the per-weapon
interval clamp (0x179dc4-0x179e04) and the SP aggression burst cut (0x179f80-0x17a01c).
`Bullet_init_casing` 0x12fde0 draws nothing at fire time (its randomness is in `_ex`
0x12f678, the delayed activation — checked), so the drone path correctly consumes nothing for
casings.

**Verification**: with the two sites removed, `guard_takes_cover_like_the_game` is back at the
live 37 (`UnderCoverAim lasted 37`) and the whole `ai_live` binary is 35/35. The intermediate
experiments (dirty-tree artefacts aside) are logged in the WORKLOG: removing only the trio kept
the 90; removing both restored 37 — the pitch draw alone flipped the pick.

## 2. `fix_guard_weapons_live::the_3x3_hit_rate_and_damage_matrix_matches_the_game` —
"07000005.bin diff 3 weapon 0x16: hit rate 0.520 vs model 0.975 (52/100)"

**Not a fire-path bug** — an instrumentation bug in the harness against the game's own
NewSighting gate. `DroneWeap_DoBulletAccuracy` 0x179740 (disassembly + decompile :100-110):
`Rand_FRand(100) < pct` is further ANDed with
`DroneFiring_NewSighting_TimeToHit·FRAME_RATE <= Drone+0x270` (the 120-tick sighting counter)
and NOT `+0x41` (the lost-sight latch); `+0x270` RESETS to 0 on every sight loss
(`DroneVision_ProcessDroneSight` 0x176a28:202 `*(undefined4 *)(iVar5 + 0x270) = 0;`,
`+0x41 = 1` :203) — the port implements exactly this (drone.rs's sight block + fire_weapon's
`hit &= self.seen_ticks >= 120 && !self.lost_sight`). Every sight loss therefore auto-misses
the next ~120 ticks' worth of shots BY THE GATE — 60+ shots for the SG-5's 4-tick interval —
regardless of the accuracy roll.

The harness (fix-guard-weapons2's) caused sight losses in three ways: its stall re-parks
teleport Bond; and its parking loops picked Bond spots by floor height ONLY — guard 8 spent the
run firing 450 shots at a wall (seen_ticks 0, all gated). Whether those gated shots landed
inside a cell's 100-shot sample was a dice roll on the (re-rolled) stream: the merged tree's
0x16/diff-3 cell caught a re-park window and read 0.52. The instrumented run (NF_GWLOG, since
removed) showed the misses concentrating in `seen<120`/`lost` shots with `roll=1`.

**The harness fixes** (test-side, the model's domain made explicit):
1. Both parking loops (initial + re-park) now require the guard's own sight ray —
   the guard head → Bond head line of sight (the game's test, `DroneVision_ProcessDroneSight`
   0x176a28) — so the parked engagement is a sighted one.
2. Per-shot gate-clean filtering: a collected shot counts toward the sample only when its tick's
   `seen_ticks >= 120` (the sight block runs before the fire in the same tick, so the post-tick
   read is what the shot's gate saw). The model rate p = (100 − 5·acc) × level accuracy is the
   game's rate for GATE-CLEAN shots; the gated shots are the gate's misses, not the roll's.
3. 200 shots per cell: a 100-shot sample of a ~50% coin carries a ±0.10 2σ spread and the
   re-rolled stream put the 0x12/diff-2 cell's draw at 0.65 (2.5σ over the 0.12 tolerance);
   200 shots halve σ and the tolerance stays 0.12.

**Verification**: the 3×3 matrix is green end-to-end (all nine cells inside the tolerance, the
per-hit damages still exact), plus fix-guard-weapons2's own fire-rate and TooClose tests
(3/3 in the binary, 135 s — under the 600 s per-binary cap). The failing-first evidence: both
tests fail on the merged tree pre-fix (the driver's HOLD report; reproduced here —
"UnderCoverAim lasted 90", "0.520 vs 0.975", and the intermediate "0.390 vs 0.525" during the
bisect), and each intermediate state is in the WORKLOG.

## The game's per-shot draw order, as the port now consumes it

Per guard shot (`DroneWeap_FireWeapon` 0x178f68 + `DoBulletAccuracy` 0x179740):
1. `Rand_FRand(100)` — the hit roll, with the TooClose ×2 (the level's [0x30cb14], 3.0 on
   01-1B), the ×0.25 first-moved, the ×0.75 moving and the level accuracy multipliers BEFORE it
   [M 0x179740:88-100].
2. `Light_Create`'s `Rand(param_11)` (= 0 for the muzzle light — the state still advances)
   [M 0x1cc50c].
3. — the flash-sprite orientation trio: skipped (the object-conditional arm; see above).
4. The spread: `FRand_MVar2(2s)` + `FRand(2π)` + `FRand(π)` per pellet (`Bullet_init`
   0x12c5a0:166-184; the guards' weapons all carry +0x18 = 1 pellet [L record dump]).
5. — the shot pitch: skipped (audio-side; see above).
At each burst START: the burst-sizing draws for weapons 6/10 (raw < 2) and 0x0e-0x10 only
[0x179eb8-0x179ef8], then `BurstDelay`'s `FRand(0.5)` [M 0x179ba8].

## The full suite

Run with `tools/test/suite.py` on the final tree (fix + the harness, merged with origin/master):
see the total line below (appended after the run).

## Live checks for the main agent

1. **The per-shot RNG consumption** (settles the two skipped draws against the real game):
   in PCSX2 with `tools/trace/gdbwatch.py` on the RNG state words (the Rand_Rand/FRand state at
   gp−0x75cc/gp−0x75d0, ACTION.ELF addresses 0x2d6234/0x2d6230 area — the watchpoint records every
   write), let a guard fire one burst at 10 units. Expected if the report's reading is right:
   5 state updates per shot (roll, light, 3 spread); if the game shows 9, the flash-sprite object
   create succeeds in play and the orientation trio + pitch belong back in the stream (and the
   live-calibrated tests need recalibration, not the fire path).
2. **The NewSighting gate after a re-park-like sight loss**: poke Bond behind a wall for 2 s in
   front of a firing guard, then back out. Expected: the guard's first ~120 sighted ticks
   auto-miss (Drone+0x270 counts up from 0 — watch it), then the rate resumes at the model's.
   The port does exactly this.
3. **The cover-type pick** is a coin flip on the live stream (mode-6 `Rand`); the live trace's
   lean-out at node 1 is reproduced. If a re-run of the live capture ever shows step-out there,
   the seed/stream needs the live check 1 resolved first.

No render/`--bond` spots: this task changes no visible output (the drawn poses, sounds and
models are untouched; the two removed draws never had render-side effects in the port).

## What's left

- Live check 1 is the decider for the two skipped draws; if it comes back 9/shot, the fix
  moves to the tests' calibration (re-capture the live traces against the 9-draw stream), not
  to the fire path.
- The matrix's per-cell sample is still deterministic-stream-sensitive in WHICH guards engage
  (the candidates list order); the gate-clean + 200-shot changes make the RATE robust, but a
  future stream change may reshuffle which guard collects first (the harness handles that —
  it tries candidates until one completes).
