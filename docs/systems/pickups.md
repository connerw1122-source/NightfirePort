# Pickups: dropped weapons, ammo, Bond's inventory

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] read in our decomp export (work/decomp/ACTION.ELF) or the disassembly
(scratchpad `dis.py` over the ELF), [L] read in the level-start snapshot `work/ee_start_05.bin` (07000005),
[S] suggested.

## Who creates pickups [M]

- `DroneWeap_DropWeapon` (0x17a2a0) from `Death_Anim` 0x44's update (each tick while the firing flag `Drone+0x3B` is 0
  or the anim set `+0xDA` is 4 / 9), from `NDrone2_SetAsDead` (Dead's enter) and from the other death / surrender /
  stun states. It runs once: it clears the weapon byte `anim+0x62` (the drone draws no gun afterwards).
  - No weapon byte (≤ 0) or no third-person model (`weapon_data[w]+0x80` = 0): only the byte is cleared.
  - Matrix: `AnimGetBoneWorldTrans(obj, datum, 0, &v, &M)` with the datum of `AnimDatumGetWeaponInfo` (the weapon's
    datum, `+0x50`, 0xFF → 0; datum 0 when it reports none): the held gun's world matrix.
  - `Collide_RayIntersect(obj+0x30 → M.translation, mask 0x125)`: when anything is in the way the translation becomes
    the drone's position `obj+0x30` (the gun never spawns behind a wall).
  - Rounds: n = `weapon_data[w]+0x92` (the clip size); f = 0.5 + FRand(0.5) when n < 10, else 0.25 + FRand(0.5);
    rounds = (short)(int)(n · f). **One `Rand_FRand` per drop** (two for weapon 0x1A) — it advances the shared RNG.
  - `Pickup_CreateSimple(M, weapon_data[w]+0x02 (the base weapon), rounds, type 0 = weapon)`.
  - Weapon 0x1A also drops 0x1B (its grenade launcher) as type 1 (ammo), rounds `weapon_data[0x1B]+0x92` ·
    (0.25 + FRand(0.5)), with `obj+0xF0 |= 0x10` (see U-036).
- `DroneFunc_OnInitDeath` (0x145bd0): a captain (`Drone+0x19` ≠ 0) drops grenades at its own matrix (`obj+0x90`) by
  `DroneFunc_DropGrenade` → `Pickup_CreateSimple(…, id, n, 0)`: 07000001–08 → weapon 0x35 with 2; 07000009–0B,
  11–16, 4A → weapon 0x34 with 1 (`a2` = 1, read in the disassembly at 0x145de8).
- Level placements (`parsemap_create_dynamic_objects`), scripts (`Script_CreateEntity`), `GT_Update`,
  `Player_CheckForDeath`, `Pickup_CreateFromSet` (types 7–11): not read yet (G-029).

## `Pickup_CreateSimple` (0x18f348) [M, disassembly]

- Rounds 0 → nothing. Id remap: 0x6A / 0x6E → 0x33; none for 1, 0x45, 0x47, 0x50, 0x51, 0x53 or id − 1 > 0x70.
- `Pickup_Create(NULL, NULL, M, glist(weapon_data[id]+0x80), type, weapon_data[id]+0x00, rounds, switch 0,
  sound 0xF5, rest 0 (drop to the floor), respawn 0, lifetime FRAME_RATE_INT · (MP ? 30 : 120), label 0)` — the stack
  arguments read at 0x18f43c–0x18f488. FRAME_RATE_INT = 60 in the snapshot [L], so a dropped gun lasts **7200 ticks**
  (2 minutes) [L+M].

## `Pickup_Create` (0x18dda0) [M]

- No glist, id −1, or type 6 (Bond bonus) without the gold medal → nothing. Types 7–11 → `Pickup_CreateFromSet`.
- `control_create_object(0x40)`; `obj+0x90` = M; PICKUPINFO at `obj+0xE0`: `+0x00` velocity (zero), `+0x10` offset
  translation − sphere centre, `+0x20` state (0 falling, 1 resting, 2 waiting to respawn), `+0x22` type, `+0x24` id,
  `+0x26` rounds, `+0x28` switch channel, `+0x2A` sound (0xFFFF none), `+0x2C` respawn, `+0x2E` lifetime, `+0x30` −1,
  `+0x34` label, `+0x38` spin (type 6 or multiplayer).
- `obj+0xF0` = glist flags & ~0x2000 | 0x840; `obj+0xFA |= 0x44`; `obj+0xFF` = 0x2F.
- Position: `build_PointOnFloor(1.0, cel, …)` on the translation → `obj+0x30` / `+0x40`.
- Rest 0 (drops): the matrix is turned upright (`Mat_Align2Up(M, (0,1,0), Mat_GetDir(M))`: it keeps its heading),
  pushed out of overlapping geometry (`Collide_SphereIntersect`, mask 0x50D), lowered by radius − |glist centre y|,
  state 0. Rest ≠ 0: state 1.

## `Pickup_Update` (0x18e148), every tick [M]

- State 0 (falling): velocity += `WldGravity` (0, −9.8, 0) [L] · `REC_FRAME_RATE`² (1/60 [L]); translation +=
  velocity. Ray from `obj+0x40` to translation − (0, |glist centre y|, 0) (mask 0x50D); when it hits and the
  translation is below the hit: position = hit + `HIT+0x30` · (−|centre y|) + info `+0x10` (see U-036), velocity
  zeroed, state 1. Hits on moving objects (types other than 2, 5, 0x11, 0x1A) give the pickup their velocity
  (`Control_InheritVelocity`, 3.0) and keep it falling.
- State 2 (taken, respawning): after `FRAME_RATE_INT · respawn · 10` ticks it shows again (`obj+0xF0 &= ~0x10`),
  state 1; nothing else that tick.
- Spin (`+0x38`): about +Y by `REC_FRAME_RATE` radians per tick.
- Touch: for each player (`glb_players`, alive `obj+0xF4` = 1, not type 0x12): squared distance from the pickup's
  sphere centre (`obj+0x80`) to the player's position (`obj+0x30`) < (radius `obj+0x8C` + 1.1)² and
  `Collide_LineOfSight(player obj+0x80 → pickup obj+0x80, mask 0x721)` → `Pickup_Handler`, and nothing else this tick.
  (Multiplayer bots likewise.)
- Lifetime (`+0x2E`): when non-zero, −1 per tick; at 0 the pickup is deleted (`obj+0xFE |= 1`).

## `Pickup_Handler` (0x18e650), Bond [M]

- Id: `Upgrade_Weapon(id)` in single player (6 → the upgraded handgun, 0x1E / 0x24 snipers, 0x43, 0x4A, 0x4E, 0x56 by
  the upgrade bytes `0x2BEEAA…`; 10 → 0xC with `GameState+0x52`; others unchanged).
- Type 0 (weapon): Bond without it (`BLData+0x1BA + id·0xC` = 0) → `Player_EquipWeapon`; with it →
  `Player_EquipAmmo` (message as ammo). Type 1 (ammo) → `Player_EquipAmmo`. Either returning 0 leaves the pickup
  where it is. Type 2 → `Player_EquipKey`; 3 → armour 50 (`+0x8B0`, only when below 50); 5, 6: special.
- Message: weapon "%s" (label `0x200000D` with `weapon_data+0x38`), ammo "%d × %s" (`0x200000E`, `ammo_data+8` of
  `weapon_data+0x90`), shown 180 ticks (`Text_AddMsg`). Weapon / ammo pickups get sound 0xF5.
- Switch channel `+0x28` → `switch_channels` = 1 with the time.
- `Sound_Play3D(sound, at the translation, 100.0, −1, −1)` — a sound the drones may hear (`Sound_Alertness`).
- Respawn 0 → deleted; −1 → stays; else hidden, state 2, stamp `obj+0xEC`.

## Bond's inventory (BLData = player `obj+0xE0`) [M, L]

- `+0x170 + 2·t`: rounds carried per ammo type t (`weapon_data[w]+0x90`); cap `ammo_data[t]+0x02` (12-byte records
  at 0x2C69B8). 07000005 start [L]: type 1 = 41, all others 0.
- `+0x1B8 + 0xC·w`: s16 rounds in weapon w's clip; `+0x1BA`: owned. 07000005 start [L]: owned 1, 2 (P2K, clip 7),
  0x4A, 0x4E, 0x50, 0x54 (gadgets).
- `Player_EquipWeapon` (0x1a8a58): needs `weapon_data[id]+0xDC` loaded (`hashtable_getitem`, the first-person model;
  else refused). Not owned (by the base `+0x02`): owned, clip = `+0x92` (a full clip), carried += rounds + old clip −
  clip size when positive. Owned: refused when carried ≥ cap or rounds = 0, else carried += rounds. Carried clamped
  to the cap. With auto-switch on (`0x2A38D2`) and `Player_IsBetterWeapon`: `anim+0x63` = id (switch request).
- `Player_EquipAmmo` (0x1a8d18): carried < cap and rounds ≠ 0 → carried += rounds (flag `+0x68 & 0x10` weapons fill
  an empty clip first), clamped; returns the rounds taken (0 = refused).
- 07000005 [L]: guards hold SG5 (0x16: base 0x16, clip 30, ammo type 5, cap 90, model 0x02000B44); the sniper holds
  0x24 (clip 5, ammo type 7, cap 30). Bond's P2K uses type 1 (cap 70).

## Port

`nf_game::pickup` (WORKLOG 0040): drops from Death_Anim / Dead, captain grenades, falling, resting, touch, lifetime,
the handler for types 0 / 1 with `EquipWeapon` / `EquipAmmo` on an inventory that feeds Bond's `Gun`; the pickup sound
goes to the hearing list. *Deviations*: the rest height (U-036), no sphere push-out at creation, static world only
(no moving platforms), `hashtable_getitem` taken as loaded, no weapon switching (Bond keeps the P2K; G-029), no text.
