# The mission layer (`Mission_*`, nf-game::mission)

The game's mission/objective system, ported from ACTION.ELF (fix-mission). The data model, the per-tick
update, the events the view renders, and the level-end/restart flow. **The parallel tasks (032–037
per-mission walkthroughs, 038-fix-actors, 210-fix-progress) build on this module — this document is the
API contract.**

Tags: [M] decompile (work/decomp/ACTION.ELF), [L] EE snapshot / static data, [S] suggested.

## 1. The data (`MissionData` 0x2a4350 + the objective tables)

`ACTION.ELF`'s `.data` holds `MissionData` (0x2a4350, 24 records × 40 bytes — docs/formats/mission-data.md)
and 24 named per-level objective tables (`Castle_Exterior` 0x2a3e28 … `Space_Station_D` 0x2a4320). The
port embeds them as static data in `crates/nf-game/src/mission/c0<level>.rs` (dumped from
ee_start_05.bin [L], the strings resolved from USATxt.dat), exposed as `mission::LEVELS: &[LevelDef]`:

```rust
pub struct LevelDef { pub level: u32, pub mission: u32, pub part: u32, pub objectives: &'static [Objective] }
pub struct Objective { pub text: u32, pub fail: u32, pub hint: u32, pub channel: u8, pub init: u8, pub enable: u8, pub flags: u8 }
```

Objective record (0x18 bytes) fields, established from the readers [M 0x1827d8/0x182270/0x182630]:
`text` (the bank-4 label, e.g. 0x04000000 "Breach the castle walls"), `fail` (a per-objective fail label,
0xffffffff = none in every shipped record), `hint` (the long description — filled but never drawn by any
ACTION.ELF code found), `channel` (the condition switch channel; 0 = the raw `switch_channels[0]`), `init`
(the value `Mission_Init` resets it to), `enable` (the announce gate channel; 0 = announce without it),
`flags` (bit 1 = the channel being ON means FAIL).

## 2. The channels of record

| channel | meaning | writer |
|---|---|---|
| 99 (0x63) | **mission fail** | the objective fail path, the 0x60/0x61 hooks, `DroneFunc_SetMissionFailReason` [M 0x13e560] |
| 100 (0x64) | **mission success** | `Mission_MonitorObjectives`' AND [M 0x1827d8:119-123] |
| 0x62 | player dead | `Player_HandleDeath` 0x190bb0:32 (cleared by `Player_Init` 0x1910f8:26,30) |
| 0x60 | killed civilians | `DroneFunc_SetMissionFailReason` reasons 1/3/6/9 |
| 0x61 | alarm raised | the alarm-button OR gates; reasons 0x10 |
| 0x63 | (the lock on the 07000001 exit touch) | — |

An objective's `channel` is its condition: normally the channel turning ON completes it; with flags bit 1
the channel turning ON FAILS the mission (e.g. 07000009's "Don't let the security guards raise the alarm"
watches 0x61). The conditions of **every part of the mission** AND together — success fires only at the
mission's last level [M 0x1827d8:20-123].

## 3. The runtime (`Mission`, mission.rs)

```rust
Mission::new()                                  // the BSS globals [L ee_start_05.bin 0x30cb98..]
Mission::init(&mut self, level, warm, ch, frame)      // Mission_Init 0x182270
Mission::update(&mut self, ch, frame, level, dead, out: &mut Vec<Event>)  // Mission_Update 0x182c60
Mission::status(&self) -> u32                   // MissionState (0x30cb9c): 1 playing, 2 failed, 3 dead, 6 complete, 5 no-mission
Mission::snapshot()/restore(&MissionSave)       // LS_MakeMission/LS_LoadMission (0x1cd720/0x1cd880) — for saves/210
```

`init` runs once per level load (the port latches it on the level's first `Drones::tick`; the game runs it
from `ResetMap_GameInit` 0x1d5080:53 with `IsWarmReset`). It sets `BaseMap`/`ThisOrderNum`, resets the
current and future parts' objective states (0 = no enable channel, 1 = waiting for the enable channel) and
the condition channels to their `init` values; previous parts keep their states [M 0x182270:68-129].

`update` runs once per tick (the game: from `Game_Run` 0x1c9790; the port: the end of `Drones::tick`, so
the tick's channel writes are visible). While the player is alive it runs
`Mission_MonitorObjectives` [M 0x1827d8]:

- state 0 → announce the objective (type 2, 300 ticks) once the record's level is reached, → 3 (or 2 for
  fail-objectives);
- state 1 → announce when the enable channel fires, → 3/2;
- state 2 (fail-objective) → the channel ON posts "Fail: <text>" (type 2, 180 ticks) and raises ch 99, → 5;
- state 3 → the channel ON posts "OBJECTIVE COMPLETE" (0x02000053, type 2, 180 ticks), → 4;
- state 5 → record the objective's own fail label (if ≠ −1).

Then the standing hooks: ch 0x61 → `FailLabel = 0x04000032` "Alarm Raised" + ch 99; ch 0x60 →
0x04000034 "Killed Civilians" + ch 99; level 0x07000016 ch 0x7b → 0x02000059; level 0x0700001b ch 199 →
0x0200005a. Finally the AND → ch 100.

`Mission_Update`'s state machine over the monitor's verdicts: ch 99 → "Mission Fail" (0x02000007, type 3,
300 ticks) → the 0x40000042 menu — the port **restarts the level**; ch 0x62 → the red fade, MissionState 3
→ the same restart; ch 100 → MissionState 6, "Mission Complete" (0x02000006, type 3, 240 ticks),
`LevelToEndTo` = 0x07300004/08/0b/0d/0f/13/17/1b for the mission's last levels (the DRIVING.ELF
interludes — the port logs and skips them), else 0x07000048 → `ResetMap_LevelToLoad(LevelToEndTo)`.

## 4. The events (what the view does)

`Drones::mission_events: Vec<mission::Event>` — drained by `Sim::hud_tick` (scene.rs):

| Event | The port's action |
|---|---|
| `Objective { label, is_new, ticks, failed }` | the objective banner queue (type 2); the text bound via `Lang::bind`, "Fail: %s" (0x170) when `failed`; the SFX 1532+469 (new) / 468 (complete) [M 0x19c310:150-163, 220-234] |
| `FailLabel(u32)` | the mission banner's second line (`Lang::bind` at draw) |
| `MissionComplete { next }` | the type-3 banner (240 ticks) |
| `MissionFailed` | the type-3 banner (300 ticks) |
| `PlayerDead` | (the red fade is render-side; not drawn yet) |
| `LevelEnd { next }` | `drones.level_exit = Some(next)` for 0x07xxxxxx codes; driving codes (0x0730xxxx) are logged and skipped |
| `Restart` | `drones.level_exit = Some(current level)` — the fail/death restart |

## 5. The HUD (nf-view/src/hud.rs)

Two banners after the pickup bar, per `HUD_UpdateStatusPane` 0x19c310 and the SprInfo tables [L
0x319580/0x319608]:

- **Objective banner** (pane 3, `MsgObjectiveStatusPane`): a 480×56 sheet-0x0300007A column bar at
  (16, 0) with 5-px caps (u 185/191), the status label ("NEW OBJECTIVE" 0x02000051 / "OBJECTIVE
  COMPLETE" 0x02000053) at (128, 20) and the text at (128, 40), left-aligned (fmt 0x30cdb0 align 2),
  colour 0x7D6D59, layer 29; alpha = time·17 over the last 15 ticks; closes 16 px/tick.
- **Mission banner** (pane 2, `MsgMissionStatusPane`): a 512×103 bar at (0, 156) (uv 198,0), the text
  centred at (256, 42) (fmt 0x30cdb8 align 1), the fail reason at (256, 64) — flashing 0x785A14 while a
  type-2 notice is up; closes 32 px/tick [M 0x1a1028 sets the pane's type 3 and slide 0x20].

All the strings resolve through the existing `Lang::bind` (USATxt.dat) — no new bindings were needed.

## 6. The escort AI (`AllyLead`, drone.rs)

07000001's Mayhew (skin 0x0500004F, switch channel 0xB, mode 12) starts in **0x1c AllyLeadInit**
(`NDrone2_StateFuncs[0x1c]` [L 0x29b300]; `DroneTypeSettings[12].start` [L 0x29aee8] — the port had this
state id mapped to a standing HostageIdle, which is why he never walked). Ported: AllyLeadInit 0x1c
(0x15e4f0 — prop 0x1d → `AssignAIPath(8)`, Bond within 3 → Wait), AllyLead 0x1d (0x15e698 — walk the
mission path, done → Done, Bond far behind → Wait), AllyLeadWait 0x20 (0x15eda8), AllyLeadDone 0x23
(0x15f2e8). The ch 0xB release comes from the level-start intro's event 0xD [L ee_start_07000001] — the
escort flow end to end: intro → ch 0xB → Mayhew leads Bond along path 0 (kind 8) to the bunker plate →
Touch #137 (out 0x10, filters [player, skin 0x4F]) → LOAD_LEVEL → 0x07000002. Not ported (038-fix-actors):
the flee/combat sub-states 0x1e PlayerInWay, 0x1f Hide, 0x21 MissionWait, 0x22 BondCombat, 0x28
GoToGoalPosition, and the exact `ObjectMissionPathDistance` (0x153f50) wait rules (the port uses
straight-line stand-ins).

## 7. Known gaps

- The pause-screen objectives list (`Mission_ObjectiveState` 0x182630, `Mission_NumVisObjectives`
  0x182540, drawn by `C_GCPAUSE_Handler` 0x204408) is not ported.
- The fail/death 0x40000042 menu is a level restart; the red death fade is not drawn.
- `DroneFunc_SetMissionFailReason` (0x13e560) is not wired to the drone paths that call it (the punch/
  alarm reasons) — the channels 0x60/0x61 hooks are.
- A cold load of a mid-mission level leaves previous parts' objectives at their `.data` states (the
  game's saves carry them via LS_MakeMission — `MissionSave` is the port's hook for 210-fix-progress).
- Ch-0 conditions read the raw `switch_channels[0]` (the game has no zero guard [M 0x1827d8:33-37]);
  the port's `Channels::set` refuses 0, so an out-0 trigger cannot set it.
