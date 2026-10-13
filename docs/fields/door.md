# DOORINFO (the door instance, 0x80 bytes at obj+0x110; class 0x1C)

Proof levels: see [README.md](README.md). Instance size from `Door_Create` 0x136100:21
(`control_create_object(0x80, …)`). Level: ee_start_05 (07000005 at start) unless noted.
Every DOORINFO access in the binary is inside the door module (grep across
`work/decomp/ACTION.ELF` for `TheDoors`/`DoorGroupsStates`/`Door_*` call sites: the only
functions that reach `obj+0xE0` for a door and touch instance offsets are the 13 listed below);
outsiders talk to doors only through the API (caller list at the bottom). Layout skeleton from
the creator; every other row re-verified from the decomp this pass.

Related globals: `TheDoors` 0x316f70 = LLISTINFO header {first +0, last +4, s16 count +8,
u16 node size +0xA} (node size 0x80 — `Door_Init` 0x136848:8 `LList_Init(0x316f70,0x80,0)`);
**the LList nodes ARE the DOORINFO exts** (`LList_Add` 0x1cca28 writes node+0 = prev, node+4 =
next; `Door_Create` 0x136100:94 adds the ext). `DoorGroupsStates` 0x316ea0, 0xCA bytes = 101 u16
slots, one per group id. `DoorMsg` 0x30cd04 (u32) gates `Door_DoMsg` in `Check_Target`.

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | ptr | TheDoors node **prev** (older ext; 0 on the first-created door) | [M*] writer `LList_Add` 0x1cca28:22 (`*puVar3 = uVar1`, node+0 = old last); no door-module reader (the generic `LList_Remove` 0x1ccac8 would use it; no call with a door node found) [L-read: ext 0x16e2180 +0 = 0 (first-created), ext 0x16e1b00 +0 = 0x16e1ca0 (last-created)] |
| +0x04 | ptr | TheDoors node **next** (newer ext; 0 on the last) | [M*] writer `LList_Add` 0x1cca28:14/:19; readers `Doors_Calc` 0x135fe0:20,27,43,46,50,54,66 (list walk) [L-read: ext 0x16e1b00 +4 = 0] |
| +0x08 | ptr | back-pointer to the `obj_tag` | [M*] writer `Door_Create` 0x136100:52 (`*(int *)(iVar3 + 8) = iVar10`); readers `Doors_Calc` 0x135fe0:23 (`iVar3 = *(int *)(iVar6 + 8)` = leader obj), :40 (follower `obj+0xF4 := leader obj+0xF4`), :41-42 (`Door_Interp(follower obj, follower ext, 0)`) [L-read: ext 0x16e1b00 +8 = 0x16e19f0 = its obj] |
| +0x0C | ptr | the anim: a 0x19 `framelist_tag` (0 = none — door never visually moves, `Spline_Interp` returns at once on NULL) | [M*] writer `Door_Create` 0x136100:43 (`+0xc ← tag+0x08`; the runtime tag's +0x08 = `m_paths[record+0x44]`, built by `parsemap_block_map_data_dynamic` 0x1d1b68); readers `Door_SetupSwing` 0x135440:54,57,61,63, `Door_Create` 0x136100:84,87,90 (slide poses), `Door_Interp` 0x135738:19,25,68 [L-read: swing door ext 0x16e1fe0 +0xC raw `04 14 9d 00` = 0x009d1404; the four sliders read 0] |
| +0x10..+0x4F | MATRIX (0x40) | closed-pose matrix, stored as its **inverse** — swing doors: the t=0.5 pose inverted (`Door_SetupSwing` 0x135440:53-59). Sliding doors: `Door_Create` 0x136100:83-88 writes the key-0 pose, `Mat_Inverse`, **then overwrites ext+0x10 with the END pose** (Spline_Interp at `frames` reuses the same target) — and nothing ever reads ext+0x10 for a sliding door, so only the swing value matters | [M*] writers above; readers `Door_Activate` 0x135be0:103 (anti-crush `RotPreTransVec(ext+0x10, activator+0x80)`), :136 (swing-side `RotPreTransVec` — both kind-0 only) [L-read: swing ext 0x16e1fe0+0x10 row 3 = (−57.753, −26.000, −89.311) = the negated closed pose with transposed rotation rows — the inverse; an anim-less slider's +0x10 is zeros with −0.0 in row 3's xyz (`00 00 00 80` ×3 — Mat_Inverse ran on the untouched zero matrix; Spline_Interp wrote nothing on the NULL framelist)] |
| +0x50 | f32 | `t`, the 0..1 anim position (a swing door's CLOSED is 0.5, ends are 0/1; a slider's closed is 0, open 1) | [M*] writers `Door_Create` 0x136100:82 (slide: 0, or 1.0 when state&4), `Door_SetupSwing` 0x135440:49-52 (0.5, or 1.0 when state&4), `Door_Interp` 0x135738:35 (advance), :41 (snap 0.5), :51/:57/:61 (clamps), `Doors_Calc` 0x135fe0:36 (group copy); readers `Door_Interp` 0x135738:33-34 (read-modify), :38-40, :60, :68 (pose at `t·frames`), `Door_Activate` 0x135be0:123 (`t > 0.5` picks the close direction), `Door_ReactToHit` 0x135980:82 (`t > 0.5` flips the local test) [L-read: swing ext 0x16e1fe0 +0x50 = 0.5 (closed swing); sliders = 0.0] |
| +0x54 | f32 | seconds per key step: create = 1/FRAME_RATE (30 during load) or 1/(`u16` rate); a swing door re-quantises to whole ticks: `0.5/(int)(0.5/(2·s))` (1/30 → 1/14) | [M*] writers `Door_Create` 0x136100:45-49, `Door_SetupSwing` 0x135440:46-47; readers `Door_Interp` 0x135738:34 (`t += speed·dir·FRAME_RATE_MUL`), :40 (swing close snap width `ABS(0.5−t) ≤ speed·FRAME_RATE_MUL`) [L-read: swing ext 0x16e1fe0 +0x54 raw `25 49 92 3d` = 0.07142857 = 1/14; sliders raw `89 88 08 3d` = 0.03333334 = 1/30] |
| +0x58 | f32 | travel direction ±1.0 (+ = toward t=1) | [M*] writers `Door_Activate` 0x135be0:124,127 (`t>0.5 ? −1 : +1` close), :142-145 (`dir = side·swing` open; :127 vs :163-165 slide ±1.0 direct — disasm 0x135e88 `swc1 f0,88(s0)`), `Doors_Calc` 0x135fe0:35 (group copy), `Door_ReactToHit` 0x135980:92,96,105 (reversal `−dir`); readers `Door_Interp` 0x135738:34, `Door_Activate` 0x135be0:104,110 (anti-crush margin `dir·swing`) [L-read: all five doors 0.0 at start] |
| +0x5C | f32 | swing-direction sign ±1.0 (`Door_SetupSwing`'s dot of the key-0 pose's dir row vs the mid pose's normal row, `sum < 0 → +1 else −1`). **Never written for sliding doors** (stays 0 from the memset) | [M*] writer `Door_SetupSwing` 0x135440:75-80; readers `Door_Activate` 0x135be0:104,110 (margin), :142 (`dir = side·swing`), `Door_ReactToHit` 0x135980:81 (`f = local.z·swing`) [L-read: swing ext 0x16e1fe0 +0x5C raw `00 00 80 bf` = −1.0; sliders 0.0] |
| +0x60 | u16 | open sound (0xFFFF = none); played entering state 8 | [M*] writer `Door_Create` 0x136100:28-32 (`0xffff` when param 0); reader `Door_SetState` 0x135618:10-13 [L-read: swing ext 0x16e1fe0 +0x60 = 285; sliders 0xFFFF] |
| +0x62 | u16 | close sound (0xFFFF = none); played entering state 0x10 | [M*] writer `Door_Create` 0x136100:33-37; reader `Door_SetState` 0x135618:17-19 [L-read: swing 284; sliders 0xFFFF] |
| +0x64 | u16 | locked/hit sound (0xFFFF = none) | [M*] writer `Door_Create` 0x136100:38-42; reader `Door_Activate` 0x135be0:85 (the locked refusal at 0x135cf8 — also reused by the approach-side refusals). (docs/atlas/door.md also lists Door_SetState as a reader — the decompile shows SetState reads only +0x60/+0x62) [L-read: all five doors 320] |
| +0x66 | u16 | open switch channel (0 = none): on opens, off closes at once | [M*] writer `Door_Create` 0x136100:26 (`← tag+0x34` = placement param 2); readers `Door_Update` 0x1363c8:33,38-40 (channel scan), `Door_SetState` 0x135618:15,23,26,28 (channel mirror + timestamp), `Door_Create` 0x136100:96-98 (starts-open forces it on). (docs/atlas/door.md lists Door_Activate as a reader — the decompile of 0x135be0 does not touch +0x66) [L-read: all five doors 0] |
| +0x68 | u16 | lock channel (0 = none; channel ON = unlocked). Cleared to 0 after the rising-edge unlock (one-shot) | [M*] writer `Door_Create` 0x136100:27 (`← tag+0x38` = placement param 3), `Door_Update` 0x1363c8:106 (= 0 after unlock); readers `Door_Update` 0x1363c8:30,36,77,80,85 (lock-block selection), :88-93 (rising-edge test vs `switch_channels_prev`), `Door_Activate` 0x135be0:53,58 (locked refusal), `Door_IsLocked` 0x136810:10-13, `Door_DoMsg` 0x135f18:20-21 [L-read: the four sliders' exts read 200 (0xC8) — the U-044 channel; swing door 0. `switch_channels[200]` @0x26fd58 = 0 → locked] |
| +0x6A | u16 | group id (index into `DoorGroupsStates`, 0 = none) | [M*] writer `Door_Create` 0x136100:25 (`← tag+0x30` = placement param 1); readers `Door_Update` 0x1363c8:163,170-171 (`DoorGroupsStates[g] |= obj+0xF4`), `Doors_Calc` 0x135fe0:15,30-31 (group match), `Door_IsClosed` 0x1367b0:15,18, `Door_DoMsg` 0x135f18:25,29 [L-read: all five doors 0] |
| +0x6C | s16 | activation cooldown (ticks) | [M*] writers `Door_Activate` 0x135be0:172 (= 2), `Door_ReactToHit` 0x135980:110 (= 2) (0 from the create memset); readers `Door_Update` 0x1363c8:22-23 (decrement each tick), `Door_ReactToHit` 0x135980:44 (runs only when 0) [L-read: all five doors 0] |
| +0x6E | u16 | last key index = (u16)frame count − 1 (0xFFFF when the count is 0) | [M*] writer `Door_Create` 0x136100:44 (`(short)tag+0x02 − 1`); readers `Door_Interp` 0x135738:15,19,25,68-69, `Door_SetupSwing` 0x135440:54,57,61,63, `Door_Create` 0x136100:84,87-91 (slide poses) [L-read: swing ext 0x16e1fe0 +0x6E = 2 (3 keys); sliders 0xFFFF] |
| +0x70 | u16 | flags — placement param 8 \| param 9. bit 0x01 = spline interp (else KeyFrame_Interp); **bit 0x02 = no reader anywhere in the door code** (editor tag, U-043 stays open); bit 0x04 = starts open (selects state 4 at create + forces the open channel on, 0x136100:54-59,96-99); bit 0x10 = busy (set by Activate/ReactToHit/Update-unlock/Doors_Calc copy; cleared for every door by Doors_Calc's second pass; Door_Update returns while set); bit 0x20 = keep-open (suppresses the 360-tick auto-close); other bits: no door-code readers | [M*] writers `Door_Create` 0x136100:53, `Door_Activate` 0x135be0:175, `Door_ReactToHit` 0x135980:111, `Door_Update` 0x1363c8:107, `Doors_Calc` 0x135fe0:38 (OR 0x10) and :64-65 (AND ~0x10, every door); readers `Door_Interp` 0x135738:17 (bit 0), `Door_Update` 0x1363c8:25 (busy gate), :97 (lock-block busy re-check), :114 (keep-open), `Door_Activate` 0x135be0:55,62-66 (busy gate), `Doors_Calc` 0x135fe0:32-34 (follower busy) [L-read: sliders 0x0002 (bit 1 only), swing door 0x0003 (spline + bit 1)] |
| +0x72 | u16 | auto-close quiet timer (ticks); reset on every state change and at the 360 threshold; grouped doors share the leader's value | [M*] writers `Door_SetState` 0x135618:31 (= 0), `Door_Update` 0x1363c8:117 (increment while open), :151 (= 0 at the threshold scan), :74 (= 0 after a channel-off close), `Doors_Calc` 0x135fe0:37,39 (leader copy); reader `Door_Update` 0x1363c8:115,118 (vs `FRAME_RATE_INT·6` = 360 [L 0x30d0cc = 60]) [L-read: all five doors 0] |
| +0x74 | u16 | "locked" message latch (one `Txt_BindLabel(0x200000b)` per close cycle) | [M*] writer/reader `Door_DoMsg` 0x135f18:16-17 (0 → set 1 and show); no other access. Port has no reader (no text system) [L-read: all five doors 0] |
| +0x76 | u8 | one-sided approach mode (0 either side, 1 front-only, 2 rear-only; 1/2 are one-shot — reset to 0 after the first activation) | [M*] writer `Door_Create` 0x136100:69 (`(u8)tag+0x54` = param 10); readers/writers `Door_Activate` 0x135be0:67-93 (the side dot block; writes 0 at :68,:90,:93) [L-read: all five doors 0] |
| +0x77 | u8 | no-E-use (never player-activated) | [M*] writer `Door_Create` 0x136100:70 (`(u8)tag+0x58` = param 11); reader `Door_Activate` 0x135be0:50-52 (`from_E && +0x77 → return 0`) [L-read: all five doors 0] |
| +0x78..+0x7F | 8 B | never touched (create memset 0) | [M — no `+ 0x78)`/`+ 0x7c)` hits on a door base in any door function] [L-read: zeros on all five exts] |

### obj-side door fields (written/read by the door module)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| obj+0xF4 | u16 | state: 2 CLOSED, 4 OPEN, 8 OPENING, 0x10 CLOSING | [M*] writers `Door_Create` 0x136100:54-59, `Door_SetState` 0x135618:30, `Doors_Calc` 0x135fe0:40 (group copy); readers `Door_SetState` 0x135618:10,17,26-28 (entry sounds + channel mirror of the OLD state's 0xC bits), `Door_Update` 0x1363c8:28-38,41,89-90,111,158,163-171, `Door_Interp` 0x135738:38-39 (state gates for the snaps), `Door_Activate` 0x135be0:59,98,100,121-122,157-158, `Door_ReactToHit` 0x135980:77,85-86,102, `Door_IsClosed` 0x1367b0:16; outsiders `NDrone2_DoorIsOpen` 0x14aaa0:11 (`== 4`), `NDrone2_FindElevatorDoor` 0x14a940:24 (`& 2`) [L-read: all five doors 0x0002] |
| obj+0xF6 | s16 | kind: 0 swing (`Door_SetupSwing` at create), 1..2 path-animated sliding, <0 or >2 inert (activation marker only; kind 1 reports closed forever in `Door_IsClosed`) | [M*] writer `Door_Create` 0x136100:60 (`← tag+0x2C` = param 0), branch 0x136100:72-74 (swing vs slide setup); readers `Door_Interp` 0x135738:36-37,53-54, `Door_Activate` 0x135be0:96,151-152, `Door_ReactToHit` 0x135980:74-76,101, `Door_IsClosed` 0x1367b0:14 (`kind == 1 → always closed`); outsider `NDrone2_DSTATE_OpenDoor` 0x1681f8:31 (`== 0` guard) [L-read: four sliders 2, swing door 0] |
| obj+0xFD | u8 | bit 3 = straddle/portal-linked | [M*] writer `Door_Create` 0x136100:63-68 (`|= 8` when `tag+0x3C` = param 4 ≠ 0); reader `Door_Link2Portal` 0x136788:7 (`(obj+0xFD >> 3) & 1`), called by `build_LinkDoors2Portals` 0x1c8ac8:91 [L-read: swing door 0x08, sliders 0x00] |
| obj+0xFA | u16 | flags2 `\|= 0xC4` at create (bits 0x4 rotation-changed, 0x40 teleport, 0x80 sphere-from-+0xDC — framework semantics, docs/atlas/object.md) | [M1] `Door_Create` 0x136100:67 |
| obj+0x80/+0x8C | vec3/f32 | gfx sphere centre/radius (`Control_BuildWorldSph` 0x133070 at create, 0x136100:95; rebuilt by the movement pass after every `Door_Interp` because Interp sets obj+0xFA bit 0x20) | [M*] readers `Door_Update` 0x1363c8:51-54,128-131 (both proximity scans: `Vec_SqDist3D(obj+0x80, other+0x80) < 2·(r+r)²`, candidates = classes 2/3/0x12 with `obj+0xF0 & 0x10 == 0`), `Door_Activate` 0x135be0:113 (anti-crush margin vs obj+0x8C), `build_LinkDoors2Portals` 0x1c8ac8:93-103 [L-read: slider obj 0x16e19f0 sphere (151.01, 16.88, 91.13) r 1.280; swing door r 1.280] |
| obj+0x90 | MATRIX | the posed matrix, written by `Door_Interp`/`Door_SetupSwing`/create's slide poses; the sound position obj+0xC0 (row 3) rides with it | [M*] readers `Door_Activate` 0x135be0:75 (`Mat_GetDir` = row 2 for the approach dot), `Door_ReactToHit` 0x135980:78-80 (invert for the door-local test), `Door_SetState` 0x135618:12,19 (Sound_Play3D at obj+0xC0) |
| obj+0x30..0x3C | vec3 | position; copied to prev-pos obj+0x40..0x4C every tick the door is NOT animating (0x1363c8:158-162) | [M1] |

### API callers (every door-function call outside the door module)

| Caller | Calls |
|---|---|
| `parsemap_create_dynamic_objects` 0x1d04f0 | `Door_Create` (placement 0xDB) |
| `ResetMap_GameInit` 0x1d5080:32 | `Door_Init` |
| `control_movement_object_handler` 0x1336f8:23 | `Doors_Calc` (per frame, before the object walk) |
| `Collide_Update` 0x1ea398 | `Door_ReactToHit` via `control_funcs[0x1C].collide` — **not** from Door_Update [ELF: control_funcs[0x1C] @0x26f9e0 = {0x1363c8, 0x135980, 0}] |
| `Player_Activate` 0x1a5170:128 | `Door_Activate(obj, player, 1)` (use key) |
| `NDrone2_OpenDoor` 0x14aa38:11 | `Door_Activate(door, drone_obj, 0)` |
| `Check_Target` 0x1a7810:186-190 | `Door_DoMsg` (gated `DoorMsg < 3`, `DoorMsg` incremented after a bind) |
| `Vision_Recurse` 0x1e82e0:51 | `Door_IsClosed` (vision culling) |
| `AINetwork_DoAStarPath` 0x1571b0:269, `AINetwork_EmitPath` 0x158cf8:233, `AINetwork_Emitter_GetNodeAtDistance` 0x159268:177 | `Door_IsLocked` (AI pathing) |
| `build_LinkDoors2Portals` 0x1c8ac8:91 | `Door_Link2Portal` |

## Param map (from the creator — placement 0xDB; params k at tag+0x2C+4k)

| level_tag param | Instance/object offset |
|---|---|
| p0 (tag+0x2C) | obj+0xF6 kind (0 swing, 1..2 slide) |
| p1 (tag+0x30) | +0x6A group |
| p2 (tag+0x34) | +0x66 open channel |
| p3 (tag+0x38) | +0x68 lock channel |
| p4 (tag+0x3C) | obj+0xFD \|= 8 (portal link) when ≠ 0 |
| p5/p6/p7 (tag+0x40/44/48) | +0x60/+0x62/+0x64 sounds (**0 → 0xFFFF**) |
| p8 \| p9 (tag+0x4C \| tag+0x50) | +0x70 flags (bit 2 = starts open → state 4 + open channel forced on) |
| p10 (tag+0x54) | +0x76 approach (u8) |
| p11 (tag+0x58) | +0x77 no-E-use (u8) |
| tag+0x02 / tag+0x04 | +0x6E frames−1 / +0x54 speed (1/rate, rate 0 → 1/30) |
| tag+0x08 | +0x0C anim id (`m_paths[record+0x44]`) |

(The task brief's probe — "DOORINFO+0x28 = LOAD_LEVEL gate channel from placement param 2,
`lhu v1,56(s1); sh v1,40(s0)`" — is **not a DOORINFO field**. Those two instructions are
`Trigger_Create` 0x1b1720 writing byte **0x28 of the TRIGGER instance** (`__s + 10` on an
`undefined4*` base = byte 0x28) from tag+0x38, and the value is the *original* param 2 only
because `Trigger_LoadLevelCreate` 0x1b2950 shifts the placement params up one slot first.
DOORINFO+0x28 is inside the closed-pose MATRIX (+0x10..+0x4F). See "Port mismatches" #1.)

## Port mismatches (crates/nf-game/src/door.rs + one in triggers.rs; report only)

1. **triggers.rs:51-52 — the trigger gate is labelled "DOORINFO+0x28"**: "The gate is
   `DOORINFO+0x28` = original `param[2]` of the placement (verified against MIPS disassembly:
   `lhu v1,56(s1); sh v1,40(s0)`)". The field is real but belongs to the **TRIGGER instance**
   (`Trigger_Create` 0x1b1720: `*(undefined2 *)(__s + 10) = *(undefined2 *)(param_3 + 0x38)` —
   `__s` is `undefined4*`, so byte 0x28), not to DOORINFO, which has no +0x28 field. The
   disassembly quote and the "original param 2" semantics are correct
   (`Trigger_LoadLevelCreate` 0x1b2950 shifts params one slot up before `Trigger_Create`).
   Comment fix only; the port's `Trigger.gate` behaviour matches the decompile.
2. **door.rs:275-276 with :323-324/:454 — sound param 0 is not mapped to 0xFFFF.** The game:
   `Door_Create` 0x136100:28-42 converts a 0 sound param to 0xFFFF (played by nobody). The port
   stores `snd_open: par(5) as u16, snd_close: par(6) as u16, snd_hit: par(7) as u16` raw and
   gates with `so != 0xffff` (`door.rs:323-324`) / `hit_snd != 0xffff` (`door.rs:454`) — a
   placement whose sound params are 0 or absent produces `SoundEvent(0, …)` on every state
   entry (consumed at drone.rs:4938-4942 `audio.play3d(0, …)`), where the game is silent. On
   07000005 the four locked doors' live +0x60/+0x62 are 0xFFFF [L] (their params were 0), so
   the first time channel 200 unlocks one of them the port would emit sound id 0 on
   open/close.
3. **door.rs:408-417 — the lock rising-edge branch is missing the game's busy re-check.**
   `Door_Update` 0x1363c8:97-100 skips the unlock when `ext+0x70 & 0x10` is set — which it is
   whenever the open-channel block already called `Door_Activate` earlier in the same tick
   (Activate sets busy). The port's `if !was && now { activate; ch_lock = 0; flags |= 0x10 }`
   runs unconditionally, so a door whose open channel and lock channel pulse on the same tick
   gets both the open **and** the unlock in one tick; the game defers the unlock to a later
   tick.
4. **door.rs:441 — group index bounds check is a safe deviation**: the port drops
   `group_states[d.group]` for group ≥ 101; the game writes `DoorGroupsStates[g]` unchecked
   (0x1363c8:170-171; `DoorGroupsStates` 0x316ea0 is 0xCA = 101 u16s) and would corrupt
   adjacent memory for group > 100. Same for `Door_IsClosed` 0x1367b0:18 / `Door_DoMsg`
   0x135f18:29 (unchecked reads). Benign, but the port cannot reproduce out-of-range groups.
5. **door.rs:300-302 — sliding create computes an `inv_closed` the game never has**: the
   game's slide path (0x136100:83-88) inverts the key-0 pose into ext+0x10 and then overwrites
   it with the END pose; ext+0x10 is read only by kind-0 (swing) code. Harmless
   (dead value), listed for completeness.
6. **door.rs:523 vs 0x135be0:138 — swing-side sign boundary**: game `local.z <= 0.0 → −1`,
   port `local[2] < 0.0 → −1` (an activator exactly on the plane gets +1 in the port, −1 in
   the game). Measure-zero nit.
7. **door.rs:533-566 — `react_to_hit` is dead code and its scan is a stand-in** (documented
   U-041): the game's `Door_ReactToHit` walks the obj+0xD0 HITDATA list for a class-3 contact
   or a filtered class-2 contact (`droneExt+0x4F8 & 0x600 == 0 && & 0x100 != 0 && +0xAC > 0
   && !(obj+0xFE & 1)`), then for kind 1 skips even the tail (0x135980:75 `if (sVar1 != 1)`);
   the port's stand-in sphere-scans `visible_actors` and returns early for kind 1 without the
   tail. Never called (`#[allow(dead_code)]`), so not functional today.
8. Documented stand-ins that are NOT mismatches (match the decompile or are flagged in the
   port already): the 2.5-unit `player_use` reach (door.rs:614) vs the game's
   `Collide_SphereIntersect(1.0)` against the mesh; the missing text labels 0x2000003/0x200000b
   (`msg_shown` at +0x74 is written but never read — Door_DoMsg is unported); the
   `DoorMsg<3` gate (0x30cd04) unported; sound positions use the placement translation where
   the game uses obj+0xC0 (identical for every 07000005 door, whose keys keep one position).

Agreements spot-checked this pass (port line → decompile): create param map (door.rs:268-284 →
0x136100), swing re-quantisation (door.rs:290 → 0x135440:46-47), state/channel mirror order
(door.rs:325-334 → 0x135618:25-31), interp snaps incl. swing-rest-at-0.5 (door.rs:349-372 →
0x135738:37-64), channel-on/off/lock/auto-close order (door.rs:397-431 → 0x1363c8:28-157),
activate kinds incl. slide ±1.0 direct (door.rs:474-512 → 0x135be0:96-166), busy/cooldown tail
(door.rs:513-515 → 0x135ee8-0x135ef8), Doors_Calc leader/follower + busy clear (door.rs:570-595
→ 0x135fe0), locked refusal + approach block (door.rs:455-471 → 0x135be0:50-95).

## [L-read] spot checks (ee_start_05.bin; file offset = EE address; obj 0x… → ext = u32 at obj+0xE0)

Walk: `u32 0x2705b4` = first obj 0x1319e30, next = obj+0x14, 523 objects, 5 × class 0x1C.

| Field | Instance | Raw | Interpreted |
|---|---|---|---|
| +0x68 lock channel | ext 0x16e1b00 (obj 0x16e19f0) | `u16 0x16e1b68 = C8 00` | **200** — the locked-door channel; `switch_channels[200]` @0x26fd58 = 0 → locked |
| +0x70 flags | ext 0x16e1fe0 (swing, obj 0x16e1ed0) | `u16 0x16e2050 = 03 00` | 0x0003 — bit 0 spline + the no-reader bit 1 (p8\|p9 = 3); sliders read 0x0002 |
| +0x54 speed | ext 0x16e1fe0 | `f32 0x16e2034 = 25 49 92 3d` | 0.07142857 = **1/14** — `Door_SetupSwing`'s whole-tick re-quantisation of 1/30; sliders read 0.03333334 = 1/30 (`89 88 08 3d`) |
| +0x50 t | ext 0x16e1fe0 | `f32 0x16e2030 = 00 00 00 3F` | 0.5 — the swing door's closed position; sliders 0.0 |
| +0x5C swing sign | ext 0x16e1fe0 | `f32 0x16e2038 = 00 00 80 BF` | −1.0 |
| +0x0C anim | ext 0x16e1fe0 / 0x16e1b00 | `04 14 9D 00` / `00…` | 0x009d1404 (3-key framelist) / 0 (no anim) |
| +0x6E frames | ext 0x16e1fe0 / sliders | `02 00` / `FF FF` | 2 / 0xFFFF |
| obj+0xF4 state, obj+0xF6 kind | obj 0x16e1ed0 | `02 00` / `00 00` | CLOSED, swing (sliders: state 2, kind 2) |
| TheDoors header | 0x316f70 | first 0x16e2180, last 0x16e1b00, count 5, node size 0x80 | creation order = list order; nodes are the exts |

All five doors: +0x60/+0x62 = 0xFFFF, +0x64 = 320, +0x66 = 0, +0x6A = 0, +0x6C/+0x72/+0x74 = 0,
+0x76/+0x77 = 0, +0x78..0x7F = 0, obj+0xFA = 0xC4.

## Live checks

Probe chain (PINE, level 07000005 loaded): `DynamicObjList 0x2705a0 → u32 0x2705b4 = first
obj → obj+0x14 …; class byte obj+0xFF == 0x1C; ext = u32 obj+0xE0`. The five exts at start:
0x16e2180, 0x16e1fe0, 0x16e1e40, 0x16e1ca0, 0x16e1b00 (creation order).

1. **Unlock channel 200** (the four locked sliders): `python -I tools/trace/fieldprobe.py poke
   0x26fd58 1 1 --shot out.png` sets `switch_channels[200]`. Expect within a tick: the four
   exts' +0x68 flip 200 → 0 (`Door_Update` rising edge), obj+0xF4 goes 8 → 0x10 → 2/4, and —
   because they have no anim — nothing moves on screen (their state machine runs invisibly).
   `watch 0x16e1b68 2` on one ext shows the write; the port currently also emits a bogus
   sound id 0 here (mismatch #2).
2. **Auto-close timer** on the swing door (`Door_UNLocked`, ext 0x16e1fe0): `watch 0x16e2052 2`
   while pressing E on it (it has no open channel, flags 0x03). Expect +0x72 to count up only
   while obj+0xF4 == 4 and Bond is outside `√2·(r_door + r_Bond)` of the LIVE obj+0x80, reset
   to 0 on every state change; at 360 the door closes (state 0x10, close sound 284).
3. **t motion** on the swing door: `python -I tools/trace/fieldprobe.py watch 0x16e2030 4` while
   it swings — +0x50 should step by ±1/14 per tick and come to rest at 0.5 (CLOSED) or 0/1
   (OPEN); `poke 0x16e2030 4 0.75 --shot` mid-swing should visibly jump the leaf.
4. Negative check for +0x66: all 07000005 doors have open channel 0, so `watch`ing
   `switch_channels[0]` (0x26fc90) during door use must show no writes from Door_SetState.
