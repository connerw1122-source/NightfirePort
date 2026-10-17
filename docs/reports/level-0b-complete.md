# level-0b-complete — 0700000b (Tower C: the executive floor and the roof)

Branch `s/level-0b-complete` (from origin/master 92db2533). The walk test
`crates/nf-view/tests/walkreal_0700000b_live.rs` is un-ignored and green (2 tests), driven through
the real input path (`Sim::step_bond` inputs, the use key, the Q-Worm gadget shot, the real pickup
touch) from the balcony spawn to the mission-complete latch and the 0x0730000b driving hand-off
[M 0x182c60:85-122].

## What this session resolved

**Objective 10's writer and the roof route are now real.** The doc's biggest open question — what
writes ch 0x59 (obj10) and ch 0x57 — resolved to the roof cut-scene script **060004FA** (pack
0790000b, played by Script Player Wedge **#177**, out ch 0x2D, mask 1) [D pack dump]:

- stream 6: **ch 0x57 = 1** at tick 0x991 (the fall-fail web's AND #324 input [D]);
- stream 13: **ch 0x59 = 1** at 0x991 (obj10 "Escape to the roof");
- stream 14: **ch 0x2A = 1** at 0x990 — the roof **helicopter's gate** (#328 Helicopter-Body
  param 6 = 0x2A [D]; flight = param 5 = 060008BE, 299 ticks, then out **ch 0x66** = param 7
  [M Copter_Update 0x17dbb8:249-268]; the countdown pulse ch 100/param 8 is unarmed because
  param 9 = 0 [D #328] — asserted never to leak mission-complete);
- stream 12: ch 0x77 pulses 0→1 (ticks 0x795→0x990); stream 15 clears ch 0x76 at 0x990 [D];
- **event 6 at stream 10's tick 0x98f re-places Bond** at key 448 **[−14.71, 58.00, −2.06]** —
  the helipad deck, inside lift_stop_trigger #245's volume (x −18.9..−9.1, y 56.8..63.8,
  z −1.9..2.2 [D trig dump]).

So the game's lift ride up IS this cut-scene (PlayerDisable → camera → the lift-doors entity
2000668 → the re-place), not a physical cab carry on this leg. The old walkthrough's "drive
ch 0x59" stand-in is gone: the test now fires only the cut-scene's start gate and everything
after — the skip-less 2450-tick play, the re-place, the channel tail, the helicopter flight —
runs through the port's real SP machinery (already implemented: event 4/6/7 and the mask-1
out-channel gate [M SP_Update 0x1dbda8 tail + SP_Hit 0x1dc470 bit 1]).

**The deck walk is real.** After the re-place, Bond WALKS the roof east to the parachute
(+471 ticks, arrived (2.05, 58.07, 35.83)) — the old transit to the pickup side is gone. The
parachute pickup (ch 0x3F), the jump into the outside-top band (ch 0x40), SEQ #281 → ch 0x41 →
mux #304 → ch 0x5A + ch 100, and the latch past the band's off-write are unchanged and green.

## Objectives — how each completes

| # | Objective | Completes | Evidence |
|---|---|---|---|
| 6 | Find door to outer balcony | ch 0x4B by TOUCH_ONCE #176 at the spawn | [D trig dump], test-asserted |
| 7 | Install Q-Worm on Mayhew's computer | the Q-Worm gadget shot (weapon 0x58) at SCOMP_QWORM_C #297 (param 1 = 76) | [M PCQWorm_Update 0x1aee20], test-asserted through the real trigger path |
| 8 | Locate the Secure Terminal Room | ch 0x4D by TOUCH_ONCE #163 | [D trig dump], test-asserted |
| 9 | Retrieve the NightFire files | OR #295 (ch 0x4E = OR(ch 0x08, ch 0x06)); ch 0x08 ← mux #294 ← ch 0x54 | [D trig dump]; **ch 0x54/0x06's writer still open** — the test drives ch 0x08, documented |
| 10 | Escape to the roof | **ch 0x59 by the roof cut-scene 060004FA at its tick 0x991** | [D pack 0790000b], test-asserted through the real SP play |
| 11 | Acquire parachute then jump | SEQ #281(ch 0x3F pickup touch, ch 0x40 volume) → ch 0x41 → mux #304 → ch 0x5A + ch 100 | [D trig dump + M 0x182c60], test-asserted (real pickup touch, real fall) |

## The route as played

spawn plates → the arrival cut-scene tail (event 6 re-places Bond; waited out) → the documented
balcony→exec-floor transit (the port's cab-carry gap, below) → the exec-floor transits to the
desk/lock/terminal (the boardroom furniture row wedges the walked line; the interactions stay
real) → the terminal plate (obj8) → the lock's ch 9 driven (the table blocks the walked E; the
bridge/terminal script chain ch 10→11 then runs for real [D nis]) → the Q-Worm shot (obj7) →
the files drive (obj9, open writer) → **the roof cut-scene gate** → the real ride + re-place →
the deck walk → the parachute touch → the jump → mission complete → the latch asserts.

## What the walk still cannot do (exact reasons)

1. **The balcony→exec-floor leg** — the cab cycle script 06000048 (#232's first script, looped:
   an invisible entity 2000367 cycles y 11.0 → 48.44 → 11.0 over 1239 ticks [D nis dump of
   0700000b.bin]) would carry Bond only if the port had a moving-platform floor; the port's
   collision is static (`CollisionWorld`), and the stairwell floors surveyed last session are
   separate geometry, not steps. Documented transit; the fix is a moving-platform floor in the
   player's floor queries.
2. **The boardroom furniture row** (x 4-6, z 22-20) blocks the exec-floor walked lines at every
   height [D probes]; the lock's E and the transits around it stay stand-ins. The real player
   rounds the table; the port's straight-line walker needs a detour route or the collision
   probe re-checked.
3. **ch 0x2D's writer** (#177's start gate, its own out channel) — swept the trigger dump, all
   20 level scripts, the 3 pack scripts, the copter scripts 060008BE/8BF, every placement's
   channel/lock params and drone-props: no Event-0xD write or object param carries 0x2D, 0x54
   or 0x06 [D]. Candidates left for the next session: a game-side control func (the Monitor's
   sensor-view channel class [M 0x1afcc8] writes arbitrary sensor param-8 channels), or a
   channel the 0a→0b carry arms (a fresh-boot PCSX2 watch on switch_channels[45]/[84]/[6]
   would settle it).
4. **Observation (not pinned):** after the re-place lands Bond inside #245's volume, the port's
   touch did not raise ch 0x75 in the walkthrough window (the focused test proves the re-place
   itself works). #245 arms the looped cab cycle — presentation on this leg — but the miss is
   worth a look at the touch's cel scoping at y 58.

## Tests and what they pin

- `walk_balcony_to_the_roof_jump` — the full route above; every assertion carries its [D]/[M]
  tag; the only driven values are the documented transits, ch 0x08 (obj9's open writer) and the
  cut-scene gate ch 0x2D.
- `the_roof_cutscene_060004fa_writes_ch59_ch57_and_arms_the_helicopter_gate` (new, focused) —
  the SP's identity (out 45, no second script, the pack in the lib), the pre-gate baseline
  (the sister: no tail channel rises while the gate is off — fails on any tree that boots the
  script unprompted or drives the channels), then the real play: ch 0x59/0x57/0x2A/0x66, no
  ch 100 leak, Bond re-placed at the deck key.
- `common/walker_b2.rs`: `lenient` made `pub` (additive) so the deck leg can bail to the
  documented transit instead of panicking.

Ran: `cargo test --offline --release -p nf-view --test walkreal_0700000b_live -- --test-threads=1`
→ 2 passed (NF_ISO slim ISO). The full `tools/test/suite.py` run was started in the background
but the sandbox OOM-kills concurrent heavy tests (a `CRASH walkreal_probe13 (exit -9)` while my
runs shared RAM); the suite did not complete in the window — the crates touched by this task are
the two test binaries above (test-only changes; no lib code changed).

## Live checks for the main agent

1. Fresh-boot PCSX2 (no save-state resume), level 0700000b: watch `switch_channels[45]`
   (0x2D), `[84]` (0x54) and `[6]` — expect 45 to rise when Bond E-uses the lift at the exec
   floor lobby (before the roof cut-scene), 84/6 around the secure-terminal use. Access:
   switch_channels base 0x2a37c0-ish per the existing fieldprobe notes.
2. On the roof after the cut-scene: confirm the real game raises ch 0x75 (#245) when Bond
   lands at [−14.71, 58.00, −2.06] — the port misses it (observation above).
3. Render checks (no drawing done here): `--bond -14.7,59.2,-2.1,2.6,0` should show the
   helipad deck with the hovering helicopter east; `--bond 2.0,59.5,36.0,0.0,0.3` the
   parachute at (2.53, 58.45, 42.34) with the helicopter body behind it.

## Usage

not visible to me
