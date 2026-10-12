# fix-mission — the mission layer: objectives, mission flow, level ends

Branch `s/fix-mission`. Everything below was verified through the real input path (`nf_view::load` +
`Sim::step_bond`), with the decompile as the source of truth. **The API contract for the parallel tasks
(032–037, 038, 210) is `docs/systems/mission.md`** — read that first.

## What was missing

The port had no mission layer at all: no objectives, no mission-complete/fail flow, no objective HUD —
and 07000001's exit stood open only as a mechanism (a test that force-set the NPC's release channel),
because nothing made the escort NPC walk to the exit plate.

## What was found in the decompile

- **The data**: `MissionData` (ACTION.ELF 0x2a4350, 24 records × 40) points at 24 named objective tables.
  The 0x18-byte objective record is fully decoded (U-024 in docs/formats/mission-data.md is closed):
  text / fail / hint labels, the condition switch channel + its init value, the enable channel, flags
  (bit 1 = the channel turning ON fails the mission), and the runtime state. Every level's objectives are
  now listed in `docs/systems/levels.md` §7 (55 objectives over 24 levels, texts resolved from
  USATxt.dat).
- **The flow**: `Mission_Init` (0x182270, from `ResetMap_GameInit`), `Mission_MonitorObjectives`
  (0x1827d8 — the per-objective state machine; the AND of every part's conditions raises **success ch
  100** so it fires at the mission's last level only; the standing fail hooks ch 0x60 "Killed Civilians"
  and ch 0x61 "Alarm Raised"), `Mission_Update` (0x182c60 — fail ch 99 → "Mission Fail" → the 0x40000042
  menu; player-death ch 0x62 → the death flow; success → "Mission Complete" →
  `ResetMap_LevelToLoad(LevelToEndTo)`, where the mission-ending levels point at the 0x0730xxxx DRIVING
  interludes the port cannot load).
- **The escort**: 07000001's Mayhew (skin 0x0500004F, mode 12) starts in state 0x1c which the port had
  mapped to a standing HostageIdle. The state table says 0x1c is **AllyLeadInit** — the escort lead that
  walks the mission AI path (path 0, kind 8) to the bunker plate. His release channel 0xB is written by
  the level-start intro's event 0xD [L ee_start_07000001].

## What was ported

1. **`crates/nf-game/src/mission.rs`** (new) + `mission/c0<level>.rs` (24 data files): the embedded
   objective tables (`mission::LEVELS`), `Mission::init`/`update`/`status`, the `Event` enum (the
   notices, the FailLabel, the level end, the restart) and `MissionSave`/`restore` — the end-of-mission
   data hook 210-fix-progress asked for. `Drones` owns a `Mission`, lazily inited per level, ticked at
   the end of `Drones::tick` (a ~15-line shared-file touch, including the ch 0x62 death write from
   `Player_HandleDeath` 0x190bb0:32).
2. **The HUD notices** (hud.rs + scene.rs): the objective banner (480×56, "NEW OBJECTIVE"/"OBJECTIVE
   COMPLETE", left-aligned text, the game's colours/fades/close speeds) and the mission banner (512×103,
   "Mission Complete"/"Mission Fail" centred, the FailLabel line flashing while an objective notice is
   up). All labels resolve through the existing `Lang::bind` — no new bindings (the first pass guessed
   the label ids as 0x200000xx; the real ids are bank-2 0x020000xx — the tests caught it).
3. **The escort AI**: `HOSTAGE_IDLE` re-homed to its real state 0x0d, and AllyLeadInit/Lead/Wait/Done
   (0x15e4f0/0x15e698/0x15eda8/0x15f2e8) ported at 0x1c/0x1d/0x20/0x23 over the existing mission-route
   machinery.
4. **The 07000001 exit end to end**: the intro plays (unskipped) → its event 0xD writes ch 0xB → Mayhew
   enables and leads Bond ~199 units along path 0 → Bond E-opens the courtyard slide door en route →
   Mayhew reaches the plate (AllyLeadDone) → Bond joins → Touch #137 fires ch 0x10 → LOAD_LEVEL →
   `level_exit = Some(0x07000002)`.

## How it was verified

Four tests in `crates/nf-view/tests/fix_mission_live.rs`, each shown failing before the fix, each with a
sister assertion:

- `objective_announces_on_its_enable_channel_and_completes_on_its_condition` — 07000005: no notice until
  the enable channel 0x45, then "Breach the castle walls" (the bound text asserted), then "OBJECTIVE
  COMPLETE" on the condition channel 0x46; sister: an unrelated channel does nothing.
- `mission_fails_when_the_alarm_raises` — 07000009: the alarm button's channel 0x7C through the disc's
  own OR gate (placement 278, out 0x61) → "Fail: Don't let the security guards raise the alarm" +
  FailLabel "Alarm Raised" + ch 99 → the "Mission Fail" banner → the level restart; sister: without the
  alarm the mission keeps playing.
- `mission_completes_when_every_objective_channel_is_on` — 07000001: all eight of the mission's
  condition channels → ch 100 → "Mission Complete" → the level end at `LevelToEndTo` 0x07000048; sister:
  one channel short → no success.
- `the_07000001_exit_works_end_to_end` — the full escort described above; the assertion is the LOAD_LEVEL
  to 0x07000002 firing with both actors on the plate.

Full suite: nf-game 129 passed, nf-view 152 passed, formats/audio green except the 3 allowed
`disc_golden` slim-ISO failures. (One shared-file fix-up: `hud_playtest_live`'s two `Msg` literals took
the new `status`/`failed` fields.)

## Shared-file touches (per the task rules)

- `crates/nf-game/src/drone.rs` — the mission fields + tick + the ch 0x62 write (~15 lines), the
  AllyLead states, the `HOSTAGE_IDLE` re-homing.
- `crates/nf-view/src/scene.rs` — the event drain in `hud_tick`, the banner queues on `HudState`.
- `crates/nf-view/tests/hud_playtest_live.rs` — two `Msg` literals.

## What is left

- The pause-screen objectives list (`Mission_ObjectiveState` 0x182630 + `C_GCPAUSE_Handler` 0x204408) is
  not ported.
- The fail/death "menu" is a level restart (the port has no menu system); the red death fade is not
  drawn.
- `DroneFunc_SetMissionFailReason` (0x13e560) is not wired to the drone paths that call it in the game
  (the punch/alarm fail reasons); the channel hooks it feeds are.
- The AllyLead sub-states (PlayerInWay 0x1e, Hide 0x1f, MissionWait 0x21, BondCombat 0x22,
  GoToGoalPosition 0x28) and `ObjectMissionPathDistance`'s exact wait rules are straight-line stand-ins —
  038-fix-actors.
- A cold load of a mid-mission level leaves previous parts' objectives at their `.data` states; the
  game's saves carry them (`LS_MakeMission` 0x1cd720) — the port's hook is `MissionSave`.
- The escort end-to-end test carries a NAV-GAP STAND-IN: the shared nav bounds/creep stalls Mayhew at the
  courtyard junction where the game's drone slides along the wall; the test nudges him toward his route
  target there. The nav follow itself is not this task's file list.
