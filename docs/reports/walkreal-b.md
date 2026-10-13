# walkreal-b — the tower, power station and second tower missions walked with the player's own movement

Branch `s/walkreal-b`. The nine levels `07000009, 0700000a, 0700000b, 0700000c, 0700000d, 07000011, 07000012,
07000013, 07000014` are walked by the tests `crates/nf-view/tests/walkreal_<code>_live.rs` — every leg drives Bond
through the real `Input` path (`Sim::step_bond`), no `bond.pos` writes after the spawn except the stand-ins listed
below (each with its reason). The harness is `crates/nf-view/tests/common/walker.rs` (walkreal-a's).

## Per-level results

| Level | Walkable end to end | Notes |
|---|---|---|
| 07000009 | **yes** | car park → stairs → lobby → the lift → level_exit 0700000a, input only. Documented blocker pinned: the elevator-activation writer (every game path enumerated in the test header). |
| 0700000a | **yes** | the loop walked by nav nodes to mission progress; the Q-Worm cubicle collision and the locked office door ch 0x30 are the level's two documented blockers — ch 0x48/0x30 driven as the documented stand-ins (the Q-Worm fire side is unported, useobj.rs). |
| 0700000b | **yes (mission complete)** | the deck jump driven east off the helicopter (south is the body's 57.92 step); the win read through the re-firing band's 20-tick window onto the mission latch. |
| 0700000c | **yes to the 0d gate** | the ST-1 plate ch 0x1b (objective 1), the REAL camera photograph (the bullet path into the CamSubject volumes, ch 0x1c, objective 2), the yard, the ladder climb, the patrol circle, the gate E → level_exit 0700000d. Stand-ins: the vent/display/secret-door/pulley crossings — all unported 0xE2 SimpleScripts on map-powerstation (the same gap the map task documented). |
| 0700000d | **yes to the documented stop** | the start trigger (objective 3), the crane stand-in (0xE2; fix-explosions owns the crane/laser), the ST-3 plate ch 0x1d (objective 4), the REAL laser photograph ch 0x1e (objective 5), the corridor plate ch 0x28, then THE DOCUMENTED STOP at the C5 door: ch 0x1f has no writer on the disc (the mounted-laser cut-scene is unported), the AND stays open. |
| 07000011 | **yes** | the spawn tile ch 0x7E, the stairwell door's south band, the stairwell-descent stand-in (the flight graph needs the level mesh — the probe could not resolve the landing steps into a walked chain), the Load Point E → level_exit 07000012. |
| 07000012 | **yes with 2 stand-ins** | everything else real (below). |
| 07000013 | **yes** | the full warm chain: 12 walked → 4A's spawn drop ch 0x4D → 13 → the lobby walk → the entrance → MISSION SUCCESS (state 6) → LevelToEndTo 0x07300013. |
| 07000014 | **in progress** | the route is written; leg 1 walks the vent store room to the drop touch #301; the vent cut-scene does not yet write ch 0x47 within the window — the next session continues from the branch. |

## The stuck points found and what was done

1. **The door lock edge never fired (FIXED, game code)** — `crates/nf-game/src/door.rs` `Doors::end_tick`.
   The game snapshots the switch-channel latches with TWO amemcpys at the head of
   `control_movement_object_handler` [M 0x1336f8: `amemcpy(0x26fd90, 0x26fe90, 0x100); amemcpy(0x26fe90,
   0x26fc90, 0x100)` — `prev ← hold; hold ← channels`]: `prev` lags the live channels by two snapshots, so
   `Door_Update`'s lock rising edge (`prev == 0 && now != 0` [M 0x136550-0x136608]) fires for any channel pulsed
   since the last snapshot — including the player's own Q-Decryptor keypad burn (which the port runs in
   `step_bond`, before `Drones::tick`). The port made ONE copy into `prev_channels`, so the burn's edge was
   invisible and the keypad door stayed locked forever. Fixed by adding the `hold_channels` latch and the
   two-stage copy. Reproduced first: `walkreal_07000012_live` failed at "through door[19]" (the burn fired,
   the door never opened) before the fix, passes after. Cited: docs/atlas/door.md's lock-path entry.
2. **07000012's door[19] doorway crossing (stand-in, unresolved)** — the closed leaf's tunnel (x 6.74..7.07,
   z −1.46..−0.26 [D probe]) is 1.2 wide; the OPEN leaf's slab (x 5.65..6.85, z −1.59..−1.26 [D probe — the
   swing matches the level's own key data]) pokes its east edge 0.2 into the tunnel mouth, and the capsule
   (r 0.55 [M Player_Collision 0x194df0]) has no convergent push path between that edge and the north jamb:
   the drives from z −0.9/−0.8/−0.76/−0.72/−0.5 all pin at x ≈ 7.14, and `capsule_push` at the pin sums an east
   push (0.057, 0.029) that cancels the move. The game passes this doorway on the same key data — the funnel
   difference is unresolved. The stand-in crosses the leaf; every other leg is real.
3. **07000012's shaft lobby entrance (stand-in, unresolved)** — the lobby (exit #669 at (2.3, 10.0), touch
   #683 at z 12.1-13.1 [D]) is walled off on every probed line (south x 0.5/2.4, east z 9.5/11, west z 9.5 —
   class-21 hits [D probe]); the call-button #699 (1.1, 8.2) is an E-touch on the wall face, not a doorway.
   The game reaches the lobby some ray-invisible way. The stand-in places Bond inside; the touch and the gated
   exit stay real.
4. **07000012's furniture wedges** (route shape, not bugs): the direct console→shaft lines climb the desk
   island (y 2.1); the walk detours south of the console.
5. **07000013's lobby desk block** (route shape): the direct spawn→entrance line wedges at z ≈ 25; the walk
   uses the east lane (the wing touch plate's own lane) then swings west.

## --bond observation spots for the main agent (renders)

- `--bond 9.3,1.1,-0.6,π+0.4,0` (07000012): the keypad wall with the two "Lock - KeypadLocked" boxes left of
  the closed door[19]; the decryptor's beam should touch the nearer box.
- `--bond 6.4,1.03,-1.5,−π/2,0` (07000012): the OPEN door[19] leaf lying along the wall west of the doorway
  (the swung leaf slab the report's item 2 pins against).
- `--bond 2.3,0.7,11.9,0,0` (07000012): inside the shaft lobby — the SS lift doors #677/#678 at their open key
  (x 3.070/1.34), the exit #669 zone behind.
- `--bond -6.4,-368.0,66.2,π,0` (07000013): the front entrance band from inside; the success plates' zone.
- `--bond 5.4,7.35,-113,−2.6,0.1` (0700000c): the jetpack display through the glass (the camera photograph
  target); `--bond 137,5.1,-321,−2.9,0.1` (0700000d): the laser display in the trench.

## Live checks for the main agent

1. **07000012 door[19] (the corner pincer)**: in PCSX2, walk Bond through the keypad door after the decryptor
   burn; `gdbwatch.py` on the player position (`obj+0x30..0x38`, the BLData matrix row 3) at 60 Hz while
   passing. Expected: the position crosses x 7.07→6.74 through the tunnel at z ≈ −0.8±0.1 with the leaf OPEN —
   the passable z band and whether the leaf's edge ever pushes Bond tell us if the game's open leaf collides
   at all (if Bond's path ignores the leaf slab, the game excludes OPEN leaves from the actor-collision query —
   a one-line gate in the port's `set_dynamic`/`near_actor` path).
2. **07000012 the shaft lobby entrance**: from the console, walk to the lift; `gdbwatch.py` the same address —
   the actual doorway the game uses (x, z) when entering the lobby; then re-probe the port's collision along
   that line.
3. **07000014 the vent drop (for the next session)**: confirm the touch #301's firing in the game (the y band
   2.07…4.14 vs Bond's feet on the store-room floor) and that SP #276 starts on it.

## Left

- 07000014: the full route (the vent cut-scene, the Kiko leg, the west generator-room fuseboxes, the escape,
  Rook, the Delta exit) — the test is written; the next session debugs the vent cut-scene's firing and walks
  the rest.
- 0700000c/0d: the 0xE2 SimpleScript crossings (the vent/pulley/crane/secret-door) stay stand-ins until the SS
  spline objects move Bond (the same gap map-powerstation documented).
- The 0a/0b Q-Worm and the 0c camera-gadget fire sides remain the mission layer's unported pieces (their own
  tasks own them).

## Verification

- The full suite (`NF_ISO=… cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view
  -- --test-threads=1`): the result is recorded in the branch's final commit message (run before the push).
- Every walkreal test drives the real input path; the enemy allowance is the walker's health top-up (the pain
  pipeline still runs) except Rook on 14 (the real gun path kills him) — not yet exercised this session.
