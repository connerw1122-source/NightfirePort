# The wire and the grapple (placement types 0x3E / 0x3A)

Plan v2 §2 struct 11. **The placement type is not the object class**: `Wire_Create` (0x3E,
0x1981d8, 188 bytes) makes a **class 0x2C** object; `Grapple_Create` (0x3A, 0x198298, 124 bytes)
makes a **class 0x51** object [M]. Neither has an update function: `control_funcs` (base 0x26f890,
stride 12 — slot 0x22 = Searchlight_Update 0x1cb900 pins the stride) rows 0x2B/0x2C/0x2D/0x51 are
all zero [L 0x26fa94..0x26fc5c]. All wire/grapple logic is player-side.

## Wire object (class 0x2C) — `Wire_Create` @0x1981d8 [M disasm 0x198200–0x19826c]

`control_create_object(8, …)` — **instance size 8 bytes** (`addiu $a0,$zero,8` @0x198200, `jal
0x1344e0` @0x198210). Placement params, read from `level_tag+0x2C+4k`:

| inst off | read | meaning | tag |
|---|---|---|---|
| +0x00 u16 | `level+0x30` (param 1) (`lhu` @0x198238, `sh` @0x19823c) | **wire type**: 0 = hand-over-hand wire, 2 = zipline; 1 is refused by `Player_CollWire` | [M] |
| +0x02 u16 | `level+0x2C` (param 0) (`lhu` @0x198224, `sh` @0x198234) | not referenced by any wire code found (snapshot value 2) | [M store] |
| +0x04 u16 | `level+0x34` (param 2) (@0x198240/0x198244) | not referenced (snapshot 0) | [M store] |
| +0x06 u16 | `level+0x38` (param 3) (@0x198248/0x19824c) | **switch channel**: set 1 while Bond is on the wire, 0 on drop (`Player_Wire` 0x1976bc–0x1976d4, `Player_Creep` 0x196a44, `Player_Zipline`) | [M] |

Header stores: `obj+0xD8` = celglist arg (@0x198250); `obj+0xF0` = `*(u32*)(glist+0x44)`
(@0x198254/0x19825c); `obj+0xFF = 0x2C` (`addiu $a3,$zero,0x2c` @0x198220, `sb` @0x198258);
`build_LinkToRoom(obj,0,glb_world)` (@0x198260) and `Controls_StraddleTest` (0x134778 @0x198268 —
puts the object on ForcedList 0x270590 with `obj+0xF0 |= 0x10000000 | 0x8000` when it spans ≥2
cels) [M]. `rot.z` is negated at create (@0x198208–0x198214) — **a wire-only quirk: `Ladder_Create`
0x1966c0 passes the rotation quaternion through unmodified** (fix-ladder disasm 0x1966c0–0x19675c;
the earlier "as for ladders" note was wrong) [M].

**There are no endpoint params.** The line geometry is derived at grab/ride time from the object's
world sphere (`obj+0x80..0x8C`, centre + radius) and the placement yaw [M]:

- grab segment (`Player_CollWire`): centre ± `Spherical(radius−2.0, yaw+π/2, …)` (consts 2.0 and
  1.5707964 @0x1971f8–0x197208);
- travel segment (`Player_Wire`): radius − 0.5, pitch forced 0;
- zipline ride (`Player_Zipline`): full radius, pitch = `rot.z` (sloped ziplines);
- creep (`Player_Creep`, the 0x2B creep-wall cousin): full radius, pitch 0.

## Grapple point (class 0x51) — `Grapple_Create` @0x198298 [M disasm 0x198298–0x1982ec]

`control_create_object(0, …)` — **instance size 0**; no placement params read at all. The anchor is
`obj+0x30` (the placement translation). Header stores as the wire (`obj+0xD8`, `obj+0xF0` =
glist+0x44, `obj+0xFF = 0x51` @0x1982c8/0x1982dc, `build_LinkToRoom` + `Controls_StraddleTest`).
Runtime consumers: `Bullet_Delete` 0x12e660 matches hit class byte `'Q'` = 0x51; `Player_SSGrapple`
uses `obj+0x20` (cel, for LOS), `obj+0x30` (pull target), `obj+0xC0..0xCC` / `obj+0x40..0x4C`
(matrix row 3 and previous position — riding carry), `obj+0xFE` bit 0 (deleted → release) [M].

## The player substate machinery [M]

- State word: **`obj+0xF6` (u16 on the player object header, not BLData)**, written by
  `Player_ChangeSubState(obj, id)` 0x190630 (returns the old id). Dispatch: `Player_Update`
  0x193600 switch @~0x193958. `ChangeSubState` side effects: clears `BLData+0x890`;
  id 5/8/9 set `obj+0xFA |= 2`, others clear it and zero `obj+0x50`; every id clears
  `obj+0xF0 & 0x10`; ids 0/4/8/9 re-init anim sets (`BLData+0x954 = 0xFF`); ids 0xD/0xE play anims
  0x60008e0 / 0x6000151; ids ≥ 4 stop looped sounds 0x12/0x3D/0xF0 (ids ≤ 3 return early — so
  **grapple (2) does not stop them, wire (6) does**).
- Substates relevant here: **6 = `Player_Wire` 0x197658** (hand-over-hand crossing, wire type 0),
  **0xF = `Player_Zipline` 0x197f50** (wire type 2), **2 = `Player_SSGrapple` 0x192860**,
  7 = `Player_Creep` 0x1969d0 (the creep-wall system — a different object, class 0x2B; a wire grab
  does **not** enter 7). Full dispatch: 0 `Player_SSWalk` 0x192310, 1 `Player_Climb` 0x196310,
  2 SSGrapple, 3 riding (inline), 4 `Player_SSCrouch` 0x1924c0, 5 `Player_SSScanMode` 0x193098,
  6 Player_Wire, 7 Player_Creep, 8/9 `Player_ZeroG` 0x192ab0, 10 scripted-cam countdown (inline;
  restores the saved substate `BLData+0x942`), 0xF Zipline. Case 3 is the swim, not only riding —
  see `docs/atlas/ladder.md` (fix-ladder).
- Entry points: `Player_CollWire` 0x197130 (the grab) is called from `Player_DealWithObjHit`
  0x194458 (hit-class dispatch: 0x2C → CollWire, 0x2D → CollLadder, 0x27 → Hurt),
  `Player_HandleJump` 0x1916b0 (jump-grab: `BLData+0x95F == 3`, nearest class-0x2C scan) and
  `Player_Enable` 0x195498 (respawn). `Player_CollWire` picks the substate from `inst[0]`:
  type 2 → 0xF, else 6 (`beql` @~0x197300). `Player_Wire` is driven only by `Player_Update` case 6.

### The grab (`Player_CollWire` 0x197130, 1320 bytes) [M]

Guards: regrab lockout `BLData+0x951 == 0`, health `+0x894 > 0`, type ≠ 1, substate ≠ target.
Builds the grab segment (radius−2.0), closest point to Bond; a zipline refuses grabs within half the
segment of the **lower** end. Then: `Player_ClearInertia`, `ChangeSubState(6 or 0xF)`,
`BLData+0x94D = 6` (type 0 also → 0xC grab pose + anim 0x60005e5; type 2 → anim 0x6000095), hang
offset **1.0207** below the wire (type 0) / **1.17363** (type 2), yaw = wireYaw ± π/2 with the
nearer direction picked, teleport to the closest point, `Player_WeaponNone`, `SetCamMode(0xB)`,
viewer `+0x1F0 = 4.0`, `+0x1E4 = −BLData+0x8A8 · π/2`, `+0x1E8 = yaw`, `BLData+0x884 = wire obj`,
and a hand-offset correction from anim 0x4000536 frames 0/0x170 (−0.082 x rotated by yaw).

### Crossing (`Player_Wire` 0x197658, substate 6) [M]

1. `Vec_Zero(BLData+0x50)`; **`Music_Event(0xD, 1)` every tick** (jal @0x1976b4);
   `switch_channels[inst+6] = 1`.
2. Clamps Bond's x/z onto the travel segment (radius−0.5, pitch 0); recomputes the closest point.
3. Auto-drop timer `+0x8CC += 100/(FRAME_RATE·30)` (paused in mode 0xC); > 100 → drop.
4. `Input_Action(slot, 7, 4)` → drop: save substate to `+0x942`, `ChangeSubState(0)`,
   `SetCamMode(0)`, viewer `+0x1F0 = 0`, `+0x8CC = 0`, `+0x951 = 0x1E` (30-frame regrab lockout),
   `AnimObject[99] = AnimObject[100]`, channel 0.
5. Analog `Input_Actionf(slot, 1, 1)`; move-mode byte `+0x94D`: 6 idle, 7/8 moving, 0xA turning,
   0xC grab pose; yaw target wireYaw + 3π/2 (mode 7) or + π/2 (mode 8).
6. Hand-over-hand anims: idle shift pair 0x60005ec / 0x60005ed (±0.0542 along the wire via
   `PS2Sinf2(−yaw+π/2)`); turning pair 0x60005ee / 0x60005ea; move pair 0x60000b4 / 0x60000b5;
   `y = closest.y − 1.0207`; after `AnimScriptAdd` the script's `+0x98` rate is set to ±1.2.
7. **No RNG calls** [M grep of all 12 wire/grapple decomp files]. No direct sounds (music event 0xD
   only; the ChangeSubState(6) entry stops looped sounds once).

Zipline (`Player_Zipline` 0x197f50, substate 0xF): loops anim 0x6000096, accumulates fall speed
`BLData+0x18 += FRAME_RATE_MUL·0.15` per tick; within 1.0 of the lower end → mode 9 + anim
0x6000094; when that anim stops → `ChangeSubState(0)`, cam 0, `+0x951 = (u8)FRAME_RATE`, channel 0.

## The grapple [M]

**Firing is a bullet.** `Bullet_Delete` 0x12e660: owner is a player and weapondef `+0x6C & 4` ⇒
`BLData+0x878 = 0`, `+0x944 = 0xF` (scripted-cam countdown → substate 10 restores); weapondef
`+0x70 & 0x40`, bullet state `+0xF4 == 2` and the HITDATA hit object class == 0x51 ⇒
`Sound_Play3D(…, 0x28D, …)` + `Player_SetGrapplePoint(BLData, bullet+0xC0, hitObj)`; any bullet
death clears `BLData+0x160` (rope visible).

`Player_SetGrapplePoint` 0x195da0: `BLData+0x154 = grapple obj`; `+0x140..0x14C = *point`;
`Player_ChangeSubState(glb_players[BLData+0x94E], 2)` — **grapple substate id = 2**; `+0x15C = 1`
(phase: extending); `+0x15E = 0xF` (15-frame timer); `+0x150 = 0` (pull speed).

**The rope is a player-owned object**: `Player_InitGrapple` 0x195d10 (called from `Player_Init`
0x1910f8) zeroes `BLData+0x140..0x16F` and creates it via `Control_CreateObjEx` with glist
`hashtable_hashcode_to_celglist(0x2000138)` — class byte never set (class 0, empty control_funcs
row), hidden (`obj+0xF0 |= 0x30`, `obj+0xFA |= 5`), proxy `obj+0x1C` = the player object; stored in
`BLData+0x158`. `Player_GrappleSetRope` 0x192728 (every collision tick from
`Player_CollisionHandler` 0x1949a8): hides it unless phase `+0x15C == 1` or `+0x160 != 0`; gate: the
AnimObject `+0x62` gadget id must be 0x50 or 0x51; then rope `+0xC0..0xCC` / `+0x30..0x3C` = the
grapple point, `Mat_Align2Dir(rope+0x90, …)` from the hand bone (bone 0 of `BLData+0x7E8`), and
**`rope+0xE8 (scale) = Dist3D(hand, point)`** — the rope model is unit-length and stretched.

### `Player_SSGrapple` 0x192860 (substate 2, 592 bytes) [M]

`Player_Move` runs first (normal physics); `+0x15E` counts down. Then by phase `+0x15C`:

- **1 extend/pull**: MP-only LOS gate (`MPSettings+0x180 != 0` [M disasm 0x1928e0];
  `Collide_LineOfSight` fail → phase 4). Rope-fire anim done (`AnimObject+0x60 & 8`) with the timer
  at 0 → phase 4 (miss). `Dist3D(playerPos, point) < 2.0` → phase 2 (hang). Otherwise pull:
  `+0x150 = min(+0x150 + 0.05, MP ? 0.5 : 1.5)`; `playerPos += normalise(point − playerPos)·+0x150`.
- **2 hang**: `Input_Action(slot, 9, 1)`; pressed → commit velocity (`+0x20..0x2F = +0x10..0x1F`,
  then zero `+0x10..0x1F`) and stay hanging; not pressed → phase 4.
- **4 detach**: `ChangeSubState(obj, 0)`, phase 0 — Bond falls with the committed momentum.
- Tail (rope-follow): if `+0x154` set, the point is carried by moving platforms
  (`obj+0xC0` row 3 − `obj+0x40` prev pos); `playerObj+0x34 (y) += objRow3.y − obj+0x44`; deleted
  obj (`obj+0xFE & 1`) → `+0x154 = 0`.

No RNG [M grep].

## BLData fields used by wire/grapple [M]

| off | type | meaning | writers | readers |
|---|---|---|---|---|
| +0x10..0x1F | 4×u32 | wish/physics velocity triple (zeroed each tick by `Player_Update`) | Player_Update, Player_Move | SSGrapple commit |
| +0x18 | f32 | fall-speed accumulator (zipline `+= FRAME_RATE_MUL·0.15`; creep drop `+= 0.05`) | Zipline, Creep | Player_Move [S] |
| +0x20..0x2F | 4×u32 | applied velocity (commit target) | SSGrapple | movement [S] |
| +0x130 | u8 | = 2 on wire/creep-wall grab | CollWire, CollCreepWall | unverified |
| +0x140..0x14C | VECTOR | grapple target point | SetGrapplePoint, SSGrapple carry | SSGrapple, GrappleSetRope |
| +0x150 | f32 | pull speed (max 1.5 SP / 0.5 MP) | SetGrapplePoint(0), SSGrapple | SSGrapple |
| +0x154 | ptr | grapple point object | SetGrapplePoint | SSGrapple |
| +0x158 | ptr | rope object | InitGrapple | GrappleSetRope |
| +0x15C | u16 | grapple phase: 0 idle, 1 extend, 2 hang, 4 detach | SetGrapplePoint, SSGrapple | SSGrapple, GrappleSetRope |
| +0x15E | u16 | 15-frame grapple timer | SetGrapplePoint, SSGrapple(−1) | SSGrapple |
| +0x160 | u8 | rope-visible flag | Bullet_Delete(0) | GrappleSetRope |
| +0x884 | ptr | current wire/creep/ladder object | CollWire, CollCreepWall, CollLadder | Player_Wire/Creep/Zipline |
| +0x890 | u32 | substate scratch (cleared on change) | ChangeSubState | substates 0xD/0xE |
| +0x894 | f32 | health | health system | CollWire, CollCreepWall |
| +0x8CC | f32 | wire auto-drop timer | Player_Wire | Player_Wire |
| +0x942 | u16 | saved substate (restored by dispatch case 10) | Wire/Creep drop paths | Player_Update case 10 |
| +0x944 | u16 | scripted-cam countdown (grapple shot sets 0xF) | Bullet_Delete | case 10 |
| +0x94D | u8 | wire/creep move-mode byte (wire: 6 idle, 7/8 move, 0xA turn, 0xC grab, 9 zipline-arrive; creep: 0 hang, 1/2 move) | CollWire, Player_Wire, Creep, Zipline | all of them |
| +0x94E | u8 | player slot (glb_players / Input index) | init | everywhere |
| +0x951 | u8 | regrab lockout (0x1E wire drop, 0x78 creep drop; decremented per tick) | Wire/Creep drops, Player_Update | CollWire, CollCreepWall |
| +0x95F | u8 | action context (3 = jump-grab, 6 = activate creep wall) | SSWalk etc. [S] | HandleJump, Player_Activate |

## Snapshot check [L] (work/ee_start_05.bin)

| addr | raw | meaning |
|---|---|---|
| 016dc510 (wire inst) | `00 00 02 00 00 00 00 00` | type 0 (hand-over-hand), param 0 = 2, param 2 = 0, **channel 0** |
| 016dc430/016dc450 | f32 (−23.4855, 34.4676, 6.26292) / (0, −1.5708, 0) | wire pos + yaw; sphere r 4.04 ⇒ travel endpoints ≈ (−19.95…−27.03, 34.47, 6.26) |
| 016dc4f0 | `00000040` (obj+0xF0) | glist+0x44 passthrough flag |
| 016d80d0 | f32 (13.7525, 32.2919, 57.3034) | grapple point #1 anchor (obj+0x30); class byte @016d819f = **0x51** |
| 016d81b0 | `98989898 016d8094 …` | grapple point #2 "instance" = malloc fill + header — instance size 0 confirmed |
| 0x016cf0d6 (player obj+0xF6) | `0000` | substate 0 (SSWalk, idle) |
| 0x016cf248 (BLData+0x158) | `016c5510` | the rope object exists at level start; its +0xF0 @016c5600 = `0x8130` (hidden 0x10 + 0x20 + 0x100 + 0x8000), +0xFA = `0x0005`, proxy +0x1C @016c552c = `016cefe0` (the player) ✓ |
| 0x016d6ab0 (creep wall inst) | `0000 0001 0000 0071` | creep wall (class 0x2B), channel 0x71 |

Note: the only class-0x3E object in the snapshot (0x0137b950) is an AI volume — placement type 0x3E
rows of `control_funcs` (delete slot 0x13a1d0) belong to that class, not to wires; likewise 0x3A's
row (Lock_Update 0x1ae3d8) belongs to locks. No class-0x2C update exists.

## Port corrections (crates/nf-game/src/wire.rs) — for the next session

- `channel` is documented as `placement.extra[2]` — actually **inst+0x06 = param 3**; the claimed
  glist-hashcode param (`extra[3]` = 0xC510) is really **inst+0x00 = the wire type** [M disasm;
  L inst bytes above]. The live object holds `00 00 02 00 00 00 00 00`, not 0x016D/0xC510.
- Endpoints: the port derives them from the placement translation + glist extent; the game derives
  them from the **world sphere radius−2.0 (grab) / −0.5 (travel)** along yaw+π/2 at grab time, with
  zipline pitch from rot.z [M].
- Missing entirely: wire type 2 (zipline, substate 0xF), the substate split (the port's comment
  "enters the creep substate" is wrong — wires enter 6, creep-walls 7), the auto-drop timer
  (+0x8CC), regrab lockout (+0x951), hang offsets 1.0207/1.17363, the per-tick (not proximity-gated)
  `Music_Event(0xD, 1)`, `Controls_StraddleTest` ForcedList membership.

## Not verified this pass

- The meaning of wire param 0 (inst+0x02) and param 2 (inst+0x04) — no reader found; wire type 1's
  other users (CollWire refuses it).
- `BLData+0x130 = 2` consumers; the exact velocity semantics of +0x10/+0x20 (inferred from
  `Player_Move` usage); FRAME_RATE's runtime value (the timer math depends on it).
- `Control_CreateObjEx`'s stack-tail args in `Player_InitGrapple` (0x30, 0xFF×3) — ABI partially
  hidden behind undecodable MXU moves; mapping to params 10–13 inferred from its signature.
- Whether other levels' wires use type 2 / nonzero channels (only 07000005 checked).
