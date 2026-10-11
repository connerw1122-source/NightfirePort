# Script player atlas (SCRIPTPLAYER_tag, SCRIPTINFO, SSTREAM)

A ScriptPlayer is a 0x3C-byte `SCRIPTPLAYER_tag` on a class-0x1B control object (`obj+0xE0`), created from
placement type 0xD9 by `SP_CreateScriptPlayer` (0x1db9a8). It owns up to two loaded scripts — 0xAC0-byte
`SCRIPTINFO` instances (`Script_Load` 0x1d8248) — each holding an array of 0x28-byte `SSTREAM` stream states.
`SP_Update` (0x1dbda8) is `control_funcs[0x1B].update` (read live in `ee_start_05.bin`).

Tags: `[M]` decompile (fn@addr:line), `[L]` live snapshot (file offset = EE address), `[D]` disc, `[S]` suggested.
No disassembler in this sandbox; decompiler-dropped arguments are marked.

## Name, size, allocation site

| symbol | addr | size | meaning | tag |
|---|---|---|---|---|
| `SCRIPTPLAYER_tag` | heap (ptr at `obj+0xE0` and `obj+0xE4`) | 0x3C | the script-player state | `[M]` `control_create_object(0x3C,…)` `SP_CreateScriptPlayer@0x1db9a8:19`, `SP_Activate@0x1dc338:42` reads `obj+0xE4` |
| `SCRIPTINFO` | heap, `Mem_Malloc(0xAC0)` | 0xAC0 | one loaded script | `[M]` `Script_Load@0x1d8248:126` |
| NIS def (SCRIPTDEF) | hashtable, `Mem_Malloc(0x214)` | 0x214 | the parsed script blob (shared, refcounted by the hashtable) | `[M]` `Script_Load@0x1d8248:84-85` |
| `CAM_INFO` | heap, `Mem_Malloc(0x50)` | 0x50 | a cut-scene camera (kind-4 stream) | `[M]` `Script_CameraStart@0x1d9140:27` |
| `ScriptList` | 0x00349930 | — | DLIST of free/in-use script defs (`DList_RemoveFromInUse2Free`), not a live-script list | `[M]` `Script_Load@0x1d8248:96` `[L]` (0 on 07000005) |
| `ScriptCam` | 0x0030d100 | 4 | hashcode of the script holding the camera (0 = none) | `[M]` `Script_Play@0x1db588:26`, `Script_EventHandler@0x1d94c0:201` |

## Placement param → field mapping (type 0xD9)

`SP_CreateScriptPlayer(pos, rot, level_tag, parent_obj, fn, data)`; parsemap case 0xD9 passes
`(pos, rot, level_tag, 0, 0, 0)` `[M 0x1d04f0:316-321]`. Parameter k is read at `level_tag+0x2C+4k`
(scattered by param index, not packed positionally) `[M 0x1db9a8:18-61]`:

| param | addr | field | meaning | tag |
|---|---|---|---|---|
| — | — | +0x00 / +0x04 / +0x08 | parent obj / fn / data (args 4-6; 0 from parsemap) | `[M :63-65]` |
| — | — | +0x0C / +0x10 | SCRIPTINFO* slot A / slot B (null at create) | `[M :69-70]` |
| p7 | +0x48 | +0x14 f32 | hit-damage threshold (SP_GetHitDamage accumulator target; uint/int cast variant) | `[M :49-55]` `[L]` |
| — | — | +0x18 f32 | accumulated hit damage (0 at create) | `[M SP_Update@0x1dbda8:244]` |
| p0 | +0x2C | +0x1C u32 | script A hashcode | `[M :27-29]` `[L]` |
| p1 | +0x30 | +0x20 u32 | script B hashcode | `[M :31-34]` `[L]` |
| p2 | +0x34 | +0x26 i16 | mode A | `[M :43]` `[L]` |
| p3 | +0x38 | +0x28 i16 | mode B | `[M :44]` `[L]` |
| — | — | +0x2A u16 | current script index (0) | `[M :66]` `[L]` |
| — | — | +0x2C u16 | activation latch (0; `SP_Activate`/`SP_UnPause` set 1) | `[M :68]` |
| p5 | +0x40 | +0x2E u16 | trigger mask | `[M :46]` `[L]` |
| p6 | +0x44 | +0x30 u16 | out channel (forced `switch_channels[out] = 0` at create) | `[M :47,72]` `[L]` |
| p9 | +0x50 | +0x32 u16 | gate channel (SP_Update returns while it is 0) | `[M :61]` `[L]` |
| — | — | +0x34 u8 | trigger countdown (0; 5 on a state-1/3 trigger) | `[M SP_Update@0x1dbda8:116]` |
| p4 | +0x3C | +0x35 u8 | stay flag (state 2 end → delete instead of switching) | `[M :45]` `[L]` |
| — | — | +0x36 u8 | disabled byte (SP_Update returns while set) | `[M :67,73]` `[L]` |
| p8 | +0x4C | +0x37 u8 | bullet-filter id: when set, only bullets of that weapon id trip mask bit 2 | `[M :56]`, `SP_Hit@0x1dc470:64-70` `[L]` |
| — | — | +0x38 u8 | loading-screen flag (1 after `GameFlow_PushState(12.0, 0xD, 0xFF)`) | `[M SP_Update@0x1dbda8:100-103]` |
| p10 | +0x54 | → `SCRIPTINFO+0x94` | scale ‰ applied to a script loaded at create (`p10 == 0 → 1.0`, else `p10·0.01`) | `[M :87-100]` `[L]` |

Create-time behaviour `[M 0x1db9a8:18-101]`: refuse unless p0 or p1 is non-zero. State (+0x24) starts 0; if mask
== 0 → state 6; if mode A == 1 → state 7; else if mode A ≠ 0 → `SP_LoadScript` now (mode dispatch inside it may
leave state 1 or 2). Mode 0 keeps state 0/6 with nothing loaded.

## `SP_Update` (0x1dbda8) state machine

Per tick `[M]`: return if +0x36 (disabled) or the gate channel is off `[:57-62]`; `Script_Update(current
SCRIPTINFO)` `[:64]`; trigger bits uVar6 `[:66-81]`: bit 1 (0x1) = out channel on (via `SP_Hit`), bits from
`SP_Hit` (below) when countdown == 0, bit 4 (0x4) = activation latch (`+0x2C`, only while the countdown is 0,
cleared after the read); countdown decrements `[:75-77]`. `bVar4 = (bits & mask) != 0`.

| state | on trigger (`bVar4`) | otherwise | out-channel write (only when triggered) |
|---|---|---|---|
| 0 wait | MP gates (health/mission; script 0x6000994 + music event 2 → delete); `isLoadable(script A)` && !+0x38 → `GameFlow_PushState(12.0, 0xD, 0xFF)`, +0x38 = 1, return; else `SP_LoadScript`, `Script_Play(cur, 0)`, state 2 | — | keeps current value |
| 1 primed | state 2, countdown 5, `Script_Play(cur, mode[cur] == 8)` | — | **1** |
| 2 playing | — | mask & 1 and out ch on → `SP_UnPause`; `Input_Action(-1, 0x19)` (skip) → `Script_FFwd(cur, 1)` unless `Script_IsDeathNIS(script A)`; when !`Script_IsPlaying`: stay → state 5, else load other script if its hash ≠ 0 and state 3 | keeps current value |
| 3 primed B | state 4, countdown 5, `Script_Play(cur, mode[cur] == 8)` | — | **0** |
| 4 playing B | — | when !playing: load other if hash ≠ 0, state 1 | keeps current value |
| 5 done | — | `obj+0xFE \|= 1` (object deleted; the delete pass runs `SP_Delete`, freeing both scripts) | keeps current value |
| 6 looping | current SCRIPTINFO+0x9C = 0; if other hash ≠ 0: `Script_Free(slot A)`, slot A = 0, switch cur, `SP_LoadScript`, state 2 | — | keeps current value |
| 7 auto | — | `SP_LoadScript` every tick (mode-1 scripts: plays immediately, state 2) | keeps current value |

`SP_LoadScript` (0x1dbc08) `[M]`: `isLoadable` → stream the NIS off disc (`LoadableLoad`, SFX suspend); slot
empty → `Script_Load(hash, obj+0x30, obj+0x50, 0, SCRIPTPLAYER_tag, +0x04, +0x08)`, else `Script_Set2Start`;
then mode dispatch: 1 → `Script_Play(0)`, state 2; 8 → `Script_Play(1)`, state 6; else → play+update+stop, state 1
(primed); `Script_Update` once more. `SP_UnPause` (0x1dca58): activation = 1, `SCRIPTINFO[cur]+0x9C = 0`.
`SP_Activate` (0x1dc338, called by triggers/sensors on the SP or its script objects): dispatches a linked
parent (class 0x36 `GT_Activate`, class 0x39 `Switch_Activate` with a facing dot test); then if mask & 4:
requires — when mask & 1 — the out channel to be ON, sets +0x2C = 1 and `SCRIPTINFO[cur]+0x9C = 0` `[M
0x1dc338:65-86]`. `SP_GetState` (0x1dc900): states 3/5 → 2, else `SCRIPTINFO[cur]+0x9C == 0`.
`SP_Hit` (0x1dc470) collects the proxy-based bits: **0x1** out channel on, **0x2** a class-5 bullet in any stream
object's proxy list (`obj+0xD0` chain, filtered to weapon p8 when +0x37 ≠ 0), **0x8** a class-3 (player) proxy
neighbour, **0x10** a class-2 (drone) neighbour `[M :17-85]`.

## SCRIPTINFO field table (0xAC0 bytes)

| offset | type | name | meaning | writers | readers | tag |
|---:|:---:|---|---|---|---|:---:|
| +0x00 | MATRIX (0x40) | `matrix` | the owner's rotation+position (`View_RotTransMatrix(obj+0x50, obj+0x30)`) | `Script_Load@0x1d8248:148` | interp base | `[M]` `[L]` |
| +0x80 | ptr | `owner` | the owning SCRIPTPLAYER_tag (arg 5) — copied into created entities' `obj+0xE4` | `Script_Load:135`, `Script_EntityStart@0x1d8d78:108-110` | `SP_Activate` | `[M]` |
| +0x84 / +0x88 | ptr/ptr | `callback` / `callback_data` | event-0xF callback (args 6/7; 0 from parsemap) | `Script_Load:144,146` | `Script_EventHandler@0x1d94c0:173-175` | `[M]` |
| +0x8C | ptr | `def_cmds` | the script def's command-blob pointer | `Script_Load:136` | — | `[M]` |
| +0x94 | f32 | `scale` | key-translation scale (1.0; p10 ‰ at create) | `Script_Load:145`, `SP_CreateScriptPlayer:96-99` | interp | `[M]` `[L]` |
| +0x98 | u32 | `hash` | the script hashcode (also `ScriptCam`'s value while it holds the camera) | `Script_Load:130` | everywhere | `[M]` `[L]` |
| +0x9C | i32 | `delay` | ticks to skip (event 3 writes p0−1; `SP_UnPause`/`SP_Activate`/state 6 zero it) | `Script_EventHandler:100`, `SP_UnPause@0x1dca58:13` | `Script_Update@0x1da998:17,112-114` | `[M]` `[L]` |
| +0xA0 | f32 | `clock` | script clock, +FRAME_RATE_MUL per tick | `Script_Update:33-35`, `Script_StreamEnd@0x1db420:29` | interp/end | `[M]` `[L]` |
| +0xA4 | f32 | `fade_clock` | camera fade clock (end-of-script fade) | `Script_Update:87,107` | fade logic | `[M]` `[L]` |
| +0xAA8 | u16 | `flags` | 0x40 camera script (def word 0), 0x10 keep objects (event 0xC), 0x20 no cameras (event 0x10), 0x80 drones disabled (event 7), 0x100 player handback (event 6) | `Script_Load:147`, `Script_EventHandler:126,136,160,178` | `Script_Free@0x1da880:41-48`, interp | `[M]` `[L]` |
| +0xAAA | u16 | `nstreams` | SSTREAM count (def word 1, ≤ 0x3E) | `Script_Load:131` | loops | `[M]` `[L]` |
| +0xAAC | u16 | `def_20a` | def word (loop-related; copied verbatim) | `Script_Load:132` | — | `[M]` `[L]` |
| +0xAAE | u16 | `loop_start` | def word 2: the clock a looping script restarts at | `Script_Load:142` | `Script_StreamEnd:28-30` | `[M]` `[L]` |
| +0xAB0 | u16 | `length` | def word 3: script length | `Script_Load:133` | `Script_Run@0x1d8548:18,46`, `Script_StreamEnd:13` | `[M]` `[L]` |
| +0xAB2 | u8 | `play` | bit 0 playing, bit 1 done, bit 3 looping, bit 4 fade-out | `Script_Play@0x1db588:14-22`, `Script_Run:47`, `Script_StreamEnd:34-37`, `Script_Update:108` | `Script_IsPlaying@0x1db630:9-16` | `[M]` `[L]` |
| +0xAB3 | u8 | `stream_idx` | the stream currently being run (Script_Run processes SSTREAM[ab3]) | `Script_Update:25-42` | `Script_Run:14`, `Script_EventHandler:75-80` | `[M]` |
| +0xAB4 / +0xAB5 | u8 | `fade_len` / `fade_flags` | fade timing (0x20 set at :90, cleared :108-109) | `Script_Update` | fade | `[M]` `[L]` |
| +0xAB6 | u8 | `player_mode` | event 6 param 0 → `Player_Enable` mode | `Script_EventHandler:133` | `Script_Free:45-46` | `[M]` |
| +0xB0 | SSTREAM[nstreams] | streams | 0x28 bytes each | `Script_Load:149-167` | — | `[M]` `[L]` |

NIS def header, parsed by `Script_Load@0x1d8248:79-118` (magic word 0x1C required): word 0 → camera flag
(+0x210), word 1 → stream count, word 2 → loop_start, word 3 → length; then n × {u16 first_key, u16 last_key,
u32 cmd_len+bytes} at def+8k; then a u32 → def+0x20A; then the command-base pointer → def+0x204.

## SSTREAM field table (0x28 bytes, at SCRIPTINFO+0xA8+0x28k)

| offset | type | name | meaning | writers | readers | tag |
|---:|:---:|---|---|---|---|:---:|
| +0x00 | — | (pad) | — | memset | — | `[M]` |
| +0x08 | f32 | `clock` | stream clock, +FRAME_RATE_MUL per tick | `Script_Update@0x1da998:36-43`, `Script_Load:157` | `Script_Run:20,28` | `[M]` `[L]` |
| +0x0C | f32 | `wait` | hold-until time (op 5; the end time for op 4) | `Script_Run:39-41`, `Script_StreamEnd:10-11,26` | `Script_Run:28` | `[M]` `[L]` |
| +0x10 | u32 | `obj` | the stream's object handle (entity/anim obj pointer, CAM_INFO, sprite, light, sound id, or nested SCRIPTINFO) | `Script_EntityStart:100`, `Script_AnimStart`, `Script_CameraStart:53-55`, `Script_KillStream` | `Script_GetObj@0x1db688:14-18`, events | `[M]` `[L]` |
| +0x14 | u32 | `pc` | byte pointer into the command blob | `Script_Load:164`, `Script_Run:32,120` | `Script_Run:23-31` | `[M]` |
| +0x18 | u32 | `cmd_base` | the stream's command start (loop rewind target) | `Script_Load:160` | `Script_StreamEnd:27` | `[M]` |
| +0x1C / +0x1E | u16 | `first_key` / `last_key` | key range in the def's key array (0xFFFF/… = none) | `Script_Load:158-159` | `Script_Interp` | `[M]` `[L]` |
| +0x20 | u16 | `tag` | entity generation: `(u16)clock + 1` when the handle was empty | `Script_EntityStart:90-92` | (reuse bookkeeping) | `[M]` `[L]` |
| +0x22 | u16 | `flags` | bit 1 (0x2) spline keys, bit 0 (0x1) set by event 0xE | `Script_EntityStart:102-107`, `Script_EventHandler:170` | `Script_Interp` | `[M]` `[L]` |
| +0x24 | u8 | `kind` | 0 none, 1 entity, 2 anim, 3 sprite, 4 camera, 5 sound, 6 light, 7 subscript | `Script_EntityStart:101`, `Script_AnimStart`, `Script_CameraStart:52`, `Script_SoundStart`, … | `Script_KillStream@0x1d9860:30`, `Script_GetObj` | `[M]` `[L]` |
| +0x25 | u8 | `parent` | parent stream index, 0xFF = none | `Script_Load:162`, `Script_EntityStart:93-98` | `Script_Interp` | `[M]` `[L]` |

## `Script_Run` (0x1d8548) — the command set

Runs SSTREAM[`+0xAB3`] `:14`; interps while `clock ≤ length` `:18-22`; then executes commands while
`clock ≥ wait` `:27-31`, advancing the byte pc. Opcodes `[M :33-119]`:

| op | handler | args (bytes after the op) |
|---|---|---|
| 4 | `Script_StreamEnd` (0x1db420) | u16 end time |
| 5 | wait := u16 | u16 |
| 6 | if `length ≤ clock` → play \|= 2 (done), stop | u16 + 1 pad |
| 7 | `Script_EntityStart` (0x1d8d78): u32 hash, u8 has_parent, u8 parent, u8 spline, u8 obj_type, u8 n, n×u32 params → global buffer 0x349974+4k | 9+4n |
| 9 | `Script_EntityEnd` (0x1dafe8) | 1 |
| 0xA | `Script_AnimStart` (0x1d8f40) | u32 script, u32 skin, u32 start (+pad) |
| 0xC | `Script_AnimEnd` (0x1db328) | 1 |
| 0xD | `Script_CameraStart` (0x1d9140): u8 spline, u16 first, u16 last; CAM_INFO 0x50 {+0x40 out matrix, +0x44 first, +0x46 last, +0x48 2\|4}; `Camera_PushStates`, enable camera 4, +0xAB5 \|= 0x10, `Music_Event(0xB, hash)`, letterbox sprites on; skipped entirely (args still consumed) while flags & 0x20 | 5 |
| 0xF | `Script_CameraEnd` (0x1db348) | 1 |
| 0x12 | `Script_EventHandler` (0x1d94c0) | u8 n, u8 event, n×u32 |
| 0x13 | `Script_FadeStart` (0x1db3d0): one `BIN_GetFloat` (the fade length) | 4 |
| 0x14 | `Script_SpriteStart` (0x1d9300): 5 × `BIN_GetWord` + `BIN_GetDWord` | 14 |
| 0x15 | `Script_SpriteEnd` (0x1db3b0) | 0 |
| 0x16 | `Script_SoundStart` (0x1d8820); kind 5, the handle field holds the sound id; `Script_Interp` case 5 re-aims it at stream `tag`'s object every frame | 3 reads |
| 0x17 | `Script_SoundEnd` (0x1daf60) → `Sound_Stop` | 1 |
| 0x18 | `Script_LightStart` (0x1db1b0): 3 × `BIN_GetByte` | 3 |
| 0x1A | `Script_LightEnd` (0x1db298) | 0 |
| 0x1B | `Script_SubScriptStart` (0x1db050): `BIN_GetDWord` + 2 × `BIN_GetByte`; a nested SCRIPTINFO in the handle; killed by `Script_KillStream` case 7 → `Script_Free` | 6 |
| 0x1D | `Script_SubScriptEnd` (0x1db148) | 0 |
| 0x1E | `Script_TextStart` (0x1db2b8): `BIN_GetDWord` + `BIN_GetWord` | 6 |
| other | no-op, pc += 1 | 0 |

The args column is argument **bytes** (BIN_GetByte = 1, Word = 2, DWord/Float = 4); each command's total
footprint is op + args + 1 trailing byte (`Script_Run` 0x1d8548:120 adds 1 after the handler). This column
was corrected by fix-player: it previously mixed BIN_Get call counts with bytes (0x13 said "0 (no args
read)" while its handler reads a float; 0x14 said "6 reads" for 5 Word + 1 DWord calls), which made the
port mis-parse real scripts — see pickup-port-check §5.

`Script_StreamEnd`: wait := u16; if `clock ≤ end` and `clock < length` → **pc −= 3** (re-held on the end command)
`[M 0x1db420:10-16]`; else if looping (play & 8): wait = 0, pc = cmd_base (+0x18), script clock = loop_start
(+0xAAE), stream clock = loop_start − 1, play \|= 2 `[M :22-32]`; else play = (play & ~1) \| 2 (stopped + done)
`[M :34-37]`.

## `Script_EventHandler` (0x1d94c0) — every event id

Args: u8 n, u8 event, n×u32 params. **"Previous stream object"** (`iVar5`): stream k ≥ 1 reads
SSTREAM[k−1]'s handle when its kind is 1 or 2 (`+0x24 − 1 < 2`), i.e. literally stream index − 1; stream 0 and
non-entity/anim previous streams give NULL `[M :75-96]`. (The Rust's `i.checked_sub(1)` + kind 1\|2 filter
matches.)

| id | effect | params | tag |
|---|---|---|---|
| 3 | `SCRIPTINFO+0x9C = p0 − 1` (delay), gated by +0xAB5 & 1; then **every stream's object gets `Vec_Copy2(obj+0xC0 → obj+0x30, obj+0x40)`** (position synced to its matrix) | p0 | `[M :98-117]` |
| 4 | `Player_Disable(glb_players, 0)`; `PlarStat_LogTimerPause(0)` | — | `[M :118-121]` |
| 5 | `Drone_EnableAll(0, hash)` — freezes all drones for the cut-scene | — | `[M :122-124]` |
| 6 | flags \|= 0x100; return matrix +0x40 := prev obj's +0x90, else the player's `glb_players+0x90`; +0xAB6 = p0 | p0 mode | `[M :125-134]` |
| 7 | flags \|= 0x80 (drones stay disabled through `Script_Free`'s re-enable) | — | `[M :135-137]` |
| 8 | prev obj → `Drone_CoderCreate(obj+0xC0, obj+0x50, p0..p3, 4)` (MP coder bot) | p0-p3 | `[M :138-143]` |
| 9 | `Player_Cam2Mode(0, p0)` | p0 | `[M :144-146]` |
| 0xA | prev obj → `Break_DoBreak(obj+0xC0)` — breaks the object | — | `[M :147-151]` |
| 0xB | prev obj with `+0xDC ≠ 0` → `*(obj+0xDC+0x62) = p0` (the anim object's held-weapon byte) | p0 weapon | `[M :152-158]` |
| 0xC | flags \|= 0x10 (keep objects) | — | `[M :159-161]` |
| 0xD | p0 ≠ 0 → `switch_channels[p0] = p1`; time stamped when p1 ≠ 0 | p0 ch, p1 value | `[M :162-168]` |
| 0xE | stream flags (+0x22) \|= 1 | — | `[M :169-171]` |
| 0xF | if +0x84 callback: `callback(prev_obj, p0, +0x88)` (null from parsemap) | p0 | `[M :172-176]` |
| 0x10 | flags \|= 0x20 (no cameras) | — | `[M :177-179]` |
| 0x11 | `target = switch_channels[p0] ? p1 : p2`; target ≠ 0 → `switch_channels[target] = p3` + time stamp | p0 test, p1 on-ch, p2 off-ch, p3 value | `[M :180-193]` |
| 0x12 | `Player_RamSave` (when `glb_blokes` set); `ResetMap_LevelToLoad(p0, 1, 0)` | p0 next level | `[M :194-199]` |
| 0x13 | `ScriptCam = hash` | — | `[M :200-202]` |
| 0,1,2,0x14+ | no-op | — | `[M :97,202-203]` |

## End of script, object lifetime

- `Script_Update` (0x1da998): only while play & 1; `+0x9C` > 0 → decrement and skip the tick `[M :16,112-114]`;
  else run every stream once in order (the `+0xAB3` loop), clock += FRAME_RATE_MUL, per-stream clocks +=, then the
  camera-fade bookkeeping (a camera script near its end triggers `Camera_SetFade`; when a script stops with
  `fade_clock > fade_len`: fade out, play &= ~0x10 `[M :44-110]`).
- `Script_IsPlaying` (0x1db630): looping → play & 1; else (play & 2) == 0 && (play & 1).
- `Script_Free` (0x1da880): `Script_KillStream` every stream; `LoadableReload(hash)` if streamed; **flags & 0x80 →
  `Drone_EnableAll(1, hash)`**; **flags & 0x100 → `Player_Enable(glb_players, +0x40, +0xAB6)`**; `ScriptCam == hash
  → ScriptCam = 0`; `Mem_Free(SCRIPTINFO)` `[M :22-52]`. Called from `SP_Update` state 6 (slot A), `SP_Delete`
  (both slots) and `Script_KillStream` case 7 (nested).
- `SP_Delete` (0x1dc8b0): `Script_Free` both slots, `obj+0xFE |= 1` `[M]`.
- `Script_KillStream` (0x1d9860) per kind: 1 entity — `obj+0xE4 = 0`; keep (flags & 0x10) → position frozen
  (`Vec_Copy2(+0xC0 → +0x30/+0x40)`, moved flag) else delete; 2 anim — keep → `obj+0xE0 = 0` (detached) else
  delete; 3 sprite → `Sprite_Delete`; 4 camera → `Camera_PopStates`, `Camera_Enable(4,0,0,0)`, free CAM_INFO,
  `Music_Event(0xC, hash)`, letterbox off; 5 sound → `Sound_Stop`; 6 light → `Light_Delete` (unless keep);
  7 subscript → `Script_Free(nested)`; all → kind 0, handle 0 `[M :30-110]`.
- `Script_GetObj` (0x1db688): stream k's handle when kind ∈ {1,2} — the only route from events to objects `[M]`.
- `Script_CreateEntity` (0x1d8968) reuse: an existing handle whose class byte (`obj+0xFF`) differs from the mapped
  class is deleted first; same class → re-glist (`hashtable_set_object_to_entity_gfx`) and reuse. obj_type →
  creator: 0x20 Break, 0x22 Ladder, 0x29 Switch, 0xE1 Destroy, 0xEA/0xEB TouchOnce/Touch triggers, 0xF0
  `Pickup_Create(p0..p3)`, 0xF2 Emitter, 0xF5 AI volume, default a plain glist object; then
  `control_unlink_object` `[M :35-176]`. `Script_EntityStart` then wires `obj+0xE4 = owner`, `+0xFA |= 0xE4`,
  interps, `Control_BuildWorldSph`, links to the room, syncs rotation (+0x50) and position (+0x30/+0x40) `[M
  0x1d8d78:108-118]`.

## Snapshot check `[L]`

`work/ee_start_05.bin` (07000005 start), 8 class-0x1B objects found in the `DynamicObjList` walk:

| obj | SP | fields read | value | meaning |
|---|---|---|---|---|
| 0x16d3650 | 0x16d3760 | +0x1C / +0x24 / +0x26 / +0x2E / +0x30 / +0x35 | 0x6000A23 / 1 / 64 (0x40) / 0x1 / 101 / 1 | the EndLevel script: mode 64 preloaded (mode ≠ 0,≠1 → `SP_LoadScript` at create), primed state 1, mask 1, out ch 101, stay |
| 0x16d3650 | SI0 0x16d2b80 | +0x98 / +0xA0 / +0xAB0 / +0xAAA / +0xAB2 / +0x94 | 0x6000A23 / 2.0 / 310 / 2 / 0x0 / 1.0 | ran 2 ticks at load then stopped; scale 1.0 (p10 = 0) |
| 0x16d3650 | SSTREAM 0-1 | +0x08 / +0x0C / +0x1C/+0x1E / +0x24 | 2.00 / 310, 300 / 0/65535 ×2 / 0 | empty key ranges (0xFFFF sentinel), no handles yet |
| 0x16e8ad0 | 0x16e8be0 | +0x1C / +0x24 / +0x26 / +0x2A | 0x6000A22 / 2 / 1 / 0 | the truck script: mode 1 → state 7 → auto-played, now state 2 |
| 0x16e8ad0 | SI0 0x137d210 | +0x98 / +0xA0 / +0xAB0 / +0xAAA / +0xAB2 | 0x6000A22 / 3.0 / 5895 / 22 / 0x1 | truck: playing, clock 3, length 5895, 22 streams |
| 0x16e8ad0 | SSTREAM 1/2 | +0x24 / +0x10 / +0x25 | 2 / 0x137d0f0, 1 / 0x137b950, parent 1 | stream 1 = the truck anim (kind 2), stream 2 an entity parented to stream 1 |
| 0x16ddb40 | 0x16ddc50 | +0x1C/+0x20 / +0x24 / +0x26/+0x28 / +0x36 | 0x6000046/0x6000047 / 6 / 8,8 / **1** | a looping pair (modes 8, state 6, mask 0) that is **disabled** (+0x36) at level start |
| 0x16ddb40 | SI0 0x16dd070 | +0xAB2 / +0xAAA / +0xA0 | 0x9 / 12 / 2.0 | play 0x9 = playing+looping; entity streams hold tag-1 objects |
| 0x16d6ad0 | 0x16d6be0 | +0x1C / +0x24 / +0x2E / +0x30 | 0x6000994 / 0 / 0x1 / 67 | mode 0 + mask 1: waits for ch 0x43 (the `SP_Update` 0x6000994/music special case) |

Also `[L]`: `SCRIPTINFO+0xB0` = SSTREAM[0]+0x08 (the clock Script_Update advances), loop pairs' clocks equal the
script clock, tags = 1 (clock 0 + 1 at creation), parents 0xFF except the parented truck stream.

## Port pointers

`crates/nf-game/src/nis.rs` models this subsystem; the disagreement list lives in
[../atlas/pickup-port-check.md](pickup-port-check.md) (same file covers `pickup.rs`).

**The skip is ported (fix-dev)** [M]: `SP_Update` state 2 reads the skip action (`Input_Action(-1, 0x19, 1)`
0x1dbda8:129-141) and calls `Script_FFwd(cur, 1)` (0x1dac78) unless `Script_IsDeathNIS(script A)` (0x1dae60).
`Script_FFwd` needs the camera-active flag (`+0xAB5` bit 4 — `Script_CameraStart` 0x1d9140 sets it,
`Script_CameraEnd` 0x1db348 clears it): on the first press it arms the fade (`+0xAB4 = min(length − clock, 45)`,
`play |= 0x10`, `+0xA4 = FRAME_RATE_MUL`, `Music_Event(5, hash)`, `Camera_SetFade(...)` view-side) or, with event
0x10 set, kills the camera stream; the kind-2 streams' dataums get `+0xC8 |= 0x40000` (view-side snap flag, not
modelled). `Script_Update` (0x1da998:19-110)'s do-while is the fast-forward itself: while
`fade_clock > fade_len` and `Script_IsPlaying`, it re-runs every stream and advances the clocks — the rest of the
cut-scene (events included) runs inside that one call; a death NIS breaks out at `length − 20`. The final block
fades back in and clears the skip latch. `Script_Interp` (0x1d9a08:146,205) freezes kinds 2/4 while `+0xAB5` bit 5.
Port fields: `Script.fade_clock` / `fade_len` / `fade_flags`, `Scripts::skip_request` (the viewer's key handler
sets it while `Scripts::cutscene_playing()`), tests in `nis.rs` (`skip_fast_forwards_a_camera_script_but_not_a_death_nis`)
and `nf-view/tests/dev_tools_live.rs` (`dev_skip_ends_the_gatehouse_cutscene`).
