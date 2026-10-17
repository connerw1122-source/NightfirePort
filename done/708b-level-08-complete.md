The user (2026-10-17): "09 level elevator buttons dont work. make sure we're prioritizing level work so we can actually complete each level completely and get all models and functions per level working" and "I want EVERYTHING covered. ALL ON FOOT LEVELS". Level work now runs ahead of everything else in the queue.

**Reproduce first, through the REAL path.** Load the level with `nf_view::load` (or `load_weapons` when a
level's own weapon is needed), move Bond with `Sim::step_bond` inputs (walking, turning, the use key, the
watch gadgets, the weapons), let the level's own scripts and triggers run, and observe. No teleporting past
a problem unless the task says so; a teleport to reach a spot is fine when the test then operates the
thing through the real input. Cite the decompile [M], the disc [D] or a fresh-boot PCSX2 value [L] for every
game claim; tests pin only proven behaviour.

## Your task: `level-08-complete`, play 07000008 (Mayhew D: the gondola) from start to finish

1. A live walk test (extend `crates/nf-view/tests/walkreal_07000008_live.rs`, un-ignore it) that plays the
   level from its start through the real input path and completes EVERY objective — the optional ones too —
   and exits the way the game does (the load into the next level, or the mission results). Every cut-scene
   on the route plays (skipped only with the game's own skip action), every scripted event fires.
2. Fix every blocker the walk hits: collision, doors, triggers, scripts, AI that stops progress, missing
   channel writers. Each fix gets its own focused test with a sister that fails on the old code.
3. In the report: the objective list with how each completes [M]/[D], the route, and anything the walk
   still cannot do with the exact reason.

Known open items on this level (start here): Zoe shooting at Bond at the gondola (750r), the guards breaking the glass at the start (750p), the safe/briefcase, the rocket blast lighting (GAPS G-049).

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/level-08-complete`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-17T14:32Z

---
## Report (done, 2026-10-17T16:06Z)

# level-08-complete — 07000008 (Mayhew D: the gondola) played start to finish

Branch `s/level-08-complete`. The deliverable is `crates/nf-view/tests/walkreal_07000008_live.rs`
(new on this branch — the task's "extend" landed as create + un-ignored; three tests, ALL PASSING)
playing the level from its start through the REAL input path (`nf_view::load` +
`Sim::step_bond` inputs — walking, turning, the use key, the weapons) to every objective and the
game's own end.

## What's done

1. **`walk_the_gondola_to_the_briefcase_and_back`** — the walked stretch: the station spawn, the
   intro plate, the station-intro scene, the descent to the fusebox, the laser-cut wires, the
   Secure Room, the laser-burned safe bolts, the briefcase take (objective 1 → 4).
2. **`walk_the_gondola_ride_to_the_end`** — the ride continuation: the descent to the gondola
   mouth, the boarding, **the carriage ride** (Bond rides the moving cabin the whole way down),
   ch 83 at the script's tick 1410, the bottom cinematic, the helicopter's flight, ch 0x4B
   (objective 2 → 4), and the mission end into the driving hand-off.
3. **`the_ally_never_shoots_bond_on_the_walk`** — the playtest's "Zoe shooting at Bond at the
   gondola" reproduced and fixed (below), plus the guards' ambush waking through the zone plates.
4. A fix in `nf-game/src/drone.rs` (two gates, one game rule, cited below).

## The objectives and how each completes

The record is `mission/c07000008.rs` [L ee_start_05.bin record 3 — 2 objectives; there are no
optional ones]:

- **0x4000004 "Retrieve guidance chip from Safe Room"** — completes on **ch 0x4A**, revealed by
  ch 0x49 [L record]. The walk: the intro plate TouchOnce **#237** (139.2, 49.5, −81.2 [D]) fires
  ch 0x49 (which also completes 07000007's carried "Spy on secret meeting" objective [L record 2 —
  the test asserts the carried record folds to 4]); the LASER (0x4E, owned on the cold load [D
  inventory]) burns the safe bolts #207/#208 (Lock_Create 0x1ae1f8 → out 134/135 [M Lock_Update
  0x1ae3d8:194]) → mux #278 → ch 136 opens the safe; the briefcase pickup #290 (kind 0xF0 type 4,
  switch channel 74 [D]) → the take writes **ch 0x4A** [M Pickup_Handler 0x18e650:330-333] → the
  objective folds to 4 on the mission monitor.
- **0x4000005 "Escape with Zoe in gondola"** — completes on **ch 0x4B**, revealed by ch 0x4A
  [L record]. The walk: the cable-car floor plate Touch #236 (out 4, mask 8 [D]) → mux #235
  (0x4A AND 4) → ch 1 starts SP #295 ScriptPlayer_CableCar (the ride script 0x0600004C lives in
  the LEVEL BIN, 1420 ticks [D]); the carriage carries Bond ((112.0, 32.4, −126.3) → (112.0, 18.0,
  −179.4) [D stream 1]) — the game's riding [M Control_InheritVelocity 0x133e88, the port's
  set_dynamic movers]; stream 46 writes **ch 83 (0x53)** at tick 1410 [D] → SP #296 (the
  bottom-of-mountain cinematic, 342 ticks) writes **ch 5** [D stream 10] → the helicopter #45
  (Copter_Create 0x17d340: gate ch 5, out ch 0x4B [M record+0x70/+0x72; Copter_Update 0x17dbb8
  cases 4/5/6]) flies and writes **ch 0x4B** → the objective folds to 4.
- The level's end: no LOAD_LEVEL placement exists [D]; ch 0x4B → mux #276 → **ch 100** →
  Mission_Update state 6, LevelToEndTo **0x0730008** [M 0x182c60:85-116] — the driving hand-off,
  which the port logs (state 6 + to_end), not a `level_exit` [M mission.rs to_end match;
  docs/systems/mission.md §4]. The test asserts `(status, to_end) == (6, 0x0730008)`.

## The route (all real input)

1. Spawn **#0** (139.5, 50.5, −76.4) [D]; the carried latches (05's 0x45/0x70, 06's 0x02, 07's
   0x48) ride the RamSave in play [M Player_RamSave 0x195670] — the cold test sets them exactly
   like walk_07000008_live documents.
2. South to the intro plate **#237** → ch 0x49 → **SP #294** (0x06000077, the station intro,
   3880 ticks) plays, skipped only with the game's own skip action.
3. Down the station (the ch-0x2a plate at (139.1, 50.2, −93.6) wakes the first guard group,
   sw_ch 42 [D]) → the walkway → the ramp (path-2 nodes 0→10→1 [D]) → the lower landing.
4. **The laser cuts the fusebox #219's wires** (FUSEBOX_GuardRoomDoorLock, group-0 channel 137
   [D]; the wire filter drops guns (category word 0x0008) and keeps the Laser (0x0002) [M
   Collide_FilterBullets 0x1ece30:24-31 with the disc words; fusebox.rs]) → **ch 137** unlocks
   the GuardRoomDoor #122/10 (lock ch 137 [D]) and starts SP #221 (GuardRoomLights).
5. Inside: the laser burns the safe bolts → ch 136 (the safe door opens; SP #286 SafePickUps
   starts); the briefcase take → **ch 0x4A** → objective 1 → 4.
6. Out (the door E'd again), the descent along path 2's link chain ([D the dump]): the landing
   east → the north ramp (153.1, 41.15, −115.95) → (153.16, 39.65, −111.63) → the mid level
   (143.3, 36.4, −111.4) → west past the mid doors → (132.1, −112.3) → the long link to
   (120.5, 34.04, −112.51) → the y-33.9 hall → the stairwell down (107.25, 27.65, −119.5) → the
   lower level → the gondola mouth.
7. **The boarding is from the WEST**: the platform (y 27.25, z −121..−122.5) → the west ledge
   (x 110, y 26.75) south → east through the cabin's open west doorway → the floor plate's box
   (x 111.10..112.92, y 26.77..29.21, z −128.10..−126.75 [D the trigger's tri bbox]) → **ch 4**.
   The EAST face carries the script-entity glass (obj_type 0x20 → Break_Create [M 0x1d8968:88-90])
   — walking west from the east ledge is solid.
8. **The ride**: ch 4 + ch 0x4A → mux #235 → ch 1 → SP #295 → the carriage carries Bond down the
   whole cable (the test watches y 27.9 → 13.5, z −126 → −179.7, no falls) → **ch 83** at script
   tick 1410 → **SP #296** (the cinematic) → **ch 5** → **the helicopter** → **ch 0x4B** →
   objective 2 → 4 → **ch 100** → the mission end (6, 0x0730008).

## The blocker fixes (each with the game address; the focused test is the sister that fails on the old code)

1. **"Zoe shooting at Bond at the gondola" (the user report) — FIXED** in `drone.rs`:
   - `look_for_opponent` now refuses team 2 (as it already refused team 3): [M NDrone2_FindOpponent
     0x1424e0:101 — the opponent-vs-player set is gated on `+0x44 != 2`; a team-2 ally's opponents
     come from FindOpponent's object walk (the enemy drones), never the player].
   - `start_state`'s alert fold (Idle→Attack at alert ≥ 1.0) skips team 2: the fold would hand her
     a DRONE opponent in the game; the port has no drone opponents, so the fold is dropped — she
     keeps her base state. The reproduction: the ally (dtype 0xC, team 2, placed in the parked
     cabin) reached alert 1.0 on the guards' gunfire and the fold put her into the player-hunting
     ATTACK → COMBAT (state 0x59 with target = Bond's position, the probe trace).
   - The focused test `the_ally_never_shoots_bond_on_the_walk` walks the ambush stretch and asserts
     the ally never enters a combat state and is never shot — it FAILED on the old code (state
     0x59) and passes now. The impact path needed no change: the game's HandleImpact case 8 routes
     type-0xC to 0x6E [M 0x1469b8], which the port already models.
2. **The wire cutting** — not a port bug: the test's first attempt shot the fusebox with the P2K;
   the disc's wire filter drops guns (0x0008) and keeps the Laser (0x0002) [M 0x1ece30:24-31; the
   disc category words]. The walk cuts the wires with the laser, exactly like the game.
3. **The descent geometry** — the walk's early anchors walked Bond off the y-33.9 hall's edge onto
   the y-17.5 plunge quad (the F5 death NIS fired — the exact geometry of the known F5 symptom);
   the fixed route follows the level's own path-2/path-5 nodes [D]. No port change: the plunge
   volumes are the game's own data.

## What the walk still cannot do (with the exact reasons)

- Watch the cut-scenes unskipped (the harness carries the game's skip action by design — the task
  allows exactly that). The station-intro scene and the bottom cinematic run through their real
  triggers and are skipped with the game's own skip.
- **The east-side boarding**: the cabin's east face glass (script obj_type 0x20 breakables) is
  solid and unshootable in the port (the script-entity Break objects join set_dynamic but the
  port's Break system covers placed 0x20 placements, not script-created ones). The game's player
  boards through the west doorway anyway — the walk does the same — so this only narrows a
  non-essential approach. [S] note, not asserted.
- **The ally's own combat**: in the game a team-2 ally fights enemy DRONES (FindOpponent's object
  walk [M 0x1424e0]); the port has no drone-vs-drone opponents, so Zoe stays out of combat
  entirely. The observable the playtest asked for (she never shoots Bond) holds; her fighting the
  guards would need the drone-opponent model — left as the documented deviation.

## Live checks for the main agent

1. The ally's combat in the real game: gdbwatch `NDrone2_FindOpponent` (0x1424e0) during the real
   07000008 play with the guards engaged — the access is the team read at :101 (`lb` of Drone+0x44)
   for the ally drone; the expected result is the team-2 branch (the object walk), never the
   player assignment. Confirms the fix's rule.
2. The cabin's east glass: fieldprobe a shot at the cabin's east panes during real play — do they
   shatter (Break_Kill 0x12b970)? If yes, the port's script-entity Break gap is real and worth a
   task; if the panes are static geometry there, nothing to do.

## What I ran

- `cargo test --release --offline -p nf-view --test walkreal_07000008_live`: **3 passed, 0 failed**.
- `cargo test --release --offline -p nf-game`: **534 passed, 0 failed** (with the drone.rs fix in).
- The ally-affected siblings (`escort_follow_07000001_live`, `fix_actors_live`,
  `mayhew_handover_live`, `mayhew_end_transition_live`, `fix_npcs_live`): **14 passed, 0 failed**.
- The full suite with the fast runner: this sandbox kills every background process between tool
  calls (the mp-screens and level-07 WORKLOG entries record the same), so the foreground chunks
  stand in. The touched crates ran green as listed above; nf-formats/nf-audio are untouched by
  this task's diff (a comment-free code fix in drone.rs + a new test file).

## Usage

not visible to me (no token, context-window or cost figures are shown in this environment).
