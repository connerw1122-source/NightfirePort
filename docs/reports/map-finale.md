# map-finale — the final missions 07000015/07000016/0700001b and the story levels 07000041/0700004a/0700004b

Branch: **s/map-finale** (from origin/master 1276eb0 — fix-mission's mission layer). Docs:
`docs/levels/07000015.md` … `0700004b.md`. Code: none new — this task needed no shared-file changes
beyond the map-powerstation deltas the branch carries byte-identical (`props.rs`, the NIS sub-script
ops, the script-less fire gate, the 0c/0d loadouts — so the eventual merges dedupe; all previously
pushed on s/map-powerstation and listed in that report). Tests: `crates/nf-view/tests/
walk_070000{15,16,1b,41,4a,4b}_live.rs` (9 tests). `docs/systems/levels.md` gained the name
corrections (the 07000014 row carried `Tower2Elevator`; the music switch
UpdateMusicalEvents 0x114e70 pins it to `EvilBase`).

## The names and roles of 0x41/0x4a/0x4b

| Code | Name (the evidence) | Role |
|---|---|---|
| 0x07000041 | no front-end entry (`sp_level` 0x2df2e0), no mission record, no music-switch case | **A scripted interlude wrapper** — the space-station approach (tether wires, a scripted-drone touch plate) |
| 0x0700004a | `UpdateMFX_HT_Level_Tower2Elevator` [M 0x114e70 case 0x700004a] | **A real playable mission level** — the Tower-2 mission's part 2 [L] between 07000012 and 07000013 |
| 0x0700004b | the loading-image switch groups it with the multiplayer codes [M 0x1d5258:369-386] | **A multiplayer arena — not a story level** (no campaign start, triggers or drones [D survey]) |

## Per-level verdicts

| Level | Completable (headless walkthrough) | The chain |
|---|---|---|
| 07000015 | **YES** | the spawn reveal plate (#484, ch 0x4b) → "Find the exit to Omega Sector"; the USE exit #445 (gate param 0, `Player_Activate` 0x1a5170 case 0x1d) loads **07000016**. The other gate (#343, gate ch 0x42) never self-fires — nothing on the disc's data writes 0x42 here ([D scans]; the keypad locks `Lock_Create` 0x1ae1f8 are the likely feed, not ported). |
| 07000016 | **Up to objective 2 + the fail path; NO for the last step** | the spawn plate (#252, ch 0x8e) completes the carried "Find the exit to Omega Sector"; the silo-pit plate (#253, ch 0x8f) completes "Reach Drake's launch silo"; the drone tripwire (#266, mask 0x10, ch 0x7b) fails the mission [M 0x1827d8:108-116] through `Mission_Update`'s fail flow to the HUD banner. **The blocker: ch 0x90 (Kiko's defeat) has no writer** in the level's triggers, scripts, sub-refs, loadable packs or any drone's death channel (scanned [D]); the AND #271 [ch 0x90] → ch 100 and the 0x7300017 hand-off are otherwise in place. |
| 0700001b | **NO (the objectives can't complete)** | "Disable the missiles" shows at once (enable 0 [L], the state-0 arm [M 0x1827d8:41-54]); the console AND (#139 [ch 21 & 22 & 27]) needs the class-0x38 `Monitor` script entities (`Script_CreateEntity` 0x1d8968 / `Monitor_Activate`) which the port does not simulate; "Defeat Drake" (ch 0x0b) has no writer in the reachable data (no drone carries death channel 11; the boss death lives in code the port does not reach). The mission end (0x0730001b [M 0x182c60:114-116]) and the ch-199 fail special are ported. |
| 07000041 | **n/a — wrapper** | The player-touch plate (#100, ch 0x6e) fires and unlocks the scripted-drone plate (#99); the five tether wires parse as 0x3E wires (the fix-wire crossing covers the ride). The wrapper's own end is driven by the driving-side flow (out of scope). |
| 0700004a | **Up to the reveal; the objective can't complete** | the shaft's 13 plates fire in descent order (the shaft descends in Y [D triggers]); the first plate's ch 0x4d reveals "Gain access to lobby"; **ch 0x4a has no reachable writer** (the real lift's lobby arrival — the lift platform is not modelled). |
| 0700004b | **n/a — multiplayer arena** | loads with no campaign start, triggers, doors or drones — asserted as the doc's sister. |

## Spots the user should check in play (with `--bond x,y,z,yaw`)

- 07000015 start → the reveal plate: `--bond 94.5,41.5,0,-1.57` — walk a step; the objective shows.
- 07000015 the exit: `--bond 0.0,41.6,-23.8,3.14` — walk into the doorway and press E; 07000016 loads.
- 07000016 arrival: `--bond 5.2,1.0,-65.3,3.09` — "Find the exit to Omega Sector" should complete as
  the level opens.
- 07000016 the silo pit: `--bond 2.5,1.2,-153.9,3.14` — the pit is a three-ledge shaft; the plate
  slab sits over the bottom floor under the mid ledge (in play the descent is the lift/stairs route).
- 07000016 the Kiko chase: her route runs from the spawn to the silo mouth — the fail tripwire
  (#266) fires if she reaches it; killing her should complete the mission once her defeat channel is
  ported (today it does not).
- 0700001b the hub: `--bond 23.0,-1.9,3.1,-1.57` — the missile-console squads hold around the
  consoles at y 9.4; the consoles' use interaction is not ported (the objectives will not complete).
- 0700004a the shaft: `--bond 3.0,-6.5,-3.4,-0.63` — the plates fire floor by floor during the ride.

## Verification

- The six walkthrough files drive `nf_view::load[_with_carry]` + `Sim::step_bond` with real input
  (walks, E presses, the drone teleports for the mask-0x10 trigger), assert every objective stage in
  order with sisters (the locked gates, the cold tripwire, the open ANDs, the hidden objectives).
- Full suite: `NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1` — green except the 3 allowed `disc_golden` failures
  (85 binaries ok), including the pre-existing wire_live/portal_live/walk_0700000c/0d suites.
- The Kiko/Drake channel hunt: the level triggers, the level scripts + their sub-refs, the loadable
  packs (07F/078/079), every drone's death-channel param 12, and the KikoMission state machine
  (0x15df28/0x15e0d8 — her route completes into `Drone_SM_SetState(0x89)`) — no writer for ch 0x90,
  ch 0x0b or ch 199 exists in the data the port loads; documented per level with the game functions.

## What's left (with the game function behind each)

1. **07000016: Kiko's defeat (ch 0x90)** — the KikoMission state family (0x15df28/0x15e0d8, the
   state-0x89 handler via `Drone_SM_SetState`) and/or a death channel the disc does not carry on any
   placement. Blocks the final mission's end.
2. **0700001b: the missile consoles (ch 0x09's inputs)** — the class-0x38 `Monitor` script entities
   (`Script_CreateEntity` 0x1d8968, `Monitor_Activate` from `Player_Activate` 0x1a5170 case 0x38).
3. **0700001b: Drake's defeat (ch 0x0b) and the ch-199 critical countdown** — the boss death lives
   in code the port does not reach; `DroneFunc_SetDeathChannel` 0x13e660 reads drone +0x13c
   (placement param 12) and no 0700001b placement carries 11.
4. **0700004a: ch 0x4a** — the lift platform's lobby arrival (the lift is a moving platform the port
   does not model).
5. **07000015: the keypad locks** — `Lock_Create` 0x1ae1f8 / `Lock_Activate` 0x1aed00 (the keypads
   draw; the unlock's channel write is not ported).
6. **07000041: the scripted-drone toucher** — the script-entity drones are visuals only.
