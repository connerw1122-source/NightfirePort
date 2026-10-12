# The player object's `obj_tag` — the class-3 (and 0x12) instance

Proof levels: see [README.md](README.md). The generic `obj_tag` map is `docs/atlas/object.md` — this doc
covers what the PLAYER path does with each header field. Live instance: `glb_players` 0x2d88e0 →
**obj 0x16cefe0** in `work/ee_start_05.bin` [L]; `glb_blokes` 0x2ba190[0] = 0x16cf0f0 = the BLData
(= obj+0x110) [L].

## The class table (`control_funcs` @ 0x26f890, 85 entries × 12 bytes: update, collide, delete)

Dispatch: `control_movement_object_handler` 0x1336f8 calls slot 0 by `*(u8*)(obj+0xFF)` [M 0x1336f8:62-63];
`Collide_Update` 0x1ea398 slot +4; `control_delete_object` 0x133178 slot +8. Relevant rows [M ELF bytes]:

| Class | Update | Collide | Delete | Name |
|---|---|---|---|---|
| 2 | 0x139268 Drone_Control | 0x140968 | 0x136ce8 | placement/script drone |
| **3** | **0x193600 Player_Update** | **0x1949a8 Player_CollisionHandler** | **NULL** | **PLAYER (alive)** |
| 0x11 | 0x139268 | NULL | 0x136ce8 | bot/drone (MP) |
| 0x12 | 0x193600 | 0x1949a8 | NULL | **PLAYER (dead)** |

The class byte: `Player_Init` 0x1910f8:76 writes `obj+0xFF = 3`; on death `Player_CheckForDeath`
0x190810:59 re-classes to **0x12**; `MP_ReSpawn` 0x185368:83 restores 3. Class 0x12 exists so collision
filters can exclude "alive" and "dead" separately: `Collide_PickObj` 0x1eae80:84-86 excludes class 3 and
0x12 for HITTEST mask 0x400, classes 2/0x11 for mask 0x100. `Player_CheckForDeath(obj, 3)`'s second arg is
the expected class [M 0x190810:34, 0x1949a8:187]. Class 3 has NO delete handler — the player object is
never deleted; it persists across levels (RamSave/RamLoad reuse the BLData [M 0x195670/0x1957a8]).

## `PLAYERS` / `glb_players` 0x2d88e0

An array of **4** `obj_tag*` slots (symtab size 16) [M], indexed by the player slot `BLData+0x94E`:
`(&glb_players)[slot] = obj; (&glb_blokes)[slot] = instance; *(char*)(instance+0x94E) = slot; *(char*)(instance+0x94F) = slot` [M 0x1910f8:36-39].
[L] slot 0 = 0x016cefe0, slots 1-3 = 0. Bare `glb_players` in the decompile = slot 0 (single player):
`Trigger_Update`'s activator 0x1b1a78:164, `NDrone2_Player()` 0x149768:7, `GT_Track` 0x17f9c8:113,
`Sub_Update`/`Copter_FireGun` position reads 0x1ad200:98-108 / 0x17d828:55-59, `Car_*` indexed by the
car's occupied-slot halfword `car+0xF6` 0x130060:184. The HUD's night-sight monitor gates on
`glb_players[slot]->+0xF4 == 1` (alive) [M 0x19e9d8:18-25]. `Mission_Status` 0x1830d0 does NOT read the
player obj. Companion: `glb_viewer` 0x2d88b0 (11 pointer slots [M symtab size 44]; `viewer+0x224 = obj`).

## The field map (class-3 meanings; the live obj values [L/S] from /tmp-fp style dumps of ee_start_05)

| Off | Type | Class-3 meaning | Proof | [L] | Port |
|---|---|---|---|---|---|
| +0x08/+0x0C | ptr | cel-list next/prev (`build_LinkToRoom` relinks at Init/StandAtNewPosition/Enable/ReSpawn) | [M atlas + 0x190d88/0x195498] | 0x016d1ec0 / 0x016c4780 | implicit |
| +0x14/+0x18 | ptr | DynamicObjList next/prev | [M atlas] | 0x016cfde0 / 0x016c82d0 | — |
| +0x20 | ptr | **cel pointer** — three player uses: (a) the water surface = `cel+0x90` (`Player_InWater` 0x195e50:12-36; the swim clamp 0x193600:184-222; the gravity skip 0x194df0:101-149), (b) the music zone `Music_Event(cel+0xA1, 1)` [M 0x193600:119-121], (c) the pick queries [M 0x195498:91-93, 0x1a7810:123]. Also cached to BL+0x7C0 [M 0x194df0:89] | [M*] | 0x016f6e20; cel+0x90 = −500 (no water) | water arrives out-of-band as `LadderCtx.water` — no cel pointer |
| +0x30..0x3C | vec4 | **position (+ the W lane)**: w (+0x3C) = the placement's vec4 W (−0.9875 on 05 [L]) written only by the whole-position copies (StandAtNewPosition 0x190d88:41-49 + the floor snap :108-112; Enable 0x195498:57-63 from a matrix row; MP_ReSpawn 0x185368:30-47); moved by `pos += M·vel` [M 0x193600:284-295] and the collision push `pos += BL+0x750` [M 0x1949a8:136-140]; the kill-plane test reads +0x34 < −10.0 on level 0x07000012 [M 0x1949a8:104-107] | [M*] | (104.954, 24.963, 108.825, −0.9875) | `Bond::pos` (no w lane) |
| +0x40..0x4C | vec3 | prev position (engine) | [M atlas] | = pos | — |
| +0x50..0x5C | vec3 | rotation (pitch, yaw, roll): yaw +0x54 from the placement [M 0x1910f8:81]; swim zeroes pitch/roll [M 0x195e50:28-30]; the ladder eases yaw toward `ladder.yaw+π` [M 0x196310:18-33]; anim root motion adds yaw [M 0x1befd8:245]; the death drift `rot += 0.001` while DAT_002d8894 == 0 [M 0x190bb0:15-19]; the debug scan-mode zeroes pitch and rolls rot into +0x60 [M 0x193600:145-153] | [M*] | (0, −2.1468, 0) | `Bond::yaw` (pitch/roll implicit) |
| +0x60..0x6C | vec3 | prev rotation | [M atlas] | = rot | — |
| +0x70..0x7C / +0x80..0x8C | vec4 ×2 | the prev / world sphere — for the player built from the **AnimObject local sphere** (flags2 bit 8 set) [M atlas 0x133070]; radius 1.0576 ≈ the capsule | [M*] | (104.826, 24.837, 108.582) r 1.0576 | recomputed |
| +0x90..0xCC | MATRIX | rows 0-2 rotation (row0 = left, row1 = up, row2 = forward), **row 3 (+0xC0..0xCC) = translation**. Rebuilt by `Control_GetObjMatrix` whenever the anim tracks change [M 0x1befd8:288-292] then `Mat_CopyRot(obj+0x90, BLData)` [M 0x1befd8:238-239]; every frame's tail copies **+0xC0..0xCC = +0x30..0x3C** and sets flags2 `+0xFA |= 0x20` (bit 0x4 gates the copy) [M 0x193600:443-450, 0x1949a8:177-183] | [M*] | rows = the yaw rotation ✓ | rows re-derived from yaw |
| +0xD0 | ptr | the HITDATA list (bullets/ladder/wire hits); freed at death | [M* 0x190810:96, 0x1949a8:110-152 → DealWithObjHit dispatch by the hit obj's class 5/0x27/0x2C/0x2D] | 0x01fbee20 | hits dispatched by class ✓ |
| +0xD4 | ptr | **class scratch = BL+0x740** — the ground HITTEST (a second lives at BL+0x110) | [M 0x1910f8:40; r 0x1949a8:76-77] | 0x016cf830 ✓ | the port's own ground ray |
| +0xD8 | ptr | the glist (drives the sphere source and pick tests) | [M 0x1eae80:90] | 0x017381d0; glist+0x44 = 0x8A0 | — |
| +0xDC | ptr | **the AnimObject** — the drawn model. Class-3 reads: +0x60 flags (bit 0 aim, bit 2 the footstep tick, bit 3 grounded/climb, bit 8 climbing, bit 0x100 in-water/death), +0x62 the weapon id byte, **+0xCC the ROOT HEIGHT** (1.0347 [L]; the feet-point math 0x194df0:86,222-236), +0xD8 the per-frame move distance (written 0x193600:359-360; consumed by the BL+0x924 stride accumulator 0x1befd8:160-179), +99/+100 the weapon-request/saved bytes | [M*] | 0x016cb8a0: flags 0x0008, weapon 0x02, +0xCC = 1.0347, +0xD8 = 0.37 | `ROOT_HEIGHT`/`CROUCH_ROOT_HEIGHT` constants + per-weapon view models |
| +0xE0 | ptr | the instance = obj+0x110 = BLData | [M 0x193600:70 first load] | 0x016cf0f0 ✓ | folded into `Bond` |
| +0xE8 | f32 | scale 1.0 (multiplies the sphere radius) — never changed for the player | [M atlas 0x134630] | 1.0 | implicit |
| +0xEC | u32 | **a frame stamp** (not wobble): init = GameState[52]; rewritten at death (`Player_CheckForDeath` — the MP respawn waits 5·FRAME_RATE past it [M 0x190810:60, 0x190bb0:41-55]) and at respawn [M 0x185368:90]; read every frame by `Env_Breath(pos, dir, cel, obj+0xEC)` [M 0x193600:369-377] | [M*] | 0x0001A676 | — |
| +0xF0 | u32 | flags: **0x800 set by Player_Init** [M 0x1910f8:149]; 0x20 cleared by AnimObjectNew/MP_ReSpawn [M 0x1be910:134, 0x185368:84]; **0x10 = disabled/not drawn** (`Player_Disable` 0x195f70:16-21 sets it + BL+0x94A=0; `Player_Enable` clears) [M*] | | 0x00000880 | none (Sim::disabled out-of-band) |
| +0xF4 | u16 | **the PLAYER STATE**: 0 = just spawned (promoted to 1 by the first update [M 0x193600:313-315]); **1 = alive & in control** (gates the whole input/physics/substate/weapon block; StandAtNewPosition sets it 0x190d88:57); **2 = dying/dead** (CheckForDeath 0x190810:66; runs AnimObjectUpdate + Player_HandleDeath [M 0x193600:317-320]; blocks the night-vision HUD; passed as `dead` into the feet test `(u16)(state−2) < 2` [M 0x1949a8:77]); 3 = fully dead (HandleDeath skips the MP respawn [M 0x190bb0:52]; no writer found); 4 = dead-but-weapon-block (jumps into the weapon block [M 0x193600:436]; no writer found). `Player_Move` refuses unless state ∈ [2,4]... precisely: the move paths check it [M 0x191dd0:43]. Setter helper `Player_ChangeState` 0x195be8 | [M*] | 0x0001 ✓ | **not modeled** — the port has no state field; every tick is the state-1 path |
| +0xF6 | u16 | **the sub-state** (movement mode): 0 walk, 1 climb, 2 grapple, 3 swim, 4 crouch, 5 scan-mode, 6 wire, 7 creep, 8/9 zero-G, 10 script-cam countdown (restores BL+0x942), 0xF zipline; written by `Player_ChangeSubState` (mirrors BL+0x942) 0x190630; the collision/camera/Check_Target dispatches read it | [M*] | 0 (walking) | `Bond::substate` ✓ |
| +0xF8 | u16 | **the per-view draw mask** — `View_SetDrawInOtherViewsOnly(obj, slot)` sets `+0xF8 = ~(1<<slot) & 0x1F` every frame from the Player_Update tail [M 0x1e7ba0:16-19, 0x193600:442] — the model is drawn in every view EXCEPT the owner's (first-person); propagated to the LOD parts and the weapon obj | [M*] | **0x001E** = ~1 & 0x1F ✓ | none (single view) |
| +0xFA | u16 | flags2: bit 0x8 = the sphere from the AnimObject; **bit 0x20 forced on every frame** by both tails (read BEFORE the \|=\) [M 0x193600:443-450] — drives the movement pass's wrap/sphere/cel/prev work; bit 0x4 gates the pos→row3 copy | [M*] | 0x0008 | none |
| +0xFC | u8 | the draw counter (init 2; decremented by the movement handler; reset by View_AddObjects when drawn); the auto-aim skips targets with +0xFC == 0 [M 0x1a73f8:50-52] | [M atlas] | 0 | — |
| +0xFD | u8 | the pick bits: `Player_Collision` sets **1** every frame [M 0x194df0:410] — as a querier it makes `Collide_PickObj` skip targets whose +0xFD & 2 (bullets set 2 [M 0x12c5a0:88]); drones set 1 too [M 0x140e90:168] | [M*] | 0x01 ✓ | — |
| +0xFE | u8 | control flags: bit 1 = the rebuild-sphere request (init, cleared by the movement handler); bits 2/3 set by `AnimObjectNew` (skinned / LOD parts) [M 0x1be910:118-125,166] | [M*] | 0x0C ✓ | — |
| +0xFF | u8 | **the class**: 3 alive → 0x12 dead → 3 on respawn | [M*] | 0x03 ✓ | none (fixed behaviour) |
| +0x100..0x102 | u8×3 | the ambient RGB (recomputed from the cel) | [M atlas] | 0x39,0x46,0x56 | engine-side |
| +0x103..0x106 | u8×4 | tint RGB + **alpha**: +0x106 = the model alpha, written every `Player_Update` from BL+0x888 (the fade): `alpha = (u8)(fade·255)` when fade ≥ 0.5, else **0x28 (40)** [M 0x193600:92-114] | [M*] | FF FF FF FF | gap |

`control_init_collision` (called by Player_Init 0x1910f8:46) is an EMPTY function [M 0x134878:3-7].

## Disagreements (never resolved silently)

1. **`obj+0xCC` is NOT the player's root height.** docs/systems/player.md (pre-correction) said
   "`obj+0xCC = obj+0x3C` every tick — the stance's root height". The tail copy at Player_Update
   0x193600:449-453 / Player_CollisionHandler 0x1949a8:179-183 is the 16-byte position→matrix-row-3 copy
   (gated on flags2 bit 4). The root height value 1.0347 [L] lives at **AnimObject+0xCC** and is consumed
   there by the feet-point math [M 0x194df0:86,222-236]. The port's NUMBERS were right; the claimed
   mechanism was wrong. **player.md and player.rs corrected this session (fields-player).**
2. The atlas's +0xEC "wobble" story: for class 3 it is a frame stamp (death frame / breath phase / the
   respawn timer) — the rewriters are CheckForDeath / MP_ReSpawn.
3. The port's "obj+0xF4 == 1" comment is right about the branch, but the port has no +0xF4 at all: no
   0→1 promotion, no dying states, no class re-class 3↔0x12, no Player_HandleDeath/MP_ReSpawn (death is a
   Sim bool + switch channel 0x62 [M mission.rs:192-200]).

## Not fully enumerated (honesty)

The readers of obj+0xFE bits 2/3; any writer of player states 3/4 (possibly stripped debug/script paths);
the exact `Control_SetGList` call in the player's creation; `glb_viewer`'s slot count (11 vs the 4
players); the `benchmark` gate setter (Player_Update 0x193600:161); Check_Target's +0x132/+0x133
target-id bytes beyond the crosshair mapping (they are in [bldata.md](bldata.md)).

## Live checks for the main agent

1. **The class byte**: `watch 0x016cf0d7 1` (obj 0x16cefe0 + 0xF7? no — +0xFF = 0x016cf0df) during a death
   on any level — it should flip 3 → 0x12 exactly when the death anim starts, and back on an MP respawn.
2. **The draw mask**: `watch 0x016cf0d8 2` (obj+0xF8) — it should sit at 0x001E in single-player (the
   model drawn in every view but the first-person one) and track the active slot in split-screen.
3. **The alpha floor**: cut-scene fade on 07000001's intro — obj+0x106 (0x016cf0e6) should hold 0x28
   while BL+0x888 < 0.5 and rise with it above.

## The census to run

`gdbwatch 0x016cefe0 0x110 readwrite --seconds 8` on 07000005 across a walk-switch-shoot pass plus one
death: the expected accessors are Player_Update/CollisionHandler/Collision (0x193600/0x1949a8/0x194df0),
AnimObjectUpdate (0x1befd8), Check_AutoAim (0x1a73f8), the movement handler (0x1336f8), Collide_PickObj
(0x1eae80) and View_* (0x1e62d8/0x1e7ba0). A reader outside that set means a missed accessor.
