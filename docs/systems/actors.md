# Actors: every non-guard actor type on the disc

The actors that make missions playable besides the hostile guards: allies, civilians, VIPs, bosses and the
spawner waves. Sources: the disc's drone placements of all 24 story levels (`nf-extract drones`/`placements 0f`,
aggregated 2026-10-12), `DroneTypeSettings` 0x29aee8 and `NDrone2_StateFuncs` 0x29b300 read from
`work/ee_start_05.bin` (file offset = EE address) [L], and the decomp [M]. Tags: [M] code (function + address),
[D] disc, [L] snapshot, [S] suggested. Port: `crates/nf-game/src/{ally,spawners,drone}.rs`; tests
`crates/nf-view/tests/fix_actors_live.rs`.

## 1. The type table (DroneTypeSettings, 0x29aee8; 85 entries × 12 bytes: start state, alternate, init fn, control fn) [L]

| type | start | alt | control fn | role | team [M 0x14cbd8] |
|---|---|---|---|---|---|
| 0 | −1 (by mode) | | | the generic guard | 1 |
| 1 | 6 InitPatrol | | | the common guard | 1 |
| 2 | 0x29 SniperIdle | 0x2a | initDTYPE_Sniper 0x14e570 | the sniper | 1 |
| 3 | 0x56 Attack | | | the aggressor | 1 |
| 4 | 8 | | | | 1 |
| 5 | 0xa | 0xa | | the heavy trooper | 1 |
| 6 | 0x56 Attack | | | the boss/aggressor (07000004's assassin) | 1 |
| 7 | 0x85 EnemyRunToPoint | 0x85 | | the runner | 1 |
| 8 | 0xb7 | 0xb7 | | the EvilBase guard | 1 |
| 9 | 0x10 CivilianInit | 0x12 | | the civilian/VIP-to-be (07000014's Rook) | 1* |
| 10 | 0x12 CivilianScared | 0x12 | | the party guests | **3** |
| 11 | −1 (by mode) | 0xb5 FailMission | | the mission actor | **3** |
| 12 | 0x1c AllyLeadInit | 0x1c | | **the VIP/escort** (Mayhew 0x4f, Zoe 0x3b, Drake 0x5e) | **2** |
| 13 | 0xa6 NinjaStand | 0xa7 | ControlDTYPE_Ninja 0x14b0f8 (the glowing eyes) | the tower staff/ninja-body actors | **3** |
| 14 | 0x89 RunToAlarm | 0x89 | | the alarm runners | **3** |
| 15 | 0 | | | | 1 |
| 16 | 0x37 | 0x56 | | | 1 |
| 17 | 4 | | ControlDTYPE_Zoe 0x14b058 | **Zoe** — ch 1 → ch 0x9B; Bond > 10 away while 0x9B → SetMissionFailReason(0xF) [M 0x14b058] | 1 |
| 19 | 0x30 CivilianGuard | 0x66 | | the undercover guard | 1 |
| 22 | 0x31 CivilianDoorGuard | | | the door guard | 1 |
| 23 | 0x32 | 0x12 | | | 1 |
| 24 | 0x36 | | | | 1 |
| 25 | 0x2a | | initDTYPE_SniperAlert | the alert sniper | 1 |
| 29 | 0xbd AstronautLaunch | | ControlDTYPE_Astronaut 0x14b128 | the astronaut | 1 |

\* type 9's team comes out 1 from the switch; its drones start in the civilian chain through
`GetDTYPENEW` 0x14c938 (below). The team switch is exactly: types 10/0xb/0xd/0xe → 3, 0xc → 2, else 1.

`GetDTYPENEW` (0x14c938) remaps the type before the table lookup — the special cases [M]:
- block 0 + prop 0x10 → 0x10 (CIVILIAN_INIT);
- types 0–9 on 07000007 (block 0): prop 0x4b ? 0x16 : 0x13 (the party's guard classes);
- types 10/11 (block 0): alert < 1 → (voice flag +0x15 set ? **0x12** : 9); alert ≥ 1 → 10 — on 07000007
  they also write props 0x1F = 0 and 0x54 = 1;
- type 0xc: voice 0xc → 0x11 else 0xc; type 0xd: kind 0xf → 0x17 (Zoe) else alert-ordered 9/10;
  type 0xe: prop 0x4e → 5, else 10/9; type 0xf → 0xd; type 0x10 → 0x1e.
`DroneTypeSettings[+0xC5].start` (−1 → by mode) is the drone's start state; the state id indexes the same
table for the alternate.

## 2. The actor states (NDrone2_StateFuncs 0x29b300 [L]; the names are the decomp's)

- **Hostage** 0x08 HostageKiller, 0x09 HostageKillerAttack, 0x0A Hostage, 0x0B HostageDie, 0x0C HostageSaved,
  0x0D HostageIdle, 0x0E HostageHide, 0x0F HostageDead; 0x64 HostageGoToGoalPosition.
- **Civilian** 0x10 CivilianInit, 0x11 Civilian, 0x12 CivilianScared, 0x13 CivilianHiding, 0x14
  CivilianPatrol, 0x15 ReturnToPatrolPath, 0x16 CivilianMission, 0x17 CivilianMissionWait; 0x30
  CivilianGuard, 0x31 CivilianDoorGuard, 0x3E CivilianChallenge.
- **Ally lead** 0x1C AllyLeadInit, 0x1D AllyLead, 0x1E AllyLeadPlayerInWay, 0x1F AllyLeadHide, 0x20
  AllyLeadWait, 0x21 AllyLeadMissionWait, 0x22 AllyLeadBondCombat, 0x23 AllyLeadDone; **follow** 0x24
  AllyFollowInit, 0x25 AllyFollow, 0x26 AllyFollowWait, 0x27 AllyFollowDone, 0x28 AllyGoToGoalPosition.
- **Scripted mission actors**: 0x19 KikoMission, 0x1A KikoMissionRun, 0x1B EnemyMission, 0x2E/0x2F
  PartyGirl(Init), 0x93–0x9C UnderCover*, 0xBD–0xC1 Astronaut*, 0xC2 SpaceDrake.
- **Scared flight** 0x85 EnemyRunToPoint, 0x86 RunAwayFromObject, 0x87 HideFromScaryObject, 0x88
  RecoverFromScaryObject; **alarm** 0x89 RunToAlarm, 0x8A PressAlarm, 0x8B DonePressAlarm, 0x8C RunForCover.
- **Special** 0x8D ElevatorJumper (07000014's Rook), 0x8E–0x92 Abseil*, 0xA6–0xB3 Ninja* (the melee chain),
  0xB4 DeleteMe, 0xB5 FailMission, 0xB6 JustStand.

Key state facts [M]:
- CivilianInit 0x172730: voice 0xD → PartyGirl 0x2F; prop 0x1D → AssignAIPath(8) → CivilianMission 0x16;
  else AssignAIPath(4) → CivilianPatrol 0x14; no path → Civilian 0x11.
- Civilian 0x15d118: ENTER arms the **scared radius** `+0xCF8` = 10.0 on 07000007, else 20.0; UPDATE:
  `NearArmedPerson(+0xCF8)` → the scared chain, else BondTalk.
- CivilianScared 0x15d3a0: FirstAttack (the surrender check applies), alert 2, the scared talk; alt mode 7
  → FindRunToPoint → HostageGoToGoalPosition 0x64 with the return 0x13; else CivilianHiding 0x13.
- AllyLeadPlayerInWay 0x15ea28: ENTER alert 2, anim 0x21; UPDATE: no armed drone within 10 (0x41200000) and
  Bond not within 1.06 rad (0x3f860a92) → Lead.
- AllyLeadHide 0x15ec08: ENTER anim 0x19 (the cower); UPDATE: `NotNearArmedDrone(14.0)` → timer C in 2 ticks
  → Lead.
- AllyLeadMissionWait 0x15efc8: the wait channel = obj+0x12C (0 → 0x99); the channel on → Lead.
- AllyLeadBondCombat 0x15f120: UPDATE: no armed drone within 10 → `DroneMove_ObjectMissionPathDistance`
  (0x153f50) 5.0 to Bond — negative (no route) stays, else → Lead.
- AllyFollowInit 0x15f3c8: ENTER alert 1, anim 0x21; UPDATE: Bond visible → AllyFollow.

## 3. The mission-fail rules for actor deaths [M DroneFunc_OnInitDeath 0x145bd0 + DroneFunc_SetMissionFailReason 0x13e560]

`SetMissionFailReason(reason)`: 1/3/6/9 → FailLabel **0x4000034 "Killed Civilians"** + ch 0x60;
2/4/5/7/8/10/11/12 → **0x4000033 "Cover Lost"** + ch 99; other (0xF = Zoe abandoned) → ch 99, no label;
0x10 → the nearest alarm switch's channel (0x61 → **0x4000032 "Alarm Raised"**).

`OnInitDeath` on a drone death, by team and the protected-actor props **0x53/0x54**:
- team 1: prop 0x54 → reason 2, else prop 0x53 → reason 4 — "Cover Lost", the mission fails;
- team 2/3: prop 0x54 → reason 1 (a drone killer) / 3 (the player's kill); else prop 0x53 → reason 3 —
  "Killed Civilians", the mission fails.
The monitor's standing hooks (ch 0x60/0x61 → the label + ch 99 [M 0x1827d8:98-107]) turn these into the fail.
**85 drones across 14 levels carry the props** — every party guest (07), the office workers (0a, 11, 12), the
hotel staff (09), the mayhew/Zoe VIPs (01, 02, 08), the 06 driver (the C1 playtest item), the 02/03 staff.

Bond's bullets hurt team 2/3 in the game: `NDrone2_DealWithObjHit` 0x13fff0 case 5 selects the message by the
WEAPON (1/0x4B/0x4D → the punch, 0x4A/0x4C → the taser, else the bullet) with no team gate — the gate the
port applies is only on the DRONE-shooter paths. The port's player-bullet path sends the message for every
drone hit (drone.rs), and the protected-actor death now raises the fail (fix-actors).

## 4. The inventory (every 0x0F placement of the 24 story levels; 593 drones)

dtype (param 15), mode (param 16), team (per §1), skin, count and levels — the non-1 rows (the guards are the
rest; the full per-level roll is in `docs/levels/<code>.md` where written):

| dtype | mode | team | skin | count | levels | role |
|---|---|---|---|---|---|---|
| 13 | 0/3 | 3 | 0500000e | 5 | 11, 12 | the tower office staff (death ch 0x63: kill = fail) |
| 13 | 0 | 3 | 0500000f | 2 | 11, 12 | the tower office staff |
| 10 | 0/5 | 3 | 05000015 | 2 | 07 | the party girls (PartyGirl 0x2F) |
| 11 | 0 | 3 | 05000018 | 2 | 07 | the mission actor |
| 10 | 0 | 3 | 05000019 | 9 | 07 | the party wanderers (Civilian) |
| 10 | 0 | 3 | 05000020/22/23/24 | 24 | 07 | the party girls |
| 11 | 0 | 3 | 05000026 | 2 | 07 | the mission actor |
| 13 | 0 | 3 | 05000027/28 | 6 | 09 | the hotel staff (switch 0x51, death ch 0x60) |
| 13 | 0 | 3 | 05000029/2a | 20 | 0a | the office workers |
| 13 | 0/3 | 3 | 0500002b | 3 | 03 | the ski-lodge staff |
| 13 | 0/3 | 3 | 05000034 | 3 | 02 | the castle staff |
| 13 | 0 | 3 | 05000035 | 1 | 06 | **the truck driver** (the C1 item) |
| 12 | 0 | 2 | 0500003b | 1 | 08 | **Zoe** (the escort VIP) |
| 12 | 0 | 2 | 0500004f | 2 | 01, 02 | **Mayhew** (the escort VIP) |
| 12 | 0 | 2 | 0500005e | 1 | 16 | **Drake** (the VIP) |
| 12 | 0 | 2 | (the type) | — | — | the escort states 0x1C–0x23 |
| 9 | 0 | 1* | 05000066 | 1 | 14 | **Rook** (the elevator-jumper boss, death ch 0x54) |
| 9 | 0 | 1* | 05000093 | 2 | 14 | the civilian-chain actors |
| 6 (0xf) | 0 | 1 | 0500001e | 3 | 04, 16 | **the assassin** (ATTACK start; 04's carries death ch 0x54) |
| 17 | — | 1 | — | 0 | — | Zoe's control fn — no disc instance (unused) |
| 1 | 0 | 1 | 05000013/14 | 31 | 07, 08 | the party guards |
| 2 | 0/2/3/4 | 1 | 05000036/39/3a | 68 | 01, 02, 03, 11 | the castle guards |
| 3 | 0/2/3 | 1 | 05000004/06/08 | 127 | 0b, 11, 12, 13, 4a | the tower guards |
| 5/8 | 0/2/3 | 1 | 05000087/8c/90/b1 | 148 | 0c, 0d, 14, 15, 16 | the base troopers |
| 1 | 0 | 1 | 0500001e/2e/2f/30/6d/7e/78/78/8f/ba | 96 | 05, 06, 08, 0c, 0d, 0e–10, 1b | the estate/base guards |

## 5. The spawners (G-025; DroneSpawner_Create 0x1378d0 / Init 0x137aa0 / Control 0x137e68)

The placement params → the data block [M 0x1378d0]: p0 = mode (> 1 required; 2 = keep `alive < p2`, 3 = only
when none alive), p1 = group (the templates are the group's 0x0F placements), p2 = max alive, p3 = waves
(0 → 32000), **p4 = the trigger channel** (consumed when it fires), **p5 = the stop channel** (un-triggers;
pauses an armed spawner, deletes an un-armed one), p6 = the done channel (all waves spawned and — when the
arm channel is set — no spawned drone alive), **p7 = the arm channel** (latches `armed`; an armed spawner
needs no trigger), p8 = the min spot distance (0 → 10).

The port (`crates/nf-game/src/spawners.rs`) models all nine; the fix-actors correction: **p7 is the arm
channel** — the port read it as a kill channel and removed the spawner when it fired (07000011's stairwell
spawner died instead of arriving; `the_spawner_arrives_on_its_channels` pins it). The per-level SFX quirk of
`Control` (0x700000c/d plays a spawn line) is power-station-only and not ported. The alarm reinforcements are
spawners gated on the alarm channels (e.g. 07000006's spawner waits ch 55 [D]).

## 6. Scripted actors (the NIS hand-over)

The game freezes every drone for a cut-scene (`Drone_EnableAll(0)` walks the llist calling `NDrone2_Enable(0)`
[M 0x1383e8:13-23]) and `Script_Free`'s 0x80 flag re-enables all but the ch-waiters [M 0x1da880:41-43] —
ported (fix-npcs). A scripted actor walks its NIS Anim streams (the root motion), then the AI resumes at
`+0x5A2` (the start state) — the escort's intro → ch 0xB → AllyLeadInit is the live example
(`fix_mission_live::the 07000001 exit end to end`); `fix_actors_live::the_escort_cowers_in_a_fight_and_resumes_the_lead`
covers the AI-side chain (Lead → Hide → Lead).

## 7. Port status

- **Ported** (fix-actors): the protected-actor death rules (ally.rs `on_init_death` + the fail-reason
  switch), the AllyLead sub-states 0x1E/0x1F/0x21/0x22 and the AllyFollow chain 0x24–0x27 + 0x28, the
  armed-peer scan, the spawner arm semantics, the scared radius (already 10/20) and the scared chain
  (already ported), the party states.
- **Stand-ins** [S, documented]: the Lead's Bond-distance waits use straight lines instead of
  `DroneMove_ObjectMissionPathDistance` 0x153f50's path distances; LeadMissionWait has no route-node channel
  source (reachable only via a forced state); AllyFollow's follow band is 8/6 straight-line; the
  LeadBondCombat cover-run (FindRunToPoint) is not ported (the cover nodes are the fight's business).
- **Not ported**: the Hostage chain 0x08–0x0F beyond the standing idle (no hostiles take hostages on the
  ported levels' completion paths), the UnderCover chain 0x93–0x9C (07000007's party guards — they fight
  through the ordinary combat there), the Abseil 0x8E–0x92, the Astronaut 0xBD–0xC1, ElevatorJumper 0x8D.
- **Ported (fix-finale-blockers)** — `crates/nf-game/src/kiko.rs` holds the addresses: the **Ninja chain
  entry** 0xa6 → NinjaAttack 0xa7 (0x16b990's blind-cheat branch is the only stand) with the distance
  bands 0xa8/0xa9/0xaa/0xb2 (0x16bb50; alert 1.0; the melee/throwing-star anims are not modelled — the
  bands advance and re-dispatch) — 07000004's assassin (dtype 0xf → 0xd [M 0x14c938 case 0xf]) and
  07000016's chaser run it; **KikoMission 0x19 / KikoMissionRun 0x1a** (0x15df28/0x15e0d8) with
  StandBlind 0x18 (0x172870) and RunToAlarm 0x89 (0x1736f8 — the none-found arm is the one the port's
  completion paths hit), entered from CivilianMission 0x15d860's voice-0x13 branch (no disc placement
  reaches it — documented); **EnemyRunToPoint 0x85** (0x168910 — the return state 0x56, on
  07000011/12/13/41 with the player unseen → Idle/Alert [M :32-36]; the port has no 0xe9 AI-point
  registry, so FindRunToPoint's none-found arm is the proven stand for the mode-7 drones);
  **DeleteMe 0xb4** (0x173f00 — the soft removal). Also fix-finale-blockers: Death_Anim raises the
  death channel on its first UPDATE (0x1637f8 case 2), not the anim's end, and the explosion kill
  flushes the pending transition (a same-tick UPDATE re-dispatch used to clobber it).
