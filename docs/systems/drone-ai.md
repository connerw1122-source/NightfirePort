# Drone (guard) AI

Sources: ACTION.ELF decomp, read by four research passes (state machine, perception, combat/firing, turning/movement;
WORKLOG 0019) and spot-checked by me; live PCSX2 traces (`tools/trace/ai_trace.py`). [M] = read in the code or bytes,
[S] = inferred, [L] = matched against a live trace. Port: `crates/nf-game`.

## State machine [M]

- State-machine info at `Drone+0x108`: +4 current (mirrored to `obj+0xF4`), +8 previous, +0xC next, +0x14 time the
  state was entered, +0x18 change-pending byte. `NDrone2_StateFuncs` (0x29b300): 250 state functions
  `(DCVars*, Drone*, obj*, msg*) → handled`. State 0 (`Global`) handles what the current state does not.
- `Drone_SM_SetState` only records the next state; `Drone_SM_RouteMsgDCV` applies it after the message: msg 2 (exit)
  to the old state, then msg 1 (enter) to the new one, repeating while enters keep changing state.
- Messages: 1 enter, 2 exit, 3 per-frame update (sent from `NDrone2_ControlSTANDARD`), 4 idle timeout, 5 animation
  completed (the 5th `DroneAnim_CallAnim` argument), 0xC / 0xD timers (`+0xCC4` / `+0xCD0` …), 0xF first sight,
  0x11–0x16 alerts, 6–9 / 0x17–0x19 impacts, 0x1D forced state, 0x1E alarm, 0x21 explosive near.
- Per frame (`control_movement_object_handler`): global `Drone_InitComms` (vision for all drones, delayed messages),
  then per drone `Drone_Control`: `NDrone2_PreDroneControl` (timers → messages) → `NDrone2_ControlSTANDARD`
  (alertness decay; **msg 3**; find opponent; tracking; move; collision; `DroneWeap_HandleFiring`) →
  `DroneAnim_SnapRotate` → animation → `DroneAnim_CallHandler` (completion messages).

## Start-up and the path to attack [M]

Global (enter) → WaitSwitch (1) / PlayScript (3) / `Drone+0x5A2` = Idle (4), or InitPatrol (6) with props 0x24/0x25
(no path → Idle). WaitSwitch ENTER calls `NDrone2_Enable(0)` [M 0x15b960:14-17 → 0x149dd0:8-15: `obj+0xF0 |= 0x10|0x20`
(not drawn, not picked — Collide_PickObj 0x1eae80:26-30, View_AddObjects 0x1e62d8:33), `+0x4F8 &= ~0x1801A0` (no anim/AI)] —
**a ch-waiter is an invisible statue until its channel fires** (its UPDATE polls the live channel, 0x15b960:21-37; the
wake re-enables and starts 0x32/3/`+0x5A2`).

The freeze does not stop the poll: the per-frame wake message is the drone's own — `NDrone2_ControlSTANDARD` 0x1490f0:81
self-sends **msg 3** every frame, and `Drone_Control` 0x139268 gates only on `NPCGlobals`/`Drone_bDisableSystem`/the MP
slot, never on the enable flag — so a frozen waiter still samples `switch_channels[+0x134]` from its own control run every
frame. (The dispatcher is one function per state: `Drone_SM_InitObject` 0x175838 wires `NDrone2_ProcessStateMachine`
0x175778 at smi+0x28, which indexes `NDrone2_StateFuncs[state]`; an unconsumed message retries with state index 0 —
`NDrone2_DSTATE_Global` — which is the port's `handle(state)` → `handle(GLOBAL)` fallback.)

Two facts pin the entry and the re-entry. `NDrone2_DefaultInit` 0x14b300:466-468 latches `+0x134` = the placement's
switch channel and `+0x136` = **`switch_channels[+0x134]` at creation time**; `NDrone2_DSTATE_Global` 0x15b830 msg-1
sends a channel-armed drone to WaitSwitch only while `+0x136 == 0` — the snapshot, not the live channel — so a drone
created with its channel already on skips WaitSwitch entirely. And the states a wake lands in consume msg-1 (Patrol
0x15c498 case 1 falls to `caseD_0` → returns 1), which is why Global's msg-1 WaitSwitch entry does not re-fire on the
post-wake state change. `NDrone2_Enable(1)` (the wake's re-enable) proceeds only while `+0x4F8 & 0x200` is CLEAR and
restores `+0x4F8 |= 0x801A0` + `obj+0xF0 &= ~0x30` [M-dis 0x149dd0-0x149e4c — the decompile's masks and its
`0x4f8 & 0x200 == 0` early-return are inverted]. A level NIS freezes every drone the same way: `Drone_EnableAll(0)` walks the
llist calling `NDrone2_Enable(0)` per drone and clears `NPCGlobals` [M 0x1383e8:13-23]; `Script_Free`'s 0x80 flag
re-enables all EXCEPT state-1 drones [M 0x1da880:41-43, 0x1383e8:14], and `Drone_CollisionHandler` returns while
`NPCGlobals == 0` — frozen drones take no damage [M 0x140968:62]. Idle/Alert/Patrol on msg 3 call
`DroneVision_EnemyLookForOpponent`, which returns Attack (0x56) when
the opponent is visible (`+0x228 & 4`) and prop 0x3C is set and (alertness ≥ 0.66 or prop 0x51). Attack →
`FirstAttack` (alertness 1.0) → AimStand (0x67, prop 0x18, in range) or Combat (0x58) → CombatNewSighting (0x5B) →
AimStandFire (0x68) ⇄ reload (0x69), moves (0x73–0x7A), NoSight (0x5C), OutOfRange (0x5A), NoOpponent (0x60).

07000005's guards (`nf-extract drone-props`): all take the NEW behaviour path (parameter 5 = 0x64); normal block
alertness 0 or 1 (property 0x20 → {0, 0, 0.66, 1.0} at 0x2f5cc8, verified); all have 0x3C and 0x51.

### Drone modes and the sniper [M, L] (WORKLOG 0037)

- `behaviour_util_get` (`DoModeSettingsNEW`): placement parameter 15 = type `+0xC7`, 16 = the normal block's mode
  `+0xC8`, 17 = its count / size (3 | 0x5B << 16), 18–20 its words; 21 = the alert block's mode `+0xC9`, 22, 23–25.
- `NDrone2_GetDroneTypeAttackTypeFriend` (0x14cbd8): equal block modes (or tag 0x65) drop the second tag `+0x13A`;
  mode `+0xC5` = `GetDTYPENEW(type, +0xC8)` (block 0 + property 0x10 → 0x10; types 10–16 special; block 2 → 2 (sniper)
  or 0x19 at alertness 1, 3 → 7 / 0xE, 4 → 4, 5 → 0x15); alternate mode `+0xC6` (0 without a tag, or without a second
  switch at full normal alertness); team `+0x44` (types 10, 11, 13, 14 → 3, 12 → 2, else 1). `DroneTypeSettings`
  (0x29aee8, 85 × 12: start, alternate, init function): the start state (−1 → by mode: 2 → 0x29 / 0x2A, default 6 / 4,
  …), at alertness 1 the attack versions; the init function (`initDTYPE_Sniper` sets `+0x4F8 & 0x10`, stationary).
  `+0x5A4` = the alternate mode's (or the mode's) alternate state; without an alternate mode, a second switch or full
  alert-block alertness maps the start state (4/5/6/8/… → 0x56, 0x29 → 0x2A, …); then a missing tag becomes 100.
- `FirstAttack` with `+0x5A4` and no second switch: the alternate mode (2 → 0x19 + `initDTYPE_SniperAlert`) and the
  alternate state.
- Sniper states: SniperIdle 0x29 (anim 0x0E; look for Bond; alerts; hit → SniperAim, no flinch), SniperAim 0x2A (anim
  0x0A, sight and combat range 1000; Bond seen → SniperFire), SniperFire 0x2B (fire after a 60-tick timer, then
  `DroneWeap_Fire` each tick; empty clip → SniperReload; 16 ticks unseen → SniperAim; hit → flinch allowed), SniperReload
  0x2C (20 ticks, then reload anim 0x52 → SniperFire; the clip refills on exit). [L] 07000005's sniper (spawner 266
  picks placement 7 or 267 at random — both mode 2, special-death scripts 0x060003DF / 0x06000A1C): mode 2, alternate
  0, team 1, `+0x5A2` 0x29, `+0x5A4` 0, tag 0, stationary, first block [0x81003C84, 0xAC700785, 0x40280448] — the port
  matches all of these (`level_start_matches_the_game`). Not modelled: `CanAltAttack` (0x72), `FindElevatorDoor` /
  `OpenDoor`, the 07000007 property writes in `GetDTYPENEW`.

## Perception [M, L]

- Opponent in single player: the player (`NDrone2_FindOpponent`); geometry each tick (`Drone_GetOpponentInfo`):
  `+0x1A0` distance, `+0x1C4` = `ATAN2_APPROX(dx, dz)`.
- Visibility `+0x230` (`DroneVision_OpponentVisibility`): 1 under 2 units; else R = e8·(2·alert + 1),
  vis = 0.7 − (dist − R)/R, × sin(margin/e4 · π/2) with margin = e4 + 22.5° − |head yaw − yaw to target| (0 outside),
  × player multiplier (AI volume boxes, × 0.75 crouching). Sight constants by class (parameter 14): e8 24 (classes
  1/2/3/4/5: 12/16/40/1000/0), e4 = π/2; on attack e8 = max(e8, fc) and e4 = 0 (all round).
- AI volumes [M, L] (WORKLOG 0032): placements 0xF5 / 0xF8 → `Drone_AIVolume_Create` (0x1376c0) → `AIBoxList`
  (0x2706c8; nodes +4 next, +8 object (matrix `obj+0x90`), +0x10 half-extents, +0x20 factor = param 0 × 0.01, +0x24
  kind from param 1: 0 → 0, 1 → 2, 2 → 1, 4 → 3; +0x26 / +0x28 / +0x2C params 4 / 2 / 3). Half-extents = |(glist max
  − min)/2| × the placement's scale (+0x34; the glist bounds are the entity params' +0x18 min / +0x24 max). On
  07000005 (or param 1 = 4) the rotation's z is negated and the position lifted by 1.5 × scale y. The object matrix is
  Rx·Rz·Ry (row vectors) — fitted to the live matrix, the builder not read. `Drone_ProcessOpponents` (0x1390c0):
  `+0x22C` = the product of the kind-0 factors of the boxes holding Bond (`Intersect_PointOOBox`), × 0.75 crouching;
  `OpponentVisibility` multiplies by it unless the drone has alert bits `+0x4FC` and alertness ≥ 1. [L] 07000005:
  eight `AI_ShadowVolume` boxes (two 0.25, six 0 around the start corner); Bond's start is in a 0.25 one, so a guard
  44.6 units away at alertness 0.75 does not see him (visibility ≈ 0.24 < 0.7). One more box in the live list
  (153.75, 17.6, 90.86, factor 0) is not from the main map's placements (U-035). Port: `nf_game::aivolume`
  (test `ai_volumes_match_the_game` against the snapshot).
- Line of sight (`DroneVision_ProcessDroneSight`): one drone per tick, round robin, only if vis ≥ 0.7; head to
  target bone (`Drone_View_Bones` cycle 2, 3, 5, …). Counters: seen ticks `+0x270`, unseen `+0x274`.
- Reaction (`DroneFunc_ReactionTime`): on levels 0x7000002/3/5/9/A `(1 − alert)·0.3·(200 − stat_b8)` ticks
  (0 once seen or at alertness 1), else 10. [L] 07000005: `drone_stats` for the guards' type is zero → 60 ticks;
  the game noticed Bond 61 ticks after he appeared (round-robin scheduling adds the tick).
- Alertness decays 1/50 per second toward its floor (never from 1.0); sight never raises it; noise does
  (`Sound_Alertness` per tick, weights from `SFXOutputData`), and so do impacts and alarms. Drone-to-drone alerts are
  visual (`FindAlertedDrones`, 1 s after the source attacks, within 50 units and the observer's view); the shout
  broadcasts are dead code in this build (`ShoutFromOtherDrone` always returns 0).

## Combat moves [M, L] (WORKLOG 0020)

- `NDrone2_ChooseCombatMove` (0x150c18): no move with prop 0x40; "aimed at" = Bond's crosshair on the drone
  (`Check_Target` every 6 ticks, held 30 → `+0x2A`), "bunched" = another drone within 3 units. Rand(15) ≥ 7 with
  neither → none. Options (picked at random, each case falling through to the next when it fails, left/right order
  random): dodge (aimed, prop 0x13; 0x77/0x78), strafe (aimed, prop 0x41; 0x75/0x76), roll (aimed, prop 0x2E;
  0x79/0x7A), step (prop 0x42; 0x73/0x74), crouch (not aimed, prop 0x11, distance ≥ 6, Rand(2) = 0; 0x6F), stand
  (not aimed, Rand(4) ≠ 0; 0x67). Final filter: no stand from 0x68, no crouch from 0x70, not the current state.
- First sight (`FirstSightState`): prop 0x1B → 50 % a combat move; else prop 0x19 → a combat move.
- From 0x68 (`DroneFunc_CombatState`): reload when the clip is empty; NoSight after 16 unseen ticks; once the aim
  loop is playing, a combat move. From 0x70: after 2 s in the state.
- Space checks (`NDrone2_Can*`): local offsets x = left: step / strafe 1.8, dodge 3.6, roll 5.2, back-off z −1.8;
  the game tests AI-network boundaries (`NDrone2_MoveTest`) — *deviation*: nf-game tests a floor within 4 units and no
  collision between at knee and chest height.
- Moves are root motion (`DroneAnim_PreTransCallback` clips it at boundaries); steps end with their animation, strafes
  and dodges on a 2–5 s / 2–3 s timer; strafes fire while moving.
- [L] 07000005 guards with Bond aiming at them have only the step: the trace shows Attack → StepAimRight (101 ticks)
  → AimStandFire (burst) → StepAimLeft (113 ticks) …; later, with the crosshair off, AimCrouch → AimCrouchFire. The
  port: Attack → step (112 ticks) → burst → step …

## Cover [M, L] (WORKLOG 0023/0079; order + tests fix-search-cover-g028)

Full node/usability/anim tables: docs/systems/drone-cover.md. The selection chain, with addresses:

- **Tick order**: `Drone_InitComms` (0x139520) refreshes the cover-node usability FIRST — `Drone_ProcessCoverNodes`
  (0x138e78: every 5th frame, counter −2 / < 1 → 10; the 8 target slots share one round-robin cursor over the
  nodes and each slot's walk stops at the first node whose check casts a sight ray, so only the crossed nodes
  get fresh bits and the rest keep their previous ones) —
  then `Drone_ProcessOpponents` (0x1390c0) and `DroneVision_ProcessDroneSight` (0x176a28). The pass's inputs
  (switch channels, the target-slot position snapshot taken at :127-149, the collision world, the node owners)
  are not written by the passes in between, so the order is a conformance fact rather than a data dependency.
- **Choosing**: `DroneFunc_CombatState` (0x148338:83,143,211,238) calls `NDrone2_CoverAvailable(needLos=1)`
  (0x155a38: prop 0x30 off → no; already claimed `+0x4F8 & 0x800` → yes; opponent visible and closer than 4 →
  no) → `NDrone2_FindCover` (0x152548): over the unoccupied nodes on the drone's nav network with an emitter
  distance byte ≠ 0xFF, the **smallest byte, strictly less (first wins ties)**, only with byte² ≤ (opponent
  distance)² and `NDrone2_CoverNodeOK(node, 0, needLos)` ≠ 0 — 0x151d68: a usable side (bits & 0xE), the LOS
  bits (& 0x70) when needLos, the node ≥ 2.0 from the opponent (≤ 1.5× combat range with prop 0x44), the anim
  predicates, and no other drone within 1.5. The claim writes the owner (+0x14, flag 0x10), the side (+0xBB8),
  the approach point (`NDrone2_GetPosForCornerCover` 0x152028, approach 1) as the goal and the in-cover point
  (approach 0) to +0x830, `+0x4F8 |= 0x800 | 0x1000`; `DisownCover` 0x155858 clears them.
- **Stance**: the corner flags (+0xC: 4 crouch, 8 stand, 0x20 step-out, 0x40 lean) and the guard's anim set
  decide the cover type in `UnderCoverInit`'s `CoverAnim(6)` (0x13c078; crouch forced when prop 0x10 or a
  corner without 8; else Rand(n) among the candidates), and the type fixes crouch vs stand for the episode
  (only 0x9A's `CoverAnim(7)` changes it). Low nodes (flag bit 0) are as-low cover for guards with prop 0xF.
- **Enter / leave**: Combat → OutOfRange/NoSight → `RunForCover` 0x8C → `UnderCoverInit` 0x93 → `Idle` 0x94 ⇄
  `Aim` 0x95 → `Fire` 0x96; leave via `Return` 0x99 / `TypeChange` 0x9A / `Leave` 0x9B → `LeaveNow` 0x9C
  (disown) → Attack. State behaviours: docs/systems/drone-cover.md's table. RNG inside the episode, in game
  order: the idle re-arm `FRand(150)`, the idle timer's `Rand(5)` re-cover roll (before the LOS re-aim check),
  the fire state's `Rand(3)` duck roll, and `CoverAnim(6/7)`'s `Rand(n)` type draw.
- **Port**: `cover.rs` (nodes, usability, positions, `CCModes`) + `drone.rs` (`find_cover`, `cover_node_ok`,
  `cover_available`, the 0x8C–0x9C / 0x97 / 0x98 / 0x37 / 0x38 arms). The usability walk now sits before the
  opponents/vision passes as in the game (the port models single-player target slot 0 only — the game walks
  all 8 slots, unallocated ones cast nothing). Tests: `cover_selection_live.rs` (the scoring rule — a
  gate-less replication picks node 3 over the trace's node 1, so the side/LOS gates are load-bearing — and
  the stance flags both ways), `ai_live.rs::guard_takes_cover_like_the_game` (the live chase trace).

## Bullets [M, L]

`DroneWeap_FireWeapon` → `DoBulletAccuracy` (hit roll; misses aim at a deterministic wobble) → `Bullet_init`
(projectile from the muzzle = weapon datum id + 1; cone ±0.0014·spread) → `Bullet_update` (speed per tick, first step
(0.5 + FRand 0.5)·speed; collision ray from the old position for max(step, 1) units; dies past the range) →
`Collide_Update` (nearest hit; Bond via 16 bone boxes — *deviation*: a cylinder r 0.25, 1.8 tall) →
`Player_HandlePain`: damage `weapon_data+0x0C` × `Drone+0x100` (×2 under 4 units) × the level's player damage factor
(07000005–08: 0.6 / 0.7 / 1.0 by difficulty), armour first, health from 100. Drone bullets draw `GUN_BULLET_TRAIL`
(0x02000123) stretched by their speed. [L] each SG5 hit on Bond in 07000005 costs exactly 1.65 (= 2.75 × 0.6); a
sniper hit 18 (= 30 × 0.6); the traced guard hit with ~37 of 42 shots, the port ~32 of 45.

## Port status (nf-game, WORKLOG 0019)

Ported: state machine and messages; Global/Idle/InitPatrol(→Idle)/Attack/Combat/NewSighting/AimStand/AimStandFire/
reload/NoSight/NoOpponent; visibility, round-robin line of sight against the collision world, reaction time,
`EnemyLookForOpponent`; firing (bursts, intervals, burst delay, Ready2Fire, clip, reload); turning (Steer 0.1) and an
upper-body aim angle; animation calls through `DroneAnimStates` / `Drone_AnimInfo` follow-ons. Checked against the
live trace (`crates/nf-game/tests/ai_live.rs`): Attack at tick 61, 3-shot bursts 4 ticks apart.

Since then: combat moves and bullets (WORKLOG 0020), drone-to-drone alerts and the upper-body aim, patrols and
routes (0021), combat movement, start-up properties and WaitSwitch (0022): docs/systems/drone-navigation.md.
Cover — selection, states and the tick order — is ported and documented above and in docs/systems/drone-cover.md
(WORKLOG 0023/0079, fix-cover-remainder, fix-search-cover-g028); grenades (0080), AI volumes (0032), head
tracking and the aim bone (fix-ai-remainder), deaths and hit reactions are ported too. Still open from
G-026/27/28: hearing beyond the sound-alert chain, per-target cover slots (only Bond is modelled), the
`RecoverFromScaryObject` (0x86) flee route update and U-030's RunForCover route shape.

## Firing [M, L]

`DroneWeap_HandleFiring` / `DoFiring` / `NextBulletTime` / `BurstDelay`: a burst of `weapon_data+0x2C` shots
(SG5 3, L96A1 1), `+0x40` frames apart (SG5 4, L96A1 120); between bursts D + interval, D from aggression `+0xB5`
(k = 1.66/1.33/1.0/0.66/0.33): `max(15, trunc(45·k·(0.5 + FRand 0.5)))`, ≤ 60, = 15 under 4 units, = 60 beyond 30.
Clip 64 (SG5) / 5 (L96A1); reload state when empty. [L] 07000005 trace: 3-shot bursts 4 frames apart, gaps 64 and 51.

Accuracy (`DoBulletAccuracy`): p = 100 − 5·`+0xB4`, × modifiers (too close ×2, target first moved ×0.25, moving ×0.75,
per-level difficulty table); misses add a deterministic wobble; every shot in the first 2 s of continuous sight
(`+0x270` < 120) misses unless closer than 3. Bullets are projectiles (SG5 15.35 units/frame, range 450).

## Turning and movement [M]

Desired yaw `Drone+0x694 = ATAN2_APPROX(dx, dz)`; per frame `yaw += Vec_AngleDifference(yaw, desired) · 0.1`
(`NDrone2_Steer`, `0.1 / FRAME_RATE_DIV`), with obstacle avoidance when moving. Snap turns (90° / 180° animations)
add ±π/2 or π when the animation ends. Walking is root motion; collision: feet sphere r = 0.4 at `pos.y − A0 + r`.
Forward = local +Z. Head tracking bone 5 (yaw ±1.1, pitch −1.0…0.6, speed 0.07 / 0.14), upper-body aim bone 2
(±0.9, speed 0.2).

## Maths [M]

`ATAN2_APPROX` (0x1e4528), `PS2Sinf` (0x120700), `Vec_AngleDifference` (0x1e2b78), `Ang_RotateToBearing` (0x1e0f10):
`crates/nf-game/src/trig.rs`.
