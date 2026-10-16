# Uncited ACTION.ELF gameplay audit (task 710, 2026-10-16)

Branch `s/uncited-gameplay-audit`. The 2026-10-16 coverage pass counted 1979 of 3038 ACTION.ELF
functions cited by the port. This audit re-derived that number, worked through the uncited
gameplay groups, added citations where the port already covers the function, and ranks the real
gaps by player impact with ready-to-queue task descriptions.

## Method

- Function list: the 3038 decomp files in `work/decomp/ACTION.ELF/` (address + base name from
  each filename, base = the name before `__` mangling).
- A function counts as cited if its base name or its address (`0x…`, short or 8-digit form)
  appears in `crates/`, `docs/`, `tools/` or `tests/`.
- Script: `/home/z/my-project/scripts/uncited_inventory.py` (worker scratch, not in the repo).
- Before the citations below: **1984 cited / 1054 uncited** (65%). After: **2063 / 975** (68%)
  — 79 functions newly counted, all of them real equivalents that already existed in the port.
  The residual delta vs the coverage pass's 1979 is method noise (their scorer differs slightly;
  e.g. it may not scan `tools/`).
- Inventory false positives this audit found and corrected: `TextBox_Update` and `C_NIS_Handler`
  were "cited" only via mentions in `docs/WORKLOG.md` (a task plan, not a citation); both are now
  cited for real at their port equivalents (see below).

Everything below carries the [M]/[D]/[L]/[S] tags. Decomp C was cross-checked against caller
greps over `work/decomp/ACTION.ELF/`; where the decompile looked incomplete the verdict says so.

## Group summary (uncited counts before this audit)

| Group | Uncited | Verdict |
|---|---|---|
| NDrone2 (states, DMODE inits, helpers) | 115 | ~30 covered → cited; rest: 6 real gaps, the rest dormant/debug/dead |
| MP_ | 26 | ~17 covered → cited; gaps are the live-host wiring + bot objective scan |
| Drone* helpers | 19 | 12 covered → cited; gaps: astronaut moves, talk trio, FirstAlertState |
| AINetwork_ | 16 | 13 covered → cited; gaps: DROUTE_Nearest pull-back, RouteNodeOffset bounce |
| Script_ | 13 | none covered — a different "Script" family: the frontend menu-def keyframe runtime |
| BOTWEAP | 11 | 3 partial → cited; weapon-choice family is a real bot gap |
| BOTSTATE | 10 | 4 partial → cited; combat-move family is a real bot gap |
| Creature_ | 5 | dead on retail data (no level ships placement type 0xe0) — document, don't port |
| camera_ | 5 | 2 reachable (track, mode 2) via NIS event 9 — unported; 3 are dead code |
| Radio_ | 4 | behaviour hand-rolled per page in the port → cited |
| BOT_ | 1 | BOT_fellOutMap missing (bot kill-plane recovery) |
| TextBox_Update, C_NIS_Handler | 2 | TextBox covered → cited; C_NIS_Handler route unported |

---

## Ready-to-queue tasks for the top 10 gaps (ranked by player impact)

### 1. AI alarm-raising: FindAlarmPoint → DSTATE_PressAlarm → DonePressAlarm
- **Missing behaviour:** guards whose goal is an alarm never raise it. The port's RunToAlarm
  (0x89, `kiko.rs`) only models the none-found arm → Idle. In the game the found arm routes to
  the closest type-8 AI point (`NDrone2_FindAlarmPoint` 0x153008 = `ClosestAIPoint(8)` +
  `MoveToGoalPosition`), and arrival enters `NDrone2_DSTATE_PressAlarm` (0x168ea8, state 0x8A):
  ENTER plays the press anim (0x21 on 07000009/0A, else 0x22), TIMER_C plays 0x3c with follow-on
  0x8B DonePressAlarm, then non-civilians raise the alarm (`+0x594=0x32`) and
  `SetMissionFailReason(0x10)` — the nearest alarm switch's channel ("Alarm Raised", actors.md
  §3) [M 0x168ea8 case 0xc; entry [M 0x1736f8 → 0x165640]].
- **Player impact:** alarm-gated reinforcement spawners and the "Alarm Raised" mission fail
  never fire from AI behaviour on levels with alarm switches — the stealth fail loop is missing.
- **Approach:** add the type-8 AI-point registry to the nav data (the port's paths already carry
  `kind`; verify type-8 points exist in the disc's AI networks), port the found arm of
  RunToAlarm and the 0x8A/0x8B states; RED test = a level with an alarm switch where a guard
  sees the player then must reach + press the switch (channel asserts).
- **Acceptance:** guard raises the alarm headlessly (channel bit asserts) within N ticks; suite
  green.

### 2. Blast stagger (DSTATE_ExplosiveImpact) and smoke (SmokedOut) for guards
- **Missing behaviour:** grenades stagger nobody and smoke does nothing to guards. The game:
  HandleImpact msg 9 → `NDrone2_DSTATE_ExplosiveImpact` (0x172fb0, state 0x54; ENTER re-runs
  `NDrone2_ExplosiveImpact(impact,0,1)` — knockback + DeathByExplosion 0x45 anim when lethal)
  [M 0x1469b8:173-201]; msg 0x17 (smoke; props 0x36/0x34 immune) → `DSTATE_SmokedOut` 0x7B →
  0x7C Loop (re-arms on further 0x17) → 0x7D Recover: cough anim 0x20 until the smoke clears
  [M 0x1469b8:203-247; 0x16b878 case 0xc]. `NDrone2_ExplosionDir` (0x14aba8) supplies the
  blast direction.
- **Player impact:** core utility (grenade/smoke) feedback on every combat level.
- **Approach:** route the port's explosion msg into a stagger state (the port currently applies
  blast damage as bullet msgs, drone.rs:6569-6581); smoke msg into cough states with the prop
  immunity gate.
- **Acceptance:** headless test — grenade at a guard → state 0x54-equivalent for its duration;
  smoke at a guard → cough then recover, prop-0x36 guards unaffected.

### 3. Ninja melee chain for the assassin/chaser fights
- **Missing behaviour:** the ported NinjaAttack router (kiko.rs ports 0xa6-0xaa + 0xb2) collapses
  the close band into standing and shooting (0xb2 stand-in, kiko.rs:56-58). The game's close
  range enters `NDrone2_DSTATE_NinjaGetCloseToPlayer` 0xab [M 0x16cb70, entered from
  NinjaAttackShortRange 0x16c650]: anim 0x6B run-in, dist ≤ 2 → `NinjaSword` 0xac (anim 0x6d
  slash); `NinjaSomersault` 0xad, `NinjaBackflip` 0xae, `NinjaSideflip` 0xb3/Left 0xaf/Right 0xb0
  (evasions), `NinjaNoRoute` 0xb1 (star throwing with no path) [M 0x16cdd0..0x16d878].
- **Player impact:** the melee boss fights (assassin 07000004/16 chaser) visibly fight wrong at
  close range.
- **Approach:** port the 0xab/0xac pair first (run-in + sword), then the evasion flips as
  choose-combat-move alternatives; keep 0xb2 for the no-route arm.
- **Acceptance:** headless chase test asserts a sword/flip state is entered when dist < 2 and a
  star projectile spawns in the NoRoute arm.

### 4. Live MP host: run the match logic every frame
- **Missing behaviour:** `MpMatch::update_objects` / `player_killed` / `touch_*` /
  `damage_site` / respawn and `BotBrain` are test-only; nf-view's live host calls only
  `update()` / `register_*` / `set_slot` (lib.rs:977-1020, scene.rs:1852). No scenario can be
  scored or won in a real ride.
- **Player impact:** every MP mode degrades to spawn + clocks + debrief.
- **Note:** branch `s/mp-bots-in-match` (task 704) already wired menu→match settings→load and
  diagnosed the bots starving on empty pickup/opponent inputs; this task is the sibling wiring
  fix on the live host. Rebase onto it rather than redoing.
- **Acceptance:** a headless-vs-live parity test: a scripted CTF capture scores in the live host
  the same as in `mp_match_rules.rs`.

### 5. Bot objective-goal scan (BOTSTATE_pickGoal slot 1)
- **Missing behaviour:** the slot-1 objective scan is explicitly unlifted (bots.rs:1236-1240).
  Game: `BOTSTATE_pickGoal` 0x126b88:143-303 dispatches on scenario subtype 1-8 through
  `MP_getFlagObj` 0x18b978 / `MP_getBaseObj` 0x18b9a0 / `MP_getUplinkObj` 0x18b9c8 (negative
  index = first terminal not owned by my team) / `MP_getDemolitionObj` 0x18bab8 /
  `MP_getProtectionObj` 0x18bac8 / `MP_getGoldenEyeObj` 0x18baf0 (state ∈ {0,2}) /
  `MP_getHillObj` 0x18bb50; the blueprint arm is `BOTSTATE_processGoals` 0x127c78:185-193 +
  `MP_getEsponageBaseObj` 0x18a338. The getters' **data** already exists in the port
  (`MpObjectsRt`, rules.rs — cited this audit); only the scan + the goal wiring are missing.
  Also the Assassin hunt arm ("if I'm the assassin, hunt the marked target",
  `getPreferredTraitOpponentObjIndex` 0x128b50:80-86 via `MP_getAssassinTarget` 0x18bd78) is
  missing in `trait_pick_opponent` (bots.rs:1415).
- **Player impact:** bots in KOTH/Demo/Prot/Uplink/CTF/GE/Blueprint play deathmatch with props.
- **Acceptance:** per-scenario headless test: a bot picks the objective goal within N ticks and
  reaches the object (KOTH hold, site attack, flag return).

### 6. Bot combat moves under fire + weapon switching
- **Missing behaviour:** `BOTSTATE_reallyWantACombatMove` (0x1267a0) picks strafe/step/roll
  (0xD4/0xD5/0xDB/0xDC) with **Rand_Rand(5)** — note the port's choose_combat_move rolls
  rand(4); fix the stream when porting (RNG parity) — gated by `BOTSTATE_checkAttackMove`
  (0x126a40, incl. the KOTH hold-hill gate: on-hill bit 0x10 rejects hill-leaving moves,
  0x40000800/0x60001000 only) with evasive fallback 0xD9/0xDA and backoff 0xD7
  (`BOTSTATE_EvasiveMove` 0x129368). The port's `choose_combat_move` (bots.rs:1540) rolls
  rand(4) and always returns None → bots stand still under fire. Weapon side:
  `BOTSTATE_changeWeapon` (0x129680), `pickupWeaponChangeChoice` (0x129728, incl. the blueprint
  capture-weapon reward — `settings.capture_reward` is currently dead), the preference-ranked
  `BOTWEAP_listHeldLoadedWeapons` (0x12aa80, table @0x2f3e10), `BOTWEAP_isValidWeapon`
  (0x12b0f0), `punchIsBetterIfClose` (0x12b1b8), `chooseUnarmedAttackAnim` (0x1292f0, rand(4)
  punch anim 0x3D), `BOTWEAP_getWeaponAmmoAmount` (0x12afa0, clip+pool with paired-weapon
  sharing). Port partials cited: `BOTWEAP_WeaponHasAmmo` (clip-only), `BOTSTATE_hasWeapon`
  (current-weapon-only).
- **Player impact:** bots are static targets that keep their init weapon forever; pickups and
  the punch/weapon decision are the visible bot skills.
- **Acceptance:** under sustained fire a bot enters a combat-move state within N ticks; a bot
  walking over a better pickup switches weapons (assert weapon id); capture reward grants the
  weapon.

### 7. Astronaut zero-G guards for the space-station level
- **Missing behaviour:** whole actor class. Start state = DroneTypeSettings row 29
  AstronautLaunch 0xBD; skin kind 0x18 forces mode 0x1D + start 0xBD [M 0x14b300:722-727].
  States `DSTATE_AstronautCombat` 0xC0 (0x174458) / `AstronautCombatMove` 0xC1 (0x174550) /
  `AstronautHit` 0xBE (0x174690) / `AstronautDeath` 0xBF (0x1747b8) with the float movement
  `DroneMove_AstronautCombat` (0x154618) and bob/float anim `DroneMove_AstronautAnim`
  (0x155ea0). Level 0700001b ships ~20 suited guards [D dronetype-tables census].
- **Player impact:** the space-station mission's guards behave like ordinary walkers — gravity,
  hit reactions and deaths are all wrong for the setting.
- **Acceptance:** headless 0700001b load: astronauts drift (no floor snap), float anim runs,
  hit/death variants fire.

### 8. Frontend menu-def keyframe runtime (the uncited Script_* family)
- **Missing behaviour:** the 13 uncited `Script_*` (0x2201c8-0x2214c8) are NOT the NIS script
  player (that family 0x1d8248-0x1db9a8 is ported; nf-formats/src/nis.rs, docs/systems/
  script-player.md) — they are the M_MANAGER/M_CONTROL keyframe runtime that animates every
  menu control: level archives carry a kind-8 "menu def" whose −0x09/−0x0b/−0x0c tags
  (SCRIPT/KFRAME/KMSG) build per-control keyframe scripts; `Script_RunFrame` (0x220580, called
  from Page_Update 0x21edc8 every menu frame) interpolates the control rect (+0x70..+0x76) with
  line (`Script_InterpolateLine` 0x220bc8) or Catmull-Rom (`Script_InterpolateSpline` 0x2208b8,
  basis `Script_CalculateSpline` 0x221410), advances/loops keyframes (flag 1 spline, 2 loop) and
  fires keyframe messages (ids ≥ 0xe1 become delayed Process_Create timers); widgets select
  scripts via msg 0x21 (`Script_Select` 0x221250) and cancel loops via msg 0x14
  (`Script_ExitLoop` 0x2213d0); init at `Script_PrepareScripting` (0x220ed8) from Page_Init
  0x21f1c0 + the 12 widget `*_Init`s [M frontend.md:29-32, menu-sfx.md:42].
- **Player impact:** every menu page-open/close animation, the wheel slide, pulsing prompts —
  all static in the port today (menu.rs renders final states only). Constant, visible cosmetic
  gap; low functional risk (no gating found).
- **Acceptance:** parse the front-end 07000048 menu def, tick `Script_RunFrame` headlessly, and
  pin a control's rect at t=0/mid/end against the game's interpolation.

### 9. Guard barks: the talk trio + FirstAlertState
- **Missing behaviour:** `DroneFunc_UnderCoverTalk` (0x13fe80, 5 s global throttle),
  `DroneFunc_RunForCoverTalk` (0x13fd08, barks 0x11b+Rand(7) alt 0x122+ on lv 14-16, 0xbd on
  11-13/4a, 0xd0 on 0c-0d), `DroneFunc_UnderCoverAimTalk` (0x13fb98, 0xd0/0xba+Rand(2)) — the
  audible under-fire chatter on 6+ levels; and `DroneFunc_FirstAlertState` (0x147f58): state 0x39
  follow-on when the armed-intruder bit (+0x4FC 0x200000) is set, lv {05, 07, 41} (the port's
  state enum jumps 0x38→0x3f; FirstAttack itself is ported at drone.rs:4087). Line-set data is
  documented at docs/systems/speech.md:252 ("level-specific sets … not yet").
- **Player impact:** combat sounds empty without guard chatter; speech.md already lists the
  line ids, so this is data-plumbing + two states.
- **Acceptance:** headless: under-cover guard with a line set plays the bark within the
  throttle window; armed-intruder flag routes to state 0x39.

### 10. NIS event 9 (cam2mode) + the C_NIS_Handler cut-scene route
- **Missing behaviour:** (a) NIS event 9 is unhandled in `nis.rs::event()` — the game routes it
  via `Player_Cam2Mode` 0x195fc8 → SetCamMode(2) = `Player_PositionCamera` 0x18f5d8 case 2 →
  `camera_track_XZ_pos_behind` (0x1bc260) + `camera_track_Y_pos` (0x1bc1f0), the cut-scene
  behind-Bond orbit [M Script_EventHandler 0x1d94c0 case 9]. (b) `C_NIS_Handler` (0x205f10) is
  the in-game cut-scene menu page (0x1000003e): msg 0x51 binds the level's cut-scene controls
  (0x308840-0x308ca0) → script hashes, msg 0x4b (page shown) boots the level cut-scene
  (LoadableLoad → Script_Load → Script_Play → Camera_Enable(7)) — the mission-driven entry the
  port lacks (port entry is SP placements + loadable packs).
- **Player impact:** cut-scenes firing event 9 keep the eye camera instead of the orbit; any
  mission that boots a cut-scene through the page route never plays it.
- **First step (cheap):** disc census — which levels' menu defs bind page 0x1000003e, and which
  cut-scene scripts fire event 9 (determines whether (b) is worth queueing alone).
- **Acceptance:** (a) event-9 test pins the camera orbit params; (b) census-driven.

### Runners-up (queued behind the top 10)
- **DROUTE_Nearest dest pull-back** (`AINetwork_AlterDestFor_DROUTE_Nearest` 0x156088): drones
  chasing a target behind cover fail/stick instead of closing to the nearest reachable point.
- **CivilianMissionWait** (0x15dbb0, state 0x17): mission civilians must pause at wait-channel
  route nodes (CivilianMission msg 0x22 [M 0x15d860 case 0x22]); the port has no msg 0x22 —
  they walk through.
- **BOT_fellOutMap** (0x1259d0 via `Drone_FellOutMap` 0x13a878): kill-plane recovery — teleport
  to spawn, full goal-machine reset (+0x730=0xF9, +0x765=0xFF, +0x766=9), SetState(0xF9).
  Without it a bot that leaves the map wanders in the void forever.
- **Tase scream broadcast** (`NDrone2_DroneAlertToPosition` 0x14a498 on tase ENTER, prop 0x32):
  the port screams on bullet/punch only (cited partial at drone.rs handle_taser).
- **Door-guard pairing** (`NDrone2_DoorGuardHandler` 0x1416d8 + `SetDronePartner` 0x141360):
  CivilianDoorGuard 0x31 (0x160648) exists as a const with no arms (WORKLOG 3969).
- **MP_NeedSkin / MP_setLoadingSkins / MP_Pickup_PostLoadInit** (0x120c48 / 0x1261e8 /
  0x18b578): per-character anim-set loading + pickup AI-path emit — invisible until bots render
  and animate; queue with any bot-render work.
- **MP_objectBeingDeleted** (0x18ac90) / **MP_getObjExtObj** (0x18bca8): scenario-object
  deletion/re-resolve — only matters once live objects can be deleted (task 4 dependency).
- **Scary-link routing** (`AINetwork_SetLinksFlagInCircle` 0x15b548): the port scans radii
  directly (drone.rs:9370-9376 region) but A* ignores the link flags — guards can path straight
  through grenade radii.
- **State-based head height** (`DroneAnim_CurrentMoveState` 0x13e378): aim/LOS accuracy nicety.

### Dormant, debug or dead on retail — document, don't port
- **camera_fixed (0x1ba4e8) / camera_snake (0x1baa78) / camera_chasecam (0x1b9c00): dead code.**
  No writer sets PositionCamera modes 4/5/6/0xa anywhere in ACTION.ELF (all 19 SetCamMode sites
  use 0,1,2,0xb,0xc,0xd,0xe,0xf) [M]. The under-door/vent snake view is NOT camera_snake.
- **Creature_* (0x134cf0-0x135340): dead on retail data.** Placement type 0xe0 (swimming
  wildlife, spline waypoints, water-gated, contact hurt 2.0) appears in no audited level's
  placement set [D docs/levels/* census]; `DynamicObject_Create` 0x18dbf8 has no 0x23 path.
- **DSTATE_Investigate 0x7E (0x168070): dead** — no C caller, no start row, no SetState literal,
  no +0x5A2 store (the live alert-2 look is 0xa5/0x56).
- **Interogate cluster 0x39-0x3D (0x1616c0/0x161860/0x162338/0x1729f0) + the DroneVision
  interrogate-assist trio (0x178308/0x178318/0x178348):** self-contained (only mutual callers);
  one agent read an 07000007 mission use, the caller grep found none — resolve with the live
  check below before spending effort.
- **DSTATE_AbseilInit/Hang 0x8E/0x90 (+0x8F/0x91/0x92): dormant** — start states of
  DroneTypeSettings rows 27/28; no disc placement uses dtype 27/28 [D census].
- **DSTATE_CivilianChallenge 0x3E (0x162558): dead** (no entry).
- **init_DMODE_* family (0x14d528-0x14e568, ~34 fns): OLD behaviour chain** — called only via
  DroneModeSettings[beh]+8 inside `DoModeSettingsOLD` 0x14cfa0:52-58 (behaviour id < 0x24). The
  port always takes the NEW chain. Whether any disc placement uses id < 0x24 is unresolved →
  live check below. Would also be where protected-actor prop 0x53 gets set for OLD drones.
- **Trivial/placeholder states:** Alerted1stEncounter 0x57, Obstructed 0x61, HoldItRightThere
  0x80, AltAttack 0x72, Prone 0x6B-0x6C, ReturnToPatrolPath 0x15 (table-only, no literal
  entry), Tester2-4 0xB8-0xBA, HangUp 0xBB, WaitForever 0xBC (debug/parking).
- **`Script_FreeFromHashTable` (0x1daec0):** reachable but a host no-op (the port keeps pack
  assets resident) — bookkeeping only.
- **`MP_pickupCountCheck` (0x18bed8):** empty body in the game. **`Drone_InitOnce` (0x13a0f8)**
  and **`DroneAnim_SnapRotate2` (0x13e230)**: empty (size 8).

---

## Citations added this audit (commit b4df396a)

Comment-only; +79 coverage hits (1984 → 2063). Grouped by file:

- `crates/nf-game/src/drone.rs`: `NDrone2_SetIdleTimeOut` 0x14a3b8, `NDrone2_ClosestAIPoint`
  0x151b18 (+ FindAlarmPoint 0x153008 gap note), `NDrone2_NearDrone` 0x14a6e8,
  `NDrone2_MoveToAlertPosition` 0x154ba8, `AINetwork_RouteIsValid` 0x15a7c0 / `InvalidateRoute`
  0x15a7e0 / `ValidateRoute` 0x15a800, `NDrone2_AtDest` 0x155050, `MoveTarget_IsRouteNode`
  0x1550c8, `CanReachGoalPosition` 0x155ad0, `GetAnglesToMoveTarget` 0x155db8,
  `SetRunToPosition` 0x155b30, `SetupAlertTarget` 0x155d48, `NDrone2_DroneAlertToPosition`
  0x14a498 (partial, scream + tase), `NDrone2_IsCivilian` 0x14aae0 (team form),
  `NDrone2_DSTATE_Combat` 0x1731b8, `NDrone2_BondIsFacingMe` 0x154f60 (upgraded from partial),
  `NDrone2_SetSoundAlertRoute` 0x154aa8 (upgraded from partial), `DroneWeap_AimTarget_IsOpponent`
  0x178ae0 (partial), RunToAlarm found-arm gap note (PressAlarm 0x168ea8 / FindAlarmPoint
  0x153008), `NDrone2_CanStrafeLeft` 0x14f9e8 / `CanStrafeRight` 0x14fbe0 / `CanStepRight`
  0x14f808 / `CanRollRight` 0x150500 / `EvasiveMove` 0x1506d8, `NDrone2_CanAimCrouch` 0x154960
  (+ BOTSTATE_EvasiveMove 0x129368 stand-in note), `NDrone2_GetHeadPos` 0x149aa8 +
  `DroneAnim_CurrentMoveState` 0x13e378 (partial), `DroneVision_DroneCanSeeDrone` 0x1760b0.
- `crates/nf-game/src/nav.rs`: `AINetwork_BuildAITarget` 0x15ac78 + `NDrone2_NearestNode`
  0x1558d8 / `NearestNodeDCV` 0x155958, `AINetwork_InitEmitter2` 0x15b2b8 / `AllocEmitter`
  0x15b390 / `FreeEmitter` 0x15b3d8 ([S] per-drone scratch deviation), `SetupRouteToObject`
  0x158948 / `SetupRouteToPosition` 0x158b10 / `CalcRouteToObject` 0x15aee8,
  `AlterDestFor_DROUTE_Nearest` 0x156088 (gap note), `AINetwork_RouteNodeOffset` 0x15a820
  (bounce mode not ported), `AINetwork_GetRouteDistance3D` 0x156540.
- `crates/nf-game/src/cover.rs`: `DroneVision_CanSeePosition` 0x178438.
- `crates/nf-game/src/world.rs`: `NDrone2_FindCel` 0x149a80.
- `crates/nf-game/src/nis.rs`: event 9 cam2mode gap note (camera_track_XZ_pos_behind 0x1bc260 /
  camera_track_Y_pos 0x1bc1f0).
- `crates/nf-game/src/mp/rules.rs`: the MP_get*Obj getter family (0x18b978/0x18b9a0/0x18b9c8/
  0x18bb50/0x18bab8/0x18bac8/0x18baf0/0x18bad8/0x18a338), `MP_getObjExtFromMPOBJECT` 0x18a210,
  `MP_recalcObjExtPaths` 0x18b888 (partial), `MP_initObjExt` 0x18b7a0 + `MP_PostLoad_Init`
  0x183718 (partial).
- `crates/nf-game/src/mp/mod.rs`: `MP_PlayerOrBotInd` 0x18bf08 (implicit), `MP_getAssassinTarget`
  0x18bd78, `MP_changeAssassinOrTarget` 0x18bd80 (n/a by design).
- `crates/nf-game/src/mp/bots.rs`: `AINetwork_SetupGoalPositionToObj` 0x15add0 +
  `BOTSTATE_gotoGoal` 0x1278a8, `MP_getObjectTeam` 0x18b6c0, `BOTSTATE_hasWeapon` 0x129658 /
  `BOTWEAP_hasWeapon` 0x12af78 (partial), `BOTSTATE_opponentIsMissile` 0x129a60 (stub),
  `BOTWEAP_WeaponHasAmmo` 0x12adb0 (partial).
- `crates/nf-formats/src/drone.rs`: `DroneData_GetAnimFuncInfo` 0x13e4d8 /
  `GetDAStateFuncInfo` 0x13e520.
- `crates/nf-view/src/menu.rs`: `C_NIS_Handler` 0x205f10 (route note),
  `Radio_Init` 0x21f8e0 / `Radio_SendMessage` 0x21fa50 / `Radio_GetIndex` 0x220130 /
  `Radio_SetIndex` 0x220180, `TextBox_Update` 0x2237c8, `Script_RunFrame` 0x220580 + splines
  (gap note).

Helpers verified covered but left to the tables here (no clean single anchor; their logic is
inline in the port): `NDrone2_HasOpponent` 0x14ad58 (seen/lost_sight), `SeenAndAttacking`
(already named inline), `SetAngleToDest` 0x154e20 / `SetAngleToAlert` 0x154e90 /
`GetAngleDifferenceToAlert` 0x154f00 (trig.rs inline), `NDrone2_InShoutingRange` 0x1788a0 and
`AnimForDist`/`DistanceToEmitter` (MP-bot-side niceties), `NDrone2_DroneInSights` 0x14a800,
`ObstructedPatrolPath` 0x14ad50, `HostageSituationActive` 0x14aaf8.

## Coverage corrections

- `TextBox_Update` (0x2237c8) and `C_NIS_Handler` (0x205f10) were counted "cited" by name greps
  that matched `docs/WORKLOG.md` mentions (this task's own plan) — not real citations. Both now
  carry real citations (above). Future coverage passes should exclude WORKLOG from the corpus.

## Live checks for the main agent

1. **OLD-chain census (decides the init_DMODE_* family):** parse every level's drone placements
   (kind 0x0F) and list the behaviour id (param 5, `piVar19[0x12]` [M 0x14b300:495-503]). If any
   id < 0x24 exists, the `DroneModeSettings` 0x29ad40 rows are live data and gap #6 of the
   NDrone2 group (OLD presets incl. protected-actor 0x53) jumps in priority. Expected: none.
2. **Interrogate reachability:** in level 07000007 (or wherever a challenge/interrogate
   trigger exists), watch whether `DroneVision_SetInterogateAssist` (0x178308) is ever entered
   (a call-count watch on the function entry suffices). Expected: 0 hits → the Interogate
   cluster is dead; >0 → queue the assist states.
3. **C_NIS_Handler census:** list which mission archives' kind-8 menu defs bind page
   0x1000003e controls (the 0x308840..0x308ca0 range) to script hashes, and which loadable
   cut-scene scripts fire NIS event 9. Decides whether the mission-driven cut-scene route is
   worth queueing alone.
4. **Alarm switch live view (gap 1):** on 07000009/0A, trigger a guard to run to the alarm
   (or fieldprobe the alarm switch object) and record: the switch channel state byte before/
   after the press anim, and `SetMissionFailReason`'s slot (+0x594=0x32 path). Anchors the
   PressAlarm acceptance test.
5. **Astronaut render check (gap 7):** `--bond` spot in 0700001b at a suited guard mid-combat —
   should show the float bob (no floor snap) and, after a hit, the AstronautHit variant.

## What ran

- `cargo build --offline --release -p nf-game -p nf-view -p nf-formats` — clean (warnings
  pre-existing). The diff is comment-only; no behaviour change is possible from this branch's
  code edits.
- Full suite (fast runner, `tools/test/suite.py`) started 17:52 UTC on this branch's tree —
  results reported in `docs/reports/uncited-gameplay-audit.md`.
