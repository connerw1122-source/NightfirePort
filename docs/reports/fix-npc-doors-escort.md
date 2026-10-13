# fix-npc-doors-escort — the NPCs' door handling and the Mayhew escort

Branch `s/fix-npc-doors-escort`, from `s/walkreal-a` (812a0fd in its history) merged with `origin/master` three
times during the session (most recent merge at commit "merge master (pre-push 2)"). Everything below is committed
and pushed on the branch.

## What was done

### 1. NPCs can't open doors — FIXED (the real path, `drone.rs` + a binding map)

The old port had a stand-in in `Drones::tick`: a 60-tick retry that scanned for the nearest door within 2.0 in
state CLOSED/CLOSING and activated it. The game never scans: `Drone+0xBA8` is a *set* hit-object (the route's door
node binding or `NDrone2_FindElevatorDoor` 0x14a940), and `ControlSTANDARD` only re-checks it. The port now runs
the game's whole chain:

- **The node → door binding** (`NDrone_InitDoorNodes` 0x139a70): every route node with the door flag (bit 1) binds
  the nearest class-0x1C door. Computed lazily per level in `Drones::tick` into `node_doors` (the doors' positions
  are static), keyed (nav path, node id) → `Doors::list` index. (The game's scan takes its first candidate
  unconditionally, so effectively the nearest overall.)
- **The route dispatch** (`NDrone2_ReachedDestNode` 0x153358, reached via `LinkCreep_Handler` 0x15a2c8): the port's
  `Route::follow` already reported the special node a drone reached — nobody consumed it. `update_mission_route`
  and `update_patrol_route` now run `route_reached_door`: a door node with a door-node successor (the doorway's
  node pair) stands the node's door in the new `hit_door` field (+0xBA8), saves the current state in `door_return`
  (+0x5A6) and enters **`DSTATE_OpenDoor` 0x1681f8 = `NDrone2_StateFuncs[0x81]`** [L 0x29b300 + 0x81·4]; on the
  pair's far side, an open door clears +0xBA8 (the tail, 0x153358:175).
- **`DSTATE_OpenDoor` (state 0x81)**, new in the state machine: ENTER plays the door anim 0x4e for the door-prop
  drones (props 0x14/0x15/0x16 via `behaviour_util_getProperty` on Drone+0x4D8, `DroneAnim_CallAnim(0, 0x4e, …)`)
  else the stand/alert idle by alertness, requests one `NDrone2_OpenDoor` and arms timer C at now + 2 ticks
  [M 0x1681f8 case 1 + LAB_001683ac]. UPDATE returns to `door_return` once `NDrone2_DoorIsOpen` 0x14aaa0 sees
  state 4 [M case 3]. Timer C re-activates and re-arms every 2 ticks [M case 0xC]. Msg 0x20 (the door anim's
  event 6, `DroneAnim_EventFunc` 0x13b750:60-64 → `Drone_SM_SendMsg(0x20, 0, own, own)`) plays the 0x27 grunt and
  re-activates [M case 0x20] — the anim event 6 outbox is new in `event_func`, delivered as `msg::DOOR` from
  `Drones::tick` (only the 0x81 state handles it; the GLOBAL fallback drops it elsewhere, like the game).
- **`NDrone2_OpenDoor` 0x14aa38** as an outbox service in `Drones::tick`: only a CLOSED/CLOSING door (+0xF4 &
  0x12) activates; `Door_Activate(door, drone_obj, 0)`; the attempt stamps `door_open_at` (+0xBB0).
- **`ControlSTANDARD`'s door check** (0x1490f0:232-248) exact: a door in +0xBA8, strictly 60 ticks after the last
  attempt (+0xBB0 + 60 < GameState): beyond 2.0 (`Vec_Dist3D` of the obj+0x30s) the drone forgets it; else
  `DoorIsOpen` false → `NDrone2_OpenDoor`. The nearest-door scan is gone.

Shared-file hooks (small, additive): `scene.rs` none; `lib.rs` none; the `Drone` struct fields (`hit_door`,
`door_return`, `door_node`, `door_attempt`, `pending_door_msg`, `hit_door_open`) and the `Drones.node_doors` map
live in `drone.rs` itself. `player.rs`/`triggers.rs` untouched.

### 2. The Mayhew escort (07000001) — the door half is FIXED, the guard-half unchanged

`crates/nf-view/tests/fix_npc_doors_live.rs` drives the real input path (the walker, Bond's `Input` only) through
the intro to ch 0xB, clears the intro's guards, and follows the contact (skin 0x4F):

- **GREEN with the fix**: door #1 (7.92, 5.34, 11.14 — the shoji door in the user's screenshot, `--bond
  6.28,6.47,6.20,6.338,-0.083`) opens at +1283 ticks **while Bond is 3.9 away** (outside his use reach — the
  escort's own `DSTATE_OpenDoor`), the escort walks through, and he leads past door #2 (x −14.06) to x −20.0 at
  +11362 ticks.
- **Fail-first shown**: the same test run against the pre-fix `drone.rs` (checkout of 89a364f~1) FAILED — the
  escort never led past the interior doors within the cap (the route stalls at the shut door; the old stand-in's
  scan only fires on 2.0 physical contact and never resumes the blocked route).
- Sister assertions in the test: door #1 opened outside Bond's reach; no closed door within 2.5 of the escort's
  settled position.
- **Still open (needs the main agent / next session)**: past x −26 the escort parks in the game's own
  `AllyLeadHide` (0x1f) while armed guards stand within 14 [M 0x15ec08] — the test's walker clears them in rounds
  (the walkreal-a pattern), but the stairwell-head guard group (2/3 — the same group the walkreal-a session
  reported as rounds 5–11) keeps re-hiding him; the escort reaching the LoadPointActivator plate is NOT yet
  asserted. The walkreal-a report documents the same stall with the old code. This is the combat-hide mechanic,
  not the door handling; a live check below asks whether the real game's Mayhew walks to the plate under the same
  guard pressure.

### 3. The level-exit door (07000001) — RESOLVED BY INVESTIGATION (no code change needed)

The exit pair (`gallery_secretdoor` placements #224/#225, kind 2 slide, group 10, flags 0x0022 keep-open) has
**no channels at all** — live-walked from `ee_start_07000001.bin` [L]: all five 07000001 doors read openCh(+0x66)
= 0, lockCh(+0x68) = 0. Nothing opens them but the two `Door_Activate` paths (0x135be0): Bond's use key
(`Player_Activate` 0x1a5170:128) and **the escort's own door handling** — the PCSX2 trace in `docs/atlas/door.md`
already recorded "Door 2 re-opened at 44.2 s with Bond ~18 m away (a drone opened it)". The user is right: the
door is not gated on the LoadPointActivator (whose AND plate — inputs 0xE + skin 0x4F per `Trigger_Hit` 0x1b1fc8
— only controls the level load, `Trigger_Update` 0x1b1a78 → the 0xE8 wrapper's gate ch 0x10). Item 1's fix is
exactly what makes Mayhew open it on approach; the port also E-opens it today (`player_use_doors`).

### 4. The 07000002 grates — the E-usable SS props PORTED (`ss.rs` + one hook in `scene.rs`)

- **The bed grate is not a door**: 07000002 has only three 0xDB doors (placements 268/330/331, all E-openable —
  probe-verified: Bond E-opens #0, state 0x8). The vent grates are **SS props** (placement 0xE2, class 0x25): the
  port's `SsList` had the channel-driven playhead but **no `SS_Activate`** — the E path was dead.
- **Ported `SS_Activate` 0x1a9ea8** exactly: `Ss.state` (ext+0x22 = create param 0) and `Ss.mask` (ext+0x1E =
  param 2) added to `SS_Create`'s field map; `activate()`: state != 0 && mask & 4 → state = 1 and the channel
  latched ON (the playhead then runs to the last key, `SS_Update` 0x1a9948's channel drive).
- **The use-key pick** (`Player_Activate` 0x1a5170 case '%'): a hook in `scene.rs`'s use-key block — the nearest
  E-armed SS under Bond's head ray within 2.2 (the game's 1.0 sphere vs the mesh; 2.2 stands in for a hatch
  panel's extent). Shared-file hook: additive, ~20 lines, listed here.
- **`crates/nf-view/tests/fix_ss_grate_live.rs` GREEN**: ss #9 (placement 308, at (−49.8, 4.8, 6.1), mask 0x4,
  ch 2) — Bond stands 1.2 away, presses E through `step_bond`'s use edge: ch 2 latched, state 0x40 → 1, playhead
  t 0.80 (the hatch animating open). Fail-first: the pre-fix code had no SS activate at all (the assertions could
  not even compile against the old `Ss` — the interaction did not exist; the E press was a no-op).
- **What 07000002 offers the user now**: three E-usable SS props — placements 272 (−16.1, 4.3, 35.8), 308 (ch 2,
  −49.8, 4.8, 6.1), 440 (−21.6, 2.8, 33.0) — open on E.
- **NOT in this task (its own queued task)**: the vent *crawl* — `CreepWall` (placement 0x3F, class 0x2B,
  `Player_CollCreepWall` 0x196758 / `Player_Creep` 0x1969d0 substate 7, the crawl's channel write at 0x1969d0:87)
  is unported entirely (the queue's `140-fix-swim-vent` carries it). The "grates that open before you climb into
  the vent" that belong to the crawl's own flow need that system; the E-usable grates above are the standalone
  ones. `docs/WORKLOG.md:4930` already records "there is NO grate class — the grate panel is an ordinary door"
  for the door-shaped ones, and this session verified the 07000002 doors E-open.

### Shared files touched (the required list)

- `drone.rs` (task file): the state/msg constants, the Drone fields, the route dispatch, the 0x81 state, the tick
  services, and a merge-drift repair for `nf-game`'s own unit-test `Drone` literal (the new fields).
- `scene.rs` (shared, additive): the SS use-key pick hook (~20 lines, above).
- `crates/nf-game/tests/ai_live.rs`, `crates/nf-game/tests/drone_portcheck_live.rs` (shared test files, minimal):
  `Player.capsule` initializers — master's fix-gadgets (6753e51) added the field and left these test constructors
  broken **on master itself** (`error[E0063]`); without them `-p nf-game`'s tests cannot build at all. Five +
  one one-line initializers, values from `player.rs`'s standing numbers (top 1.086 above the feet).

## Verification

- `fix_npc_doors_live::the_escort_opens_the_interior_doors_on_his_route` — GREEN (and fail-first FAILED on
  pre-fix `drone.rs`, shown above).
- `fix_ss_grate_live::the_use_key_activates_an_ss_grate` — GREEN.
- Both re-run GREEN after the final master merge.
- Full suite: the run got through nf-audio (7/7 ok) and nf-formats (72 passed, only the 3 allowed `disc_golden`
  failures on the slim ISO) and was then **OOM-killed compiling nf-game's tests** — the same sandbox limit
  walkreal-a's report documented ("the background full-suite runs were OOM-killed twice in this sandbox"). The
  subsets that did run on the final merged tree, all green:
  `nf-game --lib` 92/92; `nf-game --test ai_live` 32/32 (the capsule-repaired crate); `nf-view --test
  fix_npc_doors_live` + `fix_ss_grate_live` (the two new NF_ISO tests). The driver should re-run the full
  suite on the bigger box before merging.

## Live checks for the main agent

1. **The escort's doors** (07000001): let Mayhew lead from the intro; at the first interior shoji (7.9, 5.3,
   11.1) he should stop briefly, play the door anim and the door should slide/swing without Bond's E. Watch
   `Drone+0xBA8` on Mayhew's drone ext (expect the door obj set while he works it, 0 after) and `Drone+0x5A6`
   (expect 0x1d AllyLead).
2. **The exit door**: stand Bond ~18 m from the exit pair while Mayhew approaches — confirm the real game's
   Mayhew opens it before Bond is anywhere near (matches `docs/atlas/door.md`'s "Door 2 re-opened … (a drone
   opened it)"), then confirm whether the real Mayhew walks onto the AND plate at (−33.1, 2.9, 15.4) under the
   stairwell-head guard pressure, or waits for Bond to clear them (the port's hide loop is faithful to
   0x15ec08; the plate reach is the open question).
3. **The 07000002 grates**: at the bathhouse/bunker SS props (placements 272/308/440 above), press E on each —
   each should play its open anim once and stay open; `watch switch_channels[2]` (0x26fc92) during 308's E
   should show the latch.

## What's left

- The escort-to-plate end state on 07000001 (the stairwell-head guard pressure; see live check 2).
- The vent crawl (`CreepWall`, substate 7) — queued as `140-fix-swim-vent`; the grates this task delivered are
  the E-usable SS ones plus the already-openable doors.
- `FindElevatorDoor` (0x14a940) for the sniper/elevator states is not ported (the other +0xBA8 writer; out of
  this task's symptoms).
- The SS playhead model still follows the port's channel seek (`docs/fields/ss.md` mismatches #1-3 remain); the
  E-latch works with it because `SS_Activate` latches the same channel the playhead reads.
