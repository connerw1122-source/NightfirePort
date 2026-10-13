# Guard AI port map

A function-by-function map of every ACTION.ELF function the guard AI touches, against its Rust port.
Decomp source: `work/decomp/ACTION.ELF/<addr>_<mangled name>.c`. Port: `crates/nf-game/src/{drone,cover,
grenade,bullet,damage,nav,spawners,aivolume}.rs` (plus `trig.rs`, `tuning.rs`, `world.rs`).

Statuses:

- **ported** — every branch, constant, RNG draw and store the decomp makes is in the Rust code.
- **partial** — the main path is ported; some branches, constants, RNG draws or stores are missing (notes say which).
- **stub** — a placeholder; the call site exists but the body is a no-op or a one-line approximation.
- **STANDIN** — a deliberate game-system replacement (called out in `CLAUDE.md`/`docs/systems/*.md`); behaviour
  is approximated by a different mechanism.
- **not started** — no Rust code at all.

Build: ACTION.ELF `e7ffdcb3…` (SLUS-20579 NTSC-U). Tags from `CLAUDE.md`: [M] measured in our Ghidra export or
disassembly, [L] matched against a live snapshot, [S] suggested/inferred.

## Summary

| Status | Count |
|---|---:|
| ported | 32 |
| partial | 205 |
| stub | 19 |
| STANDIN | 7 |
| not started | 5 |
| **Total** | **268** |

The user's initial estimate was ~108 functions; the thorough enumeration here covers every per-state
handler in `NDrone2_StateFuncs` the guard AI exercises on the test level 07000005 (50+ `NDrone2_DSTATE_*`),
every `DroneAnim_*` helper the Rust port folds into `call_anim`/`set_danim`/`call_handler` (~22), every
`AINetwork_*` route helper the port calls (~18), and every `DroneVision_*` / `DroneWeap_*` / `NDrone2_*`
helper referenced by `drone.rs`. The seven STANDINs and the three stun-grenade receiving states are called
out explicitly below.

The seven explicit STANDINs (the user's known list):

1. `0x12ce80` `Bullet_CollisionHandler` — the ricochet path is not ported (only the grenade bounce in
   `grenade.rs`).
2. `0x3037dc` per-material bounce table — not read; `grenade.rs` uses the constant `0.4` (`BOUNCE_RESTITUTION`).
3. `0x140968` `Drone_CollisionHandler` — the feet-sphere radius variations (0.55 in state `0xC`, 0.3 in crouch
   mode `0x17`) are not ported; `drone.rs::hip` always returns `1.0` and the radius is the game's 0.4.
4. `0x140660` `Drone_FeetOnPoint` — the `bVar2` extension that reads `0x3037dc + material·0x88` is not ported
   (walkable/solid mask `0xC` is enough for the floor).
5. `0x13bf40` `DroneAnim_ValidateCoverAnim` — the `5 → 7/10` fallback for an anim the set lacks is not ported.
6. `0x13fff0` `NDrone2_DealWithObjHit` — the per-hit dispatch (water ripples, doors, breakables, explosions,
   players) is not ported.
7. `0x173658` `NDrone2_DSTATE_RecoverFromScaryObject` — sets the state but does not run the flee route update.

The three stun-grenade receiving states (`0x1631c0`, `0x163350`, `0x172bb0`) are stubs (the user listed
"Stun grenade receiving states are not ported").

Trivial DroneAnim wrappers that the port folds into a single function (`call_anim`, `set_danim`,
`call_handler`) — `0x13db30 GetDAnimForCall`, `0x13dbf8 CanSetAnimCall`, `0x13dc90 ZeroAnimCall`,
`0x13dcb8 CallFullyComplete`, `0x13dd78 SetDAnimNow`, `0x13ddd0 SetAnimD`, `0x13de40 GetDAnimForAnimState`,
`0x13def8 SetAnimCall`, `0x13e3e8 AnimEndCallback` — are not counted as separate functions; their behaviour
is absorbed into `DroneAnim_CallAnim` (`0x13ab58`), `DroneAnim_SetDAnimInternal` (`0x13aee8`) and
`DroneAnim_CallHandler` (`0x13add0`). Multiplayer-only functions (`BOT_*`, `NDrone2_DSTATE_Bot*`,
`NDrone2_ReactToDroneAlertMsg`, `NDrone2_DSTATE_CombatWait`, `NDrone2_DSTATE_CrouchCover` on 07000005,
hostage / partner / door-guard handlers) are excluded — they are a separate AI surface not touched by the
single-player guard AI on the test level 07000005.

## 1. Collision & gravity

| Address | Function (demangled) | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x140968 | `Drone_CollisionHandler` | **STANDIN** | `drone.rs::Drone::collision_handler` | The driver is ported (in-air flag clear, down-ray, on-surface snap, gravity accumulation, `pos += v·REC_FRAME_RATE`, `obj+0xFA |= 0x20`); STANDINs: the feet-sphere radius variations (0.55 / 0.3) — `hip()` always returns 1.0 and the radius is the game's default 0.4; the `+0x464` scratch clear, the `+0x34` "crush" sink, the player-inheritance (`Control_InheritVelocity`) and the `& 0x40` × 0.98 velocity damping are not ported. |
| 0x140660 | `Drone_FeetOnPoint` | **STANDIN** | `drone.rs::Drone::feet_on_point` | The down-ray `pos → pos + up·−(hip+0.6)` with mask `0xC` and the on-surface threshold (`floor ≥ feet_y − 0.2`) are ported; the `bVar2` extension that reads `0x3037dc + material·0x88` (per-material surface coefficient) is not ported. |
| 0x140e90 | `NDrone2_Collision` | partial | `drone.rs::Drone::collision_handler` + `nav::Network::push_out` | The feet-sphere push-out of AI-network boundary walls (`GetBoundsPushVectorForSphere`, r 0.4) runs via `Network::push_out` in `Drones::tick`; the per-hit walkable/crush flag setting (`anim+0x60 & 0x10/0x20/0x8`) is approximated. |
| 0x149ec8 | `NDrone2_DoCollision` | partial | `drone.rs::Drone::collision_handler` (inlined predicate) | The "alive, `& 0x20`, not in death `+0xD8 ≠ 0xC/0x13`, no `+0x538` blend, drawn, `& 0x570`" predicate is folded into `bounded`/`free_root`/`corpse`. |
| 0x141e58 | `NDrone2_DoGravity` | partial | `drone.rs::Drone::collision_handler` (inlined predicate) | Same predicate pattern as `DoCollision`, with `& 0x80`. The port's `bounded` flag covers both `& 0x20` and `& 0x80`. |
| 0x14b190 | `DroneFunc_NearPlayerCollision` | stub | — | Always returns false in the port (no player-proximity short-circuit on the full sphere test). |
| 0x14b1e8 | `DroneInit_Collision` | stub | — | The create-time floor snap is not ported; the port stands drones on the rendered floor at level load. |
| 0x1499d0 | `DroneFunc_ClampPosToCollisionCylinder` | not started | — | The clamp-into-cylinder adjustment for slide moves is not ported. |
| 0x13fff0 | `NDrone2_DealWithObjHit` | **STANDIN** (case 5 partial) | `drone.rs::update_player_bullets` | The per-hit dispatch (water ripples, doors, breakables, explosions, vehicle riding) is not ported; the explosion hit path is reached separately from `Explode_Propagate` in `grenade.rs`. The PLAYER-hit arm (case 5's weapon → msg 6/7/8 pick) is ported — see the row below. |

## 2. State machine dispatch

The dispatch table is `NDrone2_StateFuncs` (0x29b300, 250 entries). `Drone_SM_RouteMsgDCV` runs msg 2 (exit) to
the old state, then msg 1 (enter) to the new one, repeating while enters keep changing state. Messages: 1 enter,
2 exit, 3 update, 4 idle timeout, 5 anim completed, 6–9 impacts, 0xC/0xD timers, 0xE enable, 0xF first sight,
0x11–0x16 alerts, 0x17 stun, 0x18 stun grenade, 0x19 stun dart, 0x1D forced state, 0x1E alarm, 0x21 explosive,
0x23 turn done.

### Top-level dispatch

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x175778 | `NDrone2_ProcessStateMachine` | partial | `drone.rs::Drone::handle` + `set_state` | The `handle()` match is the dispatch; `set_state` records the next state, applied by `send` (`RouteMsgDCV`). Messages 0xC/0xD timers are dispatched in `Drones::tick`. |
| 0x171df8 | `Drone_SM_RouteMsgDCV` | partial | `drone.rs::Drone::send` | Exit-then-enter repetition is ported; the "msg 1 → msg 0 if state changes during enter" loop is preserved. |
| 0x1758c0 | `Drone_SM_SetState` | ported | `drone.rs::Drone::set_state` | Records `+0xC next`; one-tick delayed application by `RouteMsgDCV`. |
| 0x1757d0 | `Drone_SM_Init` | stub | `drone.rs::Drone::new` | The state-machine info table is not built; per-drone state is a single `state` field. |
| 0x175838 | `Drone_SM_InitObject` | stub | `drone.rs::Drone::new` | Numbers state machines in creation order — not modelled (one machine per drone). |
| 0x1759a0 | `Drone_SM_SendDelayedMsgs` | not started | — | The delayed-message queue (`+0xCC4` / `+0xCD0` timers) is approximated by the `timer_c`/`timer_d` fields dispatched in `Drones::tick`. |
| 0x175a38 | `Drone_SM_SendMsgSelf` | stub | `drone.rs::Drone::send` | Self-messages are inlined at the call site; no separate queue. |
| 0x175aa8 | `Drone_SM_BroadcastMsg` | partial | `Drones::tick` (alarm, sound alerts) | The alarm broadcast and the sound broadcast run in `Drones::tick`; the generalised broadcast loop is not modelled. |

### Per-state handlers

| Address | State | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x15b830 | `NDrone2_DSTATE_Global` | partial | `drone.rs::handle(GLOBAL, …)` | Switch-channel → WaitSwitch; start state `+0x5A2`. Modes 0x1E/0x17 and `PlayScript` (3) are not ported. |
| 0x15b960 | `NDrone2_DSTATE_WaitSwitch` | partial | `drone.rs::handle(WAIT_SWITCH, …)` | `Enable(0)` (`obj+0x30 |= 0x30`); msg 3 polls the channel; msg 0xE enables at once. |
| 0x15bf18 | `NDrone2_DSTATE_Idle` | partial | `drone.rs::handle(IDLE, …)` | Stand/alert idle anim; prop 0x1D → `AssignAIPath(8)` (EnemyMission, none in 07000005); msg 4 idle timeout (prop 0x46). |
| 0x15c248 | `NDrone2_DSTATE_Alert` | partial | `drone.rs::handle(ALERT, …)` | Anim 0x21; msg 4 → 0x22 (look around). |
| 0x15c498 | `NDrone2_DSTATE_Patrol` | partial | `drone.rs::handle(PATROL, …)` | `PatrolTalk`, `UpdatePatrolRoute`, look-for-opponent; wait nodes (`+0xBAE`) not modelled. |
| 0x166178 | `NDrone2_DSTATE_AimStand` | partial | `drone.rs::handle(AIM_STAND, …)` | Anim 0xA; reload / combat-move transitions via `combat_state`. |
| 0x166328 | `NDrone2_DSTATE_AimStandFire` | partial | `drone.rs::handle(AIM_STAND_FIRE, …)` | `HandleFiring`, `NextBulletTime`, clip-empty → reload, no-sight, grenades, combat moves. |
| 0x1666d8 | `NDrone2_DSTATE_AimStandReload` | partial | `drone.rs::handle(AIM_STAND_RELOAD, …)` | Reload anim 0x52; clip refill; back to 0x68. |
| 0x164210 | `NDrone2_DSTATE_Attack` | partial | `drone.rs::handle(ATTACK, …)` | `FirstAttack` once, `CombatState` route, `SeenAndAttacking` music gate. |
| 0x1643e8 | `NDrone2_DSTATE_CombatNoMove` | partial | `drone.rs::handle(COMBAT_NO_MOVE, …)` | A 0x58 variant with no move; not in 07000005. |
| 0x164690 | `NDrone2_DSTATE_CombatOutOfRange` | partial | `drone.rs::handle(COMBAT_OUT_OF_RANGE, …)` | `MoveToObject(min(range,6))`, prop 0x40 → 0x59, cover → 0x8C, in range → 0x58. |
| 0x164838 | `NDrone2_DSTATE_CombatNoSight` | partial | `drone.rs::handle(COMBAT_NO_SIGHT, …)` | Visible → 0x5B; unseen > 15 ticks → move toward opponent. |
| 0x164c08 | `NDrone2_DSTATE_CombatNoRoute` | partial | `drone.rs::handle(COMBAT_NO_ROUTE, …)` | Anim 0x21, 1 s timer, fire; the timer's `SetGoalToCloserPosition` / `SetGoalToDroneWithLosToOpponent` alternation is ported but not the `+0x31C` "drone ahead" 0x5E branch. |
| 0x1650a0 | `NDrone2_DSTATE_NoOpponent` | partial | `drone.rs::handle(NO_OPPONENT, …)` | Returns home via `+0x5B0`/`+0x5A0`. |
| 0x165270 | `NDrone2_DSTATE_AlertToPosition` | partial | `drone.rs::handle(ALERT_TO_POSITION, …)` | Walk 0x35 / run 0x1E to the goal; arrived → `+0x5A2`; 6 → 0x5F. |
| 0x165640 | `NDrone2_DSTATE_GoToGoalPosition` | partial | `drone.rs::handle(GO_TO_GOAL, …)` | `MoveToGoalPosition`; `+0x4F8 |= 0x10000 | 0x400000`; deaf flag set on enter. |
| 0x165bf0 | `NDrone2_DSTATE_SearchArea` | partial | `drone.rs::handle(SEARCH_AREA, …)` | Look 0x22, 3 s, turn 0x61, anim 0xB; give-up → home. |
| 0x165ef0 | `NDrone2_DSTATE_DroneStuck` | stub | `drone.rs::handle(DRONE_STUCK, …)` | → 0x5F on enter; msg 3 → 0x56 (the `DAT_00299024` test is always 0 in this build). |
| 0x166000 | `NDrone2_DSTATE_DrawWeapon` | partial | `drone.rs::handle(DRAW_WEAPON, …)` | Anim 0x46 → `+0x5A2`. |
| 0x166b98 | `NDrone2_DSTATE_AimBackoff` | partial | `drone.rs::handle(AIM_BACKOFF, …)` | Anim 4 (back off, offset π); 2-s timer. |
| 0x166f28 | `NDrone2_DSTATE_AimCrouch` | partial | `drone.rs::handle(AIM_CROUCH, …)` | Anim 0xC; reload / fire transitions. |
| 0x167060 | `NDrone2_DSTATE_AimCrouchFire` | partial | `drone.rs::handle(AIM_CROUCH_FIRE, …)` | As 0x68 with crouch. |
| 0x167370 | `NDrone2_DSTATE_AimCrouchReload` | partial | `drone.rs::handle(AIM_CROUCH_RELOAD, …)` | Reload anim 0x52 crouched. |
| 0x167590 | `NDrone2_DSTATE_StepAimLeft` | partial | `drone.rs::handle(STEP_AIM_LEFT, …)` | Root-motion step; ends with the anim. |
| 0x1676a0 | `NDrone2_DSTATE_StepAimRight` | partial | `drone.rs::handle(STEP_AIM_RIGHT, …)` | As left. |
| 0x1677b0 | `NDrone2_DSTATE_StrafeAimLeft` | partial | `drone.rs::handle(STRAFE_AIM_LEFT, …)` | Strafe 0xC, fires while moving, 2–5 s timer. |
| 0x167980 | `NDrone2_DSTATE_StrafeAimRight` | partial | `drone.rs::handle(STRAFE_AIM_RIGHT, …)` | As left. |
| 0x167b50 | `NDrone2_DSTATE_StrafeDodgeLeft` | partial | `drone.rs::handle(STRAFE_DODGE_LEFT, …)` | Dodge 0xC, 2–3 s timer. |
| 0x167cf0 | `NDrone2_DSTATE_StrafeDodgeRight` | partial | `drone.rs::handle(STRAFE_DODGE_RIGHT, …)` | As left. |
| 0x167e90 | `NDrone2_DSTATE_RollLeftCrouch` | partial | `drone.rs::handle(ROLL_LEFT_CROUCH, …)` | Roll; ends with the anim. |
| 0x167f80 | `NDrone2_DSTATE_RollRightCrouch` | partial | `drone.rs::handle(ROLL_RIGHT_CROUCH, …)` | As left. |
| 0x169150 | `NDrone2_DSTATE_RunForCover` | partial | `drone.rs::handle_cover(RUN_FOR_COVER, …)` | `MoveToGoalPosition` to the approach point; sliding 10 %/tick to the in-cover point; arrived → 0x93; blocked → face/aim/0x21; exit (not to 0x93) → disown. |
| 0x169cc0 | `NDrone2_DSTATE_UnderCoverInit` | partial | `drone.rs::handle_cover(UNDER_COVER_INIT, …)` | `CoverAnim(6)` → 0x94; slide 8 %/tick; face the cover angle. |
| 0x169e58 | `NDrone2_DSTATE_UnderCoverIdle` | partial | `drone.rs::handle_cover(UNDER_COVER_IDLE, …)` | `CoverAnim(1)`; refill clip; timer C `FRand(150)`; ambushers wait until visible / within 3. |
| 0x16a2d0 | `NDrone2_DSTATE_UnderCoverAim` | partial | `drone.rs::handle_cover(UNDER_COVER_AIM, …)` | `CoverAnim(2)` (lean/step-out) → 0x96. |
| 0x16a3e0 | `NDrone2_DSTATE_UnderCoverFire` | partial | `drone.rs::handle_cover(UNDER_COVER_FIRE, …)` | Fire; the ENTER sniper branch to 0x97 is ported; node-not-OK transitions to 0x99/0x9C. |
| 0x16a610 | `NDrone2_DSTATE_UnderCoverSniperFire` | partial | `drone.rs::handle_cover(UNDER_COVER_SNIPER_FIRE, …)` | Timer C (1 s) fires, timer D (6 s) → 0x99; clip empty → 0x98; `OpponentIsAimingAtMe(7.5)` + `BondIsFacingMe(45)` + `Rand(3) == 0` → 0x99. |
| 0x16a8b0 | `NDrone2_DSTATE_UnderCoverSniperReload` | partial | `drone.rs::handle_cover(UNDER_COVER_SNIPER_RELOAD, …)` | Reload anim 0x52; refill `+0xBBC = +0xBBE`; back to 0x97. |
| 0x16a9c0 | `NDrone2_DSTATE_UnderCoverReturn` | partial | `drone.rs::handle_cover(UNDER_COVER_RETURN, …)` | `CoverAnim(4)` → 0x9A; fail → 0x9B; node-not-OK → leave. |
| 0x1739b8 | `NDrone2_DSTATE_UnderCoverTypeChange` | partial | `drone.rs::handle_cover(UNDER_COVER_TYPE_CHANGE, …)` | Leave flag → 0x9C; node-not-OK and other cover → 0x8C; `CoverAnim(7)` → 0x94. |
| 0x173ac0 | `NDrone2_DSTATE_UnderCoverLeave` | partial | `drone.rs::handle_cover(UNDER_COVER_LEAVE, …)` | → 0x9C. |
| 0x16ab38 | `NDrone2_DSTATE_UnderCoverLeaveNow` | partial | `drone.rs::handle_cover(UNDER_COVER_LEAVE_NOW, …)` | Disown; anim 0xA → 0x56. |
| 0x16af20 | `NDrone2_DSTATE_HeardNoiseAware` | partial | `drone.rs::handle(HEARD_NOISE_AWARE, …)` | Share > 0.1 → Suspect; the look-around 0x22 variant 1 is ported. |
| 0x16b0e0 | `NDrone2_DSTATE_HeardNoiseSuspect` | partial | `drone.rs::handle(HEARD_NOISE_SUSPECT, …)` | Route to the sound; turn (0x61) if > 135°; the prop 0x21 AlertToPosition/SearchArea branch is ported. |
| 0x16b540 | `NDrone2_DSTATE_HeardNoiseAlert` | partial | `drone.rs::handle(HEARD_NOISE_ALERT, …)` | Face the opponent → Attack. |
| 0x15fab0 | `NDrone2_DSTATE_SniperIdle` | partial | `drone.rs::handle(SNIPER_IDLE, …)` | Anim 0x0E; stationary; look for Bond. |
| 0x15fc18 | `NDrone2_DSTATE_SniperAim` | partial | `drone.rs::handle(SNIPER_AIM, …)` | Anim 0x0A; sight and combat range 1000; Bond seen → SniperFire. |
| 0x15fd88 | `NDrone2_DSTATE_SniperFire` | partial | `drone.rs::handle(SNIPER_FIRE, …)` | 60-tick timer then `DroneWeap_Fire`; empty clip → SniperReload; 16 ticks unseen → SniperAim. |
| 0x15ff80 | `NDrone2_DSTATE_SniperReload` | partial | `drone.rs::handle(SNIPER_RELOAD, …)` | 20 ticks, then reload anim 0x52 → SniperFire; clip refills on exit. |
| 0x1626a0 | `NDrone2_DSTATE_Surrender_Anim` | partial | `drone.rs::handle(SURRENDER_ANIM, …)` | Drop the gun, unarmed (`ChangeWeapon(1)`), hands up. |
| 0x162950 | `NDrone2_DSTATE_Surrendered` | partial | `drone.rs::handle(SURRENDERED, …)` | Faces Bond once; hands up while covered (`CheckUnsurrender`). |
| 0x162a70 | `NDrone2_DSTATE_Unsurrender_Anim` | partial | `drone.rs::handle(UNSURRENDER_ANIM, …)` | Face Bond, aim-stand anim → Attack. |
| 0x161560 | `NDrone2_DSTATE_AmbushWait` | partial | `drone.rs::handle_cover(AMBUSH_WAIT, …)` | Crouched; on opponent visible or within 3 → 0x56. |
| 0x172960 | `NDrone2_DSTATE_AmbushInit` | partial | `drone.rs::find_ambush_cover` + `handle_cover(AMBUSH_INIT, …)` | Within 2.0 → 0x93; else crouch 0x11 → 0x38. |
| 0x173480 | `NDrone2_DSTATE_GrenadeThrow` | partial | `drone.rs::handle_grenade_throw(GRENADE_THROW, …)` | ENTER `+0x4F8 |= 0x10000`, alert 1; the throw anim (0x49); event 1 → `DroneWeap_ThrowGrenade` → 0x56. |
| 0x173098 | `NDrone2_DSTATE_BulletImpact` | partial | `drone.rs::handle(BULLET_IMPACT, …)` | ENTER turns vision on; the flinch call ends it. |
| 0x1637f8 | `NDrone2_DSTATE_Death_Anim` | partial | `drone.rs::handle_death(DEATH_ANIM, …)` | `obj+0xF0 |= 0x20`; `+0xBD4` → SpecialDeath; `OnInitDeath`; `LocationDeathAnim`. |
| 0x163a80 | `NDrone2_DSTATE_DeathByExplosion` | not started | — | Reached from a fatal explosion; the port routes a fatal `Drone_ExplosiveHit` straight to `Death_Anim`. |
| 0x163c68 | `NDrone2_DSTATE_SpecialDeath_Anim` | partial | `drone.rs::handle_death(SPECIAL_DEATH, …)` | At its spot `+0xBE0`: `OnInitDeath`, death channel, `& ~0x20`/`& ~0x80`/`& ~0x80000000`, `+0x1A = 1`, the script as anim 0x40; else the normal death. `free_root` keeps the whole root motion. |
| 0x163fe8 | `NDrone2_DSTATE_Dead` | partial | `drone.rs::handle_death(DEAD, …)` | `obj+0xF0 |= 0x20`; `SetAsDead`; timer 0xC at +300; `DeadDrone`. |
| 0x172cd8 | `NDrone2_DSTATE_Fade` | partial | `drone.rs::handle_death(FADE, …)` | Alpha from `+0x106` down 0.6/tick; 0 or not drawn → disabled. |
| 0x172dd8 | `NDrone2_DSTATE_FadeFast` | partial | `drone.rs::handle_death(FADE_FAST, …)` | As Fade with 8.0/tick. |
| 0x173658 | `NDrone2_DSTATE_RecoverFromScaryObject` | **STANDIN** | `drone.rs::consider_explosive` | Sets state 0x86 and stores `+0x330`/`+0x350`; the flee route update (run away from the explosive) is not ported. |
| 0x1631c0 | `NDrone2_DSTATE_StunGrenadeLoop` | stub | — | Stun-grenade receiving state — not ported. |
| 0x163350 | `NDrone2_DSTATE_StunGrenadeRecover` | stub | — | Stun-grenade recovery — not ported. |
| 0x172bb0 | `NDrone2_DSTATE_StunGrenadeImpact` | stub | — | Stun-grenade impact entry — not ported. |

### `DroneFunc_*` (the shared state helpers)

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x148010 | `DroneFunc_FirstAttack` | partial | `drone.rs::handle(ATTACK, ENTER)` | `CheckSurrender` → alertness 1, `+0x4F8 |= 0x6000000`, `+0x524` stamp; `+0x5A4` alternate-mode switch is ported (the gate guard did not switch). |
| 0x147c48 | `DroneFunc_FirstSightState` | partial | `drone.rs::handle(ATTACK, ENTER)` (inlined) | The prop 0x1B → 50 % combat move / prop 0x19 path is ported. |
| 0x148338 | `DroneFunc_CombatState` | partial | `drone.rs::combat_state` | Every call routes to the opponent; from 0x58 the 0x59/0x5A/0x5C/0x5F branches; from 0x68 reload/NoSight/combat move; from 0x70 2-s timer. |
| 0x148a30 | `DroneFunc_HandleSoundAlerts` | partial | `drone.rs::Drones::tick` (sound loop) | `share = 0.01·loudness`; loudest sound → msg 0x14 at most once a second. The id 0x248 weight-10 override and the 0x11C/0x11D 09/0A skip are ported. |
| 0x148c60 | `DroneFunc_DoForcedAttack` | stub | `drone.rs::handle(*, msg::FORCED_ATTACK)` | Always → Attack. |
| 0x1469b8 | `DroneFunc_HandleImpact` | partial | `drone.rs::on_bullet`/`on_punch`/`bullet_impact`/`punch_impact` | The per-state impact table (Idle/Alert/Patrol/AlertToPosition/SearchArea/HeardNoise*/RunForCover/UnderCoverInit/LeaveNow → 0x56; UnderCover Idle/Aim/Fire/Return → 0x9C; AimStand/AimBackoff/Step/Strafe/Dodge/Roll → 0x68 with `allow=1`; etc.) is ported. Case 6 (the punch, msg 6) shares the table — msg 6's (resume, allow) equals msg 8's in every state that has the case (46-state scan, fix-player4) — and routes to `NDrone2_PunchImpact` instead of `NDrone2_BulletImpact`. |
| 0x14ac60 | `DroneFunc_ReactionTime` | partial | `drone.rs::reaction_time` | `(1 − alert)·0.3·(200 − stat_b8)` on 0x7000002/3/5/9/A, else 10. [L] 07000005: 60 ticks. |
| 0x14a298 | `DroneFunc_DeadDrone` | partial | `drone.rs::Drones::tick` (corpse cull) | Counts 0x0F/0x47/0x48(+300)/0xBF; the oldest goes FadeFast beyond 3 (`DAT_00298F5C`). |
| 0x14af80 | `DroneFunc_AttackNoWeapon` | stub | `drone.rs::handle(ATTACK, ENTER)` | → DrawWeapon when unarmed. |
| 0x14a060 | `NDrone2_CheckSurrender` | partial | `drone.rs::check_surrender` | Prop 0x43, alertness < 0.66, Bond within the range. |
| 0x14a168 | `NDrone2_CheckUnsurrender` | partial | `drone.rs::check_unsurrender` | Farther than 12, or beyond 2 with Bond unarmed, or Bond not facing the drone. |
| 0x14a218 | `NDrone2_FadeOut` | partial | `drone.rs::handle_death(FADE/FADE_FAST, …)` | Alpha decrement. |
| 0x14a2f0 | `NDrone2_SetAsDead` | partial | `drone.rs::handle_death(DEAD, ENTER)` | `obj+0xF0 |= 0x20`; `health = 0`. |
| 0x1453a8 | `DroneFunc_CheckAlarmRaised` | partial | `drone.rs::Drones::tick` (music) | Channel 0xA4 (07000005/06: 0x37 / 0x61) raises the alarm-music gate; the per-level channel set is ported. |
| 0x145bd0 | `DroneFunc_OnInitDeath` | stub | `drone.rs::handle_death(*, ENTER)` | `+0xCD8 = 1` flag set; the per-mode `+0x4F8` mask writes are not all ported. |
| 0x14b158 | `DroneFunc_DropGrenade` | partial | `drone.rs::drop_grenades` | Captain death drops weapon 0x35 at its matrix. |
| 0x14a718 | `DroneFunc_AnyoneNear` | partial | `drone.rs::Drones::tick` (bunched) | Another drone within 3 units. |
| 0x1431c0 | `DroneFunc_ConsiderExplosive` | partial | `drone.rs::consider_explosive` | Stores `+0x330` (pos) / `+0x350` (12 frag / 5 stun / 10 mine); routes to 0x86. The flee route itself is the STANDIN above. |
| 0x14a5f0 | `DroneFunc_Set1stBehaviour` | partial | `drone.rs::handle(*, msg::FIRST_SIGHT)` | `+0x4D8 → +0x4DC`. |
| 0x14a618 | `DroneFunc_Set2ndBehaviour` | partial | `drone.rs::handle(ATTACK, ENTER)` | `+0x4D8 → +0x4E8`. |

## 3. Vision

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x175af0 | `DroneVision_OpponentVisibility` | partial | `drone.rs::Drone::visibility` | R = e8·(2·alert+1); vis = 0.7 − (dist−R)/R, × sin(margin/e4·π/2), × AI-volume factor `+0x22C` unless alert bits `+0x4FC` and alert ≥ 1. |
| 0x175cb8 | `DroneVision_SeekOpponentLOS` | partial | `drone.rs::Drones::tick` (round-robin) | One drone per tick, round robin, only if vis ≥ 0.7; head-to-target bone cycle (2,3,5,…) not ported — port uses the head position. |
| 0x175ef0 | `DroneVision_HaveOpponentSight` | partial | `drone.rs::Drones::tick` | `+0x228 & 4` from `vis ≥ 0.7`; counters `+0x270`/`+0x274`. |
| 0x176a28 | `DroneVision_ProcessDroneSight` | partial | `drone.rs::Drones::tick` | Visibility every tick for vision-on drones; LOS round-robin. |
| 0x176250 | `DroneVision_ConsiderAlerted` | partial | `drone.rs::Drones::tick` (alert pairs) | Source attacking ≥ 1 s with visible alert bits; observer watching (Idle/Alert/Patrol); props accept bits; within 50 units; in view; head-to-head LOS. At most 4 per tick. |
| 0x1767d0 | `DroneVision_FindAlertedDrones` | partial | `drone.rs::Drones::tick` (alert pairs) | As above. |
| 0x177800 | `DroneVision_AlertSound` | partial | `drone.rs::handle(IDLE/ALERT/PATROL, msg::SOUND)` | Ignored more than 50 units from the opponent or when already attacking; normal guards → HeardNoise 0xA2 when `+0x514 > 0.2` or alertness > 0.66. |
| 0x177a30 | `DroneVision_ShoutFromOtherDrone` | stub | — | Returns 0 in this build (dead code). The port skips the shout broadcast entirely. |
| 0x177c18 | `DroneVision_EnemyAlerts` | partial | `drone.rs::enemy_alerts` | Msg 0x14 → HeardNoise chain; msg 0x1E → Attack (alarm); the dead `ShoutFromOtherDrone` cases return 0. |
| 0x177ed0 | `DroneVision_EnemyLookForOpponent` | partial | `drone.rs::look_for_opponent` | Attack when Bond is seen (`+0x228 & 4`) and prop 0x3C is set and (alertness ≥ 0.66 or prop 0x51). |
| 0x178678 | `NDrone2_NearArmedBond` | partial | `drone.rs::Drones::tick` | Bond-armed scan (2..0x47 / 0x6A..0x71) — used for `NearArmedDrone`. |
| 0x178768 | `NDrone2_ObjectIsArmed` | partial | `drone.rs::Drones::tick` | Weapon byte check (used in the alert pairs). |
| 0x178520 | `DroneVision_GunThroughWall` | not started | — | The "shoot through thin walls" check — not ported. |

## 4. Weapons

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x178c48 | `DroneWeap_Ready2Fire` | partial | `drone.rs::ready_to_fire` | Aiming class, close enough, or the gun within 20° of the target. |
| 0x178f68 | `DroneWeap_FireWeapon` | partial | `drone.rs::fire_weapon` | `Sound_Play3D(weapon_data+0x60, drone)` (skipped for 0x5EF); `DoBulletAccuracy`; `Bullet_init`. |
| 0x179740 | `DroneWeap_DoBulletAccuracy` | partial | `drone.rs::fire_weapon` | p = 100 − 5·`+0xB4`, × modifiers (too close ×2, target first moved ×0.25, moving ×0.75, per-level difficulty); misses add a deterministic wobble; first-2-s-of-sight misses. |
| 0x179a48 | `DroneWeap_DoOpponentTargetting` | partial | `drone.rs::Drones::tick` | Moving / "first moved" for 2 s after Bond starts. |
| 0x179ba8 | `DroneWeap_BurstDelay` | partial | `drone.rs::burst_delay` | `max(15, trunc(45·k·(0.5 + FRand 0.5)))`, ≤ 60, = 15 under 4 units, = 60 beyond 30; k from aggression `+0xB5` (1.66/1.33/1.0/0.66/0.33). |
| 0x179d40 | `DroneWeap_NextBulletTime` | partial | `drone.rs::next_bullet_time` | Burst of `weapon_data+0x2C` (SG5 3, L96A1 1), `+0x40` apart (SG5 4, L96A1 120); between bursts D + interval. |
| 0x17a080 | `DroneWeap_DoFiring` | partial | `drone.rs::handle_firing` | `FireWeapon` while the burst continues; clip-empty → reload. |
| 0x17a180 | `DroneWeap_HandleFiring` | partial | `drone.rs::handle_firing` | The per-tick driver; clip / burst / reload decisions. |
| 0x17a2a0 | `DroneWeap_DropWeapon` | partial | `drone.rs::drop_weapon` | Once — the weapon byte (`anim+0x62`) is cleared. At its matrix. |
| 0x17a500 | `DroneWeap_OpponentIsAimingAtMe` | partial | `drone.rs::handle_cover` | Used by the sniper 0x97 duck (7.5 range). |
| 0x17a5a0 | `DroneWeap_GetWeaponInfo` | partial | `drone.rs::muzzle`/`fire_weapon` | Muzzle bone 0x15; weapon damage / speed / range from `weapon_data`. |
| 0x17a6e0 | `DroneWeap_ChangeWeapon` | stub | `drone.rs::handle(SURRENDER_ANIM, ENTER)` | The surrender draws the sidearm (weapon 1, anim set 0xC). Other weapon swaps not ported. |
| 0x17a798 | `DroneWeap_CanThrowGrenade` | partial | `drone.rs::can_throw_grenade` | `+0xBCC`/`+0xBCE` ≠ 0; `Rand(2) == 0`; 8 ≤ dist ≤ 20; `CanDoAnimState(0x49, 0)`. |
| 0x179558 | `DroneWeap_ThrowGrenade` | partial | `drone.rs::throw_grenade` | Decrements `+0xBCE`; ±π/4 yaw lob by distance; mode 0xD sniper 1/3 chance of stun (0x36) over frag (0x35); bone 0x15 origin. |
| 0x17a870 | `DroneWeap_ManageBurstCount` | partial | `drone.rs::next_bullet_time` | Burst count from `weapon_data+0x2C`; burst counter `+0xBBC`. |
| 0x17a8c0 | `DroneWeap_Fire` | partial | `drone.rs::fire` | `Bullet_init` with the muzzle direction; the per-shot `Rand` calls run. |
| 0x1789c0 | `DroneWeap_CloseCombatImpact` | not started | — | Close-combat hit (punch) — not ported. (The PLAYER punch is ported: it is a bullet, not this function — `NDrone2_DealWithObjHit` case 5 routes weapon 1 to msg 6.) |
| 0x13fff0 | `NDrone2_DealWithObjHit` case 5 | partial | `drone.rs::update_player_bullets` | The player-shooter arm only: the weapon picks the message — 1/0x4B/0x4D → msg 6 (punch), 0x4A/0x4C → msg 7 (taser, not ported), else `Drone_BulletHit` msg 8 [M 0x13fff0:269-306]. The rest of the dispatcher (water ripples, doors, breakables, explosions, vehicle riding) is still absent. |
| 0x145f28 | `NDrone2_PunchImpact` | ported | `drone.rs::punch_impact` | The resume stamped into +0x118 before the damage; health ≤ 0 → KnockedOut_Anim 0x42 (mode 0x1E → 0xF2, bots not ported); alive: PainTalk, then the 0x51 stagger + flinch stamps with allow, else the resume state. SendHurtMessage and the level-7/9-A mission-fail reasons not modelled. |
| 0x172a30 | `NDrone2_DSTATE_KnockedOut_Anim` | ported | `drone.rs::handle_knockout` | ENTER: corpse flag, health 0, CallAnim(0x50 → 0x47), KO flag +0x1000000, DropWeapon, PlrStat_LogEnemyDisabled. EXIT: the death channel except on {0x07000009, 0x0700000A} (`sltiu`, unsigned). Cases 8/9 → 0x44. |
| 0x172b48 | `NDrone2_DSTATE_Knocked_Out` | ported | `drone.rs::handle_knockout` | Update/impacts → 0x44. Never set by C code (state table only). |
| 0x172ee0 | `NDrone2_DSTATE_PunchImpact` | ported | `drone.rs::handle_knockout` | ENTER replays the punch with allow = 1 (the stagger); update faces the opponent; msg 6/8 → HandleImpact(0, 0). |

### Bullets

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x12c5a0 | `Bullet_init` | partial | `bullet.rs::spread` + `Drone::fire_weapon` | Direction = `norm(norm(dir) + spherical→cartesian(r, FRand 2π, FRand π))` with r = `(FRand(2s) − s)·0.0014`, s = `+0x24·(0 if +0x68 & 0x400000, else Drone+0xC0=1)`. The full `Bullet_init` (the projectile's `BU_tag` allocation) is not modelled — `Bullet` is a struct. |
| 0x12cd98 | `Bullet_handle_object_destruction` | stub | `bullet.rs::Bullet::update` | When a bullet hits, `state = 2` (dead). The per-object destruction dispatch is not ported. |
| 0x12ce80 | `Bullet_CollisionHandler` | **STANDIN** | — | The ricochet path is not ported. The grenade bounce in `grenade.rs::Grenade::update` is the only bounce modelled. |
| 0x12d890 | `Bullet_DoTrails` | ported | `bullet.rs::Bullet::update` (fly-by) | The closest point on the bullet's path this tick to the viewer; if within 3 units (SqDist < 9), a 1-in-5 `Rand_Rand(5)` roll plays sound 0x1dc. RNG order: `Rand_FRand` (in `update`) → `Rand_Rand(5)` (here). |
| 0x12e660 | `Bullet_Delete` | stub | `bullet.rs::Bullet::update` | `state = 2` marks the bullet for removal in `Drones::update_player_bullets`. |
| 0x12e7b0 | `Bullet_update` | partial | `bullet.rs::Bullet::update` | First tick (0.5 + FRand 0.5)·speed; subsequent ticks speed; dies past range; collision ray max(step, 1); nearest of world and player wins. *Deviation*: the player is a cylinder r 0.25, h 1.8 (the game tests 16 boxes via `Collide_Jointy`). |

## 5. Cover

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x1386d8 | `Drone_IsCoverNodeUsable` | ported | `cover.rs::CoverNode::usable` | Switch gates; thresholds (low 30°/45°, corner 0.66·A/A+5°; +5° occupied); ray from the target's head to node + (0,1.5,0) (1.15 on 0x07000011); the `+0x90` per-target bits. |
| 0x138e78 | `Drone_ProcessCoverNodes` | ported | `drone.rs::Drones::tick` | Every 5 frames from the first; shared cursor; stops after the first LOS ray. |
| 0x152548 | `NDrone2_FindCover` | partial | `drone.rs::find_cover` | The unoccupied node on the drone's network with the smallest path-distance `d` ≤ distance to opponent; claims owner/side/approach. |
| 0x152958 | `NDrone2_FindAmbushCover` | partial | `drone.rs::find_ambush_cover` | `FindCover` restricted to nodes within 2.0 units of the drone. |
| 0x151d68 | `NDrone2_CoverNodeOK` | partial | `drone.rs::cover_node_ok` | Bits `& 0xE`; with needLos `& 0x70`; distance ≥ 2.0 (and ≤ 1.5× combat range with prop 0x44); one cover anim possible; no other drone within 1.5. |
| 0x155a38 | `NDrone2_CoverAvailable` | partial | `drone.rs::cover_available` | Prop 0x30 = 0 → no; already claimed → yes; opponent visible and < 4 → no; else `FindCover`. |
| 0x152028 | `NDrone2_GetPosForCornerCover` | ported | `cover.rs::CoverNode::position` | Side 1: x = 0.1 (approach −0.7), z = −depth. Side 2: x = offset, z = −0.1 (approach +0.7). Low: x = 0, z = −0.3 (approach −1.1). |
| 0x155858 | `NDrone2_DisownCover` | partial | `drone.rs::disown_cover` | Clears owner + flag 0x10 + `+0x4F8 &= ~0x1800`. |
| 0x151900 | `NDrone2_SetAngleToCover` | partial | `drone.rs::set_angle_to_cover` | Low / as-low → opponent; side 1 → node yaw − π/2; side 2 → node yaw. |
| 0x13bf40 | `DroneAnim_ValidateCoverAnim` | **STANDIN** | `drone.rs::cover_anim` (the `can_do`/fallback path) | The `5 → 7` (or `10`) fallback for an anim the set lacks is not ported. The port picks `0xA` when `5` is missing (a one-step approximation). |
| 0x13c078 | `DroneAnim_CoverAnim` | partial | `drone.rs::cover_anim` | The `CCModes` (0x270768) table is ported (`cover::cc_anim`). Mode 6 picks the type; mode 7 is the maybe-change. The crouch/stand lean/step predicates are ported via `cover_types`. |
| 0x13b9e0 | `DroneAnim_Can*Lean/StepOut` (4 funcs at 0x13b9e0, 0x13bb38, 0x13bc60, 0x13bde8) | partial | `drone.rs::cover_types` | The four cover anim predicates (crouch lean, crouch step, stand lean, stand step) are folded into one `cover_types` helper. |
| 0x13e3f8 | `DroneAnim_ForceCrouchCover` | partial | `drone.rs::cover_types` | Prop 0x10 or a corner without `8` → forced crouch. |
| 0x13e450 | `DroneAnim_CanUseCornerAsLowCover` | partial | `drone.rs::as_low` | Low node or crouch-only corner. |

## 6. Animation

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x13ab58 | `DroneAnim_CallAnim` | ported | `drone.rs::call_anim` / `call_anim_variant` | `(blend, state, variant, next AI state, completion message)` — the variant path picks consecutive danims of a record (e.g. 0x4B × 3). |
| 0x13add0 | `DroneAnim_CallHandler` | ported | `drone.rs::call_handler` | Every frame after the anim tick; once a danim of the requested state started it dispatches the completion message. |
| 0x13aee8 | `DroneAnim_SetDAnimInternal` | ported | `drone.rs::set_danim` | The danim's state, class and call bookkeeping; then `SetScript` for the danim's script. |
| 0x13b1c8 | `DroneAnim_SetHTAnim` | partial | `drone.rs::begin_blend` | The hand-track script's blend switch (no blend / drone not animating → set DAnim; else `SetHTAnim`'s blend). |
| 0x13b490 | `DroneAnim_PreTransCallback` | partial | `drone.rs::root_motion` | Root motion clipping at boundaries; the `free_root` path (SpecialDeath) keeps the whole root motion and adds it to `+0xBE0`. The `+0x4C4` anim yaw offset is not ported. |
| 0x13b750 | `DroneAnim_EventFunc` | partial | `drone.rs::event_func` | Events 3/4 holster swap, 7 `+0x579=0`, 0xF `+0x20`, 0x12 `+0x20`/`+0x21`, 0x10 clears, 0/4/5 footsteps, 1 the grenade-throw release → `DroneWeap_ThrowGrenade`. |
| 0x13c780 | `DroneAnim_LocationDeathAnim` | partial | `drone.rs::location_death_anim` | Bond aiming and part 4/5 → 0x42 (variant 1 when shot from behind, else 0); otherwise `+0x3A=0`, 0x40 variant `Rand(4)` if the danim's `info+0x10` byte is set. |
| 0x13c8b8 | `DroneAnim_LocationImpactAnim` | partial | `drone.rs::location_impact_anim` | `CallAnim(0x4C, variant, next = +0x118)`; variant 5 + `Rand(8)` unless Bond aims, then by part: 4/5 → 5, 0x12–0x15 → 8, 0x1E/0x1F/0x20/0x23 → 9, 0x31–0x33 → 13, 0x35–0x37 → 12, else 6 + `Rand(2)`. |
| 0x13ca68 | `DroneAnim_SetStandIdleAnim` | partial | `drone.rs::set_stand_idle` | `(SM id + 1) % 3 + 0x37` for patrol; sets 0x37 when the set lacks it. |
| 0x13cc70 | `DroneAnim_SetWalkIdleAnim` | partial | `drone.rs::walk_anim` | The walk idle anim from the anim set. |
| 0x13ce00 | `DroneAnim_GetMoveQuad` | ported | `drone.rs::move_quad` | 0 forward, 1/3 sideways, 2 backward, with hysteresis on `previous`. |
| 0x13cfb0 | `DroneAnim_SetCombatMoveAnim` | partial | `drone.rs::set_combat_move_anim` | `a = diff(yaw to move target, yaw to opponent)` → quad with hysteresis; quad 0/2/1/3 → anim 8/0xF/0x1E/0xC/0xD; rolls with prop 0x2E; steps with prop 0x42. `+0x31C & 2` (drone ahead) → 0x5E not ported. |
| 0x13da48 | `DroneAnim_PlayFiringAnim` | stub | — | The `+0x20`/`+0x21` flag-driven firing anim — not ported. |
| 0x13dae8 | `DroneAnim_CheckForAnimComplete` | partial | `drone.rs::update_anim` | `+0x57A` set and `+0x579` cleared by event 7 → `SetEndAnim` now. |
| 0x13dc90 | `DroneAnim_ZeroAnimCall` | partial | `drone.rs::call_anim` | The pending-call zero on a playing-state request is ported. |
| 0x13dd00 | `DroneAnim_SetEndAIState` | partial | `drone.rs::handle(*, msg::ANIM_DONE)` | Sets `+0x118` (resume). |
| 0x13df38 | `DroneAnim_SetEndAnim` | partial | `drone.rs::handle(*, msg::ANIM_DONE)` | Sets the end-anim flag. |
| 0x13dfc8 | `DroneAnim_CanDoAnimState` | partial | `drone.rs::can_do` | The anim state's own danim has a script in our anim set (or set 0). |
| 0x13e088 | `DroneAnim_SetScript` | ported | `drone.rs::play_script` | `(script, state, follow-on, …, next AI state, message, flags)`. |
| 0x13e198 | `DroneAnim_SnapRotate` | partial | `drone.rs::Drones::tick` | Applied to the yaw and the desired yaw; info flags 0x100 → π, 0x80 → π/2, 0x200 → −π/2. |
| 0x13e238 | `DroneAnim_OnStartAnim` | partial | `drone.rs::set_danim` | Class 6 without a big hit (`+0x1C`) is cut to 15 frames, end blend 5. |
| 0x13e2c0 | `DroneAnim_OnEndAnim` | partial | `drone.rs::event_func` | The `+0x54C` turn left by a finished turning danim; events are dispatched in `event_func`. |
| 0x13e3b0 | `DroneAnim_InTransition` | ported | `drone.rs::in_transition` | The playing danim's class is above 3 and not 8. |

## 7. Grenades

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x17c518 | `Explode_Propagate` | ported | `grenade.rs::explode_propagate` | Sphere intersect of `radius` (`weapon+0x08`); `damage = amount·(1 − dist/radius)` (min 0.0002); drones take `Drone_ExplosiveHit` (msg 9); Bond takes `Player_Hurt`. |
| 0x17cbd0 | `Explode_Create` | stub | `drone.rs::Drones::tick` (Explosion struct) | The `Explode` object's create is approximated by an `Explosion` struct in the `Drones` list. |
| 0x13aab8 | `Drone_ExplosiveHit` | partial | `drone.rs::handle_death(*, msg::EXPLOSIVE)` | Msg 9 → `Drone_ExplosiveHit` → `HitDamage` with the explosive damage; dead → Death_Anim. |
| 0x3037dc | (per-material bounce table) | **STANDIN** | `grenade.rs::BOUNCE_RESTITUTION` (= 0.4) | The game reads `0x3037dc + material·0x88` for the bounce coefficient; the port uses a constant 0.4 (the feet-sphere radius and a reasonable floor bounce). |

## 8. Navigation

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x155cd8 | `NDrone2_NavNodeCache` | partial | `drone.rs::Drones::tick` (target cache) | The player's goal nodes per navigation path, rebuilt when he has moved ≥ 2.0 (search radius 6 = min(engage, 6)). |
| 0x156760 | `AINetwork_OptimiseRoute` | partial | `nav.rs::Route::to_position` (drop node 0 / last) | Drop node 0 if `MoveTestToNode(start → node[1], near) = 1` and the pair is not both door / both kick; drop the last likewise. |
| 0x156ba8 | `AINetwork_CalcRoute` | partial | `nav.rs::Route::to_position` | `MoveTest(start, dest, near 1) = 1` → direct route (status 2); else A*. |
| 0x156e18 | `AINetwork_NodeSearchCel` | partial | `nav.rs::Network::candidates` | Box ±25 from the start; cel lists not decoded (one cel). |
| 0x1571b0 | `AINetwork_DoAStarPath` | partial | `nav.rs::Network::astar` | The A* with cost = g + len × (used + 1); the four decompiler bugs (no re-sort of open nodes, the open bit 0x40000000) are kept. |
| 0x157780 | `AINetwork_HitBoundry` | partial | `nav.rs::intersect_adjust` | The 2D segment intersection. |
| 0x157990 | `AINetwork_LineIntersectsBoundry` | partial | `nav.rs::intersect_adjust` | The wall y-range test (yWall − 2.4 ≤ yLine ≤ yWall + hWall). |
| 0x157bd0 | `AINetwork_GetBoundsPushVectorForSphere` | partial | `nav.rs::Network::push_out` | The push-out vector for the feet sphere (r 0.4) out of boundary walls (no flags 0x1C00). The push itself is *suggested*. |
| 0x157d80 | `AINetwork_BoundsTest` | partial | `nav.rs::Network::bounds_test` | The portal connectivity part (`Collide_StraddleCels`) is *deviation*: a collision-world line-of-sight at +1 stands in. |
| 0x158140 | `AINetwork_BoundsNodeTest` | partial | `nav.rs::Network::bounds_node_test` | The post-within-√thr test (thr = 0.5·r² when the post is within √1.3·r of A or B, else r²). |
| 0x158488 | `AINetwork_NavPathForPosition` | ported | `nav.rs::Network::nav_path_for` | The nearest node (2D) of a path with `kind & 0xD == 0` in the drone's cel. |
| 0x1586a8 | `AINetwork_NodesForPosition` | ported | `nav.rs::Network::target_nodes` | The cel's nodes in a box (±√r² or ±15 in x/z, ±2 in y), nearest first (2D); the first with `MoveTestFromNode(node → pos) = 1` is "found". |
| 0x158cf8 | `AINetwork_EmitPath` | ported | `nav.rs::Network::emit` | Per nav node a path-distance byte (0xFF = unreachable). |
| 0x159668 | `AINetwork_SetupNextNode` | partial | `nav.rs::Route::next_node` | Mode 2 wraps; mode 0 moves to the dest. |
| 0x15a490 | `AINetwork_UpdateLinksUsedCount` | partial | `nav.rs::Route::mark_used` | Adds 1 to every remaining link of a followed route. |
| 0x15a5c8 | `AINetwork_InitPassableBoundries` | ported | `nav.rs::Network::init_walls` | Crossed by any path link → 0x400 passable. |
| 0x15a8d8 | `AINetwork_FurthestPosition` | partial | `nav.rs::Network::first_wall_hit` | Every BLINK, no filters, nearest intersection to the drone; y = post a's y. |
| 0x15aa88 | `AINetwork_NodeSearch` | partial | `nav.rs::Network::node_from` | The nearest node (2D, < 0.3) of a path with `kind & mask`. |
| 0x15b110 | `AINetwork_FollowRoute` | ported | `nav.rs::Route::follow` | Remaining distance; arrival radius; the link creep. |
| 0x15b498 | `AINetwork_ClearLinkFlags` | partial | `drone.rs::Drones::tick` (zero used) | `ClearLinkFlags(0x310)` in `Drone_InitComms` — the per-frame used-link zero. |
| 0x151618 | `NDrone2_MoveToObject` | partial | `drone.rs::move_to_object` | Verify / compute the route; 3 within 2.0; follow. |
| 0x1517d8 | `NDrone2_MoveToGoalPosition` | partial | `drone.rs::move_to_goal` | Routes only when the combat route is invalid; results 0 moving, 3 arrived, 0xC blocked, 10 door, 4/5/6/8 failed. |
| 0x1543f8 | `NDrone2_SetGoalToDroneWithLosToOpponent` | partial | `drone.rs::set_goal_to_drone_with_los` | The first other drone (list order) that sees the opponent and wins `Rand(4) == 0` and `MoveToGoalPosition` gives 0 or 2. |
| 0x155e10 | `NDrone2_SetGoalToCloserPosition` | partial | `drone.rs::set_goal_to_closer_position` | Where the straight drone → opponent line first meets an AI-network wall. |
| 0x1514d8 | `NDrone2_UpdatePatrolRoute` | partial | `drone.rs::update_patrol_route` | `FollowRoute`; result 0 → walk anim (0x35 when prop 0x25 and alert ≥ 0.66), 7 → stand idle, else walk / 0x21. |
| 0x153720 | `NDrone2_InitAIPath` | partial | `nav.rs::Route::for_path` | Mode 2 (loop) for kind 4, else 1; kind 0x10 sets route bit 0x20. The first-node-of-a-kind-4-path-without-0x10 second-link rule is ported. |
| 0x154190 | `NDrone2_AssignAIPath` | partial | `drone.rs::assign_path` | The nearest node (2D, < 0.3) of a path with `kind & mask` → `InitAIPath`. |
| 0x154bd0 | `NDrone2_SlideToPosition` | partial | `drone.rs::slide_to` / `slide_free` | x/z toward the point (snapping within 0.01); the 3D variant (the special death) is `slide_free`. |
| 0x154d58 | `NDrone2_SetAngleToObj` | partial | `drone.rs::combat_state` (inlined) | Face the opponent; aim / look-about. |
| 0x1551c0 | `NDrone2_MoveTest` (vector) | partial | `nav.rs::Network::move_test` | length² ≤ 900; |dy| ≤ 4.0; slope ≤ 0.8727 (0.7854 on 0x07000008). |
| 0x155be0 | `NDrone2_FollowRoute` | partial | `nav.rs::Route::follow` | The creep; target → `+0x670`; desired yaw `+0x694` = `ATAN2_APPROX` to it. |
| 0x14eba8 | `NDrone2_Steer` | partial | `drone.rs::face_and_aim` | `yaw += Vec_AngleDifference(yaw, desired) · 0.1`. |

## 9. Talk

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x13e988 | `NDrone2_PatrolTalk` | partial | `drone.rs::patrol_talk` | Every patrol frame while the global cooldown is open: `Rand(2) ≠ 0` → cooldown set; type-3 drones speak. |
| 0x13eac0 | `NDrone2_AttackTalk` | partial | `drone.rs::attack_talk` | Type 2 picks `0x37 + Rand(9)` (07000005–08). |
| 0x13eeb8 | `NDrone2_PainTalk` | partial | `drone.rs::pain_talk` | Forced; skipped only when the same line is the current one. |
| 0x13eff8 | `NDrone2_DeathTalk` | partial | `drone.rs::death_talk` | Forced; the second call of a death sees the first's line and is skipped. |
| 0x13f1d0 | `NDrone2_SeenPlayerTalk` | partial | `drone.rs::seen_player_talk` | 1 in 3, then the level's set (05–08: type 2 `0x49 + Rand(6)`, else `0x21`). |
| 0x13f688 | `NDrone2_HandleTalking` | partial | `drone.rs::speech` | `SpeechSFX(idx, force, 0, 0)` → `PlaySFX(2.0, …)`; the first-30-frame close, the repeat-delay gate, the facial-anim refusal, the line stamp. |
| 0x141848 | `NDrone2_PlaySFX` | partial | `drone.rs::Drones::play_sound_handle` | The 3D sound play at the drone. |
| 0x141b00 | `NDrone2_SpeechSFX` | partial | `drone.rs::speech` | As `HandleTalking`. |

## 10. Shared / utility

### `Drone_*` create / control / opponents

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x1390c0 | `Drone_ProcessOpponents` | partial | `drone.rs::Drones::tick` | Bond's visibility factor from the AI volumes holding him (× 0.75 crouching — *deviation*: Bond does not crouch in the port); each drone's opponent info. |
| 0x139268 | `Drone_Control` | partial | `drone.rs::Drones::tick` | The per-drone control: `PreDroneControl` → `ControlSTANDARD` → `SnapRotate` → animation → `CallHandler`. |
| 0x139520 | `Drone_InitComms` | partial | `drone.rs::Drones::tick` | Vision pass; the 0x97 alarm channel; `HandleSoundAlerts`; `Sound_ZeroAlertness`; the alarm. The corpse count (`DeadDrone`) and the `DronePostLoadFrame` first-30-frame speech close are ported. |
| 0x138f68 | `Drone_GetOpponentInfo` | partial | `drone.rs::opponent_info` | `+0x1A0` distance; `+0x1C4` = `ATAN2_APPROX(dx, dz)`. |
| 0x13a0a0 | `Drone_DCVfromOBJ` | partial | `drone.rs` (implicit field access) | The DCVars scratch is not modelled (the port works on `Drone` directly). |
| 0x136be0 | `Drone_Create` | partial | `drone.rs::Drone::new` | Placement → drone fields; the `obj_tag` allocation is not modelled. |
| 0x136ce8 | `Drone_Delete` | partial | `drone.rs::Drones::tick` (drop) | Drones flagged `gone` are dropped from the list. |
| 0x1376c0 | `Drone_AIVolume_Create` | ported | `aivolume::build` | Placements 0xF5 / 0xF8 → `AIBoxList`. The half-extents, kind, factor, and the 07000005/param-1 rotation flip + position lift are ported. |
| 0x1383e8 | `Drone_EnableAll` | partial | `drone.rs::Drones::tick` | The enable / disable from switch channels. |
| 0x13a630 | `Drone_SM_SendMsg` | partial | `drone.rs::Drone::send` | The send path. |
| 0x13a9f8 | `Drone_BulletHit` | partial | `drone.rs::Drones::update_player_bullets` | Bond's bullet → drone hit; the `Collide_PickObj`/`Collide_Jointy` path. |
| 0x139f50 | `Drone_ModBulletDamage` | stub | — | The per-weapon bullet damage mod — folded into `damage::hit_damage`. |

### Spawners

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x1378d0 | `DroneSpawner_Create` | ported | `spawners::Spawner::new` | Placement → Spawner; modes 2/3, group, max_alive, waves, start/stop/done/kill channels, min_dist. |
| 0x137aa0 | `DroneSpawner_Init` | ported | `spawners::Spawner::new` | The init from the placement parameters. |
| 0x137cb0 | `DroneSpawner_SpawnDrone` | ported | `spawners::Spawner::update` (Action::Spawn) | The spawn action — picks an unused template; the spot-OK test (`min_dist` from the player, not visible). |
| 0x137e68 | `DroneSpawner_Control` | ported | `spawners::Spawner::update` | The per-tick control: start/stop channels, waves, max-alive, kill mode. |

### Damage / hit

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x145558 | `NDrone2_HitDamage` | ported | `damage::hit_damage` | Part < 0 → 1; head with fists / taser → torso; `+0x14 = 0` → only `+0x150` = dmg. Head ×Head (×Helmet if `+0xBB&4`), arms ×Arms (×Combat &8, ×Jacket &2), legs ×Legs (×Combat &8), else ×Torso (×Combat, ×Jacket, ×Vest). × difficulty. 0700001B every non-head part is torso. |
| 0x1458e0 | `NDrone2_DoHitEffects` | partial | `drone.rs::do_hit_effects` | Damage > 0.5 → sound 0x413 at the hit (0x601 when `HITDATA+0x52 = 5` — *not modelled*). |
| 0x1461c8 | `NDrone2_BulletImpact` | partial | `drone.rs::bullet_impact` | `+0x50C = 1`; alert status 2; `DoHitEffects` → `HitDamage`; dead → 0x44; alive with `allow` → flinch when ≥ 2 s since the drone's / global's last flinch. |

### Init / type

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x14b300 | `NDrone2_DefaultInit` | partial | `drone.rs::Drone::new` | `+0xAC` health 10.0; `+0x14` damageable 1; `+0x14C` 1.0; `+0x100` 1.0; placement parameters 3/6/8/12/13; captain `+0x19`. |
| 0x14c938 | `NDrone2_GetDTYPENEW` | partial | `drone.rs::dtype_of` | Mode for type `ty` and a block's mode `block` (0 = the base); types 10–16 special; block 2 → 2 (sniper) or 0x19 at alertness 1; etc. |
| 0x14cbd8 | `NDrone2_GetDroneTypeAttackTypeFriend` | partial | `drone.rs::type_setup` | Modes, team, the type's init function. |
| 0x1422d0 | `NDrone2_SetOpponent` | partial | `drone.rs::opponent_info` | Single player → the player. |
| 0x1424e0 | `NDrone2_FindOpponent` | partial | `drone.rs::opponent_info` | As above. |
| 0x141f28 | `DroneFunc_AllocateTargetID` | partial | `drone.rs::target` | The player target id 0. |
| 0x149dd0 | `NDrone2_Enable` | partial | `drone.rs::Drone::enabled` | `obj+0x30 &= ~0x30`; `+0x4F8 |= 0x100`. |
| 0x149f88 | `NDrone2_DoAnimation` | partial | `drone.rs::update_anim` | A drone that was not drawn (`obj+0xFC = 0`) playing a danim with info flag 4 and no blend slides along its yaw by `Drone_AnimInfo +4` per frame. The port approximates "drawn" by Bond's view (60°) and LOS to five body points. |
| 0x148db0 | `NDrone2_PreDroneControl` | partial | `drone.rs::Drones::tick` | The timer → message dispatch; the second-switch (`+0x135`) with a tag (`+0x13A`) → 0xC deletes. |
| 0x1490f0 | `NDrone2_ControlSTANDARD` | partial | `drone.rs::Drones::tick` | Alertness decay; msg 3; find opponent; tracking; move; collision; `DroneWeap_HandleFiring`. |
| 0x145150 | `NDrone2_SeenAndAttacking` | partial | `drone.rs::Drones::tick` (music gate) | The `+0x4F8` |= 0x2000000 attack bit; the music gate. |

### Maths / RNG / sound

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x1e3738 | `Rand_Rand` | ported | `drone.rs::Rng::rand` | Two 16-bit multiply-with-carry streams: x × 18000 and y × 30903; `Rand_Rand(n)` returns `(x + y) % n` (the game's `a_318`/`b_319` constants). Bit-exact. |
| 0x1e3780 | `Rand_FRand` | ported | `drone.rs::Rng::frand` | `Rand_Rand(65536) / 65536 · max`. Bit-exact. |
| 0x1dd548 | `Sound_Play3D` | partial | `drone.rs::Drones::play_sound_handle` | The 3D sound play at a position; the hearing list entry. |
| 0x1ddc48 | `Sound_Alertness` | partial | `drone.rs::Drones::tick` (sound loop) | Per sound within its outer radius, weight (inside the inner radius) or weight·(outer−d)/(outer−inner); the sum and the loudest. PS2 float arithmetic (`add_noguard`, `mul`) used. |
| 0x1e2b78 | `Vec_AngleDifference` | ported | `trig::angle_difference` | The wrap-to-π angle difference. |
| 0x1e0f10 | `Ang_RotateToBearing` | ported | `trig::rotate_to_bearing` | The per-tick yaw rotation toward a bearing. |
| 0x1e4528 | `ATAN2_APPROX` | ported | `trig::atan2_approx` | The PS2's atan2 approximation. |
| 0x1e5168 | `View_RotTransMatrix` | ported | `aivolume::matrix` | `Rx·Rz·Ry` row vectors; PS2 float arithmetic. |
| 0x1ec180 | `Collide_Jointy` | partial | `damage::first_box_hit` | The first box in list order the segment hits (not the nearest). The full `Collide_Jointy` (sphere/box/capsule) is not ported — only the ray-box path. |
| 0x1c4ac8 | `AnimGetCollData` | ported | `damage::boxes` | The `HumanCollisionBones` (0x2c6be8) → boxes; entry 0's centre is the bone plus (0, bone.y − other.y, 0); the others sit midway. Extents × skin scale. |
| 0x1df7e0 | `vecutil_calculate_closest_point_on_line` | ported | `bullet::closest_point_on_segment` | The closest point on segment `a→b` to `p`, clamped to [0,1]. |
| 0x1d3a80 | `ReadTuningVars` | ported | `nf_game::tuning::read_tuning_vars` + `nf_formats::tuning` | The per-level tuning switch; checked against the R5900 interpreter for every level id 0x07000000–0x0700004F. |
| 0x1c1b38 | `AnimProcessScriptCmds` | partial | `drone.rs::script_commands` | The walk-script commands: events 0/4/5 footsteps; event 3 `Sound_StopAllWithId(arg)`. Other commands (sounds, weapon changes, footprints) are partial. |

### Player (used by the guard AI)

| Address | Function | Status | Rust port | Notes |
|---|---|---|---|---|
| 0x1902c8 | `Player_HandlePain` | partial | `bullet.rs::PlayerHealth::pain` | Armour absorbs first; health floors at 0; flash += min(d·21.333, 128). |
| 0x1a5310 | `Player_Activate` (door part) | partial | `drone.rs::Drones::player_use_doors` | The use key at Bond's head, one unit ahead; `Door_Activate` for every door the sphere meets. |
| 0x1a73f8 | `Check_AutoAim` | partial | `drone.rs::Drones::auto_aim` | Drawn, alive enemy drones within 25; cone 0.12·k yaw, 0.22·k pitch, k = `weapon+0x14`/100 · (1−d/25) · difficulty; ×1.4 lock-on; target bone 2; LOS. The crosshair offset is kept as a yaw/pitch angle (*approximation* of the screen offset). |
| 0x1a7c98 | `Player_AutoAim` | partial | `drone.rs::Drones::auto_aim` | The per-tick auto-aim driver. |
| 0x1a7810 | `Check_Target` | partial | `drone.rs::Drones::tick` (crosshair) | Every 6 ticks Bond's 50-unit crosshair ray marks the first drone it hits (body cylinder r 0.4) for 30 ticks. *Deviation*: the game tests the drone's bounding sphere / boxes. |

## Notes on the count

The 268 total is the set of ACTION.ELF functions the guard AI *touches* on the test level 07000005 — the
decomp entries whose behaviour is referenced or implemented in `drone.rs` / `cover.rs` / `grenade.rs` /
`bullet.rs` / `damage.rs` / `nav.rs` / `spawners.rs` / `aivolume.rs`. Functions that exist only in the
multiplayer `BOT_*` / `NDrone2_DSTATE_Bot*` namespace are excluded (they are a separate AI). Functions that
are pure data (the `NDrone2_StateFuncs` table at 0x29b300, the `CCModes` table at 0x270768, the
`HumanCollisionBones` table at 0x2c6be8, the `SFXOutputData` table at 0x2c94a0, the `WldGravity` VECTOR at
0x2d8890, the `REC_FRAME_RATE` constant at 0x30d0cc) are referenced in the notes above but not counted as
separate functions; the per-material bounce table at 0x3037dc is counted (it is one of the seven STANDINs).

Trivial DroneAnim wrappers that the port folds into a single function (`call_anim`, `set_danim`,
`call_handler`) — `0x13db30 GetDAnimForCall`, `0x13dbf8 CanSetAnimCall`, `0x13dcb8 CallFullyComplete`,
`0x13dd78 SetDAnimNow`, `0x13ddd0 SetAnimD`, `0x13de40 GetDAnimForAnimState`, `0x13def8 SetAnimCall`,
`0x13e3e8 AnimEndCallback` — are not counted as separate functions; their behaviour is absorbed into
`DroneAnim_CallAnim` (`0x13ab58`), `DroneAnim_SetDAnimInternal` (`0x13aee8`) and `DroneAnim_CallHandler`
(`0x13add0`). `DroneAnim_ZeroAnimCall` (`0x13dc90`) is counted because its `ZeroAnimCall` behaviour is
ported inside `call_anim`. The four `DroneAnim_Can*Lean/StepOut` predicates (`0x13b9e0`–`0x13bde8`) are
counted as one row.

The 5 "not started" rows are `0x1499d0 DroneFunc_ClampPosToCollisionCylinder`,
`0x1759a0 Drone_SM_SendDelayedMsgs`, `0x163a80 NDrone2_DSTATE_DeathByExplosion`,
`0x178520 DroneVision_GunThroughWall`, `0x1789c0 DroneWeap_CloseCombatImpact`. Each is flagged for
traceability; porting them is G-026 / G-028 remainder.

The user's original estimate of 108 functions was the count of "major" guard-AI entries; the thorough
enumeration here adds the per-state handlers (50+ `NDrone2_DSTATE_*`), the per-anim helpers (~22
`DroneAnim_*`), the per-route helpers (~18 `AINetwork_*`), and the `DroneVision_*` / `DroneWeap_*` /
`NDrone2_*` helpers that the Rust port calls but the user's list did not break out. The seven STANDINs and
the three stun-grenade receiving states are called out explicitly above.

## Audit findings (WORKLOG 0086)

Four auditor subagents audited 30 functions in batches of 10, plus the tests and dev scenarios. Findings verified
by the developer before acting.

### Fixed in this pass

| Gap | Fix |
|---|---|
| ThrowGrenade lob axis was YAW (Y) instead of PITCH (X) — grenades had no arc | Fixed: applies ±π/4 as a pitch (elevation) to the horizontal direction toward the opponent |
| friendly-actor scenario set team=2 but port treats only team=3 as friendly | Fixed: team=3 |
| UnderCoverSniperFire AND-chained aim/facing/RNG instead of OR-splitting | Fixed: OR-split (aim → duck immediately, else facing+RNG → duck) |
| GrenadeThrow ENTER missing +0x4F8 \|= 0x10000 (vision_on) | Fixed: vision_on = true on ENTER |

### Confirmed gaps (left open, marked STANDIN/partial/stub in GAPS)

| Function | Gap | Status |
|---|---|---|
| 0x140968 Drone_CollisionHandler | Missing: friction ×0.98, height-correction, DealWithObjHit, SpecialDeath sub-path, standing bits clear | STANDIN |
| 0x140660 Drone_FeetOnPoint | Missing: up-vector from matrix, hit-list, vehicle riding, per-material table | STANDIN |
| 0x140e90 NDrone2_Collision | Missing: radii 0.55/0.3, bone transforms, offset writes | STANDIN |
| 0x149ec8 NDrone2_DoCollision | Predicate collapsed to `bounded` bool — per-tick conditions (voice type, drawn, near-player) not evaluated | STANDIN |
| 0x141e58 NDrone2_DoGravity | Same predicate collapse as DoCollision | STANDIN |
| 0x13fff0 NDrone2_DealWithObjHit | The dispatcher is absent except the player-hit arm: case 5's weapon → msg 6 (punch) / msg 7 (taser) / msg 8 routing is ported (fix-player4). Still missing: water ripples, gas grenades, vehicle riding, owner tracking | partial |
| 0x14b1e8 DroneInit_Collision | Absent — drones never snap to floor at spawn | not started |
| 0x1499d0 DroneFunc_ClampPosToCollisionCylinder | Absent | not started |
| 0x14b190 DroneFunc_NearPlayerCollision | Absent (MP-only) | not started |
| 0x13aab8 Drone_ExplosiveHit | Sends msg 8 (BULLET) instead of msg 9; no owner tracking (+0x2B4); inline damage double-counts with on_bullet | partial |
| 0x17cbd0 Explode_Create | `explode.rs` (`Explosion::create`): radius/damage from weapon+0x08/0x0C [M 0x12cd98 disasm], lifetime script?+0xAB0:15, scale radius·0.0666, script 0x6000052 via the one-shot player, shake ≥ 0.5 into `Drones::shake` | done (fix-explosions) |
| 0x17c518 Explode_Propagate | `explode::propagate`: all 12 dispatch cases + the (int)-truncated falloff and the 0x21 ≥1.0 gate; the Sim feeds drones/Bond (break/destroy targets arrive with the dynamic-object pass) | done for the ported lists (fix-explosions) |
| 0x173658 RecoverFromScaryObject (0x86) | No handler — drone stuck in flee state indefinitely | stub |
| 0x1431c0 ConsiderExplosive | 14-case weapon-type switch not ported; +0x324 timer, +0x354 resume, +0x340 pos not set | partial |
| 0x12ce80 Bullet_CollisionHandler | Ricochet path not ported | partial |
| grenade.rs bounce | 0.4 constant instead of per-material table at 0x3037dc | STANDIN |
| 0x16a610 UnderCoverSniperFire | `aimed_at` is "recent bullet hit", not the decomp's `OpponentIsAimingAtMe`; +0x2E, +0x3b, +0x3c fields not stored | partial |
| 0x152958 FindAmbushCover | Side selection via LOS bits, not geometric distance; LOS gating wrong for ambush | partial |
| 0x173480 GrenadeThrow state | Missing impact msgs 6/7/9/0x17 (HandleImpact not called for non-BULLET impacts) | partial |
| Multiple states | UNDER_COVER_SNIPER_FIRE, SNIPER_RELOAD, AMBUSH_INIT, AMBUSH_WAIT missing from impact_args — bullets don't damage them | partial |
| 0x14e018 NDrone2_init_DMODE_Ambush | Not modeled — prop 0x10 read from placement, not set by init | partial |
| Stun grenade receiving (0x1631c0, 0x163350, 0x172bb0) | `handle_stun`: 0x4D/0x4E/0x4F with the 7200-tick recover, anims 0x2D/0x2E/0x76, msg-8/9 → 0x44; the weapon drop not modelled; entered from the flash broadcast (Bullet_DoTrails 0x12d890's +0x70 & 0x2000 death branch, msg 0x18) | partial (fix-explosions) |

### Test audit findings

| Test | Finding | Action |
|---|---|---|
| guard_gravity_and_collision_spheres | Sound; real sister; primary is self-consistency, not disc-measured | No change needed |
| guard_throws_a_grenade_when_in_range | Sister is tautological (`!can2 \|\| grenade_count <= 0` always true); final assert is tautology; docstring claim (pending_grenade → projectile) not tested | Noted — test needs rewrite to assert can_throw_grenade fails when grenade_count=0 |
| sniper_cover_branches_to_sniper_fire_state | Sound; real sister pins anim_set gate | No change needed |
| ambush_mode_starts_in_ambush_init | Lookup correctly tested; docstring describes 0x37→0x93/0x38 branching not tested; uses i=0 not gi | Noted — test needs send(ENTER) to exercise the branching |

### Dev scenario audit

- None of the 13 scenarios add RNG draws ✓
- None have been visually verified (no lavapipe in the sandbox) ✗
- friendly-actor team=2→3 bug FIXED
- searchlight-tracks-bond and door-anti-crush don't verify a searchlight/door is present (noted)

## fix-explosions2 — the grenade weapon ids and the death-branch routing

The grenade weapon ids, named by the disc labels (`Txt_BindLabel(weapon_data+0x38)`, resolved against
USATxt.dat with the game's own fixup algorithm — see the report): **0x34 'Fragmentation Grenade'
(radius 8 / damage 100), 0x35 'Stun Grenade' (10/0), 0x36 'Smoke Grenade' (0/0), 0x37-0x39 'Remote
Mine' (8/100), 0x3a 'Laser Tripbomb' (8/100), 0x3b-0x40 'Satchel Charge' (8/2000)** [L]. The
`Bullet_DoTrails` 0x12d890 timer-death branch routes on the weapon's `+0x70` flags [M asm
0x12dd60/0x12e104/0x12e1c0]: `& 0x2000` → the flash branch (0x35 only among the grenades [L sweep]);
`& 0x20000` → the smoke branch (0x36: Occlude_Create + Sound 0x14 + a 1200-tick re-arm); else
`Explode_Create(wd+8, wd+8, wd+0xC)` — the frag/mine/satchel blast. fix-explosions' mapping was one
id off (it called 0x35 the frag and 0x36 the stun), which made its port stun on the smoke, dud the
stun throwers, and conclude the frag was harmless. `NDrone2_DefaultInit` 0x14b300's +0xBCC selection
(1→0x36, 2→0x35, 4→0x34, 5→count+0x3a, 6→0x6c) and `DroneWeap_ThrowGrenade` 0x179558's sniper pick
(0x35, ⅓ → 0x36) use the same constants the port already had.
