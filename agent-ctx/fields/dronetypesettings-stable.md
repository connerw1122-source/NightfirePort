# DroneTypeSettings (0x29aee8) and NDrone2_StateFuncs (0x29b300)

Proof levels: see [README.md](README.md). Static pass, 2026-10-12, worker 4-c. Sources: the decomp
`work/decomp/ACTION.ELF/` [M] (file + decompiled line), `work/action-symtab.tsv` [M-sym], and the EE snapshots
`work/ee_start_05.bin`, `ee_start_07000001.bin`, `ee_start_0700000a.bin`, `ee_snapshot.bin` (file offset == EE
address) [L]. Reconciles `docs/systems/actors.md` (§1/§2) — disagreements listed at the end, none silent.
Port: `crates/nf-game/src/drone.rs` (`pub mod state`), `crates/nf-game/src/kiko.rs`.

Both tables are static initialised data: byte-identical across all four snapshots
(md5 of 0x29aee8..+1020 = `d361371c3740`, of 0x29b300..+1000 = `2e0b1244e821`) [L ×4]. No writer to either
table exists in the decomp (greps for `DroneTypeSettings` / `NDrone2_StateFuncs` / `DAT_0029a*`-aliases:
readers only) — with one exception treated as a global below (the u32 at byte 0x304).

## How a drone gets its mode byte (Drone+0xC5)

`DroneTypeSettings` is indexed by the drone's mode byte **Drone+0xC5** (u8). All table reads are
`DroneTypeSettings[(u8)Drone[0xC5] * 0xC + field]`. Two init chains write +0xC5, selected by the placement's
behaviour id (`DefaultInit` 0x14b300:495-503: id < 0x24 → OLD, else NEW; id from placement word `piVar19[0x12]`):

- **OLD** (behaviour id 0..0x23): `NDrone2_DoModeSettingsOLD` 0x14cfa0:29 `+0xC5 = DroneModeSettings[beh*0xC]`
  (a third table, 0x29ad40, 420 = 35×12 bytes: `+0` dtype u8 → +0xC5, `+1` team u8 → +0x44, `+4` f32 alert →
  +0x50C/+0x500, `+8` init fn). Sets `+0x32 = 1`. Then `DefaultInit` 0x14b300:676 calls
  `NDrone2_DoTypeSettingsOLD` 0x14d2e8(drone, +0xC5) — the DroneTypeSettings lookup
  (only caller). Edge: behaviour 0x23 reads row 35 = the first 12 bytes of *DroneTypeSettings* (0xffff0000…)
  → +0xC5 = 0xFF → the subsequent row read at +0xC5·0xC = 0xBF8 is **out of bounds** [M 0x14cfa0:14-29];
  not known to be used on disc.
- **NEW** (behaviour id ≥ 0x24): `NDrone2_DoModeSettingsNEW` 0x14c600:77-79 copies the behaviour block's bytes
  `+0` → **+0xC7** (raw dtype, placement param 15), `+4` → **+0xC8** (mode, param 16), `+0x10` → **+0xC9**
  (friend mode). Sets `+0x32 = 0`. Then `NDrone2_GetDroneTypeAttackTypeFriend` 0x14cbd8:
  - :22 `+0xC4 = GetDTYPENEW(drone, +0xC7, 0)` (the attack type), :24-26 `+0xC5 = GetDTYPENEW(drone, +0xC7, +0xC8)`,
    :44 `+0xC6 = GetDTYPENEW(drone, +0xC7, +0xC9)` gated on +0x13A/+0x135/+0x500 (the friend type);
  - :48-61 the team switch on the **raw** dtype +0xC7 (10/0xB/0xD/0xE → 3, 0xC → 2, else 1 → +0x44);
  - :65-121 the DroneTypeSettings start/alt/init-fn lookup (same shape as DoTypeSettingsOLD, below);
  - :122-155 alert (+0x50C ≥ 1.0) overrides: skin kind +0xD8 == 0xB → `ChangeToAttackMode`; else switch on
    (start−4): {4,5,6,0x30,0x31,0x36,0x56} → 0x56; {0xC,0xE,0x2A,0x2E,0xB1} → 0x12; {0x25,0x26} → 0x2A;
    {0xA2,0xA3} → 0xA7; default keep.

`NDrone2_GetDTYPENEW` 0x14c938 (the remap; callers: 0x14cbd8:22,24,44 only): prop 0x10 on the behaviour &
mode-arg 0 → **0x10** (the ROW: AmbushInit start 0x37); dtype 0..9 on 07000007 mode 0 → 0x13/0x16 (prop 0x4B);
dtype 10/0xB mode 0 → +0x50C < 1 ? (+0x15 set ? 0x12 : 9) : 10 (07000007 also sets props 0x1F=0, 0x54=1);
dtype 0xC mode 0 → skin 0xC ? 0x11 : 0xC; dtype 0xD mode 0 → skin 0xF ? 0x17 : (+0x50C < 1 ? 9 : 10);
dtype 0xE mode 0 → prop 0x4E ? 5 : (+0x50C < 1 ? 9 : 10); dtype 0xF mode 0 → 0xD; dtype 0x10 → 0x1E
(unconditional). Then the mode arg (when the dtype cases fall through): 0 → 0; 2 → 2 (or **0x19** if
+0x50C ≥ 1); 3 → 7 (or 0xE if prop 0x56); 4 → 4; 5 → 0x15; anything else → 0.

Further +0xC5 writers (the full census, grep `( iVar* + 0xc5) =`):
`DefaultInit` 0x14b300:189 (= 0x17, skin template 0x5000035, the 06 truck driver), :624 (= 0x1C/0x1B for
placement msgs 0x6000892/0x600089C), :723 (= 0x1D + start 0xBD for skin kind 0x18, the astronaut);
`DoModeSettingsNEW` 0x14c600:135 (= 0x17 on msg 0x600011D), :155 (= 0x18 + +0x13D on 0x600011F/0x6000124/0x6000126);
`DroneFunc_HandleImpact` 0x1469b8:122,239,304,350 (impact cases 7/0x17/0x18/0x19, levels 0x7000008/9,
flag 0x4000000: = 9 with +0xC6 = 7 and +0x5A2 = 0x14 — the office/hotel staff become the civilian chain);
the friend-switch trio (below) `+0xC5 = +0xC6` then `= 0x19` when it was 2;
`AbseilSlide` 0x169730:36 / `AbseilStepOff` 0x1738e0:21 (`= +0xC4`, the stored attack type);
`DoTypeSettingsOLD` 0x14d2e8:15 (= its arg; the only caller passes the current +0xC5 back).
The **friend-switch trio** (`PreDroneControl` 0x148db0:63-84, `DroneFunc_FirstAttack` 0x148010:107-129,
`ChangeToAttackMode` 0x14a648:11-34): when the alt state +0x5A4 fires, `+0xC5 = +0xC6`, and if the new mode
is 2 → `+0xC5 = 0x19` and the byte-0x304 hook is called (see row 64 below).

The state machine itself: `Drone_SM_InitObject` 0x175838:18 installs `NDrone2_ProcessStateMachine` 0x175778
as the drone's `+0x130` handler; it calls `NDrone2_StateFuncs[state](dcvars, …)` for `state < 0xfa` [M :10-12].

## DroneTypeSettings row layout (12 bytes; row i at 0x29aee8 + i·12)

| Offset | Type | Meaning | Proof (every reader) |
|---|---|---|---|
| +0 | s16 | **start state** → Drone+0x5A2. `−1` (0xFFFF) = "by mode": DoTypeSettingsOLD 0x14d2e8:16-48 / GetDroneTypeAttackTypeFriend 0x14cbd8:65-101 compute it — mode 4 → 8; mode 2 → 0x2A if +0x50C ≥ 1 else 0x29; mode 7 → 0x85; 0x14cbd8 also 0x10 → 0x37, 0x13 → 0x30, 0x16 → 0x31, 0x18 → 0x36 (dead: those rows are ≥ 0, see row table); otherwise behaviour prop 0x24 or 0x25 present → 6 (InitPatrol), neither → 4 (Idle). The −1 rows actually live here: 0, 3, 11, 20, 26 | [M 0x14d2e8:16,51] [M 0x14cbd8:65,104] [L rows] |
| +2 | s16 | **alt state** → Drone+0x5A4, read as `DroneTypeSettings[+0xC6].alt`, falling back to `DroneTypeSettings[+0xC5].alt` when 0 (+0xC6 = 0 → +0x5A4 = 0). +0x5A4 is the state armed for the alert/friend switch: DoTypeSettingsOLD 0x14d2e8:79-81 copies it straight to +0x5A2; the friend-switch trio consumes it (state in, type swap) | [M 0x14d2e8:53-62] [M 0x14cbd8:107-117] [L rows] |
| +4 | fn* | **init fn**, called once per application as `fn(drone)` (DoTypeSettingsOLD passes the Drone*, GetDroneTypeAttackTypeFriend the same) when non-null. Named slots: 0x14e570 `initDTYPE_Sniper` (sets +0x4F8 flag 0x10), 0x14e580 `initDTYPE_SniperAlert` (same + builds a DCVars from the obj), 0x1291e8 `initDTYPE_BotInit` (empty) | [M 0x14d2e8:64-67] [M 0x14cbd8:118-121] [M 0x14e570] [M 0x14e580] [M 0x1291e8] |
| +8 | fn* | **control fn**, called EVERY control tick from `Drone_Control` 0x139268:59-64 as `fn(&DCVars)` replacing `NDrone2_ControlSTANDARD` when non-null. Named slots: 0x14b0f8 `ControlDTYPE_Ninja` (CreateNinjaEyes + STANDARD), 0x14b058 `ControlDTYPE_Zoe` (ch 1 → ch 0x9B + timestamp; ch 0x9B && Bond > 10.0 → SetMissionFailReason(0xF); STANDARD), 0x14b128 `ControlDTYPE_Astronaut` (STANDARD only) | [M 0x139268:59-64] [M 0x14b0f8] [M 0x14b058] [M 0x14b128] |

**The byte-0x304 slot (= row 64's +4) is read as a GLOBAL hook** with no row-64 indexing: `DroneTypeSettings._304_4_`
in `PreDroneControl` 0x148db0:68-69 (`fn(drone, 0x56)`), `DroneFunc_FirstAttack` 0x148010:113-115 (`fn(drone)`),
`ChangeToAttackMode` 0x14a648:18-20 (`fn(drone)`) — all three at the "friend-switch lands on sniper mode 2 → 0x19"
point. 0x00000000 in every snapshot [L ×4]; no writer in the decomp. Treat as a patch slot; default no-op.

## The 85 rows (0x29aee8, 85 × 12 = 1020 bytes; symtab size 1020 [M-sym])

States named via NDrone2_StateFuncs (§ below). "by mode" = start −1. Reachability cites the +0xC5 writers.

| Row | Bytes (12) | Start (+0) | Alt (+2) | Init fn (+4) | Control fn (+8) | Notes |
|---|---|---|---|---|---|---|
| 0 | ffff 0000 00000000 00000000 | −1 | 0 | — | — | the generic guard; by mode → 6/4 (props 0x24/0x25) [M 0x14d2e8:27-38]. GetDTYPENEW's default return [M 0x14c938:100-103] |
| 1 | 0600 0000 … | 6 InitPatrol | 0 | — | — | OLD dtype 1 rows [M 0x14cfa0:29] |
| 2 | 2900 2a00 70e51400 00000000 | 0x29 SniperIdle | 0x2a SniperAim | 0x14e570 initDTYPE_Sniper | — | the sniper; mode 2 [M 0x14c938 via mode-arg 2] |
| 3 | ffff 0000 … | −1 | 0 | — | — | **by mode** (docs says 0x56 — wrong, see Disagreements). OLD dtype 3 rows |
| 4 | 0800 0000 … | 8 HostageKiller | 0 | — | — | mode 4 [M 0x14c938:118-120] |
| 5 | 0a00 0a00 … | 0xa Hostage | 0xa Hostage | — | — | GetDTYPENEW 0xE+prop 0x4E → 5 [M 0x14c938:80-85]; OLD dtype 5 rows |
| 6 | 5600 0000 … | 0x56 Attack | 0 | — | — | the aggressor |
| 7 | 8500 8500 … | 0x85 EnemyRunToPoint | 0x85 | — | — | mode 3 [M 0x14c938:111-117] |
| 8 | b700 b700 … | 0xb7 Tester1 | 0xb7 | — | — | OLD dtype 8 rows (EvilBase guard) |
| 9 | 1000 1200 … | 0x10 CivilianInit | 0x12 CivilianScared | — | — | GetDTYPENEW civilian chain [M 0x14c938:45-50] |
| 10 | 1200 1200 … | 0x12 CivilianScared | 0x12 | — | — | alert branch of the same |
| 11 | ffff b500 … | −1 | 0xb5 FailMission | — | — | the mission actor |
| 12 | 1c00 1c00 … | 0x1c AllyLeadInit | 0x1c | — | — | the VIP/escort; msg 0x6000892 → +0xC5 = 0x1C [M 0x14b300:615-624] |
| 13 | a600 a700 00000000 f8b01400 | 0xa6 NinjaStand | 0xa7 NinjaAttack | — | 0x14b0f8 ControlDTYPE_Ninja | dtype 0xF → 0xD [M 0x14c938:92-96] |
| 14 | 8900 8900 … | 0x89 RunToAlarm | 0x89 | — | — | the alarm runners |
| 15 | 0000 0000 … | 0 = Global | 0 | — | — | degenerate (start = state 0 re-enters Global) |
| 16 | 3700 5600 … | 0x37 AmbushInit | 0x56 Attack | — | — | prop 0x10 → row 0x10 [M 0x14c938:10-14]; −1-branch case 0x10 → 0x37 agrees [M 0x14cbd8:89-91] |
| 17 | 0400 0000 00000000 58b01400 | 4 Idle | 0 | — | 0x14b058 ControlDTYPE_Zoe | Zoe's control fn; no disc instance (docs §4) |
| 18 | 2e00 0000 … | 0x2e PartyGirlInit | 0 | — | — | not in docs §1 |
| 19 | 3000 6600 … | 0x30 CivilianGuard | 0x66 DrawWeapon | — | — | the undercover guard |
| 20 | ffff 0000 … | −1 | 0 | — | — | by mode |
| 21 | b400 b400 … | 0xb4 DeleteMe | 0xb4 | — | — | not in docs §1 |
| 22 | 3100 0000 … | 0x31 CivilianDoorGuard | 0 | — | — | the door guard |
| 23 | 3200 1200 … | 0x32 TruckDriverInit | 0x12 CivilianScared | — | — | mode 0x17 = the truck driver: DefaultInit skin 0x5000035 / GetDTYPENEW dtype 0xD skin 0xF / msg 0x600011D [M 0x14b300:186-190, 0x14c938:62-66, 0x14c600:133-137]; Global/Disabled/WaitSwitch bypass mode 0x17 → state 0x32 regardless [M 0x15b830:33-36, 0x172440:38-41, 0x15b960:27-31] |
| 24 | 3600 0000 … | 0x36 CastleChatGuard1 | 0 | — | — | −1-branch case 0x18 → 0x36 [M 0x14cbd8:98-100] |
| 25 | 2a00 0000 80e51400 00000000 | 0x2a SniperAim | 0 | 0x14e580 initDTYPE_SniperAlert | — | the alert sniper: GetDTYPENEW mode 2 + alert → 0x19 [M 0x14c938:105-110] |
| 26 | ffff 0000 … | −1 | 0 | — | — | by mode |
| 27 | 8e00 0000 … | 0x8e AbseilInit | 0 | — | — | not in docs §1 |
| 28 | 8e00 0000 … | 0x8e AbseilInit | 0 | — | — | not in docs §1 |
| 29 | bd00 0000 00000000 28b11400 | 0xbd AstronautLaunch | 0 | — | 0x14b128 ControlDTYPE_Astronaut | DefaultInit skin kind 0x18 forces +0xC5 = 0x1D + start 0xBD [M 0x14b300:722-727] |
| 30 | c300 0000 e8911200 00000000 | 0xc3 BotInit | 0 | 0x1291e8 initDTYPE_BotInit (empty) | — | **the MP bot mode**; Global ENTER re-checks mode 0x1E → 0xC3 [M 0x15b830:17-20] |
| 31 | c400 … | 0xc4 BotRespawn | | | | rows 31..84 = identity onto the Bot block: mode i → state 165+i (0xC4 + i−31) [L]. No SP writer reaches +0xC5 > 0x1E (except the 0xFF edge above) |
| 32 | c500 … | 0xc5 BotGlobal | | | | |
| 33 | c600 … | 0xc6 BotCollector | | | | |
| 34 | c700 … | 0xc7 BotGuardian | | | | |
| 35 | c800 … | 0xc8 BotTeamPlayer | | | | |
| 36 | c900 … | 0xc9 BotBully | | | | |
| 37 | ca00 … | 0xca BotBerserker | | | | |
| 38 | cb00 … | 0xcb BotGreedy | | | | |
| 39 | cc00 … | 0xcc BotVengeful | | | | |
| 40 | cd00 … | 0xcd BotJudge | | | | |
| 41 | ce00 … | 0xce BotAssassin | | | | |
| 42 | cf00 … | 0xcf BotAttack | | | | |
| 43 | d000 … | 0xd0 BotAttackRun | | | | |
| 44 | d100 … | 0xd1 BotAttackNoRoute | | | | |
| 45 | d200 … | 0xd2 BotAttackFire | | | | |
| 46 | d300 … | 0xd3 BotAttackNoOpponent | | | | |
| 47 | d400 … | 0xd4 BotAttackStrafeAimLeft | | | | |
| 48 | d500 … | 0xd5 BotAttackStrafeAimRight | | | | |
| 49 | d600 … | 0xd6 BotAttackRunChangePosition | | | | |
| 50 | d700 … | 0xd7 BotAttackBackoff | | | | |
| 51 | d800 … | 0xd8 BotAttackCrouch | | | | |
| 52 | d900 … | 0xd9 BotAttackRollLeftCrouch | | | | |
| 53 | da00 … | 0xda BotAttackRollRightCrouch | | | | |
| 54 | db00 … | 0xdb BotAttackStepAimLeft | | | | |
| 55 | dc00 … | 0xdc BotAttackStepAimRight | | | | |
| 56 | dd00 … | 0xdd BotAttackReload | | | | |
| 57 | de00 … | 0xde BotAttackChangeWeapon | | | | |
| 58 | df00 … | 0xdf BotAttackUnarmed | | | | |
| 59 | e000 … | 0xe0 BotCoverRunTo | | | | |
| 60 | e100 … | 0xe1 BotCoverInit | | | | |
| 61 | e200 … | 0xe2 BotCoverIdle | | | | |
| 62 | e300 … | 0xe3 BotCoverAim | | | | |
| 63 | e400 … | 0xe4 BotCoverFire | | | | |
| 64 | e500 … **fn4 slot = 0x29b1ec, the global hook** | 0xe5 BotCoverReturn | | | | the +4 u32 here is read UNINDEXED as the sniper-friend-switch hook [M 0x148db0:68, 0x148010:113, 0x14a648:18]; 0 in all snapshots [L ×4] |
| 65 | e600 … | 0xe6 BotCoverTypeChange | | | | |
| 66 | e700 … | 0xe7 BotCoverLeave | | | | |
| 67 | e800 … | 0xe8 BotCoverLeaveNow | | | | |
| 68 | e900 … | 0xe9 BotStuck | | | | |
| 69 | ea00 … | 0xea BotAlertToPosition | | | | |
| 70 | eb00 … | 0xeb BotGotoGoalPosition | | | | |
| 71 | ec00 … | 0xec BotSeenOpponent | | | | |
| 72 | ed00 … | 0xed BotSeenDroneShot | | | | |
| 73 | ee00 … | 0xee BotHeardNoise | | | | |
| 74 | ef00 … | 0xef BotImpactBullet | | | | |
| 75 | f000 … | 0xf0 BotImpactExplosive | | | | |
| 76 | f100 … | 0xf1 BotImpactPunch | | | | |
| 77 | f200 … | 0xf2 BotDeathAnim | | | | |
| 78 | f300 … | 0xf3 BotDeathByExplosion | | | | |
| 79 | f400 … | 0xf4 BotDead | | | | |
| 80 | f500 … | 0xf5 BotImpactStunGrenade | | | | |
| 81 | f600 … | 0xf6 BotDoorOpen | | | | |
| 82 | f700 … | 0xf7 BotGuardFriendIdle | | | | |
| 83 | f800 … | 0xf8 BotGuardFriendFollow | | | | |
| 84 | f900 … | 0xf9 BotIdle | | | | |

Sanity checks vs the task brief and docs §1: row 0/2/12/13/25/30 bytes match the brief's samples exactly [L];
docs §1 rows 0-2, 4-10, 12-14, 16-17, 19, 22-25, 29 match the bytes; row 3 does not (docs error); rows 18, 21,
27, 28 and the whole 30..84 block are missing from docs §1 (now above).

## NDrone2_StateFuncs (0x29b300)

Length: symtab size **1000 = 250 × 4** [M-sym]; next symbol 0x29b6e8 (16 bytes, the `DAT_0029b6f0/2f4` msg-routing
list heads used by `Drone_SM_RouteMsg` 0x172128:69-89) — contiguous, so no room past 250. The dispatcher guards
`state < 0xfa` [M 0x175778:10]. All 250 entries non-zero [L]; entries 0xC8..0xF9 (and the other small 0x1725xx-0x175x
stubs) are the shared no-op-ish state bodies. The only reader is `NDrone2_ProcessStateMachine` [M grep].
Meanings: `[M body]` = I read the decompile (line cites above the table); `[name]` = meaning from the decompiled
function's own name (symbol [M-sym]) without a body read; docs §2 cross-checked where listed.

| Id | Fn | Decompile name | Meaning | Port constant |
|---|---|---|---|---|
| 00 | 15b830 | DSTATE_Global | ENTER: mode 0x1E → BotInit 0xC3; wait-switch unset ch → WaitSwitch 1; mode 0x17 → 0x32; else → +0x5A2 [M body 0x15b830:15-47] | `state::GLOBAL` ✓ |
| 01 | 15b960 | DSTATE_WaitSwitch | ENTER Disable(0); UPDATE: ch [+0x134] set → Enable(1) → +0x5A2 (mode 0x17 → 0x32); msg 0xE same [M body 0x15b960:14-50] | `state::WAIT_SWITCH` ✓ |
| 02 | 172440 | DSTATE_Disabled | swallows all; UPDATE leaves only when +0x4F8 flag 0x100 → mode 0x17 ? 0x32 : (+0xCD8 or stay) [M body 0x172440:14-52] | — |
| 03 | 15ba68 | DSTATE_PlayScript | [name] runs the drone's script stream | — |
| 04 | 15bf18 | DSTATE_Idle | the standing idle; the msg-4 timeout → Attack [name + docs] | `state::IDLE` ✓ |
| 05 | 15c248 | DSTATE_Alert | [name] the alert look-around | `state::ALERT` ✓ |
| 06 | 1724f0 | DSTATE_InitPatrol | [name] assign path → Patrol | `state::INIT_PATROL` ✓ |
| 07 | 15c498 | DSTATE_Patrol | [name] walk the patrol | `state::PATROL` ✓ |
| 08 | 15c728 | DSTATE_HostageKiller | ENTER SetupHostageKiller (+flag 0x10, anim 9; none → Idle 4); ANIM_DONE → msg 0xB to the hostage + Attack 0x56 [M body 0x15c728:18-45] | — |
| 09 | 15c940 | DSTATE_HostageKillerAttack | [name] | — |
| 0A | 15cb78 | DSTATE_Hostage | [name] | — |
| 0B | 1725a0 | DSTATE_HostageDie | [name] | — |
| 0C | 15ccf0 | DSTATE_HostageSaved | [name] (alert map start → 0x12) | — |
| 0D | 15ce60 | DSTATE_HostageIdle | [name] | `state::HOSTAGE_IDLE` ✓ |
| 0E | 15cfe0 | DSTATE_HostageHide | [name] | — |
| 0F | 172628 | DSTATE_HostageDead | [name] | — |
| 10 | 172730 | DSTATE_CivilianInit | docs §2 facts (voice 0xD → PartyGirl 0x2F; prop 0x1D → mission; else patrol/no path → Civilian) [name + docs] | `state::CIVILIAN_INIT` ✓ |
| 11 | 15d118 | DSTATE_Civilian | scared radius 10/20 by level [M docs] | `state::CIVILIAN` ✓ |
| 12 | 15d3a0 | DSTATE_CivilianScared | FirstAttack + the scared chain [M docs] | `state::CIVILIAN_SCARED` ✓ |
| 13 | 15d520 | DSTATE_CivilianHiding | [name] | `state::CIVILIAN_HIDING` ✓ |
| 14 | 15d610 | DSTATE_CivilianPatrol | [name] | `state::CIVILIAN_PATROL` ✓ |
| 15 | 172810 | DSTATE_ReturnToPatrolPath | [name] (GetDTYPENEW mode 5 → row 0x15) | — |
| 16 | 15d860 | DSTATE_CivilianMission | skin 0x13 → KikoMission 0x19 [M kiko.rs:23] | `state::CIVILIAN_MISSION` ✓ |
| 17 | 15dbb0 | DSTATE_CivilianMissionWait | [name] | — |
| 18 | 172870 | DSTATE_StandBlind | the blind-cheat stand [M kiko.rs:8] | `kiko::STAND_BLIND` ✓ |
| 19 | 15df28 | DSTATE_KikoMission | [M kiko.rs:11-16] | `kiko::KIKO_MISSION` ✓ |
| 1A | 15e0d8 | DSTATE_KikoMissionRun | [M kiko.rs:16] | `kiko::KIKO_MISSION_RUN` ✓ |
| 1B | 15e1e8 | DSTATE_EnemyMission | [name]; msg 0x600089C → +0xC5 = 0x1B [M 0x14b300:620-624] | — |
| 1C | 15e4f0 | DSTATE_AllyLeadInit | the escort lead entry [M docs §2] | `state::ALLY_LEAD_INIT` ✓ |
| 1D | 15e698 | DSTATE_AllyLead | [M docs §2] | `state::ALLY_LEAD` ✓ |
| 1E | 15ea28 | DSTATE_AllyLeadPlayerInWay | [M docs §2] | `state::PLAYER_IN_WAY` ✓ |
| 1F | 15ec08 | DSTATE_AllyLeadHide | [M docs §2] | `state::LEAD_HIDE` ✓ |
| 20 | 15eda8 | DSTATE_AllyLeadWait | [M docs §2] | `state::ALLY_LEAD_WAIT` ✓ |
| 21 | 15efc8 | DSTATE_AllyLeadMissionWait | [M docs §2] | `state::LEAD_MISSION_WAIT` ✓ |
| 22 | 15f120 | DSTATE_AllyLeadBondCombat | [M docs §2] | `state::LEAD_BOND_COMBAT` ✓ |
| 23 | 15f2e8 | DSTATE_AllyLeadDone | [M docs §2] | `state::ALLY_LEAD_DONE` ✓ |
| 24 | 15f3c8 | DSTATE_AllyFollowInit | [M docs §2] | `state::FOLLOW_INIT` ✓ |
| 25 | 15f4e0 | DSTATE_AllyFollow | [M docs §2] | `state::FOLLOW` ✓ |
| 26 | 15f6a0 | DSTATE_AllyFollowWait | [M docs §2] | `state::FOLLOW_WAIT` ✓ |
| 27 | 15f858 | DSTATE_AllyFollowDone | [M docs §2] | `state::FOLLOW_DONE` ✓ |
| 28 | 15f938 | DSTATE_AllyGoToGoalPosition | [M docs §2] | `state::GO_TO_GOAL_ALLY` ✓ |
| 29 | 15fab0 | DSTATE_SniperIdle | ENTER flags 0x10000/0x2000/0x10 + anim 0xE; impacts → HandleImpact(resume 0x2A); alerts → EnemyAlerts/LookForOpponent [M body 0x15fab0:14-71] | `state::SNIPER_IDLE` ✓ |
| 2A | 15fc18 | DSTATE_SniperAim | ENTER anim 10, alert 1, timers 1000.0; UPDATE aim → 0x2B when on target [M body 0x15fc18:13-40] | `state::SNIPER_AIM` ✓ |
| 2B | 15fd88 | DSTATE_SniperFire | [name] | `state::SNIPER_FIRE` ✓ |
| 2C | 15ff80 | DSTATE_SniperReload | [name] | `state::SNIPER_RELOAD` ✓ |
| 2D | 173480 | DSTATE_GrenadeThrow | throw anim 0x49 → ThrowGrenade → 0x56 [M drone.rs:84-86] | `state::GRENADE_THROW` ✓ |
| 2E | 1728c0 | DSTATE_PartyGirlInit | [name + docs §2] | `state::PARTY_GIRL_INIT` ✓ |
| 2F | 160140 | DSTATE_PartyGirl | [name + docs §2] | `state::PARTY_GIRL` ✓ |
| 30 | 1603a8 | DSTATE_CivilianGuard | [name + docs §2] | `state::CIVILIAN_GUARD` ✓ |
| 31 | 160648 | DSTATE_CivilianDoorGuard | [name + docs §2] | `state::CIVILIAN_DOOR_GUARD` ✓ |
| 32 | 160bd0 | DSTATE_TruckDriverInit | [name]; the mode-0x17 bypass target [M 0x15b830:33-36] | `state::TRUCK_DRIVER_INIT` ✓ |
| 33 | 160d98 | DSTATE_TruckDriverInitAlert | [name] | `state::TRUCK_DRIVER_INIT_ALERT` ✓ |
| 34 | 160ed0 | DSTATE_TruckDriverIdle | [name] | `state::TRUCK_DRIVER_IDLE` ✓ |
| 35 | 161030 | DSTATE_TruckDriverMission | [name] | `state::TRUCK_DRIVER_MISSION` ✓ |
| 36 | 1612e0 | DSTATE_CastleChatGuard1 | chat pair; EXIT raises ch 0x1A when a script is attached [M body 0x1612e0:20-43] | — |
| 37 | 172960 | DSTATE_AmbushInit | ENTER FindAmbushCover → UnderCoverInit 0x93, else anim 0x11 → 0x38 [M body 0x172960:11-23] | `state::AMBUSH_INIT` ✓ |
| 38 | 161560 | DSTATE_AmbushWait | [name + drone.rs:80-83] | `state::AMBUSH_WAIT` ✓ |
| 39 | 1616c0 | DSTATE_InterogateAssist | [name] | — |
| 3A | 161860 | DSTATE_InterogateAssistWait | [name] | — |
| 3B | 1729f0 | DSTATE_Interogator | [name] | — |
| 3C | 161a40 | DSTATE_Interogate | [name] | — |
| 3D | 162338 | DSTATE_InterogateWalk | [name] | — |
| 3E | 162558 | DSTATE_CivilianChallenge | [name + docs §2] | — |
| 3F | 1626a0 | DSTATE_Surrender_Anim | [name + drone-surrender.md] | `state::SURRENDER_ANIM` ✓ |
| 40 | 162950 | DSTATE_Surrendered | [name] | `state::SURRENDERED` ✓ |
| 41 | 162a70 | DSTATE_Unsurrender_Anim | [name] | `state::UNSURRENDER_ANIM` ✓ |
| 42 | 172a30 | DSTATE_KnockedOut_Anim | [M drone.rs:49-53] | `state::KNOCKED_OUT_ANIM` ✓ |
| 43 | 172b48 | DSTATE_Knocked_Out | [M drone.rs:54-56] | `state::KNOCKED_OUT` ✓ |
| 44 | 1637f8 | DSTATE_Death_Anim | raises the death channel on first UPDATE [M docs §7] | `state::DEATH_ANIM` ✓ |
| 45 | 163a80 | DSTATE_DeathByExplosion | [name + docs §7] | — |
| 46 | 163c68 | DSTATE_SpecialDeath_Anim | [name] | `state::SPECIAL_DEATH` ✓ |
| 47 | 163fe8 | DSTATE_Dead | [name] | `state::DEAD` ✓ |
| 48 | 172cd8 | DSTATE_Fade | [name] | `state::FADE` ✓ |
| 49 | 172dd8 | DSTATE_FadeFast | [name] | `state::FADE_FAST` ✓ |
| 4A | 162b98 | DSTATE_Taser | [name] | — |
| 4B | 162eb0 | DSTATE_Stunned | [name] | — |
| 4C | 1630b8 | DSTATE_Stunned_Recover | [name] | — |
| 4D | 172bb0 | DSTATE_StunGrenadeImpact | [name] | — |
| 4E | 1631c0 | DSTATE_StunGrenadeLoop | [name] | — |
| 4F | 163350 | DSTATE_StunGrenadeRecover | [name] | — |
| 50 | 163430 | DSTATE_StunDartImpact | [name] | — |
| 51 | 163570 | DSTATE_StunDartLoop | [name] | — |
| 52 | 163700 | DSTATE_StunDartRecover | [name] | — |
| 53 | 172ee0 | DSTATE_PunchImpact | [M drone.rs:57-59] | `state::PUNCH_IMPACT` ✓ |
| 54 | 172fb0 | DSTATE_ExplosiveImpact | [name] | — |
| 55 | 173098 | DSTATE_BulletImpact | [name + drone-damage.md] | `state::BULLET_IMPACT` ✓ |
| 56 | 164210 | DSTATE_Attack | the combat dispatcher [name + docs] | `state::ATTACK` ✓ |
| 57 | 173178 | DSTATE_Alerted1stEncounter | [name] | — |
| 58 | 1731b8 | DSTATE_Combat | [name] | `state::COMBAT` ✓ |
| 59 | 1643e8 | DSTATE_CombatNoMove | [name] | `state::COMBAT_NO_MOVE` ✓ |
| 5A | 164690 | DSTATE_CombatOutOfRange | [name] | `state::COMBAT_OUT_OF_RANGE` ✓ |
| 5B | 173280 | DSTATE_CombatNewSighting | [name] | `state::COMBAT_NEW_SIGHTING` ✓ |
| 5C | 164838 | DSTATE_CombatNoSight | [name] | `state::COMBAT_NO_SIGHT` ✓ |
| 5D | 173338 | DSTATE_CombatTooClose | [name] | — |
| 5E | 164960 | DSTATE_CombatWait | [name] — **port calls it COMBAT_DRONE_AHEAD** (see Disagreements) | `state::COMBAT_DRONE_AHEAD` (name only) |
| 5F | 164c08 | DSTATE_CombatNoRoute | [name] | `state::COMBAT_NO_ROUTE` ✓ |
| 60 | 1650a0 | DSTATE_NoOpponent | [name] | `state::NO_OPPONENT` ✓ |
| 61 | 173390 | DSTATE_Obstructed | [name] | — |
| 62 | 165270 | DSTATE_AlertToPosition | [name] | `state::ALERT_TO_POSITION` ✓ |
| 63 | 165640 | DSTATE_GoToGoalPosition | [name] | `state::GO_TO_GOAL` ✓ |
| 64 | 165a68 | DSTATE_HostageGoToGoalPosition | [name + docs §2] | — |
| 65 | 165bf0 | DSTATE_SearchArea | [name] | `state::SEARCH_AREA` ✓ |
| 66 | 166000 | DSTATE_DrawWeapon | [name] | `state::DRAW_WEAPON` ✓ |
| 67 | 166178 | DSTATE_AimStand | [name] | `state::AIM_STAND` ✓ |
| 68 | 166328 | DSTATE_AimStandFire | [name] | `state::AIM_STAND_FIRE` ✓ |
| 69 | 1666d8 | DSTATE_AimStandReload | [name] | `state::AIM_STAND_RELOAD` ✓ |
| 6A | 166840 | DSTATE_AimStandDiscard | [name] | — |
| 6B | 166940 | DSTATE_Prone | [name] | — |
| 6C | 166a28 | DSTATE_ProneFire | [name] | — |
| 6D | 166b98 | DSTATE_AimBackoff | [name] | `state::AIM_BACKOFF` ✓ |
| 6E | 166d90 | DSTATE_CrouchCover | [name] | — |
| 6F | 166f28 | DSTATE_AimCrouch | [name] | `state::AIM_CROUCH` ✓ |
| 70 | 167060 | DSTATE_AimCrouchFire | [name] | `state::AIM_CROUCH_FIRE` ✓ |
| 71 | 167370 | DSTATE_AimCrouchReload | [name] | `state::AIM_CROUCH_RELOAD` ✓ |
| 72 | 167488 | DSTATE_AltAttack | [name] | — |
| 73 | 167590 | DSTATE_StepAimLeft | [name] | `state::STEP_AIM_LEFT` ✓ |
| 74 | 1676a0 | DSTATE_StepAimRight | [name] | `state::STEP_AIM_RIGHT` ✓ |
| 75 | 1677b0 | DSTATE_StrafeAimLeft | [name] | `state::STRAFE_AIM_LEFT` ✓ |
| 76 | 167980 | DSTATE_StrafeAimRight | [name] | `state::STRAFE_AIM_RIGHT` ✓ |
| 77 | 167b50 | DSTATE_StrafeDodgeLeft | [name] | `state::STRAFE_DODGE_LEFT` ✓ |
| 78 | 167cf0 | DSTATE_StrafeDodgeRight | [name] | `state::STRAFE_DODGE_RIGHT` ✓ |
| 79 | 167e90 | DSTATE_RollLeftCrouch | [name] | `state::ROLL_LEFT_CROUCH` ✓ |
| 7A | 167f80 | DSTATE_RollRightCrouch | [name] | `state::ROLL_RIGHT_CROUCH` ✓ |
| 7B | 173c50 | DSTATE_SmokedOut | [name] | — |
| 7C | 16b878 | DSTATE_SmokedOut_Loop | [name] | — |
| 7D | 173d30 | DSTATE_SmokedOut_Recover | [name] | — |
| 7E | 168070 | DSTATE_Investigate | [name] | — |
| 7F | 165ef0 | DSTATE_DroneStuck | [name] | `state::DRONE_STUCK` ✓ |
| 80 | 173570 | DSTATE_HoldItRightThere | [name] | — |
| 81 | 1681f8 | DSTATE_OpenDoor | [name] | — |
| 82 | 1684c0 | DSTATE_KickObject | [name] | — |
| 83 | 168628 | DSTATE_ActionAnim | [name] | — |
| 84 | 1687c0 | DSTATE_StandFiddle | [name] | — |
| 85 | 168910 | DSTATE_EnemyRunToPoint | return state 0x56, unseen → Idle/Alert [M docs §7] | `kiko::ENEMY_RUN_TO_POINT` ✓ |
| 86 | 168aa8 | DSTATE_RunAwayFromObject | [name] | — |
| 87 | 168d18 | DSTATE_HideFromScaryObject | [name] | — |
| 88 | 173658 | DSTATE_RecoverFromScaryObject | [name] | — |
| 89 | 1736f8 | DSTATE_RunToAlarm | none-found arm is the ported path [M docs §7] | `kiko::RUN_TO_ALARM` ✓ |
| 8A | 168ea8 | DSTATE_PressAlarm | [name] | — |
| 8B | 1737d0 | DSTATE_DonePressAlarm | [name] | — |
| 8C | 169150 | DSTATE_RunForCover | [name] | `state::RUN_FOR_COVER` ✓ |
| 8D | 169488 | DSTATE_ElevatorJumper | 07000014's Rook [M docs §2] | — |
| 8E | 169610 | DSTATE_AbseilInit | [name] | — |
| 8F | 169730 | DSTATE_AbseilSlide | sets +0xC5 = +0xC4 on the way out [M 0x169730:36] | — |
| 90 | 169940 | DSTATE_AbseilHang | [name] | — |
| 91 | 1738e0 | DSTATE_AbseilStepOff | sets +0xC5 = +0xC4 [M 0x1738e0:21] | — |
| 92 | 169b28 | DSTATE_AbseilDeath | [name] | — |
| 93 | 169cc0 | DSTATE_UnderCoverInit | [name] | `state::UNDER_COVER_INIT` ✓ |
| 94 | 169e58 | DSTATE_UnderCoverIdle | [name] | `state::UNDER_COVER_IDLE` ✓ |
| 95 | 16a2d0 | DSTATE_UnderCoverAim | [name] | `state::UNDER_COVER_AIM` ✓ |
| 96 | 16a3e0 | DSTATE_UnderCoverFire | ENTER branches to 0x97 (sniper anim set) [M drone.rs:76-79] | `state::UNDER_COVER_FIRE` ✓ |
| 97 | 16a610 | DSTATE_UnderCoverSniperFire | [name] | `state::UNDER_COVER_SNIPER_FIRE` ✓ |
| 98 | 16a8b0 | DSTATE_UnderCoverSniperReload | [name] | `state::UNDER_COVER_SNIPER_RELOAD` ✓ |
| 99 | 16a9c0 | DSTATE_UnderCoverReturn | [name] | `state::UNDER_COVER_RETURN` ✓ |
| 9A | 1739b8 | DSTATE_UnderCoverTypeChange | [name] | `state::UNDER_COVER_TYPE_CHANGE` ✓ |
| 9B | 173ac0 | DSTATE_UnderCoverLeave | [name] | `state::UNDER_COVER_LEAVE` ✓ |
| 9C | 16ab38 | DSTATE_UnderCoverLeaveNow | [name] | `state::UNDER_COVER_LEAVE_NOW` ✓ |
| 9D | 173b38 | DSTATE_SeenOpponent | [name] | — |
| 9E | 16acd0 | DSTATE_SeenDeadBody | [name] | — |
| 9F | 16ade0 | DSTATE_SeenSurrenderedDrone | [name] | — |
| A0 | 173b78 | DSTATE_SeenDroneShot | [name] | — |
| A1 | 173bb8 | DSTATE_SeenExplosive | [name] | — |
| A2 | 173be8 | DSTATE_HeardNoise | [name] | `state::HEARD_NOISE` ✓ |
| A3 | 16af20 | DSTATE_HeardNoiseAware | [name] | `state::HEARD_NOISE_AWARE` ✓ |
| A4 | 16b0e0 | DSTATE_HeardNoiseSuspect | [name] | `state::HEARD_NOISE_SUSPECT` ✓ |
| A5 | 16b540 | DSTATE_HeardNoiseAlert | [name] | `state::HEARD_NOISE_ALERT` ✓ |
| A6 | 16b990 | DSTATE_NinjaStand | blind-cheat branch → 0xA7 [M docs §7] | — (kiko.rs models the entry, no const) |
| A7 | 16bb50 | DSTATE_NinjaAttack | distance bands [M docs §7] | `kiko::NINJA_ATTACK` ✓ |
| A8 | 16bfc0 | DSTATE_NinjaAttackLongRange | [M docs §7] | `kiko::NINJA_LONG_RANGE` ✓ |
| A9 | 16c248 | DSTATE_NinjaAttackMidRange | [M docs §7] | `kiko::NINJA_MID_RANGE` ✓ |
| AA | 16c650 | DSTATE_NinjaAttackShortRange | [M docs §7] | `kiko::NINJA_SHORT_RANGE` ✓ |
| AB | 16cb70 | DSTATE_NinjaGetCloseToPlayer | [name] | — |
| AC | 16d100 | DSTATE_NinjaSword | [name] | — |
| AD | 16d298 | DSTATE_NinjaSomersault | [name] | — |
| AE | 16d430 | DSTATE_NinjaBackflip | [name] | — |
| AF | 173e08 | DSTATE_NinjaSideflip | [name] | — |
| B0 | 16cdd0 | DSTATE_NinjaSideflipLeft | [name] | — |
| B1 | 16cf68 | DSTATE_NinjaSideflipRight | [name] (alert map 0xB1 → 0x12 [M 0x14cbd8:144]) | — |
| B2 | 16d5b8 | DSTATE_NinjaStandFire | [M docs §7] | `kiko::NINJA_STAND_FIRE` ✓ |
| B3 | 16d878 | DSTATE_NinjaNoRoute | [name] | — |
| B4 | 173f00 | DSTATE_DeleteMe | ENTER Enable(0) + obj+0xFE bit 1 (soft removal) [M body 0x173f00:8-15] | `kiko::DELETE_ME` ✓ |
| B5 | 173f68 | DSTATE_FailMission | ENTER SetMissionFailReason(0) then back to +0x5A2 [M body 0x173f68:6-17] | — |
| B6 | 173fc0 | DSTATE_JustStand | UPDATE: sine-sway in place, +0x38 = 1; msg 8 → BulletImpact(0xB6) [M body 0x173fc0:11-30] | — |
| B7 | 16db90 | DSTATE_Tester1 | debug: loops anims 0x1A/0x1B/10 on timer C [M body 0x16db90:13-47] | — |
| B8 | 174150 | DSTATE_Tester2 | [name] debug | — |
| B9 | 1741c8 | DSTATE_Tester3 | [name] debug | — |
| BA | 1742d8 | DSTATE_Tester4 | [name] debug | — |
| BB | 1743d8 | DSTATE_HangUp | [name] | — |
| BC | 174430 | DSTATE_WaitForever | [name] | — |
| BD | 16dd28 | DSTATE_AstronautLaunch | [name + docs §2] | — |
| BE | 174690 | DSTATE_AstronautHit | [name] | — |
| BF | 1747b8 | DSTATE_AstronautDeath | [name] | — |
| C0 | 174458 | DSTATE_AstronautCombat | [name] | — |
| C1 | 174550 | DSTATE_AstronautCombatMove | [name] | — |
| C2 | 16dee0 | DSTATE_SpaceDrake | [name + docs §2] | — |
| C3 | 1748c8 | DSTATE_BotInit | MP bot init [name; mode 0x1E → here, M 0x15b830:17-20] | — |
| C4 | 174938 | DSTATE_BotRespawn | [name] | — |
| C5 | 16e060 | DSTATE_BotGlobal | [name] the bot dispatcher | — |
| C6 | 174980 | DSTATE_BotCollector | [name] personality | — |
| C7 | 174a28 | DSTATE_BotGuardian | [name] personality | — |
| C8 | 174a70 | DSTATE_BotTeamPlayer | [name] personality | — |
| C9 | 174ab8 | DSTATE_BotBully | [name] personality | — |
| CA | 174b00 | DSTATE_BotBerserker | [name] personality | — |
| CB | 174b48 | DSTATE_BotGreedy | [name] personality | — |
| CC | 174b90 | DSTATE_BotVengeful | [name] personality | — |
| CD | 174bd8 | DSTATE_BotJudge | [name] personality | — |
| CE | 174c20 | DSTATE_BotAssassin | [name] personality | — |
| CF | 16f198 | DSTATE_BotAttack | [name] | — |
| D0 | 16f390 | DSTATE_BotAttackRun | [name] | — |
| D1 | 174c68 | DSTATE_BotAttackNoRoute | [name] | — |
| D2 | 16f5e8 | DSTATE_BotAttackFire | [name] | — |
| D3 | 174cd0 | DSTATE_BotAttackNoOpponent | [name] | — |
| D4 | 16f850 | DSTATE_BotAttackStrafeAimLeft | [name] | — |
| D5 | 16f9b8 | DSTATE_BotAttackStrafeAimRight | [name] | — |
| D6 | 16fb20 | DSTATE_BotAttackRunChangePosition | [name] | — |
| D7 | 16fc30 | DSTATE_BotAttackBackoff | [name] | — |
| D8 | 16fe80 | DSTATE_BotAttackCrouch | [name] | — |
| D9 | 174d18 | DSTATE_BotAttackRollLeftCrouch | [name] | — |
| DA | 174dd0 | DSTATE_BotAttackRollRightCrouch | [name] | — |
| DB | 174e88 | DSTATE_BotAttackStepAimLeft | [name] | — |
| DC | 174fa8 | DSTATE_BotAttackStepAimRight | [name] | — |
| DD | 170190 | DSTATE_BotAttackReload | [name] | — |
| DE | 1750c8 | DSTATE_BotAttackChangeWeapon | [name] | — |
| DF | 175188 | DSTATE_BotAttackUnarmed | [name] | — |
| E0 | 1702e0 | DSTATE_BotCoverRunTo | [name] | — |
| E1 | 170448 | DSTATE_BotCoverInit | [name] | — |
| E2 | 1705b0 | DSTATE_BotCoverIdle | [name] | — |
| E3 | 1708e8 | DSTATE_BotCoverAim | [name] | — |
| E4 | 1709d0 | DSTATE_BotCoverFire | [name] | — |
| E5 | 170b90 | DSTATE_BotCoverReturn | [name] | — |
| E6 | 170d00 | DSTATE_BotCoverTypeChange | [name] | — |
| E7 | 170e20 | DSTATE_BotCoverLeave | [name] | — |
| E8 | 170f38 | DSTATE_BotCoverLeaveNow | [name] | — |
| E9 | 175220 | DSTATE_BotStuck | [name] | — |
| EA | 171048 | DSTATE_BotAlertToPosition | [name] | — |
| EB | 171240 | DSTATE_BotGotoGoalPosition | [name] | — |
| EC | 175268 | DSTATE_BotSeenOpponent | [name] | — |
| ED | 1752b0 | DSTATE_BotSeenDroneShot | [name] | — |
| EE | 1752f8 | DSTATE_BotHeardNoise | [name] | — |
| EF | 1713f0 | DSTATE_BotImpactBullet | [name] | — |
| F0 | 175308 | DSTATE_BotImpactExplosive | [name] | — |
| F1 | 175388 | DSTATE_BotImpactPunch | [name] | — |
| F2 | 175508 | DSTATE_BotDeathAnim | [name] | — |
| F3 | 1755a8 | DSTATE_BotDeathByExplosion | [name] | — |
| F4 | 171518 | DSTATE_BotDead | [name] | — |
| F5 | 175418 | DSTATE_BotImpactStunGrenade | [name] | — |
| F6 | 1716d0 | DSTATE_BotDoorOpen | [name] | — |
| F7 | 175680 | DSTATE_BotGuardFriendIdle | [name] | — |
| F8 | 1718e0 | DSTATE_BotGuardFriendFollow | [name] | — |
| F9 | 171b80 | DSTATE_BotIdle | [name] — the last entry [L] | — |

Port match totals: **88 of 250** have a port constant — 77 in `drone.rs::state`, 11 in `kiko.rs`
(all cited above). Every port value agrees with the table; the only mismatches are names/comments
(Disagreements below). The `state` mod also has no Disabled/PlayScript, no Hostage chain beyond 0x0D, no
Interrogate/surrender-adjacent extras (0x3C-0x3E), no Taser/Stun family (0x4A-0x52), no Prone/CrouchCover/AltAttack
(0x6B/0x6C/0x6E/0x72), no SmokedOut/Investigate (0x7B-0x7E), no door/anim states (0x80-0x84), no scare/alarm
sub-chain beyond 0x8C/0x89 (0x86-0x88, 0x8A-0x8B), no Abseil (0x8D-0x92), no Seen* (0x9D-0xA1), no Ninja stand/
flips (0xA6, 0xAB-0xB1, 0xB3), no FailMission/JustStand (0xB5/0xB6), no Tester/HangUp/WaitForever, no
Astronaut/SpaceDrake, and nothing of the Bot block 0xC3-0xF9.

## Disagreements

1. **docs/systems/actors.md §1 row 3** claims start = "0x56 Attack"; the bytes are `ffff0000…` = −1 (by mode →
   props 0x24/0x25 → 6, else 4) [L 0x29aef4 + 2·12; M 0x14d2e8:16-48]. The aggressor row with start 0x56 is
   **row 6** (docs row 6 correct). docs' "team" footnote (switch on the raw dtype) is right — the switch reads
   Drone+0xC7 [M 0x14cbd8:48-61], and in the OLD chain the team byte comes from DroneModeSettings[beh]+1
   [M 0x14cfa0:42].
2. **docs/systems/actors.md §1 GetDTYPENEW bullet** "block 0 + prop 0x10 → 0x10 (CIVILIAN_INIT)": the return 0x10
   is the DroneTypeSettings **row index** (row 16: start 0x37 AmbushInit, alt 0x56) [M 0x14cbd8:23 → 0x14cbd8:104;
   L row 16], not state 0x10 CivilianInit. docs' own §1 row 16 is correct; the gloss is wrong.
3. **docs §1 footnote** "the state id indexes the same table for the alternate": precisely, the alt state is
   `DroneTypeSettings[+0xC6].alt` with fallback `DroneTypeSettings[+0xC5].alt` when 0 [M 0x14cbd8:111-116,
   0x14d2e8:53-62] — indexed by the alt TYPE byte, not by a state id.
4. **drone.rs:38** `COMBAT_DRONE_AHEAD: u8 = 0x5e` — the decompile names 0x5E `NDrone2_DSTATE_CombatWait`
   (0x164960) [M]. Value fine; the port's semantic name has no decompile backing found in this pass.
5. **drone.rs:91** comment "HostageIdle 0x1c (0x15ce60)": 0x1C is AllyLeadInit (0x15e4f0); HostageIdle is 0x0D
   (0x15ce60) [M-sym]. The constant `HOSTAGE_IDLE = 0x0d` (drone.rs:101) is correct — the comment line is wrong.
6. **drone.rs:168** msg `EXPLOSIVE` comment "state 0x86, `RecoverFromScaryObject`": 0x86 is
   `RunAwayFromObject` (0x168aa8); `RecoverFromScaryObject` is 0x88 (0x173658) [M-sym]. Check which id the
   routing actually uses before trusting the comment's id.
7. docs §2's state names all check out against the table [M-sym]; no disagreement there.

## Live checks for the main agent

No game bytes here; EE addresses only. Static data is plain writable EE RAM — the tables are poke-able.
`fieldprobe` CLI: `python -I tools/trace/fieldprobe.py <cmd>` (dump/find/ptrs/watch/poke/diff/shot; PINE, one client).

1. **Confirm the table live**: `dump 0x29aee8 0x3fc` — expect row 0 `ffff 0000 …`, row 2 `2900 2a00 70e51400 …`,
   row 30 `c300 0000 e8911200 …`; and `dump 0x29b300 0x3e8` — expect [0] `30b81500` (0x15b830) [L snapshots].
2. **Find a live drone**: the drone llist head is `NPCGlobals+4` = **0x298e14** [M Drone_EnableAll 0x1383e8:10-16].
   `dump 0x298e14 4` → node; node+0xC = the obj; obj+0xE0 = the Drone data. Then `watch <drone>+0xC5 1 300`
   (the mode byte) and `watch <drone>+0x5A2 2 300` / `+0x5A4` (start/alt states) — 07000001's snipers (dtype 2)
   should show +0xC5 = 2 and +0x5A2 = 0x29.
3. **Poke a start state [L-verified target]**: `poke 0x29aef4 2 0x0029 --keep` (row 1 start 6 → 0x29), then
   spawn/aggro a fresh common-guard drone and watch it stand in SniperIdle instead of patrolling (or watch the
   Drone+0x5A2 of a newly placed guard = 0x29). Restore with the probe's default restore (drop `--keep`).
4. **Poke the hook slot** (row 64 fn4 at **0x29b1ec**): leave for gdb (`docs/tools/pcsx2-gdb.md`, breakpoint
   0x14a648) rather than fieldprobe — a bad pointer crashes at the first sniper friend-switch. It should be 0.
5. **State table dispatch**: gdb break 0x175778, read `param_2` (the state id) and `$t9`-style target against the
   listing above; or watch `drone_data+0x130`'s caller chain. Cheap alternative: break 0x14cbd8 once, read the
   drone from `$a0`, dump +0xC4/+0xC5/+0xC6/+0xC7/+0xC8 and compare with §"How a drone gets its mode byte".

## Unresolved / for the next pass

- Who (if anything) writes the byte-0x304 hook — no writer in ACTION.ELF; assume another module or never used.
- Whether behaviour id 0x23 (OLD path, reads the alias row → +0xC5 = 0xFF, OOB row read) occurs on disc — needs
  the level behaviour roll (task-2 territory).
- The exact placement-param ↔ behaviour-block byte mapping (bytes 0/4/0x10 → +0xC7/+0xC8/+0xC9) — behaviour
  format is task-2; here only `DoModeSettingsNEW` 0x14c600:77-79 is cited.
- The mode-0x17 → state 0x32 (TruckDriverInit) bypass in Global/Disabled/WaitSwitch: three sites agree, but the
  design intent (undercover guard row 23 also starts 0x32 via its own row) vs the CivilianScared alt is not
  chased to a disc placement.
- Rows 15 (start = 0) and the identity rows' non-bot uses (any +0xC5 writer > 0x1E besides the 0xFF edge): none found.
