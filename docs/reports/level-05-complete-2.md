# level-05-complete-2 — the held 05 completion, unheld

Branch `s/level-05-complete-2` = `origin/master` 0603094e + the held `s/level-05-complete`
32f6a18 (both WORKLOG sides kept; the 07000005 objectives section takes the held branch's corrected
census — placement 36 writes ch 0x45, 07000006's OR #498 writes ch 0x46).

## Done

1. **`fix_mission_live::objective_announces_on_its_enable_channel_and_completes_on_its_condition`
   fixed (green).** The old premise — "no objective notice before the enable channel 0x45 fires" —
   is refuted by the game's own boot: the fresh-boot snapshot already carries
   `switch_channels[0x45] = 1` with the mission record still hidden (state 1, the monitor's first
   tick pending) [L ee_start_05.bin @0x26fc90 = {0x45}, @0x2a3e28+0x14 = 1 — verified by reading the
   bytes this session; record bytes `46 00 01 45` = cond 0x46, enable 0x45]. The reveal plate
   (#36, TouchOnce out 0x45, mask 8, under the cold start [D placements]) fires ON the boot tick,
   exactly as the held branch's measured reach predicts. The test now asserts the real path: ch 0x45
   comes on during the first idle ticks WITHOUT the test setting it, and the announce follows
   [M 0x1827d8 case 1]; the 0x46 completion path and the unrelated-channel sister stay. Built-in
   sister: on the old pos−0.4 toucher the plate never fires and the test fails.

2. **The reach change is now probe-proven behaviour-neutral on 07000001 and stays evidence-backed.**
   A/B probes (channel-provenance instrumentation over the full escort walk): the set and order of
   channel writes is IDENTICAL at pos−0.4 and pos−0.6 — {0xB@47, 0xC@4955, 0x27@5964, 0x32@7006,
   0x33/0xE@7053, 0x12@10607, 0x26@11244/11238, 0xD@20249, then the door chain 0x11/0x13/0x4/0x5/
   0x99} — the only difference is a 6-tick phase shift at ch 0x26 (a guard-fight kill timing),
   chaos-amplified into different guard-clear rounds. No trigger fires at −0.6 that doesn't fire at
   −0.4. The reach value itself: −0.56 was tried and REFUTED — the 05 boot plate needs reach ≥ 0.570
   on the port's spawn (port y 24.98 − plate 24.41 [D/L]), and the port-geometry standing-ride
   constraint is < 0.65, so the port's window is [0.570, 0.65) and −0.6 sits inside it. The game's
   own window [0.557, 0.575) [L ee_start_05] describes the game's geometry, not the port stand-in's
   offset.

3. **A real port gap found and fixed on the way: the 0.4–1.0 special-node dispatch tier was
   missing.** `Route::follow` dispatched a reached special (door/kick/action) node only when the
   drone got within 0.4 of it. The game has a second window: `LinkCreep_AtSpecialNode` 0x159dd0
   returns true at d < 0.4, OR at 0.4 ≤ d < 1.0 when `NDrone2_DroneNearPos` 0x143e68 says someone
   is near (its player tier: SqDist2D < 0.3025 and |Δy| < 2.0 [M 0x143e68:8-15]). A drone blocked
   at a closed door panel stalls inside that 0.4–1.0 band, so with the player standing close the
   game dispatches the door node and the drone opens the door itself; the port never did. Ported:
   `follow` takes a `near_pos` resolver; the four route-follow call sites pass the player tier
   (`Drone::ppos_near`, the exact [M] constants). The DroneNearPos list-walk tier (a counting-class
   drone within the dropped float radius) is NOT modelled — Ghidra drops the float argument and the
   +0x4f8 word mapping is unverified; marked [S] in code and listed as a live check below.

4. **`npc_doors_live::the_escort_opens_his_door_without_bonds_help` — root-caused, NOT fixed
   (now `#[ignore]` with the diagnosis; the only red item, and it is red on every ordering this
   sandbox can produce).** What the session established:
   - The failure is NOT the reach change's trigger behaviour (see 2 — identical channel sets).
   - The harness's guard-clear ordering is the trigger: on −0.4 the last clear round was [5,15]
     (south guards), the shoji had been opened by an earlier flank crossing and auto-closed before
     the escort's arrival, and the escort node-arrived (0x81 at +58606) through his door node. On
     −0.6 the phase shift made the last round [3,14,15] (NORTH guards); Bond's approach crossed the
     shoji, the escort un-cowered the moment the last guard died (mid-round), met Bond's southbound
     return crossing at the door plane (+73079 open, +73090 crossed) and slipped past his door node
     without the 0x81 dispatch.
   - The port's dispatch at that geometry: the escort creeps to the closed panel and stalls in the
     0.4–1.0 band (the driven control stalls at 1.44 on the goal route; the creep's panel stop is
     inside 1.0), where the game defers to DroneNearPos — with Bond's crossing 1.2 west of the node
     the player tier does not fire and the un-ported list-walk tier cannot. So the escort never
     enters 0x81 on any ordering tried (escort-side-only clearing tried and reverted: the north
     guards 10/16 patrol permanently within the cower band — 147k ticks, peer 6.6 — the escort then
     cowers forever).
   - What would settle it: see "Live checks for the main agent".

## What ran

- `cargo test --release -p nf-view --test npc_doors_live` — 6 passed, 1 ignored (the diagnosed
  escort test).
- `walkreal_07000005_live` (1 passed/1 ignored), `fix_truck_live` 2/2, `level_carry_live` 8/8,
  `mayhew_handover_live` 2/2, `walkreal_07000001_live` 2/2, `walkreal_07000002_live` 1/1,
  `walkreal_07000004_live` 3/3 — the held branch's green set, all green with the tier-2 dispatch
  change.
- `fix_mission_live` — the fixed objective test green (full binary not re-run after the last edit;
  the touched test + the neighbouring alarm test were the ones exercised).
- `cargo test --release -p nf-game --lib` — 276 passed / 0 failed.
- The full four-crate fast runner did NOT fit this sandbox window (background processes are killed
  between tool calls here — the nohup suite died twice; only foreground runs survive). The tier-2
  change touches every route-follow dispatch; the suites above are the door/route-home turf.

## Live checks for the main agent

1. **The game's escort creep-stall distance at a closed door** (settles the escort test): fresh
   boot 07000001, let the intro finish, follow Mayhew to the courtyard shoji WITHOUT opening it and
   WITHOUT killing the guards beyond it, then gdbwatch the escort's position (the route object's
   feet) while he stands at the closed shoji: the distance from the escort's pos to the shoji node
   (7.92, 5.34, 11.14) when he begins working the door. Prediction from [M 0x159dd0]: he enters
   DSTATE_OpenDoor (0x81) at a stall distance inside 0.4, OR inside 1.0 with the player within
   0.55 — the port stalls at ~0.5-0.96 with nobody inside 0.55. Also gdbwatch
   `NDrone2_DroneNearPos` (0x143e68)'s float argument (the radius register at the call from
   0x159eb0's callee region) — the decompiler drops it; it is the list-walk tier's radius.
2. **The un-ported DroneNearPos list-walk tier**: with the player far away, stand a second walking
   NPC within ~1-3 units of a closed door node and watch whether the blocked drone still dispatches
   (the radius + the class bits 0x600/0x100 of +0x4f8 decide). Expected if radius ≥ stall: the
   escort opens the door with only an NPC nearby — the port cannot yet.
3. **Confirm the escort test's game-order story**: in the real game, kill the courtyard guards the
   way the harness does (Bond across the shoji) and watch whether the escort still enters 0x81 when
   he reaches the door while it is open and Bond is returning — the −0.4 port run says yes (0x81 at
   +58606 with the door open); if the real game agrees, the port's node-arrival dispatch matches and
   only the creep-stall distance is left.

## What's left for the next message

- The escort test's last arm: pin the creep-stall geometry (or the DroneNearPos radius) from the
  live checks, then un-ignore with the missing dispatch arm implemented ([M]-cited).
- The full `tools/test/suite.py` run on the final merged tree (it never fits these sandboxes when
  background processes die between tool calls; foreground chunks only).
- The ch-0x4B helicopter divergence and the 05 cable route stay as the held branch left them
  (750h2's and the main agent's render, respectively).

## Usage

not visible to me (no token, cost or context-window figures are exposed in this environment; the
sandbox clock at report time read 17:2x UTC via `date -u`).
