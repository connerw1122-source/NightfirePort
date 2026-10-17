# level-01-complete-2 — the 01 full walk finished (the held branch un-stuck)

Task 701c, branch `s/level-01-complete-2`. The held `s/level-01-complete` (701b) was blocked at
merge because `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` failed at
"the contact must walk his route to the LoadPointActivator plate". This task merged the held work
(merge 5644e22e, both WORKLOG/level-doc sides kept), found the real stall, fixed it with
decompile evidence, and the walk now passes end-to-end alongside the held tests.

## The root cause: the armed-peer snapshot, not the route

The contact (skin 0x4F) froze mid-escort because the port counted a guard he cannot even see as
an "armed peer". Reproduced first (the walk settled at (−16.6, 6.3, 14.5) with the plate at
(−33.1, 2.3, 15.4), 202 s of stalling), then fixed in the snapshot:

- **The game's armed-peer scan** [M `NDrone2_NearArmedDrone` 0x177420 / `NDrone2_NotNearArmedDrone`
  0x177640] needs, beyond started/alive/team-1/within-radius:
  - the **weapon filter** — the candidate's weapon byte (obj+0xDC → +0x62) in [2..0x46] ∪
    [0x6a..0x71]. The decompiler C is ambiguous (signed vs unsigned); the disassembly settles it:
    `sltiu` on `(b−0x6a) <u 8`, `slti (s8)b < 2`, and the 0x47 branch at 0x177564 — armed =
    [2..0x46] ∪ [0x6a..0x71], the same predicate as `NDrone2_ObjectIsArmed` 0x178768
    [M disasm 0x177538-0x177578];
  - the **sight gate** — `DroneVision_CanSeeObjectFrom` 0x176f18 with bone 5 (the head): a
    head-to-head `Collide_LineOfSight` (0xa27) behind an `AINetwork_TestRayCels` AI-cel pretest
    [M 0x176f18:56-105]. The port's stand-in is the shared head-to-head `world.line_of_sight`
    (the shout pass's shape), scan capped at 20.0 (every consumer branches at 10/14/20 — a peer
    beyond 20 flips nothing).
- **The prop-0x50 gate on the flag-1 hide-enters** [M 0x177420:57-61]: the four AllyLead enter
  sites call `NearArmedDrone(…, 1)` [M 0x15e698:30, 0x15eda8:30, 0x15ea28:30, 0x15f120:26], and
  flag=1 requires the seeker's own behaviour prop 0x50 (`behaviour_util_getProperty` 0x1c7d98).
  The hide exit has NO prop gate [M 0x177640]. The port now gates exactly those four enters.
- **The mechanism of the stall**: the meet scene's balcony guard (skin 0x05000036, parked over
  the west stairwell) sat at 3D 3.9-13.7 from the contact through the whole descent, never
  visible from the ground. Without the sight gate the port's `armed_peer_dist` never exceeded
  14.0, so `LEAD_HIDE` never exited [M 0x15ec08 case 3] and the contact never finished his route.
  With the gate he stops being a peer the moment the line blocks — the game's own reading.

The predicate is documented with every address in `docs/systems/drone-armed-peer.md`.

## The walk test itself: three player-shaped fixes + one table fix

All inside `crates/nf-view/tests/walkreal_07000001_live.rs`, each documented in place:

1. **The hall detour stages through the east door** (2.9, 6.3, 16.3): from the meet room the
   routes to the west-half guards run through door #2 (−14.1, 5.3, 13.1) — the level's known
   walker wedge from the held report — and a guard fought through that wall never exposes a part.
   The east hop is the route the guard-11 approach already proves; the three named guards then
   die at 1.2-1.8 units and the hall AND fires (ch 153 at tick 3585 in the passing run).
2. **The stairwell recovery**: the contact hides at the stairwell head from the west-side
   combat-steppers (state 0x73 STEP_AIM_LEFT, health 7-10) plus the balcony skin-0x36 — in the
   game the hide RUNS him to cover (`NDrone2_FindRunToPoint` 0x15168, the half the port does not
   model) while the player fights. The walk fights them the same way: clear every started armed
   guard within 14 of the contact, then Bond LEADS the descent (the wait bands stall only for a
   Bond BEHIND [M 0x15e698 case 3's −9.0 path arm]) through the proven stairwell-head anchor.
3. **The ch-0x47 hand rides Mayhew's arrival beat**: the exit-hold freezes the world once the
   AND fires (step_bond returns before Drones::tick, docs/systems/exit-hold.md), so a post-AND
   hand lands on a frozen monitor. The hand now lands before Bond's last step; when the exit
   races the hand (the recovery's follow can carry Bond onto the plate right behind the
   contact), one frozen-window monitor call stands in — the same monitor over the same channels
   [*suggested* stand-in, tagged in the test].
4. **The mission record indexes are resolved from the LEVELS table** by level hashcode: master's
   mission-05..08 records sit in FRONT of 01's in the table (the game's .data order), so the
   held test's hardcoded `states[0][0]`/`states[1][x]` had come to name mission 05's future-part
   records. The TEST was wrong, not the code [M 0x1827d8 walks LEVELS in table order] — fixed to
   `position(|l| l.level == 0x0700_0001)`.

## Verification

- The walk test passes end-to-end (`cargo test --release -p nf-view --test walkreal_07000001_live
  walk_from_the_spawn_to_the_exit_with_the_contact`, 181 s): the hall AND (ch 153), the escort
  route to the plate, the AND-touch (ch 0x10), level_exit 07000002, obj0's complete and
  07000002's announces through the real monitor.
- The engine change touches the shared `armed_peer_dist` snapshot (used by the AllyLead family
  and the hostage states' consumers) — the full suite is the regression gate for the escort/
  hostage tests (escort_follow, fix_actors, npc_doors, walkreal_07000007, level-02's hostage
  walkreal). The suite (`tools/test/suite.py`) was run on this exact tree; the total line is
  recorded below under Suite.

## What's left

- The +0x2B8 armed-peer latch (0x177420:126 → 0x177640:98-100's distance-only hysteresis) is not
  modelled — the walk does not hit it; the next escort task can pin it with a live check.
- The game's 0x600 down/stun bits in the peer filter are not modelled (the port's
  started/corpse/health covers the shipped levels' cases).
- The hide's run-to-cover half (`NDrone2_FindRunToPoint` 0x15168 → AllyGoToGoalPosition) is still
  unported — the walk works around it the way a player does.

## Suite

(full-suite total line recorded after the run — see the WORKLOG tail.)

## Usage

not visible to me (no token/credit/context figures are exposed in this environment; no cost
numbers are shown anywhere in the session UI).
