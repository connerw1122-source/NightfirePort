# fix-gadgets — Bond's gadgets: the selection re-enabled, three systems ported

Status: DONE (the selection + the Q-Laser/watch + the Q-Claw grapple + the decryptor's lock burn,
each with tests; the decryptor's scan UI and the mission-layer gadget functions remain).

## Which key selects gadgets

**G** cycles gadgets (the game's own bind is the d-pad Up/Down [M Player_WeaponChange 0x1a6f80
mode 1]). Q cycles weapons. The G key was commented out in the viewer with the note "disabled until
one is ported" — the machinery itself was already complete, so re-enabling was the whole fix for the
user's "can't equip the camera / it wouldn't let me switch to it".

## What was broken and fixed (each reproduced first)

1. **The selection** (the user's #1 ask — the camera on 0700000c, the briefcase on 07000012):
   - The port-map claimed "gadgets and non-weapons have 0" in the selectable flag. The weapon_data
     says otherwise: +0x04 is **1** for 0x4A/0x4C/0x4E/0x4F/0x50/0x51/0x52/0x53/0x54/0x56/0x57/0x58/
     0x59 [D ACTION.ELF weapon_data static initialiser] — the port-map note was wrong, the cycle was
     only hidden behind the dead key.
   - A real pollution bug the user's "wouldn't let me switch to it" points at: a gadget→gadget hop
     (the cycle 0x4A→0x4E→0x50→0x52) wrote the GADGET id into `prev_weapon` (+0x930) through the
     switch machinery's unconditional writes [M 0x1a6f80:134-141 — +0x930 takes weapons only,
     +0x932 gadgets only; the port wrote +0x930 unconditionally in four switch paths]. The next
     WEAPON cycle (Q) then toggled straight back into the gadgets — the user could never get back to
     a gun. Fixed: the leave-side prev slot follows the leaving weapon's type in all four paths.
   - Tests `gadget_select_live`: 0700000c's camera pickup (weapon 0x52 at (54.0, 1.3, −173.5) [D])
     takes and equips through the G cycle; the sister asserts the weapon cycle stays in 1..0x47;
     07000005's level-start gadgets (0x4A/0x4E/0x50/0x54 [L ee_start_05.bin]) are all reachable.
2. **The Q-Laser (0x50/0x51) and the laser watch (0x54)** — new `crates/nf-game/src/gadget.rs`:
   the per-tick beam ray (the head along the view, 100 units [M Player_LaserPointer 0x1a65d8's
   structure]); a lock the beam touches burns [M Lock_Update 0x1ae3d8 case 0 — the beam's weapon id
   (the 0x56/0x57 band) in the lock's contact list] and the lock's out channel rises. The laser
   hinge/safe behaviour: 07000008's two safe bolts (ch 134/135 [D #207/#208]) burn under the aimed
   beam. The port-map's laser-flag note corrected: the +0x68 bit-3 gate belongs to the GUNS' laser
   sights (0x06/0x07/0x08/0x09/0x0f/0x17/0x18), not the gadgets.
   - Test `gadget_laser_live`: the watch equips via G on 07000008, the real look input aims at the
     safe's top bolt, the burn completes (ch 134); the sister aims at the ceiling and nothing burns.
   - Geometry note: the 08 safe door's collision plane sits ~0.9 in front of the bolt centres
     [D the probe grid — every standpoint's world-capped beam lands ≥ 0.88 from a bolt], so the
     lock contact uses the aim line's closest approach within the lock's radius + a 0.25 beam radius
     [S] (the game's beam object lands ON the knob it burns; the port's world cannot represent that
     reach without the level-collision correction).
3. **The Q-Decryptor (0x4E/0x4F)**: the lock burn works through the same beam path [M 0x1ae3d8:57-63
   — the contact's weapon id 0x4E/0x4F, the burn 3/tick]. `gadget_laser_live::the_decryptor_burns_
   the_safe_bolt`.
4. **The Q-Claw grapple (0x4A/0x4C, substate 2)**: the grapple points parse (placement **0x3A** —
   the port map's "0x3E" was the WIRE's type; `Grapple_Create` 0x198298's class-0x51 object with the
   world sphere; 07000005's #281/#282 [D probe]); the fire with the claw out latches the aim-ray
   point [M Player_SetGrapplePoint 0x195da0] and enters the substate 2; `Player_SSGrapple`
   0x192860's reel (the pull 0.05→0.5/tick single-player) hangs Bond < 2.0 from the point; the fire
   releases. `gadget_grapple_live` drives the whole loop on 07000005's point #282.

## The user's three gadget reports

- **The Q-Worm on 0700000a**: there is no placed gadget pickup on that level [D the pickup dump —
  5 pickups, none 0x4A..0x5F]; the game grants it from the story scripts (the mission layer). With
  the G key live and a granted gadget in the inventory, the cycle reaches it; the grant itself needs
  the mission-layer work (031-fix-mission).
- **The camera on 0700000c**: pickup → G → equipped (the test). The camera's fire-side function
  (the photo) is the mission layer's.
- **The briefcase on 07000012**: the same 0x52 pickup — takes and equips (the same path as the
  camera; the test covers the id).

## Not ported (honest leftovers)

- The decryptor's scan-mode camera/UI (`Player_SSScanMode` 0x193098's action axes and the digit
  decode display) — the decryptor selects and burns locks; the UI is not modelled.
- The camera/briefcase/Q-Worm fire-side functions — the mission layer's script events.
- The render items: the laser beam's endpoints are parked on `Sim::laser_beam` for the renderer (the
  meshes 0x2000131/0x2000133 [M 0x1a65d8]); the grapple rope (the mesh 0x2000138 [M 0x195d10]) is
  not drawn. Both are main-agent render checks, listed below.
- The grapple's swing momentum at release [S — the release drops Bond].

## Verification

- `gadget_select_live` (2), `gadget_laser_live` (3), `gadget_grapple_live` (1), the gadget unit
  tests in `nf-game` (8) — all green.
- The full suite, run per crate (the sandbox OOM-kills the four-crate background runs):
  nf-game 23 binaries ok; nf-formats ok except the 3 allowed `disc_golden` (the slim ISO); nf-audio
  ok; nf-view 83 binaries all ok.

## Live checks for the main agent (renders)

- `--bond 160.3,44.6,-115.4` on 07000008 with the watch equipped, aimed at the safe: the beam
  (0x2000131/0x2000133) should draw from the watch to the bolt; the bolt's burn effect plays.
- 07000005 near (13.75, 32.29, 57.30): the claw's rope (0x2000138) should draw Bond → the point
  while the substate 2 runs; the reel moves Bond to the point.

## Shared files touched (minimal, additive)

- `player.rs`: SS_GRAPPLE (a new constant — no arm renamed), `Bond::grapple`, `Input::fire`, the
  dispatch arm + `grapple_tick` (new).
- `scene.rs`: the grapple enter path (before the bond tick), the laser tick hook, the prev-slot
  type gates in the four switch paths.
- `lib.rs`/`replay.rs`: `Input::fire` plumbing (the viewer's G key un-commented).
- `drone.rs`: `grapple_points` (the field, the create, the ctor — on their own lines).
- Docs: `docs/port-map/gadgets.md` (the status table + the corrected flag notes), `docs/WORKLOG.md`
  (the fix-gadgets entry appended).
