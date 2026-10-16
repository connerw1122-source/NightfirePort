# guards-0c-b — the spawner's visibility gate decoupled from the drawn gate

The merge hold on `s/guards-0c` (task 698): with guards-0c's drawn-gate change alone on master,
`crates/nf-view/tests/fix_finale_live.rs::the_0700001b_drakes_defeat_ends_the_space_station`
fails at line 161 — "the gate spawns Drake (spawner #187 [D])". Drake never spawns after the
fight gate (ch 0x0a), so the 0700001b space-station finale can never start. Branch
`s/guards-0c-b` = `origin/s/guards-0c` merged with `origin/master` (ad058a2d).

## Which reader the spawner path uses

The port fed the spawner's post-selection abort from `visible_to` — the DRAWN gate
(drone.rs, the closure passed to `Spawner::update`). That was the bug: the game's spawner path
never reads the drawn flag or the drawn gate.

- `DroneSpawner_Control` (0x137e68 [M]) reads NO visibility at all: its release test is the
  trigger channel word +0xe one-shot latched into +0x1e (0x137e68:53-70 — `switch_channels[p6]`
  clears +0xe and sets +0x1e; the spawn loop then runs `SpawnDrone` per empty group slot),
  behind the +0xfe-bit-0 disable, the p7 kill latch (+0x20) and the mode gates. No cel, no ray.
- The ONLY visibility reader on the path is inside `DroneSpawner_SpawnDrone` (0x137cb0:43-47 [M]):
  after the random pick consumes `Rand_Rand`, if stealth screening is on (+0x1f == 0) and
  **`Drone_PositionVisible`** (0x13a7a0 [M]) says the picked spot is visible, the attempt returns
  0 — the slot persists and retries next tick. That is the gate guards-0c's ray change moved.
- `Drone_PositionVisible` (0x13a7a0 [M]) is NOT the drawn gate. It is, in order:
  1. `Vision_InView(pos, 2.0, viewer)` — the four fov side planes with radius slack 2.0, no
     collision (0x1e89e0 disasm [M]: four plane dots, `< -param_1` fails);
  2. the spot's cel byte +0xa0 — when the template carries a cel (0x137cb0 passes `piVar10[1]`),
     an undrawn cel short-circuits to NOT visible → the spawn proceeds (0x13a7a0:58-60 [M]);
  3. ONE `Collide_LineOfSight` ray viewer→spot via `DroneVision_LineOfSightToObject` (0x178390
     [M]: the AI cel walk first, then the ray) with HITTEST word **0xa27** (0x13a7a0:73-74
     `param_6 | 0xa27`). 0xa27 is the FLAT segment: NO bit 0x8, so pass-flagged occluders
     (glass, foliage) BLOCK the ray — unlike the bullet family, where such hits drop outright
     (0x1ebff0-0x1ec010 [M], the disasm guards-0c cited); bit 0x20 set → the 0x1000 missile cage
     is skipped (`segment_actor`, Collide_Pick 0x1eb480 [M]); char 0 → dynamic-owned nodes tested.

So the drawn gate and the spawner gate are different readers with different rays, and
guards-0c's bullet-segment change is correct for the drawn gate and wrong for the spawner.

## Why the bullet-segment query differs on 0700001b

With the drawn gate feeding the abort, the abort verdict on #187's spot flipped when the rays
moved segments: the sightline from Bond's post-shuttle position to the Drake spot is
**bullet-CLEAR but flat-BLOCKED** — a pass-flagged surface (glist flags bit 0x40 or a surface
byte bit 0x40; `segment_bullet` = the flat segment MINUS exactly those hits [M 0x1ebff0-0x1ec010])
stands between the eye and the spot. The old flat rays judged the spot not-visible → spawn
(master green); the bullet rays judge it visible → the abort rejected every spawn (the hold's
RED). The game's own ray is the flat 0xa27 one, so the game also judges the spot not-visible
and spawns Drake — the fix restores the game's verdict by giving the spawner its own test, not
by touching the segment queries.

## What landed

`drone_position_visible` (nf-game drone.rs, new [M 0x13a7a0]) — the spawner's abort now calls it
instead of `visible_to`. It maps the game's three parts onto the port's stand-ins: the same
60°/2-unit cone for `Vision_InView` (no camera frustum in the sim — the same stand-in shape the
drawn gate uses), the five bullet-segment rays for the cel byte +0xa0 (no portal graph in the
sim; the guards-0c stand-in for the portal cel walk — an undrawn-cel spot must spawn), and the
game's exact ray: ONE flat `world.segment_actor(eye, spot)` (0xa27: flat, cage-skipped,
dynamics included). `visible_to` itself is unchanged — it remains the drawn gate only (the
comment now says so), so guards-0c's on-screen-gait fix is untouched.

## How it was verified

- RED first: on this branch pre-fix, `fix_finale_live::the_0700001b_drakes_defeat_ends_the_space_station`
  fails at fix_finale_live.rs:161 (the task's exact symptom) — reproduced 16:16 UTC.
- Post-fix: `fix_finale_live` **4/4** (the 1b Drake chain green end to end: spawn → enabled →
  death → ch 0x0e → the NIS's ch 0x0b/0x64 → MissionState 6 → 0x0730001b), `guards_0c_live` 1/1
  (the drawn-gait trace metric), `spawner_kill_live` 5/5, `level_end_live` 6/6, `ai_live` 35/35
  (the castle-patroller drawn-slide pin bit-identical), `nf-game --lib` 240/240.
- Full fast suite on the final tree: see the total line below.
- SUITE-TOTAL-PLACEHOLDER

## What's left

1. The cel byte +0xa0 stand-in is level-agnostic (the five bullet rays) because the sim has no
   portal graph and the port's spawner templates do not carry the cel word the game passes
   (0x137cb0 `piVar10[1]`). If a level ever shows a spot spawning on-screen that the game
   defers, parse the template cel and gate the stand-in on it.
2. guards-0c's remaining items stand unchanged (the portal-walk wiring, the LinkCreep backward
   creep, the wait nodes).

## Live checks for the main agent

1. PCSX2, 0700001b, after the shuttle arrival: set the fight gate (the missile choreography's
   end — in game, survive the missile run) and confirm Drake (skin 0xba) appears at spawner
   #187's spot while Bond's default aim faces roughly at it — i.e. the game DOES spawn him with
   the spot inside the view cone, matching the flat-ray reading above.
2. gdbwatch on 0700001b: break on `Drone_PositionVisible` 0x13a7a0 during the fight gate —
   expect it called once per spawn attempt with the InView pass and the LOS ray returning 0
   (blocked) for #187's spot; and read the hit surface's pass flag (HITDATA+0x51 bit 1) to name
   the exact pass-flagged surface on the sightline.
3. gdbwatch `DroneSpawner_SpawnDrone` 0x137cb0 +0x1f (the stealth-screening byte) for spawner
   #187: expect 0 (screening on) — the abort path must be live for this spawner.
