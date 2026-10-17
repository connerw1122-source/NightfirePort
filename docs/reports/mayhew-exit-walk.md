# mayhew-exit-walk — the walk to the exit before the mid cut-scene (07000005)

Branch `s/mayhew-exit-walk` (from origin/master 31dd6161). User report 2026-10-16: "mayhew is
bugged with walking to the exit as well before the mid cutscene".

## What the sequence actually is (the evidence)

"Mayhew" is the level's own name — Mayhew A = 07000005. There is **no Mayhew drone on the
level**: all 16 drones are guards 0x0500002e/2f/30 [D `nf-extract drones 07000005.bin`; the
same for 06/07/08 and the all-maps scans]. The Mayhew escort that walks with Bond is 07000001's
contact (escort-01, already ported and green).

The mid cut-scene is the **truck hand-over**:

- SP #37 "Scriptplayer_SC10" (mode 1) plays the truck script **0x06000A22** at level start
  [D placement #37; levels.md §5.2]. Its stream 8 creates a script TouchOnce out **ch 0x5C**
  at t 1882 (0x75a) and kills it at t 2312 (0x908) [D script decode]; the port's sim trace
  matches (the trigger exists between the t 2100 and t 2400 samples, centre
  **(98.49, 27.05, 106.36)** — the glist 0x02000480 "Trigger-TouchplateBig", ±1.95/±1.75 [D]).
- Touching it fires **SP #38**, script **0x060008C8** — 143 ticks, camera true, Bond skin
  0x05000007 on anim 0x06000A24 [D the script's streams] — the cut-scene that hands Bond onto
  the waiting truck. Then the ride: bed touch ch 0x71 (stream 7, parented to the truck), the
  park plate ch 0x70 (stream 6, created t 1508), the gate-guard ch 0x72 (placement #318),
  mux #316 = AND(0x70, 0x71, 0x72) → SP #320 "EndLevel" (0x06000A23, event 0x11 at t 300
  [D]) → ch 0x1D → #118 loads 07000006 [D docs/levels/07000005.md §3].
- `Trigger_Hit` 0x1b1fc8 does no geometry — it classifies the collision pass's contact chain
  [M 0x1b1fc8]; `Script_CreateEntity` 0x1d8968's 0xea arm calls `Trigger_TouchOnce` 0x1b2b88
  with the zero vector + the glist [M 0x1d8968], and `Script_Interp` 0x1d9a08 case 1 keys the
  object's matrix from the stream [M] — the plate stands where the stream's keys put it.
  Bond's walkway edge stands ~1.2 under the plate plane; his capsule (feet+0.55..feet+1.86,
  r 0.55) crosses the plate plane, which is why the edge position fires it — the game's own
  contact shape, not a port invention.

## Reproduced on master, through the real path

`nf_view::load("07000005.bin")` + `step_bond` inputs only:

1. **The chain works end to end when Bond reaches the gatehouse edge inside the window** — the
   new test `crates/nf-view/tests/mayhew_handover_live.rs::the_walk_to_the_gatehouse_fires_the_
   handover_window_and_rides_out`: the real-input walk arrives at (99.69, 25.94, 106.98) at
   t ≈ 373, **ch 0x5C fires at t 1884** (the script's own window opens at 1882 [D]), SP #38
   re-places Bond on the truck bed, Bond crouches (the game's ride stance [D fix-truck]) and
   the crouched ride parks on ch 0x70 — the chain ends the level: **level_exit 0x07000006**.
   GREEN. The sisters hold: `walk_07000005_live` 2/2, `fix_truck_live` 2/2.
2. **The manual turn+forward walk from the spawn stalls** — the same walk driven as a raw
   turn-toward-target + forward loop (what a thumb does) oscillates at
   (101.11..101.27, 24.94, 107.45..107.80) for 2400 ticks and never reaches the plate's XZ
   (needs x ≤ 100.44); the walker's nav walk crosses the same stretch fine. The collision
   there [D Room_Road_A1 tris]: the walkway/plateau surface (y 24.75-25.11) ends at z ≈ 105.8
   and x ≈ 101.4, the arch's south face rises from y 20.4 to 23.75 along it, and the lane
   south of z 106 sits at 23.75 — a beeline at z ≈ 107.5 walks into the arch face. The
   ignored `walkreal_07000005_live::walk_to_the_gatehouse_and_ride_the_truck` parks on the
   same spot ("the walked Bond stops 1.1 below and the chain never fires — 0x5c/0x70 dead for
   9000 ticks" — its own comment). With the window only 430 ticks wide (7.2 s [D]), a player
   who fumbles the approach loses the hand-over for that truck pass (the truck script is
   one-shot; the level remains completable through the gate door #118, fallback 0x1F [D]).
3. Sister `the_handover_never_fires_for_bond_who_stays_at_the_spawn` — no walk → ch 0x5C low
   through the whole window, no exit. GREEN.

## What ran

- `mayhew_handover_live` 2/2 (NF_ISO, release, --test-threads=1; the full-ride version).
- `walk_07000005_live` 2/2, `fix_truck_live` 2/2 (the sisters — no interference).
- `walkreal_07000001_live` 2/2 (the escort level, unchanged — the sanity side).
- The fast suite (`tools/test/suite.py`) **could not run in this sandbox**: background jobs
  are reaped between tool calls here (setsid/nohup detached runs died within one tool call,
  twice — the empty `suite.out` stands as the record; the same was reported by
  pause-music-1b and hatch-02-intro). The foreground runs above cover the touched surface
  (nf-view tests only; no sim code changed — this session is pins + evidence).

## What's left (the next session continues from the branch)

1. **Root-cause the manual-walk stall** at x ≈ 101.2: the collision feature that stops Bond
   (the walkway lip from the 24.75 lane to the 25.9-26.2 edge — the floor probes show the
   lane at 23.75-23.94 east of x 101 and the plateau at 24.75-26.2 west of it) and whether
   the game's Bond climbs it via the Player physics [M Player_Move's step rule] — then either
   the sim fix or the route fix, with the manual-walk test un-ignored and failing first.
2. Un-`#[ignore]` the two `walkreal_07000005_live` tests once the stall is fixed.
3. The full suite on the final merged tree (blocked by the sandbox reaping this session).

## Live checks for the main agent

- 07000005, fresh boot: walk Bond from the spawn (104.95, 25.12, 108.83) west-southwest to the
  gatehouse as the truck arrives (~31 s in). Expected: the 143-tick hand-over cut-scene plays
  (camera at (83.2, 20.99, 96.2)), then Bond rides the truck into the castle and the level
  exits to 07000006. Watch whether the player must jump/climb any lip at x ≈ 101 (the port's
  stall point) or the lane is a clean walk.
- `fieldprobe` poke: Bond at (99.6, 25.9, 106.9) at t ≈ 1900 — expected: ch 0x5C = 1 within
  the window t 1882..2312, SP #38 playing (the camera scripted), Bond re-placed on the bed.

## Usage

- Token/credit/context figures: **not visible to me** — this session's interface exposes no
  token, cost or context-window numbers I can read. The sandbox clock read 02:15 UTC at
  session start (expiry ~04:15 UTC per the task message).
