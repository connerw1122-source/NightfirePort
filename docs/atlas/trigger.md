# Struct 5 — the trigger (`control_create_object` ext, class 0x1D)

Allocation [M]: `Trigger_Create` (0x1b1720) calls `control_create_object(0x40, pos, pos2, 0)`
(jal 0x1344e0 @0x1b175c) — a 0x40-byte ext whose pointer is stored at `obj+0xE0`
(`lw s0,224(s2)` @0x1b1768) and zeroed (`memset` @0x1b1780). The object's class byte
`obj+0xFF` = 0x1D @0x1b1870-78. `obj+0xD8` = glist (0x1b1874); `obj+0xF0` = `*(u32*)(glist+0x44)`
(glist flags, 0x1b1884/0x1b1894). Update registration: `control_funcs` @0x26f890, entry 0x1D
@0x26f9ec = {0x1b1a78, 0, 0} — Trigger_Update is the per-frame update, there is **no** collide
function [M, ELF bytes].

Per-type create wrappers all do the same dance [M]: `amemcpy(TmpParam, level_tag+0x2C, 0x84)`
(TmpParam @0x319b88, 0x84 = the 33 param slots), overwrite some param slots, call
`Trigger_Create`. "param k" below = `level_tag+0x2C+4k` = placement param k
(docs/atlas/README.md); TmpParam[k] = original param k.

## Field table (ext, 0x40 bytes)

| off | type | name [S] | writers | readers |
|---|---|---|---|---|
| +0x00 | u32 | anim_id | Trigger_Create@0x1b1790 ← tag+0x08 | Trigger_Update (Spline_Interp arg) |
| +0x04 | f32 | anim_t | =0 @0x1b1788; Trigger_Update advances | Trigger_Update |
| +0x08 | f32 | anim_dur | Trigger_Create@0x1b1830 (frames·REC_FRAME_RATE·FRAME_RATE/(u16)tag+0x04) | Trigger_Update |
| +0x0C | u16 | anim_frames | Trigger_Create@0x1b17a4 ← (u16)tag+0x02 − 1 (0 when tag+0x02==0) | Trigger_Update; obj+0xFA \|= 4 when ≠0 @0x1b1864 |
| +0x10 | f32[3] | up_vec | Trigger_Create@0x1b18ac (View_RotTransMatrix(pos2,pos) → Mat_GetUp) — arg 2 is an orientation/target point, not a size | — |
| +0x1C | 4 B | (zero) | memset | — |
| +0x20 | u32 | out (mux/Touch output; LOAD_LEVEL: next level id) | Trigger_Create@0x1b17b4 ← (u32)tag+0x30 (param 1) | Trigger_Update cases; Trigger_Calc clear pass; Trigger_Activate; ResetMap_LevelToLoad arg |
| +0x24 | u16 | lock | Trigger_Create@0x1b1854 ← (u16)tag+0x54 (param 10) | Trigger_Update lock check (decomp 001b1a78:46-59); Trigger_Locked 0x1b2fc8 |
| +0x26 | u16 | input[0] / key gate (LL) | Trigger_Create@0x1b17bc ← tag+0x34 (param 2) | mux loops; Trigger_Activate key gate; Trigger_Hit filters; Trigger_Locked |
| +0x28 | u16 | input[1] / **gate (LL)** | Trigger_Create@0x1b17c4 ← tag+0x38 (param 3) | Trigger_Update case 4 pre-check, case 10; Trigger_Activate gate |
| +0x2A | u16 | input[2] / fallback (LL) | Trigger_Create@0x1b17cc ← tag+0x3C (param 4) | muxes; Trigger_Activate fallback |
| +0x2C | u16 | input[3] / success (LL) | Trigger_Create@0x1b17d4 ← tag+0x40 (param 5) | muxes; Trigger_Activate success |
| +0x2E | u16 | input[4] / gate when mask&1 | Trigger_Create@0x1b17dc ← tag+0x44 (param 6) | Trigger_Hit mask&1 path; muxes |
| +0x30 | u16 | input[5] | Trigger_Create@0x1b17e4 ← tag+0x48 (param 7) | muxes; Trigger_Hit |
| +0x32 | u16 | input[6] | Trigger_Create@0x1b17f0 ← tag+0x4C (param 8) | muxes; Trigger_Hit |
| +0x34 | u16 | (0 terminator) | =0 @0x1b17ec | loop stop |
| +0x36 | u16 | mask | Trigger_Create@0x1b1828 ← tag+0x50 (param 9), default 2 when 0; **LL forces 0xE** (Trigger_LoadLevelCreate) | Trigger_Hit; Trigger_Activate mask&4 |
| +0x38 | u16 | type (1,2,3,4,5,0xA,0xC,0xD; 99 = dead) | wrappers via param 0; Trigger_Activate sets 99 after a load | Trigger_Update switch; Trigger_Hit; Trigger_Activate; Trigger_Type 0x1b2fa8 |
| +0x3A | u16 | hide_flag_enable [S] | Trigger_Create@0x1b185c ← tag+0x58 (param 11) | Trigger_Update lock path (obj+0xF0 \|= 0x30) |
| +0x3C | s16 | msg_cooldown | =0 @0x1b17ec; Trigger_Activate `sh v0,60(s0)` @0x1b2428 = 3·FRAME_RATE_INT | Trigger_Update preamble (decr); Trigger_Activate (silent re-block) |

Object-side: obj+0xD8 glist, obj+0xE0 ext, obj+0xF0 glist flags (0x10 = self-delete enable;
LOAD_LEVEL clears it, 0x1b18b4-8c `and v1,v1,0xffffffef`), obj+0xFA \|= 4 when animated,
obj+0xFE \|= 1 = delete request, obj+0xFF = 0x1D [M].

## Placement param → field mapping

Dispatch [M]: `parsemap_create_dynamic_objects` (0x1d04f0) switches on the placement type byte:
0xDC→Trigger_Create, 0xE8→Trigger_LoadLevelCreate, 0xEA→Trigger_TouchOnce, 0xEB→Trigger_Touch,
0xEC→Trigger_MultiplexIn, 0xED→Trigger_MultiplexSIn, 0xEE→Trigger_MultiplexOut,
0xEF→Trigger_MultiplexOrIn, 0xF4→Trigger_MoviePlayer, 0xF9→SoundTrigger_Create,
0xFB→MusicTrigger_Create, 0x41→InverterTrigger_Create.

| placement | type → | out (+0x20) | mask (+0x36) | inputs (+0x26…) | lock (+0x24) | hide (+0x3A) |
|---|---|---|---|---|---|---|
| 0xDC raw | param 0 | param 1 | param 9 | params 2..8 | param 10 | param 11 |
| 0xEA TouchOnce 0x1b2b88 | 1 | p0 | p1 | p2..p7 (k2..k7) | **p7** | **p8** |
| 0xEB Touch 0x1b2aa8 | 2 | p0 | p1 | p2..p7 | **p6** | **p7** |
| 0xEC MuxIn 0x1b2c68 | 3 | p0 | raw p9 | p1..p6 (+raw p8 as 7th) | raw p10 | raw p11 |
| 0xED MuxSIn 0x1b2e08 | 4 | p0 | raw p9 | p1..p6 (+p8); ext+0x2C = 4-frame window flag | raw p10 | raw p11 |
| 0xEE MuxOut 0x1b2ed8 | 5 | p0 | raw p9 | p1..p6 (+p8) = outputs written | raw p10 | raw p11 |
| 0xEF MuxOrIn 0x1b2d38 | 0xC | p0 | raw p9 | p1..p6 (+p8) | raw p10 | raw p11 |
| 0xE8 LoadLevel 0x1b2950 | 10 (forced) | next level = **p0** | **forced 0xE** | key gate = **p1** (+0x26), gate = **p2** (+0x28), fallback = **p3** (+0x2A), success = **p4** (+0x2C) | raw p10 | raw p11 |
| 0xF4 MoviePlayer 0x1b2a08 | 0xD | p0 (movie id) | p1 | p2 (auto-play channel) | raw p10 | raw p11 |

LoadLevel wrapper disasm [M]: `amemcpy(TmpParam, tag+0x2C, 132)` then tag+0x2C := 10 (0x1b29a0),
tag+0x30 := TmpParam[0] (0x1b29b8), tag+0x34 := TmpParam[1], tag+0x38 := TmpParam[2],
tag+0x3C := TmpParam[3], tag+0x40 := TmpParam[4], tag+0x50 := 0xE. So **ext+0x28 (the gate) =
placement param 2** — WORKLOG A-10's claim, re-verified here end-to-end [M].

The gate at byte +0x28: `Trigger_Create` stores `(u16)tag+0x38` (param 3) at ext+0x28
(0x1b17c0 `lhu v1,56(s1)` / 0x1b17c4 `sh v1,40(s0)`) [M]; the LoadLevel wrapper rewrites
param 3 := placement param 2 before the call [M]. Verified live: the 07000005 LOAD_LEVEL
instances read gate 0x1D / 0 below.

Sibling structs (not this ext, for completeness) [M]: SoundTrigger_Create 0x1b2628 → class 0x40,
ext u16 map {+0x00=k0, +0x02=k4, +0x04=k1, +0x06=k2, +0x08=k3, +0x10=k5, +0x11=k7}, created only
when k1 ∉ {0,0xFFFF}, k0 ≠ 0 and k6 passes the difficulty filter vs GameState+0x28;
MusicTrigger_Create 0x1b30e0 → class 0x44, ext {+0x00 u32=k2, +0x02=k0, +0x06=k1, +0x0A=k3};
InverterTrigger_Create 0x1b3040 → class 0x53, obj+0xF4=k0, obj+0xF6=k1, ext size 4.

## Trigger_Update (0x1b1a78) — per type

Preamble [M]: ext null → return; ext+0x3C cooldown decremented if nonzero; **if type == 2 and
`ProcessTouch` (u8 @0x30cdc0) == 0 → return** (Touch triggers only run inside Trigger_Calc's
window); `Control_SetEffectType`; lock check — `ext+0x24 != 0 && switch_channels[ext+0x24] != 0`
→ if `ext+0x3A != 0` also `obj+0xF0 |= 0x30`, then return (a locked trigger is inert);
spline anim advance when anim_frames ≠ 0. Then `hit = Trigger_Hit(obj, ext)` and the type
switch (jump table @0x2fcd00 [M, audit]):

| type | fire condition / behaviour |
|---|---|
| 1 TouchOnce (0x1b1bc8) | if `ch[out]` already on **and** `obj+0xF0 & 0x10` → `obj+0xFE |= 1` (self-delete) @0x1b1d78-0x1b1d90; else falls into the type-2 body (`ch[out] |= hit != 0` + timestamp). The fall-through first guards the 0x7D special case: on level ≠ 0x07001500, out == 0x7D returns early (@0x1b1bf4-0x1b1c08) [M] |
| 2 Touch (0x1b1be4) | `ch[out] |= (hit != 0)` @0x1b1c2c + `switch_channels_time[out] = GameState+0x34` when hit; the shared tail also self-deletes when `obj+0xF0 & 0x10` and out is a Bond moment (0x65..0x6E) [M, audit] |
| 3 MuxIn (0x1b1cb8) | `ch[out] = AND(inputs)` — accumulator starts **1**, loop over ext+0x26.. (≤8, stop at 0) @0x1b1cf0-0x1b1d2c; **empty input list → ch[out] = 1**; timestamp when nonzero; Bond-moment tail as type 2 [M] |
| 4 MuxSIn (0x1b1d94) | ordered AND: acc = `ch[input0]`; for i ≥ 1 (only when ext+0x28 ≠ 0): require `ch[input_i]` **and** `time[input_{i-1}] − 4 <= time[input_i]` (@0x1b1e04-0x1b1e1c — the current input's timestamp may be up to 4 frames *older* than the previous one's, no upper bound) [M] |
| 5 MuxOut (0x1b1ed8 decomp :147-161) | copies `ch[out]` to every input slot (≤8, even 0), timestamps targets when nonzero [M] |
| 10 LoadLevel (0x1b1f88) | `gate = (u16)ext+0x28`; **gate == 0 → never activates** (`beq v0,zero,0x1b1fb4`); `ch[gate] == 0` → skip; else `Trigger_Activate(obj, glb_players)` @0x1b1fac [M] |
| 0xC MuxOrIn (0x1b1c48) | `ch[out] = OR(inputs)` (acc starts 0, `or` accumulate) + timestamp [M, audit] |
| 0xD MoviePlayer (0x1b1f24) | if `ch[input0]` on: clear it, `GameState+0x10 = out`, `GameState+0x14 = 0`, `GameFlow_PushState(0,6,0xff)`, `GameFlow_PushState(30.0,7,0xff)` [M] |
| others (0,6..9,0xB,0xE..) | no-op [M] |

## Trigger_Activate (0x1b2318) — per kind

Requires player arg non-null and player `obj+0xFF == 3` (decomp :11-19) [M]. Called from
`Player_Activate` (0x1a5170 decomp :135, the E-key path) and Trigger_Update case 10 [M].

- **cancel/alarm channels** [M]: 0x1b2388-0x1b23bc reads `ch[0x63], ch[0x62], ch[0x61], ch[0x60]`;
  any nonzero → return (no load, no channel writes).
- **type 2 (Touch)**: only when `mask & 4` (E-interactable) and out ≠ 0: `ch[out] = 1` + timestamp
  (decomp :23-32).
- **type 1 (TouchOnce)**: as type 2, then `obj+0xF0 & 0x10` → `obj+0xFE |= 1` delete (:34-49).
- **type 0xD**: mask&4; `GameState+0x10 = out`, `GameState+0x14 = 0`, `PushState(0,6,0xff)`,
  `PushState(300.0,7,0xff)` — note 300.0 here vs 30.0 in Trigger_Update [M].
- **type 10 (LoadLevel)** (:51-117):
  - key gate ext+0x26: nonzero and its channel off → if msg_cooldown (ext+0x3C) ≠ 0 silent
    return, else `Txt_BindLabel(0x2000021,0)` + `Text_AddMsg(...,0xb4)`, ext+0x3C =
    3·FRAME_RATE_INT @0x1b2428, return [M];
  - gate ext+0x28 (checked only when the key gate was 0 or passed): `ch[gate]` on → **success**
    `ch[ext+0x2C] = 1` + timestamp; off → **fallback** `ch[ext+0x2A] = 1` + timestamp — and both
    branches fall through to the load (@0x1b2498-0x1b24b8 → LAB_001b24b8) [M, audit correction:
    fallback does NOT suppress the load];
  - load [M]: `Music_Event(3,1)`, `GS_PauseGame(1)`, `ResetMap_LevelToLoad(ext+0x20, 1, 0)`
    (next level from the out field), `GameFlow_PushState(300.0,7,0xff)`, **ext+0x38 := 99**
    (trigger dead), `Player_RamSave()` if `glb_blokes`.

## Trigger_Calc (0x1b1900) and frame order

- Bond moments [M]: for ch in 0x65..0x6E, when set → `PlrStat_DoneBondMoment(0, ch)`
  (jal 0x199448 @0x1b193c, guard @0x1b1934).
- Hard-coded latches [M]: `GameState+0x0C == 0x07000005 && ch[0xA4] → ch[0x6F] = 1`
  (sb @0x1b1980); `== 0x0700000B && ch[0x34] → ch[0x2D] = 1` (sb @0x1b19a8). GameState @0x2a3768;
  +0x0C = current level id (snapshot: 0x07000005 [L]).
- Touch clear [M]: walks the object list (`control_first_object` = `DynamicObjList+0x14`,
  head @0x2705B4 [L]; next = obj+0x14), every class-0x1D object with ext+0x38 == 2 and
  ext+0x20 ≠ 0 → `ch[ext+0x20] = 0` (sb @0x1b19f4, loop 0x1b19d0-0x1b1a00).
- `ProcessTouch = 1` (sb @0x1b1a0c), walk again calling Trigger_Update for the type-2 objects,
  `ProcessTouch = 0` (sb @0x1b1a68) [M].
- Frame order [M]: `control_movement_object_handler` (0x1336f8): `Doors_Calc` @0x133740 →
  `Trigger_Calc` @0x133748 → `amemcpy(0x26fd90, 0x26fe90, 0x100)` @0x1337cc and
  `amemcpy(0x26fe90, 0x26fc90, 0x100)` @0x1337dc (prev := hold; hold := current) → the
  per-object `control_funcs[class*12]` loop (jalr @0x133830). See docs/atlas/channels.md.

## Trigger_Hit (0x1b1fc8)

- Contact bits [M]: walks obj+0xD0 collision contact nodes ({+0x04 next, +0x58 object});
  contact class byte at obj+0xFF: 3 → 0x08 (player), 2 → 0x10 (drone), 5 → 0x02 (type-5
  object), 0x4A → 0x10.
- Mask semantics (mask = ext+0x36, type = ext+0x38) [M]:
  - `mask & 0x18 && bits & 0x18 && type ∈ {1,2}`: mask == 8 → player only; mask == 0x10 →
    drone only (still runs the filter chain when drones are present, decomp :60-72); base =
    1, or `ch[ext+0x2E]` when `mask & 1`; then the **filter list AND-chain** over
    ext+0x26.. (all nonzero entries must pass): entry 0xB → needs the drone bit, 0xE → the
    player bit, anything else → a contacted drone whose `Drone_GetSkinHash()` matches the
    entry (s16).
  - `mask & 2`: a class-5 contact must match every nonzero filter entry against
    `*(*(ext+0x44))+2` (s16).
  - fallback: `(mask & bits) != 0`.
- Volume: the trigger model's collision triangles at the placement transform (no box) —
  contacts come from the collision system (docs/systems/triggers.md; writer of obj+0xD0 not
  traced here [S]).

## Snapshot checks (ee_start_05.bin, level 07000005 at start)

`control_funcs[0x1D]` @0x26f9ec = 0x001b1a78 [L]; object list head @0x2705B4 → 523 objects,
49 of class 0x1D [L]. Each trigger found by walking the list; ext read at obj+0xE0 [L]:

| ext (EE) | type +0x38 | out +0x20 | gate/key/fallback/success | mask | flags obj+0xF0 | matches placement |
|---|---|---|---|---|---|---|
| 0x16e2440 | 0xA | 0x07000006 | key 0 / **gate 0x1D** / fb 0x1F / succ 0x1D | 0xE | 0x80000 | #118 [D] |
| 0x16e25a0 | 0xA | 0x07000006 | key 0 / **gate 0** / fb 0x1E / succ 0 | 0xE | 0x80000 | #112 — the "dead" auto path [D] |
| 0x16e9d90 / 0x16e9c30 / 0x16e9ad0 / 0x16e9970 | 1 | 0x23 / 0x24 / 0x25 / 0x21 | — | 8 | 0x50 | #21/#22/#23/#24 [D] |
| 0x16e9810 | 0xC | 0x22 | in 0x23,0x24,0x25 | 0 | 0x30 | #25 [D] |
| 0x16d6820 | 1 | 0x43 | lock(+0x24) = 0xB3 | 8 | 0x50 | #301 [D] |
| 0x16d4a40 | 0xC | 0xB3 | in 0x44 | 0 | 0x30 | #304 [D] |
| 0x16d48e0 / 0x16d4780 | 1 | 0x22 / 0x44 | — | 8 | 0x50 | #305/#306 [D] |
| 0x16e9370-ish ×3 | 3 | 0xA1/0xA2/0xA3 | in 0x54+0x55 / 0x56+0x57 / 0x58+0x59 | 0 | 0x30 | #63/#62/#61 [D] |
| 0x16e90b0-ish | 0xC | 0xA4 | in 0xA3,0xA2,0xA1 | 0 | 0x30 | #270 [D] |
| 0x16e8e50-ish | 1 | 0xA4 | — | 0x18 | 0x50 | #315 [D] |
| (4 TouchOnce) | 1 | 0x7F/0x80/0x81/0x85 | lock 0xA4 / 0xA4 / 0xA4 / 0x6F | 8 | 0x50 | #311-#314 [D] |
| (MuxIn) | 3 | 0x69 | in 0x7F,0x80,0x81,0x85 | 0 | 0x30 | #80 [D] |
| (Touch) | 2 | 0x72 | — | 0x10 | 0x50 | #318 (drone-only) [D] |
| (MuxIn) | 3 | 0x65 | in 0x70,0x71,0x72 | 0 | 0x30 | #316 [D] |

Placement cross-reference [D]: placements parsed from `work/files/07000005.bin` (block 0x1A
@0x9FC154): #118 gate = param 2 = 0x1D, #112 param 2 = 0 — matches the live ext+0x28 values
above ([L] vs [D] agree on 3 fields per instance: type, out, gate/lock).

Channels at level start [L]: `switch_channels` @0x26fc90 — only ch[0x45] = 1; all of
0x21-0x25, 0x43, 0x44, 0x54-0x59, 0x65-0x73, 0x7F/0x80/0x81/0x85, 0xA1-0xA4, 0xB3 are 0;
`switch_channels_prev`/`_hold` all zero (snapshot taken before the first prev/hold memcpy of
that session — consistent with the frame order above).

## Not verified here

- Who fills obj+0xD0 contact nodes (the collision side of Trigger_Hit) [S].
- The exact semantic of glist flag 0x30 vs 0x50 on live triggers (0x10 = self-delete enable;
  0x20 = no-collision per docs/systems/triggers.md) and of ext+0x3A beyond the obj+0xF0 \|= 0x30
  write [S].
- `level_tag+0x08` anim id / Spline_Interp argument binding for animated triggers — no animated
  trigger exists in the 07000005 snapshot (ext+0x00..+0x14 all zero) [S].
- SoundTrigger/MusicTrigger/Inverter **update** semantics (only create side read here) —
  MusicTrigger_Update 0x1b31a0 and Switch_Update 0x1adea8 are covered in docs/atlas/channels.md.
