# `Drone_tag` named offsets — the AI/drone record (obj+0x110, size 0xD20)

Proof levels: see [README.md](README.md). Evidence tags: [M] = decompile (function = filename stem, address =
filename prefix, `:NN` = line in the .c file), [M-dis] = checked against raw MIPS in `work/ee_start_05.bin`, [L] = EE snapshot read.
Worker task 4-b2. Context: `Drone_tag` = `obj+0xE0` (embedded at `obj+0x110`, 0xD20 bytes — see
[dcvars-statemachine.md](drone-statemachine.md)); drone list head = u32 0x298E14.

Method note: the bit census was mechanical — every `.c` in `work/decomp/ACTION.ELF/` scanned for `+ 0x4f8)`, the enclosing
statement extracted (835 raw → 534 unique statements after de-duplicating read-modify-write double occurrences), then classified
into SET (`| const` on the RHS of an assignment), CLR (`& mask` on the RHS), TEST (`& const` in a condition, incl. via a temp
loaded from `+0x4F8` and masked later in the function) and READ. 244 files touch the word. Every non-drone-family hit
(`Door_ReactToHit` 0x135980:54, `Pickup_Update` 0x18e148:228, `GT_Track` 0x17f9c8:89,140, `HUD_RadarUpdate` 0x19f6e8:189,
`MP_GetTarget` :37, `MP_GetRadarObjects` :72, `MP_CheckForEndCondition` :106, `MP_playerIsDead` :26, `MP_isPlayerOrBotDead` :16)
was verified to deref `obj+0xE0` first — all of them read the Drone_tag. Bot-family (`BOT_*`) hits take a `Drone_tag*` first arg.

## Group 1 — `Drone+0x4F8` behaviour-flag word (u32)

The universal "live drone" eligibility gate, repeated in ~30 functions (sight, alert, opponent, MP): 
`(0x4F8 & 0x600) == 0 && (0x4F8 & 0x100) != 0 && 0.0 < *(float *)(drone+0xAC) && obj+0xFF != 0x11 && (obj+0xFE & 1) == 0`
— canonical copies: `DroneVision_ProcessDroneSight 0x176a28:81-84`, `NDrone2_FindOpponent 0x1424e0:222-225`,
`NDrone2_Steer 0x14eba8:152`, `DroneFunc_DoForcedAttack 0x148c60:21-24`, `MP_HitBy `MP_HitBy` :52-54`. (; cited by line only.)

| Bit | Type | Meaning | Proof |
|---|---|---|---|
| 0x4F8&0x1 | u32 bit | **lateral push-move active**: when set, `Drone_Control` 0x139268:76 applies the +0x58c lateral velocity (if +0xBA8==0), `DroneMove_SetBoundryFlags` 0x14f140:47 runs the 4-way reachability probe into +0x320, `DroneAnim_PreTransCallback` 0x13b490:120 same test. **No setter found** in any `+0x4F8` statement — set through an unaccounted path (suspect a block/aliased write); live-check queued | [M1 ×3 testers; setter missing — flagged] |
| 0x4F8&0x2 | u32 bit | init-on flag; **no reader found**. Set by `NDrone2_DefaultInit` 0x14b300:60 (`& 0xffea05e2 \| 0x801e2`), cleared by `NDrone2_DSTATE_AstronautLaunch` 0x16dd28:19 (`& 0xfffffffd`) | [M* writers; no reader] |
| 0x4F8&0x4 | u32 bit | **dead-body keeps gravity** (level 0x700004a): `NDrone2_DSTATE_Death_Anim` 0x1637f8:26 and `AbseilDeath` set it on enter; `NDrone2_DoGravity` 0x141e58:13 returns 1 (gravity on) when set | [M*] |
| 0x4F8&0x8 | u32 bit | **hurt-shout latch**: `DroneFunc_SendHurtMessage` 0x145e28:11-32 tests-then-sets around the hurt broadcast (msg 0x12); same pair in `DSTATE_Taser` 0x162b98 and `NDrone2_ExplosiveImpact` 0x146480 | [M*] |
| 0x4F8&0x10 | u32 bit | **stationary / movement locked** (port reading confirmed): testers refuse movement — `CanStepLeft/Right` 0x14f640/0x14f8xx:53, `CanRollLeft` 0x150338:53 / `CanRollRight` 0x150508:53, `CanStrafeDodgeLeft/Right` 0x14fdf8/0x14ffc0:53, `CanBackoff` 0x1501a0, `CanStrafeLeft/Right`, `CanMoveToRelativePos` 0x154838:43,59, `DroneMove_NoBunching` 0x150ad0:22, `DroneMove_SetBoundryFlags:51…` (movement only when clear); `DroneFunc_CombatState` 0x148338:55,76,136 returns fixed state 0x59 when set. Setters: `DefaultInit` 0x14b300:492 (special initial mode ≠ 0x600021f), `DoModeSettingsNEW` 0x14c600:149 (mode 0x600021f), `initDTYPE_Sniper` 0x14e570:6 / `initDTYPE_SniperAlert` 0x14e580:19, sniper DSTATEs (SniperAim/Fire/Idle/Reload), `DSTATE_HostageKiller:23`. **Sole clearer**: `DSTATE_HostageKillerAttack` 0x15c940:38 (`& 0xffffffef`) | [M*] |
| 0x4F8&0x20 | u32 bit | **special-physics / riding mode** (port's "gravity?" refined): set by TruckDriverInit/InitAlert/Mission, `AstronautLaunch` 0x16dd28:34, `CombatNoRoute`; cleared on death (`SetAsDead` 0x14a2f0:16, `Fade`, `FadeFast`, `SpecialDeath_Anim` 0x163c68:55, `AbseilDeath`, death TruckDriver clears). Readers: `DoGravity` 0x141e58:19 and `DoCollision` 0x149ec8:12 — when set, gravity/collision only apply in special cases (anim 0xC/0x13, +0x538 ≠ 0, near-player-collision, obj+0xFC, +0x570&2) | [M*] |
| 0x4F8&0x40 | u32 bit | **velocity damping on**: `Drone_CollisionHandler` 0x140968:139 scales the +0x480 velocity vector by 0.98 each collision tick while set; cleared by `DroneMove_AstronautDeathMove` 0x156008:45; set at init via the 0x801e2 bundle | [M*] |
| 0x4F8&0x80 | u32 bit | **gravity master enable** (port's "?" resolved): `NDrone2_DoGravity` 0x141e58:12 returns 0 (no gravity) unless set; set by `DefaultInit` 0x14b300:60/:607, `NDrone2_Enable` 0x149dd0:27, TruckDriver states; cleared by `SetAsDead` 0x14a2f0:17 and death anims (Fade/FadeFast/SpecialDeath_Anim/TruckDriverInit) | [M*] |
| 0x4F8&0x100 | u32 bit | **AI active / vision gate** — the second half of the universal gate: `ProcessDroneSight:81`, `ControlSTANDARD` 0x1490f0:140 (skips tracking when clear), `GetOpponentInfo` 0x138f68:16, `DSTATE_Global` 0x15b830:52 (msg 10 self-wake needs it), `DSTATE_Disabled` 0x172440:24, `InitComms` 0x139520:166-170 (mission checks), `NotNearArmedDrone` 0x177640:71. Set by `DefaultInit` 0x14b300:60/:607 and `Enable(1)` 0x149dd0:27; cleared by `Enable(0)` :14 (`& 0xffe7fe5f`) | [M*] |
| 0x4F8&0x200 | u32 bit | **dead / corpse** (port has no reading): set by `SetAsDead` 0x14a2f0:9; kills vision eligibility (0x600 gate), `DoCollision` 0x149ec8:11 skips collision, `DoGravity` 0x141e58:16 returns 0 (except level 0x700004a), `NDrone2_Collision` 0x140e90:64 gate, `Enable(1)` refuses re-enable 0x149dd0:16 | [M*] |
| 0x4F8&0x400 | u32 bit | **died-this-frame**: `PostDroneControl` 0x14aff8:9-13 — if 0x100 set, health (+0xAC) ≤ 0 and not yet 0x200 → set; same spot sets obj+0xFA \|= 0x20 | [M*] |
| 0x4F8&0x800 | u32 bit | **cover node assigned** (port's "claimed-run" is wrong): set after picking a CoverNode → `FindCover` 0x152548:197, `FindAmbushCover` 0x152958; cleared when released `FindCover:171`/`DisownCover` 0x155858:18 (with +0xBB4 node); tested by `CoverAvailable` 0x155a38:16 (don't re-search), `EnemyLookForOpponent` 0x177ed0:52, `ReactToOpponentSighted`, `FindCover:147` | [M*] |
| 0x4F8&0x1000 | u32 bit | **in-cover latch** (port's "has-cover" — the pair 0x800/0x1000 splits the concept): `FindCover` 0x152548:100 early-outs when set (param_2==0), `CoverNodeOK` 0x151d68 tests, cleared with 0x800 whenever cover is disowned/reassigned (`FindCover:172`, `DisownCover:19`, `FindAmbushCover`, `UnderCoverInit` 0x169cc0, `RunForCover`, `BotCoverInit` 0x170448 `& 0xfffff7ff`) | [M*] |
| 0x4F8&0x2000 | u32 bit | **may-notice / can-be-alerted** (port confirmed): set in the ENTER arm of the 31 unaware states (Idle, Patrol, Alert, HeardNoise*, Civilian*, Interogate*, SearchArea, SniperIdle, PartyGirl, KikoMission, TruckDriverIdle, PlayScript, SpaceDrake…); `DroneVision_ConsiderAlerted` 0x176250:35 requires it (or team==2 refuses); cleared on every state change by `Drone_SM_RouteMsgDCV` 0x171df8 (0x172060-70) (`& 0xffffdfff`) | [M*] |
| 0x4F8&0x4000 | u32 bit | **collided-this-tick (transient)**: cleared at the top of every `Drone_CollisionHandler` 0x140968:70 (`& 0xffffbfff`); tested only via the 0x4200 mask in `NearArmedDrone` 0x177420:89, `NotNearArmedDrone` 0x177640:69, `SetGoalToDroneWithLosToOpponent` 0x1543f8:71. **No setter found** — flag | [M1 clear+tests; setter missing — flagged] |
| 0x4F8&0x8000 | u32 bit | **head-track anim active**: `DroneAnim_SetHTAnim` 0x13b1c8:138 sets / :147 clears with the param_3&1 arm; cleared by `SetAsDead` 0x14a2f0:10 and `DSTATE_HostageDead` 0x172628 | [M*] |
| 0x4F8&0x10000 | u32 bit | **state wants sight-processing** (port's "impact-flag" refuted): set in the ENTER arm of 136 of the 250 DSTATEs (e.g. `Idle` 0x15bf18:30, `PlayScript` :24, `BotGlobal` 0x16e060:134/604), i.e. re-armed right after `Drone_SM_RouteMsgDCV` 0x172054-70 clears it (`& 0xfffeffff`; `BotGlobal:158` also clears) on every state change; `ProcessDroneSight` 0x176a28:100,172,216 only runs SeekOpponentLOS/HaveOpponentSight/sight counters for drones with it set; `BOT_handleOpponentHistory` :76 tests | [M*] |
| 0x4F8&0x20000 | u32 bit | **alerted-by-other** (port confirmed): set by `ConsiderAlerted` 0x176250:201 when an ally alert (msg 0x1F / sight of other drone) is accepted; `DetermineAlertCheck` 0x1766f8:12,27 refuses when either drone has it; cleared by `DroneFunc_DoForcedAttack` 0x148c60:29 (`& 0xfffdffff`) when it force-attacks | [M*] |
| 0x4F8&0x80000 | u32 bit | **anim system active**: `Drone_Control` 0x139268:69 gates the whole custom-anim branch on it; `DoAnimation` 0x149f88:8 returns 0 when clear; set by `DefaultInit` :60 and `Enable(1)` :27 (0x801e2/0x801a0), cleared by `Enable(0)` :14 | [M*] |
| 0x4F8&0x100000 | u32 bit | **bones updated this frame**: set in `Drone_Control` 0x139268:93 right after `AnimObjectUpdate`+`SnapRotate2`; `GetBonePos` 0x149bd0:21, `GetHeadPosOld` 0x141150:39, `SetOpponentAimPos` 0x141fe8:69,171 fall back to non-animated positions when clear | [M*] |
| 0x4F8&0x200000 | u32 bit | **attack-shout broadcast latch** (team-1): `FirstAttack` 0x148010:86-89 and `FirstSightState` 0x147c48 set+test around `NDrone2_DroneAlertToObject` (msg 0x16); also `ReactToOpponentSighted` 0x1770f0, `SendAttackMessage` 0x14a410, civilian DSTATEs | [M*] |
| 0x4F8&0x400000 | u32 bit | **alert-sound latch**: `AlertSound` 0x177800:21 and `ReactToDroneAlertMsg` 0x1437a0:66, `HandleSoundAlerts` 0x148a30 test-then-skip; cleared by the combat/roam state enters (`Attack` 0x164210, `ActionAnim`, `GoToGoalPosition`, `PlayScript`, `RunAwayFromObject` `& 0xffbfffff`, `CivilianScared` 0x15d3a0:28 sets it) and by `HandleImpact` 0x1469b8:124 on death | [M*] |
| 0x4F8&0x800000 | u32 bit | **alert-broadcast busy latch**: `EnemyAlerts` 0x177c18:21,89 and `ReactToDroneAlertMsg` 0x1437a0:119,175,224,269,302 skip all alert msgs (0x11/0x12/0x13/0x15/0x1E) when set; only setter `CivilianScared` 0x15d3a0:29 | [M*] |
| 0x4F8&0x1000000 | u32 bit | **knocked-out** (port confirmed): `DSTATE_KnockedOut_Anim` 0x172a30:17-21 zeroes +0xAC health, sets it, drops weapon, logs `PlrStat_LogEnemyDisabled`; `OnInitDeath` 0x145bd0:22,57 and `SetAsDead` 0x14a2f0:21 branch on it (KO death ≠ kill); `LogDroneDeath` 0x14ab00:28 suppresses the "enemy dispatched" stat; cleared by `Drone_Control` 0x139268:74 | [M*] |
| 0x4F8&0x2000000 | u32 bit | **first-attack-done latch** (port "first-attack-other"): `FirstAttack` 0x148010:17 early-outs when set, :37/:63 sets it when committing the first attack; `SetAsAttacking` 0x14aed8 sets; `HandleImpact` 0x1469b8:109,226,291 clears on death (`& 0xfdffffff`) | [M*] |
| 0x4F8&0x4000000 | u32 bit | **sighting-happened** (port confirmed): set by the first-sight paths (`EnemyLookForOpponent` 0x177ed0:77, `FirstAttack` :42/:68, `AlertSound` :69, `EnemyAlerts` :113, `HeardNoiseAlert/Suspect`, `AlertToPosition`, `SearchArea`, `HostageHide`, `SetAsAttacking`); readers: `HandleImpact` 0x1469b8:80,105,222,287 (death handling splits on it: with it → death anim 0x14/0x15 path writing +0x5A2=0x14/+0x5A4=0x89; without → obj+0x10=0x14), `DoHeadTrackingBehaviour` 0x147578:28 disables head-tracking when set, `EnemyRunToPoint`, `AlertSound:48`; cleared by `HandleImpact:124` (`& 0xfbffffff`) and `DSTATE_HostageIdle` 0x15ce60:20 | [M*] |
| 0x4F8&0x8000000 | u32 bit | **own-first-sight** (port confirmed): set with 0x4000000 by `EnemyLookForOpponent` 0x177ed0:79 and the civilian enter arms; `FirstAttack` 0x148010:23 routes to `FirstSightState` when set (vs `FirstAlertState` for the sound-alert path) | [M*] |
| 0x4F8&0x10000000 | u32 bit | **sound-alert pending**: set by `AlertSound` 0x177800:70, `AlertToPosition`, `HeardNoiseSuspect`, `SearchArea`; consumed by `FirstAttack` via the 0x70000000 test → `FirstAlertState` | [M* writers; reader shared] |
| 0x4F8&0x20000000 | u32 bit | **forced-attack pending**: set only by `DoForcedAttack` 0x148c60:30 (as 0x24000000 with 0x4000000) after clearing its target's 0x20000; consumed by the same 0x70000000 test | [M*] |
| 0x4F8&0x40000000 | u32 bit | **alarm pending**: set by `EnemyAlerts` 0x177c18:114 and `DefaultInit` 0x14b300:687 (0x44000000 bundle); consumed via 0x70000000 | [M*] |
| 0x4F8&0x80000000 | u32 bit | **AI bounds-push** (port confirmed): set by `DroneInit_Collision` 0x14b1e8:39, `BOT_postLoadInit` 0x1262b8:10, `NinjaAttack` 0x16bb50:25, `NinjaStand` 0x16b990; read as *signed < 0* in `NDrone2_Collision` 0x140e90:169 → `AINetwork_GetBoundsPushVectorForSphere` push; cleared by `SpecialDeath_Anim` 0x163c68:40 (`& 0x7fffffff`) | [M*] |

Multi-bit masks: `0x600` = dead+died gate (universal); `0x4200` = may-notice|collided, excludes unaware/colliding drones from
armed-peer selection (NearArmedDrone/NotNearArmedDrone/SetGoalToDroneWithLosToOpponent); `0x2020000` = DoForcedAttack
0x148c60:28 tests `(word & 0x2020000) == 0x20000` (alerted-by-other set, first-attack clear); `0x70000000` = any pending alert
(`FirstAttack` 0x148010:24); `0x1a0`/`0x801a0`/`0x801e2`/`0xffea05e2|0x801e2` = Enable/DefaultInit bundles (see bits
0x20/0x40/0x80/0x100/0x80000); `0xffe7fe5f` = Enable(0) clear-bundle; `0x24000000`/`0x44000000` = forced-attack / init
sighting+alarm bundles; `0x7fffffff` = SpecialDeath_Anim clears bounds-push.

## Group 2 — `Drone+0x2B0` (hostage/killer partner) and `Drone+0x2B4` (last-hit-by / killer)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x2B0 | obj_tag* | **hostage↔killer partner obj pointer** (mutual pairing): `NDrone2_SetupHostageKiller` 0x144030:23-67 scans the list for a drone with +0x2B0 == 0, then writes `killer_drone+0x2B0 = hostage_obj` and `hostage_drone+0x2B0 = killer_obj`. Readers: `DSTATE_HostageKiller` 0x15c728:42-43 (msg 0xB to partner's SMI via +0x2B0→obj+0xE0→SMI) and :64 (partner's health via +0x2B0→obj+0xE0→**drone+0xAC** float — this is the brief's "HostageKiller case 10 +0xAC" hit: the health lives on the **drone**), `SetupHostageExecute` 0x14a808:19-22, `MonitorTalkToMissionObj` 0x144340:29,45 (`Drone_Message(0x1A)` to partner; partner's props), `DSTATE_Hostage` 0x15cb78, `DSTATE_HostageSaved` 0x15ccf0, `DroneAnim_EventFunc` 0x13b750, `Drone_Delete` 0x136ce8:65-67 (clears if it pointed at the deleted obj) | [M*] |
| +0x2B4 | obj_tag* | **last-hit-by (killer) obj pointer**: writers — `Drone_BulletHit` 0x13a9f8:21-23 `drone+0x2B4 = *(HITDATA+0x30)` (attacker obj; also +0x1D8 impact-count++), `NDrone2_DealWithObjHit` 0x13fff0:302-304 `drone+0x2B4 = *(hitdata+0x30)` (melee/thrown path, +0x1D8++ at :303), `Drone_ExplosiveHit` 0x13aab8:20 `drone+0x2B4 = *(DroneImpact+0x20)` (guarded against self-blast while anim 0x10). Readers — `DroneFunc_OnInitDeath` 0x145bd0:18-19 (killer's obj class == 3 check), :50/:66 (killer == glb_players / DAT_00298fb4 / drone+0x170 → stat logging), `LogDroneDeath` 0x14ab00:16-28 (killer==player && not KO → `PlrStat_LogEnemyDispatched`), `HitDamage` 0x145558:58,87,121 (killer == drone+0x170 → `PlrStat_LogShotHitEnemy`), `SetDeathChannel` 0x13e660:22,36 (killer==player → mission-fail reason 1/2), `Drone_Delete` 0x136ce8:68-70 (dangling clear) | [M*] (port's `killer = +0x2B4` correct) |

Note: `+0x2B4` is an **obj_tag\***, not a drone pointer — every reader that wants drone fields goes through `+0xE0`
(and OnInitDeath:19 reads killer's `obj+0xFF` class directly).

## Group 3 — the alertness block `+0x500 / +0x504 / +0x50C`

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x500 | f32 | **resting/base alertness** — the level +0x50C relaxes toward: `ControlSTANDARD` 0x1490f0:65-79 — if `+0x500 < +0x50C`, `+0x50C -= 1/(FRAME_RATE·50)` per tick (slow decay); if `+0x50C < +0x500` → snap up to +0x500. Initialised from DroneTypeSettings: `DoModeSettingsNEW` 0x14c600:70-77 `+0x50C = +0x500 = table[property 0x20 of props(+0x4D8)]` | [M1 reader+writer; table DAT_002f5cc8] |
| +0x504 | f32 | **alt-behaviour resting alertness** — same table but from the *second* behaviour struct at `drone+0x4E8`: `DoModeSettingsNEW` 0x14c600:77-78. Reader: `GetDroneTypeAttackTypeFriend` 0x14cbd8:165 `if (+0x504 < 1.0) use +0x5A4` (picks the type's alt-attack state when the alt profile is not fully alert). **It is not the live alertness** — the port's `alertness = +0x504` is a mismatch | [M*] |
| +0x50C | f32 | **live alertness 0..1+**: raised to 1.0 when the drone attacks/sights (`FirstAttack` 0x148010:43/69, `FirstSightState` 0x147c48:40,48, `ReactToOpponentSighted` 0x1770f0:53,71, :88 sets 0x75, `EnemyAlerts` 0x177c18, `ReactToDroneAlertMsg` 0x1437a0:122,216, `BotGlobal`), set to 0 by `DSTATE_HostageIdle` 0x15ce60:19; gates sight/reaction thresholds — `OpponentVisibility` 0x175af0:31-56 (detection multiplier), `FirstSightState:26` (< 0.66 && dist < 2.0), `ReactToOpponentSighted:30,75-88` (0.66/0.75 branches), `DSTATE_Idle` 0x15bf18:40,74,88 (0.66 → suspicious vs attack), `ConsiderAlerted` 0x176250:62 (`>= 1.0` refuses), decayed by `ControlSTANDARD` (above) | [M*] |

The brief's "+0x504 alertness" and the port's `alertness = +0x504` should both read **+0x50C** (live) / +0x500 (base).

## Group 4 — `Drone+0x5A2` / `Drone+0x5A4` (u16 state slots)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x5A2 | s16 | **start/return state slot** (brief confirmed): written from the DroneTypeSettings row — `DoTypeSettingsOLD` 0x14d2e8:48/51 `+0x5A2 = row[param_2].start` (row stride 0xC) or fixed 0x85/0x89 specials; `Drone_CoderCreate` 0x13a108:64 takes it as param_7; `DefaultInit` 0x14b300:724 writes 0xBD for a mode. ~18 readers use it as the SetState target when the current activity ends — `GoToGoalPosition` 0x165640:60,181, `AllyGoToGoalPosition` 0x15f938:28,58, `PlayScript` 0x15ba68:146, `WaitSwitch` 0x15b960:35,47, `Global` 0x15b830:41, `FailMission` 0x173f68:14, `EnemyRunToPoint` 0x168910:35, `AlertToPosition` 0x165270:60,135, `AimStandDiscard` 0x166840:45,50, `HostageGoToGoalPosition` 0x165a68:31,56, `DrawWeapon` 0x166000:26, `RunAwayFromObject` 0x168aa8:51, `BotAttackChangeWeapon` 0x1750c8:29, `GetDroneTypeAttackTypeFriend` 0x14cbd8:128,169,172 (switch on it); combat writers stamp it with the resume state — `Attack` 0x164210:81 (0x68), `CombatOutOfRange` 0x164690:30 (0x7E), `HeardNoiseSuspect` 0x16b0e0:62,153 (0x65), `CivilianGuard` (0x56), `InterogateAssist` (0x3A), `HostageSaved` (0xE), `BotAttackReload` (0xCF), `DonePressAlarm` (0x12), `AllyLeadWait`/`AllyLeadBondCombat` (0x1F), `ReturnToPatrolPath` (0x14), `FirstAttack` 0x148010:104 (0x56), `HandleImpact` death path 0x1469b8:126 (0x14), `SearchArea` 0x165bf0:95,117 (+0x5A2 = +0x5A0), `ReachedDestNode` 0x153358:91, `BotGlobal` 0x16e060:654 | [M*] |
| +0x5A4 | s16 | **one-shot alt/attack state** (brief confirmed; port `alt state = +0x5A4` correct): written by `DoTypeSettingsOLD` 0x14d2e8:53-62 — 0 when `+0xC6` (saved type) == 0, else `row[c6].alt` falling back to `row[c5].alt`; death path `HandleImpact` 0x1469b8:110,227,292 writes 0x89; `EnemyMission`/`CivilianMission`/`HostageSaved` write it. **Consumers clear it after use** — `PreDroneControl` 0x148db0:58-82 (when tag word +0x13A ≠ 0: read +0x5A4, restore type +0xC6→+0xC5, clear +0x13A/+0x135/+0xC6/+0x5A4, if `+0xC48 == 100` switch props to +0x4E8 and set +0x29), `ChangeToAttackMode` 0x14a648:11-32 (same sequence), `FirstAttack` 0x148010:100-133 (same + returns the alt state, 0x56 if it was 0xB4); `GetDroneTypeAttackTypeFriend` 0x14cbd8:166 reads it as the candidate | [M*] |

Neighbour: `+0x5A0` (u16) is a third state slot — `SearchArea` 0x165bf0:95 copies +0x5A0 → +0x5A2 (the "original start state" restored before searching), `EnemyRunToPoint` 0x168910:38 saves +0x5A2 → +0x5A0 first. Not in the brief; mapped [M1×2].

## Group 5 — secondary offsets (best effort)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x14C | f32 | **hit-effect intensity scalar**: reset to 1.0 on every state change (`Drone_SM_RouteMsgDCV` 0x171df8:184 — the brief's [M-dis 0x17207c-88]), reduced by dodge/roll states (`RollLeftCrouch`/`RollRightCrouch` 0x167e90:17/0x167f80:17 → 0.5; `StrafeDodgeLeft/Right` 0x167b50/0x167cf0:25 → 0.0); passed as the intensity arg to `NDrone2_DoHitEffects` (`PunchImpact` 0x145f28:46, `BulletImpact` 0x1461c8:29) | [M*] |
| +0x154 | f32[3] | **per-player distance table** (3 players): `PreDroneControl` 0x148db0:37-47 writes `0x47c35000` (100000.0 = far) for absent players, else the distance (loop `glb_players[i]`) | [M1] |
| +0x19 | u8 | **captain/elite flag** (brief's "drop flag" is wrong): `DefaultInit` 0x14b300:498/502 clears it, :505-531 sets it from props 9/5/8 + `GameState._40_4_` ∈ {1,2,3}; when set, the anim set is remapped (0x5000038/0x500002f/…) and the `DroneCaptain_Mod_*` multipliers apply to health (+0xAC) and damage (+0x100) :556-560; `OnInitDeath` 0x145bd0:23,76,79,108 branches the death accounting on it | [M*] |
| +0x3A | u8 | **silent-death latch (explosive kill)**: set by `Explode_CollisionHandler` 0x17cde0:9-10 (one-shot around `Explode_Propagate`) and `HitDamage` 0x145558:78; suppresses `NDrone2_DeathTalk` in `OnInitDeath` 0x145bd0:106-108 and gates `DSTATE_Dead` 0x163fe8:53 / `BotDead` 0x171518:29,50 chatter; cleared by `DroneAnim_LocationDeathAnim` 0x13c780:36 | [M*] |
| +0x3B / +0x3D | u8 | death-flag fixups written by `Drone_SM_RouteMsgDCV` (0x171fb4-0x171ffc; next state ∈ {0x44,0x46,0x47,0x55} → +0x3D = 1, else +0x3B = 0) — already mapped in dcvars-statemachine.md; users: `DroneWeap_Fire` 0x17a8c0, `HandleFiring` 0x17a180 (+0x3D gates firing while dying), `SetCombatMoveAnim` 0x13cfb0, combat DSTATEs (+0x3B) | [M-dis, cross-ref] |
| +0x44 | u8 | **team** (1 = enemy, 2 = civilian/guard-friendly — checked with 0x2000 in `ConsiderAlerted:35`): read all over the drone family; `OnInitDeath` 0x145bd0:21/58 (`== 1` → `PlrStat_*`), `FirstAttack` 0x148010:86. Port ✓ | [M*] |
| +0x135 | u8 | **second switch-channel id** (placement prop 0x13 per `DefaultInit` 0x14b300:465-466; the live level value is cached at +0x137 = `switch_channels[+0x135]`): `PreDroneControl` 0x148db0:51-61 — if `+0x135` set and `switch_channels[+0x135] != 0`, consumes the alt-state (+0x5A4) and clears +0x13A/+0x135; `DroneFunc_HostageSaved` 0x1441c0:11-18 raises `switch_channels_time[+0x135]`; `DSTATE_HostageSaved` 0x15ccf0:24 gates on it | [M*] |
| +0x13A | u16 | **attack/second tag word** (placement prop 0x15, `DefaultInit:471`): `PreDroneControl` 0x148db0:52-61 — 0xC = force-attack path (`DroneFunc_DoForcedAttack`), any nonzero = consume +0x5A4 alt state; cleared by `FirstAttack` 0x148010:36,62, `SetAsAttacking` 0x14aed8:8; read by `DoModeSettingsOLD` 0x14cfa0:28 and `GetDroneTypeAttackTypeFriend` | [M*] |
| +0x1D8 | s16 | **impact counter**: ++ in `NDrone2_DealWithObjHit` 0x13fff0:303 and `Drone_BulletHit` 0x13a9f8:22 (every hit taken); read back at 0x13fff0:272-296 (hit-state machine) and via the u64 read at `DroneWeap_DoOpponentTargetting` 0x179a48:65 (bytes +0x1DE/+0x1DF) | [M1] |
| +0x2B8 | u32 | **armed-peer snapshot** (brief already mapped): `NearArmedDrone` 0x177420:55 (zero) / :126 (store), compared in `NotNearArmedDrone` 0x177640:98 | [M*, cross-ref] |
| +0x2C4 / +0x2C8 | u32 / u32 | **threat-decay stamp + counter**: `PreDroneControl` 0x148db0:22-33 — if `frame > +0x2C4 + 30` clears byte +0x2A; while +0x2A set, +0x2C8 counts frames, else resets to 0. (+0x2C4 written where the threat timer is armed; not in the extraction window) | [M1] |
| +0x470 | u32 | **hit-location selector** for impact/death anims: written by `DealWithObjHit` 0x13fff0:103, zeroed every frame by `PreDroneControl:22`; read by `DroneAnim_LocationDeathAnim` 0x13c780:14, `LocationImpactAnim` 0x13c8b8:40, `AstronautHitMove` 0x155f78:26 | [M*] |
| +0x4C4 | — | zeroed every frame by `PreDroneControl` 0x148db0:25; `SetCombatMoveAnim` 0x13cfb0:399 stores a pointer-sized value, `ControlSTANDARD` 0x1490f0:214-216 adds/subtracts it as f32 to/from +0x4B4 — mixed-type reads, **unresolved** (Ghidra type confusion; flag) | [S] |
| +0x4D8 / +0x4E8 | ptr | **behaviour-struct pointers** (props): +0x4D8 = drone+0x4DC ("1st behaviour"), +0x4E8 = the alternate block; selected by `DoModeSettingsNEW` 0x14c600:73-78, `Set1st/Set2ndBehaviour` 0x14a5f0/0x14a618 (`+0x4D8 = +0x4DC` or `+0x4E8`, gated on +0xC48 == 100); property-0x20 of each seeds the alertness block (+0x500/+0x50C vs +0x504). 129/… files touch +0x4D8 (props 0x53/0x54 live behind it — cross-ref dronetypesettings-stable.md) | [M*] |
| +0x508 | f32 | **alertness snapshot**: `PreDroneControl` 0x148db0:19 copies +0x50C → +0x508 each frame (only access) — the per-frame "alertness at frame start" for the vision code | [M1] |
| +0x524 | u32 | **attack-start frame** (brief confirmed): set to `GameState._52_4_` if 0 by `FirstAttack` 0x148010:39-41,65-67 and `SetAsAttacking` 0x14aed8:11-12; `ConsiderAlerted` 0x176250:91 refuses alerts within 60 frames of it; `BOTWEAP_InitWeapon` 0x129e48 reads | [M*] |
| +0x710 | V3 | **post/hold position** for the idle-return states: `UnderCoverIdle` 0x169e58:82, `UnderCoverReturn` 0x16a9c0:39, `BotCoverIdle` 0x1705b0:44, `BotCoverReturn` 0x170b90:40 all `Vec_Dist2D(obj+0x30, drone+0x710)` ("am I at my post?"). Port's "goal point" comment is close; it is the undercover/cover-idle anchor specifically | [M1×4] |
| +0x830 | V3 | **cover position** (corner-cover stand point): written by `FindCover`/`FindAmbushCover` (`GetPosForCornerCover` into +0x830, then nudged 0x152548/0x152958:118-156), `Vec_Dist3D` checks against it; not the generic goal (that is +0xBE0 / AITarget at +0xA80) | [M*] |
| +0xAC | f32 | **health — on the DRONE struct** (brief's open question resolved): writers `DefaultInit` 0x14b300:52 (10.0), :560 (× `DroneCaptain_Mod_Health` when +0x19), :713 (×3), :735/:749 (from settings, 100.0), `HitDamage` 0x145558:154 (`health -= damage`), `KnockedOut_Anim` 0x172a30:17 (= 0), `SetAsDead` 0x14a2f0:22 (= 0 unless KO). Readers: the universal live-gate (`0.0 < drone+0xAC`), `DoGravity`/`PostDroneControl` (`<= 0.0` → dead), `HandleImpact` 0x1469b8:75 (flinch compare), `MP_HitBy`/`MP_BluePrintReachedBase`, `HostageKiller` 0x15c728:64 via +0x2B0→obj+0xE0→**drone**+0xAC (the brief's "obj→?+0xAC" — the deref chain lands on the drone). Every drone-path reader that touches +0xAC goes through obj+0xE0 first; no obj_tag-relative +0xAC health reader found in the drone family | [M*] |
| +0xBE0 | V3(+f32,+u32) | **special-death goal block**: `DefaultInit` 0x14b300:484-490 copies 12 bytes (+0xBE0 pos) + yaw at +0xBF4 + script id at +0xBD4 from the placement row; `SpecialDeath_Anim` 0x163c68:32-47 walks to it (`Vec_Dist2D < 1.0` + angle check), then plays `SetScript(+0xBD4, 0x40, …)`, clears bit31 and gravity bits | [M*] |
| +0xC48 | u32 | **behaviour-block selector** (100 = 2nd block active): `Set1st/Set2ndBehaviour` gate on `!= 100` (0x14a5f0:6, 0x14a618:6); consumers switch props when == 100 — `HandleImpact` 0x1469b8:114, `PreDroneControl` 0x148db0:72-81, `FirstAttack` 0x148010:117-127, `ChangeToAttackMode` 0x14a648:29-32 (then `+0x4D8 = +0x4E8`, `+0x29 = 1`) | [M*] |
| +0xC5 / +0xC6 | u8 / u8 | **active type id / saved (alt) type id**: +0xC5 written by `DoTypeSettingsOLD` (from settings; 0x17/0x18/0x19 specials), `Drone_CoderCreate`; +0xC6 = the saved type restored over +0xC5 when the alt state (+0x5A4) is consumed (`PreDroneControl` 0x148db0:64, `ChangeToAttackMode` :14, `FirstAttack` :109) then +0xC6 = 0; +0xC5 == 0x1E marks MP bots (`BOT_getMovementSpeedMul` callers 0x14f640:68 etc.), 0x02/0x04/0x05/0x11/0x13/0x16 type checks throughout; a third slot +0xC4 feeds `GoToGoalPosition` 0x165640:78-80 (`+0xC5 = +0xC4` after `Set2ndBehaviour`) | [M*] |
| +0xCC0/+0xCCC | timers | timer C/D triples (+0xCC0 armed byte, +0xCC4 deadline, +0xCC8 payload; +0xCCC/+0xCD0/+0xCD4) → msgs 0xC/0xD in `PreDroneControl` 0x148db0:93-122; cleared on state change 0x17207c-88 — already mapped (drone-statemachine.md); port leaves timer D armed across transitions (mismatch noted there) | [M-dis, cross-ref] |
| +0xCF8 | f32 | **civilian armed-person scare radius**: `DSTATE_Civilian` 0x15d118:28-34 enter — 10.0 (0x41200000) on level 07000007, else 20.0 (0x41a00000); consumed at :45-47 `NDrone2_NearArmedPerson(+0xCF8, dc)` (case 3). Only writer+reader. Port ✓ (drone.rs:635-636) | [M*] |
| +0xD1C | ptr | **MP-bot data pointer**: `BOT_init` 0x1249e8:153 `drone+0xD1C = BOT_vars + i` (per-bot slice of a static array); read by the whole BOT_* family (161 hits: handleOpponentHistory 0x1257c0:80, validateStateChange, setOtherPlayerInfo, …; `Drone_SM_SetState`'s +0xD1C→+0x762 script escape per dcvars doc). **Highest offset touched in the drone family** — the 0xD20 extent ends exactly above it | [M*] |

### Struct size and function pointers

`control_create_object` 0x1344e0 mallocs `size+0x110` = **0xE30** and stamps `*(obj+0xE0) = obj+0x110` [M-dis, dcvars doc];
drone extent = 0xE30 − 0x110 = **0xD20** [M]. Highest drone offset any decompiled function touches: **+0xD1C** (u32/ptr,
BOT family; census over all Drone*/NDrone2*/BOT_* files finds nothing ≥ 0xD20) — consistent with a 0xD20-byte struct whose
last word is the bot-data pointer. Function pointers visible in the struct: **+0x108** = the embedded StateMachineInfo
(+0x130 inside it = `NDrone2_ProcessStateMachine`) [M-dis 0x17588c], **+0x530** = the anim-script object (`AnimScriptCall*`
targets, `SetHTAnim` 0x13b1c8:129-147), **+0x544** = per-drone update hook called as `(*(drone+0x544))(drone,1)`
(`Drone_Control` 0x139268:95-96). No C++-style vtable: the "vtbl" is the StateFuncs table at 0x29B300 + these three slots.

## Port cross-check (`crates/nf-game/src/drone.rs`) — report only

| Port item | Port reading | Verdict |
|---|---|---|
| `knocked_out` = 0x4F8&0x1000000 (drone.rs:649-651, 1742) | matches `KnockedOut_Anim` 0x172a30:21 / death accounting | ✓ |
| `killer` = +0x2B4 (drone.rs:566) | matches (obj_tag*, set in BulletHit/DealWithObjHit/ExplosiveHit) | ✓ |
| `stationary` = 0x4F8&0x10 (drone.rs:583-587, 805-807) | matches; sole clearer HostageKillerAttack:38 not modelled (minor) | ✓ |
| `return_state` = +0x5A2 (drone.rs:507, 809) | matches | ✓ |
| `alt_state` = +0x5A4 (drone.rs:633, 793-800) | matches; consumption sequence (+0xC6→+0xC5, +0xC48==100 → props swap) only partly modelled | ✓ |
| `scared_radius` = +0xCF8 (drone.rs:635-636, 3118) | matches | ✓ |
| `second_channel` +0x135 / `second_tag` +0x13A (drone.rs:713) | matches (0x13A==0xC forced-attack path extra) | ✓ |
| `attack_frame` +0x524 (drone.rs:436, 708) | matches | ✓ |
| `cover_claimed` 0x800 / `has_cover` 0x1000 (drone.rs:490, 497-498) | 0x800 = cover **node assigned** (not "claimed this run" in the route sense), 0x1000 = in-cover latch — directionally right, comment should say "cover node assigned" | ~ |
| **`alert_block` documented as +0x504** (drone.rs:626, 634) | **mismatch**: live alertness is **+0x50C** (base +0x500, alt-profile base +0x504). Any port logic that treats the field as the per-frame alertness (vision thresholds `OpponentVisibility`, `ConsiderAlerted >= 1.0`, `FirstAttack` setting 1.0) is reading the wrong slot; +0x504 only feeds `GetDroneTypeAttackTypeFriend` | ✗ drone.rs:626/634 |
| **`in_air` = 0x4F8&0x4000 "down-ray missed"** (drone.rs:643-644, 2153) | **unsupported**: census finds no setter of 0x4000 anywhere; the only clear is `Drone_CollisionHandler` 0x140968:70 (collision-tick start) and the only tests are the 0x4200 armed-peer masks. The port's down-ray semantics is its own invention | ✗ drone.rs:643 |
| 0x4F8&0x10000 modelled as `vision_on` (drone.rs:2453, 2922…) | matches the census reading ("state wants sight-processing", cleared on state change 0x172054-70, re-set by 136 DSTATE enter arms) | ✓ |
| +0x2B0 hostage/killer pairing | not modelled in drone.rs (no hit in the grep) — needed only if the hostage system gets ported | (gap) |

## Disagreements

1. **Brief's "+0x504 alertness"** — the live alertness float is **+0x50C**; +0x500 is the resting level, +0x504 the
   *alternate behaviour block's* resting level (props at +0x4E8). Port `alert_block` inherits the wrong comment.
2. **Brief's "0x4F8&0x4000 in-air-this-tick" / port `in_air`** — no setter exists in the decompile; only a per-tick clear
   (Drone_CollisionHandler:70) and the 0x4200 tests. Reading it as "in-air" is [S]-grade; same for bit0 and 0x4000's
   missing setters (bit0 tested 3×, never set; likely written by a path Ghidra rendered as a wider/block store).
3. **Brief's "+0x19 drop flag"** — it is the **captain/elite flag** (DroneCaptain_Mod_* multipliers, DefaultInit:505-560).
4. **Brief's "0x4F8&0x800 claimed-run"** — it is "cover node assigned" (FindCover:197); the run-claim reading fits nothing.
5. **dcvars-statemachine.md says SMI+0x1C/+0x20 (drone+0x124/+0x128) are unreferenced** — `DroneFunc_DoForcedAttack`
   0x148c60:31 writes `*(SMI+0x1C) = 0x56` (`param_1[3]+0x1c`); their census grepped `+ 0x124)` via the *drone* base and
   missed the SMI-relative access. The port should not treat +0x124 as scratch.
6. **Brief's "HostageKiller reads opponent's obj→?+0xAC"** — resolved: `HostageKiller` 0x15c728:64 reads
   `+0x2B0 → obj+0xE0 → drone+0xAC`; the health float lives on the drone (writers listed above), not on obj_tag.

## Live checks for the main agent

Pointer chain: u32 **0x298E14** = first Drone_tag* (07000005 start: 0x016EDA50); drone+0x04 next, +0x0C obj, obj+0xE0 → drone
[dcvars doc]. Suggested situations: 07000005 at start (13 drones; guards Idle(4), ch-waiters WaitSwitch(1), drone 2
HostageKiller(7), sniper last node in 0x29), then shoot a guard.

1. **0x4F8 word census (PINE `dump`/gdbwatch readwrite, a few seconds of play)** on drone 0x016EDA50+0x4F8:
   expected at start ≈ 0x000921E3 ({0x2,0x20,0x40,0x80,0x100,0x80000} from DefaultInit:60 `& 0xffea05e2 | 0x801e2`, :607 `| 0x1a0`,
   plus 0x2000|0x10000 from the Idle enter; no 0x10 for normal guards — snipers/HostageKiller would add 0x10). After Bond is spotted: 0x4000000|0x8000000 set,
   0x10000 stays set; after death: 0x200 set, 0x20/0x80 cleared (SetAsDead), 0x4000000/0x2000000 cleared (HandleImpact).
2. **gdbwatch `watch w drone+0x4F8`**: writers should be `Drone_SM_RouteMsgDCV` 0x172054-70 (clears 0x10000|0x2000 on state
   change), the DSTATE enter arms (0x10000/0x2000), `Drone_Control` 0x139268:74/93 (0x1000000 clear / 0x100000 set),
   `PostDroneControl` 0x14aff8:12 (0x400 on death), `FindCover`/`DisownCover` (0x800/0x1000). Anything else = census gap.
3. **Poke-style**: pin `drone+0x50C` (live alertness) to 0.0 with `poke --shot` while a guard patrols → his detection
   (`OpponentVisibility` multiplies by it) should collapse; restore to 1.0 → instant spotting at the same distance.
   The +0x504-vs-+0x50C question is live-checkable the same way: poking +0x504 should do nothing to detection.
4. **+0xCF8**: on 07000007 (civilian level), `dump` a civilian drone's +0xCF8 — expect 10.0; on other levels 20.0
   ([L-read] candidate; writer 0x15d118:28-34 runs on the Civilian ENTER only).
5. **bit0/0x4000 setter hunt**: gdbwatch `watch w drone+0x4F8` (full word) while Bond pushes a guard / a guard is stunned —
   if any write appears from neither 0x172054 nor the enter arms, capture the PC; that is the missing setter for 0x1/0x4000.
6. **+0x2B4**: shoot a guard and `dump` drone+0x2B4 — expect Bond's obj (from HITDATA+0x30); KO him with a taser and
   re-check after he recovers (the value persists; only a new hit or `Drone_Delete` rewrites/clears it).
7. **byte range / access type**: whole drone = obj+0x110 .. obj+0xE2F (0xD20 bytes), all u32 fields little-endian; +0x4F8
   word is only ever accessed as u32 (plus one u64 read at DoForcedAttack:28 and the signed-`<0` bit31 test at 0x140e90:169).

## Port fixes driven by this map (044y-fields-actors)

- `alert_block`'s doc said `+0x504`: the live alertness is `+0x50C` (the port's `alert` field, already used by the
  vision code), `alert_block` is the resting value `+0x500`, `alert_floor` is the alt profile's `+0x504` — comment
  fixed (drone.rs `Data` field docs).
- `in_air` claimed to be `0x4F8 & 0x4000`: that bit has **no setter** in the decompile (only the per-tick clear
  0x140968:70 and the `0x4200` tests). The field is now documented as the port's own down-ray model; no behaviour
  change.
- `0x4F8 & 0x800` / `0x1000`: the port's comments ("claimed this run" / "has cover") corrected to the census reading
  — 0x800 = cover **node assigned** (FindCover 0x152548:197), 0x1000 = the in-cover latch.
- `Drone+0x19` is the **captain/elite** flag (DroneCaptain_Mod_* multipliers, DefaultInit 0x14b300:505-560), not a
  drop flag — the port's `captain` field doc already says `+0x19`; the brief's "drop flag" was wrong.
- `AllyLeadMissionWait`'s wait channel: the port read `second_channel` (+0x135); the game latches `Drone+0x12C`
  (0 → 0x99) [M 0x15efc8:17-20]. Fixed: new `mission_wait_channel` field models +0x12C; test
  `fields_actors_live::a_state_change_clears_both_message_timers`.
- `DroneFunc_OnInitDeath`'s mission-fail corners (`ally.rs::on_init_death`): the prop-0x54 + non-drone-kill arm
  re-checks prop 0x53 (unset → no fail) [M 0x145bd0 LAB_00145cc8 / LAB_00145c3c], prop 0x53 + a drone killer gives
  reason 2 (team 1) / 1 (team 2/3) [M the bVar7 gotos], and the KO check opens the team-1 arm only [M 0x145bd0:20-22]
  (the caller now passes it). Test: `ally::tests::oninitdeath_corners_match_the_decompile`.
- Every state change now clears **both** message timers and resets the hit-effect multiplier `+0x14C` to 1.0
  [M-dis 0x17207c-88] — the port cleared only timer C and never reset `damage_taken`. Test:
  `fields_actors_live::a_state_change_clears_both_message_timers` (fails before the fix).
- Not fixed (documented, need a bigger change): no msg state-filter (msg+4 ∉ {0, 0xC5, current} should drop,
  [M-dis 0x171e28-44]); `sm_id` = list index vs the game's monotonic u16 instance counter [M 0x13cc70]; the
  death-flag fixups +0x3B/+0x3D are covered by the port's `firing = false` on death enters, not modelled byte-exact.
