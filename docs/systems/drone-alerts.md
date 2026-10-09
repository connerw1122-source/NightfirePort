# Drone hearing, alerts, goal positions and hit reactions

Sources: ACTION.ELF decomp and disassembly (research pass, WORKLOG 0027). [M] = read in the code or bytes, [S] =
inferred. Port: `crates/nf-game/src/drone.rs` (partly).

## Goal positions [M]

`NDrone2_MoveToGoalPosition(goal, radius, flags)` (0x1517d8): routes only when the combat route is invalid
(`CalcRouteToPosition`; the radius goes to `+0x704`); results 0 moving, 3 arrived (route < 2.0), 0xC blocked, 10 door
(pending state 0x81), 4/5/6/8 failed — never 1, 2 or 9.

- `SetGoalToCloserPosition` (0x155e10): the point where the straight drone → opponent line first meets an AI-network
  wall (`AINetwork_FurthestPosition`: every BLINK, no filters, nearest intersection to the drone; y = post a's y); goal
  only if some wall was hit, the point has a cel and `MoveToGoalPosition(point, 3.0)` < 3.
- `SetGoalToDroneWithLosToOpponent` (0x1543f8): the first other drone (list order) that is enabled (`+0x4F8` 0x100 set,
  0x600 / 0x4200 clear), alive, targets the same opponent and sees it, **and wins Rand(4) == 0**; goal = its position,
  radius 3.0; accepted when `MoveToGoalPosition` gives 0 or 2.

## GoToGoalPosition 0x63 [M]

Enter: `+0x4F8 |= 0x10000 | 0x400000` (0x400000 = deaf); exit clears 0x400000. Update: except on 07000009/0A, visible
and not (`+0x228 & 8` with dist ≥ engage) and not `+0x4FC & 0x200000` and not running to an alarm → look for the
opponent. Then `MoveToGoalPosition(+0x710, +0x704)`: 0/1/2 → run 0x1E; 9/0xC → armed drone faces the opponent and aims /
fires when visible, else anim 0x21; 10 → nothing; arrived or failed → prop 0x59 → DeleteMe 0xB4, else `+0x33 = 0` and
the return state `+0x5A2` (mode 7: behaviour switching). Impacts: `HandleImpact(msg, current state, 0 / 1)`.

Callers: CombatNoRoute's timer (both setters, `+0x5A2` unchanged), SearchArea give-up (home `+0x5B0`, return
`+0x5A0`), RunToAlarm (`+0x5A2` = 0x8A), EnemyRunToPoint, ReturnToPatrolPath (`+0x5A2` = 0x14).

## Hearing [M]

- Every `Sound_Play3D` / `Sound_Play` enters a list (0x349b00) with inner / outer radius and weight from `SFXOutputData`
  (0x2c94a0, 0x60D × 0x18: +4 inner, +8 outer, +0xC weight, +0x14 looped → ignored). The player's footsteps and drone
  impact sounds weigh 0; bullet impacts on surfaces have weights 20–50 by material; shots 50–90 (outer 20–100) —
  *partly contradicted* (WORKLOG 0030): the SG5's third-person fire sound 0x605 / 0x604 weighs 0; Bond's P2K shot is
  0x403 (inner 2, outer 40, weight 65) from the first-person fire script. Which sounds are shots needs a per-weapon
  check (U-034).
- `Sound_Alertness(pos)` (0x1ddc48): per sound within its outer radius, weight (inside the inner radius) or weight ·
  (outer − d) / (outer − inner); the sum (≥ 0) and the loudest. `Drone_InitComms` zeroes every weight after all drones
  ran: each sound alerts once, the frame after it was made.
- `DroneFunc_HandleSoundAlerts` (0x148a30, every frame, every drone; needs prop 0x1F, not deaf): loudness > 0 →
  `+0x514` = 0.01 · loudness, alertness += it; the loudest sound's position becomes the alert target and msg 0x14 is
  sent (at most once a second unless louder).
- `DroneVision_AlertSound` (0x177800): ignored more than 50 units from the opponent (the player) or when already
  attacking; normal guards → HeardNoise 0xA2 when `+0x514` > 0.2 or alertness > 0.66.
- Chain: HeardNoise 0xA2 → Aware 0xA3 (first time: look around 0x22, back to `+0x5A0`) → Suspect 0xA4 (route to the
  sound with radius 2.0; turn (0x61) if more than 135° away; prop 0x21 → AlertToPosition 0x62 then SearchArea 0x65) →
  Alert 0xA5 (face the opponent → Attack). Any further msg 0x14 in Suspect → Attack. The noise level `Drone+0x43`
  never resets.
- AlertToPosition 0x62: walk 0x35 / run 0x1E (route ≥ 10) to the goal; arrived → `+0x5A2`; 6 → 0x5F; others →
  `+0x5A0`. SearchArea 0x65: look around 0x22, 3 s, turn 0x61, anim 0xB, then give up (alertness 0) and go home.
- Look-arounds in the chain [M] (0x16af20, 0x16b0e0, 0x16b540): Aware, Suspect (prop 0x40, or no prop 0x21) and Alert
  (prop 0x40) call 0x22 with **variant 1**; Suspect with prop 0x21 calls 0x22 variant 0 → AlertToPosition. Aware takes
  msg 0x14 only to go to Suspect (share > 0.1). Suspect's turn done (msg 0x23): already turned with prop 0x21 →
  AlertToPosition; else face the sound, then without prop 0x21 look (variant 1) and go back, with it set `+0x5A2` =
  SearchArea and take the short look 0x0B → AlertToPosition when in alert idle 0x21, else AlertToPosition at once.
  Alert has no msg 0x14 case and the global state ignores it.
- **Port** (WORKLOG 0029): `nf-formats::sfx` reads `SFXOutputData`; `Drones::play_sound` queues a sound for the next
  tick's `HandleSoundAlerts`; the chain above, AlertToPosition and SearchArea are ported. Not yet checked live (U-033);
  Bond's gunfire does not make sounds yet (G-026).
- Drone-to-drone shouts (msgs 0x11, 0x12, 0x13, 0x15, 0x16) are dead code in this build (`ShoutFromOtherDrone`
  returns 0). The alarm (msg 0x1E, every 30 frames while `switch_channels[0x96]`) → Attack.

## Hit reactions [M]

`DroneFunc_HandleImpact(msg, retState, allow)` (0x1469b8): bullets (msg 8) → `NDrone2_BulletImpact`: alertness 1,
damage = weapon damage × part × armour × difficulty, only when vulnerable (`Drone+0x14`). **Correction** (WORKLOG
0030): the factors once quoted here (head ×100, Easy 2.0 …) are the ELF statics; `ReadTuningVars` replaces them per level
(07000005: head ×10, legs ×0.75, Easy 1.5 / Normal 1.0 / Hard 0.8) — see docs/systems/drone-damage.md, which supersedes
this section. Health ≤ 0
→ Death_Anim 0x44 (→ Dead 0x47; death channel). Otherwise a flinch (anim 0x4C, variant by hit part) when allowed and
at most one per 2 s for the drone **and** one per 2 s game-wide → state 0x55, then back to the resume state; without a
flinch the drone goes to retState. Idle / Patrol / Alert / AlertToPosition / SearchArea / HeardNoise* pass (Attack,
allow 0): a shot unaware guard goes straight to Attack. Punch 6 → 0x53, explosive 9 → 0x54, taser 7 → 0x4A, smoke 0x17
→ 0x7B, stun grenade 0x18 → 0x4D, stun dart 0x19 → 0x50. There is no near-miss reaction except through hearing.
