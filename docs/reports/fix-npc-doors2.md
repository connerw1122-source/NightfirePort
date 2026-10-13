# fix-npc-doors2 — finishing the NPC door work so it merges green

Branch `s/fix-npc-doors2`, from `s/fix-npc-doors-escort` (754c7d9) merged with `origin/master` twice
(03ed005 at the start, 323bf00 mid-session — weapons-all, level-0c). Final merged tree 3ec4fd5.

## What the two red tests actually were

### 1. `fix_mission_live::the_07000001_exit_works_end_to_end` — the port is right, the test's E-mash deadlocks the door

Reproduced first (the held merge's log shape): Mayhew enters state 0x81 at the courtyard shoji
(placement 14, kind 2 slide, (6.99, 6.60, 11.13)) and stays there 4,800 ticks, oscillating z
10.1↔12.1. The door log (`NF_DOORLOG=1`) showed the shoji's state **ping-ponging 0x8↔0x10 with pos01
frozen at 0.033** — the door never reached state 4, and `DSTATE_OpenDoor` has no give-up.

The game's answer, from the decompile:

- `Door_Activate` (0x135be0) **reverses an OPENING door** when re-activated from outside the swing
  zone: the state-4/8 arm sets dir toward 0.5 and `SetState(0x10)`. A press on a CLOSING door
  re-opens it (the state-2/0x10 arm swings away from the activator).
- EVERY successful activation sets the tick-busy marker (`info+0x70 |= 0x10`, LAB_00135ef8), and
  `Door_Update` (0x1363c8:25) returns early while it stands — **the door's motion skips that tick**.
  `Doors_Calc` (0x135fe0) clears the marker at the end of every tick, so the skip is per-tick.
- `DSTATE_OpenDoor` (0x1681f8) exits ONLY on `NDrone2_DoorIsOpen` (0x14aaa0, door+0xF4 == 4); its
  timer C re-activates every 2 ticks (case 0xC), and `ControlSTANDARD`'s independent check
  (0x1490f0:232-248) retries once per 60 ticks within 2.0 — both gated on door+0xF4 & 0x12 (CLOSED/
  CLOSING) exactly like `NDrone2_OpenDoor` (0x14aa38). Neither ever forces an opening door.
- A walk-through drone mills into the doorway while it waits: `LinkCreep_Handler` (0x15a2c8:67-70)
  skips the blocked/bounds test entirely while +0xBA8 stands and keeps creeping forward.

So the old test — Bond pressing E every 2nd tick while the door was ajar, the opposite parity to the
escort's own 2-tick timer-C retry — **activated the shoji every single tick**: the busy marker skipped
every motion tick, the presses kept reversing the openings, and the door could never complete.
**The game itself deadlocks under that input** (all four mechanisms above are the game's code); a
real player presses until the door starts to swing, then stops. Per the task's rule the fix is the
test's, and the port needed none:

- Bond now presses only while **Mayhew is in `DSTATE_OpenDoor` (0x81)** — the game's choreography:
  the escort halts at the route's door node and works the door himself (the escort test's "door #1
  opened with Bond 3.9 away" and the atlas's PCSX2 trace "Door 2 re-opened at 44.2 s with Bond ~18 m
  away (a drone opened it)"), with Bond's E the other activator — **and only while the door is
  CLOSED/CLOSING** (`state & 0x12 != 0`), the game's own NPC activation gate.
- Running Bond ahead EARLIER (while the door was merely closed) turned out to be its own deadlock:
  he stood inside the lead's PlayerInWay band and parked Mayhew 2 units short of the node (state
  0x1e forever). Gating the press on 0x81 removed that too.
- Sister latch added: `door_opened` asserts the shoji reached state 4 at least once (the old mash
  pattern fails it — the door log's frozen pos01 proves it never did).

Result: Mayhew walks the whole route in state 0x1d — through the shoji at ~t 1700 (his own 0x81
work), the interior doors, the secretdoor pair, down the stairwell — and ends ON the plate
(−33.13, 3.31, 14.80; the AND-plate centre is (−33.1, 2.30, 15.43)); the AND-touch fires ch 0x10 and
`level_exit` is 0x07000002. 3/3 stable re-runs.

Two more test-harness fixes the walk exposed (both inside `fix_mission_live.rs`):

- The follow offset (2.5 behind along Mayhew's **yaw**) put Bond inside PlayerInWay's 3.0 band
  [M 0x15ea28: entry needs route-blocked + Bond within 3.0; case 3 resumes only when Bond is beyond
  3.0] and flipped Bond ACROSS Mayhew whenever a wait state turned Mayhew to face him. The follow now
  trails **3.9 along the route direction** (the escort test's own distance: outside 3.0 and 12, inside
  the lead's ≤ 10 resume band [M 0x15eda8 case 3]), re-floored each tick (the off-line trail spot can
  hang over a drop — Bond fell to y −3367 once).
- The nav-gap nudge fired on a **per-tick** `moved < 0.05` read. The stairwell-head creep advances
  ~0.03/tick (real, slow progress), which counted as a stall, so Mayhew was yanked 2 units toward his
  target every 30 ticks and ping-ponged around route node 27 (−26.24, 14.48) for 3,200 ticks without
  ever ARRIVING — the route never advanced to the doorway nodes. The stall test now reads the
  movement accumulated over the 30-tick window (a true stall — ~0 movement — still fires). With that,
  the escort's mission route walks out **in full**: its last node (−33.11, 2.68, 14.35) is ON the
  AND-plate [L nav node dump], so the escort reaches the plate by his own route, exactly the game's
  design.

### 2. `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` — no longer fails on the merged tree

The held-merge failure predates fix-script-start-tick's control-walk change landing in master. On the
merged tree the test passes 3/3 stable: the contact walks his route (guard-clear rounds asserting the
game's hide/clear loop [M 0x15ec08]), settles within 5.0 of the plate, and the test's own asserts hold.
No port change, no test change. The escort reaching the plate *under stairwell guard pressure in the
real game* remains the escort report's open live check (live check 2).

### 3. `agent-ctx/fields/` removed

`git rm -r agent-ctx/fields` (the scratch copies of the field docs; the real ones are `docs/fields/`).
`agent-ctx/B-3a-b.md` left alone as instructed.

## What is still red, and why

**`fix_npc_doors_live::the_escort_opens_the_interior_doors_on_his_route` (1 failure).** This is the
predecessor task's test — it does not exist on master (the held merge was never pushed), so master
alone cannot reproduce it; it fails on the escort branch **plus master's newest merges**. The shape:
the escort's door work is green (door #1 opens at +1327 with Bond 3.9 away — the test's own OPEN
event), but the stairwell-head guard group now reaches and re-hides the contact (armed peers at 0.3–1.5
for thousands of ticks) and drains Bond dry (clip 1 spare 0 against health-10 guards, the walker
resupply wedging at the courtyard's east ledge for 2×6000 ticks per leg), so the test's fixed follow
cap expires with him at x −2.98 (needed x < −20).

In-file mitigations landed (the test is in this task's file list): per-round ammo top-up before the
fights (the walkreal test's per-guard pattern), the follow cap raised to the walkreal precedent
(120k), `clear_guard` re-checks doors the escort opened himself (a pointless "open the way" leg onto
an OPEN door wedged the walker for 6000 ticks), and the door approach stops 2.2 short (an open kind-2
leaf parks its collider along the wall jog; walking to the door position wedged on it). The walker
panic is gone and the run gets further, but the cap still expires.

Not fixed — out of this task's reach in the time box. It needs either a walker resupply that avoids
the east-ledge wedge (`common/walker.rs` is a shared file outside this task's list) or a look at the
new guard flow from the weapons/walk-order merges. The door handling the test exists to verify is
green: the OPEN events fire on schedule, and the mission test now walks the same escort through the
same doors end to end.

**Suggested live checks for the main agent**

1. 07000001, let Mayhew lead and press E on the courtyard shoji repeatedly while he works it — the
   door should oscillate ajar/closed and never complete (the game's own mash deadlock this task
   documented). Then stop pressing: the door should complete its swing and Mayhew should resume
   within ~a second.
2. The stairwell head: watch how close the real game's guards get to Mayhew and how long he hides
   (0x1f) — the port now shows armed peers at 0.3–1.5 for thousands of ticks, which is what ages the
   escort test.
3. Confirm the real Mayhew descends the stairwell to the AND-plate (the port's route node 36 is
   (−33.11, 2.68, 14.35), on the plate) — the escort report's live check 2, still open.

## Verification

- `fix_mission_live`: 4/4 (the fixed end-to-end test 3/3 stable re-runs; the sister `door_opened`
  latch fails under the old mash pattern — the failing runs are quoted above).
- `walkreal_07000001_live`: 2/2 (3/3 stable).
- Full suite on the merged tree (3ec4fd5, NF_ISO, `--offline --release -j2 --no-fail-fast`,
  `--test-threads=1`): **595 passed, 4 failed** — the 3 allowed `disc_golden` failures (the slim
  ISO's missing `/DRIVING` BIGF files) plus the `fix_npc_doors_live` escort test above.
- Files touched: `crates/nf-view/tests/fix_mission_live.rs` (the task's), `crates/nf-view/tests/
  fix_npc_doors_live.rs` (the task's), `docs/WORKLOG.md` (append-only), this report, the
  `agent-ctx/fields/` removal. **No shared source files touched** — `drone.rs`, `door.rs`, `scene.rs`,
  `player.rs`, `lib.rs`, `common/walker.rs` are all untouched by this task (the merge commits bring
  master's own versions).

## What's left

- The escort test's guard-combat economy (above) — next session, with `common/walker.rs` in scope or
  after a look at the post-weapons-all guard flow.
- The escort report's open live checks (the real Mayhew's plate reach under guard pressure; the
  exit-pair drone-open trace) — for the main agent.
