# fix-wire — the cable crossing (07000005's wire, and 07000002's)

Branch `s/fix-wire`. The user's report: at the cable left of the castle gate, pressing E plays the
falling-to-death animation instead of the hand-over-hand crossing.

## What was wrong

The port had no wire gameplay at all. `crates/nf-game/src/wire.rs` held a misnamed parameter pair and a
guessed endpoint (translation + a hardcoded 4.0 length), and the only wire behaviour was a proximity hack
that changed the music when Bond walked within 1.5 units of a wire. Nothing grabbed. The cable's spot is
over the chasm, so Bond walked off and fell — the "falling-to-death animation" (`tests/wire_repro.rs`
shows it: substate stayed 0, Bond sank 1.2 units in 30 ticks).

## What was done

- **`wire.rs` rebuilt from the atlas** (docs/atlas/wire-grapple.md): the 8-byte instance is placement
  params 1/0/2/3 at inst+0x00/02/04/06 (the old `channel`/`glist` names were shifted); the endpoints are
  not placement data at all — the grab, travel and zipline segments derive at grab time from the object's
  world sphere (centre + radius resolved from the placement's object bounds, matching the live snapshot
  `ee_start_05.bin` 016dc430). `Wire::all` resolves placements against their map objects like the ladders.
- **The segment axis pinned by disassembly**: `Vec_Spherical_2_Cartesian` (0x1e3948) plus the call sites'
  `+π/2` put 07000005's wire along **Z**; the atlas's earlier X-span reading was cos/sin-swapped and is
  corrected in the doc.
- **`Player_CollWire` (0x197130) ported** — the grab: guards (regrab lockout, type ≠ 1, substate ≠
  target, the zipline lower-end refusal), inertia cleared, substate 6 (hand-over-hand) or 0xF (zipline)
  by wire type, the grab pose + camera mode 0xB, the hang offsets 1.0207 / 1.17363, the yaw = the nearer
  of wire.yaw ± π/2, the hand-reach correction.
- **`Player_Wire` (0x197658) ported whole** — the crossing: the travel-segment clamp, the auto-drop timer
  (30 s, paused in the grab pose), the drop path (jump press or the timer; the 30-frame regrab lockout and
  the weapon re-equip carry), the strafe-axis move modes 7/8 with the end-of-wire input zeroing, the
  idle/turn/move animation trees exactly as the decompile alternates them, the ±0.0542 hand sway, and the
  anims' root motion driving the travel (the move pair travels 0.7637/0.7106 per cycle at the ±1.2 rate —
  measured from the disc's sequences). No fall or death check while attached, matching the decompile.
- **`Player_Zipline` (0x197f50) ported** (stretch goal): the ride anim, the accelerating fall-speed
  accumulator, the lower-end arrival on the lifted y, and the mode-9 dismount (regrab 60).
- **The Sim wiring** (minimal scene.rs changes): the grab scan runs every walking tick against the wires'
  world spheres (the game's own object hit test is sphere-vs-swept-segment, `Collide_PickObj` 0x1eae80)
  and on the use key; `Player_WeaponNone` on grab; `Music_Event(0xD, 1)` every on-wire tick and the wire
  switch channel 1/0 now come from the substate — the proximity hack is gone.

## How it was verified

- Reproduce-first: `tests/wire_repro.rs` fails on the pre-fix code (E near the cable does nothing; Bond
  falls) and passes on the fix.
- `tests/wire_live.rs`: on 07000005, pressing E attaches Bond (substate 6, hanging at 34.4676 − 1.0207);
  with no input he stays at his end (the control leg); holding the strafe axis carries him 4.6+ units
  hand-over-hand to the far end with health 100 the whole way; the clamp holds the line at rest; the
  action press dismounts (regrab 0x1E) and he stays alive. The same chain passes on 07000002's wire
  (offset sphere centre, 11.5 radius). A synthetic zipline test rides downhill, arrives at the lower end
  and dismounts, with the lower-end grab-refusal sister.
- Full suite: 353 passed, 3 failed — the 3 `disc_golden` failures the slim ISO is allowed (missing
  DRIVING audio files).

## What's left

- The zipline has no live level to check against (both disc wires are hand-over-hand); its fall-speed
  consumer mapping is tagged [S] in the code.
- The wire's third-person camera: the grab records `cam_mode = 0xB` on Bond, but the viewer still renders
  first-person; the renderer work is out of this task's scope.
- The jump-grab's icon context (+0x95F == 3) is not ported; the collision grab (walk into the sphere) and
  the use key cover the real paths, and the tests exercise those.
- The hand-reach correction's exact MMI sign order is reconstructed [S]; its effect is erased by the grab
  anim's own root motion as it plays.
- On 07000002 the wire's ends sit inside trigger volumes (a cut-scene trigger disables Bond at the B
  end); the test grabs mid-wire. Whatever script is supposed to run there is the script layer's business.
