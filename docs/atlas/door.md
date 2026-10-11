# Struct 6 — the door (DOORINFO ext, class 0x1C)

Allocation [M]: `Door_Create` (0x136100) calls `control_create_object(0x80, ...)` @0x1b.../0x136... — a
0x80-byte ext at `obj+0xE0`; class byte `obj+0xFF` = 0x1C @0x136259-ish (decomp :62). The ext is
appended to the global list `TheDoors` @0x316f70 (`LList_Add(0x316f70, ext)`, decomp :94) — the
list nodes ARE the ext records (ext+0x04 = next, ext+0x08 = obj) [M]. `Door_Init` (0x136848)
initializes the list (`LList_Init(0x316f70, 0x80, 0)`) and zeroes `DoorGroupsStates` (0xCA bytes)
[M]. Registration: `control_funcs` @0x26f890 entry 0x1C @0x26f9e0 = {0x1363c8, 0x135980, 0} —
Door_Update is the update function and **Door_ReactToHit is the collide function**, called by
`Collide_Update` (0x1ea398, decomp :59-63: `control_funcs[class*0xC+4](obj)`) — **not** from
Door_Update [M, ELF bytes].

## The state register `obj+0xF4` (u16)

`Door_SetState` (0x135618) overwrites it (`*(ushort*)(obj+0xF4) = param_2`, decomp :30) [M].
States observed in code [M]:

| value | meaning |
|---|---|
| 0x02 | CLOSED (steady) |
| 0x04 | OPEN (steady) |
| 0x08 | OPENING (animating; open sound on entry) |
| 0x10 | CLOSING (animating; close sound on entry) |

Evidence: Door_Activate toggles {0x04,0x08} → SetState(0x10) and {0x02,0x10} → SetState(8)
(decomp 0x135be0 :121-166, disasm 0x135d4c-0x135d88); Door_SetState plays ext+0x60 on 0x08-entry
and ext+0x62 on 0x10-entry (:10-24); Door_Update runs Door_Interp only while
`state & 0x18` (animating) and copies the placement matrices otherwise (decomp :158-168);
Door_IsClosed (0x1367b0) tests `state == 2` exactly. Door_Activate's locked-door check treats
{0x02, 0x10} (`state & 0x12`) as "closed family" [M].

## Field table (ext, 0x80 bytes)

| off | type | name [S] | writers | readers |
|---|---|---|---|---|
| +0x00 | 4 B | list node prev/next pair with +0x04 | LList_Add | Doors_Calc walk (+0x04 next) |
| +0x04 | ptr | next ext in TheDoors | LList_Add | Doors_Calc |
| +0x08 | ptr | obj_tag | Door_Create (decomp :52) | everywhere (ext→obj) |
| +0x0C | u32 | anim_id (m_paths) | Door_Create ← tag+0x08 (:43) | Door_Interp / Spline_Interp, Door_Create spline setup |
| +0x10 | MATRIX | closed-pose matrix (inverse of frame-0) | Door_Create / Door_SetupSwing Spline_Interp | Door_Activate RotPreTransVec (anti-crush, swing only) |
| +0x50 | f32 | t (anim param 0..1) | Door_Create (0 or 1 when created open); Door_SetupSwing (0.5 or 1.0); Door_Interp advance; Doors_Calc group copy | Door_Activate (t>0.5 flip), Door_Interp, Door_ReactToHit |
| +0x54 | f32 | sec per frame | Door_Create = 1.0/((u16)tag+0x04 or FRAME_RATE) (:46-49); Door_SetupSwing re-derives `0.5/(int)(0.5/(2·0x54))` (:25-27) | Door_Interp advance |
| +0x58 | f32 | dir (+1 opening / −1 closing; ±1.0 directly for sliding) | Door_Activate (disasm 0x135e88 `swc1 f0,88(s0)`); Doors_Calc group copy; Door_ReactToHit negation | Door_Interp, Door_Activate, Door_ReactToHit |
| +0x5C | f32 | swing dir sign (±1.0) | Door_SetupSwing only (:76-79, from the frame-0 → frame-N direction dot) — **never written for sliding doors** (stays 0 from the memset; sliding paths store ±1.0 directly) | Door_Activate swing path, Door_ReactToHit swing path |
| +0x60 | u16 | open sound | Door_Create ← tag+0x40 (param 5), 0xFFFF when 0 (:28-32) | Door_SetState |
| +0x62 | u16 | close sound | Door_Create ← tag+0x44 (param 6) (:33-37) | Door_SetState |
| +0x64 | u16 | locked sound | Door_Create ← tag+0x48 (param 7) (:38-42) | Door_Activate, Door_SetState |
| +0x66 | u16 | open channel | Door_Create ← tag+0x34 (param 2) (:26) | Door_Update, Door_SetState, Door_Activate, Door_Create |
| +0x68 | u16 | lock channel (on = unlocked) | Door_Create ← tag+0x38 (param 3) (:27); Door_Update clears it after a rising-edge unlock | Door_Update, Door_Activate, Door_IsLocked 0x136810, Door_DoMsg 0x135f18 |
| +0x6A | u16 | group id (DoorGroupsStates index) | Door_Create ← tag+0x30 (param 1) (:25) | Door_Update (DoorGroupsStates[g] \|= state), Doors_Calc sync, Door_IsClosed, Door_DoMsg |
| +0x6C | s16 | cooldown (frames) | Door_Activate = 2; Door_ReactToHit = 2; Door_Update decrements | Door_Update (gate for everything), Door_ReactToHit guard |
| +0x6E | u16 | anim frame count − 1 | Door_Create ← (u16)tag+0x02 − 1, 0 when tag+0x02==0 (:44) | Spline_Interp everywhere |
| +0x70 | u16 | **flags** (see bit table) | Door_Create = (u16)tag+0x4C \| (u16)tag+0x50 (:50-53); Door_Activate \|= 0x10; Door_ReactToHit \|= 0x10; Door_Update lock path \|= 0x10; Doors_Calc &= ~0x10 | see bit table |
| +0x72 | u16 | quiet timer (auto-close) | Door_Update increments @0x136644-0x136650; Door_SetState = 0 (:31); Door_Update = 0 after each scan; Doors_Calc group copy | Door_Update threshold test |
| +0x74 | u16 | msg latch (one "locked" message per close cycle) | Door_DoMsg = 1 (0x135f18) | Door_DoMsg |
| +0x76 | u8 | one-sided mode | Door_Create ← (u8)tag+0x54 (:69) | Door_Activate (direction memory block) |
| +0x77 | u8 | no-E-use | Door_Create ← (u8)tag+0x58 (:70) | Door_Activate entry (param_3 && +0x77 → refuse) |

Object-side [M]: obj+0xF4 state (Door_SetState), obj+0xF6 = (u16)tag+0x2C — **kind**: 0 = swing
(Door_SetupSwing at create), 1..2 = path-animated sliding, >2 or <0 = inert toggling
(Door_Activate `slti v0,3` / `bltz` 0x135d3c-0x135d44), 1 = Door_IsClosed always reports closed;
obj+0xFD bit 3 = portal-linked (Door_Create sets it when tag+0x3C != 0; `Door_Link2Portal`
0x136788 returns `(obj+0xFD >> 3) & 1`); obj+0x80 gfx sphere centre, obj+0x8C radius
(`Control_BuildWorldSph` at create, decomp :95); obj+0xFA \|= 0xC4 (draw flags).

## Flags word ext+0x70 — bits

Formed at create [M]: `ext+0x70 = (u16)tag+0x4C | (u16)tag+0x50` — **placement param 8 | param 9**
(Door_Create decomp :50-53). Bits [M unless noted]:

| bit | meaning | evidence |
|---|---|---|
| 0x01 | interp mode: 0 = KeyFrame_Interp, 1 = Spline_Interp (Catmull-Rom/slerp) | Door_Interp 0x135738 :17-27 |
| 0x02 | **no reader found in any door function** — an editor/level tag (U-043 stays open) | rg over all door decomps [M] |
| 0x04 | "starts open": initial state = 4 instead of 2, and the open channel is forced on at create (`switch_channels[open]=1` + timestamp, Door_Create :96-99) | Door_Create :54-59, :96-99 |
| 0x10 | busy / animating (transient): set by Door_Activate (0x135ef8 `ori v1,v1,10`) and Door_ReactToHit; cleared for every door every frame by Doors_Calc's second pass; Door_Update returns immediately when set (0x136408-0x136410) | [M] |
| 0x20 | **keep-open — suppresses the 360-tick auto-close** | Door_Update 0x136620/0x136624 [M] |
| 0x02..0xC8 others | unexamined (no reads found in door code) | [S] |

## Placement param → field mapping (0xDB)

| param | field |
|---|---|
| p0 (tag+0x2C) | obj+0xF6 kind (0 swing / 1,2 sliding) |
| p1 (tag+0x30) | ext+0x6A group id |
| p2 (tag+0x34) | ext+0x66 open channel |
| p3 (tag+0x38) | ext+0x68 lock channel |
| p4 (tag+0x3C) | obj+0xFD \|= 8 (portal link) when ≠0 |
| p5/p6/p7 (tag+0x40/44/48) | open / close / locked sound (0xFFFF when the u32 is 0) |
| p8 (tag+0x4C) | flags low byte |
| p9 (tag+0x50) | flags high byte |
| p10 (tag+0x54) | ext+0x76 one-sided mode (byte) |
| p11 (tag+0x58) | ext+0x77 no-E-use (byte) |
| tag+0x02 / tag+0x04 | anim frame count − 1 / frames per second (→ ext+0x6E / ext+0x54) |
| tag+0x08 | anim id (→ ext+0x0C) |

## Door_Update (0x1363c8) — per frame

1. ext+0x6C cooldown decrement (0x1363f8-0x136404) [M].
2. **`ext+0x70 & 0x10` (busy) → return** (0x136408-0x136410) [M].
3. Channel logic, only when `state & 6` (steady states) and open channel ≠ 0 (0x136418-0x1364a4)
   [M]: state 0x04 (open) + `ch[open]` off → proximity scan; nobody near → `Door_Activate` (close)
   + ext+0x72 = 0 — the **immediate channel-off close**; state 0x02 (closed) + `ch[open]` on →
   `Door_Activate` (open) (guarded by `obj+0xF4 & 4` already-open flag at 0x136458).
4. Lock channel rising edge [M] (0x136550-0x136608, decomp :88-108): `ext+0x68 != 0` and
   `switch_channels_prev[lock] == 0` and `ch[lock] != 0` and not busy → "locked-unlock" message
   (label 0x2000003, skipped on levels 0x07000014 / 0x0700004A @0x13659c-0x1365b8) +
   `Door_Activate` (opens) + **ext+0x68 = 0** (one-shot unlock) + busy.
5. **The 360-tick auto-close** (0x13660c-0x136700) [M]:

```
0013660c  lhu v0,244(s1)    ; state = obj+0xF4
00136610  andi v0,v0,4      ; state & OPEN
00136614  beql v0,zero,00136704   ; not open -> skip
0013661c  lhu v0,112(s2)    ; flags = ext+0x70
00136620  andi v0,v0,0x20   ; keep-open
00136624  bnel v0,zero,00136704   ; keep-open SET -> skip auto-close   <-- THE branch
0013662c  lui a0,31
00136630  lw  a0,-12084(a0) ; FRAME_RATE_INT @0x30d0cc  (= 60 [L])
00136634  addiu a1,zero,6   ; 6
00136638  lhu v0,114(s2)    ; quiet = ext+0x72
0013663c  mult a0,a1        ; 360
00136644  addiu v0,v0,1
00136648  sltu v1,v1,a0     ; quiet < 360 ?
0013664c  bne  v1,zero,00136700   ; below threshold -> just store quiet+1
; --- timer expired: proximity scan 0x136654..0x1366e0 ---
;   skip objects with obj+0xF0 & 0x10 (0x136678-0x136684)
;   classes 2 (drone) / 3 (player) / 0x12 via obj+0xFF (0x13668c-0x1366a0)
;   Vec_SqDist3D(obj+0x80, other+0x80) vs 2*(r_door+r_other)^2   (0x1366b4-0x1366c8)
001366e4  beq  s7,zero,001366fc   ; somebody near -> skip close
001366f4  jal  Door_Activate      ; nobody near -> close it
001366fc  sh   zero,114(s6) ; ext+0x72 = 0
```

   Predicate [M]: **auto-close fires iff `state == OPEN (0x04)` AND `!(ext+0x70 & 0x20)` AND
   `ext+0x72` reaches `FRAME_RATE_INT·6` (= 360 ticks = 6 s) AND no candidate object is within
   `sqrt(2)·(r_door+r_other)` of obj+0x80.** A state change resets the timer (Door_SetState).
   The candidate filter `obj+0xF0 & 0x10` skips objects whose glist flags carry bit 0x10 — the
   player and drones both have it SET on 07000001 (player obj+0xF0 = 0x890, drone 0x8B0 [L]) but
   the 07000005 player has it CLEAR (0x880 [L]) — so on 07000001 the scan finds nobody and the
   auto-close is effectively unconditional; on 07000005 a player within ~2.5 m of the sphere
   centre delays it (r_door + r_player ≈ 2.3 → zone ≈ 3.3).
6. `state & 0x18` → `Door_Interp(obj, ext, 1)` (0x136704-0x136718); else copy placement matrices
   obj+0x30..3C → obj+0x40..4C (0x136724-0x13672c) [M].
7. `ext+0x6A != 0` → `DoorGroupsStates[g] |= obj+0xF4` (0x136730-0x136754) [M].

## Door_Activate (0x135be0), Door_SetState (0x135618), Door_Interp (0x135738)

Door_Activate(obj, player, from_E) [M]:
- `from_E && ext+0x77` → return 0 (no-E-use doors);
- busy `ext+0x70 & 0x10` → return 0;
- lock channel defined and `ch[lock] == 0` and `state & 0x12` (closed family) →
  `Sound_Play3D(ext+0x64)` (locked sound) + return 0 (decomp :53-61, :83-87);
- ext+0x76 one-sided direction memory block (dot of player−door vs the door's facing;
  can refuse with the locked sound);
- **kind 0 (swing)**: anti-crush — RotPreTransVec of the player's position into the closed-pose
  matrix; aborts the close (busy only) when the player is within obj+0x8C of the swing plane
  (decomp :98-133, disasm 0x135d90-0x135e30); otherwise toggles: closed family →
  `ext+0x58 = ±1.0 · ext+0x5C`, SetState(8); open family → `ext+0x58 = ±1.0` (by t > 0.5),
  SetState(0x10);
- **kind 1..2 (sliding)**: NO anti-crush, NO ext+0x5C multiply [M, disasm 0x135d4c-0x135d88]:
  `(state & 0xC) != 0` → `ext+0x58 = −1.0`, SetState(0x10); else `ext+0x58 = +1.0`,
  SetState(8) — the ±1.0 is stored directly (`swc1 f0,88(s0)` @0x135e88);
- kind >2 or <0: cooldown only, no state change;
- tail: ext+0x6C = 2, ext+0x70 |= 0x10 (0x135ee8-0x135ef8).

Door_SetState(obj, state) [M]: open sound ext+0x60 when 0x08 newly set; close sound ext+0x62 when
0x10 newly set; broadcast `switch_channels[ext+0x66] = ((OLD obj+0xF4 & 0xC) != 0)` when the open
channel is defined (evaluated **before** the state store — one transition delayed; timestamped
when on); then `obj+0xF4 = state`; **ext+0x72 = 0** (timer reset on every state change).

Door_Interp(obj, ext, advance) [M]: `ext+0x70 & 1` selects Spline_Interp vs KeyFrame_Interp over
the m_paths keys (ext+0x0C, ext+0x6E frames); when advance: `t += ext+0x54 · ext+0x58 ·
FRAME_RATE_MUL`; sliding (kind 1..2): t < 0 → t = 0 + SetState(2); t > 1 → t = 1 + SetState(4);
swing (kind 0): state 8 and t outside [0,1] → clamp + SetState(4); state 0x10 and
`|0.5 − t| <= ext+0x54 · FRAME_RATE_MUL` → t = 0.5 + SetState(2) — **swing doors come to rest at
t = 0.5, sliding doors at t = 1.0**; both end OPEN at state 0x04. Recomputes the matrix via
Spline_Interp at `t · frames` after any state change.

## Door_ReactToHit (0x135980) — the collide function

Runs from `Collide_Update` (0x1ea398 decomp :59-63) via `control_funcs[0x1C·0xC+4]`, only when
ext+0x6C == 0 [M]. Scans the obj+0xD0 contact list for the first class-3 (player) contact, else
the first class-2 (drone) contact passing `(droneExt+0x4F8 & 0x600) == 0 && (droneExt+0x4F8 &
0x100) != 0 && droneExt+0xAC > 0 && !(obj+0xFE & 1)` [M]. Then:
- kind 0 (swing), animating (`state & 0x18`): the touched object's position in door-local space,
  projected against ext+0x5C (and flipped when t > 0.5): closing into something behind the swing →
  `ext+0x58 = −ext+0x58` + SetState(8) (reopen); opening into something ahead → reverse +
  SetState(0x10) [M];
- kind 2 (sliding), closing (`state & 0x10`): `ext+0x58 = −ext+0x58` + SetState(8) + Door_Interp(0)
  — the sliding anti-crush reversal [M];
- kind 1: **no reversal** [M];
- tail: Door_Interp(0), ext+0x6C = 2, busy.

## Doors_Calc (0x135fe0) — group sync

Every frame [M]: `memset(DoorGroupsStates, 0, 0xCA)`; for each ext in TheDoors with group
ext+0x6A ≠ 0, every OTHER same-group ext that is not busy gets the leader's ext+0x58 (dir),
ext+0x50 (t), **ext+0x72 (quiet timer)** copied, flags |= 0x10, `obj+0xF4 := leader's state`,
Door_Interp(..., 0) (matrix only); second pass clears busy (`ext+0x70 &= ~0x10`) for every door.
Grouped doors therefore share their auto-close timer [M].

## Small functions [M]

Door_DoMsg (0x135f18): one-shot per +0x74 latch — if the lock channel is not blocking and the
(group or own) state ≠ 4, binds label 0x200000b (the "won't open" message). Door_IsClosed
(0x1367b0): kind 1 → true; group → `DoorGroupsStates[g] == 2`; else `obj+0xF4 == 2`.
Door_IsLocked (0x136810): lock channel defined and `ch[lock] == 0`. Door_Link2Portal (0x136788):
`(obj+0xFD >> 3) & 1`. Door_Init (0x136848): list + DoorGroupsStates init.

## Snapshot checks (ee_start_07000001.bin, level start) — all [L]

`TheDoors` @0x316f70 → 5 doors:

| ext | obj | flags +0x70 | state obj+0xF4 | kind obj+0xF6 | group | openCh/lockCh | sounds o/c/l | t / dir / spd | radius | pos |
|---|---|---|---|---|---|---|---|---|---|---|
| 0x01fbcd90 | 0x01fbcc80 | 0x0003 | 0x0002 | 2 | 0 | 0/0 | 23/24/65535 | 0/0/0 | 1.564 | (7.88, 6.59, 1.94) |
| 0x01fbcbf0 | 0x01fbcae0 | 0x0002 | 0x0002 | 2 | 0 | 0/0 | 23/24/65535 | 0/0/0 | 1.587 | (6.99, 6.60, 11.13) |
| 0x019177c0 | 0x019176b0 | 0x0003 | 0x0002 | 2 | 0 | 0/0 | 23/24/65535 | 0/0/0 | 1.564 | (−14.05, 6.58, 13.98) |
| 0x019071f0 | 0x019070e0 | 0x0022 | 0x0002 | 2 | 10 | 0/0 | 23/24/65535 | 0/0/0 | 1.169 | (−33.08, 7.26, 9.00) |
| 0x01907050 | 0x01906f40 | 0x0022 | 0x0002 | 2 | 10 | 0/0 | 23/24/65535 | 0/0/0 | 1.189 | (−33.06, 6.01, 9.00) |

`FRAME_RATE_INT` @0x30d0cc = 60 [L]. Player obj+0xF0 = 0x890, drone 0x8B0 (bit 0x10 SET —
invisible to the auto-close scan) [L]. Cross-checked against 07000005's snapshot: 5 doors
(ext 0x016e2180 … 0x016e1b00), four sliding with **lockCh = 200** and flags 0x0002, one swing
(kind 0) with flags 0x0003 [L] — the channel-200 lock readers of U-044.

## Disc checks (07000001.bin, kind-1 map 01000093, placements block 0x1A) — all [D]

| placement | name | pos | p0 | p1 | p5/p6/p7 | p8 | p9 | flags p8\|p9 | auto-close |
|---|---|---|---|---|---|---|---|---|---|
| #13 | SlidingDoor | (7.89, 5.35, 2.87) | 2 | 0 | 23/24/65535 | 1 | 2 | 0x0003 | **YES** |
| #14 | SlidingDoor1 | (7.92, 5.34, 11.14) | 2 | 0 | 23/24/65535 | 0 | 2 | 0x0002 | **YES** |
| #69 | SlidingDoor2 | (−14.06, 5.35, 13.05) | 2 | 0 | 23/24/65535 | 1 | 2 | 0x0003 | **YES** |
| #224 | gallery_secretdoor | (−33.08, 7.26, 9.00) | 2 | 10 | 23/24/65535 | 0 | 34 | 0x0022 | no (keep-open) |
| #225 | gallery_secretdoor | (−33.06, 6.01, 9.00) | 2 | 10 | 23/24/65535 | 0 | 34 | 0x0022 | no (keep-open) |

All five: p2 = p3 = 0 (no open/lock channel), p4 = 1 (portal-linked). Live snapshot ↔ disc
agreement on four fields per door (flags, state, group, positions) [L vs D].

## The user's report vs the code

The user says the 07000001 sliding doors do NOT auto-close in the real game. The game's own code
and data say they do: the auto-close branch (0x136624) is taken for placements 13/14/69 (flags
0x02/0x03, keep-open clear) and the two snapshot/disc sources agree the running doors carry those
flags. WORKLOG A-12c reached the same conclusion; this pass re-verified it from the decomp, the
full Door_Update disassembly, the ELF control_funcs table, the live DOORINFO records and the disc
params. What the earlier passes had NOT measured (and what could still reconcile the report):

- the candidate filter `obj+0xF0 & 0x10`: on 07000001 the player is invisible to the scan, so the
  door closes 6 s after fully opening even with Bond standing beside it — **more** auto-closing,
  not less (the only protection is Door_ReactToHit's reversal when he is inside the closing path);
- grouped doors (224/225, group 10) share the timer; ungrouped doors each time their own;
- every state change resets ext+0x72, so the 6 s count starts when the door *finishes* opening.

A PCSX2 trace (press E on placement 13, stand still 8 s, then walk away 8 s, dump ext+0x72 and
state) would settle it. U-046 remains the ledger entry; the port-check items for door.rs are in
docs/atlas/trigger-port-check.md.

## Not verified here

- `build_LinkDoors2Portals` (0x1c8ac8, block 0x21) internals — only its effect (obj+0xFD bit 3)
  via Door_Link2Portal/Door_Create [S].
- Who writes droneExt+0x4F8 / +0xAC (the ReactToHit drone filter inputs) [S].
- The ext+0x76 one-sided direction memory (exact write/read pairing inside Door_Activate) beyond
  the structure above [M-partial].
- Whether any code changes obj+0xF0 bit 0x10 on the player at runtime (the scan-visibility bit) —
  measured at level start only [L].
