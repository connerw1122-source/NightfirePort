# drone-investigate — NDrone2's Investigate state (0x7E)

## What was asked

Port `NDrone2_DSTATE_Investigate` (state 0x7E): what triggers it (a sound, a corpse, a thrown object), where
the guard walks and how long he searches, how it hands back to his normal state or escalates; then tests on
07000005 — a sound below the alert threshold sends a guard to investigate its origin and he returns to his
post at the game's tick, with a sister that hears nothing staying put. First check whether master already
behaves like the game.

## Finding: master already behaves like the game; Investigate 0x7E is cheat-only

**Dispatch [M].** `NDrone2_ProcessStateMachine` (0x175778) indexes the `NDrone2_StateFuncs` fn-pointer table
with the state id (ids < 0xfa). So 0x7E can only be entered by (a) a `Drone_SM_SetState(.., 0x7e, ..)` call,
(b) a store of 0x7e into the drone's state fields, or (c) a DroneTypeSettings start row.

**The only entry site in all of ACTION.ELF is cheat-gated [M].** An exhaustive grep for `0x7e` across every
decompiled C file finds exactly one state-typed store: `NDrone2_DSTATE_CombatOutOfRange` (0x164690):30,
`*(u16*)(drone+0x5a2) = 0x7e` — inside the ENTER branch taken only when the global `switch_BLIND_DRONES != 0`
(0x164690:19). That branch also stores `+0x5a0 = 0x5f` (CombatNoRoute, the resume state), invalidates the
route, sets a 2.0-radius goal, and does `SetState(99 = 0x63 GoToGoalPosition)`: the blind guard walks to his
last goal and lands in Investigate on arrival (GoToGoalPosition 0x165640 consumes +0x5a2). No
`SetState(0x7e)` literal exists anywhere, and `Drone_CoderCreate` (0x13a108) callers pass no 0x7e.

**`switch_BLIND_DRONES` is the Blind Drones cheat [M].** Its only writer is `C_CHCHBLIND_Handler`
(0x215d60): menu message 0x4b toggles it, 0x51 reads it back for the menu. It is a .bss global, so retail
play starts with it 0 — with the cheat off, `CombatOutOfRange` ENTER takes the normal branch
(AlertStatusSet(1), alert = 1.0) and 0x7E is unreachable. `DroneFunc_HandleSoundAlerts` (0x148a30:43) and
`DroneVision_ProcessDroneSight` (0x176a28:105) also require `switch_BLIND_DRONES == 0`, so with the cheat on
guards neither hear nor see normally — the cheat is the state's whole reason to exist.

**The task-710 audit is corrected on one point.** `docs/systems/uncited-audit.md` said "no +0x5A2 store" —
there is one (0x164690:30, above). Its conclusion (dead in normal play) stands, and I verified the "no
SetState literal / no caller" parts independently.

**The game's real below-threshold sound path is the HeardNoise family — and master already ports it [M].**
`DroneFunc_HandleSoundAlerts` (0x148a30, every drone every frame) sums `Sound_Alertness` (0x1ddc48) into
`+0x510` (loudness), adds `share = loudness·0.01` to the accumulated alert `+0x50c`, and when the sound is
newer than 60 ticks or louder than the last peak, stores the loudest sound's position and pings msg 0x14 to
itself. `DroneVision_EnemyAlerts` (0x177c18) routes msg 0x14 to `DroneVision_AlertSound` (0x177800), which
decides: from any state outside 9/0x12–0x16 the answer is **0xa2 HeardNoise** unless the sound was quiet
(share ≤ 0.2 and alert ≤ 0.66 → stay); the **0x56 Attack** return needs alert > 0.7 from inside the
9/0x12–0x16 block, or an already-attacking flag. So a below-threshold sound never attacks — it investigates.
The walk/search/hand-back:

| step | state | game behaviour [M] |
|------|-------|--------------------|
| hear | 0xa2 HeardNoise (0x173be8) → ENTER 0xa3 | noise level 2, alert look variant 1, back = resume +0x5a0 |
| Aware 0xa3 (0x16af20) | look; a further sound with share > 0.1 → 0xa4 | 0x16af20:67-75 |
| Suspect 0xa4 (0x16b0e0) | `SetSoundAlertRoute` 0x154aa8 homes the goal onto the loudest sound (radius 2.0); with the walk prop 0x21 sets +0x5a2 = 0x65 and plays the alert look with back 0x62; another sound → 0x56 Attack | 0x16b0e0:21-78, 120-126 |
| AlertToPosition 0x62 (0x165270) | walk anim 0xf (run when the route ≥ 10, `MoveToAlertPosition` 0x154ba8), 300-tick timer (+0xcc4 = tick + FRAME_RATE_INT·5, expiry msg 0xc only speeds him up, `PreDroneControl` 0x148db0:97-111); arrival SetStates +0x5a2 = 0x65 | 0x165270:21-76 |
| SearchArea 0x65 (0x165bf0) | alert look; timer 180 ticks (tick + FRAME_RATE_INT·3, 0x165bf0:25); then the done msg (0x25) sets alert `+0x50c` = 0, +0x5a2 = +0x5a0 (the resume state), homes the goal onto the post +0x5b0 (radius 1) and SetStates 0x63 | 0x165bf0:19-28, 113-124 |
| GoToGoal 0x63 (0x165640) | walks home; arrival SetStates +0x5a2 → back to his post's state | 0x165640:60, 181 |

Escalations: a sound while Suspect (share > 0.1) or while AlertToPosition/SearchArea (share > 0.2, not
already attacking) goes straight to 0x56 Attack; seeing the player goes to combat via the normal sight path.

**Every row of that chain is already on master** in `crates/nf-game/src/drone.rs` (HEARD_NOISE →
HEARD_NOISE_AWARE/SUSPECT/ALERT → ALERT_TO_POSITION → SEARCH_AREA → GO_TO_GOAL with the same anims, timers
300/180, the 60-tick/peak re-hear gate, the home goal +0x5b0 and the alert reset), each with its [M] address
in a comment. The existing `shot_sends_a_guard_through_the_noise_chain` test pinned the chain's start; the
walk-to-origin-and-back half was unpinned.

## What I did

- **No lib code changed.** Master matches the game on this whole path; per the task, I pinned it instead of
  rewriting working code.
- **New pinning test** `crates/nf-game/tests/ai_live.rs::two_sounds_send_the_idle_guard_to_investigate_and_back_to_his_post`
  (Level05 = 07000005, the idle guard placement 9 at (99.69, 16.78, 101.58), seed 1, Bond behind-right at
  (108, 104) so the guard is drawn and his look anims play out instead of being culled):
  - sound 1 (sfx 1, weight 75) 14.7 units ahead → share 0.27: **HeardNoise → Aware at tick 101**, the frame
    after the shot, and **no Attack anywhere in the log** — the below-threshold pin (AlertSound 0x177800);
  - a louder sound (sfx 4) 12.0 units ahead → share 0.48 > 0.1 lands while he is still Aware (the re-hear
    gate at 0x148a30 requires the second sound to be the louder one within 60 ticks): **Suspect →
    AlertToPosition at tick 109**, he leaves his post and walks to the origin;
  - **SearchArea at tick 535** within 2.5 units of the sound's origin (a real ≥ 10-unit walk away), spanning
    265 ticks (≥ the game's 180 [M 0x165bf0:25]);
  - **GoToGoal at 800, Idle back at his post at tick 889** — 12.5 units home in 153 ticks — with his
    accumulated alertness back to 0.0 (the done msg 0x165bf0:113-117);
  - sisters: guard 8, 62 units from the sound (beyond its outer radius), stays Idle at his exact post; the
    WaitSwitch waiter 302 never moves (a sound from WaitSwitch needs alert > 0.7 to react at all,
    AlertSound 0x177800:52-64).

## Verification

The background suite runner is killed between tool calls in this sandbox (observed twice), so the rule-5 set
ran per crate in the foreground on the final tree (= origin/master c8186aa1 + this commit; master had not
moved, so no merge was needed), `cargo test --offline --release -j2 -p <crate> -- --test-threads=1`:

| crate | passed | failed | ignored |
|-------|--------|--------|---------|
| nf-game | 467 | 0 | 2 |
| nf-formats | 92 | 3 (the allowed disc_golden trio on the slim ISO) | 0 |
| nf-audio | 15 | 0 | 0 |
| nf-view | 180 | 2 (pre-existing 0c-pylon render probes — they panic on `request_adapter` in this headless sandbox; master baseline, no GPU here) | 9 |
| **total** | **754** | **5** | **11** |

All 5 failures are pre-existing master-baseline environment failures (slim-ISO goldens + no-GPU pylon
probes), none touched by this task (this commit adds a test file and docs only).

## What's left

- `DSTATE_Investigate` 0x7E itself stays unported — it is unreachable without the Blind Drones cheat, and the
  port has no cheat-menu handlers (no C_CHCH* wiring), so porting it now would be dead code. If a cheat task
  ever lands, port `CombatOutOfRange`'s blind branch (0x164690:19-32) and 0x7E together — the handler is small
  and fully understood: ENTER = flags 0x10000/0x2000, AlertStatusSet(2), anim 0x21; msg 3 = LookForOpponent,
  no opponent and alert < 0.66 → Idle (4), else stay; msgs 0x11/0x12/0x14/0x15/0x16/0x1e → EnemyAlerts; 0x1f
  forced attack; 0x21 explosive.
- Live checks for the main agent (below).

## Live checks for the main agent

1. **Cheat reachability of 0x7E** (confirms the dead-state verdict on real hardware): with the BLIND cheat
   active (cheat menu), start a fight on 07000005 and break line of sight so a guard's combat target goes out
   of range. Expected: the guard walks to his goal (GoToGoalPosition) and then does a long standing alert
   look (Investigate, Drone+0xC5 == 0x7E, alert status 2) instead of instantly re-acquiring; watch +0x5a2
   (gdbwatch `--addr` on the drone's +0xC5/+0x5A2) hold 0x7E through the walk.
2. **Below-threshold chain visuals** (same spots as the test): silenced shot ~15 units in front of the idle
   guard at (99.69, 16.78, 101.58) with Bond behind-right at (108, 104) — expected: one alert-look, no
   approach; a second, louder noise within a few seconds — expected: he walks to the noise, looks around ~3 s,
   then walks back to his post and relaxes (no combat). If the on-screen look gets culled instead (Bond moved),
   the guard should return to Idle immediately — matching the existing noise-chain test.

## Usage

not visible to me
