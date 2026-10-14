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

## Every weapon (second pass, same match): draw, fire, square, fire

Each weapon was given with rounds for it and its next three records; square pressed between the two shots. The modes
are the HUD's fire-mode lane. "→" is the record square switched to (anim+0x62 after the press).

| Weapon (id) | Mode 1 | Mode 2 (square) | What was seen |
|---|---|---|---|
| Wolfram PP7 0x02 | Semi | Silenced (→0x03) | silencer on the barrel in mode 2 |
| Wolfram P2K 0x06 | Semi | Silenced (→0x07) | |
| Kowloon Type 40 0x0A | Semi | Burst (→0x0B) | burst = 3 rounds per press (7 → 4) |
| Kowloon Type 80 0x0C | | none | square did nothing |
| Raptor Magnum 0x0E | Semi | Semi (→0x0F) | the twin has the same label |
| Raptor Magnum .50 0x10 | | none | |
| Delta Repeater 0x11 (crossbow) | Single | none | |
| Storm M32 0x12 | Auto | Semi (→0x13) | |
| Deutsche M9K 0x14 | Silenced | Burst (→0x15) | burst = 3 rounds |
| SG5 Commando 0x16 | Burst | Semi (→0x17) | |
| AIMS-20 0x1A | Burst | **Grenade** (→0x1B) | the underbarrel launcher: lobs a grenade that explodes on impact |
| Frinesi Auto 12 0x1C | Pump | Auto (→0x1D) | |
| Winter Tactical Sniper 0x1E | Single | Armor Piercing (→0x1F) | |
| Winter Covert Sniper 0x24 | Single | Armor Piercing (→0x25) | |
| AT-600 Scorpion 0x2E | Heatseeker | Unguided (→0x2F) | rocket with a smoke trail, large fireball |
| Torpedo Launcher 0x30 | Guided | none | a slow glowing ball projectile; no rocket camera |
| Phoenix Samurai 0x32 | Overcharge | Beam (→0x33) | see below |
| Fragmentation Grenade 0x34 | Grenade | none | the throw animation, the grenade arcs out |
| Stun / Smoke 0x35 / 0x36 | | none | |
| Remote Mine 0x37 | (the mine) | detonator (→0x39) | square swaps to the detonator; firing it blows the mine |
| Laser Tripbomb 0x3A | | none | sticks to the wall, a red laser line across; set off by crossing it |
| Satchel Charge 0x3B | | (→0x47) | the device shows a green countdown (05, 04, ...) after it's thrown; the blast killed the player at close range |
| Q-Pen 0x41 | Q-Pen | none | fires a small explosive dart (an orange burst on the wall) |
| Golden Gun 0x42 | Single | none | |
| Korsakov K5 0x43 | Dart | none | |
| Oddjob's Hat 0x45 | | | the switch drew 0x42 instead: not available in this match |
| Flare Gun 0x46 | Flare Gun | none | a magenta flare streak with a large magenta glow |
| Stunner 0x4A, Laser 0x4E | | none | no visible beam in either shot; their charge numbers counted down |
| Grapple 0x50 | Grapple | none | |
| Phoenix Ronin 0x52 | Deploy | none | the deployable turret case |

### The Phoenix Samurai (0x32 / 0x33), tapped, held 1.5 s and held 3 s

- **0x32 "Overcharge":**
  - holding fire charges it: the HUD number climbs 0 → 100 and the gun's glow brightens;
  - releasing fires one bright cyan bolt, with a cyan blast where it lands;
  - a tap at a low charge fired a small bolt. The number then stays at 100 (fully charged) until the next shot.
- **0x33 "Beam":** a short cyan beam from the barrel. The number drains (100 → 81 → 0 while holding) and refills when
  released. The beam only shows while there's charge left.
