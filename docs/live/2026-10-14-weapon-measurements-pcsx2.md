# Weapon measurements in PCSX2 (2026-10-14, main agent)

Live measurements of the retail game (NTSC-U) in PCSX2 through the real pad input (the keyboard-mapped copy), read
with PINE. They are the ground truth for the playtest 1014-1725 tasks (109a, 109b, 109e). Tools: scratchpad
`projectile2.py` (new DynamicObjList objects + per-frame position), `firetiming.py` (the fire action float, the clip,
new objects by frame), `burst.ps1` (screen frames at fixed intervals). The captures carry the user's CRT filter
(presence/placement only, not colour).

## Memory facts found on the way [L]

- The DynamicObjList head pointer is at **0x270594** in a running level (newest first, next at obj+0x14; ~500
  objects on 07000005). 0x2705a0 (docs/fields/obj-tag.md) reads 0 in these states.
- The fire action float is PlayerSetting 0x2a38c8 + 0x14 + 9·4 = **0x2a3900** (non-zero while fire is held).
- The game writes the clip on the **press frame** for both the rocket and the frag (the frag's count drops before
  the throw animation).
- A twin record shares its base's clip slot: the unguided 0x2d's rounds are counted on the base 0x2c's slot.
- 07000005 holds no frag grenades: equipping 0x34 there is refused and the game falls back to the P2K.

## The AT-420 Sentinel (07000005, castle approach)

- **Unguided 0x2d: the rocket flies at 0.3367 units per frame**, constant over its flight (BU+0xF8 reads
  0.33666, and the measured per-frame displacement is the same 0.3367; 76 units in ~227 frames). That is
  0.25 × 1.3467 — the `+0x6C & 0x200` quarter-speed arm applies to a base speed of ~1.3467, not to 0.25. The port's
  0.0625 is 5.4× too slow (the user: "The rockets are way too slow").
- **Guided 0x2c: the view becomes the rocket.** For ~2 s after the press the whole screen is the rocket's
  forward view with a **red tint** over the frame; no view model, no crosshair; the HUD's ammo pane stays
  ("3-50 Guided"). The camera flies over the castle wall toward the keep. (The user: "The rocket launcher doesn't do
  the first person" in the port.)

## The frag grenade 0x34 (07000015, the start corridor)

Frames 70 ms apart from the press (the count 9 → 8 shows on the press frame):

1. 00: the grenade in the right hand (idle).
2. 01–02: the off hand reaches over and pulls the pin (the hand sweeps across the lower left).
3. 03: the grenade raised.
4. 04–06: the throw; the hands leave the screen.
5. 07: the grenade visible in flight down the corridor.
6. 09: the next grenade comes up in the hand.

**The grenade leaves the hand ≈ 0.3 s (≈ 18–21 game frames) after the press**, after the pin pull. (The user, three
times: the port releases it "while I'm still pulling the pin".)

## The stunner 0x4a

Not measured yet. A first attempt equipped it by poking the held-weapon byte; the HUD read "Stunner" but the
watch was not in Bond's hands (the user caught it), so that capture was invalid and was removed. The stunner must be
selected through the watch's gadget cycle (D-pad) before measuring.
