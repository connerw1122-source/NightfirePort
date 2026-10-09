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
(no path → Idle). Idle/Alert/Patrol on msg 3 call `DroneVision_EnemyLookForOpponent`, which returns Attack (0x56) when
the opponent is visible (`+0x228 & 4`) and prop 0x3C is set and (alertness ≥ 0.66 or prop 0x51). Attack →
`FirstAttack` (alertness 1.0) → AimStand (0x67, prop 0x18, in range) or Combat (0x58) → CombatNewSighting (0x5B) →
AimStandFire (0x68) ⇄ reload (0x69), moves (0x73–0x7A), NoSight (0x5C), OutOfRange (0x5A), NoOpponent (0x60).

07000005's guards (`nf-extract drone-props`): all take the NEW behaviour path (parameter 5 = 0x64); normal block
alertness 0 or 1 (property 0x20 → {0, 0, 0.66, 1.0} at 0x2f5cc8, verified); all have 0x3C and 0x51.

## Perception [M, L]

- Opponent in single player: the player (`NDrone2_FindOpponent`); geometry each tick (`Drone_GetOpponentInfo`):
  `+0x1A0` distance, `+0x1C4` = `ATAN2_APPROX(dx, dz)`.
- Visibility `+0x230` (`DroneVision_OpponentVisibility`): 1 under 2 units; else R = e8·(2·alert + 1),
  vis = 0.7 − (dist − R)/R, × sin(margin/e4 · π/2) with margin = e4 + 22.5° − |head yaw − yaw to target| (0 outside),
  × player multiplier (AI volume boxes, × 0.75 crouching). Sight constants by class (parameter 14): e8 24 (classes
  1/2/3/4/5: 12/16/40/1000/0), e4 = π/2; on attack e8 = max(e8, fc) and e4 = 0 (all round).
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

Not yet (G-026): first-sight / combat moves (props 0x19, 0x1B; steps, strafes, rolls, crouch, cover — the traced
guard sidestepped), patrol paths, hearing and noise, drone-to-drone alerts, head tracking and the aim bone in the
pose, bullets and damage to Bond, AI volumes, the game's random generator.

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
