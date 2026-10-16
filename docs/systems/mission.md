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
0xffffffff = none in every shipped record), `hint` (the long description — drawn by the pause
OBJECTIVES tab's ✕ Hint box: `C_GCPAUSE_Handler` 0x204408 case 1 binds record+0x08 in the ok-only
option box, 0x31d substituted for −1 [M 0x205bd0-0x205bdc → Menu_CreateOptionBox 0x202218;
docs/systems/objective-hints.md]), `channel` (the condition switch channel; 0 = the raw `switch_channels[0]`), `init`
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

`update` runs once per tick (the game: from `Game_Run` 0x1c9790:17, BEFORE the object handler — so the
monitor reads last frame's channels and a trigger's announce lands after any cut-scene its trigger
started [M; fix-objective-banner]; the port: the top of `Drones::tick`). While the player is alive it runs
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
| `Objective { label, is_new, ticks, failed }` | the objective banner queue (type 2); the text bound via `Lang::bind`, "Fail: %s" (0x170) when `failed`. The SFX play from the pane, not when the event lands: case 2 (the open) only schedules the pair [M 0x19c310:148-164]; case 3 — the next pane frame — fires 1532 (468 for "OBJECTIVE COMPLETE") once the flash window (remaining + 60 > life), ONLY while `ScriptCam == 0` [M 0x19c310:219-229], and 469 sixty ticks into the display (the steady phase; new objectives only) [M 0x19c310:232-235]. A cut-scene starting inside a banner's flash window holds the chime at the hand-back (the timer frozen by the same viewer+0x2a gate [M 0x1b11d8:16-18]) [fix-objective-banner; objective-banner-timing] |
| `FailLabel(u32)` | the mission banner's second line (`Lang::bind` at draw) |
| `MissionComplete { next }` | the type-3 banner (240 ticks) |
| `MissionFailed` | the type-3 banner (300 ticks) |
| `PlayerDead` | (the red fade is render-side; not drawn yet) |
| `LevelEnd { next }` | `drones.level_exit = Some(next)` for 0x07xxxxxx codes; driving codes (0x0730xxxx) are logged and skipped |
| `Restart` | `drones.level_exit = Some(current level)` + `level_exit_restart` — the fail/death restart: the CURRENT level, warm, on the entry checkpoint [M 0x216618:19-21] |

## 5. The HUD (nf-view/src/hud.rs)

Two banners after the pickup bar, per `HUD_UpdateStatusPane` 0x19c310 and the SprInfo tables [L
0x319580/0x319608]:

- **Objective banner** (pane 3, `MsgObjectiveStatusPane`): a 480×56 sheet-0x0300007A column bar at
  (16, 0) with 5-px caps (u 185/191) — case 1 re-pins the middle's x/w to the template on every
  open [M 0x19c310:101-111], so the pane-x 128 add at creation is overwritten for the bar. The
  status label ("NEW OBJECTIVE" 0x02000051 / "OBJECTIVE COMPLETE" 0x02000053) and the text are
  text sprites at template (128, 20)/(128, 40) + pane x 128 = **256, justify centre** (fmt
  0x30cdb0's 0xfe-3 byte [M Font_ParseFormat 0x1ee3e0; Font_DrawText 0x1eee48 flag 2 → x −= w/2])
  — centred on the bar [L 0x319608 entries 5/0 + CREATE 0x319710; fix-objective-banner], colour
  0x7D6D59, layer 29; alpha = time·17 over the last 15 ticks; closes 16 px/tick. `Text_UpdateMsg`
  ticks only the displayed head [M 0x1b11d8:48-73] — queued banners wait their turn.
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

- ~~The pause-screen objectives list (`Mission_ObjectiveState` 0x182630, `Mission_NumVisObjectives`
  0x182540, drawn by `C_GCPAUSE_Handler` 0x204408) is not ported~~ → the data + flow are ported
  (objective-hints: `Mission::num_vis_objectives`/`objective_state`, the `pause_objectives`/
  `pause_objectives_sel`/`pause_hint` FrontEnd state and the ✕ flow — docs/systems/objective-hints.md);
  the DRAWING of the ✕ hint box is the main agent's.
- The fail/death 0x40000042 menu is a level restart; the red death fade is not drawn.
- `DroneFunc_SetMissionFailReason` (0x13e560) is not wired to the drone paths that call it (the punch/
  alarm reasons) — the channels 0x60/0x61 hooks are.
- A cold load of a mid-mission level leaves previous parts' objectives at their `.data` states (the
  game's saves carry them via LS_MakeMission — `MissionSave` is the port's hook for 210-fix-progress).
- Ch-0 conditions read the raw `switch_channels[0]` (the game has no zero guard [M 0x1827d8:33-37]);
  the port's `Channels::set` refuses 0, so an out-0 trigger cannot set it.

## 8. The death sequence and the fail/death restart (fix-death-restart)

The full path the player sees, ported end to end (nf-game::death + mission.rs):

1. **The death starts** — `Player_CheckForDeath` (0x190810), on health ≤ 0: ch 0x5e, `Music_Event(6,1)`,
   `Player_ClearInertia`, `Player_WeaponNone` (the fists), the death marker (`obj+0xff = 0x12`), the start
   tick (`obj+0xec`), cam mode 0, health 0, `obj+0xf4 = 2` and substate 0xd (the death anim), the death
   sound 0x89 [M 0x190810:34-97]. While `obj+0xf4` is 2/3 `Player_Update` runs only the anim +
   `Player_HandleDeath` — no input [M 0x193600:317-319].
2. **The red fade** — `Player_HandleDeath` (0x190bb0) enables the blood pane on the first dead tick
   (`HUD_DisableAll` + `HUD_Enable(pane 12, 1, 0)` [M :23-25]); `HUD_UpdateBloodPane` (0x19a6a8) grows the
   two blood sprites 1 px/tick and sets the pane's state word to 1 at full cover [M :23-34]. The gate is
   `w − viewer.w ≥ viewer.h`, viewer.h = 448 [M 0x19f4e8:33], template width 512 [L BloodSprInfo 0x2bcfd8]
   → **448 ticks (~7.5 s)**. The pane's template colours are 0x007f7fff (two sprites, layer 9, textures
   0x0300005a/0x03000002 [L 0x2bcfd8]) — the render-side fade is the main agent's check.
3. **The channel** — ch 0x62 rises when the pane covers the screen or the player presses action 25 (✕ /
   Start, player 0 — the skip), unless the menu already resolved the death [M 0x190bb0:29-33].
   `Mission_Update` reads ch 0x62 [M 0x182c60:21] — **not health**; the port's old health read fired the
   whole restart the tick the pain landed.
4. **The mission states** — case 1 → 3 (`FadeClr` 0xff0000ff — red, `Music_Event(6,1)`, health 0), case 3
   (`TimeOut` 1.0 = 1 tick — `FRAME_RATE_MUL` 1.0 [L 0x30d0d8]) → case 7 (`GS_PausePlayer(1,0)` per tick)
   → case 4 → the menu. The fail path (ch 99) is the same shape with the type-3 "Mission Fail" banner and
   `TimeOut` 300 [M 0x182c60:45-64] — the banner shows ~5 s before the menu, the death ~0 s after the fade.
5. **The 0x40000042 menu** — `MenuManager_Create(-0x80000004, 0x40000042, 0, 0, 0x28, 0, 4)` +
   `GS_PauseGame(1)` [M 0x182c60:77-81]; the handler is `P_ENDMISSION_Handler` (0x216618 —
   `Handler_HandleMessage` 0x2029e0:498-500 routes menu id 0x40000042 to it; `C_GCPAUSE_Handler`
   0x204408 is the PAUSE menu 0x10000028's handler, :217-219 — fix-death-restart had them swapped).
   The event-0x4b arms over `__Menu_Send(0x100001a4, 0x35)`'s row index [M 0x216618:11-27]:
   **row 0 (Restart) → `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` + `Player_RamLoad(Bond, 1)`**
   — the CURRENT level, `IsWarmReset = 1`, no 0x70→0x71 remap [M :19-21]; row 1 →
   `ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)` — BaseMap, cold [M :15-17]; row 2 →
   `GameState+0x28 = 0x4000001c` + `ResetMap_LevelToLoad(0x7000048, 0, 0)` — the front end
   [M :24-27]. `Mission_BaseMapHCode` = `BaseMap` — the mission's FIRST level [M 0x1830a0]; in
   multiplayer it is the current level instead [M 0x1830a0:6-9].
6. **The port's API** — `Mission.menu_open` / `menu_choice` (`MenuChoice::Restart|Quit`), the
   `Event::MenuOpen` event, and `Drones::fail_menu_open()` / `fail_choose_restart()` /
   `fail_choose_quit()` (crate::death) for 180-fix-menus to draw and answer. While nothing draws the
   menu, `Drones.auto_restart` (default true) answers Restart the tick the menu opens — the pre-menu
   behaviour, behind the flag. Quit queues `level_exit = 0x07000048`; the viewer logs it and holds (the
   front end is not ported).

**What the Restart restores (fix-restart-target)** — the fail/death menu's Restart is the WARM arm
`IsWarmReset = 1` [M 0x216618:20], and it is aimed at `GameState+0xC` — the level Bond is on — NOT
`BaseMap` (that is the PAUSE menu's Restart, `C_GCPAUSE_Handler` 0x204408:139, which the port keeps
cold-to-BaseMap; the playtest that named "the very beginning map, 05" was the fail menu). What the
reload produces [M]:

- **The channels survive** — `Init_SwitchChannels` only runs `IsWarmReset == 0` [M 0x1d5080:48-50],
  and `Player_RamLoad` restores all four channel arrays from the RamSave [M 0x1957a8:17-20].
- **The checkpoint is the ENTRY state, not the dying state.** `Player_RamSave` runs at the previous
  level's exit (`Trigger_Activate` 0x1b2318:115; script checkpoints too — `Script_EventHandler` case
  0x12, 0x1d94c0:194-199) — so the buffer's ch 99/100/0x62 predate this level's fail. That is why the
  warm restart does NOT re-fail (the replay loop fix-death-restart feared is an artefact of capturing
  at the fail, which the game never does). The port keeps the save each level was LOADED with
  (`App::entry_save`) and re-serves it on `Drones::level_exit_restart`.
- **The health floor (the boost argument, fix-death-retry-boost)** — the handler passes
  `Player_RamLoad`'s boost arg 1 [M 0x216618:21]. With it set, `Player_RamLoad` 0x1957a8:21-34
  floors the saved health — the float at **RamSave+0x5FC** (`RamSave._1532_4_`) — at the
  difficulty's `ContinueHealthBoost*`:
  - **The pick** [M 0x1957a8:21-33]: the difficulty global **`GameState+0x40`** (Ghidra DECIMAL 40 = hex +0x28 = 0x2a3790) —
    1 → `ContinueHealthBoostEasy` (0x30cc34), 2 → `ContinueHealthBoostMedium` (0x30cc38),
    anything else (0/3/4) → `ContinueHealthBoostHard` (0x30cc3c). All three are **50.0** on the
    retail build [L ee_start_05.bin 0x30cc34..3c].
  - **A max, not a set** [M :24-32]: the saved value survives when it is already above the floor;
    the result is written BACK into the buffer (`swc1 $f12, 0x5fc` at 0x1958e4) before
    `Player_SetHealth` (jal 0x1960b8 at 0x1958e8) applies it — so the floored value is what the
    reload's `Player_InitWeapon` re-save captures.
  - **Armour is NOT boosted** [M :36] — `BL+0x8B0 = RamSave._1536_4_` restores the saved armour
    as-is.
  - **Gated on the warm flag** [M :9] — the whole body (boost included) sits inside
    `IsWarmReset != 0`; a cold load never boosts.
  - **Who passes what**: the fail/death menu's Restart row 0 passes **1** [M 0x216618:21];
    `Player_InitWeapon` 0x1a7ef8 passes **0** on ALL EIGHT of its `Player_RamLoad` calls
    [M :158-305] — the trigger-exit warm load never boosts, so a level change at low health
    carries the low health and only a death-Restart floors it.
  - The port: `RamSave::continue_boost(difficulty)` + `continue_health_boost` (nf-game::player,
    the difficulty mirror `Progress::difficulty`), applied by the restart serve
    `nf_view::fail_restart_save` — the handler's `Player_RamLoad` position in the reload flow
    (fix-restart-health's hardcoded `max(50.0)` viewer constant replaced).
- **The mission layer** — `Mission_Init(level, 1)` re-derives `BaseMap`/`ThisOrderNum` from the
  level, resets the CURRENT and future parts' objective states + condition channels (previous parts
  keep theirs) [M 0x182270:68-129], and clears `LevelToEndTo`/`FailLabel`.
- **The loadout** — `Player_InitWeapon` (0x1a7ef8) re-runs the level's entry case on the warm load:
  0x07000007 RamLoads again, runs `Player_InitAmmoWeapons`, zeroes armour and equips the fixed party
  loadout [M :176-186] — the port's `party()` (nf-game::player).

A restart of a COLD-loaded level (no entry checkpoint — e.g. mission select → 05 → die): the game's
buffer holds the BSS zeros, so the restore zeroes the arrays and the health floors at 50.0; the
per-level `Player_InitWeapon` case re-equips the fresh loadout over them [S — the shapes are the
game's fixed block widths; the port zeroes at the live shapes].

**What a restart resets and keeps** — see "What the Restart restores" above (fix-restart-target
corrected this section: the fail/death menu's Restart is WARM on the entry checkpoint;
`Init_SwitchChannels` [M 0x134310, 0x1d5080:48-50] and the RamLoad gate [M 0x1957a8] belong to the
COLD arm — the PAUSE menu's Restart [M 0x204408:139] — and the mission-select cold load). The
trigger-exit level change is the other WARM arm (`ResetMap_LevelToLoad(next, 1, 0)` +
`Player_RamSave` [M 0x1b2318:111-116], fix-level2's carry). The carry is exactly what looped the
user's 07000007 session (playtest 2026-10-12) — but only because that reload carried the DYING
state's ch 99; the game's entry checkpoint predates the fail, and the fixed restart no longer loops
(the NF_ISO tests prove both arms).

Falling deaths and the chasm/death-NIS volumes reach this path through Bond's health; the death NIS
scripts themselves (`Script_IsDeathNIS` 0x1dae60, the 07000011 atrium fall) play their script and the
sim's death sequence runs underneath — the render-side check is the main agent's. **Verified
(fix-restart-target)**: the NF_ISO tests `fail_restart_reloads_the_current_level_warm` and
`restart_restores_the_entry_checkpoint_with_the_health_floor` (crates/nf-view/tests/
fix_restart_target_live.rs) drive the real fail path on 07000007 — the punched-guest ch 0x60 → ch 99
→ the menu → Restart targets `Some(0x07000007)` warm with `level_exit_restart`, and the reloaded sim
holds the entry checkpoint's state with the 50.0 floor, the fixed party loadout, armour 0, clean
channels, and no re-fail over 360 ticks. `fix_death_restart_live.rs`'s tests were re-pointed at the
corrected model (they asserted the old cold-BaseMap behaviour). The fall test
(`lethal_fall_reaches_the_death_path_and_the_restart`, a real 400-unit drop on 07000011) still lands
in the full sequence (`Player_CollisionHandler` 0x1949a8:117-126 → `Player_HandlePain(timer − 60,
type 6)` → the pane, ch 0x5e/0x62, the menu) — its restart is now the warm arm; an 8-unit drop still
triggers none of it.

### Known gaps (updated)

- ~~The pause-screen objectives list (as before) is not ported~~ → ported (objective-hints, above).
- ~~The fail/death 0x40000042 menu is a level restart; the red death fade is not drawn~~ → the menu
  logic + API are ported (§8); the menu DRAWING is 180-fix-menus'; the red fade drawing (pane 12's
  growing sprites, `death.pane_on`/`death.ticks`/`death.covered`) is render-side, not drawn yet.
- `DroneFunc_SetMissionFailReason` (as before) — the 0x60/0x61 hooks are ported.
- The death substate's animation (substate 0xd's fall) is not animated by the sim; Bond just stops
  taking input while `death.active` (the game runs `AnimObjectUpdate` [M 0x193600:317-319]).
