# SCRIPTPLAYER_tag (the script-player instance, 0x3C bytes at obj+0x110; class 0x1B)

Proof levels: see [README.md](README.md). Instance size 0x3C from `control_create_object(0x3c, …)` in
`SP_CreateScriptPlayer` 0x1db9a8 [M 001db9a8_SP_CreateScriptPlayer…c:19]; class byte `obj+0xFF = 0x1B` [M :71].
Placement type 0xD9 → `SP_CreateScriptPlayer(pos, rot, level_tag, 0, 0, 0)` [M 0x1d04f0:316-321]. Level:
ee_start_05 (07000005 at start) — 8 class-0x1B objects [L].

The whole block is memset 0 by `control_create_object`; `SP_CreateScriptPlayer` then writes every field except
+0x18 (accumulated damage) and +0x2C (activation latch), which stay 0 from the memset. `SP_Update` and most
helpers load the instance from `obj+0xE0` (`iVar2 = *(int *)(iVar11 + 0xe0)` [M 0x1dbda8:56]); `SP_Activate`
alone works off **`obj+0xE4`** [M 0x1dc338:42 — see the entity section]. `SP_Hit` is the only function whose
mangled signature takes the tag directly (`param_offsets.py SCRIPTPLAYER_tag` → offsets 0x2a/0x2e/0x30/0x37).

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | ptr | parent obj: a GT (class 0x36) or Switch (class 0x39) that `SP_Activate` dispatches to — the FuseBox and Switch SPs point at their owner object; 0 from parsemap, Copter, MP and ResetMap | w 0 [M 0x1db9a8:63 (arg 4; parsemap passes 0)], `FuseBox_Create` 0x1b0448:30 (the box), `Switch_Create` 0x1adcc0:45 (the switch); r `SP_Activate` [M 0x1dc338:46-68 (`iVar3 = *piVar2` → class 0x36 `GT_Activate`, 0x39 facing-dot + `Switch_Activate`)] |
| +0x04 | ptr | create/event callback `(fn)(obj_tag*, ushort, void*)` — handed to `Script_Load` as `SCRIPTINFO+0x84`, called by script event 0xF; zeroed after the first `SP_LoadScript` | w 0x1db9a8:64 (arg 5), zeroed [M 0x1dbc08:40]; r [M 0x1dbc08:33 (`Script_Load(…, *(iVar6+4), *(iVar6+8))`)], event 0xF via `SCRIPTINFO+0x84` [M 0x1d94c0:172-176]. FuseBox passes 0x1b0970 (`FuseBox_Register`) [M 0x1b0448:30] |
| +0x08 | ptr | callback data → `SCRIPTINFO+0x88` | w 0x1db9a8:65 (arg 6); r [M 0x1dbc08:33] |
| +0x0C | ptr | SCRIPTINFO slot A (null at create) | w 0 [M 0x1db9a8:69], filled by `SP_LoadScript` [M 0x1dbc08:34]; r every `cur`-slot access `*(iVar2 + 0xc + idx*4)` [M 0x1dbda8:63-64,106,118,142,195,230; 0x1dbc08:30,43,47,51-53; 0x1dc900:13; 0x1dcc30:13; 0x1dc5f8:82; 0x1dcad8:20]; freed by `SP_Delete`/state 6 [M 0x1dc8b0:9-10, 0x1dbda8:204-205]; **[L-read: 8/8 SPs]** (SI0 0x16d2b80 EndLevel, 0x16dd070, 0x16de890, 0x137d210 truck, rest 0 = unloaded) |
| +0x10 | ptr | SCRIPTINFO slot B (null at create) | w 0 [M 0x1db9a8:70]; r same `cur`-slot + the "other" hash probes [M 0x1dbda8:149,181,203 — `*(iVar2 + 0x1c + 4·other)` is the hash; the slot via `load`]; freed [M 0x1dc8b0:11-12]. **[L-read: 0 on all 8 — no 07000005 SP uses two scripts]** |
| +0x14 | f32 | hit-damage threshold: when the accumulated +0x18 reaches it the script is un-paused (replayed) — the shooting-range target mechanic | w s32→f32 of param 7 [M 0x1db9a8:49-55 (`(float)uVar3` unsigned / `(float)(int)uVar3` signed)]; r [M 0x1dbda8:237 (`== 0.0` skip), :265 (`+0x14 <= +0x18` → `SP_UnPause`)]. **[L-read: 0.0 ×8 (no threshold on 05)]** |
| +0x18 | f32 | accumulated hit damage (0 at create); +0x14 subtracted on each fire | r/w [M 0x1dbda8:244 (`+= SP_GetHitDamage()`), :267 (`-= +0x14`)]. **[L-read: 0.0 ×8]** |
| +0x1C | u32 | script A hashcode (0 = none) | w param 0 [M 0x1db9a8:27-29 (`puVar2[7] = *(iVar7 + 0x2c)`); 0 if param_3 NULL]; r [M 0x1dbc08:14 (`isLoadable(+0x1C)`), :32 (`Script_Load(*(iVar6 + idx + 0x1c)…)`)], state 0 gates [M 0x1dbda8:93 (`== 0x6000994`), :99, :134 (`Script_IsDeathNIS(+0x1C)`), :246 (range 0x60007e2..0x60007ea)], load-other probes [M 0x1dbda8:149,181,203]. **[L-read: 0x6000A23 (EndLevel), 0x6000994, 0x6000952, 0x6000046 ×2, 0x6000074, 0x60008C8, 0x6000A22 (truck)]** |
| +0x20 | u32 | script B hashcode | w param 1 [M 0x1db9a8:31-34]; r via `*(iVar2 + 0x1c + 4·other)` [M 0x1dbda8:149,181,203]. **[L-read: 0x6000047 on the two looping pairs, 0 elsewhere]** |
| +0x24 | u16 | state machine: 0 wait, 1 primed, 2 playing, 3 primed B, 4 playing B, 5 done/delete, 6 looping, 7 auto | w create [M 0x1db9a8:66,75 (6), :82 (7)], `SP_LoadScript` mode dispatch [M 0x1dbc08:44 (2), :48 (6), :54 (1)], `SP_Update` [M 0x1dbda8:107 (2), :115 (2), :153 (3), :157 (5), :165 (4), :185 (1), :208 (2)]; r switch [M 0x1dbda8:83], `SP_GetState` [M 0x1dc900:12 (3/5 → true)]. **[L-read: EndLevel 1 (mode 64 preloaded), truck 2 (mode 1 auto-played), looping pairs 6 (mask 0), 0x6000994/0x6000952/0x6000074/0x60008C8 0 (mode 0, untriggered)]** |
| +0x26 | u16 (s16) | mode A (script A's play mode: 0 none, 1 auto, 8 loop, 64 preload-then-prime; read as s16) | w param 2 [M 0x1db9a8:43]; r create dispatch [M 0x1db9a8:76,79-85], `SP_LoadScript` [M 0x1dbc08:41], mode-of-current [M 0x1dbda8:117,167 (`*(short *)(iVar2 + idx*2 + 0x26)` — +0x26/+0x28 indexed)], `SP_SwitchToScript` [M 0x1dcad8:25,30]. **[L-read: 64 EndLevel, 1 truck, 8/8 looping pairs, 0 rest]** |
| +0x28 | u16 (s16) | mode B | w param 3 [M 0x1db9a8:44]; r mode-of-current [M 0x1dbda8:117,167 (indexed), 0x1dcad8:30]. **[L-read: 64 truck slot B, 8/8 looping pairs, 0 rest]** |
| +0x2A | u16 | current script index 0/1 (slot A/B) | w 0 [M 0x1db9a8:68], flip on load-other [M 0x1dbda8:150,182,206 (`(ushort)(*(short *)(iVar2 + 0x2a) == 0)`)], `SP_SwitchToScript` [M 0x1dcad8:24]; r every slot access [M 0x1dbda8:64,106,117,142,167,175,195,230; 0x1dbc08:17,30-34,39-53; 0x1dc470:16; 0x1dc900:13; 0x1dcc30:13; 0x1dc5f8:82,84,118; 0x1dca58:11; 0x1dc990:14,23; 0x1dc9e8:11,13; 0x1dcbf0:12; 0x1dcad8:15-30]. **[L-read: 0 ×8]** |
| +0x2C | u16 | activation latch — `SP_Update` ORs trigger bit 4 from it while the countdown is 0, then clears unconditionally | w 1 by `SP_UnPause` [M 0x1dca58:10] and `SP_Activate` [M 0x1dc338:82 (`piVar2[0xb]` = +0x2C)]; r+w [M 0x1dbda8:78-81 (`!= 0 && countdown == 0 → bit 4`; `= 0`)]. 0 at create (memset). **[L-read: 0 ×8]** |
| +0x2E | u16 | trigger mask (bits of `SP_Hit`/latch the SP reacts to) | w param 5 [M 0x1db9a8:46]; r trigger AND [M 0x1dbda8:82], create state gate [M 0x1db9a8:74], `SP_Activate` [M 0x1dc338:65,72-77], `SP_Hit`'s Collide_FilterBullets special case [M 0x1dc470:31 (`== 2`)]. **[L-read: 1 on EndLevel/0x6000994/0x6000952/0x6000074/0x60008C8, 0 on the looping pairs and truck]** |
| +0x30 | u16 | OUT channel — written 1 on a state-1 trigger, 0 on a state-3 trigger, otherwise rewritten with its current value; forced to 0 at create | w [M 0x1db9a8:47,72 (`switch_channels[+0x30] = 0`)], trigger write [M 0x1dbda8:221-229 (`switch_channels[uVar7] = cVar10 != 0`, cVar10 = 1 in state 1 / 0 in state 3 / the channel itself elsewhere)]; r [M 0x1dbda8:69-74,111,122,125-126,161,188,211,214,219,225-226], `SP_Activate`'s mask&1 gate [M 0x1dc338:77], `SP_Hit` bit 1 [M 0x1dc470:17-18]. **[L-read: 101 EndLevel, 67 (0x43) the 0x6000994 SP, 190, 125, 92, 0 rest]** |
| +0x32 | u16 | gate channel — `SP_Update` returns while `switch_channels[gate] == 0` | w param 9 [M 0x1db9a8:58-61]; r [M 0x1dbda8:60-62]. **[L-read: 0 ×8 (no gate on 05)]** |
| +0x34 | u8 | trigger countdown — 5 on a state-1/3 trigger; while ≠ 0 `SP_Hit` is skipped and the latch read is gated | w 0 [M 0x1db9a8:67], 5 [M 0x1dbda8:116,166]; r/w decrement [M 0x1dbda8:66,75-77]. **[L-read: 0 ×8]** |
| +0x35 | u8 | stay flag — a state-2 script that finishes deletes the SP (state 5) instead of switching to the next | w param 4 (u8) [M 0x1db9a8:45]; r [M 0x1dbda8:144], `SP_SwitchToScript` [M 0x1dcad8:28]. **[L-read: 1 on EndLevel/0x6000994/0x6000952/0x6000074/0x60008C8/truck, 0 on the looping pairs]** |
| +0x36 | u8 | disabled byte — `SP_Update` returns while set; `SP_HideObj` writes it | w [M 0x1dcbf0:10 (`param_2`)], Copter wake-up [M 0x17dbb8:95 (`SP_HideObj(sp, 0)`), set path `Copter_Create` 0x17d340:73-74 (`SP_HideObj(sp, 1)` while the hide channel is off)]; r [M 0x1dbda8:57-59]. **[L-read: 1 on the 0x16ddb40 looping pair (hidden at level start), 0 on the other 7]** |
| +0x37 | u8 | bullet-filter weapon id — when set, `SP_Hit` bit 2 (a class-5 bullet in a stream object's proxy list) fires only for bullets of that weapon | w param 8 (u8) [M 0x1db9a8:56]; r [M 0x1dc470:30 (`== 0` → unfiltered + the mask-2 filter path), :64-70 (bullet's `instance+0x44` u16 == +0x37)]. **[L-read: 0 ×8]** |
| +0x38 | u8 | loading-screen flag — armed after `GameFlow_PushState(12.0, 0xD, 0xFF)` so state 0 loads the script next tick instead of re-pushing | w [M 0x1dbda8:101]; r [M 0x1dbda8:100]. 0 at create [M 0x1db9a8:73]. **[L-read: 0 ×8]** |
| +0x39..+0x3B | — | pad (inside the 0x3C block) | no reader/writer found |

### The `obj+0xE4` handle (not instance — the entity↔SP binding)

`obj+0xE4` is **not** part of the 0x3C block; it lives in the obj_tag header. For a placement SP object it is 0
([L] all 8). It is the **SP handle on script-created entities**:

| dir | fn | proof |
|---|---|---|
| w | `Script_EntityStart` 0x1d8d78:108-110 — `*(u32*)(obj+0xe4) = *(SCRIPTINFO+0x80)` (the owner SCRIPTPLAYER_tag, stored by `Script_Load`:135) | [M] |
| w 0 | `Script_KillStream` 0x1d9860:37 (kind-1 entity kill: `*(iVar2 + 0xe4) = 0`) | [M] |
| r | `SP_Activate` 0x1dc338:42 — the only SP entry reached through the object (returns immediately when 0) | [M] |
| r | `control_delete_object` 0x133178:28 — `SP_RemoveObj(obj, *(obj+0xe4))` → `Script_RemoveObj(obj, SP->cur SCRIPTINFO)` kills the entity's stream | [M 0x133178:28, 0x1dc958:7-9, 0x1db6e0:16-19] |

`SP_Activate` therefore fires on a **class-0 script entity** (`Player_Activate` 0x1a5170:121-122 requires
`obj+0xE4 != 0 && obj+0xFF == 0`), not on the class-0x1B SP object itself; it then both dispatches the SP's
+0x00 parent and sets the +0x2C latch (mask & 4 required, out-channel gate when mask & 1) [M 0x1dc338:46-86].
`Script_EntityStart` also ORs `obj+0xFA |= 0xE4` on the entity [M 0x1d8d78:109].

## Param map (from `SP_CreateScriptPlayer` 0x1db9a8; param k = level_tag+0x2C+4k)

| param | level_tag | Instance offset | note |
|---|---|---|---|
| — | — | +0x00 / +0x04 / +0x08 | args 4/5/6 (parent obj, callback, data) — 0 from parsemap [M 0x1d04f0:318-319] |
| p0 | +0x2C | +0x1C hash A | refuse unless p0 ∨ p1 [M 0x1db9a8:18] |
| p1 | +0x30 | +0x20 hash B | |
| p2 | +0x34 | +0x26 mode A | |
| p3 | +0x38 | +0x28 mode B | |
| p4 | +0x3C | +0x35 stay (u8) | |
| p5 | +0x40 | +0x2E mask | 0 → state 6 [M :74-75] |
| p6 | +0x44 | +0x30 out channel | `switch_channels[out] = 0` at create [M :72] |
| p7 | +0x48 | +0x14 hit-damage threshold | s32→f32 (two-cast form) [M :49-55] |
| p8 | +0x4C | +0x37 bullet filter (u8) | |
| p9 | +0x50 | +0x32 gate channel | |
| p10 | +0x54 | → `SCRIPTINFO+0x94` scale | 0 → 1.0, else p10·0.01, applied to both loaded slots [M :87-101]; **[L]** EndLevel SI+0x94 raw 0x3F7FFFFF (0.99999994) vs truck 0x3F800000 — p10 = 100 run through the EE's multiply rounds one ulp low; the formula is p10·0.01 either way |

Create order [M 0x1db9a8:18-101]: refuse without p0/p1 → create 0x3C object, class 0x1B → fields → latch/lazy
fields 0 → `switch_channels[out] = 0` → state 0 → mask 0 → state 6 → mode A: 1 → state 7; ≠0 → `SP_LoadScript`
(mode dispatch inside may re-write state 1/2/6) → p10 scale onto both slots that are loaded by then.

## `SP_Create` 0x1dc7e0 — the runtime wrapper

`SP_Create(VECTOR* pos, VECTOR rot, u32 hashA, u32 hashB, u32 mode, u32 stay, obj_tag* parent, fn, void* data,
u32 mask)` builds a 0xB0 synthetic level_tag on its stack (memset 0 [M 0x1dc7e0:67]) with +0x2C = hashA,
+0x30 = hashB, +0x34 = mode, +0x38 = **mode again (both slots get the same mode)** [M :72-75 — the decompiler
shows param_5 stored twice; frame partially garbled (unaff_s0..s8 spills land above the tag), cite with care],
+0x3C = stay, +0x40 = mask [M :76,68], then tail-calls `SP_CreateScriptPlayer(pos, rot, tag, parent, fn, data)`
[M :77-78]. `rot == 0` → the zero-VECTOR constant 0x2d6200 [M :69-71]. Call sites:

| caller | args | proof |
|---|---|---|
| `FuseBox_Create` 0x1b0448:29-30 | (param 0 hash, 0, mode 0x40, stay 0, box obj, `FuseBox_Register` 0x1b0970); data and mask decompiler-dropped trailing args (0) | [M] |
| `Switch_Create` 0x1adcc0:44-45 | (param 1 hash, 0, mode 8, stay 0, switch obj, 0) | [M] |
| `Copter_Create` 0x17d340:68-70 | (param 4 hash, param 5 hash, mode 8, stay 0, 0, callback 0x17e530) | [M] |
| `MP_CreateObject` 0x185de8:187-189 | (MPOBJ+0x10, MPOBJ+0x28, MPOBJ+0x18, MPOBJ+0x20, 0, 0) | [M] |
| `ResetMap_Load` 0x1d6360:97-99 | (level hash, 0, mode 1, stay 1, 0, 0) | [M] |

## `SP_Update` 0x1dbda8 — state machine (every field touch)

Early returns: +0x36 disabled [M :57-59]; gate +0x32 on and `switch_channels[gate] == 0` [M :60-62].
Then `Script_Update(cur)` [M :63-64]; trigger bits: `SP_Hit(cur)` only while countdown == 0 [M :66-68],
cVar10 = out-channel value [M :69-74], countdown−− [M :75-77], latch → bit 4 (countdown-gated) and latch = 0
[M :78-81]; `bVar4 = bits & mask` [M :82]. Out-channel write after the switch, only when bVar4:
`switch_channels[out] = cVar10 != 0` [M :221-229] — state 1 sets cVar10 = 1 [M :119], state 3 sets 0 [M :169],
all other states rewrite the current value. Tail (cur SCRIPTINFO loaded, +0x9C ≠ 0, +0x14 ≠ 0):
`+0x18 += SP_GetHitDamage(…)`; the 0x60007e2..0x60007ea hashes also log `PlrStat_LogShotHitScenery(0)` when the
damage came from a player bullet [M :230-268]; `+0x14 <= +0x18` → `SP_UnPause` + `+0x18 -= +0x14`.

| state | on trigger (bVar4) | otherwise | out write |
|---|---|---|---|
| 0 wait | SP: health/mission gate fails (`glb_blokes+0x894 ≤ 0` ∨ `Mission_Status() != 1`) or (hash 0x6000994 ∧ `Get_Music_Event(2)`) → state 5 [M :86-98]; `isLoadable(A)` ∧ !+0x38 → `GameFlow_PushState(12.0, 0xD, 0xFF)`, +0x38 = 1, return [M :99-104]; else `SP_LoadScript`, `Script_Play(cur, 0)`, state 2 [M :105-107] | falls through (no trigger = nothing) | keep |
| 1 primed | state 2, countdown 5, `Script_Play(cur, mode[cur] == 8)` [M :113-123] | — | **1** |
| 2 playing | — | mask&1 ∧ out on → `SP_UnPause` [M :125-128]; skip action `Input_Action(-1, 0x19, 1)` → `Script_FFwd(cur, 1)` unless `Script_IsDeathNIS(A)` [M :129-141]; when !`Script_IsPlaying`: stay → state 5, else load other if its hash ≠ 0 → state 3 [M :142-159] | keep |
| 3 primed B | state 4, countdown 5, `Script_Play(cur, mode[cur] == 8)` [M :163-171] | — | **0** |
| 4 playing B | — | when !playing: load other if hash ≠ 0 → state 1 [M :173-188] | keep |
| 5 done | — | `obj+0xFE ∨ 1` (the delete pass runs `SP_Delete` → `Script_Free` both slots) [M :190-192, 0x1dc8b0:9-13] | keep |
| 6 looping | cur SI+0x9C = 0 [M :195-198]; other hash ≠ 0 → `Script_Free(slot A)`, slot A = 0, flip idx, `SP_LoadScript`, state 2 [M :199-210] | — | keep |
| 7 auto | — | `SP_LoadScript` every tick (mode-1 scripts play immediately → state 2) [M :217-219] | keep |

`SP_LoadScript` 0x1dbc08: `isLoadable(A)` → stream the NIS off disc (`LoadableLoad` + SFX suspend) [M :14-28];
slot empty → `Script_Load(hash, obj+0x30, obj+0x50, 0, SP, +0x04, +0x08)` else `Script_Set2Start` [M :30-38];
+0x04 callback = 0 after use [M :40]; mode dispatch 1 → play(0)/state 2, 8 → play(1)/state 6, else
play+update+stop/state 1; one more `Script_Update` [M :41-56]. `SP_UnPause` 0x1dca58: latch = 1, cur+0x9C = 0
[M :10-14]. `SP_GetState` 0x1dc900: states 3/5 → true(2), else `cur+0x9C == 0` [M :12-16]. `SP_Hit` 0x1dc470:
bit 1 out channel on, bit 2 a class-5 bullet in any stream object's HITDATA proxy chain (obj+0xD0 walk,
entry+0x58 objects; filtered to the +0x37 weapon via the bullet instance's +0x44 u16; mask == 2 also runs
`Collide_FilterBullets(…, 0x106)`), bit 8 class-3 neighbour, bit 0x10 class-2 neighbour [M :15-87].
`SP_SwitchToScript` 0x1dcad8 (runtime swap): frees the OTHER slot, zeroes its hash, flips idx, writes the new
mode at +0x26+idx·2 and the callback at +0x04, `SP_LoadScript`; stay 0 → state 3 (2 when the new mode is 1)
[M :12-35]. `SP_SetPos`/`SP_SetPosRot` 0x1dc990/0x1dc9e8 write the position into the current SCRIPTINFO's
matrix row 3 (+0x30..+0x3C) and call `Script_SetPosRot` [M — the Ghidra print of both is garbled (single-arg
`Mat_Copy`), cite the +0x30 stores 0x1dc990:19-22].

## The entity `Script_CreateEntity` creates (0x1d8968)

`Script_CreateEntity(SSTREAM* stream, u32 gfx_hashcode, u8 obj_type)` — called only from
`Script_EntityStart` 0x1d8d78:88, itself only from `Script_Run` op 7 [M 0x1d8548:53]. The created thing is an
**obj_tag** — plain class 0 with NO instance for unmapped types (`control_create_object(0,0,0,0)` [M :170-173]);
mapped types go through the real creators, so the "instance" is that class's own:

| obj_type | creator | resulting class | note |
|---|---|---|---|
| 0x20 | `Break_Create` | 0x20 | [M :120-123] |
| 0x22 | `Ladder_Create` | **0x2D** | [M :127-130] — the script type 0x22 makes a ladder |
| 0x29 | `Switch_Create` | 0x39 | [M :113-117] |
| 0xE1 | `Destroy_Create` | 0xE1 | [M :106-110] |
| 0xEA | `Trigger_TouchOnce` | **0x1D** | [M :148-152]; TouchOnce forces the tag's param 0 = 1 [M 0x1b2b88:14] before `Trigger_Create` (class 0x1D, 0x40-byte instance [M 0x1b1720:52,93]) — 0xEA/0xEB are script *obj types*, not class bytes |
| 0xEB | `Trigger_Touch` | **0x1D** | [M :141-146] (param 0 left 0) |
| 0xF0 | `Pickup_Create(0x2d6200, 0x2d6200, 0, glist, kind, id, rounds, switch)` | 0x2F | [M :156-161]; kind/id/rounds/switch = the u32 params at 0x349974/78/7C/80 written by `Script_EntityStart` [M 0x1d8d78:80-87] — the fake level_tag 0x349948's params (+0x2C+4k) [M] |
| 0xF2 | `Emitter_CreatePlist(identity matrix, 0x349948)` | 0xF2 | [M :163-166] |
| 0xF5 | `Drone_AIVolume_Create` | 0x3E | [M :56-63 + :132-137] |
| other | plain `control_create_object(0, …)` + `hashtable_set_object_to_entity_gfx` | 0 | [M :170-173] |

Reuse: an existing `stream+0x10` handle whose class byte (`obj+0xFF`) differs from the mapped class is deleted
(`obj+0xFE ∨ 1`) and the handle nulled; same class → re-glist (`hashtable_set_object_to_entity_gfx`) and reuse
[M :92-101]. Every path ends in `control_unlink_object` [M :175]. `Script_EntityStart` then sets the SSTREAM
fields (tag = clock+1 when the handle was empty, parent, kind 1, spline flag 0x2) [M 0x1d8d78:90-107] and wires
the entity: `obj+0xE4 = owner SP`, `obj+0xFA ∨ 0xE4`, `Script_Interp`, `Control_BuildWorldSph`,
`build_LinkToRoom`, rotation +0x50 from the matrix, pos +0x30/+0x40 = matrix row 3 [M :108-118].

## [L-read] spot checks (ee_start_05.bin; walk head 0x2705a0 → first u32 @0x2705b4 → next obj+0x14; filter obj+0xFF == 0x1B; instance = u32 at obj+0xE0)

1. **+0x24 state matches the create-time load beat**: EndLevel obj 0x16d3650 → SP 0x16d3760: state 1, mode A 64,
   mask 1, out 101, stay 1 (mode 64 → `SP_LoadScript` at create → primed). Truck obj 0x16e8ad0 → SP 0x16e8be0:
   state 2, mode A 1 (auto-played at create → playing). Looping pairs 0x16ddb40 / 0x16df360: state 6, modes 8/8,
   mask 0 (mask 0 → state 6). Mode-0 SPs (0x16d6ad0, 0x16d7e10, 0x16e2d60, 0x16e8970): state 0, mask 1.
2. **+0x36 disabled byte**: 0x16ddb40's SP reads 1 (hidden at level start), the twin 0x16df360 reads 0 — the
   only 1 of the eight; `SP_Update` returns on it [M 0x1dbda8:57].
3. **+0x30 out channel + `switch_channels[out] = 0` at create**: 101 / 67 / 190 / 125 / 92 on the mask-1 SPs, 0
   elsewhere, matching `SP_CreateScriptPlayer:72`.
4. **obj+0xE4 = 0 on all 8 SP objects** — the handle exists only on script entities; `SP_Activate` is reached
   through `Player_Activate`'s class-0 arm only (0x1a5170:121-122).
5. **+0x1C hashes**: 0x6000A23 / 0x6000994 / 0x6000952 / 0x6000046+0x6000047 ×2 / 0x6000074 / 0x60008C8 /
   0x6000A22 — and the EndLevel's SI0 0x16d2b80 has +0x98 = 0x6000A23, +0xA0 = 2.0, +0xAB0 = 310, +0xAAA = 2,
   play 0x0 (primed, stopped); the truck's SI0 0x137d210 has +0x80 = 0x16e8be0 (**the owner back-pointer**),
   +0x98 = 0x6000A22, +0xA0 = 3.0, +0xAB0 = 5895, +0xAAA = 22, play 0x1, SSTREAM[2] tag 1 / kind 1 / parent 1,
   SSTREAM[1] kind 2 (the truck anim).
6. **+0x14/+0x18 hit damage = 0.0 ×8** and **+0x37 = 0, +0x32 = 0, +0x34 = 0, +0x2A = 0, +0x2C = 0, +0x38 = 0** —
   no 07000005 SP uses the damage, gate, filter or loading paths.

## Live checks

Walk as in the spot checks (8 class-0x1B objects on 07000005). Instance = u32 at obj+0xE0.

1. **+0x2C latch + mask bit 4 → out channel.** Poke the EndLevel SP (0x16d3760): `+0x2C = 1`. Its mask is 1
   (no bit 4) so nothing happens — instead poke **mask +0x2E = 5** (bits 1+4) and `+0x2C = 1`: on the next tick
   `SP_Update` consumes bit 4, state 1 → 2 (countdown 5) and `switch_channels[101]` flips to 1 (watch
   `switch_channels` 0x26fc90+101). Reverting +0x2E to 1 restores the game behaviour.
2. **+0x36 disabled byte gates the whole update.** Poke 0x16df360's SP +0x36 = 1 (its twin's value): the
   looping script freezes (SI 0x16de890's +0xA0 clock stops advancing; the pair's objects stop moving). Poke 0
   → resumes. The port has no such byte — this probe demonstrates the mismatch.
3. **+0x24 state 5 → deletion.** Poke the truck SP's +0x24 = 5 → the object disappears next tick
   (`obj+0xFE ∨ 1`; the class-0x1B count drops to 7; SP_Delete frees SI 0x137d210).
4. **+0x30 out-channel write shape.** With the EndLevel SP triggered (check 1), the channel writes exactly once
   (on the trigger tick): `watch` 0x26fc90+101 — one 0→1 transition, then state 2 rewrites the same value while
   triggered (no-op). If the channel toggles every tick, the cVar10 reading is wrong.
5. **+0x14 hit damage (needs a shooting-range level).** On a level with hash 0x60007e2..0x60007ea scripts, shoot
   the target entity and watch SP+0x18 accumulate by the bullet damage and the script replay at +0x14.

## Port mismatches (crates/nf-game/src/nis.rs `Player` @ master; report only)

The port models the SP as `nis::Player` (nis.rs:592-614: scripts/modes/stay/mask/out/gate/scale/cur/state/
activated/countdown/info/deleted) — the mapping to +0x1C/+0x20, +0x26/+0x28, +0x35, +0x2E, +0x30, +0x32,
SCRIPTINFO slots, +0x2A, +0x24, +0x2C, +0x34 is correct, and the state table, latch consumption, countdown,
out-channel write shape, `SP_LoadScript` mode dispatch and the skip path match the decompile. Mismatches:

1. **No +0x36 disabled byte.** The game returns from `SP_Update` while it is set (0x1dbda8:57-59); `SP_HideObj`
   0x1dcbf0:10 writes it (Copter_Create 0x17d340:73-74 sets 1, Copter_Update 0x17dbb8:95 clears). The port has
   no field; the hidden looping pair on 07000005 stays quiet only because `prime()` refuses mask-0 SPs
   (nis.rs:653-656 "only mask ≠ 0 SPs are primed (the port has no disabled byte (port-check §9))"). Any mask ≠ 0
   SP hidden through the copter path would run in the port.
2. **No +0x14/+0x18 hit-damage accumulator.** The game un-pauses (replays) a script when the bullet damage on
   its stream objects reaches +0x14 and logs shot-hit-scenery for hashes 0x60007e2..0x60007ea (0x1dbda8:237-268,
   `SP_GetHitDamage` 0x1dc5f8). The shooting-range targets run on this; the port has nothing (no field, no tail).
3. **No `SP_Hit` trigger bits 2/8/0x10 and no +0x37 bullet filter.** `SP_Hit` 0x1dc470's proxy-walk bits are not
   modelled — the port says so itself (nis.rs:667-668 "Trigger flags: only the channel bit (1) and activation
   (4); bullets and touching objects (2, 8, 0x10, `SP_Hit`) are not modelled yet").
4. **No +0x38 loading-screen flag / `GameFlow_PushState(12.0, 0xD, 0xFF)`.** The game arms it when a state-0
   script is disc-streamable (0x1dbda8:99-104); the port loads from the archive and never defers (benign without
   streaming, but state 0's isLoadable beat is absent).
5. **No +0x00 parent dispatch in activation.** The game's `SP_Activate` forwards to a GT (0x36) or, with a
   facing-dot test, to a Switch (0x39) before latching (0x1dc338:46-68); the port's E-activation stand-in
   (useobj.rs:178-197) latches directly and documents the gap ("in the game `Player_Activate` dispatches to
   `SP_Activate` only for class-0 objects while 0xD9 placements create class-0x1B objects… the E path cannot
   reach the Tower SPs either" — useobj.rs:22-27, which matches this survey: the only game caller is
   `Player_Activate` 0x1a5170:122, gated on obj+0xE4 ≠ 0 ∧ class 0).
6. **State 0's gates absent.** The MP health/mission gate and the 0x6000994 + `Get_Music_Event(2)` → state 5
   delete (0x1dbda8:86-98) are not ported; on 07000005 the 0x6000994 SP (obj 0x16d6ad0, out ch 67) would survive
   in the port where the game deletes it when music event 2 fires.
7. **+0x04 create callback not modelled.** `SP_Create`'s callback (FuseBox_Register 0x1b0970, Copter 0x17e530)
   is how the fusebox files its wire entities and the copter registers path entities (`SP_CreateScriptPlayer`
   → `Script_Load` → `SCRIPTINFO+0x84`, event 0xF); the port's `Scripts` has no callback hook.
8. Cosmetic, not a mismatch: the port's scale `g(10) as f32 * 0.01` (nis.rs:612) gives exactly 1.0 for p10 = 100
   where the game's EE multiply leaves 0x3F7FFFFF [L] — one ulp, no visible effect.

`ss.rs` itself makes no script-player claims (the 0xEA/0xEB re-class wording lives in `triggers.rs:45`
"Made by a script (Script_CreateEntity 0xEA / 0xEB)"; the decompile reads: 0xEA/0xEB are script obj types that
create **class-0x1D** triggers via `Trigger_TouchOnce`/`Trigger_Touch` — see the entity table above; the port's
trigger follow-the-entity behaviour stands in for the stream binding).

## Atlas cross-notes (docs/atlas/script-player.md, verified)

- Everything cited from it reproduced, except: its SSTREAM table row says streams start at `SCRIPTINFO+0xB0` —
  the decompile puts **SSTREAM[k] at +0xA8 + 0x28k** (`Script_Load` 0x1d8248:150 `__s = iVar5 + 0xa8`;
  `Script_Update` 0x1da998:38 `* 0x28 + iVar8 + 0xb0` = stream k's clock; `Script_RemoveObj` 0x1db6e0:16-18 kind
  byte at +0xcc = 0xa8+0x24). The atlas's own snapshot note ("`SCRIPTINFO+0xB0` = SSTREAM[0]+0x08") agrees with
  +0xA8; the table row is 8 high. [L] truck SI 0x137d210: stream 1 kind 2 @ +0xD0, stream 2 kind 1 @ +0xF8.
- Its NIS-def layout "n × {u16, u16, u32} at def+8k": the parse (`Script_Load` 0x1d8248:100-113) starts the
  table at **def+4** (entry k: u16 @ 4+8k, u16 @ 6+8k, u32 @ 8+8k; def+0 = the hash).
- Its "SP_Activate (called by triggers/sensors on the SP or its script objects)": the only caller in the
  decompile is `Player_Activate` 0x1a5170:122 (class-0, obj+0xE4-carrying objects). No trigger/sensor calls it.
- Its EndLevel snapshot row says "+0x94 … 1.0 (p10 = 0)" [L] — the raw word is 0x3F7FFFFF (0.99999994), i.e.
  p10 = 100 through the EE's multiply (truck p10 = 0 gives exact 0x3F800000). Cosmetic, but the "p10 = 0" claim
  is wrong.
