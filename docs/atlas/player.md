# Player structs atlas

The per-player runtime state: `BLData` (the big block), `obj_tag` (the player object), `M_CONTROL` (the input/control
struct, which holds the aim flag), `PlayerSetting` (the save-file per-player settings), and the `ammo_data` /
inventory arrays with their indexing.

Tags: `[M]` decompile, `[L]` `work/ee_start_05.bin` snapshot, `[S]` suggested.

---

## BLData — per-player runtime block

### Allocation site `[M]`

BLData is **not allocated on its own**. It is the trailing 0x970 bytes of a single `obj_tag` allocation, laid out as:

```
obj_tag header   : 0x000 .. 0x110   (0x110 bytes — control fields, matrix, etc.)
BLData           : 0x110 .. 0xA80   (0x970 bytes — per-player runtime state)
```

The allocation is performed by **`control_create_object`** (`001344e0_control_create_object__FUiP7_VECTORT1P14quaternion_tag.c`),
called from **`Player_Init`** (`001910f8_Player_Init__FsP7_VECTORT1P9level_tag.c`):

```c
uVar4 = control_create_object__FUiP7_VECTORT1P14quaternion_tag(0x970, param_2, 0, 0);
```

Inside `control_create_object`:

```c
lVar1 = Mem_Malloc__FUiUiUi((long)(param_1 + 0x110), 0x404, 0);  // alloc 0xA80 bytes
memset(__s, 0, (long)(param_1 + 0x110));                          // zero whole block
__s[0x38] = __s + 0x44;                                           // *(obj+0xe0) = obj+0x110
```

`__s[0x38]` (offset `0x38 * 4 = 0xe0` in the obj_tag) is set to `__s + 0x44` (offset `0x44 * 4 = 0x110`), i.e.
**`obj_tag+0xe0` is a self-pointer to the embedded BLData** (start of the per-player block) `[M]`.

Player slots are tracked by two parallel globals:

| global | addr | size | meaning | tag |
|---|---|---|---|---|
| `glb_players` | `0x002d88e0` | 16 B | `obj_tag*[4]` — one per port | `[M]` symtab |
| `glb_blokes`  | `0x002ba190` | 16 B | `BLData*[4]` — aliases `glb_players[i]+0x110` | `[M]` |

### Size `[M]`

**`sizeof(BLData) == 0x970`** (2416 bytes). Total allocation: `0x970 + 0x110 = 0xA80` (2688 bytes). Highest field
offset observed: `+0x968`.

### Field atlas

Offsets are from `BLData*` (= `obj_tag* + 0x110`).

#### Velocity / orientation block (0x10..0x60)

| offset | type | name | meaning | writers (fn@addr) | readers (fn@addr) | tag |
|---|---|---|---|---|---|---|
| `0x10` | f32[4] | `localVelSmooth` | Smoothed local-space velocity (XYZ+pad) | `Player_Move`@`0x191dd0`, `Player_StandAtNewPosition`@`0x190d88` | `Player_Collision`@`0x194df0` | `[M]` |
| `0x20` | f32[4] | `localVel` | Local-space input/target velocity (controller-derived); the value the smoother chases | `Player_SSGrapple`@`0x192860`, `Player_StandAtNewPosition` | `Player_Move`@`0x191dd0` | `[M]` |
| `0x28` | f32 | `forwardSpeed` | Forward speed component (drives the view bob `+0x8DC`) | `Player_Move`@`0x191dd0` | `Player_Aiming` (bob), `Sim::run` bob | `[M]` |
| `0x30` | f32[4] | `rotDelta` | Rotation delta accumulator; Y (`+0x34`) holds body-yaw delta | `Player_Aiming`@`0x1a1d28`, `Player_Move`@`0x191dd0` | `Player_FeetOnPoint`@`0x194188`, `Player_CollisionHandler`@`0x1949a8` | `[M]` |
| `0x40` | f32[4] | `turnRate` | Turn-rate accumulator (X=aim yaw, Y=aim pitch deltas) | `Player_Aiming`@`0x1a1d28` | `Player_Move` | `[M]` |
| `0x50` | f32[4] | `worldVel` | World-space velocity vector (XYZ+pad); zeroed on jump/grapple landing | `Player_Init`@`0x1910f8`, `Player_HandleJump`@`0x1916b0` | `Player_HandleJump`, `Player_CollisionHandler` (reads `+0x54`) | `[M]` |

#### Collision / floor block (0x110..0x740)

| offset | type | name | meaning | writers | readers | tag |
|---|---|---|---|---|---|---|
| `0x110` | f32[4] | `floorNormal` | Floor normal vector from `Collide_RayIntersect`; defaulted to `(1,0,0,0)` if no hit | `Player_FeetOnPoint`@`0x194188` | `Player_CollisionHandler`@`0x1949a8` | `[M]` |
| `0x118` | f32 | `cursorX` | Aim-mode cursor X (−1..1 across half the screen) | `Player_Aiming`@`0x1a1d28` | `Player_SetWeaponAnimObj`@`0x1a2e38`, `Player_WeaponFiring` | `[M]` |
| `0x11C` | f32 | `cursorY` | Aim-mode cursor Y | `Player_Aiming` | `Player_SetWeaponAnimObj`, `Player_WeaponFiring` | `[M]` |
| `0x120` | f32 | `crossX` | Smoothed crosshair offset X (the smoothed copy of `cursorX`) | `Sim::run` (smooths toward `+0x118`) | `crosshair()`, `gun_matrix` | `[M]` |
| `0x124` | f32 | `crossY` | Smoothed crosshair offset Y | `Sim::run` (smooths toward `+0x11C`) | `crosshair()`, `gun_matrix` | `[M]` |
| `0x130` | u8 | `moveMode` | 0=normal, 1=aiming, 2=locked (vehicle/ladder/scan), 3=weapon init | `Player_Aiming`, `Player_InitWeapon`@`0x1a7ef8`, `Player_SSWalk`@`0x192310`, `Player_SSCrouch`@`0x1924c0` | `Player_SSWalk`, `Player_SSCrouch` | `[M]` |
| `0x15E` | s16 | `grappleActive` | Grapple mode flag (checked by `Player_SSGrapple`) | — | `Player_SSGrapple`@`0x192860` | `[M]` |
| `0x170` | s16[33] | `ammoSpare` | Spare-rounds count per ammo type `a`; `BLData+0x170 + a*2` | `Player_EquipWeapon`@`0x1a8a58`, `Player_EquipAmmo`@`0x1a8d18` | `Player_AmmoInGun`@`0x1a9190`, `Player_WeaponHasAmmo`@`0x1a16c0`, `Player_WeaponFiring` | `[M]` |

#### Weapon entry array (0x1B4..0x70C)

`BLData+0x1B4 + w·0xC` — per-weapon 0xC-byte entry (114 entries, 0x558 bytes total).

| entry offset | type | name | meaning | tag |
|---|---|---|---|---|
| `+0x0` (`0x1B4+w·0xC`) | f32 | `savedZoom` | Per-weapon saved zoom factor; written by `Player_Zoom`; restored on weapon switch | `[M]` |
| `+0x4` (`0x1B8+w·0xC`) | s16 | `clip` | Rounds in magazine for weapon `w` | `[M]` |
| `+0x6` (`0x1BA+w·0xC`) | u8 | `owned` | 1 = weapon owned; cleared by `Player_SetWeaponAnimObj` when thrown | `[M]` |
| `+0x7` (`0x1BB+w·0xC`) | u8 | `ammoTypeIdx` | Loaded ammo-type index for this weapon | `[M]` |
| `+0x8` (`0x1BC+w·0xC`) | u8[4] | (reserved) | Tail padding / unused | `[M]` |

**`+0x4F4` is NOT a separate field** — it is the clip of grenade weapon 0x45: `0x1B8 + 0x45·0xC = 0x4F4` `[M]`.
`Player_WeaponHasAmmo` reads `*(s16*)(BLData + 0x4F4)` only when `param_2 == 0x45`.

#### Health / view-state block (0x740..0x970)

| offset | type | name | meaning | writers | readers | tag |
|---|---|---|---|---|---|---|
| `0x740` | struct | `feetHitTest` | Collision/feet hit-test substruct (`obj_tag+0xd4` → here) | `Player_Init`, `Player_CollisionHandler` | `Player_FeetOnPoint`@`0x194188` | `[M]` |
| `0x7E8` | obj_tag* | `weaponViewModel` | Pointer to the weapon's anim object (view-model) | `Player_InitWeapon`@`0x1a7ef8` | `Player_SetWeaponAnimObj`, `Player_WeaponFiring`, `Player_PositionGun` | `[M]` |
| `0x86C` | obj_tag* | `muzzleFlashObj` | Pointer to the muzzle-flash sprite object | `Player_InitWeapon` | `Player_WeaponFiring`, `Player_MuzzleFlash` | `[M]` |
| `0x88C` | obj_tag* | `bodyGlow` | Pointer to BodyGlow effect object | `Player_Init`@`0x1910f8` | `BodyGlow_Update`@`0x1ac2c8` | `[M]` |
| `0x894` | f32 | `health` | Current health points | `Player_SetHealth`@`0x1960b8` | `Player_Hurt`@`0x196060`, `DroneWeap_Ready2Fire`@`0x178c48` | `[M]` |
| `0x8A8` | f32 | `pitch` | Pitch angle in units of π/2; range [−1.0, +1.0]; × π/2 wherever used | `Player_Init` (0.0), `Player_Aiming` (lines 192, 276), `Player_ViewClamping`@`0x1a92e8` (clamps ±1) | `Player_GetAimingPoint`@`0x1a4a98`, `Player_PositionGun`@`0x1a3b40`, `Camera_SetToPlayer`@`0x1bc048` | `[M]` |
| `0x8B0` | f32 | `armor` | Armor points (max 50.0); decremented by `Player_HandlePain` | `Player_Init` (0), `Player_HandlePain`@`0x1902c8`, `Pickup_Handler`@`0x18e650` (50.0) | `Player_RamSave`, `HUD_UpdateMPHealthPane` | `[M]` |
| `0x8B8` | f32 | `respawnHealth` | Saved health at last spawn point | `Player_Init`, `MP_ReSpawn`@`0x185368` | — | `[M]` |
| `0x8D0` | f32 | `zoom` | Current zoom multiplier (FOV scale); 1.0 = no zoom; ≤50.0 | `Player_Init` (1.0), `Player_Zoom`@`0x1a1a70`, `Player_WeaponNone`@`0x1a9858` (1.0) | `Player_Aiming`, `Player_PositionCamera`@`0x18f5d8`, `HUD_UpdateLensFlarePane` | `[M]` |
| `0x8D4` | f32 | `zoomTarget` | Target zoom (per-weapon); clamped to [1.0, `+0x8D8`] | `Player_Init` (1.0), `Player_Zoom`, `Player_SetWeaponAnimObj` (restored from `savedZoom`) | `Player_Zoom` | `[M]` |
| `0x8D8` | f32 | `zoomMax` | Per-weapon max zoom cap; loaded from `weapon_data[+0x88]` | `Player_Init` (1.0), `Player_SetWeaponAnimObj` | `Player_Zoom` | `[M]` |
| `0x8DC` | f32 | `bobPhaseFwd` | View bob phase (driven by `+0x28` forward speed) | `Sim::run` (bob update) | `gun_matrix` | `[M]` |
| `0x8E8` | f32 | `bobPhaseIdle` | View bob phase (idle; += (π/2)/60 each tick) | `Sim::run` (bob update) | `gun_matrix` | `[M]` |
| `0x900` | f32 | `noAimTurnSpeedY` | Non-aim turn-speed Y accumulator | `Player_Aiming` | `Player_Aiming` | `[M]` |
| `0x908` | f32 | `landThud` | Eye-height offset from landing impact; exponentially smoothed (×0.1) | `Player_PositionCamera`@`0x18f5d8` | `Player_PositionCamera`, `Player_GetHeadPos`@`0x195aa0` | `[M]` |
| `0x90C` | f32 | `eyeDropCrouch` | Crouch-induced eye-drop offset; interpolated ±0.0225/frame toward 0 (stand) or 0.45 (crouch) | `Player_PositionCamera`, `Player_Init` (0.45 for crouch), `Player_StandAtNewPosition` (0) | `Player_PositionCamera`, `Player_GetHeadPos`, `Player_SetWeaponAnimObj` | `[M]` |
| `0x924` | f32 | `animPhase` | Per-frame anim phase accumulator | `Player_HandleJump`@`0x1916b0`, `AnimObjectUpdate`@`0x1befd8` | `AnimObjectUpdate` | `[M]` |
| `0x928` | f32 | `ticksFalling` | Fall timer (seconds); >60.0 ⇒ fall damage | `Player_CollisionHandler`@`0x1949a8` | `Player_CollisionHandler`, `Player_HandlePain` | `[M]` |
| `0x92C` | f32 | `refireTimer` | Time-until-next-shot; set to `weapon_data[+0x40]` after each shot | `Player_InitWeapon`@`0x1a7ef8` (0), `Player_WeaponFiring`@`0x1a5460`, `Player_SetWeaponAnimObj` | `Player_WeaponFiring` | `[M]` |
| `0x930` | s16 | `lastWeapon` | Last non-gadget weapon slot (toggle-back) | `Player_WeaponChange`@`0x1a6f80`, `Player_InitAmmoWeapons`@`0x1a17f8` (0) | `Player_WeaponChange` | `[M]` |
| `0x932` | s16 | `lastGadget` | Last gadget slot (toggle-back); 0x54 if none | `Player_WeaponChange`, `Player_InitAmmoWeapons` (0) | `Player_WeaponChange` | `[M]` |
| `0x934` | s16 | `equipSlot` | Currently-equipped weapon slot; saved across checkpoints | `Player_RamLoad`@`0x1957a8` | `Player_InitWeapon`@`0x1a7ef8` | `[M]` |
| `0x936` | u16 | `shotsLeftPull` | Shots remaining in the current trigger pull | `Player_InitWeapon` (0), `Player_WeaponFiring` | `Player_WeaponFiring`, `Bullet_init`@`0x12c5a0` | `[M]` |
| `0x938` | u8 | `flashTimer` | Bond's muzzle-flash timer (3 ticks after his shot) | `Player_WeaponFiring` | `Sim::run` (checks == 3) | `[M]` |
| `0x940` | s16 (low 4 bits) | `grounded` | 4-tick grounded shift register; bit 3 = most recent | `Player_CollisionHandler`@`0x1949a8` | `Player_CollisionHandler`, `Player_HandleJump` | `[M]` |
| `0x94A` | u8 | `disabled` | Player-disabled timer (cut-scene lock); 0 = disabled, 1 = active, 2 = enable-requested | `Player_Init` (1), `Player_Disable`@`0x195f70` (0), `Player_Enable`@`0x195498` (2), `Player_Update`@`0x193600` (decrement) | `Player_Update`, `Player_CollisionHandler` (early-return if 0), `HUD_MonitorNightSight` | `[M]` |
| `0x94C` | u8 | `jumpState` | 0=grounded, 1=jumping (apex), 2=falling | `Player_HandleJump`@`0x1916b0`, `Player_Enable` (0) | `Player_HandleJump`, `Player_Collision`@`0x194df0` | `[M]` |
| `0x94E` | u8 | `playerIdx` | Player index (0..3) | `Player_Init` (line 38) | ~every `Player_*` fn | `[M]` |
| `0x952` | u8 | `landingDelay` | Jump-buffer / landing-delay countdown (set to 4 on jump apex) | `Player_HandleJump` (4, decrement) | `Player_HandleJump` | `[M]` |
| `0x959` | u8 | `viewScriptState` | View-model anim-script state (0=idle, 1=firing-started, 2=anim-playing, 3=finished) | `Player_SetWeaponAnimObj`@`0x1a2e38`, `Player_WeaponFiring` | `Player_SetWeaponAnimObj` | `[M]` |
| `0x960` | u8 | `crouchLockout` | Stand/crouch transition lockout timer (frames); 0 = not locked | `Player_ChangeSubState`@`0x190630`, `Player_SSWalk` (decrement), `Player_SSCrouch` (decrement) | `Player_SSWalk`, `Player_SSCrouch`, `Player_HandleJump` | `[M]` |

### Snapshot check `[L]`

Read from `work/ee_start_05.bin` (level 0x07000005 start). Pointer chase:
- `glb_players[0] @ 0x002d88e0` → `0x016cefe0` (obj_tag)
- `obj_tag+0xe0` → `0x016cf0f0` (BLData; diff from obj = 0x110 ✓)

| address | offset | field | value | tag |
|---|---|---|---|---|
| `0x016cf0f0 + 0x94A` | `0x94A` | `disabled` (u8) | `1` (active) | `[L]` |
| `0x016cf0f0 + 0x8A8` | `0x8A8` | `pitch` (f32) | `0.0` (looking ahead) | `[L]` |
| `0x016cf0f0 + 0x8D0` | `0x8D0` | `zoom` (f32) | `1.0` (no zoom) | `[L]` |
| `0x016cf0f0 + 0x894` | `0x894` | `health` (f32) | `100.0` (full) | `[L]` |
| `0x016cf0f0 + 0x8B0` | `0x8B0` | `armor` (f32) | `0.0` (none at start) | `[L]` |
| `0x016cf0f0 + 0x94E` | `0x94E` | `playerIdx` (u8) | `0` (player 0) | `[L]` |
| `0x016cf0f0 + 0x170` | `0x170` | `ammoSpare[1]` (s16) | `41` (pistol spare rounds) | `[L]` |
| `0x016cf0f0 + 0x1B8` | `0x1B8` | `weaponEntry[2].clip` (s16) | `7` (P2K clip) | `[L]` |

### Inventory / ammo indexing `[M]`

```
weapon id w (1..0x47 weapons, 0x4A..0x5E gadgets)
  → clip:      BLData + 0x1B8 + w·0xC   (s16)
  → owned:     BLData + 0x1BA + w·0xC   (u8)
  → savedZoom: BLData + 0x1B4 + w·0xC   (f32)

ammo type a (0..32)
  → spare:     BLData + 0x170 + a·2     (s16)
  → cap:       ammo_data + a·12 + 0x02  (s16)
```

The weapon id → ammo type mapping is `weapon_data[w·0x10C + 0x90]` (see [weapon-data.md](weapon-data.md)).

---

## obj_tag — the player object (header 0x000..0x110)

The `obj_tag` is the generic control object. For the player, `control_create_object(0x970, ...)` allocates an
obj_tag + 0x970 bytes of BLData. The player's obj_tag fields the port reads:

| offset | type | name | meaning | readers (fn@addr) | tag |
|---|---|---|---|---|---|
| `0x30` | f32[3] | `position` | World position (XYZ) | ~every `Player_*` fn | `[M]` |
| `0x54` | f32 | `yaw` | Body yaw (0 faces +Z; larger turns left) | `Player_Move`, `Player_Aiming` | `[M]` |
| `0x90` | f32[4][4] | `matrix` | World matrix (rows 0–2 axes, row 3 translation) | `Player_PositionGun`, renderer | `[M]` |
| `0xD4` | ptr | `feetHitTest` | → `BLData+0x740` (set by `Player_Init`) | `Player_FeetOnPoint` | `[M]` |
| `0xDC` | ptr | `mControl` | → `M_CONTROL` (the control/input struct) | `Player_Aiming`@`0x1a1d28`, `Player_Zoom`@`0x1a1a70`, `Player_HandleJump`@`0x1916b0` | `[M]` |
| `0xE0` | ptr | `blData` | → `BLData` (self-pointer to `obj+0x110`) | ~every `Player_*` fn | `[M]` |
| `0xF0` | u32 | `flags` | Object flags (bit 0x10 = hidden) | renderer | `[M]` |
| `0xF6` | s16 | `subState` | Player sub-state (7 < s < 0xD and s == 0x10 skip aiming) | `Player_Aiming`@`0x1a1d28` | `[M]` |
| `0xFE` | u8 | `deleted` | Deleted flag (bit 0 |= 1) | object handler | `[M]` |

### Snapshot check `[L]`

| address | offset | field | value | tag |
|---|---|---|---|---|
| `0x016cefe0 + 0xE0` | `0xE0` | `blData` (ptr) | `0x016cf0f0` (= obj + 0x110 ✓) | `[L]` |
| `0x016cefe0 + 0xDC` | `0xDC` | `mControl` (ptr) | `0x016cb8a0` | `[L]` |

---

## M_CONTROL — the control/input struct (aim flag)

`M_CONTROL` is the per-player control/input struct. It is pointed to by `obj_tag+0xDC`. The **aim flag** is at
`M_CONTROL+0x60` (a u16 bitmask).

### Aim flag bits `[M]`

| bit | name | meaning | readers (fn@addr) | tag |
|---|---|---|---|---|
| `0x1` | `aiming` | Currently in aim mode (right mouse held / aim toggle) | `Player_Aiming`@`0x1a1d28`:31, `Player_Zoom`@`0x1a1a70`:18 | `[M]` |
| `0x8` | `jumpPending` | Jump requested (set by jump input, cleared on land) | `Player_HandleJump`@`0x1916b0`:67,103,182,191, `Player_SSGrapple`@`0x192860`:89 | `[M]` |
| `0x20` | `touchedAbove` | Something touched above the player (head bump) | `Player_HandleJump`@`0x1916b0`:103 | `[M]` |

### Snapshot check `[L]`

| address | offset | field | value | tag |
|---|---|---|---|---|
| `0x016cb8a0 + 0x60` | `0x60` | `aimFlags` (u16) | `0x0008` (bit 0x8 set — jump pending; bit 0x1 clear — not aiming) | `[L]` |

---

## PlayerSetting — per-player save settings

### Allocation site `[M]`

| symbol | addr | size | meaning | tag |
|---|---|---|---|---|
| `PlayerSetting` | `0x002a38c8` | 1376 B | 4 players × 0x158 (344) bytes | `[M]` symtab |
| `player_setting` | `0x002de708` | 1376 B | mirror / alternate access | `[M]` symtab |

`Input_Init`@`0x17edd8` does `memset(&PlayerSetting + player_idx, 0, 0x158)` — confirms the 0x158 per-player stride.
`LS_LoadPlrSettings`@`0x1cd5b8` saves/loads the block via `BIN_PullBits`.

### Fields (player 0 block at `0x002a38c8`)

| offset | type | name | meaning | tag |
|---|---|---|---|---|
| `+0x01` | u8 | `autoAim` | Auto-aim option (1 = on in the game; the port turns it off by default with a mouse, D-001) | `[M]` `scene.rs:181`, `[L]` |
| `+0x02` | u8 | (unknown) | Read by `LS_LoadPlrSettings`; a per-player setting byte | `[M]` (reader only) |
| `+0x04`..`+0x0A` | u8 × 7 | (control flags) | Per-player control setting bytes | `[M]` (reader only) |

### Snapshot check `[L]`

Read from `work/ee_start_05.bin` at `0x002a38c8` (player 0's block):

| address | offset | field | value | tag |
|---|---|---|---|---|
| `0x002a38c8 + 0x01` | `+0x01` | `autoAim` (u8) | `1` (on) | `[L]` |
| `0x002a38c8 + 0x04` | `+0x04` | (control flag) | `1` | `[L]` |
| `0x002a38c8 + 0x00` | `+0x00` | (first byte) | `0` | `[L]` |

---

## ammo_data — the ammo type table

### Allocation site `[M]`

| symbol | addr | size | meaning | tag |
|---|---|---|---|---|
| `ammo_data` | `0x002c69b8` | 408 B | 12-byte records × 34 ammo types | `[M]` symtab |

### Field table

| offset | type | name | meaning | readers | tag |
|---|---|---|---|---|---|
| `+0x02` | s16 | `cap` | Max rounds of this ammo type Bond can carry | `WeaponData::ammo_cap`, `Player_EquipAmmo` | `[M]` |

### Snapshot check `[L]`

Read from `work/ee_start_05.bin` at `0x002c69b8`:

| address | ammo type | `+0x02` cap | tag |
|---|---|---|---|
| `0x002c69b8 + 1·12 + 2` | 1 (pistol) | 70 | `[L]` |
| `0x002c69b8 + 5·12 + 2` | 5 (SMG/SG5) | 90 | `[L]` |
| `0x002c69b8 + 7·12 + 2` | 7 (sniper) | 30 | `[L]` |
| `0x002c69b8 + 9·12 + 2` | 9 (shotgun) | 24 | `[L]` |

---

## Open questions

- **`BLData+0x1BC`** (the per-weapon entry's `+0x8` reserved tail): the port marks it STANDIN for a "variant-slot
  adjustment" but the decompile shows it as tail padding. `[S]`
- **`PlayerSetting+0x02`..`+0x0A`**: the per-player control setting bytes — their individual meanings need the
  `C_CNCONTROLS_Handler` / `P_CNCONTROLS_Handler` decompile to decode. `[M]` (reader only, meaning unclear)
- **M_CONTROL full layout**: only `+0x60` (the aim flag u16) is documented here; the rest of the M_CONTROL struct
  (input state, button buffers) is out of scope for the player atlas but worth a separate entry if the port needs it.

## PCSX2 trace check: firing at civilians (tests/traces/02-civilian.trace, 2026-10-11) [L]

User play on 07000002 with real ammo (god mode only), aiming at a civilian under the red no-shoot crosshair and pulling the trigger several times, then one shot at a wall. The P2K's clip (BLData+0x1B8 + 2·0xC) fell on every pull: 7→6 (6.2 s), 6→5 (7.2 s), reload, then 7→5→4→3→2→0 (14.2–16.1 s), reload, 7→6 (19.6 s). **The trigger is not blocked by the red mark** — it gates the auto-aim lock only, as the code reading (`Check_Target` state 3 read only by the HUD) said. Caveat: the trace does not record the crosshair state, so it shows the shots fired while the user aimed at the civilian, not the mark at each pull.
