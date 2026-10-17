# rook-door-06 — report

Branch `s/rook-door-06` (from `origin/master` bee8c129). Status: **not done — the session
window ended during the reproduce step.** The doorway stall is NOT yet reproduced headless;
what is in place is the research (the full game-side door chain with [M] citations), a
working real-path probe on 07000006, and the first hard finding: after the truck-arrival
plates, **no NPC walks a lead route in the port at all** — so the stall the user sees is
downstream of a missing walk-start, and the repro needs that trigger first.

## What's done

1. **Setup/branch** — `s/rook-door-06` cut from master bee8c129, WORKLOG entry pushed
   (e8cd6e1d), probe pushed (48b0c8c3).
2. **The game-side door chain, re-verified in the decompile** (research pass; every claim
   below was read in `work/decomp/ACTION.ELF/` this session):
   - `NDrone2_ReachedDestNode` 0x153358 — a door node (node flags bit 1) with a door-node
     successor stands the node's bound door in `Drone+0xBA8`, saves the return state in
     `Drone+0x5A6` and enters state 0x81 [:149-163]; on the pair's far side
     `NDrone2_DoorIsOpen` (0x14aaa0 — the door's `obj+0xF4 == 4`, exactly OPEN) decides the
     clear [:165-175].
   - `NDrone2_DSTATE_OpenDoor` (0x81) 0x1681f8 — enter: `NDrone2_OpenDoor` (0x14aa38 →
     `Door_Activate` 0x135be0, only from state 2/0x10) + timer C at +2 ticks; tick: wait
     while not open; timer C re-activates every 2 ticks; exit to `+0x5A6` when open.
   - `NDrone_InitDoorNodes` 0x139a70 — every flag-2 node binds the **globally nearest
     class-0x1C object with no radius cutoff** (verified in machine code 0x139b74-b88); a
     node with no door loses its flag [:68-70]; the BLINK between a pair gets 0x800.
   - `LinkCreep_Handler` 0x15a2c8 — **while `+0xBA8` stands the blocked test is skipped
     entirely** [:66-68]: the creep keeps walking into the doorway.
   - Return code 10 = "pending 0x81", produced by `NDrone2_MoveToGoalPosition` 0x1517d8:61-63
     (queued-state check), consumed by the combat/go-to-goal/ally states.
3. **The port already has all of that** — `crates/nf-game/src/drone.rs:1443-1454`
   (`route_reached_special`), `:4766-4807` (the 0x81 arms), `:8580-8592`/`:8699-8709` (the
   `door_attempt` outbox → `Doors::activate`), `:8684-8698` (ControlSTANDARD's exact 60-tick
   forget/re-ask rule), and the 663 npc-doors step-back fix (`nav.rs:620-630`). So the bug is
   NOT a missing door state.
4. **The real-path probe** — `crates/nf-view/tests/rook_door_06_live.rs`
   (`probe_rook_leads_and_stalls`): loads 07000006 through `nf_view::load_with_carry` with the
   truck-ride gates `[0x1d, 0x45, 0x70]` (start #496), waits out the intro NIS, walks the
   arrival plates (ch 185 = 0xB9 rises at t=0 — the truck NIS itself raises it), then watches
   every present drone for 70k ticks (per-tick state/pos/route/hit_door/door-state logging,
   Bond trailing at the 3..8 band on real inputs). Run it:
   `NF_ISO=<iso> cargo test --release -p nf-view --test rook_door_06_live -- --nocapture`.
5. **Finding (the probe's 70k-tick watch, twice, Bond close and Bond hanging back):**
   - The two NPCs waiting on ch 185 are **drone #15** (skin 0x05000035 = the driver d17,
     TruckDriverIdle 0x32, mission route = kind-8 path 10, 13 nodes) and **drone #12**
     (skin 0x0500002e, at (68.9, 39.0, 27.4) — the escort candidate d14). Both sit right at
     the truck arrival.
   - With Bond hanging back: **neither ever enters a lead state** (0x1c/0x1d/0x20/0x21/0x25/
     0x26/0x81 — zero hits in 70k ticks) and **no drone of the whole level moves** ([w] mover
     lines: 0). With Bond walking into him, #12 plays his start script (0x3) and then goes
     AIM_BACKOFF 0x6d → AIM_STAND 0x68 → STEP_AIM 0x73 — a guard-family armed-Bond reaction,
     not a lead.
   - So the port never starts Rook's walk; the user's doorway stall must be behind whatever
     starts it (a script event, a lead-state entry, or a drone msg the greeting SP writes).
   - 06's door-node pairs [D the probe's dump]: path 7 n0(35.9,40.7,-15.7)–n1(35.9,40.7,-16.8)
     → **door #1** (36.50, 40.00, −16.25) (the courtyard→castle-interior entrance), path 3
     n11–n14 (4.7, 49.8, −65.6/-64.2) → **door #4** (4.00, 49.00, −64.88) (the upper floor).
     Locked ch-200 doors on the map: #0, #5, #6, #7 (docs/levels/07000006.md §5 also lists
     #46/#368/#369/#370 — the port's door list from placements differs; the census dump in
     the probe log has all 12 with channels).
6. **The remaining candidate stall shapes** (from the decompile pass, in check order once the
   lead walks):
   - a locked door on his route: the A* only skips a link when BOTH ends are flag-2 and the
     bound door is locked (`DoAStarPath` 0x1571b0:262-272) — one unflagged end and the route
     crosses a locked door → 0x81 forever (locked refusal is sound-only, `Door_Activate`
     0x135be0:82-91);
   - the no-radius nearest-door binding (0x139a70) putting the wrong door in `+0xBA8`;
   - the far-side re-close pass (0x153358:165-175 returns without re-entering 0x81);
   - the port's documented deviation `drone.rs:4787-4789`: the route is frozen while parked
     in 0x81 (the game's creep keeps walking), so a slow/Bond-blocked door wedges him.

## What's left

1. Find what starts Rook's walk in the real game: dump the ScriptPlayers watching ch 185
   (`Player.mask` bit 0xB9, `crates/nf-game/src/nis.rs:977`) and their script events; check
   `NDrone2_DSTATE_Global`'s start-state params for #12/#15 (kind/param 3, the mission-route
   props 0x24/0x25). The probe's census dump (`[start] drone #…` lines) has skins/kinds/
   switch channels to start from.
2. Get the lead walking in the port through the real path (fix whatever gate eats it).
3. Then the doorway repro: follow him with the probe's per-tick log, capture the stall at
   door #1 or #4, and port the game's exact behaviour with the [M] addresses above.
4. Full suite + the fail-first/pin pattern on the final repro.

## How to resume (sandbox-reset notes)

- Repo `/home/z/NightfirePort`, branch `s/rook-door-06`; `work/` and the ISO are in place.
- **ALSA**: the dev headers live in `/home/z/my-project/scratch/alsa-prefix` (no root); the
  prefix's `alsa.pc` points libdir at the system dir, so use the patched copy:
  `export PKG_CONFIG_PATH=/home/z/my-project/scratch/alsa-pcfix` (it emits
  `-I…/alsa-prefix/usr/include -L…/alsa-prefix/usr/lib/x86_64-linux-gnu -lasound`).
- **cargo** is not on PATH under nohup: `PATH=/home/z/.cargo/bin:$PATH`.
- Full suite: `NF_ISO=$(pwd)/Nightfire-onfoot.iso PATH=/home/z/.cargo/bin:$PATH
  PKG_CONFIG_PATH=/home/z/my-project/scratch/alsa-pcfix PYTHONUNBUFFERED=1 python3
  tools/test/suite.py > suite.out 2>&1 &` (this session it was started three times; the
  first died on PATH, the second on the alsa .pc, the third was still building at report
  time — `suite.out` was empty and its result is NOT reported here).

## Live checks for the main agent

1. On a FRESH boot of 07000006 (the truck-ride entry), after the arrival cut-scene: which
   NPC walks away from the truck and leads Bond east/south — the driver (skin 0x05000035) or
   the other man (skin 0x0500002e)? Watch drone object states: the walking NPC's state byte
   (expected 0x1c/0x1d = AllyLeadInit/AllyLead if he leads) and his position over ~30 s.
2. At the moment that NPC stands in/at any doorway: his state byte (0x81 expected if the
   door path engages), `Drone+0xBA8` (the remembered door object pointer, expected non-null),
   and the door object's `obj+0xF4` (2/4/8/0x10).
3. Whether the walk start needs Bond to do something first (stand at a spot, walk to a
   plate): does the NPC move if Bond stays at the spawn?

## Usage

not visible to me — this session shows no token, context-window or cost figures anywhere in
its interface.
