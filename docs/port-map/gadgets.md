# Port map: gadgets (0x4A–0x5E)

The gadgets are weapons 0x4A..0x5E that trigger player substates instead of firing bullets. Each is a full
subsystem: the grapple moves Bond through the air on a rope; the decryptor opens a scan-mode UI; the laser
burns through locks; the watch fires a laser or taser. **None are ported.** Until they are, they are removed
from the selection cycle (the user's instruction: "if you can't finish one, remove it from selection rather
than ship it broken").

Tags: [M] decompile, [D] disc, [L] snapshot, [S] suggested.

## Identification [L ee_start_05.bin + ELF strings]

| Id | Gadget | Evidence |
|---|---|---|
| 0x4A/0x4C | Q-Claw (grapple) | `Player_SSGrapple` 0x192860, `Player_InitGrapple` 0x195d10, `Player_GrappleSetRope` 0x192728; sounds SFX_GADGET_GRAPPLE_* |
| 0x4E/0x4F | Q-Decryptor | `Player_SSScanMode` 0x193098; sounds SFX_GADGET_DECRYPTOR_* |
| 0x50 | Q-Laser (watch laser) | `Player_LaserPointer` 0x1a65d8, `Draw_LaserBeam` 0x1a4560; sounds SFX_GADGET_WATCH_LASER_* |
| 0x54 | Laser Watch (scoped) | `weapon_data[0x54]+0x68` flag 0x40 (scoped); sounds SFX_GADGET_WATCH_LASER_* / WATCH_TASER |

## Status: all NOT STARTED

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x192860 | `Player_SSGrapple` | **not started** | The grapple substate (substate 2): fires the grapple, attaches to a surface, swings Bond on the rope. 592 bytes. |
| 0x195d10 | `Player_InitGrapple` | **not started** | Initialises the grapple rope object. 144 bytes. |
| 0x192728 | `Player_GrappleSetRope` | **not started** | Sets the rope's endpoints. |
| 0x195da0 | `Player_SetGrapplePoint` | **not started** | Sets the grapple's target point. |
| 0x193098 | `Player_SSScanMode` | **not started** | The decryptor substate (substate 5): opens the scan-mode camera, decodes digits. 1020 bytes. |
| 0x1a65d8 | `Player_LaserPointer` | **not started** | The laser pointer: casts a ray, draws the beam, burns through locks. 1016 bytes. |
| 0x1a4560 | `Draw_LaserBeam` | **not started** | Draws the laser beam effect. |
| 0x198298 | `Grapple_Create` | **not started** | Creates a grapple point placement (type 0x3E). |

## What each gadget does (from the decompile)

### Q-Claw (0x4A/0x4C) — `Player_SSGrapple` 0x192860
- On fire: `Player_InitGrapple` creates a rope object; `Player_GrappleSetRope` sets the endpoints from Bond's
  hand to the ray-hit point; Bond enters substate 2 (swinging).
- Each tick: Bond swings on the rope (pendulum physics), the rope is drawn as a line.
- Release: Bond detaches and enters walk substate.
- Sounds: SFX_GADGET_GRAPPLE_FIRE (0x600-ish), GRAPPLE_ATTACH, GRAPPLE_OPEN/CLOSE.

### Q-Decryptor (0x4E/0x4F) — `Player_SSScanMode` 0x193098
- On select: Bond enters substate 5 (scan mode). The camera changes to a scanning view.
- On fire: decodes a digit of a lock's code; when all digits are decoded, the lock opens.
- Sounds: SFX_GADGET_DECRYPTOR_DECODE_LOOP, DECODE_DIGIT, CODE_COMPLETE, DECRYPT, ABORT, BUTTON_PRESS.

### Q-Laser (0x50) — `Player_LaserPointer` 0x1a65d8
- On fire (held): casts a ray from Bond's head; `Draw_LaserBeam` draws the beam; if the ray hits a lock/object
  with flag 0x8, it burns through. The beam object (0x2000131 or 0x2000133) is positioned and scaled.
- `Collide_RayIntersect` finds the hit; `Mat_Align2Dir` orients the beam.
- Sounds: SFX_GADGET_WATCH_LASER_SHOOT, WATCH_LASER_BURN_LOOP.

### Laser Watch (0x54) — scoped weapon
- `weapon_data[0x54]+0x68` has flag 0x40 (scoped): selecting it enters aim mode (zoom).
- The watch fires a laser or taser (the variant at 0x55 → 0x54 toggle in `Player_Weapon`).
- Sounds: SFX_GADGET_WATCH_LASER_SHOOT, WATCH_TASER_SHOOT.

## Why they're removed from selection

Each gadget triggers a player substate (`Player_ChangeSubState`) that is not ported. Selecting a gadget would
put Bond into a broken state (no grapple movement, no scan mode, no laser beam). The cycle's gadget mode
(`CycleMode::Gadgets`) is kept in the code but the G key is disabled until at least one gadget is ported.

## Counts

- **Ported**: 0
- **Partial**: 0
- **Stub**: 0
- **Not started**: 8 (all functions above)
