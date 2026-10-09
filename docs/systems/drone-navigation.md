# Drone navigation: AI network, routes, patrols

Sources: ACTION.ELF decomp and disassembly (three research passes, WORKLOG 0021, spot-checked by me:
`AIPath_Parse`, `AIPath_Prepare`, `AIPath_BindNodes`, `NDrone2_AssignAIPath`), the 07000005 data. [M] = read in
the code or bytes, [S] = inferred, [L] = matched live. Data format: docs/formats/ai-path.md. Port:
`crates/nf-game/src/nav.rs`.

## Network [M]

- Block 0x05 holds **paths** (kind bit 0 clear) and **boundaries** (bit 0 set). Path kinds: 2 navigation, 4 patrol,
  8 mission (prop 0x1D), 0x10 patrol variant (changes the first step); masks test the whole word.
- Path node: position +0x10 (y + 0.4 at load), node flags +4 (0x2468ABCE → 0). Link: nodes a/b, length = 3D
  distance (`AIPath_Prepare`), runtime flags +4 and a used-count +6 (cleared every frame by
  `AINetwork_ClearLinkFlags(0x310)` in `Drone_InitComms`). Per node: its links in link order.
- Boundary post: position (y + 0.4), height +8 = a ray straight up 256 units (`Collide_RayIntersect` flag 0x90):
  max(hit distance, 2.0), 256 when nothing is hit. A boundary link (BLINK) is a wall between two posts; flags 0x800
  door / 0x1000 kick (crossed by a link between two door / kick nodes, `NDrone_InitDoorNodes` /
  `NDrone_InitKickNodes`), 0x400 passable (crossed by any path link, `AINetwork_InitPassableBoundries`). Drone
  collision ignores walls with 0x1C00.
- Cels: every node and post belongs to the room cel containing it (`build_FindCel`: room cels whose AABB contains the
  point; several → nearest ray hit up/down). Searches only look at the cel's lists (`cel+0x24` nodes, `+0x34`
  BLINKs). *Port deviation*: cel AABBs and portals are not decoded (their source is unknown: the gfx-box union does
  not contain all nodes), so nf-game treats the level as one cel and adds a collision-world line-of-sight check at
  +1 to every move test in their place.
- Each drone routes in **one** navigation path (`Drone+0x954` = `AINetwork_NavPathForPosition`: the nearest node, 2D,
  among paths with `kind & 0xD == 0` in the drone's cel, else over all of them).

## Move tests [M]

Constants (`Drone_PostLoad_Init`): length² ≤ 900; |dy| ≤ 4.0; when |dy| > 2.0 (1.5 on 0x07000008) the slope
`atan2(|dy|, horizontal)` must be ≤ 0.8727 (0.7854 on 0x07000008).

- `NDrone2_MoveTest(A, B, …, hit, near)`: fails on the limits above; with no boundaries → 1; else `BoundsTest`.
- `AINetwork_BoundsTest`: the segment A→B (both y + 1) must reach B's cel through portals (`Collide_StraddleCels`),
  then for every BLINK of the cels crossed: `LineIntersectsBoundry` hit → 0 (or −1 with the nearest hit returned
  when a hit pointer is given). With `near` (and no hit pointer): skip walls with a post within 0.4 (dist² 0.16) of A
  or B; else if the closest point P on AB to post a has dist² < 0.16, or to post b has dist < 0.16 (unsquared: as
  written), and |P.y − a.y| < 2 → 0.
- `LineIntersectsBoundry`: 2D segment intersection I (a parallel overlap returns 1 without I); yLine = lerp(A.y,
  B.y, |AI|/|AB|), yWall / hWall interpolated along the wall; hit when yWall − 2.4 ≤ yLine ≤ yWall + hWall.
- `AINetwork_BoundsNodeTest(r, A, B)` (route creep): posts only: blocked when the closest point on AB to a post is
  within √thr (thr = 0.5 r² when the post is within √1.3 r of A or B, else r²) and |P.y − post.y| < 2.

## Routes [M]

`AIRoute` (`Drone+0x860` combat, `+0x970` patrol): mode (&7: 0 one-way, 1 one-way to an end point, 2 loop, 3
ping-pong), valid 0x10, node list, index (s16), count, start / dest / move target, remaining distance +0x88,
arrival radius +0x8C (2.0), look-ahead +0x90 (2.0; 4.0 when `Drone+0x23`), the link creep.

- **Target nodes** (`AINetwork_NodesForPosition`, player target rebuilt when he moved ≥ 2.0): per navigation path,
  the cel's nodes in a box (±√r² or ±15 in x/z, ±2 in y), nearest first (2D; ties after equals; cel lists are in
  reverse file order); the first with `MoveTestFromNode(node → pos)` = 1 is "found"; the first failure is the
  fallback.
- **`AINetwork_CalcRoute`**: if `MoveTest(start, dest, near 1)` = 1 → direct route (no nodes, status 2). Else A*:
  - start nodes: `NodeSearchCel` mode 1 (box ±25) from the start: each node with `MoveTestToNode(start → node)` = 1
    enters the open list with g = dist2D(start, node), h = dist2D(node, dest), sorted by f; −1 aborts.
  - pop the head; if it is a goal node (found mask; else the fallback mask, result 1) → the route is the parent
    chain; route distance = g + dist2D(node, dest).
  - expand each link: skip locked doors (both ends flag 2); cost = g + len × (used + 1); skip neighbours already
    open/closed with g ≤ cost; else parent, g, h = dist2D(n, dest), f; insert into the open list before the first
    strictly larger f unless already open (an open node is **not** re-sorted when its f drops). The expanded node is
    marked with the *open* bit (0x40000000), so closed nodes are never re-queued but their g/parent may still be
    overwritten. Empty open list → 6. (All four are bugs of this build, kept.)
  - `OptimiseRoute`: drop node 0 if `MoveTestToNode(start → node[1], near)` = 1 and the pair is not both door or
    both kick nodes; drop the last node likewise from the dest.
- **Link creep** (the move target): the link A→B is cut into steps of 0.4 (steps = trunc(trunc(|AB|) × 2.5), else
  1 step of AB when |AB| ≤ 0.4). Per frame: if the next point Q is ≥ look-ahead from the feet (2D) keep the target;
  else if it is < 0.4 away or `BoundsNodeTest(0.4)` passes, the target becomes Q. Past the last step:
  `SetupNextNode(+1)` (mode 2 wraps; mode 0 moves to the dest). Blocked (`BoundsNodeTest(0.2)` fails toward the
  target, dist² ≥ 1): step back one point if it passes at 0.1, else return 9 / 0xC.
- **Special nodes** (`node flags & 0xF007F`): the target is the node itself until the feet are within 0.4 (2D; or
  < 1.0 with another drone near), then `NDrone2_ReachedDestNode`: 1 look/wait, 2 door → 0x81, 4 kick → 0x82,
  8 → 0x83, 0x40 fiddle → 0x84, 0x20 → 0x21, 0x10000–0x80000 Mayhew lines.
- **`AINetwork_FollowRoute`**: remaining distance = dist2D(feet, next node) + links to the end + last → dest; past
  the end (mode ≠ 2) and closer than the arrival radius → 3 (arrived); else the creep (0, or 9/0xC blocked).
  `NDrone2_FollowRoute`: target → `Drone+0x670`, desired yaw `+0x694` = `ATAN2_APPROX` to it.
- Links in use: `UpdateLinksUsedCount` adds 1 to every remaining link of a followed route, so later drones in the
  frame avoid them.
- `NDrone2_MoveToObject` return codes: 0/1/2 moving, 3 arrived (route distance < 2.0), 4 target has no nodes, 5 no
  start node, 6 A* exhausted, 8 no network, 9/0xC blocked, 10 door.

## Moving [M]

Drawn or not [M, L] (WORKLOG 0024): a drone whose object was not drawn last frame (`obj+0xFC` = 0, the renderer's cel
visibility) and whose danim has info flag 4 and no blend does not animate (`NDrone2_DoAnimation` false): it slides along
its yaw by `Drone_AnimInfo +4` per frame (walk 392: 0.02, twice its root-motion speed). The port approximates "drawn"
by Bond's view (60°) and line of sight to any of five body points.

Collision with the network: `NDrone2_Collision` pushes the feet sphere (r 0.4) out of boundary walls without flags
0x1C00 (`GetBoundsPushVectorForSphere`; ported, the push itself *suggested*). Without it a sidestep left a guard 0.11
from the gate pillar's post and every route segment failed the post test (WORKLOG 0023).

Translation is root motion; `NDrone2_Steer` turns 0.1 per tick toward `+0x694` (plus the anim yaw offset
`+0x4C4`: the body faces desired − offset). Obstacle avoidance (drones and the player within 3 units) is not ported
yet.

`DroneAnim_SetCombatMoveAnim`: a = diff(yaw to move target, yaw to opponent) → quad with hysteresis (previous 0:
|a| > 2.356 → 2, > 0.925 → side; previous 1/3: |a| > 2.496 → 2, toward the other side past 0.785 / 0.925;
previous 2: |a| > 2.217 → 2, > 0.785 → side; side = 3 when a > 0 else 1).
- quad 0 (forward): prop 1 or 0 → anim 8 (run aim), else prop 2 → 0xF, else 0x1E (run); fire while moving if
  unseen < 3 s.
- quad 2 (backward): anim 4 (back off), offset π.
- quad 1/3 (sideways): anim 0xC / 0xD (strafe; 0x29 / 0x2A kept), offset ±π/2; rolls with prop 0x2E at 2 < d < 2.5;
  steps (prop 0x42) when the next node is < 3 away and the turn there < 150°; else 0x1E.
- Each only if the anim set has it; `+0x31C & 2` (a drone ahead) → state 0x5E.

## Patrols [M]

- Global enter → InitPatrol (6) with props 0x24/0x25. InitPatrol: `AssignAIPath(4)`: the nearest node (2D, < 0.3)
  of a kind-4 path in the drone's cel → `InitAIPath` → Patrol (7); none → Idle.
- `InitAIPath`: mode 2 (loop) for kind 4, else 1; kind 0x10 sets route bit 0x20. The node list starts at the start
  node and follows links: a node with 2 links leaves by the one not leading back; **the first node of a kind-4 path
  without 0x10 leaves by its second link** (link-index order); a node without 2 links re-appends itself; until the
  list holds the path's node count.
- Patrol msg 1: with prop 0x46 an idle timeout of (45 + Rand 10) s ((5, 10) when prop 0x25 and alert ≥ 0.66).
  Msg 3: `PatrolTalk` (rolls Rand(2) every frame once its timer expires; type-3 drones speak), `UpdatePatrolRoute`
  (FollowRoute; result 0 → walk anim (0x35 when prop 0x25 and alert ≥ 0.66), 7 → stand idle, else walk / 0x21),
  then `EnemyLookForOpponent`. Msg 4 (prop 0x46): anim 0x4B variant Rand(3) (0x36 when alert), then Patrol again.
- Wait nodes (flag 1): when alertness < 0.33 and the node is not the last one waited at (`+0xBAE`), msg 4 to itself
  1 tick later; otherwise forget the last one. So a loop with one wait node waits every other lap.
- `SetWalkIdleAnim`: in Patrol (SM id + 1) % 3 + 0x37 (0x37–0x39), 0x37 when the set lacks it (sets 5 / 0x13 have
  only 0x37).
- 07000005 (data): three drones patrol: (50.85, 28.27, 93.63) on PatrolPath 1 from node 20 (route 20, 26, 21, 22,
  23*, 24, 25, 0, 1*, 2 … 19; * = wait nodes), (−31.30, 37.58, −37.05) and (−32.77, 37.65, −22.32) on PatrolPath
  (Dr_17) from nodes 10 and 3. The other guards go Idle. The first one is *suggested* to be the drone WORKLOG 0017
  found "gone" (it had walked off).
- [L] PCSX2 (work/patrol_trace.tsv, 2,400 frames from level load, Bond at his start): the first guard is in Patrol
  (state 7) and plays walk danim 392 (anim state 0x37) **throughout**: it does not stop at the wait nodes. The wait
  node's msg 4 does call the look-around (0x4B) but the walk request in the same tick's msg 3 calls the playing state
  0x37, which zeroes the pending call (`ZeroAnimCall`). Its path matches the port within 0.35 units at every 300th
  sample, including the turn at the loop's far end (`crates/nf-game/tests/ai_live.rs`).

## Idle and alert [M]

Idle enter: prop 0x1D → `AssignAIPath(8)` → EnemyMission 0x1B (none in 07000005); alertness < 0.66 → stand idle
(0x24 + Rand 3 held 5 + Rand 5 s), idle timeout (45, 5) with prop 0x46 (msg 4 → anim 0x4B variant Rand 3); else
Alert 0x05 (anim 0x21; msg 4 → 0x22).

## Start-up [M, L]

- `DSTATE_Global` enter: mode 0x1E → 0xC3; a switch channel (`+0x134` = placement parameter 3) that is off →
  WaitSwitch (1); mode 0x17 → 0x32; a start script → PlayScript (3); else the start state `+0x5A2` (InitPatrol with
  props 0x24/0x25, else Idle).
- WaitSwitch: `NDrone2_Enable(0)` (object flags |= 0x30: not drawn; drone flags cleared); msg 3 polls
  `switch_channels[+0x134]` → `Enable(1)` → the start state; msg 0xE enables at once.
- [L] 07000005 at level start: 8 of 13 drones wait (channels 0x22, 0xA4, 0x43), 4 are Idle/Patrol, 1 spawner-held.
- Properties at creation (first block `+0x4DC`, active through `+0x4D8`): `DoModeSettingsNEW` clears 0x1A (sets 0x1A
  and 0x56 on 07000009/0A; untouched on 07000007); `NDrone2_DefaultInit` (single player) clears 0x17; on 07000005/06
  SG5 guards (parameter 9 = 0x16) get 0, 1, 6, 0x41, 0x42, lose 0x13 and get engage range 10. [L] matches the
  level-start snapshot for all 12 compared drones. `FirstAttack` can switch to the second block (`+0x4E8`) when
  `+0x5A4` ≠ 0; the gate guard did not switch in a live fight.

## Combat movement states [M]

- `DroneFunc_CombatState` (0x148338) routes to the opponent on every call (`MoveToObject(min(engage, 6), follow =
  not from 0x68/0x70)`); "routed" = result < 4 or 10. From 0x68/0x70 the out-of-range exit needs a route; from 0x58:
  prop 0x40 → 0x59, out of range → 0x5A, unseen → 0x5C (> 15 ticks) or 0x5F, else 0x5B; 0x5E faces and aims, routed →
  0x58. Returns 0 while the anim is in a transition.
- `NDrone2_MoveToObject` (0x151618): verify / compute the route, 3 when the route distance < `+0x8EC` (2.0), else
  follow (results < 2 → 0; 2, 3, 0xC kept; others → 0).

- **CombatOutOfRange 0x5A**: msg 3 → `CombatState`: `MoveToObject(min(range, 6), opponent)`; prop 0x40 / `+0x4F8 &
  0x10` → 0x59; cover → 0x8C; in range → 0x58; moving → `SetCombatMoveAnim` (07000005/06) or 0x1E; 3/9/B/C → face
  and aim; 5 → 0x7F; 10 → nothing; else → 0x5F.
- **CombatNoSight 0x5C**: visible → 0x5B; unseen > 15 ticks → move as above toward the opponent; else face, anim 10
  or 0x21.
- **CombatNoRoute 0x5F**: anim 0x21, 1 s timer, fire; msg 3 faces the opponent (not visible → a combat move or 0x21;
  visible → anim 10 and fire). Timer: cover → 0x8C; else alternately `MoveToObject(2)` (moving → 0x58) or
  `SetGoalToDroneWithLosToOpponent` → 0x63; fallback 0x67 (50 %, in range) / `SetGoalToCloserPosition` → 0x63 /
  anim 0x21 and re-arm.
- **DroneStuck 0x7F**: → 0x5F on enter; msg 3 → 0x56 when the route's target frame is older than `DAT_00299024`
  (only ever 0).
- **GoToGoalPosition 0x63**: `MoveToGoalPosition`: moving → 0x1E; blocked → aim / 0x21; arrived or failed → the
  return state `+0x5A2`.

## Unknowns

U-030: record words +0x84..+0xA0, the extra bytes, cel AABBs, `Intersect_Portal`, route+2, `+0x664`,
`DAT_00299024`, `SetGoalToCloserPosition` / `SetGoalToDroneWithLosToOpponent`, animation speeds.
