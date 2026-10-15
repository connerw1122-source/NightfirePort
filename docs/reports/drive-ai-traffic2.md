# Report: drive-ai-traffic2 (branch `s/drive-ai-traffic2`)

Task 507b — land the held 507 (`s/drive-ai-traffic`, the driving AI cars) on top of merged 506
(`s/drive-physics`): fix the module-layout collision, wire the AI onto 506's physics model, get
both crates' tests green. **No drawing** — data/state/flow only, headless.

## The layout chosen

`crates/nf-game/src/driving/` is now ONE module with four files:

- `physics.rs` — 506's `driving.rs` moved intact (`git mv`, zero content changes).
- `mod.rs` — declares `pub mod {ai, physics, road}` and keeps every public path 506's tests and
  callers use with one line: `pub use physics::{Car, CarInput, CarParams, DT, FlatWorld, Friction,
  Rigid, SURFACES, Tuning};`. `nf_game::driving::{Car, …}` resolves exactly as before (pinned by a
  layout test, not by hope).
- `ai.rs` — 507's AI car step, now plus the physics wiring (below).
- `road.rs` — 507's RNgp road network, untouched.

`lib.rs` needed nothing (it already said `pub mod driving;`); there were no internal
`crate::driving` users to migrate. The E0761 ("file for module `driving` found at both") is gone
because only one layout remains.

## The wiring (the seam 507 left open, now closed)

507's report named `driving::ai::SimCar` the agreed seam ("a PVehicle plugs in"). It is now wired:

- `SimCar::of_car(&Car)` builds the AI's sim view from 506's physics car: pos = instance+0x10,
  vel = instance+0x20, forward = the yaw's `[sin, 0, cos]` row (the row `Car::step` drives along
  [M physics `step`'s `fwd`]); `alive` = 1.0 — FUN_00195a30's health gate is the actor layer, out
  of the headless scope [S].
- `AiDriver { brain: AiCar, car: Car, world: FlatWorld, rigid: Rigid }` — one 60 Hz step: the
  brain picks the pedals (FUN_0012e808 target speed → FUN_00133ab0) and the command maps 1:1 onto
  506's `CarInput` (SetSteer/SetThrottle/SetBrake/SetHandbrake [M 0x133ab0:135-180]); `Car::step`
  integrates it [M 0x1ff3d8's order]; the **pre-integration** sim speed drives the path progress
  (the AI reads |vel| before the vehicle sim integrates [M 0x133ab0:103-113] — the progress walk
  is `AiCar::advance_path`, moved verbatim out of `AiCar::step`, whose rail behavior is
  unchanged); the car lands back on the spline [M 0x222468] facing the travel direction.
- The brake rest clamp [S]: 506's brake integrates through zero into a negative velocity (reverse
  is the R-gear throttle path 506's gearbox work owns), which would back the rail car up; the
  driver zeroes a brake-induced backwards velocity, and at rest the AI's own speed ≤ 0 early-out
  [M 0x133ab0:153] keeps it parked. Without it the arrival test runs the car away in reverse.
- Known gap recorded, not hidden: `Car::step` does not read `handbrake` yet (on 506's own "what's
  left" list) — the forced-stop command is exact [M 0x133ab0:173-180] but applies no force;
  `the_forced_stop_handbrakes_and_the_speed_holds_until_506s_handbrake_lands` fails the day 506's
  handbrake lands, on purpose.

Docs: `docs/systems/driving-ai.md` gained §8 (the wiring, evidence-tagged) and §7's stale
"a small trait" line now points at it.

## How it was verified

- New tests `crates/nf-game/tests/driving_ai_physics.rs` (8, all green): the layout paths, the
  sim view, the acceleration numbers with a **bit-exact bare-`Car` replay** (the driver adds no
  physics of its own — a fresh `Car` fed the recorded commands and rail snaps ends bit-identical
  in vel/speed/gear/steer_angle), the steering-angle falloff [M 0x18cef8:458-474], the arrival
  rest (no reverse runaway), the forced-stop record, the clean-link crossing at the physics
  speed, standby. Fixture: the hand-built format-exact `.crp` (the 507 walk, trimmed).
- Existing suites on the final merged tree (origin/master merged; WORKLOG conflict resolved
  "both kept"; no code overlap): `drive_physics` 13/13 (506 on the moved file), `driving_ai_follow`
  9/9 (507 after the refactor), `objective_state` 5/5.
- The full suite, foreground slices (the background `tools/test/suite.py` start died empty again —
  this sandbox reaps detached processes, the note is in 507's and screen-adjust-model's entries):
  - **nf-game: 41 binaries, 371 passed / 0 failed** (`--test-threads=1`) — the crate I changed,
    run in full.
  - **nf-formats + nf-audio: 104 passed / 3 failed** — the 3 are exactly the `disc_golden` tests
    the task allows to fail on the slim ISO.
  - **nf-view** (untouched by this task): 304 test binaries, ~81 executed serially before the
    window closed. The only real failure is the known no-GPU `fix_0c_pylon_live` pair (2 tests —
    the same environmental pair 507 recorded). `campaign_flow_live` (5/5) and
    `fix_guard_weapons_live` (3/3) pass with 180 s budgets (my first 60 s cap caused their
    timeouts). Two operational notes for the next session: `cargo test -p nf-view` aborts at the
    first failing binary — use `--no-fail-fast`; and running the GPU-less live binaries 4-way
    parallel OOM-kills them (rc 137) — serial only. The remaining ~220 binaries need a serial
    sweep next session.

## What's left

- The nf-view serial sweep completion (~220 binaries of 304) — environmental coverage only; my
  change touches none of them.
- 507's carry-overs, unchanged: the real-data NF_DRIVING golden (the 429 MB driving archive vs
  disk), the FUN_001a85d0 curve-eval live check (L-3), the profile +0x84 case table.
- When 506's handbrake lands in `Car::step`, update the forced-stop test to assert the decel
  (it is written to fail loudly that day).

## Live checks for the main agent

- **L-6 — the AI car's speed comes from the vehicle sim**: breakpoint FUN_00133ab0 0x133ab0 during
  a paris_mis01 traffic run; expect the speed read from AIVehicle's sim +0x20 to equal |velocity|
  and the SetThrottle/SetBrake args to match the port's ramp numbers (span 7.5, floor 0.05) [M
  expectation].
- **L-7 — the brake rests, never reverses**: with an AI car parked at its aim point, watch the
  sim +0x20 velocity: expect the game's brake to hold ~0 (no negative drift) — this pins the
  port's rest clamp [S expectation].
- **L-8 — the AI car rides the spline**: breakpoint FUN_00222468 0x222468 for a traffic car;
  expect the position to be the segment evaluation `A + (B−A)·t` with t advancing by
  speed·dt/|B−A| per frame [M expectation].

## Shared files touched

- `crates/nf-game/src/lib.rs` — nothing (unchanged from master's `pub mod driving;`).
- `docs/WORKLOG.md` — appended my entries only; the merge conflict resolved by keeping both sides.
- `docs/systems/driving-ai.md` — appended §8; one stale §7 line corrected to point at it.

## ntfy failed

(recorded below only if all three fallbacks fail — see the final section.)
