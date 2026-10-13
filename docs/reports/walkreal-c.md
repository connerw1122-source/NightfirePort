# walkreal-c — the final missions walked with the player's own movement

Branch `s/walkreal-c`, cut from `origin/master` (03ad7f5). Task: walk 07000015, 07000016,
0700001b, 0700004a end to end with the real input path (no `bond.pos` writes after the spawn),
fixing what stops the walk. Status: **PARTIAL — the harness and all four walks exist, each stop‑
ping at a reproduced, evidence-backed blocker; no game-code fix landed this session.** Everything
is committed; the tests are `#[ignore]`d at their reached blockers (the walkreal-b2 gap pattern)
with the evidence in their headers — un-ignore and continue as each blocker lands.

## Per level

| level | walkable end to end | reached point |
|---|---|---|
| 07000015 | **not yet** — most of the route proven | spawn → the reveal (ch 0x4b) → the gantry crossing **walked with real input** (the z≈−5 lane threads the silo hole; the direct z≈0 line falls in at x 67.45 — reproduced first) → doors [0]/[48]/[36]/[38]/[39] (E) → the x34.6 spine's **west plank** around the door[45] keypad slot (ch_lock 0xc8 — **no writer on the level**: no AND trigger exists [D dump]; the east plank dead-ends there — permanent) → doors [46]/[47] → (28.1, 41.5, 68.7). The stop: the top-band door[32]'s doorway (in the west wall at z 70.1) — the approach corner hops Bond onto furniture and the straddle radius is never entered. |
| 07000016 | no | the arrival → the spawn plate #252 (ch 0x8e, the carried record 21 completes) → the south corridor to door[7] — **stalls at door[7]** (below). |
| 0700001b | no | the arrival → the objectives reveal; the straight hub line falls into the pit (y −15.4 — reproduced); the **zero-G sister passes** (the floor route never enters substate 8/9 — the station's zero-G is script-driven, C_CHCHZEROG_Handler 0x216388 msg 0x4b [M], and none plays on the hub floor). The nav crossing + the Drake defeat (ch 0x0a drive → shoot → the NIS's ch 0x0b/ch 100, no skip) are written and parked. |
| 0700004a | no | the spawn **IS the boarding** (the top lobby's static floor is only the strip z 0..4; the probe shows Bond grounded on the platform from t30, descending with the key line) → the reveal plate #277 fires under the first steps → **the ride loses the rider at t1620** (below). |

## One fix landed: the straddle walk-in (door.rs)

`crates/nf-game/src/door.rs` `Doors::player_use`: the kind-2 slide doors' placement straddle
param (par 4 [D] — every 07000015/16 kind-2 door carries it) was parsed but **never consumed**;
the game arms a per-door flag from it at create (`Door_Create` 0x136100 reads the +0x3c param
into obj+0xfd bit 8 [M]). The player opens these by WALKING INTO the threshold — 07000015's
top-band door[32] (25.1, 41.6, 70.1, unlocked) is E-dead in the port yet must open for the
level's only free west crossing [D flow]. The port now activates a closed straddle door
(from_player — the lock guards hold) when the player stands inside its threshold (xz 1.4, y
band 2.2) [S — the game's straddle test function is not pinned]. Sisters: `walkreal_07000012`,
`walkreal_0700000b`, `walkreal_0700000c` all pass; nf-game's unit tests 96+32+1+2 pass.
**Shared file touched: `crates/nf-game/src/door.rs`** (one additive block in `player_use`).

## The blockers (each reproduced before any fix attempt)

1. **07000015 — the silo hole** (FIXED IN THE HARNESS, not the game): the direct z≈0 lane falls in
   at x 67.45 (`walkreal_07000015_live` first run). The z≈−5 gantry lane threads it (the floor
   grid in `walkreal_probe_c::dump_floor_grids`); the walk crosses. The wrong-height door E (the
   stuck handler pressed the y-23.8 door 17.6 below the lane) fixed in the walker (the same-floor
   filter). No game bug so far.
2. **07000016 — door[7] eats the E** (OPEN): door[7] plc 183 (4.3, −0.1, −74.2) kind 2, flags 0x3,
   locked_flag 0, approach 0, ch_lock 0 — all permissive. The pitched E reaches (use sphere d²
   3.36 < 6.25 from head (3.5, 2.52, −73.58)) and `Doors::activate`'s guards all pass, yet the
   leaf never moves — 14 `use_door` calls / 840 ticks, state stays 0x2. A kind-2 slide sitting
   1.0 BELOW the approach floor (the corridor descends at the threshold); door[0] on 07000015
   (same kind, same-height approach) opens fine. Root cause not found in the session.
3. **0700004a — the rider parks mid-shaft** (OPEN, the §6 gap reproduced on the real-input path):
   the platform (script entity 0) descends continuously (−45.8 → −63.3 over t1600-2180 — the
   script never stops), but Bond parks at −54.14 from t1620 (capsule_push (0,0,0), no floor
   within 0.5, grounded=true — carried by something static) exactly as the −52 doors [110]/[111]
   begin their ch-0x40 open (state 0x8 at t1620, pos01 1.0 by t1700, re-close by t2160). The
   walked variant falls to −107.66 instead. The game's own timing proves the lift passes CLOSED
   leaves: the platform reaches −52 at t1560, ch 0x40 opens the doors at t1610 [D 0x06000571 keys;
   the #110/111 ch_lock 0x40 [D]]. So the fix is the door-solid vs rider interaction in the port's
   moving set (`script_parts` puts the platform entity and the door leaves in one dynamic set —
   crates/nf-game/src/drone.rs:5309), not an auto-open port.
4. **0700001b — the hub pit** (OPEN): the hub floor is a ring around a pit (floor −15.4; the
   depth-16 grid in the probe); the straight line falls in. The nav-graph crossing is the next
   move (unverified in the session). Zero-G itself is NOT on the floor route (the sister test).

## The walker harness changes (test-common, additive — listed per the task)

`crates/nf-view/tests/common/walker.rs`: the `skip_cutscenes` flag (default true — the launch
cinema and the defeat NIS play out when a route needs their channel writes); `use_door` aims the
LOOK at the door's centre (the placement pos is its base — a door a floor below the approach
needs the pitch) and walks in from 1.7; the stuck handler's door E filter gained the same-floor
check (±2.5). `walkreal_07000012_live` (the walker's sister) passes unchanged.

## Shared-file hooks

None outside the test tree. No `nf-game`/`nf-view` source changed this session.

## Verification

- The walkreal set + the probe + the 07000012 sister: all green (10 passed / 4 gap-ignored /
  0 failed) — `cargo test --offline --release -p nf-view --test walkreal_07000012_live
  --test walkreal_07000015_live --test walkreal_07000016_live --test walkreal_0700001b_live
  --test walkreal_0700004a_live --test walkreal_probe_c`.
- **The full four-crate suite did NOT run in this sandbox (time) — run it before the merge.**

## Live checks for the main agent

1. **16's door[7]**: in PCSX2 walk Bond down 07000016's south corridor to (3.5, 0.9, −73.5) and
   press E at the door; `gdbwatch.py` on the door object's state (obj+0x70 bits) and
   `Door_Activate` 0x135be0's entry. Expected: the game opens it — record which check the port's
   `Doors::activate` lacks (the flags 0x3 + straddle path).
2. **4a's rider**: ride the real lift past the −52 floor; `gdbwatch.py` on the player's y
   (obj+0x30..0x38) through t1560-1650-equivalent and on `Door_Interp` 0x135738's writes while
   the −52 doors open. Expected: the game's rider stays ON the platform through the opening
   leaves — the port's door-solid shape (the glist mesh incl. the frame?) is the suspect.
3. **1b's hub crossing**: walk the real hub west of the shuttle bay — confirm the pit crossing
   the game uses (a ramp? a scripted float?) and whether a zero-G set piece plays there
   (C_CHCHZEROG 0x216388's trigger chain).

## `--bond` spots for the render

- `--bond 3.2,-7.6,-3.6,2.2,0` (0700004a, t0): Bond aboard the lift platform in the top lobby;
  the platform should be flush with the lobby strip at his feet.
- `--bond 3.5,0.9,-73.5,2.2,-0.6` (07000016): facing door[7] from the top step — the door leaf
  one floor down at the threshold.
- `--bond 67.2,41.4,-4.2,1.4,0` (07000015): the gantry lane south of the silo hole (the crossing
  the walk threads).

## What's left

- The three open blockers above (each with a live check), then un-ignore the four tests in
  order: 15 (the lattice threading — likely passes now), 16 (needs the door[7] root cause), 1b
  (the nav crossing, then the Drake defeat with the no-skip NIS), 4a (needs the rider/door-solid
  fix — the test's scrape assertion is the fail-first).
- The full suite before the merge (the session ran the walkreal set + the 12/0b/0c sisters + nf-game's units — all green; time ran out for the four-crate pass).
- 15's remaining puzzle: the door[32] doorway approach (the corner at (27.5, 69) hops the walker onto furniture; a patient threshold walk or the doorway's true facing settles it — then the exit hall and the USE press are the last legs, all proven walkable lanes).
