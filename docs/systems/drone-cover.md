# Drone cover

Sources: ACTION.ELF decomp and disassembly (research pass, WORKLOG 0021); placements on the disc. [M] = read in
the code or bytes, [S] = inferred. Two decompiler artefacts: `DroneAnim_CallAnim`'s real argument order is
(anim state, flag, next state, …); `NDrone2_FindCover` stores `CoverNodeOK`'s side (dropped by Ghidra; disassembly
0x152770). Port: `crates/nf-game/src/cover.rs`.

## Cover nodes [M]

Placements with flags word 0xE5 (`Drone_CoverCornerNode`) and 0xE6 (`Drone_CoverLowNode`), created at level load
into `CoverNodes` (0x283E10, 256 × 0xB0). Position = placement translation, yaw = rotation y.

| Field | Corner (type 0) | Low (type 1) |
|---|---|---|
| +1 / +2 | switch channel that must be on / off (p4 / p5; 0 = none) | same |
| +3 / +4 | – | idle / aim anim state |
| +5 / +6 | side-1 depth, side-2 offset × 0.1 (p6 / p7, 0 → 8) | – |
| +8 | A = p3° (0 → 45°) | – |
| +0xC flags | p0: 1 → 8 stand only, 2 → 4 crouch only, else 0xC; p1: 1 → 0x20 step-out, 2 → 0x40 lean, else 0x60; p2: 1 → 0x80 side 2, 2 → 0x100 side 1, else 0x180; 0x10 occupied | p0 = 2 → 1 (+3 / +4 from u16 {0x11, 5, 0xA, 7, 0x1D}[p1 / p2]); else +3 = 0x11 and +4 = 5 (p0 = 1) or 0xA; p3 = 1 → 2 |
| +0x14 | owner | owner |
| +0x40 | emitter: per nav node a path-distance byte (`AINetwork_EmitPath`, 0xFF = unreachable) | same |
| +0x90 | per-target usability bits (8 targets) | same |

07000005: 18 corner nodes, all p = (0, 0, 0, 46, 0, 0, 8, 8) (both stances, both kinds, both sides, A = 46°).

## Usability [M]

`Drone_ProcessCoverNodes` runs every 5 frames (from the first): for each target slot (Bond in single player) it walks
the nodes from a shared round-robin cursor and stops after the first node that cast a line-of-sight ray.
`Drone_IsCoverNodeUsable(t, node)` clears the bits, then:
1. switch gates fail → unusable;
2. thresholds: low f14 = 45°, f13 = 30° (80° / 75° on 0x0700000D); corner f13 = 0.66·A, f14 = A + 5°; when occupied,
   low +5° each, corner 0.77·A and A + 10°;
3. target within 4 → bits 0x80, unusable (no ray);
4. diff = wrap(yaw to target − node yaw): low: |diff| ≤ f14 → 0x48, else 0x200. Corner side 1 (flag 0x100):
   π/2 − f13 < diff < π/2 + f14 → 2 (| 0x10 when diff < π/2); side 2 (flag 0x80): diff > π − f14 or diff < f13 − π →
   4 | 0x20; else 0x200 (no ray);
5. ray from the target's head to node + (0, 1.5, 0) (1.15 on 0x07000011); not visible → bits & ~0x70 | 0x400.

## Choosing cover [M]

- `NDrone2_CoverAvailable(needLos)`: prop 0x30 = 0 → no; already claimed (`+0x4F8 & 0x800`) → yes; opponent visible
  and closer than 4 → no; else `FindCover`.
- `NDrone2_FindCover`: the drone's nav node n (`AINetwork_NodeSearch`, limit 625); over unoccupied nodes on the same
  network with d = distance[n] ≠ 0xFF: the **smallest d** with d² ≤ (distance to the opponent)² and
  `CoverNodeOK(node, 0, needLos)` ≠ 0. Claims it: owner, flag 0x10, side, approach point =
  `GetPosForCornerCover(approach 1)` (goal radius 0.5), in-cover point (approach 0), `+0x4F8 |= 0x1800`.
- `CoverNodeOK(node, side, needLos)`: an opponent; bits & 0xE; with needLos bits & 0x70; distance ≥ 2.0 (and ≤ 1.5 ×
  combat range with prop 0x44); corner: one cover anim possible (crouch lean / crouch step / stand lean / stand step /
  as-low; crouch only when forced) and the side's bit (2 → side 1, 4 → side 2; 0 / 3 picks); low: prop 0xF and bit 8,
  side 3; no other drone within 1.5 of the node unless already claimed.
- Anim predicates (the anim set must have the state): crouch lean = (0x10 or 0xF) and flag 4, (0xA, 0xB or 0xD), flag
  0x40, anim 0x13; crouch step = (0x10 or 0xF) and 4, 0xC, 0x20, anim 0x15; stand lean = (0xA, 0xB or 0xD) and 8,
  0x40, anim 0x17; stand step = 0xC and 8, 0x20, anim 0x1B; forced crouch = prop 0x10 or (corner without 8); as-low
  = low node or crouch-only corner.
- `GetPosForCornerCover(node, side, approach)`: node + x·(cos y, −sin y) + z·(sin y, cos y): side 1 x = 0.1 (approach
  −0.7), z = −depth (−0.8); side 2 x = offset (0.8), z = −0.1 (approach +0.7); low x = 0, z = −0.3 (approach −1.1).
- `SetAngleToCover`: low node or as-low → face the opponent; side 1 → node yaw − π/2; side 2 → node yaw.

## States [M]

Entry: `DroneFunc_CombatState` → 0x8C when `CoverAvailable(1)`, from AimStandFire / AimCrouchFire (after the
reload, out-of-range and no-sight checks, before grenades and combat moves) and from 0x5A / 0x5C / 0x5F (unless
`+0x4F8 & 0x10` or prop 0x40). Timers C/D are cleared on every state change.

| State | Behaviour |
|---|---|
| RunForCover 0x8C | Update: node no longer OK, or opponent visible within 4 → 0x56. `MoveToGoalPosition` to the approach point: moving → combat-move anim, sliding 10 %/tick to the in-cover point once the route has < 1 left; arrived (route < 2.0) → 0x93; blocked → face, aim or 0x21; else 0x56. Exit (not to 0x93): disown. |
| UnderCoverInit 0x93 | `CoverAnim(6)` → 0x94; slide 8 %/tick (xz) to the in-cover point, face the cover angle. |
| UnderCoverIdle 0x94 | `CoverAnim(1)`; refill the clip; timer C in FRand(150) ticks (not with prop 0x10). Update: slide, face; node not OK or > 1.5 from the goal → 0x9C. Ambushers (prop 0x10) wait until the opponent is visible or within 3 → 0x9C. Timer C: Rand(5) = 0 and another cover → 0x8C; node OK with LOS → 0x95; other cover → 0x8C; else retry up to 3 times, then 0x56. |
| UnderCoverAim 0x95 | `CoverAnim(2)` (lean / step-out) → 0x96. |
| UnderCoverFire 0x96 | Fire. Update: node not OK (needLos) → 0x99 (leans) or 0x9C; aimed at and Rand(3) = 0 → 0x99; visible → fire. |
| UnderCoverReturn 0x99 | `CoverAnim(4)` → 0x9A (fail → 0x9B); node not OK or > 1.5 from the goal → leave after the anim. |
| UnderCoverTypeChange 0x9A | leave flag → 0x9C; node not OK and other cover → 0x8C; `CoverAnim(7)` → 0x94. |
| UnderCoverLeave 0x9B | → 0x9C. |
| UnderCoverLeaveNow 0x9C | disown; anim 0xA → 0x56. |
| CrouchCover 0x6E | not node cover: crouch (0x11) 2–3 s → 0x67 (drone type 0xC and crouch modes). |
| AmbushInit 0x37 / AmbushWait 0x38 | drone type 0x10: `FindAmbushCover` (nearest node within 2.0) → 0x93, else crouch and wait until seen / within 3 → 0x56. |

`DroneAnim_CoverAnim(mode, next)`: cover type (0 crouch lean, 1 crouch step, 2 stand lean, 3 stand step, 4 as low)
× column (0 enter, 1 idle, 2 aim, 4 return, 5 change) from `CCModes` (0x270768):

| Type | Columns 0..5 (side 1 / side 2 where they differ) |
|---|---|
| CrouchLean | 10 10 13/14 58 10 1A |
| CrouchStep | 10 10 15/16 58 10 1A |
| StandLean | 1A 1A 17/18 58 1A 1A |
| StandStep | 1A 1A 1B/1C 58 1A 1A |
| AsLowCover | 11 11 0A 58 11 0A |

Mode 6 picks the type: forced crouch → crouch lean / step by predicate, else stand lean / step; none → as-low for a
crouch-only corner, else fail; Rand(n) among the candidates. Mode 7 (type change): Rand(2) ≠ 0 → no change; else
candidates as mode 6 (distance > 8 also allows the crouch types); plays column 5 of the old type. Other modes: that
column. Low nodes: modes 0/6/1/4 play +3, 2 plays +4, 3 plays 0x58.

## Live check [L] (WORKLOG 0023)

work/chase_trace.tsv: the gate guard (61.34, 99.50), Bond 19 units away at (79.86, 22.92, 96.0): Combat (65) →
OutOfRange (66) → RunForCover (67; strafe 0xC, back-off 0x4, strafe 0xD around the gate pillar) → UnderCoverInit (265,
stand-cover loop 171) → UnderCoverIdle (296, clip refilled) → UnderCoverAim (394, lean-out 176 at speed 1.5, then loop
182) → UnderCoverFire (431) at corner node 1 (59.11, 98.69, yaw −90°), position (58.93, 99.45) — 0.28 from the
computed in-cover point (59.21, 99.49) (U-031). The port matches the sequence, node, UnderCoverInit / Aim durations and
the end position (0.2); RunForCover is shorter in the port (U-030).

## Sniper cover (0x97/0x98) [M] (WORKLOG 0079)

`UnderCoverFire` ENTER (0x16a3e0) branches to `UnderCoverSniperFire` (0x97) when the drone's anim set `+0xDA == 0x13`
(sniper) AND `level < 0x0700000E` (the test level 07000005 qualifies). On later levels the sniper fires through
`UnderCoverFire` directly. State 0x97: timer C at +1 s (fires), timer D at +6 s (returns to 0x99); clip empty → 0x98
(reload anim 0x52, refill `+0xBBC = +0xBBE`, back to 0x97). `OpponentIsAimingAtMe(7.5)` + `BondIsFacingMe(45)` +
`Rand(3) == 0` → duck to 0x99. Ported in `drone.rs::handle_cover`.

## Ambush (0x37/0x38) [M] (WORKLOG 0079)

Drone mode 0x10 (`NDrone2_init_DMODE_Ambush`) sets the start state `+0x5A2 = 0x37` (AmbushInit). `NDrone2_FindAmbushCover`
(0x152958): the unoccupied cover node within **2.0 units** of the drone whose anim predicates pass; claims it the same
way `FindCover` does → 0x93 (UnderCoverInit). Without one: `CallAnim(0, 0x11, 0, 0, …)` (crouch) → 0x38 (AmbushWait),
which waits until the opponent is visible (`+0x228 & 4`) or within 3.0 → 0x56 (Attack). Ported in `drone.rs::find_ambush_cover`
and `handle_cover`. `DroneFunc_ConsiderExplosive` (msg 0x21): an explosive landed near the drone → stores `+0x330` /
`+0x350` and routes to `RecoverFromScaryObject` (0x86); the flee route update is G-026.

## 07000005 [M]

18 corner nodes (`nf-extract covers`), all reachable through NavPath A's emitter.

Cover prop 0x30 sits in the alert set (parameters 23–25), which governs after `FirstAttack`. 8 SG5 guards have {0xA,
0xC, 0xF, 0x30}; the snipers have 0x40 and no 0x30; two guards have an empty alert set. With default nodes they use
stand lean / stand step (0x17/0x18/0x1B/0x1C, idle 0x1A).

## Unknowns

`+0x2E`, `+0x38`, `+0x4C0`, `+0x4F8 & 0x10000`, the meaning of low-node flag 2, UnderCoverFire's clip handling, the
units of `OpponentIsAimingAtMe` / `BondIsFacingMe` (U-031).
