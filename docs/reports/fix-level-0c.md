# fix-level-0c — making 0700000c playable

Branch `s/fix-level-0c` (latest `be7e3f0` = merge of master into the task commits). Verified: the
full suite is green except the 3 expected `disc_golden` slim-ISO failures; the new tests and the
existing 0c walkthrough all pass after the merge.

## What's done

### 1+2. The hatches — the chain is verified end to end through the REAL laser path

The user's soft lock ("the way down is a hatch that doesn't work", "crouching through it you're
blocked off") and the roof hatch ("opens with the laser watch melting its hinges; it doesn't work")
both run through ONE chain, which turns out to be fully assembled in the port already — what was
missing was any proof it interlocks on 0c. New `crates/nf-view/tests/fix_level_0c_live.rs` (4 NF_ISO
tests, 3.0 s) now drives it through the real path:

- The display vent: `ventmesh-front` #138 (class 0xE2 SimpleScript, param 3 = ch 18, 6 keys — the
  hatch slides from y 16.40 down to 14.76 [D keys]), the four hinges `hingelock1/3` #139–142 (class
  0x2E = `Lock_Create` 0x1ae1f8, out chs 14/15/16/17 [D param 5]), `VentBlockingPoly` #179 (0xE2, ch
  18, 2 keys — rises y 16.3 → 20.0, rate 10), and the AND mux #505 (type 3, out 0x12 = 18, list
  [14,15,16,17] [D triggers]).
- Test `burning_the_four_hinges_opens_the_display_vent`: Bond stands on the lower walkway with the
  Q-Laser out (0x50 — the 0c loadout [M 0x1a7ef8 case 0x700000c]); the sim's per-tick beam
  (`Player_LaserPointer` 0x1a65d8 → `gadget::laser_tick`) is aimed at each hinge and burns all four
  (`Lock_Update` 0x1ae3d8 case 0 → the lock's out write [:194-195]); the mux raises ch 18; both SSs
  play to their last keys; a collision ray through the opening passes. `the_vent_is_shut_while_the_
  hinges_hold` reproduces the user's blocker (the ray hits the hatch+blocker meshes at key 0, ch 18
  off); `three_hinges_are_not_enough` is the AND sister.
- The warehouse vent (the second way down): `VentLatch` #182 (0x2E, out 84) → ch 84 → `ventmesh`
  #180 + `VentmeshBlockingPloy` #184 both play. Verified.
- **No sim code needed changing** — the pieces merged from fix-objects2 / fix-gadgets /
  map-powerstation already interlock correctly. The laser CONTACT side is fix-gadgets' and works
  through the real path (this task's test drives it from the player's aim).
- What is still open on the hatch (render-side, for the main agent): the SS framelist POSE is not
  drawn (the meshes sit at key 0 visually even while the sim's collision follows the played keys —
  `docs/systems/` the SS module doc), and the `obj_stor*_pulley` winches #351/#364/#377 are SSs on
  ch 0 with no driver.

### 3. The camera gadget — my side is fine; the equip is 045d's

Weapon 0x54 IS in the 0c loadout [M 0x1a7ef8 case 0x700000c] and the photograph chain works in the
port's walkthrough (`the_camera_photograph_writes_the_jetpack_channel` passes). For 045d: the
`CamSubject_Update` 0x1bc668 path reads the contact's weapon id == **0x54** — the same id
`gadget.rs` documents as "the laser watch". The watch's mode split (laser vs camera vs taser 0x55)
is the likely reason the user "can't equip the camera": the selection gives one 0x54 entry. That
split is 045d-fix-gadgets' to own.

### 4. The AI — the guards are spawner-held, not broken in the sim

Probed all 40 drones on 0c after the intro and over 600 ticks:

- 35+ sit `present=false` with switch channels 4/6/27/1 — the spawner gates (the trigger plates
  #426 (out 4, ground floor), #479 (out 6, the big volume (25.4,−215.8)..(39.7,−197.0)), and
  ch 27 = 0x1b = objective 1 complete [D triggers]). They release as Bond walks the route — the
  game's own design on this level.
- The visible-and-waiting ones (dr7/dr9/dr17/dr24/dr27) are in WAIT_SWITCH with `enabled=false` —
  the channel gate, correct per `DSTATE_WaitSwitch`.
- The two patrolling guards (dr1/dr39, skin …93) DO walk the roof with advancing anims (danim
  282→328→392, script time 1:1) — the sim's AI/anim state is alive.
- The user's "walking animation is kind of fucked" is therefore most likely the RENDER (the anim
  posing / the character mesh following the danim) — the main agent's render check (merge lesson
  6); live checks below. `docs/fields/drone*.md` states verified against the port's fields.

### 5. The white floor patch — the object is named

The `--bond 14.22,16.73,-154.43` spot sits inside **`room_ex10_5` placement #117** (class 0xc047 —
a static room piece, t (0,0,0), world-space boxes; the same class as the room_sk* pieces). The draw
is 045i-fix-render-artifacts' (a texture/material lookup on that mesh's floor batch — the port draws
some 0xc047 pieces untextured there).

### 6. The invisible guards — unblocked by this task

Nothing in the sim hides them (`present`/`drawn` gate the render; the spawner holdback is the
game's). With the hatch chain proven the level is completable; the user can now check the guards'
models (models-characters') in play.

## How it was verified

- `crates/nf-view/tests/fix_level_0c_live.rs` — 4 tests, all failing before… none: the chain was
  already correct in code (no fix was needed — the deliverable is the proof + the doc). The two
  reproductions (`the_vent_is_shut…`, `three_hinges…`) fail ONLY if the chain breaks; the fix test
  drives the real beam through the player's aim, not `Lock::burn`.
- The existing `walk_0700000c_live.rs` (4 tests) still passes; the full suite green except the 3
  expected `disc_golden` (slim ISO), run on the pushed tree.
- Disc evidence: the placements/triggers dumps (`nf-extract placements/triggers`) — every [D] cite
  above is from those; the decompile cites are [M] with addresses.

## What's left

- The SS framelist render (the hatch/blocker meshes don't visually move yet) — render-side.
- The watch-mode split (0x54 camera vs laser) — 045d-fix-gadgets.
- The pulley winches (#351/#364/#377) have no channel driver — small, next session.
- The guards' walking-animation render check — the main agent.

## Live checks for the main agent

1. **The hatch render**: on 0c, stand Bond at the lower walkway before the display vent —
   `--bond 6.0,14.5,-110.5,2.6,-0.1`. Expected after a 4-hinge burn: the vent hatch mesh slides
   down/back (key 5: (5.78, 14.76, −113.83)) and the invisible blocker rises (y 20). Today the sim
   opens it but the MESH will still be drawn at key 0 (the framelist render gap).
2. **The guards' walk**: `--bond 13.0,13.5,-197.0,0.5,0.0` (over patrolling dr1 at (13.0,12.1,
   −197.9)) — the guard walks the roof; check the legs animate with the danim (the port plays
   danim 328/392 at speed 1.0) and don't slide/T-pose.
3. **The white patch**: `--bond 14.22,16.73,-154.43,9.919,-0.349` — the floor belongs to
   `room_ex10_5` #117; 045i can compare that batch's material against a neighbouring batch.
4. **The locks**: gdbwatch on `switch_channels[14]` (the first hinge's out, `Lock_Update`
   0x1ae3d8:194 writes it) while melting the hinge — expected 0→1 the tick the beam holds on it;
   then ch 18 (the mux #505) 0→1 when all four are done.
