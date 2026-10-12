# The AnimObject (`obj+0xDC`, 0x120 bytes) and its weapon / LOD-fade bytes

Proof levels: see [README.md](README.md). Live session: EE snapshots
`work/ee_start_05.bin` (07000005 at start), `ee_start_07000001.bin`, `ee_start_0700000a.bin` (file offset == EE address).

## What it is

`AnimObjectNew(obj, skin_hashcode)` (0x1be910) `Mem_Malloc(0x120)`s one block, zeroes it, and stores it at
**`obj+0xDC`** [M AnimObjectNew 0x1be910:48-52]. Ghidra's type `sAnimObject_tag` is the **sub-block at
+0x70**: every function whose signature says `P15sAnimObject_tag` receives `*(obj+0xDC) + 0x70`
(e.g. AnimObjectUpdate 0x1befd8:105 passes `anim+0x70` to AnimScriptTick/AnimFrameResolve/psiBuildMatrixPalette).
So the 0x120 block = a small base (0x00–0x6F: weapon bytes, event word) + the `sAnimObject_tag` the anim engine
passes around (0x70–0x11F). Created by: `Player_Init` 0x1910f8, `NDrone2_DefaultInit` 0x14b300, `Sub_Create`,
`MiniSub_Create`, `Env_Tree_Create`, `Script_AnimStart` (cut-scene doubles), `AnimDebugCreate/Handler`
[ callers of 0x1be910 ]. Deleted by `AnimObjectDelete` 0x1bee80 (frees +0xA4/+0xAC/+0xB0/+0xB4/+0xB8/+0xBC/+0xC0, then the block).

**obj+0xDC vs obj+0xE0** (for the StateMachineInfo worker): no union, no overlap — two different allocations.
+0xDC is the AnimObject for *every* anim class. +0xE0 is the per-class state block: player → `BLData`
(AnimObjectUpdate 0x1befd8:161 writes `BL+0x924`), drone → `DCVars` (NDrone2_PostLoad_Init 0x14d890:
`NDrone2_DefaultInit(*(obj+0xE0)+0xC00)`, `Drone_SM_InitObject` writes DCVars+0x108…0x130), the weapon-model
attachments created by AnimObjectNew → parent `obj_tag*` (0x1be910:163 `*(att+0xE0) = parent`). Attachments
have +0xDC = 0.

`obj+0xFC` (seen by the drone doc) is the viewer-seen counter: `View_AddForcedObjects` 0x1e5fb8:174 sets it to 2
when the object enters a draw list; `control_movement_object_handler` 0x1336f8 decrements it.

## How to reach one live

`PLAYERS` 0x2D88E0 → player obj → `obj+0xDC` = AnimObject. [L-read ee_start_05.bin: obj 016cefe0, anim
016cb8a0]. Drone 0 of the list at 0x298E14 (`drone+0xC` = obj): [L-read: obj 016ed940, anim 01fbd470].
07000001: player anim 018f2050, drone anim 0190b670. 0700000A: player anim 01b74f00 (weapon 0x43).

## The base block (0x00–0x6F)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00–0x4F | ? | not touched by any `obj+0xDC` reader found (the block may be copied as a whole, see disagreements) | [M census: only 5 files touch `anim+0x00..0x4F`, all at +0x50] |
| +0x50 | u8 | movement action state; low 6 bits: 0x0D/0x0E = airborne (jump/fall) while Player_HandleJump gates on `- 0xd < 2`; cleared on landing/feet-on-point | [M1 Player_HandleJump 0x1916b0; Player_CollisionHandler 0x1949a8 and Drone_FeetOnPoint 0x140660 write 0; Bullet_init_casing_ex 0x12f678 masks 0x3F; Player_DealWithObjHit 0x194458] |
| +0x60 | u16 | **animation event flags for this frame**. bit 0x1 = special weapon anim playing (set by Player_SetWeaponAnimObj 0x1a2e38, cleared by Player_WeaponNone 0x1a9858; gates firing/zoom/move all over the player code); bit 0x4 = footstep this frame (cleared every `AnimObjectUpdate` 0x1befd8:157, set by the walk-cycle accumulator :175 or `ScriptFootstepType` :186; read by NDrone2_DealWithObjHit 0x13fff0); bit 0x8 = feet-on-ground (set by Player_CollisionHandler 0x1949a8, read by Player_Update/HandleJump/SSGrapple, NDrone2_DSTATE_AbseilDeath 0x169b28); bit 0x10 = ? (Player_Collision 0x194df0, NDrone2_DealWithObjHit); bit 0x20 = climbing (set Player_CollisionHandler, read Player_Climb 0x196310 / Player_SSCrouch 0x1924c0 / Player_HandleJump); bit 0x80 = footstep on a "foot point" (Player_FeetOnPoint 0x194188, Drone_FeetOnPoint 0x140660) | [M* per bit, lists above; bit 0x2 unwitnessed] |
| +0x62 | u8 | **held weapon id** (index into `weapon_data`, 0x10C-byte records). The id the held model, muzzle flash, auto-aim, cross-hair, reload, drone AI and the player anim set all key off. Drones: `NDrone2_DefaultInit` stores placement parameter 9 here (1 when 0, 0 for skin 0x0500001E). 0 = nothing (drop), 0x47 = unarmed/fists, 0x50/0x51 = grapple pair (`- 0x50 < 2` in Player_Move/GrappleSetRope/Check_AutoAim), 0x52/0x53 = no-ammo / turret (`GT_LoseControl` puts the player on 0x53), 0x54/0x55 = sniper scope states, 0x18 = the drone's put-away/idle-aim weapon (DSTATE_AimStand/-Fire/Attack set it), 0x1C = anim debug. | **[M*]** writers: NDrone2_DefaultInit 0x14b300, DroneWeap_DropWeapon 0x17a2a0 (→0), NDrone2_DSTATE_AimStand 0x166178 / AimStandFire 0x166328 / Attack 0x164210 (→0x18), AnimDebugHandler 0x1bc6e8 (→0x1C), Player_WeaponSelect 0x1a2cc0:20/34 (← +0x63, the applier), Player_Weapon 0x1a24c8:125 (→0x54), GT_LoseControl 0x181408 (→0x53), MP_EquipPlayer 0x1855f8, Player_InitWeapon 0x1a7ef8, Player_WeaponNone 0x1a9858 (→0x47), HUD_UpdateCrossHair 0x19e6d0. Readers (weapon_data[w·0x10C]): Player_PositionCamera 0x18f5d8, Check_AutoAim 0x1a73f8, PlayerAnimCrouch2Stand 0x190068, Player_MuzzleFlash 0x1a4298, NDrone2_DSTATE_Interogate 0x161a40, BOTWEAP_decrRounds 0x12a470 (==0x45 special-case), Bullet_init_casing_ex 0x12f678, Player_WeaponChange 0x1a6f80, Player_Zoom 0x1a1a70, Player_WeaponFiring 0x1a5460, DroneFunc_AttackNoWeapon 0x14af80, NDrone2_DSTATE_DonePressAlarm 0x1737d0, Player_SetFiringAnim 0x1a3c70, Player_RoundToFire 0x1a18e0, Player_SetWeaponAnim 0x1a9338, Player_Weapon 0x1a24c8, NDrone2_ObjectIsArmed 0x178768 (armed = 2..=0x46 except 0x47, or 0x6A..0x71), Player_Move 0x191dd0, Player_GrappleSetRope 0x192728, Player_Update 0x193600, Player_Climb 0x196310 (== 0x47 dismount carry), PlayerAnimSetInitNormal 0x18fcd8 / Crouch 0x18fea0, AnimObjectDraw 0x1c4490:267 (== 0x18 draws the laser-sight ray), Player_ReloadAmmoType 0x1a1988, Draw_MuzzleFlash 0x1a3fe0, MP_BluePrintReachedBase 0x188970, Player_Aiming 0x1a1d28, Player_HandleHasNoAmmo 0x1a2ba8, AnimObjectAimAt 0x1bfe58 (MP: weapon_data[w]+0x84 == 0 → no aim), Player_WeaponSelect, AnimObjectSetSleeveType-family callers. **[L-read]**: 07000005 player 0x02 (P2K), drones 0x16; 07000001 player 0x02 / drone 0x0E; 0700000A player 0x43 |
| +0x63 | u8 | **selected / queued weapon**: the id a switch will land on. Written by the cycle/prev logic (Player_WeaponChange 0x1a6f80 +0x63 = BLData+0x930/+0x932 candidate, then resolved through `weapon_data[+2]` base-weapon chain), Player_InitWeapon (start weapon), Player_HandleHasNoAmmo 0x1a2ba8 (0x52/0x53/derived), Player_Weapon (0x54 scope state; `Upgrade_Weapon(6)` result), GunImp_Deactivate 0x132c18 (= +0x64, restore after the gun implant), Player_Climb 0x196310 (= +0x64), Player_WeaponNone (0x47). **`Player_WeaponSelect` 0x1a2cc0 is the applier**: when the view-model skins match it writes +0x62 = +0x63 outright, else it queues and writes +0x62 after the draw anim (BLData+0x7E8 view-model state +0xF4 = 1/3/8). | [M1 each; applier [M] 0x1a2cc0] **[L-read]**: 07000005 player +0x63 = 0x02 (== current after the switch), +0x64 = 0x47 (was unarmed) |
| +0x64 | u8 | **previous weapon** (the id held before the current one): written by Player_WeaponChange 0x1a6f80:142 (`+0x64 = +0x62`) and Player_InitWeapon (0x47); read by GunImp_Deactivate and Player_Climb (both `+0x63 = +0x64`). | [M1 each] **[L-read]**: 07000005 player = 0x47 |
| +0x65–0x6F | — | untouched | [M census] |

## The `sAnimObject_tag` block (+0x70; Ghidra's `P15sAnimObject_tag` = this pointer)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x70 | ptr | head-track **target object** (`obj_tag*`); 0 = none (SetHeadTracking clears) | [M AnimObjectSetHeadTracking 0x1c6748:35 (`+0x70 = param`), AnimObjectHeadTrack 0x1bf618:164 (`*param_2 = tag+0` → target's anim)] |
| +0x74 | ptr | aim-at **target object** (`obj_tag*`), used when flags & 0x200; also the aim speed float lands in +0xE4 (see below) | [M AnimObjectSetAiming 0x1c6878:20/23 (`+0x74 = param`), AnimObjectAimAt 0x1bfe58:211/218 (target's anim+0xB8 matrix at bone +0x105)] |
| +0x90 | fn ptr | **pre-trans root-motion callback** `(obj, tag, VECTOR* root_translation)` — may rewrite or zero the root motion; drones install `DroneAnim_PreTransCallback` (0x13b490); players leave 0 | [M NDrone2_DefaultInit 0x14b300 (`anim+0x90 = DroneAnim_PreTransCallback`); caller AnimFrameResolve 0x1c35f8:425-432 applies the result to obj+0x30] **[L-read]** drone 0x13b490, player 0 |
| +0x94 | ptr | the **skin resource** (`sAnimSkin_tag` from `hashtable_getitem(skin_hashcode)`): +0x04 glist array, +0x0C attachment bone array, +0x10/+0x14/+0x18 skin scale xyz, +0x1C word, +0x24 datum (f32), +0x24 attachments' transform table (quaternion+vec, 0x24 stride), +0x28 bind matrix table, +0x2C glist count, +0x2D attachment count, +0x2E facial-morph count, +0x2F extra-attachment count, +0x30 skeleton index | [M AnimObjectNew 0x1be910:56; AnimObjectUpdate 0x1befd8:111-114; AnimObjectDraw 0x1c4490:118,143,236; AnimFrameResolve 0x1c35f8:470; psiBuildMatrixPalette 0x111498:114-115,147] **[L-read]** 07000005 player 01714050, drone 0171afd0 |
| +0x98 | ptr | **anim-set list** head: ≤ 4 `sAnimSet_tag` nodes of 0x34 bytes (walk anim sets; node+0 prev, +4 next, +8/+C/+10/+14 script ptrs, +18 script-table, +20 hashcode, +24 float 0.37, +28 float 0.5, +2C u16, +2E/+2F u8, +30 blend countdown, +31 count, +33 delete flag); players only | [M AnimSetAppend 0x1c07b8:20-34 (creates at anim+0x98), AnimSetInit 0x1c6a48, AnimSetUpdate 0x1c0938] **[L-read]** player 013851c0, drone 0 |
| +0x9C | ptr | **playing-script list head** (`sAnimScript_tag`; next at script+0x48); walked every frame by AnimObjectUpdate 0x1befd8:128-156 and by AnimListBuild 0x1c0fc0:28 | [M] **[L-read]** drone 016ef4f0; player 0 (idle between scripts at snapshot time), 07000001 player 018ebbb0 |
| +0xA0 | fn ptr | post-resolve callback, called by AnimFrameResolve 0x1c35f8:490-498 as `(obj, tag, bone_count, pose_quats, pose_trans)`. **No writer found and both live anims have 0** — cut-scene/setup only? | [M1 reader 0x1c35f8; writer missing] |
| +0xA4 | ptr | per-extra-attachment entity array (u32 per entry, count = skin+0x2F; 0 = skip) | [M AnimObjectNew 0x1be910:75 (malloc count·4), AnimObjectDraw 0x1c4490:337, AnimObjectDelete] |
| +0xA8 | ptr | **sleeve-type celglist** replacement for the first draw glist (`AnimObjectSetSleeveType` 0x1c6610:24; 0 = the skin's own) | [M AnimObjectSetSleeveType; AnimObjectDraw 0x1c4490:235-247] |
| +0xAC | ptr | **pose buffer**, `anim+0x100` bytes = bone_count·32: `[bone_count × 16B translation][bone_count × 16B quaternion]` (psiBuildMatrixPalette reads translations from the front and quaternions from `pose + bones·0x10`) | [M AnimObjectNew 0x1be910:62-66 (malloc bones<<5; +0x100 = bones<<5); AnimFrameResolve 0x1c35f8:168-170; psiBuildMatrixPalette 0x111498:119,131,144-146] **[L-read]** +0x100 = 0x920 ⇒ 0x49 bones (player) |
| +0xB0 | ptr | **facial-morph weight array** (u32 per morph, count = skin+0x2E); copied into the globals `Morph_Weight`/`Morph_Index` and sorted by AnimObjectDraw | [M AnimObjectNew 0x1be910:69-71 (malloc skin+0x2E·4), AnimObjectDraw 0x1c4490:143-155] |
| +0xB4 | ptr | per-extra-attachment **life bytes** (count = skin+0x2F): 0 = not drawn, 0xFF = always drawn, N>0 = drawn and decremented per draw | [M AnimObjectNew 0x1be910:77-90 (zeroed), AnimObjectDraw 0x1c4490:328-335] |
| +0xB8 | ptr | **bone world-matrix palette** (4×4 f32 per bone, 0x40 stride, built by psiBuildMatrixPalette); read for attachment placement, blood effects (Effect_Player 0x1cec48 +0xCC0/+0xDC0), cross-object aim/head-track targets | [M AnimObjectNew 0x1be910:93-94 (malloc bones·64); psiBuildMatrixPalette 0x111498:112 (param_2+0x48); AnimObjectUpdate 0x1befd8:313] |
| +0xBC | ptr | **skinning matrix palette** (0x40 per bone) handed to `psiDrawSkinObjectMatrix` and `AnimGetBoneWorldTrans2`; allocated next to +0xB8, written by the helper psiBuildMatrixPalette calls with a 13-word stack block (Ghidra dropped the call) — writer not visible in the decompile | [M1 AnimObjectNew 0x1be910:96-97; readers AnimObjectDraw 0x1c4490:249, psiDrawSkinObjectMatrix 0x111828:163, psiBuildMatrixPalette 0x111498:127, AnimGetBoneWorldTrans2 0x1c32e0] |
| +0xC0 | ptr | **attachment object array** (obj_tag\* per attachment, count = skin+0x2D; created by `Control_CreateObjEx` with type from skin+0x0C, +0xFF = 0x3D, +0xE0 = owner) | [M AnimObjectNew 0x1be910:144-169; deleted by AnimObjectDelete 0x1bee80:16-36] **[L-read]** player 0 (Bond's view model has none at start), drone? (see live checks) |
| +0xC4 | u32 | **last animation tick** = `GameState+0x3C` snapshot; AnimObjectDraw skips the draw when `GameState+0x3C - +0xC4 < 2` (updated twice in a frame) then re-stamps | [M AnimObjectUpdate 0x1befd8:115, AnimObjectDraw 0x1c4490:114-115] **[L-read]** 0x1a9fe in both player and drone (same tick) |
| +0xC8 | u32 | **status flags**: 0x01 = a pose has been resolved (gate of AnimObjectDraw and of the DroneWeap_* / NDrone2_GetHeadPosOld "is it posed" checks); 0x02 = big skeleton (≥ 0x47 bones, from skin+0x30's skeleton bone byte in AnimObjectNew); 0x04 = pose dirty → psiBuildMatrixPalette rebuilds and clears it (psiBuildMatrixPalette 0x111498:116-118); 0x08 = attachment matrices written this build (psiBuildMatrixPalette 0x111498:219-221); 0x10 = head tracking on; 0x20 = head tracking settled (HeadTrack clears 0x10|0x20 when the error < 0.01); 0x40 = head-track target is an object (vs the +0x70 pointer being a position?); 0x80 = aiming; 0x100 = aiming active (`AnimObjectIsAiming` = 0x180); 0x200 = aim target is an object; 0x1000/0x2000 = aim yaw/pitch clamped this frame (AnimObjectAimAt); 0x4000 = aim clamp flag; 0x8000 = skin-scale inverse active (Script_AnimStart sets it, AnimObjectUpdate 0x1befd8:116 divides the scale globals); 0x400 = big-skeleton facial variant (AnimObjectNew; picks script 0x06000259 vs 0x0600035E in AnimObjectInitialPose; MP check in AnimFrameResolve 0x1c35f8:377); 0x10000 = facial animation off (AnimFacialUpdate 0x1be310:22) | [M*, per bit: AnimObjectIsHeadTracking 0x1c6850 (0x30), AnimObjectIsAiming 0x1c6900 (0x180), SetAiming 0x1c6878, SetHeadTracking 0x1c6748, HeadTrack 0x1bf618:137-151, AimAt 0x1bfe58:142-164,245,312,382,471, AnimObjectUpdate 0x1befd8:110,291, AnimFrameResolve 0x1c35f8:166,342,364, AnimFacialUpdate 0x1be310:22-23, Script_AnimStart 0x1d8f40, ScriptFootstepNone] **[L-read]** player 0x407 (posed·big·dirty·variant), drone 0x45f (+ head-track 0x10+0x40, attachments 0x8), 07000001 drone 0x402 (created, not yet ticked) |
| +0xCC | f32 | **resolved root Y** (hip height): AnimFrameResolve writes `blend_result + anim+0xD0` (tag+0x5C); read as the drone's hip adjust (DroneAnim_PreTransCallback 0x13b490:59-63 → DCVars+0xA8), NDrone2_FeetPos 0x149b88 (− 0.4), NDrone2_DoAnimation 0x149f88, Player_Collision 0x194df0 | [M*] **[L-read]** player 1.0347 (standing), drone 1.171 |
| +0xD0 | f32 | **root-Y offset constant** computed in AnimObjectNew for big skeletons: `(−1.160398 − skin+0x24-datum/skin+0x14) + 0.02` (−0.995208 base when flags 0x400 clear) | [M1 AnimObjectNew 0x1be910:132-139] **[L-read]** player 0.02388 |
| +0xD4 | f32 | walk-set stride: `AnimSetUpdate` 0x1c0938:96 writes `param_2 · sAnimSet+0x28` (the set's 0.5 float) | [M1] **[L-read]** 0.0 (no walk script at snapshot) |
| +0xD8 | f32 | **stride per tick** fed to the footstep accumulator: `Player_Update` 0x193600 writes it; Player_SSScanMode 0x193098 zeroes it; `AnimObjectUpdate` 0x1befd8:163 adds |  to `BL+0x924` (wraps at 1.65 / 0.4125 crouched → footstep bit 0x4) | [M1 each] **[L-read]** 0.0 standing |
| +0xDC | f32 | walk-set constant 0.37 (0x3ebd70a4): `AnimSetInit` 0x1c6a48:51 stores it, `AnimSetUpdate` 0x1c0938:216 copies it back from sAnimSet+0x24, `Player_InWater` 0x195e50 zeroes it | [M1 each] **[L-read]** player 0x3ebd70a4, drone 0 |
| +0xE0 | f32 | **head-track turn speed** (rad/tick input to `Ang_RotateToBearing`); drones write 0.07 (0x3d8f5c29) from NDrone2_DoHeadTracking 0x1472c8 / NDrone2_HeadTrackObj 0x14ada0, or a computed value; `AnimObjectSetHeadTracking` 0x1c6748:15 writes param | [M*] **[L-read]** drone 0.07 |
| +0xE4 | f32 | **aim turn speed** (AnimObjectSetAiming 0x1c6878:12; AnimObjectAimAt 0x1bfe58:395 rotates +0xF4 toward target with it) | [M2] |
| +0xE8 | f32 | **head-track max distance²** (beyond it no head tracking; AnimObjectHeadTrack 0x1bf618:191; SetHeadTracking writes param²) | [M2] **[L-read]** drone 9.0 (3 m) |
| +0xEC | f32 | **aim max distance²** (AnimObjectAimAt 0x1bfe58:242 skips beyond; SetAiming writes param²) | [M2] |
| +0xF0 | f32 | head-track current **yaw** (Ang_RotateToBearing state; HeadTrack 0x1bf618:315-317) | [M1] |
| +0xF4 | f32 | aim current **yaw** (AnimObjectAimAt 0x1bfe58:400) | [M1] |
| +0xF8 | f32 | head-track current **pitch** (HeadTrack 0x1bf618:316-318) | [M1] |
| +0xFC | f32 | aim current **pitch** (AnimObjectAimAt 0x1bfe58:401) | [M1] |
| +0x100 | u16 | **pose buffer size** = bone_count·32 (set in AnimObjectNew from `pSkeletons[skin+0x30]+2`); memcpy length between pose buffers in AnimFrameResolve | [M2 AnimObjectNew 0x1be910:62; AnimFrameResolve 0x1c35f8:170] **[L-read]** 0x920 |
| +0x102 | u16 | **facial re-trigger timer** (frames): `AnimFacialUpdate` 0x1be310:53 decrements, refills with 60–120 or 90–270 depending on state (alive vs alert chatter) | [M1] **[L-read]** 72, state running |
| +0x104 | u8 | head-track **target bone** on the target object (written by SetHeadTracking when the target-is-object bit 0x40 is set; HeadTrack reads the target's anim+0xB8 palette at this bone) | [M2] **[L-read]** drone = 5 |
| +0x105 | u8 | aim-at **target bone** (SetAiming 0x1c6878:18; AimAt 0x1bfe58:221) | [M2] |
| +0x106 | u8 | head-track **snap delay** (frames): SetHeadTracking writes `rand(15)+6` when param_5, 0 otherwise; HeadTrack 0x1bf618:310-314 decrements and snaps while > 0 | [M2] **[L-read]** drone = 3 |
| +0x107 | u8 | head-track **yaw morph byte** (0–255): HeadTrack writes `Δyaw·255` when the skin has facial morphs (feeds the face bones' pose alongside the morph weights) | [M1] **[L-read]** 0 |
| +0x108 | u8 | head-track **pitch morph byte** (`(1−Δpitch)·255`) | [M1] **[L-read]** 0 |
| +0x109 | u8 | **facial state**: 0 = none, 1 = idle (blink/breathe chatter), 2 = alert-talking (AnimFacialUpdate state machine over `Male/Female_Facial_State_Anims` tables); bit 0x80 = blink sub-flag | [M1 AnimFacialUpdate 0x1be310:28-107] **[L-read]** 1 (both player and drone) |
| +0x10A–0x10F | — | untouched | [M census] |
| +0x110 | u8 | **LOD / fade byte, viewer slot 0** — see next section | |
| +0x111 | u8 | **LOD / fade byte, viewer slot 1** | |
| +0x112 | u8 | **LOD / fade byte, viewer slot 2** | |
| +0x113 | u8 | **LOD / fade byte, viewer slot 3** | |
| +0x114–0x11F | — | untouched (12 bytes of slack; the block is 0x120 because of the allocation) | [M census] |

## The LOD fade bytes (+0x110..+0x113)

One u8 per viewer (up to 4; `viewer+0x28` = the slot index). **Writers** (every one found):

- `control_movement_object_handler` 0x1336f8 — start of each frame's object walk, for every object with an anim,
  while a `GameState` byte (index 0x40) is 0: `memset(anim+0x110, 0xFF, 4)` — "no viewer sees it yet".
- `View_AddForcedObjects` 0x1e5fb8:178-192 — per object in this viewer's view (needs `obj+0xF0 & 0x8000` set,
  `& 0x10` clear, display mask, `obj+0xD8`, `Vision_InView`): slot = `min(viewer+0x28, 3)`, value =
  `clamp((Vec_SqDist3D(obj+0x80, viewer_pos) / viewer+0x108) · 0.25, 0..255)` truncated to u8 (0 near → 255 far),
  and sets `obj+0xFC = 2`.

**Readers** (all take `min(+0x110..+0x113)`, i.e. the closest viewer wins):

- `AnimObjectDraw` 0x1c4490:126-142 — `min ≥ 0xD` (13) and no script camera → skip facial-morph weight collection
  (Morph_Count stays 0).
- `psiDrawSkinObjectMatrix` 0x111828:126-155 — `min > 2.0` → draw the skin without morph weighting
  (facial morphs only visible within LOD bucket 2).
- `AnimFrameResolve` 0x1c35f8:470-488 — `min ≥ 0xD` → skip `AnimFacialBlend` for the frame.
- `AnimObjectHeadTrack` 0x1bf618:117-136 — `min ≥ 0x21` (33) → no head tracking.
- `AnimObjectAimAt` 0x1bfe58:145-161 — `min == 0xFF` → no aim posing (nobody sees it).
- `AnimFrameBlend` 0x1c2b80 — `min ≥ 0xF1` (241) → skip the per-bone blend (copy instead): the far fade-out.
- `AnimObjectIsClose(obj, n)` 0x1c66f8 — `min ≤ n` (ScriptCam forces true); used for the big-skeleton facial
  update gate (n = 0xC) in AnimObjectUpdate 0x1befd8:131-134.

Character alpha fade is **not** here: the death fade-out (`NDrone2_FadeOut` 0x14a218) counts a timer in
`DCVars+0xCFC` into **`obj+0x106`**, one of the four tweak-ARGB bytes `AnimObjectDraw` 0x1c4490:350-352 feeds to
`psiSetTweakARGB` (`obj+0x106` = alpha, `obj+0x103/0x104/0x105` = RGB). Don't confuse the two (the sprite +0x38
lesson).

**[L-read]** in all three start snapshots the four bytes are 0xFF,0xFF,0xFF,0xFF on every anim probed — the
reset state; the snapshots were taken before a viewer pass (or the objects failed the `obj+0xF0 & 0x8000` gate).
Live confirmation of real values pending (below).

## Disagreements / open questions

- **`+0xC8` bit 0x40**: SetHeadTracking sets it with the "target is object" param and HeadTrack only then reads
  `*(+0x70)` as an `obj_tag*`; when clear, +0x70 may still hold something else (a position?) — SetHeadTracking
  writes a param there in both branches (0x1c6748:35 vs :39), so +0x70's non-object meaning is unproven.
- **`+0x00..0x5F`**: `Player_DealWithObjHit` 0x194458 `amemcpy(anim, hit_or_other, 0x60)` copies 0x60 bytes over
  this range in one branch — either the low block doubles as scratch, or Ghidra's argument is wrong. Untested.
- **`+0xA0` post-resolve callback**: reader only (AnimFrameResolve); no writer found; 0 on both live anims.
- **`+0xBC` skinning-matrix writer**: inside the 13-word stack block psiBuildMatrixPalette builds (0x111498:122-139)
  for a call Ghidra dropped. Allocated and read everywhere, write not visible.
- **`+0xD4` vs `+0xD8`**: two stride floats with disjoint writers (AnimSetUpdate for walk sets vs Player_Update);
  whether both feed the same footstep accumulator is untraced (+0xD8's consumer is AnimObjectUpdate:163 only).
- **`GameState` byte gating the 0xFF memset** (control_movement_object_handler 0x1336f8): meaning unknown (pause?).
- **`anim+0x50`**: 0x0D/0x0E = airborne is the only pinned pair; whether drones use the same byte beyond the
  FeetOnPoint zero-write is unproven.
- **Weapon 0x18** (drone AimStand family) vs **0x47** (player fists): both read as "no real weapon" in their
  contexts, but 0x18 is a real entry in the drone anim-set table (set 5) and draws a laser sight in AnimObjectDraw —
  the exact weapon-name mapping lives in weapon-data.md, not here.

## Live checks for the main agent

1. **Pointer chain**: `PLAYERS` 0x2D88E0 → obj → `obj+0xDC` (07000005: 016cefe0 → 016cb8a0; block ends at
   016cb9c0 = +0x120). Drone 0: obj 016ed940 → anim 01fbd470.
2. **Weapon byte**: `watch` anim+0x62 while switching weapons (it should take the +0x63 value at the switch
   moment), then `poke anim+0x62 <other gun id> --shot` on a visible drone — the held model should swap
   (`weapon_data[id]+0x80` third-person hashcode via `AnimDatumSetEntity`).
3. **LOD bytes**: gdbwatch (`readwrite`) anim+0x110..0x113 during play — expect the
   `control_movement_object_handler` memset (0xFF×4) plus `View_AddForcedObjects` byte stores each frame, values
   small near the camera. Then `poke anim+0x110 1 0xFF` ×4 at close range → the drone's facial morphs and head
   tracking should stop; restore 0 → back.
4. **Flags**: `poke anim+0xC8 +1` → bit0 set → the drone should stop being drawn at the next AnimObjectDraw
   (its matrices stop updating too: AnimObjectUpdate keeps running). Restore by clearing bit0.
5. **Root Y**: with a drone walking, `watch` anim+0xCC — it should track the walk cycle's hip height and match
   `drone obj y − 0.02 − 0.4 ≈ feet` per NDrone2_FeetPos.
6. **Tick**: confirm `anim+0xC4 == GameState+0x3C` right after a frame (both were 0x1a9fe in ee_start_05).
7. **Facial timer**: watch anim+0x102 count down from ≤ 270 to 0 and jump back (state 1 → idle chatter), drone
   in view, close.
