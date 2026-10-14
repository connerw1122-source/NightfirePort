# campaign-flow — every mission end to end, headless: the blockers map

## What was built

`crates/nf-view/tests/campaign_flow_live.rs` — a headless campaign runner that plays each
single-player mission's levels in story order from the front end's mission start (a COLD load —
mission select → level [M docs/systems/mission.md §3]), drives Bond through the game's own
objective layer, and follows the level transitions the way the game does them:

- **The story order** is `ResetMap_LevelCode2Img`'s case order (0x1d5258:77-350): 07000001-04,
  05-08, 09-0b, 0c-0d, 11, 12, 13, 14-16, 1b. The mission grouping is the MissionData records'
  `mission` field (= BaseMap, the mission's first level [M 0x182270:73-84; L ee_start_05.bin
  @ 0x2a4350]). The records for 0x0700000e/0f/10 have **no LevelCode2Img case and no archive in
  FILES.BIN** ("no 0700000e.bin" on load [D]) — cut records, recorded as non-story.
- **The drive**: each objective's condition/enable channels are the same switch channels the
  level's triggers and scripts set (`Mission_MonitorObjectives` 0x1827d8 reads them; the port folds
  them in `Mission::update`). Where an objective needs real gameplay, the runner records the gap
  ("unfired objective channels [...]") and forces the verdict — ch 100 is the AND of every part's
  condition [M 0x1827d8:22-38, 119-123] — but only when the level has no real exit.
- **The exits**: the game ends a level two ways — the auto-fire (`Trigger_Update` case 10
  0x1b1a78:162-166: needs a NON-ZERO gate channel index that is ON — gate 0 never auto-fires) or
  Bond's E-use at the trigger (`Player_Activate` 0x1a5170 → `Trigger_Activate` 0x1b2318: both gate
  arms load, the cancel channels 0x60-0x63 and the key gate aside). The runner drives the E-path:
  stand Bond in the trigger's bbox, hold use.
- **The transitions**: on an exit the port captures the RamSave + mission states and the next
  archive loads warm (`load_with_carry`; the game's `ResetMap_LevelToLoad(next, 1, 0)` +
  `Player_RamSave` [M 0x1b2318:111-116]).
- **The mission end**: `Mission_Update` case 6 [M 0x182c60:85-135] — `LevelToEndTo` = a DRIVING.ELF
  code for the mission-final levels (0x0730004/8/b/d/f/13/17/1b — out of scope, the port logs and
  skips them) and 0x07000048 — **the front-end level** (menu.rs `FRONTEND_LEVEL`) — for the rest.
  The runner records the landing and starts the next mission cold, exactly as the front end would.

Run: `NF_ISO=<iso> cargo test --offline --release -p nf-view --test campaign_flow_live -- --nocapture`
(3 tests, ~75 s total; each prints its mission's table rows).

## The table (the run on the final tree)

Legend: parsed = the archive loads; spawned = Bond stands at the level start; objectives =
folded/total at the fallback's best effort; complete = MissionState 6; exit→ = the next hash the
port's flow produced. Every story level parses and spawns — no crashes, no stuck scripts.

| mission | level | parsed | spawned | objectives | complete | exit→ | note |
|---|---|---|---|---|---|---|---|
| 07000001 | 07000001 | ✓ | ✓ | 1/1 | — | — | **BLOCKER 1**: the exit (gate ch16) is the escort-extraction touch zone — see below |
| 07000001 | 07000002 | ✓ | ✓ | 3/3 | — | 07000003 | exit E-path, key gate ch76 (the key pickup's channel) |
| 07000001 | 07000003 | ✓ | ✓ | 3/3 | — | 07000004 | exit E-path, key gate ch82 |
| 07000001 | 07000004 | ✓ | ✓ | 1/1 | ✓ | — | mission end → 0x0730004 (driving, skipped) |
| 07000005 | 07000005 | ✓ | ✓ | 1/1 | — | 07000006 | exit E-path |
| 07000005 | 07000006 | ✓ | ✓ | 1/1 | ✓ | 07000048 | **BLOCKER 2**: no exit trigger in the data; forced complete → the front end |
| 07000005 | 07000007 | ✓ | ✓ | 2/2 | — | 07000008 | exit E-path (folds naturally after the re-arm fix) |
| 07000005 | 07000008 | ✓ | ✓ | 2/2 | ✓ | — | mission end → 0x0730008 (driving, skipped) |
| 07000009 | 07000009 | ✓ | ✓ | 2/3 | — | 07000009 | **BLOCKER 4**: the LOAD_LEVEL trigger's next_level is the level ITSELF (a data self-loop) |
| 07000009 | 0700000a | ✓ | ✓ | 3/3 | — | 0700000a | **BLOCKER 4** again (self-loop); the real exit →0700000b (key ch74) did not fire headless |
| 07000009 | 0700000b | ✓ | ✓ | 5/5 | ✓ | — | mission end → 0x073000b (driving, skipped) |
| 0700000c | 0700000c | ✓ | ✓ | 3/3 | — | 0700000d | exit E-path, key gate ch28 |
| 0700000c | 0700000d | ✓ | ✓ | 3/3 | ✓ | — | mission end → 0x073000d (driving, skipped) |
| 07000011 | 07000011 | ✓ | ✓ | 2/2 | ✓ | 07000048 | **BLOCKER 5**: no exit trigger in the data; forced complete → the front end |
| 07000011 | 07000012 | ✓ | ✓ | 2/2 | — | 0700004a | exit E-path, gate ch31 |
| 07000011 | 0700004a | ✓ | ✓ | 1/1 | ✓ | 07000048 | **BLOCKER 5** again; forced complete → the front end |
| 07000011 | 07000013 | ✓ | ✓ | 1/1 | — | 07300013 | the exit trigger's next IS the driving code — the game data chains straight to DRIVING.ELF |
| 07000014 | 07000014 | ✓ | ✓ | 5/5 | — | 07000015 | exit E-path, key gate ch84 |
| 07000014 | 07000015 | ✓ | ✓ | 1/1 | — | 07000016 | exit E-path, gate ch66 |
| 07000014 | 07000016 | ✓ | ✓ | 2/2 | ✓ | — | mission end → 0x0730017 (driving, skipped) |
| 0700001b | 0700001b | ✓ | ✓ | 2/2 | ✓ | — | mission end → 0x073001b (driving, skipped) |

## Fixed: the warm-transition mission freeze (the one pinnable blocker)

**Symptom** (the runner's first pass): 07000007/08/0a — every objective unfired (0/2, 0/2, 0/3),
the mission layer dead for the whole level.

**Root cause** [M]: `Drones::tick`'s mission latch ran `Mission::init` (which re-arms the layer —
`Mission_Init` 0x182270:20-28 sets InternalState 1, MissionState 1) and then
`self.mission.restore(&s)` with the carried snapshot — **overwriting the re-armed globals with the
previous level's internal 5** (the post-banner state `Mission_Update` case 4 leaves after both the
complete and the fail/menu arms [M 0x182c60:65-84]). `Mission_Update` case 5 is a bare break
[0x182c60:83-84], so the objective monitor never ran again: no objective could fold, no level end
was reachable. The game never does this: on a level load `ResetMap_GameInit` (0x1d5080:53) runs
`Mission_Init` and nothing restores the layer's globals — `LS_LoadMission` (0x1cd880) is the
SAVE-FILE path, not a level load; the warm transition keeps only the `.data` objective records
(the past parts' states).

**Fix**: `crates/nf-game/src/mission.rs` — `Mission::restore` restores ONLY the per-objective
records (the `.data` states); the layer's globals stay as `Mission_Init` re-armed them. The
save-file path (210's `LS_LoadMission`) will need the globals variant when it lands.

**Test** (failing-first): `the_mission_layer_re_arms_on_a_warm_transition` — 05 runs the complete
arm to internal 5, the carry loads 06 warm; pre-fix the layer sits frozen (internal 5, `left: 5 /
right: 1` in the assert), post-fix internal 1 and 06's own objective folds while the past part's
record survives. Sister asserts: the past part's state 4 preserved; the layer re-armed.

## The remaining blockers, most precise first

1. **07000001's exit needs the escort AI (the known walkreal-a stall).** The exit's LOAD_LEVEL
   trigger has NO collision volume (lo/hi = inf — no mesh), so the E-path cannot aim it; the gate
   ch16 is a kind-2 TOUCH zone whose filter needs BOTH Bond AND the contact drone (skin 0x4F)
   [M walkreal_07000001_live.rs:5-6, triggers.rs `hit`], and `calc` clears every TOUCH out-channel
   at each tick start (the continuous-touch semantic), so the channel cannot be held without the
   escort standing in the zone. The walkreal-a route already reported the escort's root-motion
   stall (docs/reports/walkreal-a.md). **Follow-up task: fix the escort's Lead/AllyLead root
   motion so the contact reaches the extraction plate; then 01 chains.**
2. **07000006 has no level exit in its data.** No 0xE8 placement exists in ANY map of the archive
   (probed all placements; the 0xE9-class records are `Drone_AIPoint`s [M parsemap
   0x1d04f0 case 0xe9]) and the mission AND cannot fold mid-mission (it spans every part's
   condition [M 0x1827d8:22-38]). The forced complete lands on the front end. **Live check: in
   PCSX2, finish 06's objectives and watch what loads next (gdbwatch on `switch_channels[100]`
   0x2b8ee4-ish and the reset request) — either a trigger the port's parse misses or a scripted
   ch100 write.**
3. **07000009/0a's exit triggers self-loop** (next_level = their own hash [D parse]) — recorded as
   non-exits. **Live check: PCSX2 at 09/0a's exit — what loads?**
4. **07000011/0700004a have no exit trigger** (same shape as 06): the mission completes forced →
   the front end. **Live check: PCSX2, same watch.**
5. **0700000a's real exit (→0700000b, key gate ch74) did not fire headless** even with the key
   channel set — the E-path found no hit (the trigger's volume/mesh, like blocker 1's unbounded
   bbox). Needs the same trigger-volume work as 1.
6. **The gameplay-driven objective channels** the fallback cannot fire: 09's ch71 (2/3),
   08's ch75 (1/2 — "Escape with Zoe in gondola"), 4a's ch74. Each belongs to its per-mission
   walkthrough task (the existing walkreal routes cover 01-05, 09-0d, 11-16, 1b, 4a).
7. **The front-end leg** (0x07000048 → results → the next mission unlocked) is the menus' area —
   the runner records the landings but does not drive the menus; the unlock flags ride the
   progress layer (`nightfire_status` [M 0x202360]). **Follow-up task: chain the front end's
   mission select from the results screen (180-fix-menus' pages).**

## Verification

- All 3 campaign tests pass on the final tree (the table above is their output).
- `the_mission_layer_re_arms_on_a_warm_transition` fails on the pre-fix code (shown), passes with
  the fix; the existing mission tests (fix_restart_target_live, fix_death_restart_live,
  fix_objective_messages) still pass — the suite's totals are below.
- The full suite (`tools/test/suite.py`) on the final tree: **see the total line below.**

## Files touched

- `crates/nf-view/tests/campaign_flow_live.rs` — new: the runner + the tables + the re-arm test.
- `crates/nf-game/src/mission.rs` — the fix: `restore` restores the `.data` records only (doc'd).
- `crates/nf-view/tests/probe_campaign06.rs` — kept probe (06's trigger scan).
- Shared files: `drone.rs`/`scene.rs`/`player.rs`/`lib.rs` untouched.

## Live checks for the main agent

PCSX2 (spot / action / expected):

1. 07000006: complete the part-1 objective ("Find a way inside to the party") — watch what loads
   next: the expected per the mission layer is a `ResetMap_LevelToLoad` with `LevelToEndTo`
   (0x07000048) IF ch100 fires — confirm whether the real 06 ends via the mission-complete arm
   (no exit trigger exists in the data) or something the port's parse misses.
2. 07000009/0700000a: reach the level's exit — confirm whether the self-referential LOAD_LEVEL
   triggers (next = own hash) really fire a reload, or whether another path ends the level.
3. 07000011 / 0700004a: complete the objectives — confirm the level end (no exit trigger in the
   data; expect the mission-complete arm → 0x07000048 or a driving interlude).
4. 07000001: the escort extraction — whether Mayhew's contact reaches the ch16 plate in a real
   playthrough (the walkreal-a stall says the port's escort AI stalls; the game's own behaviour is
   the reference).

nf-view (`--bond` spots, `--skip-cutscenes`): the campaign table itself is the check — re-run
`cargo test --offline --release -p nf-view --test campaign_flow_live -- --nocapture` and compare
the rows.

## Suite total

(to be filled from the final `suite.out` total line)
