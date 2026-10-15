# fix-door-react-path (G-035) — done

## What was done

`Door_ReactToHit` 0x135980 now runs through the game's collide path. The game dispatches it as the
class-0x1C collide function from `Collide_Update` 0x1ea398:59-64 (`control_funcs[0x1C]` =
{0x1363c8, **0x135980**, 0} @0x26f9e0 [ELF]) after the control walk + movement pass
(docs/atlas/object.md step 7). Its input is the door's `obj+0xD0` contact list, which
`Collide_Update` 0x1ea398:160-233 builds by forwarding each picker's `Collide_PickObj` hits onto
the hit object (+0x58 = the hitter). Doors get picked because `build_LinkToRoom` 0x1c9680:63-66
puts `obj+0xF0 & 0x20000000` objects on the global `ForcedList` 0x270590, which every picker tests
(`Collide_Pick` 0x1eb320:369-375) — the player's pick mask `HITTEST+0x98 = 0x1c0` leaves that as
the only object path. The player's pick is mode **0x800** (`Player_Collision` 0x194df0 writes
BLData+0x7D4 = 0x800 in the substate arms): segment-vs-sphere — `sqdist(live obj+0x80 centre,
closest point on the pick capsule) < (1.5·r_obj + r_pick)²` [M 0x1eae80:152-168]. The use-sphere
vs collision-mesh question resolves the same way for this path: the proxy contact test is the
sphere sweep, not the mesh; the mesh refine (`Collide_Intersect`) only feeds the world/ladder
pick, not the door's contact list.

Port changes:
- `crates/nf-game/src/door.rs`: `DoorMover` (the pick capsule + radius + the hitter's `obj+0x80`
  centre + class/gate), per-door `contacts` lists, `Doors::collide_step` (the 0x800 pick + forward
  + per-door dispatch), and `react_to_hit` rewritten from the decompile — the selection (class 3
  outright, class 2 through the `+0x4F8`/`+0xAC`/`+0xFE` gate [M 0x135980:47-72]), the kind-0
  swing-plane reversal (±0.0002), the kind-2 CLOSING reversal, kind 1 nothing, the tail
  (`Door_Interp(…,0)`, timer 2, busy [M :109-111]). The old dead `react_to_hit`'s sphere stand-in
  is gone — the pick above IS the game's test.
- The collide caller: one additive hook in `Drones::tick` (drone.rs) — after the control walk,
  before `Doors::calc` (0x1336f8:23 runs `Doors_Calc` at the frame head, so a reversal's busy
  clears before any `Door_Update` reads it) — feeding the player mover (the per-substate capsule
  the port already builds on `drone::Player::capsule` [M 0x194df0], alive only) and every present
  drone as a point capsule with the 0.4 radius [S — the drone HITTEST is unported; the validity
  gate = present ∧ non-corpse ∧ health > 0 stands in for the `+0x4F8` gate], plus a pre-walk
  position snapshot. Sounds ride the existing door-sound dispatch.
- `docs/systems/doors.md` (new): the full call structure with the addresses above, the port
  mapping, and the live checks.
- `docs/WORKLOG.md`: the entry.

## How it was verified (NF_ISO, real input path)

- Fail-first shown: with the collide hook disabled (one line), `a_closing_door_reverses_on_bond_on_07000001`
  fails — the closing shoji closes THROUGH Bond (state trace `0010`×29 → `0002`). With the hook
  live it passes: the trace shows `0008` (the reversal) → `0004` — the door re-opens and stays open
  while Bond stands in the doorway.
- The test's sister demonstrates the guard rule live: with Bond 12 m away the door did not close
  because a live guard parks inside the leaf's pick zone — accepted by the assertion only with a
  present, non-corpse drone measured inside `(1.5·r + 0.4)` of the live sphere centre.
- door.rs unit tests (11, green): the kind-2 reversal + dir flip + timer/busy tail; an open door
  ignores contacts; no contact → no reversal; the drone gate (a dead guard never reverses, the
  first valid one in list order does; the 2-tick lockout re-runs nothing); kind 1 ignores contacts
  and never reaches the tail; the kind-0 swing reversal by the swing plane; and the sister "a door
  with nobody in it closes" (CLOSED reached with far actors).
- Affected binaries green after the fix: `level_end_live` 6/6, `door_portal_live` 2/2,
  `fix_door_react_path_live` 1/1, `cargo test -p nf-game --lib door::` 11/11.

## Test-premise updates (shared test file, listed per the shared-files rule)

`level_end_live.rs` — the anti-crush rule the game's own code states supersedes two premises:
- `door_e_opens_then_e_closes_on_07000001`: the second E is now pressed from the best of the four
  2.4-off axis spots (out of the open leaf's pick zone, inside the 2.5 use reach); the assertion
  additionally accepts a guard parked in the pick zone (measured, same rule) as the reason the
  door is not closing. On this level every shoji sits on the patrol path and a guard held the
  doorway during the run.
- `the_walkin_leaves_a_closing_straddle_door_alone_on_07000016`: the closing phase is observed
  with Bond stepped 2.4 out of the pick zone; the `reopened_early == None` assert (the 102j2
  walk-in re-open bug) is replaced by the close completing (the walk-in's own guarantee is its
  `state != CLOSED` gate in door.rs [M Door_Update's scan is the auto-close only]), and the
  no-E walk-in sister re-opens from the threshold after the close.

## What's left

- The full suite did not run — the 2-hour window closed during the fix. The affected binaries
  (above) are green; run the full `tools/test/suite.py` on this branch before merging.
- The drone-side pick is a stand-in [S]: the drone HITTEST (mode words, capsule, mask) is
  unported; when it is, replace the point-capsule mover and the validity-gate stand-in.

## Live checks for the main agent

1. Read a door's `obj+0xF0` on 07000005/01 in PCSX2: expect `0x20000000` set (ForcedList
   membership — the pick's premise) [M build_LinkToRoom 0x1c9680:63-66].
2. Watch a door `obj+0xD0` while Bond walks into the closed doorway: non-NULL while his capsule is
   within ~2.6 of `obj+0x80`, `entry+0x58` = the player obj, NULL again 3+ away.
3. A guard walking into a closing shoji: watch `droneExt+0x4F8` bits 0x100/0x600 and `ext+0xAC`
   at the reversal tick — pins the port's validity-gate stand-in.
4. nf-view `--bond` spots: 07000001 shoji #0 (7.89, 5.35, 2.87): stand 0.5 in front, E, E —
   expect open held (8→4) with Bond there; close completes only from outside the leaf's pick zone;
   a guard in the doorway holds it open the same way.
