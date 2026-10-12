# DCVars_tag and StateMachineInfo_tag — the drone state-machine context

Proof levels: see [README.md](README.md). Evidence tags: [M] = decompile/disassembly (function + address from the
filename; line numbers where helpful), [M-dis] = verified against the raw MIPS at the address in `ee_start_05.bin` (file offset ==
EE address), [L] = EE snapshot read (address + value). Snapshots used: `work/ee_start_05.bin` (07000005 at start, 13 drones,
frame 108153), `work/ee_start_07000001.bin`, `work/ee_snapshot.bin` (mid-game, one drone dead), `work/ee_start_0700000a.bin`
(no drones — boot).

## Correction to the brief

`obj_tag+0xE0` does **not** point to a separately allocated "state-machine info block". It points to the **`Drone_tag`**, which is
embedded in the same allocation immediately after the 0x110-byte `obj_tag`
[M NDrone2_CreateObj 0x14d7c8:10 `control_create_object(0xd20,…)`; M-dis control_create_object 0x1344e0: `Mem_Malloc(size+0x110)`,
then `*(obj+0xE0) = obj+0x110`; same in Control_CreateObjEx 0x1343c8 for MP drones]. The `StateMachineInfo_tag` is **embedded inside
Drone_tag at +0x108** [M Drone_DCVfromOBJ 0x13a0a0:15 `param_2[3] = drone + 0x108`]. So absolute: `StateMachineInfo == obj+0x218`,
`Drone_tag == obj+0x110`, whole object = one 0xE30-byte malloc (0x110 obj_tag + 0xD20 drone). `obj+0xFF` gates everything: only class
0x02 (drone) / 0x11 (scripted drone variant) build a DCVars [M Drone_DCVfromOBJ 0x13a0a0:11].

There is no per-object SMI allocation and no free list — `Drone_SM_InitObject` (0x175838) just stamps the fields at post-load init
[M NDrone2_PostLoad_Init 0x14d890:21].

## How to reach them (live)

`drone list head` = the u32 at **0x298E14** = first `Drone_tag*` (0 when empty). It is the same cell `LList_Add(0x298E14, drone)`
initialises [M NDrone2_CreateObj 0x14d7c8:30]; the list walks `drone+0x04` = next, and `drone+0x0C` = the drone's `obj_tag*`
[M Drone_EnableAll 0x1383e8:14-20, Drone_MessageObjVicinity 0x13a438:17-46; M-dis Drone_SM_SetState 0x1758e8 `lw a0,-0x71ec(v0)` loads
*0x298E14* then steps `+4` and reads `+0x108`/`+0xC`].
[L ee_start_05.bin: 0x298E14 = 0x016eda50, 0x298E18 = 0x01382cd0 (last), u16 0x298E1C = 13 drones; chain: 016eda50 → 016ecc10 →
… → 01382cd0, each +4 = next, each +0xC = a valid obj].

From any drone: `Drone+0x108` = the StateMachineInfo. From a player-like handle there is no path — drones are only reachable through
the list (or `obj+0xE0` if you hold a drone obj). Snapshot facts for the whole map below were read off these live drones.

## `DCVars_tag` — 0x10 bytes, built on the caller's stack

Built by `Drone_DCVfromOBJ(obj, dc)` [M 0x13a0a0], inline in `Drone_Control` [M 0x139268:43-48], `Drone_Message` [M 0x13a678:28-31],
`DroneVision_HaveOpponentSight` [M 0x175ef0], `DroneAnim_SetEndAIState` [M 0x13dd00], `Drone_SM_SetState`'s bot check [M 0x1758c0:48]
and manually in `Drone_SM_RouteMsg`'s broadcast loop [M 0x172128:128-133]. Signature of every state function:
`fn(DCVars* dc, Drone* drone, obj* obj, MsgObject* msg)` — proven by the dispatch [M NDrone2_ProcessStateMachine 0x175778:11-12:
`StateFuncs[state](dc, dc->drone, dc->obj, msg)`].

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | obj_tag* | the drone's obj | [M 0x13a0a0:13 `*param_2 = obj`; M-dis RouteMsgDCV 0x172048 `lw v1,(s0)` → `sh v0,0xf4(v1)` writes obj+0xF4 through it] — every DSTATE's 3rd param |
| +0x04 | Drone_tag* | the drone (obj+0xE0) | [M 0x13a0a0:14; M-dis 0x171e30 `lw s1,0xc(s0)` = SMI, 0x171ec8 `lw a0,4(s0)` → `lbu 0xc5(a0)` type check] — every DSTATE's 2nd param |
| +0x08 | u32 | copy of `obj+0x20` (the object's cel/collision record pointer — passed to `AINetwork_NavPathForPosition(obj+0x30, obj+0x20, 0)` [M 0x14d890:18-19]) | [M 0x13a0a0:16; M 0x139268:44; M 0x172128:130]. **No reader found** in any DCVars consumer checked (DSTATE files, PreDroneControl, RouteMsgDCV, impact handlers) — carried but unused in this build |
| +0x0C | StateMachineInfo* | `Drone+0x108` | [M 0x13a0a0:15; M-dis 0x171e30] — read by Drone_SM_SetState (0x148db0:85 passes it), RouteMsgDCV, all impact handlers |

Size: 0x10 bytes — 4 consecutive stack words in `Drone_Control` (iStack_60/iStack_5c/uStack_58/iStack_54) and in every manual build;
`Drone_DCVfromOBJ` writes exactly `param_2[0..3]` [M 0x13a0a0:13-16].

## `MsgObject` — 0x1C bytes (stack temp, or a 0x20-byte pooled node)

Pooled: `Drone_SM_Init` (0x1757d0) sets up 0x400 free nodes of 0x20 bytes at `Drone_SM_LLItems` (0x29B6F8, symtab size 0x8000),
free head = u32 at 0x29B6F4 (initially 0x29B718 = node 1; node 0 = the list-head sentinel), free count `Drone_SM_LLDelayed`
(u32 0x29B6E8). Delayed queue head = u32 at 0x29B6F0 (handle; the head cell itself is 0x29B6F8's first word). [M 0x1757d0:10-24;
symtab; L ee_start_05.bin: 0x29B6E8 = 0x400, 0x29B6EC = 0x29B6F8, 0x29B6F0 = 0x29B6F8, 0x29B6F4 = 0x29B718, 0x29B6F8 = 0
(queue empty), free chain intact at 0x29B718/0x29B738.]

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | **message id** (what the DSTATE `switch(*msg)` reads) | [M every DSTATE; M-dis SendMsgSelf 0x175a40 `sw a0,(sp)`] |
| +0x04 | u32 | **state filter**: 0 = any state; 0xC5 = BotGlobal; otherwise must equal the target's current state or the message is **dropped** | [M-dis RouteMsgDCV 0x171e28-44: `lw v1,4(s3)`; `bne msg+4, SMI+4 → 0x172104 return`] |
| +0x08 | u32 | data word 1 (for self-msgs: the sender's SMI instance id — `*smi`) | [M SendMsgSelf 0x175a58-60 `lw v0,(a3); sw v0,8(sp)`; BroadcastMsg 0x175aa8 `uStack_28 = param_4`] |
| +0x0C | u32 | **destination**: the target SMI's instance id; 0 = broadcast to every drone on the list | [M RouteMsg 0x172128:106 `uVar4 = *(msg+0xC)`, compares against `*(drone+0x108)` 0x172128:150-158; SendMsgSelf 0x175a6c] |
| +0x10 | u32 | frame the message was sent (`GameState+0x34` frame counter, cell 0x2A379C) | [M-dis 0x175a78 `sw t0,0x10(sp)` where t0 = `lw 0x379c(0x2a0000)`] [L ee_start_05.bin: 0x2A379C = 108153] |
| +0x14 | u32 | **deliver-at frame** (send frame + delay); the delayed queue is sorted and drained on this | [M-dis 0x175a68 `addu a0,t0,a2; sw a0,0x14(sp)`; RouteMsg 0x172128:68 queues while `frame < msg+0x14`; sort key = node+0x18 = msg+0x14 0x172128:92; SendDelayedMsgs 0x1759a0:17-18] |
| +0x18 | u32 | **data word 2** — the payload: forced-state id (0x1D), HITDATA*/DroneImpact*/obj* for impacts/timers/explosives, or 0 | [M DSTATE_Global 0x15b830:62 `param_4[6]` = state for msg 0x1D; HandleImpact 0x1469b8:37 `param_2[6]` = HITDATA*; PreDroneControl 0x148db0:106-122; M-dis SendMsgSelf 0x175a70 `sw a1,0x18(sp)`] |

Node copy in the delayed queue: node+0x00 next, node+0x04..0x1F = the 0x1C-byte MsgObject [M RouteMsg 0x172128:76-103,
M-dis SendDelayedMsgs `piVar6[6]` = msg+0x14].

## `StateMachineInfo_tag` — 0x2C bytes, embedded at `Drone+0x108` (== obj+0x218)

Initialised by `Drone_SM_InitObject(obj)` [M 0x175838; M-dis 0x175848-90] and driven by `Drone_SM_SetState` /
`Drone_SM_RouteMsgDCV`. Offset below is **SMI-relative**; the absolute Drone offset is +0x108+k.

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | **instance counter** — a **u16** at 0x299090, incremented per `Drone_SM_InitObject` (never reset except `Drone_LevelReset` 0x13a5a8:15); used as the message address (msg+0x0C), the anim-variant seed (`SetWalkIdleAnim` 0x13cc70:17) and a stable ordering key (`Steer` 0x14eba8:162) | [M 0x175838:12-14; M-dis 0x175858 `lhu v0,0x280(a3)` / 0x175874 `sh` — a3 = 0x298E10, so u16 @ **0x299090**; the u32 read in the decompile is Ghidra's guess] [L ee_start_05.bin: u16@0x299090 = 13 == drones created; 07000001: 15] |
| +0x04 | u32 | **current state** — the id passed to the state fn; mirrored to `obj+0xF4` (u16) on every change | [M-dis RouteMsgDCV 0x171ea0 `lw a1,4(s1)` = state arg to the fn-ptr call; 0x172030-50 `sw v1,4(s1)` … `sh v0,0xf4(obj)`] [L ee_start_05.bin: first drone 016eda50 SMI+0x10C = 4 == obj(016ed940)+0xF4 = 4] |
| +0x08 | u32 | **previous state** (old current, written before the swap) | [M-dis 0x172020-30 `lw v0,4(s1); … sw v0,8(s1)`] [L ee_start_05.bin: guards show cur=4 (Idle) prev=6 (InitPatrol); ch-waiters cur=1 prev=0 (never changed)] |
| +0x0C | u32 | **queued/next state** — the ONLY slot `Drone_SM_SetState` writes (with the pending byte) | [M-dis SetState 0x175978 `sw s0,0xc(s1)`; consumed by RouteMsgDCV 0x172028 `lw v1,0xc(s1)`] [L ee_start_05.bin: next == cur on every idle drone] |
| +0x10 | u32 | **impact resume state** — the state to return to after a stun/knockdown; written by the impact handlers, read back into the stun anim's argument (`DroneAnim_CallAnim(0,0x51,0,resume,…)` PunchImpact 0x145f28:90-91, ExplosiveImpact 0x146480:204-205) and by the per-state `HandleImpact` resume logic | [M PunchImpact 0x145f28:41 `*(iVar1+0x118) = uVar6`; HandleImpact 0x1469b8:52,101,168,190,218,283,329; BulletImpact 0x1461c8:85; ConsiderExplosive 0x1431c0:188; InitObject sets it to the initial state 0x175898] |
| +0x14 | u32 | **frame the current state was entered** (`GameState+0x34`), written before the ENTER message | [M-dis 0x1720cc-d0 `sw v0,0x14(s1)` (v0 = frame); readers: DroneFunc_CombatState 0x148338:246 (`+120 frames`), PlayScript 0x15ba68:107-110, AimCrouchFire 0x167060:79-82, InitComms 0x139520:223-240] [L ee_start_05.bin: first drone +0x11C = 108150, frame 108153 → 3 ticks in Idle] |
| +0x18 | u8 | **change-pending byte**: SetState writes 1; RouteMsgDCV clears it and re-checks after every ENTER (loop while set) | [M-dis SetState 0x17597c `sb v0(=1),0x18(s1)`; RouteMsgDCV 0x171f28 `sb zero,0x18(s1)`, reads 0x171ef4/0x1720f4 `lb v0,0x18(s1)`] [L ee_start_05.bin: 0 on all 13 drones] |
| +0x1C / +0x20 | — | **unreferenced** (no read or write found anywhere in the 774 drone-family decompile files) | census `\+ 0x124\)` / `\+ 0x128\)` → 0 hits |
| +0x24 | u32 | **transition argument ("channel")** — SetState's 3rd parameter, stored for the new state's ENTER handler to read; consumed only by `AllyLeadMissionWait` (0x15efc8:17-21: 0 → default channel 0x99 → drone+0xCD8). Cleared to 0 by InitObject; `ExplosiveImpact` passes its `DroneImpact*` here in one path (0x146480:230) — nothing reads it then | [M-dis SetState 0x175974 `sw s2,0x24(s1)`; M AllyLeadMissionWait 0x15efc8:17; M InitObject 0x175884 `sw zero,0x12c(v1)`] |
| +0x28 | fn ptr | **process function** = `NDrone2_ProcessStateMachine(DCVars*, uint state, MsgObject*)` — the only SMI reader-dispatcher is `Drone_SM_RouteMsgDCV` | [M InitObject 0x17588c `sw 0x175778,0x130(v1)`; M-dis RouteMsgDCV 0x171e70/90 `lw v0,0x28(v0); jalr v0`] [L ee_start_05.bin: 0x00175778 at +0x130 on all 13 drones] |

Size: **0x2C bytes** (0x108..0x133 inclusive of Drone). The next drone field starts at Drone+0x134 = the placement-prop byte
`props[0x10]` (the WaitSwitch channel; `NDrone2_DefaultInit` 0x14b300:463-464 writes it, WaitSwitch 0x15b960:22 and EnableAll
0x1383e8:41 read it) — the SMI cannot extend past it. Fields +0x1C/+0x20 are untouched, so the C struct plausibly pads the +0x18
byte out to +0x24. Every reader/writer of +0x108..+0x134 found in the drone-family census is listed in the rows above; they agree.

## The runner (who calls what, per frame)

1. `control_movement_object_handler` (0x1336f8:21) → `Drone_InitComms` (0x139520) → **`Drone_SM_SendDelayedMsgs`** (0x139520:261 →
   0x1759a0): pops every queued message whose deliver-at frame ≤ now and re-routes it through `Drone_SM_RouteMsg`.
2. Per drone: `Drone_Control` (0x139268) builds the DCVars on its stack (obj, drone, obj+0x20, drone+0x108) [M 0x139268:43-48] and calls
   `NDrone2_PreDroneControl` (0x148db0) — converts timers into messages: idle timer `Drone+0x104` → **msg 4** (filter = current
   state, 0x148db0:95); timer C/D (`+0xCC0/+0xCC4/+0xCC8`, `+0xCCC/+0xCD0/+0xCD4`) → **msg 0xC / 0xD** (payload = `+0xCC8`/`+0xCD4`,
   0x148db0:106-122) — all via `Drone_SM_SendMsgSelf`; also a live switch-channel may `Drone_SM_SetState` directly (0x148db0:85).
3. `NDrone2_ControlSTANDARD` (0x1490f0:81) sends **msg 3** (the per-frame update) via `SendMsgSelf(3,0,0,0,dc)` — then finding the
   opponent, tracking, movement, collision, firing.
4. **`Drone_SM_SendMsgSelf`** (0x175a38, M-dis 0x175a38-a0): builds the MsgObject `{id, filter, *smi, *smi, frame, frame+delay, data}`;
   delay 0 → **`Drone_SM_RouteMsgDCV(dc, msg)` directly (nested, immediate)**; else `Drone_SM_RouteMsg` (queue).
5. **`Drone_SM_RouteMsg`** (0x172128): if `frame < msg+0x14` → pool-alloc a node, insert sorted by msg+0x14. Else resolve the target:
   msg+0x0C == 0 → walk the drone list (skip drones whose current state `+0x10C` ∈ {1, 3, 0xF, 0x47, 0x48}) and deliver to each,
   rewriting msg+0x0C per drone; else find the drone whose `+0x108` == msg+0x0C (MP: negative ids → player objects) →
   `Drone_DCVfromOBJ` → `Drone_SM_RouteMsgDCV`. Filter re-check per target: msg+0x04 ∈ {0, 0xC5, current state} else drop
   (0x172128:163-176).
6. **`Drone_SM_RouteMsgDCV`** (0x171df8, M-dis 0x171df8-0x172104) — the generic SM runner, the only reader of SMI+0x28:
   - drop if msg+0x04 ∉ {0, 0xC5, SMI+0x04} (0x171e28-44);
   - call `smi->fn(dc, SMI+0x04, msg)` — the current state (0x171e90-…);
   - if it returns 0 (unhandled): call `smi->fn(dc, 0 or 0xC5, msg)` — **state 0 = Global** fallback, 0xC5 = BotGlobal for MP bots
     (0x171ec8-f0); bots additionally get fn(dc, 0xC5, msg) on every msg 3 (0x171e50-88);
   - if the pending byte SMI+0x18 is set, run the **change loop** (0x171f10-0x1720f4):
     clear pending; bot-only state-type gates (`BOTSTATE_getStateType` 0x1291f0, jal 0x171f3c); death-flag fixups —
     next state ∈ {0x44 Death_Anim, 0x46 SpecialDeath, 0x47 Dead, 0x55 BulletImpact} → `drone+0x3D = 1`, else `drone+0x3B = 0`
     (0x171fb4-0x171ffc); call `fn(dc, old_current, &{msg=2})` **exit** (0x172000-18, local msg = {2, <uninitialised +0x4>,
     inst, inst, msg+0x18}); then `SMI+0x08 = old current; SMI+0x04 = SMI+0x0C; obj+0xF4 = new (u16)` (0x172020-50);
     `drone+0x4F8 &= ~0x10000` and `&= ~0x2000` (0x172054-70); `drone+0x14C = 1.0f` (0x172074-78); timers C and D cleared
     `drone+0xCC0 = drone+0xCCC = 0` (0x17207c-88); bots: `fn(dc, 0xC5, &{0x2E, new state})` (0x1720a0-c4);
     `SMI+0x14 = frame` (0x1720cc-d0); call `fn(dc, new current, &{msg=1})` **enter** (0x1720d4-f0); loop while pending again.
   - the exit/enter local messages only set msg+0x00, +0x08, +0x0C, +0x18 — **msg+0x04 is whatever was on the stack** (never written
     before the call; M-dis shows no store to 4(sp) on that path). Harmless because these are delivered straight through the fn
     pointer, bypassing the filter — but a state fn that read msg+0x04 on enter/exit would read garbage (none found).
7. `DroneAnim_CallHandler` (0x13add0) → `DroneAnim_CallFullyComplete` (0x13dcb8) → `DroneAnim_SetEndAIState` (0x13dd00): on animation
   completion, optional `SetState(drone+0x108, endState)` and optional `SendMsgSelf(endMsg)` — the msg-5 path.
8. `Drone_SM_SetState` (0x1758c0, M-dis 0x1758c0-98): `SetState(smi, state, arg)` → MP-bot veto check (walks the drone list matching
   SMI+0x00, `BOT_validateStateChange` 0x124fb0, plus a `drone+0xD1C→+0x762` script escape) then
   `SMI+0x24 = arg; SMI+0x0C = state; SMI+0x18 = 1`. It does **not** touch the current state — application always happens in
   RouteMsgDCV after the in-flight message finishes.

## Message IDs (sender → consumer, one citation each)

| ID | Meaning | Sender [M] | Consumer [M] |
|---|---|---|---|
| 0 | benign/no-op ("handled"); MP bots get it on player-status changes | `MP_setPlayerStatus` 0x18a518 (`SendMsgSelf(0,0,0,0xC5,dc)`); broadcast form: `Drone_Message(0,…)` → `Drone_SM_BroadcastMsg` | `DSTATE_Global` 0x15b830 case 0 → return 1; `WaitSwitch` 0x15b960 case 0 → 1 |
| 1 | **enter** state | `Drone_SM_InitObject` 0x175838:23 `SendMsg(1,0,inst,inst)`; RouteMsgDCV 0x1720d4 | every DSTATE case 1 (163 fns); e.g. `WaitSwitch` 0x15b960:14-16 → `NDrone2_Enable(0)` |
| 2 | **exit** state | RouteMsgDCV 0x172000-18 (`sw 2,(sp)` before the old-state call) | 61 DSTATEs; e.g. ported `UnderCoverInit` 0x169cc0 case 2 |
| 3 | **per-frame update** | `NDrone2_ControlSTANDARD` 0x1490f0:81 | 142 DSTATEs; e.g. `Idle` 0x15bf18 |
| 4 | idle timeout | `PreDroneControl` 0x148db0:95 (timer `Drone+0x104`); `ReachedDestNode` 0x153358 (delay 1, filter = current state) | `Idle`/`Alert`/`Patrol`/`WaitSwitch`/… (6 DSTATEs), `DSTATE_Idle` 0x15bf18 |
| 5 | animation completed | `DroneAnim_CallFullyComplete` 0x13dcb8 → `SetEndAIState` 0x13dd00 (msg = `drone+0x598`, queued by `DroneAnim_CallAnim`) | 15 DSTATEs; e.g. `PlayScript` 0x15ba68 case 5 |
| 6 | punch impact (HITDATA* in msg+0x18) | `Drone_Message` from `NDrone2_DealWithObjHit` 0x13fff0 (fists / 0x4B / 0x4D weapons) | `HandleImpact` 0x1469b8 case 6 → `PunchImpact` 0x145f28 |
| 7 | taser / soft hit (flinch) | `DealWithObjHit` 0x13fff0 (weapons 0x4A/0x4C) | `HandleImpact` case 7 (flinch timer `Drone+0xBC`) |
| 8 | bullet impact (HITDATA*) | `Drone_BulletHit` 0x13a9f8 `SendMsgSelf(8, hitdata, 0, 0)` | `HandleImpact` case 8 → `NDrone2_BulletImpact` 0x1461c8 |
| 9 | explosive impact (DroneImpact*) | `Drone_ExplosiveHit` 0x13aab8 `SendMsgSelf(9, impact, 0, 0)` | `HandleImpact` case 9 → `NDrone2_ExplosiveImpact` 0x146480 |
| 0xA | self wake / repeat timer (delayed) | `DroneVision_HaveOpponentSight` 0x175ef0 (`10,0,FRAME<<3,0` — type-4 flyers, music event 0xF); `HostageKillerAttack` 0x15c940 (`10,0,FRAME,0`); `EnableAll` 0x1383e8:27 broadcasts it 8 frames after an NIS ends | `DSTATE_Global` 0x15b830 case 0xA (fall check → state 9); `HostageKillerAttack` re-arms it |
| 0xB | "die now" to the hostage (delivered only while the target is in state 0xA Hostage) | `HostageKiller` 0x15c728 / `HostageKillerAttack` 0x15c940 / `DroneAnim_EventFunc` 0x13b750: `SendMsg(0xB, 10, own_inst, hostage_inst)` | `DSTATE_Hostage` 0x15cb78 case 0xB |
| 0xC | timer C (payload = `Drone+0xCC8`) | `PreDroneControl` 0x148db0:106-109 | 54 DSTATEs (death channel raise: `Death_Anim` 0x1637f8); payload read as `msg+0x18` |
| 0xD | timer D (payload = `Drone+0xCD4`) | `PreDroneControl` 0x148db0:119-122 | 3 DSTATEs (sniper cover return etc.) |
| 0xE | forced re-enable (level NIS ended) | `Drone_EnableAll` 0x1383e8:42 per state-1 waiter with `drone+0x134 == 0x0A` | `WaitSwitch` 0x15b960 case 0xE → Enable(1) + start state |
| 0xF | first sight of the opponent | `DroneVision_HaveOpponentSight` 0x175ef0 (first time `+0x228 & 8` sticks) | 26 DSTATEs; `Global` case 0xF unhandled → drop |
| 0x11 | first-sight alert broadcast (delay 0x1E; msg+0x08 = sender inst; data 0x2990A0) | `DroneFunc_FirstSightState` 0x147c48:84 `BroadcastMsg(0x11, 0x2990A0, 0x1E, own_inst)` | `DroneVision_EnemyAlerts` 0x177c18 consumers |
| 0x12 | hurt alert broadcast (same shape) | `DroneFunc_SendHurtMessage` 0x145e28; `ExplosiveImpact` 0x146480 | `EnemyAlerts` |
| 0x14 | sound alert handled | `DroneFunc_HandleSoundAlerts` 0x148a30 | ported as `msg::SOUND` |
| 0x15 | alert-to-object | `NDrone2_DroneAlertToObject` 0x14a538 `BroadcastMsg(param_5, 0x2990A0, 0x1E, corpse_inst)` | `DSTATE_AlertToPosition` 0x165270 |
| 0x17 / 0x18 / 0x19 | near-miss impacts (explosion near-miss / bullet whizz-by with LOS / punch-family) | `Bullet_DoTrails` 0x12d890 → `Drone_MessageObjVicinity(20.0, 0x18, obj, 0, 0)` for 0x18 | `HandleImpact` cases 0x17/0x18/0x19 (0x1469b8:197/250/315) |
| 0x1A | mission monitor talk | `NDrone2_MonitorTalkToMissionObj` 0x144340 | `KikoMission` family |
| 0x1D | **forced state change — new state id in msg+0x18** | generic (anim/script senders via `SetEndAIState`) | `DSTATE_Global` 0x15b830:59-67 `SetState(smi, msg[6], 0)` |
| 0x1E | alarm broadcast (while `switch_channels[0x96]`) | `Drone_InitComms` 0x139520 `Drone_Message(0, 0x1E, 0, 0)` | 29 DSTATEs (`RunToAlarm` chain) |
| 0x1F | alerted-by-ally | `DroneVision_ConsiderAlerted` 0x176250 `Drone_Message(obj, 0x1F, 0, 0)` | 27 DSTATEs (→ Attack) |
| 0x20 | open that door (obj in msg+0x08) | `DroneAnim_EventFunc` 0x13b750 `SendMsg(0x20, 0, obj, obj)` | `DSTATE_OpenDoor` 0x1681f8 |
| 0x21 | explosive landed near (obj* in msg+0x18) | `DroneFunc_HandleExplosives` 0x1435f0 `SendMsgSelf(0x21, obj, 0, 0)` | 110 DSTATEs → `DroneFunc_ConsiderExplosive` 0x1431c0 |
| 0x22 | hit by thrown/picked object | `DealWithObjHit` 0x13fff0 | 2 DSTATEs |
| 0x23 / 0x24 / 0x25 | turned / search-look step / search done | search states | `SearchArea` 0x165bf0 |
| 0x2E | bot state-change notice (msg+0x04 = new state) | RouteMsgDCV internal 0x1720a0-c4 | `BotGlobal` 0x16e060 case 0x2E |
| 0x2E–0x45, 0xD4–0xDF | MP-bot protocol (goal/impact/weapon…) | `MP_sendBotMessage` 0x18bb60 (filter 0xC5), `MP_setPlayerStatus` 0x18a518 (0x3B) | `BotGlobal` 0x16e060 only |

Unhandled ids: state fn returns 0 → one Global (0) / BotGlobal (0xC5) retry → if that also fails, dropped; pending-byte loop still runs.

## State table

`NDrone2_StateFuncs` = 250 fn pointers at **0x29B300** (symtab size 1000; `ProcessStateMachine` guards `state < 0xFA`)
[M 0x175778:10-12]. Dumped from ee_start_05.bin and name-resolved through action-symtab.tsv: entries 0x00–0xF9 map 1:1 to
`NDrone2_DSTATE_<Name>` in order (Global, WaitSwitch, Disabled, PlayScript, Idle, Alert, InitPatrol, Patrol, HostageKiller,
HostageKillerAttack, Hostage, … , SniperIdle 0x29, …, BotInit 0xC3, BotRespawn 0xC4, **BotGlobal 0xC5**, BotCollector …,
BotIdle 0xF9). Spot checks against the live drones: cur=4 → Idle, cur=7 → Patrol, cur=1 → WaitSwitch, cur=41 (0x29) → SniperIdle —
all [L ee_start_05.bin / ee_snapshot.bin @0x29B300+k·4].

## Disagreements

1. **`Drone_SM_RouteMsg`'s broadcast walk** starts at the *value* of 0x298E14 (first drone) and steps `+4` — consistent with
   `LList_Add`'s node layout (drone+0 prev, +4 next) and with every other walk (EnableAll, MessageObjVicinity, SetState). No
   disagreement, but note the decompile renders it as `DAT_00298e14 + 0x108` — that is `*(first_drone) + 0x108`, **not** the static
   address 0x298F1C (M-dis 0x1758e8/0x172128: `lw a0,-0x71ec(0x2a0000)` then `lw v0,0x108(a0)`).
2. **Instance counter width**: the decompile reads `DAT_00299090` as u32; the emitted code is `lhu/sh` at 0x299090 — it is a **u16**
   (the upper half of a 32-bit read at that address belongs to a neighbouring variable: a u32 read gave 0x1000D in ee_start_05 where
   the u16 is 0x000D). Any port/model must use u16.
3. **msg+0x04 on the synthetic exit/enter messages is uninitialised** (never stored before the direct fn-ptr call). No current reader
   is affected; flagged so a future reader doesn't assume 0.
4. **SMI+0x1C/+0x20 unmapped**: no access found; they may be padding after the +0x18 byte or genuinely dead fields. Size is proven
   ≥ 0x2C bounded by the next drone field at +0x134, and ≤ 0x2C by the same argument — the C struct is 0x2C only if +0x1C/+0x20 are
   members (e.g. a 3-word hole); a 0x28-byte struct + alignment would place the next field at +0x2C anyway. Either way the byte
   layout of every *used* field is proven.
5. **docs/systems/drone-ai.md** says "State-machine info at `Drone+0x108`" — correct, but it is a *tag in place*, not a pointer
   target; `obj+0xE0` is the Drone_tag pointer (see correction above). No numeric disagreement with the doc's +4/+8/+0xC/+0x14/+0x18
   claims — all proven here.

## Port cross-check (`crates/nf-game/src/drone.rs`) — observations only, nothing fixed

- **Modelled correctly**: `set_state` only queues (`next_state = Some(s)`, drone.rs:2439); `send` = RouteMsgDCV — deliver to
  current, Global fallback, then the exit→swap→enter loop with re-drain of `next_state`, `prev_state`/`entered` updates and the
  0x4F8 0x10000 clear (drone.rs:2448-2465); `impact_resume` models SMI+0x10 (drone.rs:522,539,1416); msg set matches (module `msg`,
  drone.rs:152-181); delayed single message + immediate self_msg (drone.rs:447,486,5022).
- **drone.rs:2456-2458 (in `send`)**: on a state change the game clears **both** timer bytes `+0xCC0` **and** `+0xCCC` (M-dis
  0x17207c-88) and resets `drone+0x14C = 1.0` (0x172074-78). The port clears `timer_c` + `idle_timeout` but leaves **`timer_d`
  armed** across the transition (it is only cleared at expiry, drone.rs:5020), and does not model the +0x14C reset.
- **drone.rs:2451-2464**: the game also does the death-flag fixups on every change — next ∈ {0x44,0x46,0x47,0x55} → `drone+0x3D = 1`,
  else `drone+0x3B = 0` (M-dis 0x171fb4-0x171ffc) — and clears the 0x4F8 **0x2000** bit (0x172060-70). The port models neither
  (its `exiting_to` only feeds UnderCoverInit's EXIT arm).
- **drone.rs:3236** (`LEAD_MISSION_WAIT` enter): `wait_channel = self.second_channel or 0x99`. The game reads **SMI+0x24** — the
  channel passed to the `SetState` that queued 0x21 — and the only caller (`ReachedDestNode` 0x153358:92) passes **0**, so the game
  always uses 0x99. A drone with `second_channel != 0` would wait on the wrong channel in the port.
- **No msg state-filter**: the game drops a message whose msg+0x04 ∉ {0, 0xC5, current state} (M-dis 0x171e28-44). Concretely, msg 4
  is sent with filter = the *sending-time* current state (0x148db0:95; `ReachedDestNode` 0x153358) and may sit in the queue (delay 1)
  across a state change — the game then silently drops it; the port delivers `IDLE_TIMEOUT` into whatever state is current
  (drone.rs:919 → 5022). The port's `delayed`/`send` carry no filter field at all.
- **drone.rs:4119**: `sm_id` = the drone's list index; the game's instance counter (SMI+0x00) is the monotonic u16 at 0x299090 that
  keeps counting past dead drones, and `DroneAnim_SetWalkIdleAnim` (0x13cc70:17) seeds the idle anim with `(inst+1)%3` — the port's
  anim pick can differ once a drone has died.
- Minor: game timers 0xC/0xD carry a payload word (msg+0x18); the port models TIMER_C/TIMER_D as bare messages with state fields
  (e.g. `death_pending`, drone.rs:1686-1687) — equivalent for the states ported so far.
- Minor: in the game a `SendMsgSelf(delay 0)` from inside a state handler is processed **immediately, nested, before the rest of
  that handler runs** (direct `RouteMsgDCV` call, M-dis 0x175a90); the port defers `self_msg` until the handler returns
  (drone.rs:2450, 2462).

## Live checks for the main agent

All addresses are EE addresses (PINE / gdb-server). Best situation: **07000005 at level start** (the ee_start_05 state): 13 drones;
guards 1-4 in Idle(4) from InitPatrol(6); five ch-waiters in WaitSwitch(1) on channels 34/164/67; drone 2 (016ecc10) in
HostageKiller(7); the sniper as the LAST list node (inst 13) in SniperIdle(0x29).

1. **Find a live instance (PINE `find`/`ptrs`)**: read u32 0x298E14 → first Drone_tag* (expect 0x016eda50 on 07000005 start;
   0 when the level has none, e.g. boot 0700000A). Chain: drone+0x04 next, drone+0x0C obj (obj+0xE0 must point back at the drone,
   obj+0xFF = 0x02). Verify SMI: drone+0x130 == 0x00175778 (`NDrone2_ProcessStateMachine`) on every drone.
2. **Census (PINE `dump` / `watch`)**: `drone+0x108` (inst, increments 1..13), `+0x10C` (cur), `+0x110` (prev), `+0x114` (next),
   `+0x118` (resume), `+0x11C` (entered frame), `+0x120` byte (pending), `+0x12C` (channel arg), and u16 **0x299090** == number of
   SMIs ever created this level (13).
3. **gdbwatch census, readwrite, a few seconds of play**:
   - `watch w drone+0x114` (queued state): the ONLY writer should be `Drone_SM_SetState+0xB8` (0x175978). Expect bursts of
     0x56/0x58/0x68 when a guard spots Bond, 0x47/0x48 on kills.
   - `watch w drone+0x10C` (current): the ONLY writer should be inside `Drone_SM_RouteMsgDCV` (0x172030), immediately preceded by a
     write of the same value to `drone+0x110` (0x172030 vs +0x110 store at 0x172020-30) and a u16 store to obj+0xF4 (0x17204c-50).
   - `watch r/w drone+0x120` (pending byte): writes of 1 from 0x17597c (SetState), writes of 0 and reads from 0x171f28 /
     0x171ef4 / 0x1720f4 (RouteMsgDCV). Nothing else should touch it.
   - `watch w drone+0x118` (resume): writers only in the impact handlers — 0x145f41 (PunchImpact), 0x1461c8-block (BulletImpact),
     0x146480-block (ExplosiveImpact), 0x1469b8-block (HandleImpact), 0x1431c0-block (ConsiderExplosive) — plus the init write
     0x175898. Shoot/punch a guard to see it fill with the pre-impact state, then Stunned states consume it.
   - `watch w 0x299090` (u16): writes only from `Drone_SM_InitObject+0x3c` (0x175874), on drone creation.
   - `watch w 0x29B6F8` (delayed-queue head): writes from `Drone_SM_RouteMsg` (insert 0x172093-0x172208) whenever an alert broadcast
     (0x11/0x12/0x15, delay 0x1E) or a delayed self-msg is queued; drains from `Drone_SM_SendDelayedMsgs` (0x1759a0) each frame.
4. **Poke-style verifications** (pin + screenshot, per README lessons — these fields are recomputed, so poke inputs, not the fields):
   - instance id → message routing: hard to poke safely; instead watch `msg+0x0C` resolution by breaking on `Drone_SM_RouteMsg+0xE8`
     (0x172138) with a data breakpoint on 0x29B6F8 after throwing a grenade near two guards (0x21/0x12 traffic).
   - queued-slot visibility: set a gdb `awatch` on drone+0x114 and provoke a state change (fire one shot at a patrolling guard):
     expect exactly one 0x175978 hit, then a 0x172028 read, then the value re-copied into +0x10C/+0xF4 within the same frame.
   - what the screen/state should show: guard transitions Idle→Alert/Attack within ~1 s of being shot (61-tick reaction on
     07000005); after death the drone's +0x10C walks 0x44/0x46 → 0x47 → 0x48/0x49 while `+0x110` trails one step behind; the sniper
     (last node) stays in 0x29 until Bond is visible.
5. **Never-true sanity probes**: `drone+0x124` / `drone+0x128` should stay 0x00000000 for a whole level (unmapped fields — if a
   write shows up, the census missed a reader; report back). `+0x12C` returns to 0 after InitObject unless some SetState caller
   passed a nonzero third argument (only ExplosiveImpact's pointer path could).

## Port fixes driven by this map (044y-fields-actors)

- Every state change now clears timer D as well as timer C and resets the hit-effect multiplier (`+0x14C` = the
  port's `damage_taken`) to 1.0 [M-dis 0x17207c-88]; test `fields_actors_live::a_state_change_clears_both_message_timers`
  (fails before the fix). The idle-timeout clear stays (the game re-arms it per enter — *suggested*).
- `LEAD_MISSION_WAIT` latches its wait channel from `Drone+0x12C` (new `mission_wait_channel` field; 0 → 0x99)
  [M 0x15efc8:17-20] — the port previously read `second_channel` (+0x135), a different field.
- The SMI+0x24 channel arg of `Drone_SM_SetState`: the only writer into the 0x21 path passes 0 (ReachedDestNode
  0x153358:92), which this map first read as "the game always waits on 0x99" via the wrong slot — corrected by the
  `Drone+0x12C` reader in 0x15efc8 (see docs/fields/drone.md).
- Not fixed (documented): the msg state-filter and the monotonic instance counter (drone.rs `send`); self-msgs stay
  deferred vs the game's immediate nested dispatch; SMI+0x1C/+0x20 do get one writer (`DroneFunc_DoForcedAttack`
  0x148c60:31 writes SMI+0x1C = 0x56) — the port has no forced-state message yet.
