# Surrender: a guard caught from behind gives up

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp (work/decomp/ACTION.ELF), [L] level-start snapshot
`work/ee_start_05.bin`, [S] suggested. Source: a research pass (subagent, 2026-10-09); "checked" = re-read here.

**There is no disarm button.** The guard surrenders by itself when its first sighting of Bond is from close behind;
it drops its gun as an ordinary pickup (docs/systems/pickups.md), which Bond collects by walking over it.

## Trigger [M]

- Visibility: Bond within 2.0 with sight range > 0 and a line of sight → visibility 1.0, ignoring the cone and AI
  volumes (`DroneVision_OpponentVisibility` 0x175af0; already ported). The reaction time still applies.
- `DroneVision_EnemyLookForOpponent` (0x177ed0) → Attack 0x56; the first time (`+0x4F8 & 0x4000000` clear) it sets
  `|= 0x4000000 | 0x8000000` (0x8000000 = the drone's own sighting).
- Attack's enter → `DroneFunc_FirstAttack` (0x148010; once: `+0x4F8 & 0x2000000`) → with 0x8000000,
  `FirstSightState` (0x147c48) with `NDrone2_CheckSurrender` (0x14a060) inlined. All required: an opponent; **prop
  0x43**; **alertness < 0.66**; distance `+0x1A0` < 2.0; |AngleDiff(drone yaw, bearing to Bond)| (`+0x1D4`) > 120°
  (Bond behind); line of sight (bone 5); Bond armed (`DroneWeap_OpponentHasWeapon`: weapon byte 2–0x46 or 0x6A–0x71);
  `BondIsFacingMe(45°)` (|`+0x1D0`| < 45°, `+0x1D0` = AngleDiff(Bond's yaw, bearing Bond → drone)).
- Then alertness 1.0, `+0x4F8 |= 0x6000000`, `+0x524` = now if 0, sight widened, no shout → Surrender_Anim 0x3F.
- Not possible after an alarm, a forced attack, noise reaching Attack, any earlier fight, or alertness ≥ 0.66
  (being shot or heard sounds). [L] 07000005: all 13 guards have props 0x43, 0x4F, 0x51, 0x3C; four start at
  alertness 1.0 and never surrender.

## States [M; Surrender_Anim checked]

- **Surrender_Anim 0x3F** (0x1626a0): enter — `CallAnim(0x5C, next 0x40)` (with the armed anim set: the call comes
  first), alert status 2, `DroneWeap_DropWeapon`, `ChangeWeapon(1)` (weapon 1, anim set 0xC, no rounds), timer C in
  15 ticks, `PlrStat_LogEnemySurrender`; script 0x0600021F's special death and its channel are cancelled. Update: turns
  his back to Bond (desired yaw = bearing + π). Timer C: `+0x4FC |= 4`, rearmed every 15 ticks (the msg 0x13
  broadcast is dead in single player). Bullet → `HandleImpact(0x44, 0)`: any hit kills. Explosive → 0x44; punch →
  KnockedOut 0x42 (not ported).
- **Surrendered 0x40** (0x162950): enter faces Bond once. Update: `NDrone2_CheckUnsurrender` (0x14a168) — distance > 12,
  or > 2 with Bond unarmed, or Bond not facing him within 90°, or no line of sight → 0x41. Bullets as above.
- **Unsurrender_Anim 0x41** (0x162a70): faces Bond, `CallAnim(0x0A, next Attack)`.
- **Attack** unarmed (weapon ≤ 1) → `DroneFunc_AttackNoWeapon` (0x14af80) → **DrawWeapon 0x66** when `+0x45` > 1
  ([L] 10 for every 07000005 guard), else CivilianScared 0x12 (not ported).
- **DrawWeapon 0x66** (0x166000): `CallAnim(0x46, next +0x5A2)` ([L] 6 InitPatrol for most guards; the port uses
  the return state or InitPatrol); exit, still unarmed → `ChangeWeapon(6)`: weapon 6, anim set 4, 16 rounds.

## Other guards [M]

`DroneVision_FindAlertedDrones` → `ConsiderAlerted`: a calm observer with prop 0x4F that sees the surrendered guard
(alert bit 4) from 60 ticks after its `+0x524` gets a forced attack — it attacks Bond. Ported through the existing
alerted-drone pass (bit 4 → prop 0x4F).

## Port

WORKLOG 0045: the sighting flags, the surrender gate in Attack's enter, states 0x3F / 0x40 / 0x41 / 0x66, the unarmed
Attack, Bond's weapon lent to the drones. Test `guard_surrenders_to_bond_behind_him`: placement 6 with Bond 1.5 behind
it surrenders at tick 69 (Attack → 0x3F), its SG5 lies on the ground, the hands-up transition (danim 0x141, script
0x060001B0) ends in Surrendered at 279 (hands-up loop danim 0x174); Bond 15 away → 0x41 → Attack → DrawWeapon →
weapon 6 → InitPatrol; sister: in front of it, a normal attack. Not ported: KnockedOut (no melee), CivilianScared,
the sidearm's own mesh in nf-view. Open: U-038.
