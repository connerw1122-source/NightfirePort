# fix-0c-vent2 — 0700000c: through the display vent (task 103c)

The task: finish the vent traversal 101g-fix-0c-hatch left open — from the inner ledge down to the
display floor (~6.3–7.3 at (5.4, −113)), with a test from `--bond 5.8,15.8,-107.5,3.14,0` with the
four latches burned, the fit-through requirement (crouch / creep volume / clearance), and the exact
keys for the next playtest. Branch `s/fix-0c-vent2` (the task footer said `s/0c-vent2`; the header's
name is used, as in 101g).

**Status: partial — one real port blocker found and fixed (channel 0), the route mapped to the
last two unknowns, the full end-to-end walk is NOT green yet. Details and what's left below.**

## Done: channel 0 was a dead channel in the port — FIXED

**The find.** The level doc's own open item said "the `obj_stor*_pulley` winches #351/#364/#377
(SSs on ch 0 — no driver)" and calls the vent route "the way down". I dumped the three winch SSs
from the sim: placement 351/364/377, **channel 0**, rate 75, collision hashes 802d0092/9f/ac [D].
The game's `SS_Update` 0x1a9948:20-23 reads `switch_channels[ch]` and self-writes it while the
channel *value* is 0 — for **any** channel word including 0 (checked in
`work/decomp/ACTION.ELF/001a9948_SS_Update__FP7obj_tag.c`); the trigger/AND writers copy Out into
the array unconditionally [M 0x1b1a78]. The port's `Channels::set` early-returned on `c == 0` and
`Channels::on` hardcoded `c != 0` (crates/nf-game/src/triggers.rs), and `SsList::tick` skipped any
SS with `s.channel != 0` (crates/nf-game/src/ss.rs) — so a ch-0 SS could **never** run, and the
winches (the mapped way down off the balcony) sat frozen at key 0.

**Fix** (two lines + comments): ch 0 is a real channel — `set`/`on` accept it, `SsList::tick` gates
on the channel *value* only. Nothing else changed; the value gate is untouched, so objects still
need a nonzero channel to act.

**Tests** (`crates/nf-view/tests/fix_0c_vent2_live.rs`, NF_ISO-gated):
- `channel_0_is_a_real_channel` — set(0,1) sticks, reads back, stamps `time`, and a zero write
  clears. **Failed before the fix** (set(0,·) was a no-op), passes now.
- `the_pulley_winches_sweep_once_channel_0_rises` — the three winch SSs exist with rate 75 [D];
  with ch 0 raised through the sim, all three playheads advance off key 0. **Failed before the
  fix**, passes now.
- `from_the_duct_top_the_hole_drops_bond_onto_the_balcony` — the legs 101g verified, as a
  regression: hatch fully open, crouch, walk off the duct edge through the open hole, land
  grounded on the balcony (14.61–14.81), keep walking south along it.
- Neighbours green: `fix_0c_hatch_live` (2), `fix_level_0c_live` (4), `walk_0700000c_live` (4),
  `nf-game --lib` 143 unit tests.

## The route, mapped end to end (this session's probes, kept in `crates/nf-view/tests/probe_vent.rs`)

Collision-world maps over the whole vent/display area [D probes through the sim's world]:

- The walkway ramps 12.25 → 14.01 and **ends at a face at z ≈ −110.0** spanning y 14.01..15.0+ at
  every probed x (3.0..8.5). Behind it: side ledges at 15.16 (x ≤ ~4.4 and x ≥ ~7.6) and the duct
  ramp 17.17 → 17.50 (x 4.5..7.0) rising to the wall plane.
- **There is no wall at x 3.0..4.4 on the wall plane z −112.25** (rays at y 14.2..16.4 from
  z −110.8 to −115.8 all pass): the vent wall spans only the opening's x range. The side ledges
  connect around it onto the inner ledge.
- **The inner ledge is a big balcony**: top 14.61–14.81, continuous x −2..14, z −112.5..−121.5,
  **no holes, no ceilings up to y 26** (nothing to crouch under on it). A raised strip (14.72–14.81)
  sits under the wall hole (x 5–7, z −112.5..−114.5) where the drop lands.
- The lower walkway (11.27) runs under the balcony's north half (z −112.4..−115.7, x −2..14) —
  the level doc's 11.84 lower walkway is the same structure in front of the wall (11.84 at
  z −111..−112 under the duct).
- The ground (6.27) below, with a rising ramp 6.5 → 7.7 southward at the south-east
  (x 10..12, z −121.5..−123) below the balcony's south-east edge; the space under the lower
  walkway at y 7.0 is open from the south at x 7..9. **No 7.3 floor exists at (5.4, −113) in the
  static world** — the task's "display floor (5.4, 7.3, −113)" reads as Bond's standing *root*
  over the 6.27 ground (root ≈ floor + 1.03), or the display-case structure at (5.4, 8.1–8.4,
  −110.2) whose jetpack (`Camera_Subject` #414) sits at 7.55 [D].

## What's left (the honest list)

1. **What drives the winches in the game.** The ch-0 path is now open, but I did not find the
   game's writer: `SS_Update`'s self-latch (`switch_channels[ch] = SS_Operate(...)` while the
   channel value is 0 [M 0x1a9948:20-23]) is not ported — `SS_Operate`'s return semantics are
   unread. Until then the test raises ch 0 directly. Next session: decompile `SS_Operate`
   (called at 0x1a9948:18), port the self-latch, then ride a winch platform down.
2. **The ascent to the duct is still unidentified** (101g's open leg 1): walkway 14.01 → side
   ledge 15.16 is a 1.15 face, ledge → duct 17.17+ is 2.0; the port's climb is ladder-only
   [M 0x196118] and 0c has no ladder/creepwall near the vent [D]. The no-wall gap at x 3.0..4.4
   (new fact above) may be the game's actual way in — walk the side ledge around the wall plane
   onto the balcony, skipping the duct entirely; needs a jump onto the 15.16 ledge first.
3. **A crouched wedge on the balcony**: Bond crouch-walking into the balcony's south wall
   (z ≈ −120.8) oscillates ungrounded (y 15.37..15.62 over floor 14.61) instead of settling at
   the crouch root 15.26 — the closest thing I found to the playtest's "can't crouch through".
   Strafing unwedges him. Needs a physics look (capsule push vs the 14.61 → 15.21 rising face).
4. **The full end-to-end test from `--bond 5.8,15.8,-107.5,3.14,0` is not green** — blocked by
   1–3. What IS green: the ch-0 fix tests and the hole-drop regression above.
5. The full suite did not fit the window (the ISO had to be re-downloaded — the sandbox's /tmp
   symlink target was a 39-byte stub; fresh download verified 369823203 B, extracted
   1061754880 B). **The next session must run it** before merging:
   `NF_ISO=$NF_ISO python3 tools/test/suite.py .` (background nohup dies between tool calls in
   this sandbox — run it in foreground chunks per the last three sessions' notes).

## The exact keys for the next playtest (what works today)

From the walkway in front of the display, with the four hinges burnt (the laser watch on each
latch): the hatch opens in ~0.8 s. The verified playable path today needs the duct: jump onto the
duct (not yet possible — live check below), **crouch** (toggle), hold **forward** — Bond drops
through the hole onto the inner ledge and walks the balcony south. The way down off the balcony
awaits the winch work (item 1) — in the real game, look for the pulley platforms cycling near the
balcony's south/south-east and step on.

## Live checks for the main agent

PCSX2 (0700000c, display area, hinges burnt):
1. **The winches**: watch the `obj_stor*_pulley` platforms (#351/#364/#377) — expected: they move
   (cycle or sweep) on their own or after a USE/step-on, WITHOUT any hatch/channel precondition.
   `gdbwatch.py` on `switch_channels[0]` (the address of `switch_channels` + 0) during play:
   expected — it becomes nonzero at some point; note what the player did right before.
2. **The ascent**: from the walkway (5.8, ~14.0, −109.5) facing −z: can Bond jump onto the side
   ledges (15.16) and walk around the wall plane at x < 4.5 onto the inner ledge (14.61)?
   Expected per the new no-wall fact: yes in the game's collision — confirm what the player must
   press (jump + forward, or a vault).
3. **The crouch wedge**: on the inner ledge at the south wall — does the real game's Bond crouch
   through anything low there (a gap under the wall face at z ≈ −121.3, top 15.21)?

nf-view (`--bond x,y,z,yaw,pitch`, hinges burnt):
- `--bond 5.8,18.4,-110.8,3.14,0` — on the duct ramp; crouch + forward drops through the hole
  onto the balcony (now a passing regression test).
- `--bond 5.8,15.4,-116.0,3.14,0` — on the balcony mid: the flat 14.61 floor, the lower walkway
  11.27 below its north edge, no ceiling.
- `--bond 10.8,15.2,-121.0,0.0,0` — the balcony's south-east: the ramp below (6.5 → 7.7) and the
  winch platforms' area.

## Files

- `crates/nf-game/src/triggers.rs` — ch 0 accepted by `Channels::set`/`on` (+ doc) [M 0x1a9948:20-23, 0x1b1a78].
- `crates/nf-game/src/ss.rs` — `SsList::tick` gates on the channel value only (+ doc).
- `crates/nf-view/tests/fix_0c_vent2_live.rs` — new: 3 tests (2 fail-first, 1 regression).
- `crates/nf-view/tests/probe_vent.rs` — new: the kept route/geometry probes.
- `docs/levels/0700000c.md` — the vent row updated.
- Shared files: none touched.
