# fix-nav-remainder — report

Branch `s/fix-nav-remainder` (from master 0cda458). Time-boxed session; the task's order was
door/kick/action nodes first, then GoToGoalPosition, then the rest.

## What's done

**The route's special nodes — `NDrone2_ReachedDestNode` 0x153358 (G-027's door/kick/action item).**
A node-flag census over all 20 story levels (disc data, run as an ephemeral test) found:

- kick nodes (flag 4) only on **0700000c** (the "DoorOpen" kind-2 path, nodes n0/n1 at
  (40.5, 11.6, −209.7)/(39.5, 11.6, −209.6) — the level the user called "bugged as hell") and
  **07000012** ("Default Navigation" n144 (1.7, 0.4, 17.4) / n248 (0.8, 0.4, 17.4), plus "Patrol
  Path - C");
- action (8) and fiddle (0x40) nodes **nowhere on the disc**;
- ally-wait (0x20) and the four Mayhew-line flags only on 07000001's kind-8 "Mayhew Route".

Ported in `crates/nf-game/src/drone.rs`:

- `route_reached_door` → **`route_reached_special`**: the full 0x153358 dispatch in the game's
  order — door 2 (existing), kick 4 (bind + the node's once-only 0x8000 flag → `DSTATE_KickObject`
  0x82), action 8 → `DSTATE_ActionAnim` 0x83, ally-wait 0x20 (gated on `switch_channels[0x99]`,
  checked in `Drones::tick` which owns the channels) → state 0x21 with the return state saved in
  +0x5A2, fiddle 0x40 → `DSTATE_StandFiddle` 0x84; the Mayhew lines (flags 0x10000/0x20000/0x40000/
  0x80000 → `Drone_MayhewTalk` 0x149818 speeches 0x44/0x45/0x46/0x47) fire before the dispatch.
- **`move_to_goal` (GoToGoalPosition 0x63's route half) now runs the same dispatch** — the game's
  `LinkCreep_Handler` 0x15a2c8 serves every followed route; the port skipped it for 0x63, so a
  repositioning guard walked through door/kick nodes without ever working them.
- New state arms, each citing its decompile: KickObject 0x82 (0x1684c0 — ENTER the kick anim 0x4e
  + timer C at +2 ticks; msg 0x20 (the anim's event 6) → the grunt sfx 0x28 + `NDrone2_KickObject`
  0x14aa00 + straight back to the saved state +0x5A6; timer C re-kicks), ActionAnim 0x83 (0x168628
  — ENTER plays the drone's start script `+0x554` as `DroneAnim_SetScript(+0x554, 10, 0x32, 0x1e,
  0, 5, 0)`; script-done → state 0x6f), StandFiddle 0x84 (0x1687c0 — ENTER anim 0x23 + timer C at
  +5 ticks; the timer and msg 0x20 → +0x5A6).
- **The kick binding**: `NDrone_InitKickNodes` 0x139ce0 binds each kick node the nearest class-0x1b
  object whose script info reads 0x0600_05bb (the decomp prints `0x60005bb`). Port: `Drones::tick`
  builds a (nav path, node) → `Scripts::players` map for kick nodes, keyed to the ScriptPlayer
  carrying that script, and services the kick outbox as `SP_UnPause` 0x1dca58 (`activated = true` +
  the current script's pause cleared — the same fields the port's own mask-bit-1 unpause uses). The
  nodes' 0x8000 once-only flags live in a `Drones`-owned set (the game sets them on the shared
  node, never clears them).

**GoToGoalPosition 0x63** — already implemented before this session (verified, not rewritten:
`move_to_goal`, `SetGoalToCloserPosition` 0x155e10, `SetGoalToDroneWithLosToOpponent` 0x1543f8 are
all present and cited in drone.rs); this session only added the missing `ReachedDestNode` dispatch
inside it.

## How it was verified

`crates/nf-game/tests/nav_kick_live.rs` (NF_ISO), reproduced BEFORE the fix (the drive test failed
"the guard must enter DSTATE_KickObject (0x82) at the pair (pre-fix: never)"; the log showed the
guard walking [144, 248] and arriving without a kick):

1. **07000012 kick drive**: the guard nearest the pair, driven by GoToGoalPosition (the real 0x63
   input path: `dr.state = GO_TO_GOAL` + ENTER, the drone_portcheck_live pattern) with a goal that
   forces the route [144, 248] (asserted consecutive — a link 144↔248 exists). Post-fix: the guard
   enters 0x82 exactly once (the once-only node flag), the kick grunt sfx 0x28 fires, and the
   script-0x0600_05bb ScriptPlayer (#662, mask 4 = the activated-trigger bit) leaves its wait
   (state 0 → 2), plays the script and self-deletes per its stay flag — `SP_UnPause`'s observable.
   Sister assertion: the kick happens once (count == 1 — the 0x8000 flag).
2. **0700000c data**: the DoorOpen pair is adjacent (the link that arms the kick wall), the level
   has **no** 0x0600_05bb ScriptPlayer (so the game's own binding is null — `NDrone2_KickObject`'s
   null check), and a start guard does pin the DoorOpen path.
3. Full suite: started in the background at the end of the session (see the tally below if it
   finished; on the slim ISO only the 3 `disc_golden` tests may fail).

Two real port bugs surfaced while reproducing (both fixed, both inside this task's dispatch):
the tick's route-path lookup only knew the patrol/mission `route`, not 0x63's `croute`; and the
"next node" for the dispatch was read as `nodes[index + 1]` AFTER `follow` — but the port's follow
advances the index in the same call that consumes a special node (the game reads it before
`SetupNextNode`), so the pair never looked consecutive. The next node now comes from the reached
node's position in the route list.

## What's left (G-027)

- **+0x31C drone-ahead flags** (`DroneMove_NoBunching`): not ported; state 0x5E is currently only
  reachable as a constant. Needs the flag's writer (which drone marks +0x31C & 2 on the one ahead).
- **Obstacle avoidance** (drones/player within 3 units): not ported.
- **EnemyMission (kind-8 assign)**: the Idle-enter `AssignAIPath(8)` path to EnemyMission 0x1B;
  the Mayhew Route's ally-wait node is now handled but the kind-8 assign itself is not.
- **`PatrolTalk`'s RNG use**: untouched.
- **Boundary push-out**: `push_out` exists (nav.rs, the push direction *suggested*); unchanged.
- **Mayhew lines**: the pushes land in `talks` (indices 0x44–0x47 into the voice table) but the
  index space and `Drone_MayhewTalk`'s candidate gates (+0x4F8 chain, the drone+0xac > 0 float —
  no ported reader) are unverified; *suggested*: the walker speaks, not the type-0x0c drone.

## Live checks for the main agent

1. **07000012 kick**: place Bond so the guard near (5.0, 1.0, 17.1) chases through the doorway at
   (0.8–1.7, 17.4); expect the kick anim 0x4e + grunt and the script-0x0600_05bb prop (#662's
   placement) activating (its script plays). gdbwatch on the drone's +0xBA8: expect the SP object
   pointer stood in when the route reaches node 144.
2. **0700000c DoorOpen**: whether any guard's `Drone+0x954` ever pins path 18 ("DoorOpen") during
   play and crosses n1→n0 (the port's start census says the pin exists at level start but the
   start route to the pair is direct). gdbwatch `Drone+0x954` for the value mapping to path 18.
3. **nf-view render spots** (07000012): `--bond 3.0,1.0,17.4,3.1,0` (guard approaching the doorway
   from the east), `--bond -1.5,1.0,17.4,0,0` (the far side, the kicked prop's script playing).

## Shared files touched

`crates/nf-game/src/drone.rs` (the task's primary file) and `docs/WORKLOG.md` (append-only). No
other shared file touched; `nis.rs` had a temporary debug line during development, removed before
commit.
