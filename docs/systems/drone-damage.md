# Drone damage, hit reactions, death; Bond's gunfire

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] read in our decomp export (work/decomp/ACTION.ELF) or disassembly,
[L] read in the level-start snapshot `work/ee_start_05.bin` (07000005, difficulty `GameState+0x28` = 1), [S] suggested.
Source: a research pass (subagent, 2026-10-09) re-checked here where marked "checked"; the rest is that pass's reading.

## Tuning (`ReadTuningVars` 0x1d3a80) [M, checked]

Per level, over the ELF's static values: `nf_formats::tuning` reads the statics by symbol, `nf_game::tuning::
read_tuning_vars` ports the switch (generated from the decomp). **Checked**: the port equals the game's own function run
in the R5900 interpreter for every level id 0x07000000–0x0700004F (`tests/tuning_live.rs`). 07000005–08 [L, checked]:
DroneDamage Easy 1.5 / Normal 1.0 / Hard 0.8, Head 10, Legs 0.75, Arms 1.0, Torso 1.0; armour Helmet 1.0, Combat 0.25,
Jacket 0.5, Vest 0.75; captain modifiers 2.0 / 2.0 / 2.0; `Autoaim_Range` 25. (The "head ×100, Easy 2.0 …" values once
quoted in drone-alerts.md are the ELF statics, replaced at level load.)

## Health at creation [M]

`NDrone2_DefaultInit` (0x14b300): `+0xAC` health 10.0, `+0x14` damageable 1, `+0x14C` damage-taken multiplier 1.0,
`+0x100` 1.0. `DoModeSettingsNEW`: health = `drone_stats[type]+8` when non-zero (0 for 07000005's type 2 [L]).
Captains (`+0x19`: prop 9 / 5 / 8 at difficulty 1 / 2 / 3, types 1, 2, 3, 6, 7): health × `DroneCaptain_Mod_Health`.
`+0xB0` max = `+0xAC`; `+0xBB` armour bits stay 0 in this build. 07000005 [L]: every guard 10 / 10.

Placement parameters (DefaultInit, `param(k)` = `piVar19[0xD + k]`) [M, checked]: 3 → `+0x134` switch channel,
6 → `+0x135` second switch, 8 → `+0x13A` its tag, **12 → `+0x13C` death channel**, 13 → `+0xBD4` special-death script.

## A bullet hitting a drone [M]

- `Bullet_update` → `Collide_Update` → `Collide_PickObj` (sphere `obj+0x80`, r `obj+0x8C` ≈ 1.05 [L]; skips the
  shooter and objects with `obj+0xF0 & 0x20`, i.e. corpses) → `Collide_Jointy` (0x1ec180, checked): the boxes of
  `AnimGetCollData` are tested **in list order, the first box hit wins**; HITDATA +0x52 part (bone id), +0x08 damage
  (`weapon+0x0C`), +0x40 point = start + t·ray, +0x30 ray.
- Boxes (`AnimGetCollData` 0x1c4ac8, checked): `HumanCollisionBones` (0x2c6be8, 20 bytes: s16 bone, s16 other (−1 =
  skeleton parent), f32 unused, f32 ×3 half-extents; ends at a negative bone). Matrix = the *other* bone's world
  matrix with translation = midpoint(bone, other); entry 0: bone + (0, bone.y − other.y, 0). Extents × skin scale.
  Rows: 5 head; 4 torso; 53–56, 49–52 legs; 20, 21, 23, 32, 35, 39 arms.
- `Drone_BulletHit` → msg 8 → the state's `DroneFunc_HandleImpact(msg, resume, allow)` (0x1469b8, checked). Per state
  [M, checked by a script over the DSTATE decomps]: Idle / Alert / Patrol / AlertToPosition / SearchArea / HeardNoise* /
  RunForCover / UnderCoverInit / LeaveNow → (0x56, 0); UnderCover Idle / Aim / Fire / Return → (0x9C, 0); AimStand,
  AimBackoff, Step/Strafe/Dodge/Roll → (0x68, 1); AimStandFire, AimCrouchFire, CombatNoMove, CombatNoRoute → (current,
  1); AimCrouch, CombatOutOfRange, CombatNoSight, CombatWait, GoToGoalPosition, DroneStuck → (current, 0); BulletImpact
  0x55 → `BulletImpact(…, 0, 0)` directly; WaitSwitch, InitPatrol, Attack, Combat, CombatNewSighting, NoOpponent,
  UnderCoverTypeChange / Leave, Death / Dead / Fade → nothing.
- HandleImpact case 8: `+0x1B` set → ignore; resume = given or the current state; → `NDrone2_BulletImpact` (0x1461c8,
  checked): `+0x50C` = 1, alert status 2, `DoHitEffects` → `HitDamage`; dead → state 0x44 (modes 0x1B/0x1C 0x92, 0x1D
  0xBF, 0x1E 0xF2). Alive with `allow`: `+0x1C` = (dmg > ¼ max && health > ⅓ max); flinch when frame ≥ `+0x540` + 120
  and ≥ global `0x298FC0` + 120 (2 · `FRAME_RATE_INT`): `+0x118` = resume (unless 0 or 0x53–0x55),
  `LocationImpactAnim`, stamp both, then HandleImpact sets state 0x55. Otherwise a resume other than the current state
  (and not 0x53–0x55) is entered.
- `NDrone2_HitDamage` (0x145558, checked): part < 0 → 1; head with fists / taser weapons → torso; `+0x14` = 0 → only
  `+0x150` = dmg. Head ×Head (sets `+0x3A` silent kill; ×2·Torso for mode 0xD / skin 0x50000BA / type 0x10 on
  07000014), ×Helmet if `+0xBB&4`; arms 0x14 0x15 0x17 0x20 0x23 0x27 ×Arms (×Combat &8, ×Jacket &2); legs 0x31–0x38
  ×Legs (×Combat &8); else ×Torso (×Combat, ×Jacket, ×Vest). × difficulty (1 Easy, 3–4 Hard, else Normal).
  `health −= dmg`; `+0x150` = dmg. On 0700001B every non-head part is torso. Damage passed in = HIT+8 ·
  `+0x14C`. Hit sounds 0x601 / 0x413 weigh 0.

## Flinch, death, corpse [M, checked]

- `LocationImpactAnim` (0x13c8b8): `CallAnim(0x4C, variant, next = +0x118)`; variant 5 + Rand(8) unless Bond aims (L1,
  player `anim+0x60` bit 0), then by part: 4/5 → 5, 0x12–0x15 → 8, 0x1E/0x1F/0x20/0x23 → 9, 0x31–0x33 → 13, 0x35–0x37 →
  12, else 6 + Rand(2). State 0x55 `BulletImpact`: enter sets `+0x3B` = 0; msg 8 → `BulletImpact(…, 0, 0)`.
- Death_Anim 0x44 (0x1637f8): enter `obj+0xF0 |= 0x20`; `+0xBD4` → SpecialDeath 0x46; else alert status 3,
  `OnInitDeath`, `+0xCD8` = 1, `LocationDeathAnim(next 0x47)`: Bond aiming and part 4/5 → 0x42 variant 1 when the
  bullet's heading is within 90° of the drone's yaw (shot from behind) else 0; otherwise `+0x3A` = 0, 0x40 variant
  Rand(4) if the danim's `info+0x10` byte is set. Update: `+0x3B` = 0 or anim set 4 / 9 → `DropWeapon` (docs/systems/pickups.md). Exit or msg
  0xC with `+0xCD8` → `SetDeathChannel` (0x13e660): `switch_channels[+0x13C]` = 1 (0x63: mission-fail reasons instead).
- SpecialDeath 0x46 (0x163c68, read): at its spot (`+0xBE0`, within 1.0 in 2D, facing within 90° of `+0xBF4`;
  script 0x0600021F also needs Bond ≥ 2 away) → `OnInitDeath`, death channel, `+0x4F8 &= ~0x80000000` (no AI-boundary
  push-out), `& ~0x20` (no collision), `& ~0x80` (no gravity), `+0x1A` = 1, the script as anim state 0x40 (follow-on
  0xBF, then Dead; 0x0F in mode 5); else the normal death. Update: `NDrone2_SlideToPosition(+0xBE0, 0.05, 3D)` and
  the weapon drop as in Death_Anim. With `+0x1A`, `DroneAnim_PreTransCallback` keeps the **whole** root motion
  (vertical too — normally replaced by the change of `+0xA0`) and adds it to `+0xBE0` as well; `AnimFrameResolve` adds
  it to `obj+0x30`. So the script carries the body out of the tower: 07000005's sniper (placement 267, 0x06000A1C)
  ends 9.5 lower with the hip 0.51 above the ground below, the other pick (7, 0x060003DF) 8.1 lower, 0.12 above it
  (port, test `tower_guards_fall_out_of_the_towers`).
- Gravity / collision otherwise (`Drone_CollisionHandler` 0x140968, `NDrone2_DoGravity` / `DoCollision`): while
  `+0x4F8 & 0x80` and not dead (`& 0x200`), velocity `+0x480` += `WldGravity` / 60 per tick and position += velocity
  / 60 unless the feet are on a surface (the hit within 0.2 below the feet sphere, `anim+0x60 & 8`). Not ported: the
  port stands drones on a floor within 2 units below (G-026).
- Dead 0x47 (0x163fe8): enter `obj+0xF0 |= 0x20`, `SetAsDead`, timer 0xC at now + 300 (`DAT_00298F58` = 10 ·
  `FRAME_RATE_INT` [L]); prop 0x31 and not a silent kill → `DroneAlertToObject(msg 0x15, corpse)`: a broadcast that
  single-player states pass to `DroneVision_EnemyAlerts`, whose case 0x15 ends in `ShoutFromOtherDrone` (returns 0) —
  only the return state / home are saved; SeenDeadBody 0x9E is entered only by the bots' `ReactToDroneAlertMsg`
  (multiplayer) [M, WORKLOG 0033]. Update: not drawn →
  disabled; `DeadDrone`. Timer 0xC → Fade 0x48.
- Fade 0x48 / FadeFast 0x49 (`NDrone2_FadeOut`): alpha from `obj+0x106` down 0.6 (8.0 for 0x49) per tick; at 0 or not
  drawn → disabled. `DeadDrone` (0x14a298): `Drone_InitComms` counts drones in 0x0F, 0x47, 0x48, 0xBF (`+0x11C` the
  state's start, + 300 for 0x48); more than 3 (`DAT_00298F5C`) → the oldest goes FadeFast.
- 07000005 [L]: two guards fire death channel 0xA4 (the WaitSwitch drones' channel); the sniper 0x0A.

## Bond's gunfire [M]

- Start weapon on 07000005 (`Player_InitWeapon`): weapon 2 (P2K), clip 7, 41 spare [L]. Record [L]: damage 3.5,
  range 50, speed 5, spread 0, interval 8, clip 7, flags `+0x68` 0x1004, `+0x6C` 0x40050.
- `Player_WeaponFiring` (0x1a5460): action 9; one shot per press for the P2K, at most every 8 frames.
- `Player_WeaponInitBullet`: from the head (y + 0.70) towards the aim point (camera forward × 256, replaced by the
  first world hit), `Bullet_init` with owner = player. `Bullet_update`: first step (0.5 + FRand(0.5)) · speed, then
  speed per frame; dies past 50 units.
- Auto-aim (`Check_AutoAim`): only when not aiming (L1); drawn, alive enemy drones within 25; cone 0.12·k yaw,
  0.22·k pitch, k = `weapon+0x14`/100 · (1 − d/25) · (Easy 1.9, Normal 1, Hard 0), ×1.4 for the last target; target
  bone 2; line of sight. Checked in the decomp (0x1a73f8, 0x1a7c98): the nearest qualifying drone wins (the range
  shrinks to each accepted distance); a hit sets the crosshair offset `BL+0x120 / +0x124` to the target's screen
  position and `+0x114` to it; without one `+0x114` = 0; the offsets then halve every tick (weapons 0x50 / 0x51 use
  k = 1 and scan type-0x51 objects instead). Port: `Drones::auto_aim` keeps the offset as a yaw / pitch angle
  (*approximation* of the screen offset), test `auto_aim_snaps_to_a_guard_near_the_crosshair`.
- Gunshot sound: the first-person fire script plays **0x403** (inner 2, outer 40, weight 65) at the gun: guards
  within ~28 units hear it. Silenced variants remap to 2 (weight 0).

## Port status

Ported (WORKLOG 0030): tuning, hit boxes, damage, the per-state impact table, flinch, death, corpse, fade, death
channels, Bond's bullets and trigger, the shot sound for hearing. Not checked live (U-034); gaps in G-026.
