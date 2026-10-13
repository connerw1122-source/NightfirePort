# fix-swim-vent3 — the last two reds, investigated; the fixes are not landed yet

Branch `s/fix-swim-vent3` = `s/fix-swim-vent2` (8b6374a) merged with `origin/master` (3b04015, twice —
7fb019b then 3b04015 as master moved; both merges append-only in WORKLOG). No production code changed
this session; the two reds were dug from the disc and the port's world with two new read-only probe
examples. The task's goal (the two live tests green) is NOT met yet — everything needed to land it is
below.

## Suite status on the merged branch

- `nf-game`: lib 121/121 and every test binary green (the full crate run, `--test-threads=1`).
- `nf-formats` + `nf-audio`: green (only the allowed `disc_golden` trio remains slim-ISO-failing there —
  none seen in these runs' outputs beyond it).
- `nf-view`: the branch-adjacent binaries green — `fix_objects2_live` 5/5 (the cage), `impacts_live` 2/2,
  `fix_swim_vent_live` 4/4. The two reds confirmed: `fix_mission_live::the_07000001_exit_works_end_to_end`
  (Mayhew stalls 3.6 short of the plate) and `walkreal_0700000c_live` (ch 0x1b never fires).
- NOT yet run on the branch: the remaining long nf-view walk binaries (the full four-crate pass is owed
  before the swim fix lands on master).

## Red 1 — 07000001 exit (fix_mission_live:347): the pads are fine; the stall is dynamic

The previous session's probe spot was wrong. The plate centre is the touch volume's centre
**(−33.105, 2.301, 15.429)** (`Touch` out 0x10, the test reads it from the triggers), not (−36.6, 11.8).

Evidence (new example `crates/nf-view/examples/probe_pad.rs`, raw 0x2E blocks via `probe_planes`-style
parsing + the port's `CollisionWorld`):

- The disc HAS the pad geometry: object 137's descending step slabs x −33.8..−32.4, tops stored-plane-UP
  (raw s16 (0, +16384, 0) class): y 3.959 @ z 12.2-13.6, y 1.809 @ z 15.4-16.8 (the plate pad), y 1.520,
  1.202, 0.903, 0.643 descending to z 18.7. Winding matches stored (no flip; the level has 2 flips in
  54,823 triangles, neither here).
- The port's flat world finds the whole corridor: down-rays at x −33.5/−32.5 hit y 4.14 → 0.14 (mat 0x0a,
  normals up) at every step, with `segment_surface(no_water=false)`, `segment_surface(no_water=true)`,
  `segment` (floor_below's query) and `segment_hit` ALL agreeing. The water-material drop and the re-wind
  cannot be hiding the pad: it is hit by every query, including the player's grounding one.
- At Mayhew's exact stall pose (−33.015, 4.539, 11.800, feet 3.539): `move_by`'s two probe rays
  (`line_of_sight` at pos.y and +0.5 toward the route target) are **CLEAR on both master and the branch**,
  `feet_on_point` finds the floor 3.539 on both, the boundary walls (the nav `Wall` list) are **byte-identical**
  between the trees, and `follow()`'s block test `bounds_node_test(0.2, feet, target)` is unblocked.

So the three named static suspects are eliminated at the stall: the re-wind (geometry-identical, hit sets
only shrink for rays), the cel scoping (Mayhew is a drone — every query he runs is the flat one), and the
water drop (no 0xE material anywhere under the corridor). What actually happens (temporary env-gated walk
debug, **reverted before commit**): Mayhew is 1-2 route nodes AHEAD of master's escort from t1600 (one
2.0 test stall-nudge of lead), works door 4 at (−32.6, 8.3) (`hit_door Some(4)`), descends the obj 139
stairs and then ping-pongs at their bottom — walk (`AllyLead` 0x1d, gait script 100663498 running with
real root deltas) ↔ `DSTATE_OpenDoor` (0x81) ↔ the 180° turn anim (script 100664792) — so the net xz
advance is ~0.02/1000 ticks and the route never reaches the plate leg. Master's escort reaches the same
stairs ~400 ticks later and walks through.

Next session: root-cause the creep/target flip that turns Mayhew around at the stairs' bottom (the
`follow()` creep advance vs `next_node` order on the last legs; the walkdbg showed no bounds block and no
blocked flag, so the flip is in the route node order or the arrival radius), or make the escort's descend
leg survive the door-4 re-entry. No test change is warranted: the plate, the pads and every query the
walk uses check out.

## Red 2 — 0700000c ST-1 plate (walkreal_0700000c_live:94): the pad is there; the approach stops on the band's edge

Evidence (new example `crates/nf-view/examples/probe_scoped.rs`):

- The plate trigger is placement **506**, out 0x1b, volume x 4.67..7.00, y 12.73..13.67, z −155.13..−153.18
  (centre (5.84, 13.20, −154.15)). The pad floor under it is in the port's world: flat down-ray y 11.84,
  normal up, at the centre and ±0.5 around it.
- Bond's grounding in the walk is the FLAT `segment_hit` (player.rs:509/994), so the cel scoping does not
  gate the grounding here; Bond's scoped query misses the pad only from his spawn cel (212, 38 units away),
  which is correct cel behaviour.
- The test's walk target (6.8, 12.2, −153.15) sits **0.03 outside the volume's z hi (−153.175)**: the
  walker breaks at `dxz < 0.5`, so Bond parks on the band's northern edge and the touch never sees him.
  Master passes because its walk dynamics carry him a fraction further south. The test itself is not
  wrong — the game's Bond walks ONTO the pad (centre z −154.15, 1.0 inside the band).

Session-2 addendum (probe_scoped's seam map + poke): the touch evaluation itself is PROVEN good —
Bond poked to the pad centre (5.84, 12.9, −154.15) fires ch 0x1b on the poke tick (his cel relinks to
284), and the pad floor spans x 5.0..6.5 south of the corridor (y 11.84, up plane), with VOID (floor
6.27) at x ≤ 4.5 and x ≥ 7.0 south of z −153.2; the corridor itself (y 11.27-11.32) runs to z −153.2
even at x 7.0-7.2. The seam triangles are mats 05/06 — the water drop is NOT the cause. The failure is
purely the approach walk: Bond parks at (7.2, −153.1) FACING SOUTH, and every walker variant (pad-centre
loop target, dogleg via x 5.5, walk_to the pad, turn-in-place phases) bled into the x ≥ 7.0 void before
the 0.07/tick turn caught up, or stopped 0.03 outside the band. The test file was left at HEAD (no
change landed — the walker variants are documented here instead of committed half-working). Next
session: teach the loop turn-in-place FIRST at the parked spot (safe floor at (7.2, −153.1)), walk the
corridor WEST to x ≤ 6.5 (targets (x−0.5, −152.95)), then due south over the pad's wide part to
(5.84, −154.15); the poke proves the touch fires the moment Bond is on the pad.

## Files touched this session

`crates/nf-view/examples/probe_pad.rs`, `crates/nf-view/examples/probe_scoped.rs` (new, read-only
probes), `docs/WORKLOG.md` (this entry), this report. The temporary walk-debug instrumentation in
`drone.rs` and the diagnostic print in `fix_mission_live.rs` were reverted before the commit
(`git checkout HEAD --`); no shared file carries a change.

## Live checks for the main agent

1. 07000001, gdbwatch on the escort: while Mayhew walks the last upper-floor legs (t≈3500-4000 in the
   port's clock), watch `NDrone2_ReachedDestNode` 0x153358 and the `LinkCreep` target word (Drone+0x670)
   at the stairs' head — the port flips the move target ahead/behind at the stairs' bottom; the live
   order of the descend legs (node list order around the west stairs, nodes ~33-40 of the lead path)
   would settle whether the port's `next_node` walks the same order the game does.
2. 0700000c, fieldprobe: place Bond at the plate centre (5.84, 12.9, −154.15) standing (feet 11.84) and
   read the touch trigger's state word for placement 506 — whether the game's touch needs the full volume
   overlap or a centre-point test (the port's band edge stop at z −153.15 must be inside whatever the
   game tests).

## What's left

1. The 07000001 escort stall fix (dynamic — see above), then `fix_mission_live` green.
2. The 0700000c approach/touch fix, then `walkreal_0700000c_live` green.
3. The full four-crate suite on the branch (`--no-fail-fast --test-threads=1`), then the swim fix is
   ready for the main agent's merge review.
