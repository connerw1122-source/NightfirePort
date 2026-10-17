# level-0a-complete — Tower B (0700000a) end-to-end play

Task 710b, branch `s/level-0a-complete`, 2026-10-17.

## What's done

**The security keyboard (objective 5's writer) now completes through the REAL input path** — the
session's main result. The prior sessions' "the keyboard SP sits at the placement origin, the
game's own use ray has no target either, the activation writer stays a live check" premise was
stale:

- The SP's **mode-64 preload** (`SP_CreateScriptPlayer` 0x1db9a8's load beat + `SP_LoadScript`
  0x1dbc08's mode dispatch [M]) runs script 0x06000597's t=0 Entity op once at level load. The
  keyboard prop — glist hash 0x020005E1, created by `Script_EntityStart` 0x1d8d78 with
  `obj+0xE4` = the owner SP [M] — therefore exists from level load with its **baked-absolute**
  model sphere at (−1.54, 1.33, 8.48) r 0.66 (`Control_BuildWorldSph` 0x133070 multiplies the
  model's bounds by the identity entity matrix [M]; the same shape that makes the 09 elevator
  panel and the M.Hatch indicator resolvable — SP #681's entity resolves to (21.78, 2.54, 19.84),
  the hatch's own position, in the same probe).
- The cab's east wall (a real collision wall at x ≈ −2.0 spanning z 8.5–12.5, [D] probe segments
  through the port's collision world) stops the direct east walk; Bond wedges at
  (−3.00, 1.10, 9.68) rounding its south end — and **that wedge is inside the E pick sphere**:
  head+look sits ≈0.94 from the entity centre (the pick is a 1.0-radius sphere at head+look vs
  the entity sphere [M 0x1a5170]).
- `keyboard_0700000a_live.rs` (new) pins it end-to-end: walk out of the cab → wedge → **pulse**
  the use key aimed at the entity → **ch 0x3A on** [M `SP_Activate` 0x1dc338's `ext+0x2C = 1` +
  `SP_Update` 0x1dbda8 state 1's play-and-toggle]. Pulses, not holds: every dispatch toggles the
  out channel [M 0x1dbda8's bottom write], so a held key oscillates it — the test taps like the
  game's player does and stops once the channel reads on. Sister: on the old tree the test fails
  (no real E path was ever exercised; the walkreal drove the channel by hand).

The mechanism needed **no code change** — the elevator-buttons-09 machinery already in `nf-game`
handles it. The prior session missed it because the census note tracked the SP's placement
transform (the origin) and not the preload's entity.

## How it was verified

- `keyboard_0700000a_live` 1/0 (new), `interactables_0700000a_live` 7/0, `walk_0700000a_live`
  3/0, `walkreal_0700000a_live` 1/0 — all green on the final tree, `--test-threads=1`, release.
- The probe session (scratch, not committed) measured: the SP's primed state (state 1, script
  loaded, entity target list), the collision segments around the lobby, the wedge positions, the
  use-sphere distances, and the E → channel writes quoted above.

## What's left (the honest list)

1. **The SP-write → AND-mux follow (open inconsistency).** After the SP's own
   `ch.set_script(0x3A, 1)` the AND mux #741 (out 0x49 = AND[0x3A] [D triggers dump]) did **not**
   write 0x49 within 6 waited ticks in the probe, while the *identical* write through
   `set_channel` fires it (the walkreal's long-standing assertion). ch 0x3A itself reads on after
   the E. Something in the SP-write → mux-tick ordering needs one focused look; until then the
   walkreal keeps the channel drive (commented, pointing at the new test) and the route stays
   green. Live check for the main agent: gdbwatch `switch_channels[0x3A]` (0x26fc90+58) and
   `switch_channels[0x49]` (+73) on a real run while pressing E at the security keyboard —
   if the game's own mux follows the SP's write, the port's gap is real and small.
2. **The Q-Worm PC cubicle (objective 4's install) — unchanged documented blocker.** This
   session re-measured it: Bond wedges at (−16.19, 1.11, −30.42), 1.87 from the PC, and the
   standing eye-ray still clips the cubicle partition at t 0.36 (the block point (−16.56, 1.53,
   −31.05) is under the partition's 1.75 top [D probe]). The install keeps the walkreal's
   documented channel drive. The carried live check stands: does the real game let Bond inside
   the Q-Worm PC's cubicle (then the port's collision for that office cell is the bug), or does
   the game's bullet pass ignore that partition (then the bullet filter is)?
3. **The full route re-walk with the real keyboard E.** The session's window went to the
   keyboard mechanism and the probes; the walkreal's route (spawn → PC → keyboard → the SW
   office door → the hatch → 0700000b) is unchanged and green. Wiring the real E into the
   walkreal end-to-end (instead of the channel drive) rides on item 1.
4. **The ch-0x30 door (room-L_door2) has no writer in the level's data** [D] — unchanged; the
   walkreal drives it, documented. The MusicTrigger lead died for good this session:
   `MusicTrigger_Update` only READS `switch_channels[data+2]` and fires `Music_Event(p2, 2)` for
   30 ticks — it never writes a channel [M 0x1b31a0].
5. Objectives 3 (arrival plate, ch 0x47) and the exit (the hatch E with ch 0x4A →
   `level_exit = 0700000b` [M 0x1b2318]) are pinned by the walkreal, which also asserts the
   keyboard-lasers switch's inertness and the voice plate's lock state. Objective 6 completes on
   0700000b's own arrival plate (next level's business).
6. The full `tools/test/suite.py` did not run in this window either (the sandbox kills
   background processes between tool calls — same constraint the last two sessions recorded);
   the touched-crates runs above are the verification.

## Route (current, all through `Sim::step_bond` inputs)

Spawn (−3.93, 0, 10.40) → the arrival plate (obj 3) → out of the cab south → the lobby south
band → the west hall → the Q-Worm PC (documented blocker) → back to the security keyboard
(real E this session, ch 0x3A; mux item 1 open) → the SW office (the ch-0x30 door, driven) →
the hatch E → `level_exit = 0x0700000b`.

## Usage

not visible to me (no token, cost or context-window figures are shown in this environment).
