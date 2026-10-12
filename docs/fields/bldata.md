# BLData — the player's data block (the class-3 instance, 0x970 bytes)

Proof levels: see [README.md](README.md). Live instance: `glb_players` 0x2d88e0 → obj 0x16cefe0 →
`obj+0xE0` → **BLData = obj+0x110 = 0x16cf0f0** in `work/ee_start_05.bin` (07000005 at start) [L].
The allocation is `control_create_object(0x970, …)` in `Player_Init` 0x1910f8:33 [M] — **the struct is
0x970 bytes**; `obj+0xD4 = BL+0x740` (the ground HITTEST back-pointer) [M 0x1910f8:40]. Port: the fields
live on `nf_game::player::Bond` + `nf_game::gun::Gun` + `nf_game::bullet::PlayerHealth` + `Drones`
(inventory/health), each cited at its row.

Evidence: [M] decompile `work/decomp/ACTION.ELF/<file>:<line>` (line numbers from the Ghidra export);
[L] the EE snapshot values at the cited address; [S] suggested. The decompiler's shifted-pointer
artifacts (`puVar2 + 0x13d` etc. in `Player_Update`) were settled against the disassembly
(`python3 tools/crosscheck/mipsdis.py`) — the regen block's real slots are the clip addresses below,
not the byte offsets the C prints.

## 0x000–0x0F: the position snapshot

| Offset | Type | Meaning | Proof | [L] ee_start_05 | Port |
|---|---|---|---|---|---|
| +0x00..0x0C | vec4 | copy of `obj+0x30..0x3C` (position + the placement W −0.9875) saved each tick | [M1 Player_Update 0x193600:127-135 (stack copy into the save); L values = spawn pos] | (104.954, 24.963, 108.825, −0.9875) | not modeled (port keeps `before` locally) |

## 0x10–0x5C: velocity block

| Offset | Type | Meaning | Proof | [L] | Port |
|---|---|---|---|---|---|
| +0x10 / +0x14 | f32 ×2 | the local velocity being built this tick (x strafe-left, z forward), zeroed every tick | [M* Player_Update 0x193600 (`Vec_Zero(puVar2+4)` = +0x10), Player_Move 0x191dd0:158-169 (writes the quadword), Player_SSWalk 0x192310:53] | 0 | `Bond::vel`-adjacent; the port eases directly into `local` |
| +0x18 | f32 | the z lane of the same store: Player_Move writes `+0x18 = z_target · k` (the `f` param × the eased value) | [M1 0x191dd0:169] | 0 | (same store) |
| +0x20 / +0x28 | f32 ×2 | last tick's local velocity (x, z) — the ease source (`target − last)·0.2`), zeroed by `Player_ClearInertia` 0x195e08 | [M1 0x191dd0:99-100, 119-120; M1 0x195e08] | 0 | `Bond::local: [f32;2]` [M player.rs:100] |
| +0x30..0x3C | vec4 | rotation delta (zeroed every tick with +0x10) | [M1 0x193600 (`Vec_Zero(puVar2+0xc)`); M1 0x192310:74 `RotMatrix(BL+0x30, …)`] | 0 | not modeled (turn applied directly) |
| +0x40..0x4C | vec4 | zeroed by `Player_ClearInertia` 0x195e08 with the others; no other accessor found | [M1 0x195e08] | 0 | — |
| +0x50..0x5C | vec4 | the world velocity (m/s): position += M·vel each tick; `Vec_Cartesian_2_Spherical` writes the debug scan-mode's spherical angles into it; swim copy to +0x60 | [M* 0x193600:284-305 (lqc2/sqc2 at +0x50), 0x192310, Player_InWater 0x195e50; L 0] | 0 | `Bond::vel` [M player.rs:102] |
| +0x60..0x6C | vec4 | copy of +0x50 made in the debug scan-mode branch (0x193600:146-155); snapshot value (−109.85, 14.89, −30.6) has no pre-debug writer found | [M1 0x193600:150-152] | nonzero, unresolved | — |

## 0x110–0x13F: floor, aim cursor, crosshair state

| Offset | Type | Meaning | Proof | [L] | Port |
|---|---|---|---|---|---|
| +0x110 | f32 | floor normal y — written by the feet-point query, read as the walkable-face gate (< 0.5) | [M* Player_CollisionHandler 0x1949a8:77 (passed to `Player_FeetOnPoint`), Player_Collision 0x194df0:104] | 0.9954 | `Bond::floor_ny` [M player.rs:106] |
| +0x114 | u32 | the auto-aim target obj (stored/compared by `Check_AutoAim` 0x1a73f8) | [M1 0x1a73f8:155,175] | 0 | implicit in auto-aim |
| +0x118 / +0x11C | f32 ×2 | the raw aim-mode cursor (x, y): stick · speed · 0.125, clamped ±0.296875 / ±0.4, zeroed when aim mode ends | [M* Player_Aiming 0x1a1d28:83-106, 268-272] | 0 | `scene.rs` cursor [M scene.rs:scene head] |
| +0x120 / +0x124 | f32 ×2 | the smoothed crosshair offset (x, y) — the auto-aim snap writes the target's screen position; the shot direction uses (0.76966·x, 0.57724·y) | [M1 Check_AutoAim 0x1a73f8:172-176; M docs/systems/player.md §Aim] | 0 | `Sim::cross` [M scene.rs] |
| +0x128 / +0x12C | f32 ×2 | the aim-mode look deltas applied this tick (y, x) — × the +0x8F4/+0x8F8 ramps, ÷ the zoom, then pitch += y, yaw −= x·π/2 | [M1 Player_Aiming 0x1a1d28:181-190] | 0 | applied directly, not stored |
| +0x130 | u8 | the weapon-raise state: 0 raised / 1 lowering (`Player_Aiming` 0x1a1d28:72, `Player_Weapon` 0x1a24c8:160,184) / 2 special-anim re-trigger (SSWalk 0x192310:67, InWater 0x195e50:25, ZeroG 0x192ab0:73, Update 0x193600:189) / 3 lean-settled & spawn init (0x193600:274, `Player_InitWeapon` 0x1a7ef8:138); Player_Update acts on 2..3 | [M*] | 3 | not modeled as a field (the port's raise path is immediate) |
| +0x131 | u8 | the walk-anim speed band: 1 = \|v\| ≤ 0.1 (slow), 2 = ≥ 0.8 (run), else 0; a CHANGE re-triggers the anim set (+0x130 = 2) and (setting-dependent) a sound | [M* SSWalk 0x192310:59-67, SSCrouch 0x1924c0:75-80, StandAtNewPosition 0x190d88:82 (=1)] | 1 | not modeled |
| +0x132 | u8 | a target is under the crosshair (1) — `Check_Target` 0x1a7810 zeroes it each 6th-frame pass and sets it when the pick hits | [M1 0x1a7810:106,132,181] | 0 | implicit (cross_state) |
| +0x133 | u8 | the crosshair state: 2 = normal, 3 = a non-enemy target (the red "no" mark); 0 before the first pass | [M* Check_Target 0x1a7810:108,131,182; L 0 at frame 0] | 0→2 | `Sim::cross_state` [M scene.rs:218,1338] |

NOTE: **+0x44 is NOT a BLData field of the player** — the `BLData+0x44 != 1` test in `Check_Target`
0x1a7810:136-139 reads the TARGET's instance (a drone's team byte). The player's own +0x40..0x4C is the
zeroed vec4 above.

## 0x140–0x15F: the grapple block

| Offset | Type | Meaning | Proof | [L] | Port |
|---|---|---|---|---|---|
| +0x140..0x14C | vec4 | the grapple point (set by `Player_SetGrapplePoint` 0x195da0) | [M1 0x195da0:18-27] | 0 | grapple not ported |
| +0x150 | u32 | the grapple state counter (0 at set) | [M1 0x195da0:31] | 0 | — |
| +0x154 | u32 | the grapple target obj | [M1 0x195da0:15] | 0 | — |
| +0x158 | u32 | the grapple rope obj (`Control_CreateObjEx` with mesh 0x02000138, `Player_InitGrapple` 0x195d10) | [M1 0x195d10:9-15] | 0x016c5510 | — |
| +0x15C / +0x15E | u16 / u16 | grapple armed (1) / the timeout countdown (0xF) — `Player_SSGrapple` 0x192860:89 tests +0x15E == 0 | [M1 0x195da0:29-30] | 0 | — |

## 0x170–0x1B2: the ammo store (33 × u16 spare rounds, indexed by ammo type)

| Offset | Type | Meaning | Proof | [L] | Port |
|---|---|---|---|---|---|
| +0x170 + 2·i (i < 0x21) | u16 | spare rounds of ammo type i (the `ammo_data` record index, 0..32) | [M* Player_InitAmmoWeapons 0x1a17f8:16-29 (seed = ammo_data[i].max only under the all-weapons cheat 0x2a373c), Player_EquipAmmo 0x1a8d18:37,69-78 (add + clamp to ammo_data[type].max), Player_ReloadAmmoType 0x1a1988:23-38, Player_RamSave 0x195670:6 (→ RamSave+0x38, 0x42 bytes)] | mostly 0 | `Inventory::carried` [M pickup.rs/dev.rs] |

The full ammo-type table (34 × 12-byte `ammo_data` records at 0x2c69b8) is in [ammo-data.md](ammo-data.md).

## 0x1B4–0x70B: the weapon records (114 × 0xC, indices 0..0x71)

Proved by the initialiser loop (`Player_InitAmmoWeapons` 0x1a17f8:30-54, bound `< BL+0x70C`), the
RamSave copy (0x1B4..0x70C, 0x558 bytes → RamSave+0x7C) [M 0x195670:7] and the snapshot's sqrt lane
values. **+0x4F4 is not a separate field: it is `0x1B8 + 0x45·0xC` — weapon 0x45's clip** (the
no-reserve special of `Player_WeaponHasAmmo` 0x1a16c0:14-20; [D mipsdis 0x1a16f0 `lh v0,1268(s1)`]).

| Lane | Address | Type | Meaning | Proof | [L] (07000005 start) | Port |
|---|---|---|---|---|---|---|
| +0 | 0x1B4 + w·0xC | f32 | per-gun zoom level: init `SQRT(weapon_data[w].zoom f32@0x88)`, persisted from BL+0x8D0 by `Player_Zoom`, loaded into BL+0x8D4 on raise | [M* 0x1a17f8:51, Player_Zoom 0x1a1a70:72-77, Player_Weapon 0x1a24c8:175-177/220-222, SetWeaponAnimObj 0x1a2e38:324-326] | rec[0x21]=4.0, rec[0x5F]=1.4142 | not modeled (zoom is per-weapon f32 read directly) |
| +4 | 0x1B8 + w·0xC | s16 | clip rounds (init = weapon_data[w].clip_size +0x92); 0x4F4 = clip[0x45] regenerates (below); the fixed gadget slots 0x2F0=[0x1A], 0x2FC=[0x1B], 0x41C=[0x33], 0x530=[0x4A], 0x548=[0x4C], 0x560=[0x4E], 0x56C=[0x4F] are the regen/charge meters | [M* 0x1a17f8:41, Player_EquipWeapon 0x1a8a58:38-61, Player_EquipAmmo 0x1a8d18:39-64, Player_ReloadAmmoType 0x1a1988:26-37, Player_AmmoInGun 0x1a9190:19, the regen block 0x193600 disasm 0x193eb8-0x19405c] | clip[0x4A]=100, clip[0x4E]=500, clip[0x33]=1, clip[0x45]=0 | `Inventory::clip` [M gun.rs:105-112] — **regen ported (fields-player)** |
| +6 | 0x1BA + w·0xC | u8 | owned (init 0/1 new-game/load, 0 when weapon_data[w].selectable@+4 == 0; cleared on swap-out) | [M* 0x1a17f8:36-45, SetWeaponAnimObj 0x1a2e38:250-252, Pickup_Handler 0x18e650:197-199, Player_WeaponChange 0x1a6f80:181,191-195] | rec[2]=1, rec[0x4A]=1 | `Inventory::owned` ✓ |
| +7 | 0x1BB + w·0xC | u8 | shot counter (wraps at the AnimObj+0xDC record's threshold; indexes the burst table) | [M1 0x1a24c8:201-206, Player_WeaponFiring 0x1a5460:412-414] | 0 | not modeled |
| +8 | 0x1BC + w·0xC | u8 | the weapon-cycle skip counter (skips empty guns in the cycle) | [M1 0x1a17f8:48, 0x1a6f80:181,191-195] | 0 | not modeled (the port's cycle re-scans) |

The clip regen [M 0x193600 disasm 0x193eb8-0x19405c], all gated on the view-anim state (`AnimObj+0xF4`) != 9,
each slot +FRAME_RATE_MUL toward `weapon_data[w].clip_size` (+0x92), no ownership gate:
clip[0x4A] (+0x530) and clip[0x4E] (+0x560) and clip[0x33] (+0x41C) on `GameState+52 & 3 == 0`;
clip[0x4C] (+0x548) and clip[0x4F] (+0x56C) on `& 1 == 0`; clip[0x45] (+0x4F4) on `% 6 == 0`.
Port: `scene.rs::step_bond` (fields-player) with the same gates on `drones.frame`.

## 0x70C–0x73E: the key ring (51 bytes)

| Offset | Type | Meaning | Proof | [L] | Port |
|---|---|---|---|---|---|
| +0x70C + id | u8 | key `id` held (id = the raw u16 from the pickup/lock record — no remap table): set by `Player_EquipKey` 0x1a97a0 (pickup kind 2, id = pickupinfo+0x24), cleared by `Player_UnEquipKey` 0x1a97b8 / `Lock_ReLock` 0x1b0750; **gates the locks**: `Lock_Activate` 0x1aed00:19,26 (`BL+0x70C+id == 0` → stays locked), `Lock_Update` 0x1ae3d8:143 | [M* above + Pickup_Handler 0x18e650:217-219 + RamSave 0x195670:8 (→ +0xA1A, 0x33 bytes)] | all 0 | `Inventory::keys` set on pickup ✓; **the lock side is NOT consulted (gap — `lock.rs` burns only; reported)** |

No HUD reader of +0x70C exists (repo-wide rg) — the on-screen key display is not driven by the ring.

## 0x740–0x7DF: the player's HITTEST scratch

`obj+0xD4` points here [M 0x1910f8:40]; rebuilt every `Player_Collision` 0x194df0. +0x760 the per-substate
view offset (written :218,244,270), +0x7B8 the attached obj (weapon obj / +0x884 wire / +0x878 scan-mode,
:230,256,283,314,362), +0x7C0 the cel (:89), +0x7CC the cam distance (:204,231,255,341), +0x7D4 the u16
mode 0x800/0 (:205,229,257; `Player_Enable` 0x195498:75; `Player_Disable` 0x195f70:18), +0x7D8 u16 (:88,382).
The generic HITTEST_tag fields (engine, +0x10 = BL+0x750) are the atlas's business (docs/atlas/object.md).

## +0x7E0: the HUD block — a POINTER

`HUD_Init` 0x199e88:18-20 `Mem_Malloc(0x2C8)` → `*(BL+0x7E0)`; memset(hud+8, 0, 0x2C0). Read everywhere via
`*(BL+0x7E0)` (HUD_Update 0x19a088, HUD_UpdateHealthPane 0x19b9c8:124, Player_HandleDeath 0x190bb0:23-28,
Player_MonitorAir 0x191a70:45-50, Player_SetCamMode 0x195928:28,41, Monitor_Update 0x1afcc8, BodyGlow_Update
0x1ac2c8…). [L] 0x7E0 = 0x016c8630 (a heap pointer below BL). Layout: +0 the crosshair sprite, 22 pane
records of 0x20 at +8 (see [hud-panes.md](hud-panes.md) and [sprite.md](sprite.md)); HUDINFO scalars the
player path reads: +0x184 (HandleDeath gate), +0x1A4, +0x244 (night-sight on). +0x7E4 = 0 (no refs).

## 0x7E8–0x890: the object-pointer block

| Offset | Type | Meaning | Proof | [L] | Port |
|---|---|---|---|---|---|
| +0x7E8 | ptr | the weapon AnimObj (created in `Player_InitWeapon` 0x1a7ef8:67; the view-model) | [M* 33 files: 0x1949a8:185, 0x194df0:314, 0x1a2e38:40…] | 0x016c7c70 | `Scene::view_models` (per-weapon characters) |
| +0x7EC..0x868 | ptr ×32 | the tracer/beam/muzzle-effect objs, one per view slot (InitWeapon loop < 0x20, each `View_DrawInThisViewOnly`); entry 0 = the laser/taser beam | [M* 0x1a7ef8:79-95, 0x1a5460:87-97, 0x1a65d8:103,139-144, 0x1a4560:40, 0x1a46a8:80,114] | 32 heap ptrs | not modeled |
| +0x86C | ptr | the laser-dot obj | [M1 0x1a7ef8:110; r 0x1a65d8:92] | 0x016c5750 | — |
| +0x874 | ptr | the muzzle-flash obj | [M1 0x1a7ef8:121; r 0x1a3fe0:99-124, 0x1949a8:198] | 0x016c5630 | — |
| +0x878 | ptr | the control-override obj (missile-cam / gun-implant / GT takeover) | [M* w 0x12c5a0:249, 0x132f00:17, 0x181740:16; r 0x18f5d8:192-252, 0x193600:613, 0x1a5460:616, 0x194df0:281-283] | 0 | — |
| +0x87C | ptr | the chase-cam obj | [M1 0x1910f8:73; r 0x18f5d8:221-223] | 0 | — |
| +0x880 | ptr | the auto-aim/lock target obj (`Check_Target` 0x1a7810:107,125; re-zeroed each 6th-frame pass) | [M1] | 0 | — |
| +0x884 | ptr | **the wire/zipline/ladder/creep-wall obj_tag\*** (NOT an index: compared to hitobj+0x58, dereferenced `->0x54` yaw, `->0xE0`) | [M* w 0x1910f8:74, CollCreepWall 0x196758:143, CollWire; r Player_Climb 0x196310:20-22, Player_Wire 0x197658:106-282, Zipline 0x197f50:53, 0x194df0:256,380, 0x1949a8:46] | 0 | `Bond::ladder/wire: Option<usize>` — indices into the port's tables; documented, never an id |
| +0x888 | f32 | the fade/alpha scalar 0..1 (init 1.0; Pickup_Handler zeroes it on respawn): `obj+0x106` alpha = `f ≥ 0.5 ? f·255 : 40` | [M* w 0x1910f8:132, 0x18e650:231; rw 0x193600:92-114] | 1.0 | not modeled (gap) |
| +0x88C | ptr | the BodyGlow obj | [M1 0x1910f8:185; r 0x1ac2c8] | 0x016c4780 | — |
| +0x890 | ptr | the anim-set handle (`AnimSetAppend` return; arg to `AnimSetUpdate` from SSWalk/ZeroG/SSCrouch) | [M* w 0x18fcd8:51, 0x18fea0:51, 0x195b90:6; r 0x192310:53, 0x192ab0:205, 0x1924c0:64] | 0x013851c0 | not modeled |

## 0x894–0x96F: health, armour, and the per-tick state

| Offset | Type | Meaning | Proof | [L] | Port |
|---|---|---|---|---|---|
| +0x894 | f32 | health 0..100 (SP init 100; `Player_SetHealth` 0x1960b8 clamps <0→0 and logs `PlrStat_LogHealth`) | [M* w 0x1960b8:13, HandlePain 0x1902c8:100-134, RamLoad→SetHealth; r HUD 0x19b9c8:159, CheckForDeath 0x190810:23,34, DealWithObjHit 0x194458:161-185, Check_AutoAim 0x1a73f8:103, RamSave 0x195670:16] | 100.0 | `PlayerHealth::health` ✓ |
| +0x898 | u32 | the zoom-sound latch (0 init; rw `Player_Zoom` 0x1a1a70:27-83, `Player_Weapon` 0x1a24c8:163-165) | [M1] | 0 | gap |
| +0x89C | u32 | the underwater-loop sound handle (`Player_MonitorAir` 0x191a70:105-123 only) | [M1] | 0 | gap (audio) |
| +0x8A0 / +0x8A4 | f32 | the climb's top / bottom dismount thresholds (`ladder->0x84 + 0x8C + 0.85` / `centre.y − radius + 1.0`) | [M* w Player_CollLadder 0x196118:23-25, Player_Climb 0x196310:51-53; r 0x196118:31] | 0 | `Bond::climb_top/bottom` ✓ |
| +0x8A8 | f32 | the view pitch, units of π/2, clamped ±1 (`Player_ViewClamping` 0x1a92e8:9-13); lerp target +0x8AC | [M* w Player_Aiming 0x1a1d28:192,276, cine cam 0x1ba1c8:105, init 0x1910f8:51; r 0x1a3b40:47, 0x191dd0:151, 0x1bc048:25, 0x12f678:92-98] | 0.0 | `Bond::pitch` ✓ |
| +0x8AC | f32 | the pitch lerp target (walk/crouch zero it) | [M1 0x192310:57, 0x1924c0:72; r 0x193600:264] | 0 | gap (the port applies pitch directly) |
| +0x8B0 | f32 | armour 0..50: the pain-type switch decides whether it absorbs (below) | [M* w HandlePain 0x1902c8:104, RamLoad 0x1957a8:36, =0 Init 0x1910f8:125 / InitWeapon 0x1a7ef8:180 / MP_ReSpawn 0x185368:69, =50 InitWeapon 0x1a7ef8:316 / Pickup_Handler 0x18e650:224 / cheat 0x215dc8:31; r HUD 0x19b9c8:93,114, 0x19b630:84-90] | 0.0 | `PlayerHealth::armour` ✓ |
| +0x8B4 | u32 | zeroed only (Init 0x1910f8:126, MP_ReSpawn 0x185368:70) — purpose unknown | [M1] | 0 | — |
| +0x8B8 | f32 | the health mirror at spawn (the respawn floor) | [M1 w 0x1910f8:140, 0x185368:77] | 100.0 | gap |
| +0x8BC | f32 | the pain/red-overlay intensity: 1.0 on any pain, decays in `Player_Update` 0x193600:414-428; ×4 into the health pane's alpha | [M* w 0x1902c8:27, 0x1910f8:127, 0x18e650:225; rw 0x19b9c8:118-122] | 1.0 | gap (the flash byte is ported, this f32 is not) |
| +0x8C0 | f32 | the air meter 0..100 (−5/s head under, +20/s surfaced, clamp) | [M* Player_MonitorAir 0x191a70:100-134; r HUD air pane 0x1a11c8:24] | 100.0 | `Bond::air` ✓ |
| +0x8C4 | f32 | the underwater fade 0..1 (→1 under, →0 surfaced) | [M1 0x191a70:61-87] | 0 | `Bond::water_fade` declared; **no nf-view consumer (gap)** |
| +0x8C8 | u32 | the water flags (0x10 = head under the surface; 0x1 = the was-under edge) | [M1 0x191a70:73-97; r 0x1a11c8:26] | 0 | `Bond::underwater` (bit 0x10) ✓ |
| +0x8CC | f32 | the wire auto-drop progress → 100 (+= 100/(FRAME_RATE·30)/tick; paused in the grab window) | [M* w 0x197658:147,157, =0 0x197130:151; r 0x197658:166, 0x1a11c8:21] | 0 | `Bond::auto_drop` ✓ |
| +0x8D0 | f32 | the current zoom factor (1 = none): `Player_Zoom` writes it; ÷ into the turn speeds and the viewer FOV | [M* w 0x1a1a70:57,67, =1 0x1a9858:16, init 0x1910f8:60; r 0x1a1d28:186-274, 0x192ab0:80-82, 0x18f5d8:129, 0x19d770:98-101] | 1.0 | **gap** (the port zooms the FOV without the factor) |
| +0x8D4 | f32 | the zoom target (per-weapon, persisted to the record lane 0) | [M1 0x1a2e38:205,320-331, 0x1a24c8:161-226] | 1.0 | gap |
| +0x8D8 | f32 | the weapon's max-zoom clamp | [M1 0x1a2e38:329, 0x1a24c8:180,225] | 1.0 | gap |
| +0x8DC / +0x8E0 / +0x8E4 / +0x8E8 | f32 ×4 | the view-model anim phases: +0x8DC recoil-kick (+= BL+0x28 per shot), +0x8E0/+0x8E4 the recoil offsets X/Y (decay ×0.75; +0x8E4 feeds the gun rot), +0x8E8 the idle-sway (+= (π/2)/FRAME_RATE) | [M* Player_WeaponRecoil 0x1a3de8:34-57; =0 InitWeapon 0x1a7ef8:68-72] | 0 / 0 / 0 / 0.0785 | the port bobs in `scene.rs` (its own phase accumulators) |
| +0x8EC..0x900 | f32 ×4 | the turn-speed ramps: +0x8EC/+0x8F0 scope-mode X/Y, +0x8F4/+0x8F8 aim-mode X/Y, +0x8FC/+0x900 no-aim X/Y (`AccelFunc0` outputs) | [M* Player_Aiming 0x1a1d28:146,181,234-269, Player_Move 0x191dd0:145-147; init 0x1910f8:53-58] | 0.04 / 0.01 at +0x8FC/+0x900 | gap (the port ramps differently) |
| +0x904 | f32 | the ladder yaw copy | [M1 0x196118:42] | 0 | — |
| +0x908 | f32 | the eye height (0.7; smoothed ×0.1 by `Player_PositionCamera`) | [M* w 0x18f5d8:117-128; r Player_GetHeadPos 0x195aa0:25,49] | 0.7 | `Bond::eye` / EYE ✓ |
| +0x90C | f32 | the crouch eye drop (rate 0.0225/tick; init 0.45 if the level starts crouched) | [M* 0x18f5d8:73-131, 0x1910f8:175; r 0x195aa0:25,49, 0x1a2e38:573-581] | 0 | `Bond::crouch_drop` ✓ |
| +0x910 | f32 | the bullet hit-push (±0.1 per hit, decays ×0.9/tick, added to the move vec Y) | [M* w DealWithObjHit 0x194458:239,246,253; rw 0x193600:125,300; =0 0x190d88] | 0 | gap |
| +0x914 | f32 | the shadow darkness (byte/255, halved in shade) | [M1 Player_InShadow 0x193498:75-77] | 0.7255 | gap |
| +0x918 / +0x91C | f32 | the flash-bang brightness / duration countdown (`Player_SetFlashBang` 0x196040:11-12; decay in Update 0x193600:322-352 → `Camera_SetFade`) | [M1] | 0 / 0 | gap |
| +0x920 | f32 | the night-sight/XRay battery (max 1800.0, drain 1/t, refill 3/t) | [M* w 0x19e9d8:156-172, =1800 0x190d88; r 0x19d218:31-34, 0x1a1410:21] | 1800.0 | gap |
| +0x924 | f32 | the footstep distance accumulator (+= \|AnimObj+0xD8\|, wraps mod the stride; =1000 on a land thud) | [M* AnimObjectUpdate 0x1befd8:163-174, HandleJump 0x1916b0:201] | 0 | `scene.rs::step_dist` ✓ |
| +0x928 | f32 | the fall timer (> 60 on landing → pain type 6, damage = timer − 60 if ≥ 10) | [M* w 0x1949a8:54,68,71,95,99,102,117-126; =0 0x132c18:43, 0x190d88] | 0 | `Bond::fall_timer` ✓ |
| +0x92C | f32 | the refire delay (first shot 1.0; then max(def+0x40, 1.0) from weapon_data+0x40; −FRAME_RATE_MUL/t) | [M* w 0x1a5460:460,622,630,662; =0 0x1a2e38:316,541, 0x1a7ef8:74] | 0 | `Gun::timer` ✓ |
| +0x930 / +0x932 | s16 | the last gun-class / gadget-class weapon (the cycle toggle-back; 0x55→0x54 fixup) | [M* Player_WeaponChange 0x1a6f80:69-140, 0x1a17f8:55] | 0 / 0 | `Gun::prev_weapon/prev_gadget` ✓ |
| +0x934 | u16 | the saved weapon id (the RamLoad target; low byte → AnimObj+99) | [M* w RamLoad 0x1957a8:37; r 0x1a7ef8:159-306] | 0 | the port's `restore` return ✓ |
| +0x936 | s16 | the shots left in this trigger pull | [M1 0x1a5460 (fire loop), =0 0x1a7ef8] | 0 | `Gun::pending` ✓ (semantics) |
| +0x938 | s16 | the muzzle-flash view timer | [M1 w 0x1a4298:86, =0 0x1a9858:17; rw 0x1949a8:192-201] | 0 | gap |
| +0x93A | s16 | the idle-fidget counter (≥ FRAME_RATE_INT·0x14 → play the idle anim) | [M1 0x1a2e38:42-54,83,190; r 0x1a5460:264,314] | 0 | gap |
| +0x93C | s16 | the random ambient countdown ((Rand&0x3FF)+0xFF) | [M1 0x193600:169,330,355-356] | 992 | gap |
| +0x93E | s16 | the weapon-raise cooldown (= FRAME_RATE_INT·0x14 = 1200) | [M1 0x1a2e38:47-57,93,191] | 1200 | gap |
| +0x940 | u16 | the grounded history (>>1 per tick, bit 3 = grounded) | [M* w 0x1949a8:78-81; r 0x1949a8:97, 0x1916b0:91,98] | 0x000C | `Bond::grounded_history` ✓ |
| +0x942 | u16 | the saved substate (the dispatch case 10's restore target) | [M* w 0x196310:60-94, 0x197f50:58, 0x197658:153, 0x1969d0:137, 0x12c5a0:251, 0x132f00:18, 0x181740:17; r 0x193600:254, 0x191dd0:89] | 0xFFFF | `Bond::saved_sub` ✓ |
| +0x944 | s16 | the missile-cam/scan-mode return countdown (case 10 decrements → restores +0x942) | [M1 w 0x12c5a0:253, 0x12e660:28; rw 0x193600:243-249] | 0 | gap (substate 10 unported) |
| +0x946 | u16 | the pause-menu handle (0xFFFF init) | [M1 0x193600:76, 0x1910f8:141] | 0xFFFF | gap |
| +0x948 | s16 | the bond-moment health-cross pulse (200 on `PlrStat_DoneBondMoment`; sine 100..200; −1/t) | [M* w 0x199448:25; rw 0x19b9c8:232-259] | 0 | gap |
| +0x94A | u8 | **the freeze gate**: 0 frozen (`Player_Disable` 0x195f70:16), 1 run, >1 countdown (`Player_Update` 0x193600:80-87; `Player_Enable` writes 2) | [M*] | 1 | gap (the port freezes via `Sim::disabled` out-of-band) |
| +0x94B | u8 | the cine/script lock (skips the substate dispatch, 0x193600:170) | [M1 w 0x195f70:13; =0 0x195498, 0x1910f8:151] | 0 | gap |
| +0x94C | u8 | the jump state: 0 ground, 1 rising, 2 bumped/falling | [M* w HandleJump 0x1916b0:69,181,193; r 0x1916b0:64, 0x1949a8:169, 0x194df0:120] | 0 | `Bond::jump` ✓ |
| +0x94D | u8 | the move-mode enum (0 foot, 1 ladder, 6/0xC wire, 9 zipline, 0 creep-reset) | [M* w 0x196118:45, 0x197658:288, 0x197f50:111, 0x196758; r 0x197658:146,186,268-271, 0x1ba5e0:124] | 0 | `Bond::mode` ✓ |
| +0x94E / +0x94F | u8 | the player slot (index into glb_players/glb_viewer/inputs) / its mirror (no reader) | [M1 0x1910f8:38,39] | 0 | implicit (single Sim slot) |
| +0x950 | u8 | the camera mode (`Player_SetCamMode` 0x195928:11; enables HUD panes 9/10/0x12) | [M1] | 0 | gap |
| +0x951 | u8 | the regrab/input lockout in ticks (ladder 60, creep 0x78, wire 0x1E, water f·5; Update decrements 0x193600:135-137) | [M*] | 0 | `Bond::regrab` ✓ |
| +0x952 | u8 | the landing delay (4 on jump) | [M1 0x1916b0:73-74,178] | 0 | `Bond::land_delay` ✓ |
| +0x953 | u8 | a countdown decremented in Update (0x193600:138-139); no nonzero writer found | [M1] | 0 | — |
| +0x954 | u8 | the walk-anim reset flag (0xFF written by 0x195a88/0x190630); no reader found | [M1] | 0xFF | — |
| +0x955 / +0x956 | u8 | init-0-only writes (0x1910f8:59, 0x1a7ef8:76) — unused? | [M1] | 0 | — |
| +0x957 | u8 | the pitch-lerp hold counter (Update decrements/gates 0x193600:263-280); no nonzero writer found | [M1] | 0 | — |
| +0x958 | u8 | the fire-anim alternation toggle (`1 − x` per shot; picks the +0xAC/+0xB0 anims) — NOT a switch state | [M* w 0x1a3c70:41-42, 0x1a5460:427-428,487-488; =0 0x1a2e38:370; r 0x1a4298:78, 0x1a3fe0:77, 0x12f678:96] | 0 | `Gun::alt_fire_toggle` ✓ |
| +0x959 | u8 | the view-anim phase (0 idle, 1 up, 3 raising — `Player_SetWeaponAnimObj`'s own state) | [M1 0x1a2e38:37,53,61,72,85,125,189,482,559,566] | 0 | `Gun::switch_state` ✓ |
| +0x95A | u8 | =0 on substate change only (0x190630:79,88) | [M1] | 0 | — |
| +0x95B | u8 | the muzzle-flash pending flag (`Player_WeaponInitBullet` 0x1a4bf0:188,203) | [M1] | 0 | gap |
| +0x95C | u8 | the bullet-spawned latch per pull | [M1 0x1a2e38:417-418,517-518, 0x1a5460:689,693] | 0 | `Gun::latch` ✓ |
| +0x95E | u8 | **the ammo-pane switch timer = (char)FRAME_RATE (60)**, written by **`HUD_UpdateAmmoPane` 0x19a7f0:216** — condition `AnimObj[99] != AnimObj[0x62] && weapon_data[req].+2 != weapon_data[drawn].+2` — then −1 (:223); =0 on `Player_StandAtNewPosition` 0x190d88:85. While > 0 the name shows and the ammo text hides | [M*] | 28 | `hud.rs::weapon_label_timer` ✓ (the writer is the HUD, not the weapon code) |
| +0x95F | u8 | the context/action icon id (0xFF none; `ThirdIcon` 0x196e80:52; gates the rocket 0x1a5460:523, the jump-wire grab 0x1916b0:118, `Player_Activate` 0x1a5170:67) | [M*] | 0xFF | gap |
| +0x960 | u8 | the crouch lockout (=60 on the stance transition) | [M* w ChangeSubState 0x190630:36,46; rw SSWalk 0x192310:34-50, SSCrouch 0x1924c0:45-60] | 0 | `Bond::lockout` ✓ |
| +0x961 | u8 | the night-sight active latch | [M1 0x19e9d8:67,82] | 0 | gap |
| +0x962 | u8 | the lens-flare active flag | [M1 0x19d770:64,90,103] | 0 | gap |
| +0x963 | u8 | the blur amount (init 0xFF; → `GlobalBlur`/`Camera_SetFade`) | [M1 0x1910f8:142; r 0x193600:334,343,349] | 0xFF | gap |
| +0x964 | u8 | the laser pointer on/off (the anim-script command toggles, 0x1c1b38:377) | [M1 0x1a9338:22,25; r 0x1a65d8:93] | 0 | gap |
| +0x965 | u8 | the weapon-hold anim variant (skin/level derived 0..3) | [M1 w 0x1910f8:114; r 0x1a9338:19] | 0 | — |
| +0x966 | u8 | the land-footstep sound delay (8) | [M1 w 0x1916b0:200; dec 0x1befd8:168-169] | 0 | — |
| +0x967 | u8 | the hit-direction bits: `Player_HandlePain` 0x1902c8:121-125 writes 0xC (2 for pain type 6); `Player_DealWithObjHit` 0x194458:199-232 the directional 1/2/4/8 | [M*; r HUD_UpdateHealthPane 0x19b9c8:92-113] | 0 | `PlayerHealth::dir_bits` ✓ |
| +0x968 | u8 | the hit-flash alpha (+= min(d·21.333, 128) cap 255 on pain; −FRAME_RATE_MUL in the health pane) | [M* 0x1902c8:126-130; rw 0x19b9c8:89-113,123-126] | 0 | `PlayerHealth::flash` ✓ |
| +0x96A..0x96F | — | no references | repo-wide rg | 0 | the struct ends at 0x970 |

## The health & armour arithmetic (`Player_HandlePain` 0x1902c8)

1. Early-outs: switch channel 0x5F set; health ≤ 0; damage ≤ 0; MP + GameFlow ≠ 2 [M :14-26].
2. `BL+0x8BC = 1.0` (the pain overlay) [M :27].
3. The difficulty factor (SP): `Plr_DMod_Easy/Normal/Hard` by `GameState+0x40` 1/other/3; state 4 → ×2 [M :28-49].
   MP: limb multipliers from MPSettings [M :51-82]. Port: `bullet::damage_factor` ✓.
4. **The armour gate** [M :84-104]: the pain-type switch sets the armour fraction — **default/type 0 → 1.0;
   types 1..7 → 0.0** (the MP kill-feed writes in the 5/6/7 arms fall through to 0.0). absorbed = min(d·frac, armour);
   armour −= absorbed; health −= d − absorbed. Falls are type 6 [M 0x1949a8:120], drowning type 7 [M 0x191a70:142],
   bullets type 0 [M 0x194458:195-197]. **Port fix (fields-player)**: `PlayerHealth::pain_generic` now bypasses
   the armour for types 1..7 (fail-first test `armour_bypass_by_pain_type`), and the Sim's fall/drown paths route
   through it (fail-first `fall_damage_runs_the_full_pain_pipeline`).
5. With a cheat on, the health subtraction is skipped entirely (the armour still drains) [M :106-109] — unported.
6. `BL+0x967 = 0xC` (2 for type 6); `BL+0x968 += min(d·21.333, 128)` cap 255 [M :121-130]. Port ✓ for both.
7. health < 1.0 → 0.0 → `Player_SetHealth` [M :131-134].

## The RamSave anchors (`Player_RamSave` 0x195670 / `Player_RamLoad` 0x1957a8; base 0x2b9440)

| RamSave | source | len | | RamSave | source | len |
|---|---|---|---|---|---|---|
| +0x000 | 0x2bcc20 PlrMissionStats | 0x38 | | +0x5FC | **BL+0x894 health** | 4 |
| +0x038 | **BL+0x170 ammo store** | 0x42 | | +0x600 | **BL+0x8B0 armour** | 4 |
| +0x07C | **BL+0x1B4 weapon records** | 0x558 | | +0xA1A | **BL+0x70C key ring** | 0x33 |
| +0x5D4 | 0x2befd0 SSys | 0x28 | | +0xA18 | glb_players[slot]->0xDC+99 (the weapon byte; RamLoad → **BL+0x934**) | 2 |
| +0x604 | 0x270190 switch channels | 0x400 | | +0xA04 / +0xA4D… | BondMoments 0x14 / the channel lists 3×0x100 | |

[M 0x195670:6-20; the contiguity proves 0x170+0x42 = 0x1B2 (a 2-byte hole at 0x1B2), 0x1B4+0x558 = 0x70C,
0x70C+0x33 = 0x73F.] Nothing in 0x7E0..0x970 is copied en-bloc. Port: `player.rs::RamSave` capture/restore ✓.

## Disagreements (written down, not resolved silently)

- **+0x44**: the decompile's `Check_Target` "+0x44 != 1" hit is the TARGET's (drone) instance field, not
  the player's — the player's own +0x40..0x4C is the ClearInertia vec4. Two different structs share the offset.
- **+0x958**: named "switch state" in an earlier pass; it is the fire-anim ALTERNATION toggle. The switch
  phase is +0x959 and the latch is +0x95C. The port's `Gun` fields already match the corrected meanings.
- **+0x95E** is written by `HUD_UpdateAmmoPane`, not by the weapon-switch code (sprite.md's input table says
  this too).
- **+0x884** is an obj_tag POINTER; the port stores table indices — equivalent for the port's own tables,
  never a small integer id in the game's terms.
- **+0x60..0x6C**: the only writer found is the debug scan-mode branch, but the snapshot holds (−109.85,
  14.89, −30.6) at level start — a pre-debug writer was not located (honesty).
- **The armour**: types 1..7 bypass it (now ported); the CHEAT skip of the health subtraction is unported.

## Live checks for the main agent

1. **Armour bypass**: in 07000005 give Bond the armour pickup (or `poke` BL+0x8B0 = 50 at 0x016cf0f0+0x8B0),
   step off the castle stairs to trigger a fall (`watch 0x016cf984 4 readwrite` = BL+0x894 — it should drop
   WITHOUT BL+0x8B4+0xFC... precisely BL+0x8B0 at 0x016cf990 staying 50.0). The pre-fix port drained the armour.
2. **Grenade-belt regen**: `watch 0x016cf8e4 2` (= BL+0x4F4) for 2 s of play on 07000005 after throwing a
   grenade — the value should climb +1 per 6 frames toward 100. (The snapshot starts it at 0.)
3. **Switch timer writer**: `gdbwatch 0x016cf0f0+0x95e 1 write --seconds 4` while switching weapons — the
   expected writer is `HUD_UpdateAmmoPane` (0x19a7f0+0x…), NOT the weapon-switch state machine.

## The census to run

`gdbwatch <BL> 0x970 readwrite --seconds 8` on 07000005 during a walk-switch-shoot pass (BL = 0x016cf0f0),
which should surface exactly the accessors in the rows above; any extra accessor means this map missed a
reader. Priority ranges: 0x118-0x134 (the crosshair block — Check_AutoAim/Check_Target/Aiming), 0x92C-0x960
(the fire/switch block) and 0x4F4/0x530/0x548/0x560/0x56C/0x41C (the regen slots).
