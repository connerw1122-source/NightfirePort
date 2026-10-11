# Struct 9 — the switch channels

Not a heap struct: five global arrays plus the object-side channel fields that index them.

## Storage

| symbol | addr | size/type | role |
|---|---|---|---|
| `switch_channels` | 0x26fc90 | u8[256] | live state |
| `switch_channels_prev` | 0x26fd90 | u8[256] | snapshot N−2 |
| `switch_channels_hold` | 0x26fe90 | u8[256] | snapshot N−1 |
| `switch_channels_MusicVars` | 0x26ff90 | u16[256] | per-channel alarm counters (DroneFunc_CheckAlarmRaised 0x1453a8: elems 0xA4 on 0x07000005, 0x37 on 0x07000006, 0x61 otherwise) |
| `switch_channels_time` | 0x270190 | u32[256] | last-set timestamp (GameState+0x34 frame counter) |

All [S]ymtab-confirmed and code-referenced [M]. The `Channels` symbol @0x31aa20 is **unrelated**
(animation transform-channel ids read by `AnimFrameCopy` 0x1c5668 at 0x1c5884/0x1c6144) [M].

`Init_SwitchChannels` (0x134310) [M, disasm 0x134348-0x134378]: per iteration `sw` → 0x270190
(time pre-seeded with the value at GameState+0x34 @0x2a379c — **not** zeroed), `sh` → 0x26ff90
(u16s, 512 B), `sb` → 0x26fc90 / 0x26fd90 / 0x26fe90. Exactly one caller in the ELF:
`jal 0x134310` @0x1d5208 (level-load path; not in the decompiled set) [M]. Persistence:
`Player_RamSave` (0x195670) saves all five arrays (time 0x400 B @0x2b9a44, live/hold/prev bytes
@0x2b9e8d/0x2b9f8d/0x2ba08d); `Player_RamLoad` (0x1957a8) restores them [M].

## Frame order [M]

`control_movement_object_handler` (0x1336f8):

```
0x133730  Drone_InitComms
0x133740  jal Doors_Calc            (0x135fe0)
0x133748  jal Trigger_Calc          (0x1b1900)
0x133770-0x1337b8  static-object cel-swap loop: reads ch[o->0xA2] vs hold[o->0xA2]
                  (lbu @0x1337a0/0x1337a4, bnel @0x1337a8), swaps the cel on mismatch
0x1337cc  amemcpy(0x26fd90, 0x26fe90, 0x100)   prev := hold
0x1337dc  amemcpy(0x26fe90, 0x26fc90, 0x100)   hold := current
0x1337e4+ dynamic-object loop via control_funcs[obj->0xFF] (newest first)
```

Consequences [M]: a channel set during frame N's object updates is visible (live array) to every
reader in the same pass; the hold snapshot captures it for frame N+1's cel-swap loop and
`switch_channels_prev` for frame N+2's — `Player_Weapon` (0x1a24c8 :247) and `Door_Update`
(0x1363c8 :89, the lock rising edge) are the `_prev` readers [M]. A set-then-cleared-within-one-
update-window pulse never reaches hold/prev at all. `switch_channels_time` is written by the
setters (below) and read by Trigger_Update's case-4 window and the save/load path only [M, audit:
Player_RamSave bulk-copies it].

Setters stamp `switch_channels_time[ch] = GameState+0x34` in most but not all paths: trigger
writes stamp on set [M]; `Switch_Update` stamps on the **falling** edge (0x1adf98 toggle,
timestamp @0x1adfc8 only when the new value is 0) [M]; `Drone_InitComms` (0x139520 :198/201,
ch 0x97/0x98) writes **no** timestamp (proven by the snapshot: their time entries still hold the
Init seed [L]); `DroneFunc_SetDeathChannel` does stamp (0x13e754) [M, audit].

## Writers (every decomp hit; semantics: S1 = set 1, S0 = clear, TGL = toggle, CPY = copy, VAL = value)

Core system [M]:

| function@addr | channel | semantics |
|---|---|---|
| Trigger_Calc 0x1b1900 | Bond moments 0x65-0x6E read → PlrStat_DoneBondMoment; latches (0x07000005: ch 0xA4 → ch 0x6F=1 @0x1b1980; 0x0700000B: ch 0x34 → ch 0x2D=1 @0x1b19a8); Touch-output clear pass @0x1b19f4 | S1 / S0 |
| Trigger_Update 0x1b1a78 | ext+0x20: Touch OR-in (case 1/2), MuxIn AND (case 3, empty list → 1), MuxOut copy to ≤8 outputs (case 5), MuxOrIn OR (case 0xC); case 0xD clears ch[ext+0x26] | S0/S1/OR/CPY |
| Trigger_Activate 0x1b2318 | ch[ext+0x20]=1 (types 1/2, mask&4); LoadLevel: success ch[ext+0x2C]=1 or fallback ch[ext+0x2A]=1 (both then load); reads 0x60-0x63 as cancel | S1 |
| Switch_Create 0x1adcc0 | ch[ext+0x12] = ext+0x16 initial byte | VAL |
| Switch_Update 0x1adea8 | ch[ext+0x12] TGL after the ext+0x10 timer; timestamp on falling edge | TGL |
| Switch_Activate 0x1b02a8 | ch[ext+0x12] = !ch[ext+0x12] | TGL |
| InverterTrigger_Update 0x1b30a8 | clears ch[obj+0xF6] (obj+0xF4 = watched channel) | S0 |

Doors [M]: `Door_SetState` (0x135618 :26) broadcasts `ch[ext+0x66] = ((OLD state & 0xC) != 0)` +
timestamp; `Door_Create` (0x136100 :97) forces `ch[ext+0x66] = 1` + timestamp for a created-open
door; `Doors_Calc`/`Door_Update`/`Door_Activate` write no channels [M, audit C8].

Player [M]: `Player_Init` 0x1910f8 clears ch 0x62, 0x5E; `Player_CheckForDeath` 0x190810 sets
0x5E; `Player_HandleDeath` 0x190bb0 sets 0x62; `Player_CollisionHandler` 0x1949a8 sets 0x7C;
`Player_Creep` 0x1969d0, `Player_Wire` 0x197658, `Player_Zipline` 0x197f50 set/clear their
placement param channels.

Drones / mission [M]: `Drone_InitComms` 0x139520 sets 0x97 and 0x98; `Drone_PostLoad_Init`
0x136938 clears 0x96; `DroneFunc_SetDeathChannel` 0x13e660 sets ch[drone+0x13C] + timestamp;
`DroneFunc_SetMissionFailReason` 0x13e560 sets 0x60, 99 (0x27031c), and a param channel;
`DroneSpawner_Control` 0x137e68 sets ch[ext+0x12] (done channel); `DroneFunc_HostageSaved`
0x1441c0 sets a param channel; `Mission_MonitorObjectives` 0x1827d8 sets 99/100; `Mission_Update`
0x182c60 reads 0x62 (dead) / 99 (failed) / 100 (complete); `MP_CheckForEndCondition` 0x184268
writes 0xFE/0xFD. Cancel-channel writers: 0x60 = SetMissionFailReason, 0x61 = Drone_InitComms,
0x62 = Player_HandleDeath, 0x63 = Mission_MonitorObjectives / P_CHEATMEDAL / SetMissionFailReason
[M].

Scripts [M]: `Script_EventHandler` (0x1d94c0) — case 0xD (:166): `ch[p0] = p1` + timestamp when
p1 ≠ 0; case 0x11 (:181-187): `if ch[p0] == 0 → ch[p1... per-param roles; the 07000005 EndLevel
event [0x98,0x98,0x1D,1] writes ch 0x1D via the else-branch [D, audit C3 reproduced]. Script
players: `SP_CreateScriptPlayer` (0x1db9a8 :72) clears the watch channel (level-data+0x44);
`SP_Update` (0x1dbda8 :226) `ch[target] = (ch[watch] != 0)`.

Other object updates [M]: Monitor_Update 0x1afcc8, PCQWorm_Update 0x1aee20, CamSubject_Update
0x1bc668, Emitter_Update 0x17b080, Copter_Update 0x17dbb8, Destroy_* 0x12bb60/0x12c4f8/0x12bd48,
Pickup_Handler 0x18e650, Searchlight_Update 0x1cb900, FuseBox_Update 0x1adfe8, SS_Activate
0x1a9ea8, Sensor_Update 0x1af618, Flicker_Update 0x1ab960 — each sets/clears its ext param
channel.

`MusicTrigger_Update` (0x1b31a0) writes no channel: watch ch = ext+4, gate ext+6, on set loads a
30-frame countdown then `Music_Event(ext+0, 2)` [M].

## Readers (how the channel id arrives) [M]

Trigger_Update via ext fields (out +0x20, inputs +0x26.., lock +0x24); doors via DOORINFO
ext+0x66 (open) / ext+0x68 (lock) in Door_Update / Door_Activate / Door_SetState / Door_IsLocked /
Door_DoMsg; drones: `NDrone2_DSTATE_WaitSwitch` (0x15b960 :22) reads ch[drone+0x134],
`NDrone2_PreDroneControl` (0x148db0 :51) ch[+0x135]; spawners via their ext params; script
players via SPINFO watch/gate/target (+0x30/+0x32/+0x2A); music via Trigger_Calc's Bond moments
0x65-0x6E and `DroneFunc_CheckAlarmRaised`'s MusicVars counters; mission via 0x62/99/100;
the static cel-swap loop via hold.

## 07000005 — every channel the level uses [D]

Parsed from `work/files/07000005.bin` (kind-1 map 01000099, placements block 0x1A @0x9FC154,
363 records; the 14 type-7 scripts parsed for channel events). Setter kinds: TO = TouchOnce out,
T = Touch out, MX = MuxIn AND, MO = MuxOrIn OR, SP = spawner, DR = drone, SCR = script event,
DOOR = door broadcast, LATCH = hard-coded.

| ch | setters (placement / script) | setter kind | readers (placement) | reader kind |
|---|---|---|---|---|
| 0x0A | #7, #267 (p12) | DR death | none found | — |
| 0x1D | script 06000A23 ev 0x11 t300 [0x98,0x98,0x1D,1]; #118 success (p4) | SCR / LL success | #118 (p2) | LL gate |
| 0x1E | #112 fallback (p3, unreachable: key gate p1=0) | LL fallback | none | — |
| 0x1F | #118 fallback (p3, unreachable) | LL fallback | none | — |
| 0x21 | #24 | TO | — | — |
| 0x22 | #25 MuxOrIn, #305 TO | MO / TO | #3, #4 (p3 wait), #65, #68 (p3), #268 spawner (p4 start) | drone wait; spawner start |
| 0x23 / 0x24 / 0x25 | #21 / #22 / #23 | TO | #25 inputs | mux |
| 0x42 | #269 | TO | #242 copter p6 (semantics unverified) | copter? |
| 0x43 | #301 (lock 0xB3) | TO | #268 (p7 kill), #298 (p6 gate), #302, #303 (p3 wait) | spawner kill; ScriptPlayer gate; drone wait |
| 0x44 | #306; #3, #4 (p12) | TO / DR | #304 input | mux |
| 0x45 | #36 "Mission Trigger" | TO | none found | — |
| 0x54 / 0x55 | #57 / #60 | T | #63 inputs | mux |
| 0x56 / 0x57 | #56 / #59 | T | #62 inputs | mux |
| 0x58 / 0x59 | #54 / #58 | T | #61 inputs | mux |
| 0x5C | script 06000A22 s8 TouchOnce t1882 (entity 0x02000480) | SCR-created trigger | #38 (p6) | ScriptPlayer gate |
| 0x61 | none (game alarm only) | — | #261 searchlight p1; CheckAlarmRaised | searchlight on; music |
| 0x62 | script 06000952 (SP #284) ev 0xD t93 [0x62,1] | SCR | Mission_Update | mission fail |
| 0x65 | #316 MuxIn (AND 0x70,0x71,0x72) | MX | #320 (p6, mode 0x40); #82 sound trigger; Bond-moment consumer | ScriptPlayer gate; sound |
| 0x69 | #80 MuxIn (AND 0x7F,0x80,0x81,0x85) | MX | #81, #83 sound triggers; Bond moments | sound; game |
| 0x6F | Trigger_Calc latch (0xA4 → 0x6F) | LATCH | #314 (p7) | TO lock |
| 0x70 | script 06000A22 s6 TouchOnce t5384 (hex 0x1508) | SCR-created trigger | #316 input | mux |
| 0x71 | script 06000A22 s7 Touch t0 (parent = truck) | SCR-created trigger | #316 input | mux |
| 0x72 | #318 Touch (mask 0x10 drone) | T | #316, #319 inputs | mux |
| 0x73 | script 06000A22 s9 ev 0xD t5412 [0x73,1] | SCR | #319 input | mux |
| 0x74 | #319 MuxIn (AND 0x72,0x73) | MX | #317 sound trigger | sound |
| 0x7D | #70 Touch — dead here (0x7D Touch runs only on 0x07001500) | T (dormant) | #71 (p6; script 06000074 not in this archive) | ScriptPlayer gate |
| 0x7F / 0x80 / 0x81 | #311 / #312 / #313 (lock 0xA4) | TO | #80 inputs | mux |
| 0x85 | #314 (lock 0x6F) | TO | #80 input | mux |
| 0x8B | #253 | TO | #254 sound trigger | sound |
| 0x98 | script 06000A23 ev 0x11 then-branch (self-latch; nothing sets it → else-branch always taken) | SCR | — | — |
| 0xA1 / 0xA2 / 0xA3 | #63 / #62 / #61 MuxIn | MX | #270 inputs | mux |
| 0xA4 | #315 TO (mask 0x18); #270 MuxOrIn; #65, #68 death (p12) | TO / MO / DR | #311-#313 (p7 lock), #66/#67/#271/#272 (p3 wait), #65/#68 (p6 second switch, p8=100), #27 searchlight p1; music alarm + latch → 0x6F | TO lock; drone wait; drone second switch; searchlight; music/game |
| 0xB2 | #307 TouchPlate_GrappleLedge1 | T | none found | — |
| 0xB3 | #304 MuxOrIn (OR 0x44) | MO | #301 (p7) | TO lock |
| 0xBE | #285-#297 PlungeTrigger ×13 | TO | #284 (p6) → plays 06000952 → ch 0x62 | ScriptPlayer gate |
| 0xC8 (200) | **none found on disc** (U-044 stays open) | — | #152, #154, #155, #156 (p3) | door lock |
| 0x66-0x68, 0x6A-0x6E | game only (Bond moments) | game | game | — |

Complete referenced set: 0x0A, 0x1D, 0x1E, 0x1F, 0x21-0x25, 0x42, 0x43, 0x44, 0x45, 0x54-0x59,
0x5C, 0x61, 0x62, 0x65, 0x69, 0x6F, 0x70-0x74, 0x7D, 0x7F, 0x80, 0x81, 0x85, 0x8B, 0x98,
0xA1-0xA4, 0xB2, 0xB3, 0xBE, 0xC8 (+ game-only 0x37, 0x66-0x68, 0x6A-0x6E). Read-but-never-
written on disc: 0xC8, 0x37, 0x61, 0x7D, 0x1F, 0x98. Written-but-never-read: 0x0A, 0x1E, 0x45,
0xB2. No 0xDC / 0xED / 0xEE / 0xF4 / 0x41 / 0x29 placements on the main map; no door has an open
channel (all p2 = 0); no hide channel used anywhere [D].

## Snapshot checks [L]

| snapshot | live (0x26fc90) | prev (0x26fd90) | hold (0x26fe90) | time (0x270190) |
|---|---|---|---|---|
| ee_start_05 (0x07000005) | only ch[0x45] = 1 | all 0 | all 0 | uniform 0x1a676; ch[0x45] = 0x1a678 (init seed + 2) |
| ee_start_07000001 | ch[0x97] = 1, ch[0x98] = 1 | 0x97, 0x98 | 0x97, 0x98 | uniform 0x2211e (the Init seed — InitComms stamps no time) |
| ee_start_0700000a | only ch[0x47] = 1 | 0x47 | 0x47 | uniform 0x221b5; ch[0x47] = 0x221b7 |

MusicVars u16[256] all zero in all three [L]. The uniform time values equal the Init seed →
`switch_channels_time` is pre-seeded from GameState+0x34, and the ee_start_05 prev/hold-zero
pattern confirms the snapshot was taken before that session's first prev/hold memcpy [M+L
consistent]. U-049's "07000001 ch 0x0E = 1 at level start" is **false** [L] — the set channels
are 0x97/0x98 (Drone_InitComms).

## Not verified here

- The `jal 0x134310` caller's function identity (0x1d5208 sits in non-decompiled code) [S: level
  load, from the surrounding code's behavior].
- Readers of the raw time cells 0x270310/0x27031c/0x270320/0x2703fc/0x2701f8 (mission writers'
  extra stamps) — no read-side decomp hits [S].
- SP "mode 0x40" field semantics (docs use it; not found in the code by this pass) [S].
- Full reader enumeration of the ~50 reader-only objects (name-swept, not itemized) [S].
