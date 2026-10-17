# The Mayhew glass breach — guards through the windows

The user, 2026-10-16 evening: "beginning of mayhew level guards are supposed to break through the
glass". Level 07000001 (Paris Prelude, Castle A — the Mayhew estate). Builds on 750e
glass-shatter's breakables (`docs/systems/breakables.md`, `crates/nf-game/src/breakable.rs`).

## The scene

Bond starts in the courtyard (start #0, (−2.78, 6.84, −12.40) [D placement]); the entry-hall's
front wall (the z ≈ −1 plane) carries three kind-2 glass panes [D bounds from the port's
`Breakables::create`]:

| placement | name | bounds |
|---|---|---|
| 130 | radioroom-adjacent wall pane | lo [−7.94, 5.40, −1.01] hi [−2.07, 7.85, −1.01] |
| 136 | hall centre pane | lo [−1.91, 5.34, −1.167] hi [−0.09, 7.95, −1.167] |
| 155 | the transom above 136 | lo [−1.91, 7.95, −1.167] hi [−0.09, 8.87, −1.167] |

plus the east-wing pair 121/122 (x 15-18, z −5), comroom 123/124 (x 17.98, z 3-4.9), the west-wing
quad 206/231/232/233 (x ≈ −27, z 7.2-9.2) and the radioroom screen 76 — 12 kind-2 panes on the
main map [D]. ALL kind-2 panes carry health 0.0 and no filter [D placements].

The breach squad: the 0xF1 spawner placement **#297 — mode 2, group 1, start channel 11** [D
params; the port's `Spawner::new` parse] holds **5 group-1 guards** (placements 168, 169, 261,
298, 299 — skins 0x02000637/38/3a inside the hall, x −13..+7, z 0..10 [D]) as templates. The intro
cut-scene (SP #277, script 0x06000087) writes **ch 11 = 1** at its stream 8 [D script decode; the
port's measured tick ~2119, `walk_07000001_live`] → the spawner activates and spawns the squad
inside the hall → their AI charges Bond in the courtyard → they run into the glass.

## The smash arm [M]

`Break_Update` 0x12bfd8 runs per intact pane per tick (`obj+0xFE & 1 == 0`):

```
Collide_FilterBullets(obj+0xD0, filter | 0x106)          // :15 — the mirrored contacts
for each hitter HITDATA+0x58:
  class 2 (a drone / the player) and GameState._12_4_ == 0x7000001 and kind (instance+0) == 2:
      health -= Vec_SqDist3D(hitter+0x30, hitter+0x40) * 20.0    // :26-27
  class 5 (a bullet): health -= HITDATA+8                        // :36-37
health < 0.0 → Break_Kill(pane, lastHit)                          // :49-51
```

- The class-2 drain is **level-gated to 0x07000001 alone** — no other level smashes glass by
  contact [M :24]. The kind gate is inside the level arm [M :25].
- The delta is the hitter's **tick movement**: the movement pass copies +0x30 → +0x40 after each
  move [M 0x1336f8:212; docs/fields/obj-tag.md +0x40], so `+0x30 − +0x40` = this tick's step.
  A motionless brush drains 0 — a health-0 pane survives a stationary contact [M :27,49].
- `Break_Kill` 0x12b970: `obj+0xFE |= 1` (the pane leaves the world — the collision "goes away",
  the guard walks through) and the sound: **kind 2 always 0x2e** (no RNG draw — the shared stream
  is untouched by glass kills [M 0x12b970:65-76]).
- The pane's triangles are surface 0x10 glass; `Bullet_CollisionHandler`'s glass rule lets ROUNDS
  pierce (750e), but the class-2 body contact is the collision pass's ordinary mirrored hit — the
  pane DOES stop/blocks bodies while intact (the guards pile against it until it breaks).
- `Break_DoBreak` 0x12c168 (the scripted kill): walks the object list for the **first** intact
  class-0x20 object whose world sphere contains the point [M :39-51], builds the synthetic
  HITDATA from the pane's facing (Mat_GetDir of RotMatrix(obj+0x50) → HITDATA+0x10; +0x38 = −0.5;
  +0x40..+0x4C = the pane position/radius [M :52-60]) and calls Break_Kill [M :61]. Its ONLY
  caller is `Script_EventHandler` 0x1d94c0:147-151 — **NIS event kind 0x0A**, on the previous
  stream's object (the same rule as events 6/0xB/0xF [M :94-96]), position = that object's live
  animated position `obj+0xC0` [M :112]. The disc's ONE 0x0A occurrence: 07000001's briefing
  script 0600005b stream 8 gate 127 [D census] — orphaned (no placement references the script
  [D scan]) but ported for the system's completeness.

## The port

- `Breakables::drone_smash(prev, pos, level)` (breakable.rs): the class-2 arm — per intact kind-2
  pane, the tick's movement segment (the game's +0x40/+0x30 pair) against the pane's bounds grown
  by the body radius 0.35 [*suggested* — the port's standing body half-width; the game's contact
  is the full collision volume against the pane's triangles], drain `SqDist(prev, pos)·20`, kill
  at < 0 with sound 0x2e, no RNG [M 0x12bfd8:23-51, 0x12b970:65-76]. Kind-2 kills never draw the
  shared stream (750e's determinism rule holds).
- `Breakables::do_break(pos)`: `Break_DoBreak` [M 0x12c168] — the first intact breakable whose
  bounds (grown 0.2 [*suggested*; the game's sphere is the glist world-sphere]) contains pos, the
  standard kill. Wired to NIS event 0x0A in `nis.rs` (the previous-stream object's position, the
  events-6/0xB/0xF rule [M 0x1d94c0:94-96,147-151]).
- `Drone::prev_pos` (+0x40): captured at the head of the per-drone control block each tick
  [M 0x1336f8:212 — the movement pass copies +0x30 → +0x40 after the move]. Bond's tick-start
  position is the existing `Drones::player_pos` cache.
- The smash pass runs after the control walk in `Drones::tick_inner` (the game's Break_Update is
  the breakable object's own control pass — the port folds it after the drones' moves, which is
  the same per-tick data [S order; the game's list order interleaves panes and drones]).

## Tests

`crates/nf-view/tests/glass_breach_live.rs` — the real path: the intro rides itself (no input),
ch 11 fires, the spawner releases the squad, Bond WALKS (real step_bond inputs) to the courtyard
in front of the west pane 130, and a charging guard shatters it and crosses into the courtyard.
Unit tests in breakable.rs pin the arm's numbers (the drain formula, the no-draw rule, the
level gate, the stationary-contact rule).
