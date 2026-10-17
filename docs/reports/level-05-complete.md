# level-05-complete — 07000005 (Mayhew A, the castle approach) played start to finish

Branch `s/level-05-complete`. The walk test is `crates/nf-view/tests/walkreal_07000005_live.rs`
(`walk_to_the_gatehouse_and_ride_the_truck`, un-ignored and green).

## The objective list, and how each completes

The level has **one** objective record of its own [L ee_start_05.bin @0x2a4350 entry 0: level
0x07000005, order 0, record @0x2a3e28, count 1; @0x2a3e28: text 0x04000000, hint 0x0400005b,
condition ch **0x46**, init 0, enable ch **0x45**; M Mission_Init 0x182270:106-118, Mission_MonitorObjectives
0x1827d8]. The Mayhew mission (BaseMap 0x07000005) spans four entries — 05/06/07/08 [L the table
@0x2a4350..0x2a4420] — the port carries them as mission/c0700000{5,6,7,8}.rs.

- **ch 0x45 (the reveal)**: placement **36**, TouchOnce out 0x45, mask 8, a flat plate
  (104.51, 24.41, 106.78)-(107.37, 24.41, 109.49) — UNDER the cold start #26 (104.95, 25.12, 108.83)
  [D placements dump]. The level page's old claim "ch 0x45's writer is not on-disc" is **wrong** —
  the earlier census missed it. The real game fires it on the boot tick: the fresh-boot snapshot
  carries switch_channels[0x45] = 1 with the record still at state 1 (the monitor's first tick has
  not run) [L ee_start_05.bin @0x26fc90 = {0x45}, @0x2a3e28+0x14 = 1]. The port now fires it on
  frame 1 too (the toucher fix below).
- **ch 0x46 ("Breach the castle walls" completes)**: **no writer on 07000005** — the full census
  (all 47 main-map trigger placements, all 6 SP placements, every kind-7 script of the archive —
  only ONE channel event exists on the level: the EndLevel script 0x06000A23's event 0x11
  [0x62,0x62,0x1D,1] → ch 0x1D [D nis dump], no drone death out 0x46, no 0xE2, no switch) [D].
  **The writer is on 07000006**: placement **498**, MultiplexOrIn out 0x46, list [0x50, 0x51, 0x52]
  [D 07000006 trigger dump]; its inputs are the three arrival routes' TouchOnce plates — 0x50 (the
  zip-wire landing, #495/#766 at (35.3, 49.6, −69.5)), 0x51 (the truck-ride spawn, #497/#743-745 at
  (55.3-58.8, 39.2-39.9, 21.2-23.9)), 0x52 (the gate walk, #767-769 at (27.6-29.9, 37, 45.2-47.4)).
  The live ride-time snapshot agrees the objective is still open after the truck ride:
  record state 3 with switch_channels = {0x45, 0x73, 0x97} [L ee_snapshot.bin @0x2a3e28+0x14 = 3,
  @0x26fc90 = {45, 73, 97}]. So the game completes "Breach the castle walls" at the 07000006
  arrival plate, not during the ride.
- **ch 0x02 (part 1, "Find a way inside to the party")**: reveals on ch 0x46 (enable byte 0x46
  [L ee_start_05 @0x2a3e40+0x0f]); its own completion belongs to 07000006's walk (level-06-complete).

## The route the walk drives (all through Sim::step_bond inputs)

1. Cold start #26 [D placements kind 0x24] — the port spawns Bond at (104.95, 24.98, 108.83), the
   game's boot BLData (104.954, **24.9633**, 108.825) [L ee_start_05.bin glb_players→+0xc0..+0xc8].
2. The reveal plate fires on the boot tick → ch 0x45 → the monitor announces the objective (state 3).
3. Walk the road to the gatehouse walkway's west edge (98.6, 26.0, 106.5) — the ch-0x5C plate
   window; SP #38's hand-over (script 0x060008C8) re-enables Bond on the truck bed.
4. The crouched ride [D fix-truck's trace] — the bed touch (ch 0x71, script trigger) holds; the
   truck parks on the ch-0x70 plate (29.85, 37.74, 50.30).
5. Mux #316 = AND(0x70, 0x71, 0x72) → ch 0x65; SP #320 "EndLevel" (0x06000A23) fires event 0x11 →
   **ch 0x1D** at its tick 0x12c [D nis dump]; LOAD_LEVEL #118 (gate 0x1D) auto-fires → level_exit
   0x07000006. Walk total: 5687 ticks, no falls.
6. The game's own hand-over: `RamSave::capture` at the exit → the warm load of 07000006 → the 0x1D
   start #496 (57.18, 39.73, 22.61) [level_carry_live pins it].
7. The truck-ride spawn sits INSIDE the 0x51 arrival plate cluster → tick 1 raises 0x51 → OR #498 →
   **ch 0x46** → the monitor completes the carried record (**state 4**) and reveals part 1
   (**state 3**) [M 0x1827d8 cases 1/3].

## The fix (each with its sister)

- **The player trigger toucher's reach** (`crates/nf-game/src/drone.rs`, Drones::calc): the
  stand-in built the capsule at `pos − 0.4`; the measured live bounds are reach ≥ **0.557** below
  the position reference (the boot plate [L ee_start_05]) and < **0.575** on the game's geometry
  (the standing ride must not touch the crouch-only ch-0x70 park plate [D fix-truck's trace]); the
  port's own geometry gives the standing-ride miss boundary at 0.65 (probe), so the toucher now
  builds at `pos − 0.6`. Sisters that stayed green: the standing-ride sister
  (`the_handover_never_fires_for_bond_who_stays_at_the_spawn`), fix_truck_live, level_carry_live
  (8/8), walkreal_07000001/02/04_live, the whole nf-game unit suite.

## What the walk still cannot do, with the exact reason

- **The cable route** (`walk_the_west_ledge_and_cross_the_cable`, still `#[ignore]`d): the real
  path reaches the ladder #151 top (61.64, 34.62, 89.20) and the reveal/assert stages pass, but the
  ledge west of the ladder dead-ends: a 16-yaw sweep from the top stalls at x ≈ 57.9-58.0 (Bond
  holds y ≈ 34.7 with grounded flickering; the yaw-5.89 walk drops off the ledge's south edge to
  y 28). The wire #251 grab (−23.5, 34.5, 6.3) and the zip door #112 (fallback 0x1E) are therefore
  unreachable through this lane. Whether the x≈58 wall is the game's own collision (the level's
  113 kind-0x19 placements include several `BlockingPoly_*`), the port's collision of the tower
  objects, or simply the wrong lane, is unresolved — see the live checks.
- **The stray ch-0x4B raise (pre-existing, not this task's regression)**: the helicopter SC3 script
  chain (0x06000047, spawned at boot near the spawn; the `SWITCH_Helicopterstart(SC3)` object) ends
  with the port raising ch 0x4B on tick ~299 while Bond idles at the spawn. The live ride-time
  snapshot has 0x4B **off** well past that point [L ee_snapshot = {45, 73, 97}]. ch 0x4B is the
  condition channel of the "Escape with Zoe in gondola" record (mission/c07000008.rs) and of the
  07000002 geisha record — a false raise here can pre-complete those objectives on later levels.
  Verified pre-existing (reproduces with the toucher change stashed). The helicopters belong to
  task 750h2; flagged here because the walk crosses the script.

## Live checks for the main agent

1. **The ledge wall at x ≈ 58** (the cable route): render Bond at `--bond 60.5,34.6,88.5,3.5,0`
   (the ladder top, facing west) — does the real game show a wall/rail blocking the ledge west of
   the ladder top? Then `--bond 57.0,34.6,87.0,3.1,0` — is the real game's Bond able to stand
   there (i.e. is the port's wall real)? A gdbwatch trace on Bond's x while walking west from the
   ladder top (writes/reads of the player position during the walk) would settle whether the real
   route turns (south, along z) before the tower.
2. **ch 0x4B**: a fresh boot of 07000005, wait ~10 s idle, then read switch_channels[0x4B]
   (0x26fc90+0x4B): the live snapshot predicts **0** [L ee_snapshot]. If the real game raises it,
   the port's helicopter chain is right and the snapshot was taken later; if it stays 0, the port's
   0x06000047 chain ends with a wrong channel write (fieldprobe on the script's out).
3. **The truck-ride spawn's arrival plate**: fresh-boot 07000006 via the truck route, confirm
   "OBJECTIVE COMPLETE — Breach the castle walls" appears within a second of the load (the walk's
   state-4 assertion predicts yes; the banner text label 0x2000053).

## What ran

- `cargo test --offline --release -p nf-view --test walkreal_07000005_live` — truck route green
  (the cable test stays ignored), plus `mayhew_handover_live` (2/2), `level_carry_live` (8/8),
  `walkreal_07000001_live` + `walkreal_07000002_live` + `walkreal_07000004_live` + `fix_truck_live`
  (all green), and the whole `nf-game` unit suite (no failures).
- The full four-crate suite (`tools/test/suite.py`) was started on the final tree in the
  background; it does not finish inside this sandbox's window (the earlier baseline run's only
  failures were the two pre-existing `fix_0c_pylon_live` render failures and per-binary timeout
  kills — none touched the trigger/mission paths).

## Usage

not visible to me (no token, cost or context-window figures are exposed in this environment; the
sandbox clock at report time read 13:1x UTC via `date -u`).
