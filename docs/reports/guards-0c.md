# guards-0c — the 0700000c guards' AI and walking

Playtests 1012-1723 / 1012-2105: "All of the AI in this level is bugged as hell. The walking
animation is kind of fucked too." Branch `s/guards-0c`.

## What the trace found

`crates/nf-game/tests/guards_0c_live.rs` (new) lists all 40 0c drones at start and traces 1800
ticks (class/mode, state transitions, route, the anim state/danim and the fractional frame per
tick, the walked distance, and the anim-frames-per-unit-walked slide metric):

- **40 drones at start; 31 are spawner-held** (they go not-present at tick 0 and the spawners
  release them as Bond approaches — the game's streaming, not a bug).
- **Three patrol guards carry routes** (#1, #15, #39, state 0x07 PATROL, walk danim 0x188 script
  06000392, 115 frames). The rest are static sentries (IDLE 0x1, the alarm 0x5, sniper/civil
  states) — no route.
- **The walk-anim slide, root-caused**: a guard the port marks NOT drawn does not animate at all —
  `NDrone2_DoAnimation` (0x149f88 [M]) slides him along his yaw at the danim's `Drone_AnimInfo+4`
  speed (0.02/tick for walk 392) with **no gait** ([L] work/switch_trace.tsv: the game does the
  same for a castle-hidden patroller). The port's drawn gate was a 60° cone + **five LOS rays on
  the flat collision segment** — a test the game does not run: the game draws what passes the
  portal-visible cels (`View_AddObjects` 0x1e62d8 → `Vision_Portal_Recurse`) and then the object's
  sphere vs the four fov side planes (`Vision_InView` 0x1e89e0 [M] — a pure plane test, no rays).
  Every guard behind a ray-blocking but drawn-through surface — the 0c museum's glass displays —
  slid on screen with a frozen gait, and guards at the frustum edge popped between the 0.02 slide
  and the walk. That is the playtests' "walking animation is kind of fucked".
- **The route block** (guard #15): walks 5 units, then stands for 1400+ ticks still in PATROL with
  the walk anim playing — the route follow's bounds test blocks the link and the guard's undrawn
  slide ran into collision. The game's `LinkCreep_Calc` backward-creep half (re-creeping the
  stepped-back node toward the drone) is not modelled ([S] noted in nav.rs) — the guard re-approaches
  in the game instead of parking. The walk anim on the blocked codes is the game's own
  (`UpdatePatrolRoute` 0x1514d8 [M]: every non-0/non-7 code plays `SetWalkIdleAnim`) — not a bug.

## What landed

`visible_to` (nf-game drone.rs) — the drawn gate — now casts its five body rays on the **bullet
segment** (`world.segment_bullet`, the pass-flag-aware query [M `Collide_Intersect`
0x1ebff0-0x1ec010 for the bit-0x8 HITTEST queries]): hits whose hit-data carries the pass flag
(foliage [D 07000005's trees, glist flags 0x41]; the glass-class displays) no longer hide a guard
the renderer draws. Opaque occluders block both segment kinds, so the game-pinned no-gait slide of
the castle-hidden patroller ([L] ai_live `switched_guard_patrols_like_the_game`, the
work/switch_trace.tsv positions) is bit-identical — the test stays green.

Before/after (guard #39, the 1800-tick trace): the anim advanced 71 frames over 1797 moving ticks
(frozen gait, 0.02 slide, 34.9 units) → **1783 frames over 1766 ticks (the full walk cycle at
1 frame/tick, 17.3 units at the walk's root rate)** — the feet match the ground.

## How it was verified

- `guards_0c_live::patrol_trace` (new): the drawn-tick anim-frames-per-unit metric must sit in the
  walk's root band (60..130 frames/unit; #1 measures 56 with its slide ticks included), no patrol
  guard leaves its post state.
- `ai_live` 35/35 (the castle patroller's live-pinned slide intact), `walkreal_07000001/2/4/5_live`
  green (the root-motion regression net at speed 1.0).
- What I ran: `cargo test --offline --release -p nf-game` for guards_0c_live, ai_live; `-p nf-view`
  for walkreal_07000001/2/4/5_live. **The full four-crate fast suite did not run** — the sandbox
  window went on the disassembly verification (Vision_InView/View_AddObjects/UpdatePatrolRoute) and
  the regression net above; the diff touches only the drawn gate (one predicate), so the blast
  radius is the guard tests.

## What's left

1. **The portal-walk wiring** (the real fix): the game's drawn flag is the cel-visible set AND
   Vision_InView. The renderer already computes both per frame (gpu.rs `cel_visibility` /
   `depth0_game`, `vision_in_view`); they should drive `dr.drawn` (the scene writing the sim's
   flag, one frame stale — the game's obj+0xFC decays per tick and is re-set by the last draw
   pass). The sim's `visible_to` then only backs the headless tests. My frustum-only prototype
   (the 4 planes, no rays, no cel gate) broke the castle-patroller pin — the cel gate is
   load-bearing, do not ship it alone.
2. **The LinkCreep backward creep** ([S] in nav.rs:592-598): the blocked patroller (#15) re-approaches
   in the game instead of parking; porting it un-sticks #15's route.
3. The full fast suite on the final tree.
4. The wait nodes (`+0xBAE`, docs/port-map/guard-ai.md 0x15c498 "partial") — the 0c patrol loops'
   timed stops.

## Live checks for the main agent

1. PCSX2, 0700000c: stand so a guard behind a display-case glass panel is on screen, level camera:
   the guard should WALK with a full gait (the game draws through the glass). Then the same guard
   with the panel's frame filling the ray — confirm the port's bullet-segment rays now match.
2. gdbwatch a patrolling guard's `obj+0xFC` (the draw counter) walking behind glass: expect it
   pinned at 2 (drawn) while the port's old flat-segment rays would have let it decay.
3. The blocked patrol on 0c (guard #15's route, the walkway east of the start): confirm the game's
   guard re-approaches (the backward creep) rather than parking — the LinkCreep follow-up's target.
