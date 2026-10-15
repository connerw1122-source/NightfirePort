# fix-driver-air — report

Branch `s/fix-driver-air` (the task card's bottom said `s/driver-air`; the header and rule 1 say
`s/fix-driver-air` — that is the branch pushed here, cut from `origin/master` d2ce47e5). The task:
the 07000006 friendly truck driver (skin 0x05000035, dtype 13 → dtypenew 0x17, team 3) at the level
start — playtest 1014-1725: "he's sitting in the air, why didnt you test this."

## What the game does (the evidence chain)

**The sit is the level's own start script.** The driver's placement param 1 = start script 0600011d,
the 880-frame sit-in-truck one-shot [D nf-extract script-info; M 0x160bd0 case 1 plays it from
TruckDriverInit 0x32 and holds the state]. Its windows: body 04000064 (222 ch) + face 04000065
(22 ch, flags 0x8), both 1..880 [D]. His seat is the crate `Object_Crates_Seat` beside the wine
truck (collision AABB hi y 38.9987 [D the level's collision]; the truck itself spans x 59.06–65.39).

**The game's numbers, from its own trace** [L `tests/traces/05-truck.trace`, the 07000006 segment
frames 37174..37878, drone 13, state 0x32 (50) the whole window]: x holds 66.118 (span 0.024), z
creeps 28.872 → 28.877, and **y settles 39.305 → 39.19 → 39.2** — the crate top **+ 0.2**. The decay
shape is exponential (39.305 → 39.289 → 39.241 → 39.226 → 39.210 → 39.194 → 39.193, then creeping to
39.2) — the game's SOFT settle, not a snap:

- `Drone_FeetOnPoint` 0x140660: the down-ray `pos → pos − up·(drone+0xA0 + 0.6)`, mask 0xC; the
  correction out-param = `(obj.y − drone+0xA0) − hit.y`; on-surface when the correction < 0.2
  (anim+0x60 & 8) [M].
- `Drone_CollisionHandler` 0x140968 tail: on-surface → velocity zero and **`obj.y -= correction ×
  0.125`** per tick, gated `correction > −0.1` [M] — converging to `floor + hip`. The trace's decay
  matches ×0.125 exactly (39.305 − 0.105×0.125 = 39.292 ≈ the trace's 39.289).
- The hip `Drone+0xA0` is ANIMATED: `DroneAnim_PreTransCallback` 0x13b490:58-67 rewrites it each
  resolve as `clamp(animObj+0x5C − 0.02, 0.2)` (+0x5C = the resolve's root height + `sAnimObject+0x60`
  [M 0x1c35f8:374-378]; DefaultInit 0x14b300:48 starts it at 1.0). The sit lands on the **0.2 clamp
  floor** [L the trace's crate+0.2].
- The object position IS the hip point: the palette draws the root bone AT obj+0x30 [M
  docs/formats/animation.md "psiBuildMatrixPalette then places the root bone at the object matrix"].

**The cower clamps too** [L `tests/traces/06-searchlight.trace` frame 142242ff: the driver in state
0x13 (CivilianHiding), y 39.196 constant for thousands of frames = crate + 0.196] — while the cower
anim 04000048's root ch1 is 0.5661 [D probe] — so the clamp there is NOT the raw root either.

**The guards track their anim roots** [L the same 05-truck segment: the stairs guard at (68.890,
27.334) rises y 39.003 (state 1) → 39.155 (state 3) as his anim changes — the gap = the root]. So
the game's hip rule differs between the guard anims (the root passes through) and the sit/cower
(the 0.2 clamp); the `+0x60` addend that reconciles them is UNREAD — see the residuals.

## The pre-fix port (the symptom)

`hip()` was hard 1.0 and the collision snapped `pos.y = floor + 1.0`: the driver sat at 40.0 (0.8
above the game's 39.2), the sit pose hanging from there — the pelvis at 40.007 (the draw maps the
pose root 1.0266 → `pos.y − 1.0 + 1.007`). The user's "sitting in the air". The probe (master
d2ce47e5): pos.y 39.999 the whole sit; the palette span 39.205..40.753.

## The fix (`crates/nf-game/src/drone.rs`, the only source file)

- `TruckDriverInit` (0x160bd0 case 1) sets **hip_anim = 0.2** when it plays the start script [L the
  trace's crate+0.2; M the clamp 0x13b490:64-66], restored to 1.0 on the exits (ANIM_DONE → 0x34,
  ALARM → 0x33) [M 0x160bd0 case 5 / 0x11-0x1e arm].
- `hip()` returns the new `hip_anim` field (own line in both constructors — merge lesson 4) instead
  of the hard 1.0; `Drone_FeetOnPoint`'s ray length (+0xA0 + 0.6) and the on-surface test read it
  [M 0x140660].
- `Drone_CollisionHandler` runs the game's **soft settle** (`pos.y -= correction × 0.125` when
  correction > −0.1) instead of the snap; the correction ≤ −0.1 case keeps the snap fallback (the
  game's deep-correction path is unread — without the fallback a tunnelling start would sink) [M
  0x140968 tail].
- The unit test `drone_stands_on_the_floor` re-pinned to the settle (one tick = correction × 0.125,
  converging to floor + hip) [M the same tail].

**Deliberately reverted:** a wider fix that made the consumed root follow the newest script window
(face tracks included) reproduced the game's sit EXACTLY (x held 66.118, y 39.199, the drift gone)
but stalled the walking guards whose walk scripts also pair face tracks — the ai_traces 05-truck
segment shifted by a drone (a spawner chain stalled). The game's consumed-root rule must distinguish
the sit from the walks through the unread `+0x60`/resolve path; the port keeps the pre-fix window
selection (the body root for motion) and fixes the hip only. The sit's residual x/z drift (≤ 0.03
over the window here, the facial3 known gap) stays.

## The verification

`crates/nf-view/tests/fix_driver_air_live.rs` (new), through the real paths (`nf_view::load` +
`Sim::step_bond`, default `Input` — no internal setters):

- `the_sitting_driver_rests_on_his_seat_at_the_game_height` — the driver holds 0x32, pos.y lands at
  **39.2 ± 0.05** (the game's trace value) by tick 60 via the settle, the sit palette renders below
  40.8 (the pre-fix float's span was 39.2..40.75 with pos.y 40.0), x holds 66.118 ± 0.1 through the
  traced window, and the script's end steps into 0x34.
- `the_floor_settle_runs_at_the_games_one_eighth_rate` — a standing driver lifted 0.15 moves exactly
  correction × 0.125 on the first tick (the game's rate [M 0x140968 tail; L the trace's decay]).

**Fail-first (shown)**: with the pre-fix `drone.rs` (84f814b8) both tests fail — the sit at
`39.99867` (the exact symptom) and the settle snapping the full 0.15 (`39.15055 → 39.00055`).
Post-fix: 2/2 green.

Sisters re-pinned: `fix_06_driver_live` 2/2, `fix_drone3_live` 5/5, `facial3_live` 3/3,
`fix_truck_live` 2/2, `ai_traces_live` 4/4, nf-game lib 160/0 — all on the fixed tree. The wider
fast runner was restarted on the final tree near the deadline (the sandbox window closed before a
total line; the earlier run also died on a missing PATH) — **no full-suite total this session**;
next session: `tail -1 suite.out`. The touched suites above are the drone/anim blast radius.

## Shared files

`drone.rs` (the task's file) only, plus the two sister tests' re-pinned asserts
(`fix_06_driver_live.rs`, `fix_drone3_live.rs` — the "feet on the floor" asserts now go through the
animated hip, the same rule). No scene.rs/player.rs/lib.rs changes.

## Renders (for the main agent's GPU pass)

The change is visible. Headless `--shot`/`--bond` spots (frame ~300 of the sit, i.e. `--pull 300`):

- **A — the seat, before/after**: `--bond 62.0,40.6,36.5` yaw ≈ 3.6 (looking north-west at the
  crate): the driver should sit ON the crate top (his pelvis ≈ 39.2, the thighs forward), not float
  a standing hip above it. The pre-fix build drew his pelvis at 40.0 — visibly hovering.
- **B — from the truck cab**: `--bond 64.5,41.0,24.0` yaw ≈ 0.4: the driver past the truck's front,
  seated on the crate, the wine truck's cab on the left.
- **C — after the script (t > 880)**: the same spot at frame 1000: he stands on the crate at the
  standing height (pos.y = crate + 1.0), hands behind back.

## Residuals (open)

1. **The hip's per-anim rule** — the game's guard anims pass the root straight through (the y = the
   root: 39.003/39.155 [L]) while the sit/cower clamp to 0.2 [L both traces]; the reconciling
   `+0x60` addend (0x1c35f8:376, per AnimSeqGetInfo 0x1c79a0 a raw-seq field) is unread. The port
   pins the sit's 0.2 by state and keeps 1.0 elsewhere (the pre-fix behaviour).
2. **The cower's pos.y semantics** — the game's cowering driver y = crate + 0.196 [L]; the port's =
   crate + 1.0 (the cower hip stays the standing value). The RENDER matches (the draw maps the pose
   root: the pelvis lands at the crate top both ways — the 06-driver test's cower pins stay green),
   but the collision anchor is 0.8 high while cowering.
3. **The sit's x/z drift** — the body root's 0.57 tail run still reaches him (bounded ≤ 0.03 here;
   the game's span 0.024/0.063 [L]) — the facial3 known gap, the same consumed-root mechanism as (1).
4. **The driver's post-script walk** — the game's driver ends up at x ≈ 69-70 [L the main agent's
   doc] (the truck-bed chain/AssignAIPath(8) path walk, not ported); the port's stays on the crate.

## Live checks for the main agent

1. **The sit hip's mechanics (gdbwatch.py)** — watch `drone+0xA0` (hip), `+0xA4`, `+0xA8` and the
   anim object's `+0x5C`/`+0x60` for the driver (the 06 truck, from the load, ~950 frames), plus the
   same on a stairs guard: expected [L-derived] — the driver's +0xA0 ≈ 0.2 during the sit script and
   ≈ 1.13 after; the guard's +0xA0 tracks his anim changes (≈ 1.02 idle, ≈ 1.17 alert). The +0x5C/
   +0x60 pair during the sit vs the guard idle is THE open question (residual 1).
2. **The soft settle** — the same watch on obj+0x30.y right after the load: expect the ×0.125 decay
   (39.305 → 39.19 → 39.2 over ~8 ticks) [L already measured in 05-truck.trace — confirming live
   with the hip values visible].
3. **The cower** — walk armed Bond at the driver (after frame 900), watch obj+0x30.y + drone+0xA0 in
   state 0x13: expected y ≈ 39.196 with +0xA0 ≈ 0.2 [L 06-searchlight.trace] — and whether +0x5C is
   small there too (residual 2).
4. **The death-fails-mission check** (the ground-truth doc's ask): kill the driver in PCSX2 → the
   "Mission Failed" box at once; then in nf-view the same.
