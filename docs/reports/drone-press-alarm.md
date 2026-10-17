# drone-press-alarm — report (branch `s/drone-press-alarm`, ends mid-task at sandbox expiry)

Status: **partially done** — the decompile work is complete and the state chain is ported and
compiles clean; the end-to-end sim test was **not** written or run (the sandbox hit its 2-hour limit
mid-implementation, plus a disk-full detour). Everything below is pushed on the branch
(commits 28582c23, eb888ad0) for the next message to continue.

## What the game does (all [M], verified against decompile + disassembly)

The PressAlarm chain, from the entry points down:

- **How a guard is picked / how the state is reached** — three ways into RunToAlarm 0x89:
  1. `DroneTypeSettings` row 14 (0x29aee8) = start/alt **0x89** — mode 14 drones start there
     [M-dis table dump]. Mode 14 comes from `DroneModeSettings` row 18 = "AlarmRaiser"
     (mode 14, team 1, alert float 1.0, init `NDrone2_init_DMODE_AlarmRaiser` 0x14df98 = props
     0x23 + 0x56), or `GetDTYPENEW` 0x14c938 block 3 with prop 0x56 set → mode 0xe.
  2. `EnemyRunToPoint` 0x85 ENTER with behaviour property **0x56** set → `SM_SetState(0x89)`
     [M 0x168910:17-27].
  3. **The disc's real trigger (levels 0x07000008/0x07000009 only)**: `DroneFunc_HandleImpact`
     0x1469b8 (msgs 8/9/0x17/0x18/0x19 — the stamps at :110/:227/:292/:338) checks
     `GameState ∈ {08, 09}`; if the drone was sighted (+0x4F8 & 0x4000000) it **reassigns the
     guard**: `+0x5A4 = 0x89`, mode = 9, alt mode = 7, `+0x5A2 = 0x14` (CivilianPatrol), `+0x16 = 0`,
     `+0x4FC = 0`. The guard then walks CivilianPatrol; on his first own sighting
     `DroneFunc_FirstAttack` 0x148010:106-156 returns **+0x5A4 = 0x89** as the next state → RunToAlarm.
  4. (Also `KikoMissionRun` 0x1a → 0x89 on 0x07000016 — already ported before this task.)
- **How the panel is picked**: `NDrone2_FindAlarmPoint` 0x153008 — reuse the assigned AI point
  (Drone+0xA70) if it is type 8, else `NDrone2_ClosestAIPoint` 0x151b18 (type 8, enabled, same AI
  path, per-node cost ≠ 0xff, nearest; the cel-match word AIPoint+0xC = Drone+0x144 preferred);
  stores the point's facing yaw (AIPoint+0x44 = placement rotation y [M-dis 0x1530c8]) into
  **Drone+0x734**, sets the goal to the point position and `MoveToGoalPosition` — found iff the move
  code is 0..3 [M-dis jump tables 0x2f6390/0x2f63d0]. Then `+0x5A2 = 0x8A PressAlarm`,
  `+0x5A0 = 0x56`, `+0x16 = 1`, GoToGoalPosition (99).
- **The press**: `PressAlarm` 0x8A (0x168ea8) ENTER — face the panel (`+0x694 = +0x734`), arms timer C
  3 s with payload 0 (`+0xCC4/+0xCC8/+0xCC0`; the dispatcher is `PreDroneControl` 0x148db0:93-108,
  msg 0xC carries +0xCC8), stand anim **0x21 on 07000009/0a else 0x22** [M :24-29]. Timer C payload 0 →
  the press anim **0x3c** with follow-on state 0x8B, timer C 0.5 s payload 1. Timer C payload 1 →
  **`DroneFunc_SetMissionFailReason(drone, 0x10)`** 0x13e560 — `Switch_GetNearest` 0x1b0398 finds the
  nearest Switch object (0x29 placement, channel = its param 0) to the guard's position and
  **`switch_channels[ch] = 1` + the time stamp**; ch 0x61 takes the "Alarm Raised" fail label
  0x4000032. Non-civilians also get the speech timers (+0x594 = 0x32, +0x584 = 0x1e) and `+0x16 = 0`.
- **Abandonment**: the impact msgs (6/7/8/9/0x17/0x18/0x19 → `HandleImpact` with allow 0/0/1; death
  and stun run through the same base handler → the normal death/stun states); msg 3 (UPDATE) is
  `NDrone2_SeenAndAttacking` 0x145150 — the music side only, not an abort.
- **After the press**: `DonePressAlarm` 0x8B (0x1737d0) — voice set (+0xD8) 0x13 → JustStand 0xb6;
  else weapon (anim+0x62) < 2 and Drone+0x45 (= (byte)placement param 23, `DefaultInit` 0x14b300:266)
  < 2 → **clear prop 0x56**, `NDrone2_FindRunToPoint` 0x153168 (prop 0x56 cleared → team 1 → type-0
  points): found → `+0x5A2 = 0x12` + state 100 = **HostageGoToGoalPosition** 0x64; none → state 0x12
  (CivilianScared). Otherwise Attack (0x56).

## The disc census [D] (measured through the port's own placement parser)

- **AI points (0xE9 placements, type = param 0)**: type-8 alarm panels exist ONLY on
  **0x07000009** (12, on sub-map 39) and **0x0700000a** (9, on sub-map 37). Every other story level
  has only type-0 AI points.
- **Alarm switches (0x29, ch = param 0)**: **0x0700000a: 10 switches on ch 0x61** (map 37) + 1 on 59;
  **0x07000009: 1 switch on ch 0x37** (map 39, the tunnel klaxon — matches `DroneFunc_CheckAlarmRaised`
  0x1453a8 reading ch 0x37) plus 124/125 switches; ch 0xa4 has no switch on any main map (level
  07000005's channel comes from elsewhere).
- **The guards**: 0x07000009 has 6 guards (skins 0x5000027/28), 0x0700000a has 20 (0x5000029/2a);
  all dtype 0xd, block param 16 = 0, behaviour id param 5 = 0x64 (NEW), param 23 = 0x20000000
  (→ +0x45 = 0 < 2, so DonePressAlarm takes the return-to-patrol arm; weapon param 9 = 1 < 2).
  Prop 0x56 is set on both levels at creation (`DoModeSettingsNEW` 0x14c600:112-125 — already ported
  as `init_properties`).
- So: **0x07000008/0x07000009 reach the alarm through the HandleImpact reassignment; 0x0700000a's
  20 guards all carry prop 0x56 (the EnemyRunToPoint gate) and 9 panels + 10 ch-0x61 switches.**
  The concrete event that flips a 0x0700000a guard into 0x85/0x89 is **not pinned yet** — the
  candidates are the mission script enabling them (`WaitSwitch`/second-channel pulse → the +0x5A4
  route, `PreDroneControl` 0x148db0:59-88) or the alert chain; needs the live check below.

## What is ported on the branch (compiles; nf-game `cargo check` clean, nf-view check aborted on disk space)

- `AiPoint` + the level's point/switch registry built in `Drones::attach_level` from the 0xE9/0x29
  placements (type = param 0, yaw = rotation y, [M-dis verified field map]) and stamped onto each drone.
- `closest_ai_point` / `find_alarm_point` (0x153008 incl. the assigned-point reuse arm and the
  move-code 0..3 gate) / `find_run_to_point` (0x153168 with the prop-0x56/0x4e/team kind table and the
  team-3 type-1 fallback).
- The real `RUN_TO_ALARM` arm (replaces the "no alarm-point registry → Idle" stub), `PRESS_ALARM`
  (ENTER / TIMER_C payload 0 / TIMER_C payload 1), `DONE_PRESS_ALARM`, `HOSTAGE_GO_TO_GOAL` UPDATE,
  `JUST_STAND` ENTER (kiko.rs constants 0x8a/0x8b/0x64/0xb6 — new names, nothing renamed).
- `EnemyRunToPoint` ENTER prop-0x56 gate → RunToAlarm.
- The channel raise through the real `Switch_GetNearest` semantics (`nearest_switch_channel`, all
  maps' 0x29 placements) feeding the existing `fail_reason` outbox → ch 0x61/0x37/... + the
  0x4000032 label; the +0x594/+0x584 speech timers are not modelled (noted below).
- `Drone` fields (own lines, merge lesson 4): `alarm_yaw` (+0x734), `alarm_run` (+0x16),
  `timer_c_payload` (+0xCC8 — the scratch word's other readers never overlap PressAlarm),
  `alarm_patrol` (+0x45), `alarm_point` (+0xA70), `ai_points`, `alarm_switches`.

## Not done (the next message continues from the branch)

1. **The HandleImpact 0x1469b8 08/09 reassignment** (the `sighted` arm: mode 9 / alt mode 7 /
   +0x5A4 = 0x89 / +0x5A2 = 0x14 / +0x16 = 0) is NOT in the port yet — that is the main entry path on
   the level with the most panels. The stamps sit at 0x1469b8:110 (bullet), :227 (flash), :292, :338.
2. **Tests** — the task's two scenarios are not written: (a) on 0x0700000a an alerted guard runs to
   the nearest panel, presses, ch 0x61 rises at the game's tick (3 s wait + 0.5 s press, the
   `attack_gate`-free path through `PRESS_ALARM`'s timers); (b) the sister: the same guard shot
   mid-run never raises it. The harness pattern to use is `guard_reaction_live.rs` (Level05-style
   owned Data; swap the level bin for 0700000a and drive the guard's state through the real tick).
3. **The seen-and-attacking tick list** — add `RUN_TO_ALARM | PRESS_ALARM` to the `Drones::tick`
   gate (drone.rs ~line 8649) so the +0x16 music post fires while running.
4. Full suite (`tools/test/suite.py`) — NOT run (background jobs die between tool calls in this
   sandbox; run it in foreground chunks `--only`, or as the driver does). The baseline was never
   captured either; the last merged-tree full run on record is the kimiko task's
   `1864 passed, 6 failed, 72 ignored` (docs/WORKLOG.md 2026-10-16 16:45 UTC).
5. `PRESS_ALARM`'s msg-9/0x17/0x19 arms pass allow=1 (explosive/gas/shot-stat) — the port routes
   those through the GLOBAL fallback (allow 0) and has no msg-9 route; only the bullet/punch/tase
   arms are exact. `HostageGoToGoalPosition`'s msg 0x1b arm and `DistanceToAIPoint` are not ported.

## First check the task asked for: does master already behave like the game?

No. `master`'s `RUN_TO_ALARM` arm was the documented stub ("no alarm point reachable on the ported
paths → Idle"), no `PRESS_ALARM`/`DONE_PRESS_ALARM` states existed, and the 0xE9 AI points were never
parsed (docs/systems/actors.md:219 said "the port has no 0xe9 AI-point"). The new census test (when
written) fails on the old behaviour by construction (the stub never raises ch 0x61).

## Live checks for the main agent

1. **0x0700000a entry**: with `gdbwatch.py` on a guard's `+0xC5`/`+0x5A2`/`+0x5A4` (drone base =
   obj+0x110), fire the alarm scenario and record which write chain puts a guard into 0x89 — expect
   either the mission script's `Drone_Message` (WaitSwitch enable → row-14 start) or
   `HandleImpact`'s stamp. Address hints: the +0x5A4 write sites 0x1469b8:110/227/292/338 and
   `PreDroneControl` 0x148db0:59.
2. **Press timing**: breakpoint 0x168ea8 case 0xC's `SetMissionFailReason` call (0x13e560) on
   0x0700000a; confirm it fires ~3.5 s (210 frames) after PressAlarm ENTER, and that
   `switch_channels[0x61]` (0x26fc90+0x61) goes 1 on that frame.
3. **0x07000009 klaxon**: confirm the nearest switch to the map-39 panels is the ch-0x37 one
   (`Switch_GetNearest` return) — the port's all-map scan says ch 0x37 [D], the tick trace should
   show `switch_channels[0x37]` rising.

## Usage

not visible to me — this sandbox exposes no token, cost or context-window figures anywhere in the
interface or the session metadata I can read (the IM gateway metadata carries only session/chat/trace
ids). Input/output token counts: not visible to me.
