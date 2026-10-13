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

## Status (fix-gadgets, 2026-10-13)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x192860 | `Player_SSGrapple` | **ported (logic)** | Substate 2: the fire latches the aim-ray point, the reel (0.05→0.5/tick SP) hangs Bond < 2.0 from it, the fire releases. The rope's visual and the swing momentum's restore are not modelled [S]. `gadget_grapple_live`. |
| 0x195d10 | `Player_InitGrapple` | **stand-in** | The rope object (mesh 0x2000138) is not created (the renderer's item). |
| 0x192728 | `Player_GrappleSetRope` | **stand-in** | The rope endpoints are not drawn. |
| 0x195da0 | `Player_SetGrapplePoint` | **ported (logic)** | The enter path in scene.rs (the aim ray vs the point's sphere; the exact selection cone [S]). |
| 0x193098 | `Player_SSScanMode` | **partial** | The decryptor's lock burn works through the beam path (`runs_beam` — 0x4E/0x4F in `Lock_Update` 0x1ae3d8's contact scan); the scan-mode camera/UI is not ported. |
| 0x1a65d8 | `Player_LaserPointer` | **ported (logic)** | The beam ray per tick while the laser family is out; the lock contact via the aim line's closest approach [S — the 08 safe door's collision plane sits ~0.9 in front of the bolt centres, so the world-capped beam can never touch them]. `gadget_laser_live`. The +0x68 bit-3 gate belongs to the GUNS' laser sights (0x06/0x07/0x08/0x09/0x0f/0x17/0x18), not the gadgets. |
| 0x1a4560 | `Draw_LaserBeam` | **render item** | The beam's endpoints are parked on `Sim::laser_beam` for the renderer (the meshes 0x2000131/0x2000133). |
| 0x198298 | `Grapple_Create` | **ported** | The grapple points parse (placement 0x3A — this page's 0x3E was the WIRE's type; corrected). `drones.grapple_points`. |

The SELECTION is live: the weapon_data's +0x04 selectable flag is 1 for
0x4A/0x4C/0x4E/0x4F/0x50/0x51/0x52/0x53/0x54/0x56/0x57/0x58/0x59 [D ACTION.ELF weapon_data] — the
"Gadgets and non-weapons have 0" note in weapon.rs was wrong. The G key cycles gadgets (the game's
d-pad Up/Down [M Player_WeaponChange 0x1a6f80 mode 1]); the prev-slot pollution (a gadget→gadget hop
writing +0x930) is fixed. The Q-Worm (0700000a) arrives from the story scripts (no placed pickup —
the mission layer owns the grant); the camera (0700000c's 0x52) and the briefcase (07000012's 0x52)
pick up and equip; their fire-side functions are the mission layer's (not ported).

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
