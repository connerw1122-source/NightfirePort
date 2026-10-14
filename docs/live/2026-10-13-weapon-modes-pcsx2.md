# Weapon fire modes in the real game (PCSX2, 2026-10-13)

Measured by the main agent in a multiplayer Arena match. The setup:
- **Match:** Skyrail, one Snow Guard bot, weapon set Normal, PCSX2 2.x with the keyboard-mapped pad.
- **Weapons:** given through PINE: the owned byte `BL+0x1BA+w*0xC`, the clip `BL+0x1B8+w*0xC` and the switch request `anim+0x63`.
- **Player pointers in MP:** `*(0x2D88E0)` = the player object; `obj+0xDC` = anim, `obj+0xE0` = BLData. These are the same offsets as single player.

Screenshots stayed in the session (no game images in the repo). The numbers below are from those frames.

## AT-420 Sentinel (0x2C / 0x2D): the guided missile

- **Draw:** the HUD's fire-mode lane reads **"Guided"** for 0x2C.
- **Firing:** the view *becomes the rocket's*. A red-tinted full-screen camera rides the missile, with a red HUD overlay:
  - a centre ring;
  - horizontal bracket lines left and right of it, angled lines below;
  - faint horizontal scanlines.
- **Normal HUD:** the health bar and the crosshair are gone while it flies. The kill counters, clock, radar and the ammo/mode lanes stay.
- **Steering:** the right stick (the look input) steers the missile while it flies. Holding the turn input curved it into the trench wall within about a second.
- **On impact:** one or two frames of **red static** (dense red horizontal noise lines over the overlay). Then the view snaps back to Bond's first-person view, with the explosion visible where the missile hit.
- **Not measured:** the speed, the time limit, and whether fire detonates early. The narrow trench ended every flight in under ~1 s. Measure these from the decompile (100f).

## Militek MGL (0x2A / 0x2B): impact vs delay

- **The two modes are twin records, not a mode byte.**
  - 0x2A reads **"Grenade"**; 0x2B reads **"Delay"**.
  - The switch needed the twin to have rounds (`Player_WeaponHasAmmo`): with 0x2B empty the press did nothing; with rounds it switched (anim+0x62/0x63 = 0x2B).
- **0x2A "Grenade": impact detonation.** Fired at a rock wall ~10 units away, it exploded **on contact**, within about 0.3 s of the shot (the next captured frame).
- **0x2B "Delay": timed fuse.** Fired at the same wall, the round **bounced off it**, dropped and rolled left, and exploded **about 3–4 s** after the shot (the match clock went 07:09 → 07:05; the capture timing is coarse, so take the fuse from the decompile).
- The user's report "the MGL is always delayed" matches: the port lacks the impact mode.

## The mode button

With the pad's default "Classic Bond" layout, the circle button did NOT switch the MGL. **Square** (the Controls tab's "Alt. Fire") did. fix-weapon-modes documented circle (action 12) from style 0's table. Check which action the Classic Bond layout puts on square, and which button really toggles modes in each layout. The port's B key stands for the mode button: keep B, but make it drive whatever the game's mode action is.
