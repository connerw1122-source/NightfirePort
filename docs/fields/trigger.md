# Trigger ext (the trigger instance, 0x40 bytes at obj+0x110; class 0x1D)

Proof levels: see [README.md](README.md). Instance size 0x40 from `Trigger_Create` 0x1b1720
(`control_create_object(0x40,…)` 0x1b1720:52); class byte `obj+0xFF = 0x1D` (0x1b1720:93).
Level: ee_start_05 (07000005 at start) unless noted. Shared classes that reuse this layout:
none — SoundTrigger (0x40) / MusicTrigger (0x44) / InverterTrigger (0x53) have their own smaller
exts (docs/atlas/trigger.md sibling note), and their updates do **not** call `Trigger_Update`
(the earlier "callers" hit was the substring `…Trigger_Update` inside their own names).

Full reader census: the ext is touched only by the Trigger\_\* family, the eight placement
wrappers, and indirect callers (`Player_Activate` → `Trigger_Activate`; `Check_Target` →
`Trigger_Type`/`Trigger_Locked`). `param_offsets.py TRIGGER_Tag` finds exactly one signature
hit (`Trigger_Hit`); every other function in the decompile that prints these byte offsets is
reading its own class's instance (Bullet/Mine/Explode/Emitter/SS/Sub/Pickup/Spawner/HUD/SP/
camera/AnimObject/Sensor…, all verified as different creators). `control_funcs[0x1D]`
@0x26f9ec = {0x1b1a78, 0, 0} — update only, no collide/delete handler [M docs/atlas/trigger.md,
ELF bytes].

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | framelist pointer for the spline anim (0 = static; the placement record's `+0x08` = `m_paths[record+0x44]`, the same door-style path the parsemap builds) | [M\*] W `Trigger_Create` 0x1b1720:61 `*__s = *(undefined4 *)(param_3 + 8)`; R `Trigger_Update` 0x1b1a78:61-62 (arg of `Spline_Interp`). Ghidra prints the call as `Spline_Interp(puVar3[1],*puVar3,0,sVar1,…)` — its float/integer register assignment is scrambled (cf. `Hurt_Update` 0x17e5d0:124 and `ThirdCam_GetPos` 0x1bc590:9, where the framelist arg lands on `ext+0x00`/`ext+0x08` with the float t in `param_1`); the pointer can only be +0x00 (0x1b1720:61 writes the u32 path pointer there and 0x1b1720:60 zeroes +0x04) | [L-read] all 49 exts on 07000005 read 0x00000000 (no animated trigger) |
| +0x04 | f32 | anim time; advanced by `+0x08 · FRAME_RATE_MUL`, wrapped to 0 when > `+0x0C` | [M\*] W 0x1b1720:60 (`__s[1] = 0`), 0x1b1a78:65-69; R 0x1b1a78:61-62 (Spline_Interp t). [L-read] 0.0 on all 49 |
| +0x08 | f32 | anim duration = `frames · REC_FRAME_RATE · FRAME_RATE / (u16)level_tag+0x04` (the record's rate; first store 0x1b1720:85 is dead — 0x1b1720:86 overwrites it) | [M\*] W 0x1b1720:85-86; R 0x1b1a78:65 (`(float)puVar3[2] * fVar12`) |
| +0x0C | u16 | anim key count − 1 (0 when the record's +0x02 is 0) | [M\*] W 0x1b1720:62-66; R 0x1b1720:85-86 (dur), 0x1b1720:89 (≠0 → `obj+0xFA \|= 4`), 0x1b1a78:47/58 (`sVar1 = *(short *)(puVar3 + 3)`), 0x1b1a78:67 (wrap test) |
| +0x10..+0x1B | f32[3] | up vector of the placement matrix (`View_RotTransMatrix(rot,pos)` → `Mat_GetUp`) — never read | [M1] W 0x1b1720:97-98 (`Mat_GetUp(__s + 4, …)`); no reader found in the whole decompile. [L-read] census over all 49: (0,1,0) × 39, (−0,1,0) × 8, (0,1,−0) × 1 (0x016d3a20), (0,0,1) × 1 (the floor-plate TouchOnce 0x016e9970) |
| +0x1C..+0x1F | — | padding (memset 0, never touched) | [M\*] 0x1b1720:59 `memset(__s,0,0x40)` |
| +0x20 | u32 | **out** channel (u32! the channel arrays are u8[256], values ≤ 255; LOAD_LEVEL: the next level's hashcode, e.g. 0x07000006) | [M\*] W 0x1b1720:68 (`__s[8] = *(undefined4 *)(param_3 + 0x30)` = param 1); R `Trigger_Update` 0x1b1a78:74/79/83-87/102-107/128/148-156/170/188/194 (`puVar3[8]` in every case arm), `Trigger_Calc` 0x1b1900:33-35 (Touch clear pass), `Trigger_Activate` 0x1b2318:27-31/41-43/58/96-98/111 (writes + `ResetMap_LevelToLoad(out,1,0)`), `Trigger_Hit` reads it only via param_2+0x20? no — Hit never reads +0x20. [L-read] ext 0x016e2440 → 0x07000006, ext 0x016e9810 → 0x22, ext 0x016d6820 → 0x43 |
| +0x24 | u16 | lock channel: a locked trigger is inert (and hides its model when +0x3A ≠ 0) | [M\*] W 0x1b1720:87 (`*(undefined2 *)(__s + 9)` = param 10); R 0x1b1a78:46-59 (`switch_channels[*(ushort *)(puVar3 + 9)]` → return / `obj+0xF0 \|= 0x30`), `Trigger_Locked` 0x1b2fc8:20-22. [L-read] ext 0x016d6820 → 179 (0xB3), ext 0x016d3e60 → 111 (0x6F), ext 0x016d3fc0 → 164 (0xA4) |
| +0x26 | u16 | input[0]; for LOAD_LEVEL the **key gate**; filter entry 0 of the Touch/mux list | [M\*] W 0x1b1720:69 (`(int)__s + 0x26` ← param 2 slot); R 0x1b1a78:93/97/100 (AND loop), 0x1b1a78:126 (SEQ input0), 0x1b1a78:171/175/178 (OR loop), 0x1b1a78:151/153 (OUT targets), 0x1b1a78:191-193 (MoviePlayer trigger channel), `Trigger_Hit` 0x1b1fc8:70/74-77/120 (filter chain base), `Trigger_Activate` 0x1b2318:76-88 (key gate), `Trigger_Locked` 0x1b2fc8:13-15. [L-read] LL ext 0x016e2440 → 0 (no key), OR-mux ext 0x016e9810 → 35 (0x23) |
| +0x28 | u16 | input[1]; LOAD_LEVEL **gate** (case 10 fires only when its channel is on); **MuxSIn window-enable flag** (the 4-frame chain runs only when this ≠ 0). This is the field the port's `gate` models — it lives in the TRIGGER ext, **not** DOORINFO (see mismatches #11) | [M\*] W `Trigger_LoadLevelCreate` 0x1b2950:15 (level_tag slot 3 ← `TmpParam[2]` = placement param 2) → `Trigger_Create` 0x1b1720:70 (`*(undefined2 *)(__s + 10)` on an `undefined4*` base = byte 0x28); R 0x1b1a78:129 (`if ((*(short *)(puVar3 + 10) != 0) && …)` — SEQ window enable), 0x1b1a78:132/135-136 (SEQ chain inputs), 0x1b1a78:163 (case-10 gate), `Trigger_Activate` 0x1b2318:77/89/95 (gate channel), mux loops. NOTE: docs/atlas/trigger.md's param table says the MuxSIn flag is "ext+0x2C" — the decompile says **ext+0x28** (`puVar3 + 10` on an `undefined4*` = byte 0x28); atlas row is wrong. [L-read] LL ext 0x016e2440 → 0x1D (29), LL ext 0x016e25a0 → 0 (the dead auto path) |
| +0x2A | u16 | input[2]; LOAD_LEVEL **fallback** channel (written when the gate is off — the load still happens) | [M\*] W 0x1b1720:71 (`(int)__s + 0x2a` ← param 4 slot); R mux loops 0x1b1a78 (in[2]), `Trigger_Activate` 0x1b2318:92/102/104-106 (fallback write + timestamp). [L-read] LL ext 0x016e2440 → 0x1F (31) |
| +0x2C | u16 | input[3]; LOAD_LEVEL **success** channel (gate on) | [M\*] W 0x1b1720:72 (`__s + 0xb` ← param 5 slot); R mux loops, `Trigger_Activate` 0x1b2318:96-98 (success write). [L-read] LL ext 0x016e2440 → 0x1D (29) |
| +0x2E | u16 | input[4]; **`Trigger_Hit`'s mask&1 base gate** | [M\*] W 0x1b1720:73 (`(int)__s + 0x2e` ← param 6 slot); R `Trigger_Hit` 0x1b1fc8:63-67 (`bVar11 = switch_channels[*(ushort *)(param_2 + 0x2e)]`), mux loops. [L-read] TouchOnce ext 0x016da620 (out 0x42) → 51 (0x33) |
| +0x30 | u16 | input[5]; on the Touch wrappers this is **the same param 7 the lock reads** (wrapper does not clear the slot) | [M\*] W 0x1b1720:74 (`__s + 0xc` ← param 7 slot); R mux loops 0x1b1a78. [L-read] TouchOnce ext 0x016d3e60 (out 0x85) → 111 = its lock at +0x24 (param 7 lands in both) |
| +0x32 | u16 | input[6] | [M\*] W 0x1b1720:75-77 (`(int)__s + 0x32` ← param 8 slot); R mux loops. [L-read] 0 on all 49 |
| +0x34 | u16 | terminator, written 0 | [M1] W 0x1b1720:76 (`*(undefined2 *)(__s + 0xd) = 0`); "reader" = the ≤8-iteration loops that stop on the 0 slot (0x1b1a78:100/140/178; 0x1b1fc8:120/155). [L-read] 0 on all 49 |
| +0x36 | u16 | **mask** (contact bits: 8 player, 0x10 drone, 2 class-5 object, 1 = gate hit on +0x2E, 4 = E-usable; default 2 when param 9 is 0) | [M\*] W 0x1b1720:78-84 (default 2), `Trigger_LoadLevelCreate` 0x1b2950:18 (forces 0xE); R `Trigger_Hit` 0x1b1fc8:55-63/124/126/161 (every branch), `Trigger_Activate` 0x1b2318:24/38/55 (`mask & 4`). [L-read] LL 0x016e2440 → 0xE; TouchOnce 0x016e9d90 → 8; Touch 0x016e7160 → 0x18; mux 0x016e9810 → 2 |
| +0x38 | u16 | **type** (1 TouchOnce, 2 Touch, 3 AND, 4 SEQ_AND, 5 OUT, 10 LOAD_LEVEL, 0xC OR, 0xD MoviePlayer; 99 = dead after a load) | [M\*] W 0x1b1720:67 (`__s + 0xe` ← param 0 slot), wrappers 0x1b2950:12/0x1b2a08:11/0x1b2aa8:14/0x1b2b88:14/0x1b2c68:15/0x1b2d38:15/0x1b2e08:15/0x1b2ed8:15 (forced values), `Trigger_Activate` 0x1b2318:113 (= 99); R 0x1b1a78:30 (ProcessTouch gate for type 2), 0x1b1a78:72 (switch), `Trigger_Calc` 0x1b1900:32/56 (type-2 clear + update passes), `Trigger_Hit` 0x1b1fc8:56/130 (`type − 1 < 2`), `Trigger_Activate` 0x1b2318:22, `Trigger_Type` 0x1b2fa8:7, `Trigger_Locked` 0x1b2fc8:12. [L-read] 0x016e2440 → 10, 0x016e9d90 → 1, 0x016e7160 → 2, 0x016e9810 → 0xC, 0x016d37b0 → 3 |
| +0x3A | u16 | hide-flag enable: while **locked**, `obj+0xF0 \|= 0x30` (not-drawn + no-vision bits) only when this ≠ 0 | [M\*] W 0x1b1720:88 (`(int)__s + 0x3a` ← param 11); R 0x1b1a78:52 (`if (*(short *)((int)puVar3 + 0x3a) == 0)` selects the plain flags). Single reader+writer, fully enumerated |
| +0x3C | s16 | locked-message cooldown (frames); set to `3·FRAME_RATE_INT` after the "locked" text, silently blocks re-showing | [M\*] W memset 0x1b1720:59, `Trigger_Activate` 0x1b2318:86 (`*(short *)(iVar2 + 0x3c) = (short)(FRAME_RATE_INT << 1) + (short)FRAME_RATE_INT`); R 0x1b1a78:27-29 (decrement), 0x1b2318:81-83 (silent return while ≠ 0). [L-read] 0 on all 49 at level start |
| +0x3E | — | unused (memset only) | [M\*] 0x1b1720:59 |

Object-side fields this class uses: `obj+0xD8` = glist (0x1b1720:92), `obj+0xF0` =
`*(u32*)(glist+0x44)` (0x1b1720:95 — direct store, NOT `Control_SetGList`, so the
0x30008000 preserved bits are dropped), cleared of bit 0x10 when type == 10 (0x1b1720:99-101
`& 0xffffffef` — LOAD_LEVEL triggers never self-delete via the 0x10 path); `obj+0xFA |= 4`
when animated (0x1b1720:89-91), `|= 0x20` while the spline advances (0x1b1a78:64);
`obj+0xFE |= 1` = self-delete (0x1b1a78:121, 0x1b2318:48); `obj+0xFF = 0x1D`; `obj+0xF4`
stays 0. `obj+0xF0 & 0x10` (a glist flag) gates the TouchOnce/Bond-moment self-delete
(0x1b1a78:119-123) — live values: 0x50 on the touch family, 0x30 on muxes, 0x80000 on the
LL pair [L].

## Param map (from the creator)

`level_tag` = the 0xB0-byte runtime placement record parsemap builds; param k at
`level_tag+0x2C+4k`. `Trigger_Create` (0x1b1720) reads: +0x02 u16 key count → +0x0C,
+0x04 u16 rate → +0x08, +0x08 u32 path ptr → +0x00, +0x2C..+0x58 = params 0..11.

| placement type (wrapper) | type (+0x38) | out (+0x20) | mask (+0x36) | inputs (+0x26..+0x32) | lock (+0x24) | hide (+0x3A) |
|---|---|---|---|---|---|---|
| 0xDC raw (no wrapper) | p0 | p1 | p9 (default 2) | p2, p3, p4, p5, p6, p7, p8 | p10 | p11 |
| 0xEA TouchOnce 0x1b2b88 | 1 | p0 | p1 | p2, p3, p4, p5, p6, **p7**, p8 | **p7** | **p8** |
| 0xEB Touch 0x1b2aa8 | 2 | p0 | p1 | p2, p3, p4, p5, **p6**, p7, p8 | **p6** | **p7** |
| 0xEC MuxIn 0x1b2c68 | 3 | p0 | raw p9 | p1, p2, p3, p4, p5, p6, raw p8 | raw p10 | raw p11 |
| 0xED MuxSIn 0x1b2e08 | 4 | p0 | raw p9 | p1..p6 + raw p8; **+0x28 (input[1]) doubles as the window-enable flag** | raw p10 | raw p11 |
| 0xEE MuxOut 0x1b2ed8 | 5 | p0 (source) | raw p9 | p1..p6 + raw p8 = the targets written | raw p10 | raw p11 |
| 0xEF MuxOrIn 0x1b2d38 | 0xC | p0 | raw p9 | p1..p6 + raw p8 | raw p10 | raw p11 |
| 0xE8 LoadLevel 0x1b2950 | 10 (forced) | **p0 = next level hashcode** | **forced 0xE** | key = p1, gate = p2, fallback = p3, success = p4 (slots 6-8 keep raw p5..p7, unread) | raw p10 | raw p11 |
| 0xF4 MoviePlayer 0x1b2a08 | 0xD | p0 (movie id → GameState+0x10) | p1 | trigger channel = p2 (input[0]) | raw p10 | raw p11 |

Wrapper mechanics [M]: every wrapper first `amemcpy(TmpParam@0x319b88, tag+0x2C, 0x84)`,
rewrites slots, then calls `Trigger_Create`. TouchOnce/Touch restore slots 2-7 unchanged
(0x1b2b88:20-25) and move p7/p8 (or p6/p7) into the lock/hide slots (0x1b2b88:17-19,
0x1b2aa8:17-19); the mux family writes `slot[k] = p(k−1)` for k = 2..7 only (loop bound
`uVar3 < 8`, 0x1b2c68:17-22) leaving slot 8 = raw p8 and slots 9-11 = raw p9/p10/p11;
LoadLevel sets type 10, moves TmpParam[0..4] into slots 1-5 and forces slot 9 = 0xE
(0x1b2950:12-19); MoviePlayer sets type 0xD, out = p0, input[0] = p2, mask = p1
(0x1b2a08:11-14).

## Trigger_Update (0x1b1a78) per type — reader summary

Preamble: cooldown −1 (0x1b1a78:27-29); type 2 runs only inside `Trigger_Calc`'s
`ProcessTouch` window (0x1b1a78:30-33, global u8 `ProcessTouch` set 0x1b1900:50-71);
`Control_SetEffectType(obj, glist+0x44)`; lock check → return (+0x24, hide side effect
0x1b1a78:46-59); spline advance when +0x0C ≠ 0 (0x1b1a78:60-70). Then `hit = Trigger_Hit`
(0x1b1a78:71) and the switch on +0x38:

| type | behaviour [M] |
|---|---|
| 1 TouchOnce | `ch[out]` already on → self-delete iff `obj+0xF0 & 0x10` (0x1b1a78:74-77, 119-123); else falls into the type-2 body (`ch[out] \|= hit`, timestamp on hit; 0x7D early-returns off 0x07001500, 0x1b1a78:80-82) |
| 2 Touch | `ch[out] \|= (hit != 0)` + timestamp (0x1b1a78:83-88). No Bond tail (the fall-through from case 1 `break`s here) — docs/atlas/trigger.md's "type 2 shared Bond tail" row is not in this decompile |
| 3 AND | acc = 1; AND every nonzero input at +0x26.. (≤8, 0 slot ends); write acc; timestamp; **Bond-moment tail**: out in 0x65..0x6E && acc && `obj+0xF0 & 0x10` → delete (0x1b1a78:90-124) |
| 4 SEQ_AND | acc = `ch[input0]`; the window chain runs **only when +0x28 ≠ 0** and acc: for i ≥ 1, `ch[in[i]]` && `time[in[i−1]] − 4 <= time[in[i]]` (no upper bound; 0x1b1a78:125-146); write + timestamp, **no Bond tail** |
| 5 OUT | copies `ch[out]` to every nonzero input slot (all 8 scanned, 0x1b1a78:147-161), timestamps nonzero targets |
| 10 LOAD_LEVEL | `+0x28 != 0 && ch[+0x28]` → `Trigger_Activate(obj, glb_players)` (0x1b1a78:162-166) |
| 0xC OR | acc = 0; OR the inputs; write + timestamp (0x1b1a78:167-189) |
| 0xD MoviePlayer | `ch[input0]` on → clear it, `GameState+0x10 = out`, `GameState+0x14 = 0`, `GameFlow_PushState(0,6,0xff)`, `GameFlow_PushState(30.0,7,0xff)` (0x1b1a78:190-199) |

`Trigger_Activate` (0x1b2318, from `Player_Activate` 0x1a5170:135 case 0x1D and case 10):
requires the activator's `obj+0xFF == 3`; reads ch 0x63/0x62/0x61/0x60 → abort (0x1b2318:64-75);
type 2/1 need `mask & 4` then write out (+0x20) with timestamp; type 1 then self-deletes on
`obj+0xF0 & 0x10`; type 0xD = the movie push with **300.0** (0x1b2318:55-62); type 10: key gate
(+0x26) off & set → message 0x2000021 + cooldown +0x3C = 3·FRAME_RATE_INT, no write, no load
(0x1b2318:76-90); gate (+0x28) on → success write (+0x2C), off → fallback write (+0x2A) —
**both fall through to the load** (0x1b2318:91-107 → LAB_001b24b8): `Music_Event(3,1)`,
`GS_PauseGame(1)`, `ResetMap_LevelToLoad(out,1,0)`, `GameFlow_PushState(300.0,7,0xff)`,
`+0x38 = 99`, `Player_RamSave()` when `glb_blokes` (0x1b2318:108-116).

`Trigger_Hit` (0x1b1fc8): contact bits from `obj+0xD0` nodes ({+4 next, +0x58 obj}):
class 3 → 8, 2 → 0x10, 5 → 2, 0x4A → 0x10 (0x1b1fc8:27-47). Mask & 0x18 with a player/drone
contact and type ∈ {1,2}: mask 8 → player bit only; mask 0x10 → drone bit only (chain still
runs); base = 1 or `ch[+0x2E]` (mask & 1); then the AND-chain over +0x26.. (0xB needs the
drone bit, 0xE the player bit, else a contacted class-2 object whose `Drone_GetSkinHash()`
(0x13a578 — returns instance+0xE0 u32, compared as s16) equals the entry; 0x1b1fc8:56-123).
Mask & 2: a class-5 contact matches every nonzero filter entry against
`*(s16*)(*(contact->ext+0x44) + 2)` (0x1b1fc8:124-159). Fallback: `(mask & bits) != 0`.

`Trigger_Calc` (0x1b1900, called from `control_movement_object_handler` 0x1336f8 before the
prev/hold channel snapshot): Bond moments 0x65..0x6E → `PlrStat_DoneBondMoment`; the two
hard-coded latches (0x07000005: ch 0xA4 → ch 0x6F; 0x0700000B: ch 0x34 → ch 0x2D); clears
`ch[out]` of every class-0x1D object with type == 2 && out ≠ 0 (walks DynamicObjList,
0x1b1900:27-48); `ProcessTouch = 1`, re-walks calling `Trigger_Update` for the type-2 objects,
`ProcessTouch = 0` (0x1b1900:50-71).

`Check_Target` (0x1a7810:193-197): aiming within 5 units of a type-10 trigger with
`LoadMsg == 0` on a level ≠ 0x07000009 shows message 0x2000009 when `Trigger_Locked` says
locked (`Trigger_Locked` 0x1b2fc8: type 10 → locked iff key gate set && its channel off;
else lock channel on). `DroneFunc_SetMissionFailReason` (0x13e560:33-40, reason 0x10):
`Switch_GetNearest(drone pos)` → `Switch_GetSwitchChannel` → turns that channel ON (0x61 gets
the special fail label 0x4000032) — a guard raising the alarm via the nearest alarm switch.

## Snapshot checks (ee_start_05.bin, 07000005 at start) [L]

49 class-0x1D objects (matches the class histogram). Walker: head 0x2705a0 → first
0x01319e30, next obj+0x14; instance = u32 at obj+0xE0. Decode script:
`python3 /tmp/trigger_read.py`-style walk (not committed). Highlights (raw EE bytes,
little-endian):

- LL exit ext **0x016e2440** (obj 0x016e2330): bytes at +0x20 `06 00 00 07 | 00 00 | 00 00 |
  1d 00 | 1f 00 | 1d 00 | …` → out 0x07000006, lock 0, key 0, **gate 0x1D**, fallback 0x1F,
  success 0x1D, mask 0x000E, type 0x000A; `obj+0xF0` = 0x00080000 (bit 0x10 cleared by the
  type-10 branch). Sister ext 0x016e25a0: gate 0, fallback 0x1E, success 0 — the auto path.
- Gate TouchOnce ext 0x016e9d90/0x016e9c30/0x016e9ad0/0x016e9970: type 1, out 0x23/0x24/0x25/0x21,
  mask 8, `obj+0xF0` = 0x50.
- OR mux ext 0x016e9810: type 0xC, out 0x22, inputs 35,36,37 (0x23|0x24|0x25), mask 2,
  `obj+0xF0` = 0x30. AND muxes 0x016e6a30/0x016e6b90/0x016e6cf0: out 0xA1/0xA2/0xA3,
  inputs 84+85 / 86+87 / 88+89.
- Locked TouchOnce ext 0x016d6820: out 0x43, **lock 179 (0xB3)**; the 0x7F/0x80/0x81 plates
  carry lock 164 (0xA4), the 0x85 plate lock 111 (0x6F) — and each lock value also appears in
  input[5] (+0x30) because the TouchOnce wrapper leaves param 7 in its slot.
- The 0x7D special: ext 0x016e2fd0 type 2 out 0x7D mask 8 (inert off 0x07001500).
- anim block (+0x00..+0x0F) = 0 on all 49; up vec +0x10 = (0,1,0) except three plates.
- `switch_channels` @0x26FC90: only ch 0x45 = 1; `switch_channels_time` @0x270190
  pre-seeded with 0x1a676 (the frame counter) in all 256 cells — `Init_SwitchChannels`
  0x134310's pre-seed, live-confirmed.

## Port mismatches (crates/nf-game/src/triggers.rs; report only)

1. **Raw 0xDC param shift** — build() applies `let shift = raw as u32` (triggers.rs:389) to the
   whole param set: out = param(1), list = params 2..7 + **param 9**, mask = **param 10**,
   lock = **param 11**, hide = **param 12**. The game's raw type is dispatched with NO shift:
   `Trigger_Create` reads out = p1, inputs = p2..p8, mask = p9, lock = p10, hide = p11
   (0x1b1720:67-91; dispatch 0x1d04f0 case 0xDC). The port marks it `*suggested*`; whether any
   0xDC placement exists in shipped levels is unverified.
2. **SEQ_AND window-enable gate missing** — the game runs the 4-frame window chain only when
   input[1] (+0x28 = mux param 2) is nonzero (`if ((*(short *)(puVar3 + 10) != 0) && …)`,
   0x1b1a78:129); with p2 = 0 the output is just `ch[input0]`. The port always chains every
   list entry with the window test (triggers.rs:341-356). Latent: no 0xED placement on
   07000005.
3. **SEQ_AND Bond-moment self-delete is extra** — port: `if acc && (0x65..=0x6e).contains(&self.out)
   && self.glist_0x10 { self.gone = true; }` (triggers.rs:356). The game's case 4 tail
   (LAB_001b1e60 → LAB_001b1e8c) writes + timestamps only — no delete (0x1b1a78:183-189).
   (The AND arm's delete at 0x1b1a78:112-123 is correctly ported.)
4. **MoviePlayer (type 0xD, placement 0xF4) not modelled** — no kind constant, build()
   `_ => continue` (triggers.rs:387). Game: case 0xD clears the trigger channel and pushes the
   movie states (0x1b1a78:190-199; E-path variant with 300.0 at 0x1b2318:55-62).
5. **`hide` stored but never applied** — port keeps `hide: bool` (triggers.rs:31) but nothing
   reads it; the game hides a *locked* trigger's model (`obj+0xF0 |= 0x30` when lock on &&
   +0x3A ≠ 0, 0x1b1a78:50-55). Render-side only; nf-view draws no trigger models today.
6. **`Channels::default()` zero-fills time** (triggers.rs:73) vs `Init_SwitchChannels`
   pre-seeding all 256 cells with the frame counter — live-confirmed here (all cells 0x1a676
   [L]). Already open as trigger-port-check #4/§Channels.
7. **Locked key-gate stand-in** — the game shows message 0x2000021 and sets the +0x3C cooldown
   (3·FRAME_RATE_INT) which silently blocks re-messageing (0x1b2318:81-87); port returns with
   no write and no cooldown (triggers.rs:295-298, documented "no text system").
8. **Anim/spline fields absent** — the port has no framelist/t/dur/frames equivalent (no
   animated trigger in the shipped levels; all 49 exts have the block zeroed [L]).
9. **Doc-comment rot (not code)** — `from_params`' comment says the mux wrappers map
   "inputs = raw p1..p7, mask = raw p8, lock = raw p9, hide = raw p10" (triggers.rs:154-156);
   the code one line below (and the game) use inputs p1..p6 + raw p8, mask p9, lock p10,
   hide p11.
10. **docs/atlas/trigger.md discrepancies found while verifying** (report only): (a) the
    MuxSIn param-table row "ext+0x2C = 4-frame window flag" should be **ext+0x28** (=
    input[1] = p2); (b) the type-2 row's "shared tail also self-deletes … Bond moment" — the
    decompile's case 2 ends in `break` with no Bond tail (the tail belongs to case 3 and the
    case-1 channel-on branch).
11. **`gate` mislabelled as a DOORINFO field** (comment-only, found by agent 2-b, re-verified here):
    triggers.rs:50-52 says *"The gate is `DOORINFO+0x28` = original `param[2]` of the placement
    (verified against MIPS disassembly: `lhu v1,56(s1); sh v1,40(s0)`) [M]"*. The offset and
    param are right but the struct is the **TRIGGER ext+0x28**: `Trigger_LoadLevelCreate`
    0x1b2950:15 writes placement param 2 (level_tag+0x38 — the `lhu …,56(s1)` read) into tag
    slot 3, and `Trigger_Create` 0x1b1720:70 stores it at ext+0x28 (the `sh …,40(s0)` — `__s + 10`
    on an `undefined4*` base = byte 0x28). Readers: `Trigger_Update` 0x1b1a78:163 (case 10) and
    `Trigger_Activate` 0x1b2318:77/89/95. DOORINFO (0x80 bytes, class 0x1C) has no +0x28 field
    at all (docs/fields/door.md). The `param[2]`/semantics half of the comment is correct.

## Live checks (main agent)

The interesting fields are the LL gate (+0x28) and the lock (+0x24). Instance chain:
`DynamicObjList` 0x2705a0 → head+0x14 → next obj+0x14; filter `obj+0xFF == 0x1D`; ext = u32 at
obj+0xE0. The 07000005 LL pair: obj 0x016e2330 (ext 0x016e2440, gate 0x1D) / obj 0x016e2490
(ext 0x016e25a0, gate 0) at level start — addresses move per session, re-walk.

- **Gate channel drives the auto-load**: `fieldprobe.py watch <ext>+0x28 2` while the
  EndLevel script fires (walk into the exit volume) — the byte pair should read 0x1D 00 the
  whole session; then `poke` switch_channels[0x1D] (0x26FC90+0x1D) to 1 and the level should
  load (Music_Event 3, pause, 07000006). If it does not, the gate is read elsewhere.
- **Lock kills a TouchOnce**: stand Bond on the wall-hug plate (out 0x43, lock 0xB3) —
  `fieldprobe.py watch <ext>+0x24 2` (0xB3 constant) and `switch_channels[0xB3]`; force
  `poke 0x26FC90+0xB3 1 1` → the 0x43 channel must stop latching while crept; `poke` 0 again
  → latching resumes. (The +0x3A hide variant: none of the 49 carries +0x3A ≠ 0 [L], so the
  `obj+0xF0 |= 0x30` side effect is not observable on this level.)
- **TouchOnce self-delete**: `watch <ext>+0x38 2` on a gate plate while triggering it — after
  the frame where `switch_channels[out]` turns 1, the next update should write 0x0063 (99 =
  dead) into +0x38 only for the Bond-moment AND path (out 0x65..0x6E); gate plates (out
  0x21-0x25) must NOT flip to 99 (their delete comes from `obj+0xF0 & 0x10` at 0x1b1d78 —
  they carry 0x50, so they DO flip to 99 one frame after their channel turns on; the valley
  0x22/0x44 pair likewise).
- **Cooldown**: with a locked LL exit (key gate set — none on 07000005 [L] both key gates are
  0), press E twice: the second press within 3 s must not re-show the message. Probe
  `<ext>+0x3C` (s16) right after the first E — expect 180 (3·FRAME_RATE_INT).

## Fixed

The raw 0xDC param shift (mismatch 1) is FIXED in `crates/nf-game/src/triggers.rs` `build()`:
the shift is gone — every trigger family reads out = p1, inputs = p2..p8, mask = p9, lock = p10,
hide = p11 [M Trigger_Create 0x1b1720:61-91]. Verified against the disc: 3 raw 0xDC placements
exist on 0700000c (kind 2 TOUCH, out 27); the NF_ISO census test
`raw_0dc_triggers_use_the_unshifted_param_layout` (crates/nf-game/tests/fields_live.rs) pins the
mapping. Mismatches 2-8 remain open (SEQ_AND window gate, MoviePlayer, hide, Channels::default
time seeding, the locked key-gate message, the anim fields).
