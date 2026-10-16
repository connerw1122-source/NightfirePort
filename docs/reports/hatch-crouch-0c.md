# hatch-crouch-0c — 0700000c: dropping through the hatch and crouching through

Playtests 1012-1723, 1013-1758, 1014-1725 (0700000c): after the hatch opens, "he still can't get
through because he can't crouch through" — Bond blocked and soft-locked under the hatch. Branch
`s/hatch-crouch-0c` (from origin/master ad761677).

## The find: the grounded rule was missing its pick-contact arm — FIXED

**Cause [M].** The game's grounded flag (`obj+0x60 |= 8`) is set by `Player_FeetOnPoint`
0x194188:176-185 when **the capsule's pick hit list is non-empty (`HITTEST+0x9c != 0`) OR the
floor ray lands within `cc + 0.1`** (the tolerance test at 0x194188:106, `hit+0xc − 0.1 <= cc`);
the history shift register takes the same flag (`Player_CollisionHandler` 0x1949a8:78-82). The
port grounded only off the floor ray / a `walkable` (ny > 0.7) push surface — the first arm was
missing. Riding the balcony's south pier (a ~58° ramp face, 14.61 → 16.49 over z −121..−122,
[D] probes in `crates/nf-view/tests/hatch_crouch_probe.rs`: the floor rises 0.08 per 0.05 z with a
0.25 lip at z −122.0) the capsule push lifts the crouched Bond past the ray tolerance (crouch rest
0.646 = floor + cc 0.55 + the capsule's −0.10 bottom vs the 0.55+0.1 gate — grounded by 0.004 on
flat ground), so he bounced ungrounded at the pier base forever (y 15.31..15.62, never past
z −120.85, `vel_y` accumulating to −5): exactly the playtest's stuck-under-the-hatch soft-lock.
The fix (`crates/nf-game/src/player.rs`, the walk `physics_tick`): `grounded = push != 0 ||
floor-within-tolerance` — a nonzero accumulated push is the port's pick-hit signal (every contact
within the radius resolves to `r − dist` of push). With it Bond rides the pier face grounded and
climbs (y 15.26 → 16.55, z −121.43) instead of bouncing.

**The capsule itself needed no change** [M `Player_Collision` 0x194df0]: crouch (case 3, obj+0xF6
= 4) is radius 0x3f0ccccd = 0.55, A = pos + up·(0.1 − (cc − 0.55)), B = pos + up·(0.45 − cc) → top
= feet + 1.20; standing (case 0/6) is radius 0x3e8ccccd = 0.275, A = pos + 0.55, B = pos + 0.275 −
cc → the same [0, feet+1.86] envelope. The port's capsules match both arms exactly; the "crouch
height in the player tables" the task pointed at resolved to these per-substate constants (the
`CCModeCrouch*`/`LCModeCrouch` symbols at 0x2706d8+ are camera-collision modes, not player
heights). No crawl-space trigger exists: there is no creepwall on 0700000c [D kind-census, 101g]
and the climb substate is ladder-only [M `Player_CollLadder` 0x196118]; no trigger in the well
writes a crouch channel.

**The hatch's collision after it opens is correct in the port** [D probes, `probe_panel_collision`]:
with the hatch shut the opening is blocked at z −112.36 for y 15.3..17.2; open, the whole band
15.3..17.6 passes and the panel's collision sits low behind (hits at z −112.48/−114.77 at
y 14.76) — the panel moved with the sweep, matching the game's slide-into-the-slot (SS last key
(5.78, 14.76, −113.83) [D #138 key5]).

## The route, verified headless through the real input path

`crates/nf-view/tests/hatch_crouch_live.rs` (NF_ISO-gated, 4 passing):
1. `from_the_hatch_top_the_open_hole_drops_the_crouch_onto_the_balcony` — burn the four hinges,
   the hatch sweeps to its last key, crouch, hold forward off the duct edge: Bond drops through
   the open hole and lands GROUNDED on the balcony (root 14.5..15.5 over the 14.61–14.81 floor,
   x 4.5..7.0 under the opening).
2. `crouch_walking_the_balcony_south_reaches_the_far_walkway` — crouch-walking the balcony south
   (east half, x 11 — the pier ramp occupies x ≤ 9) crosses the well's south edge and reaches the
   far walkway (the 14.61 floor continues at z −126.5+ [D probes]) at z < −125.
3. `the_south_pier_ramp_ride_stays_grounded_and_climbs` — the fix's regression: the crouch rides
   the pier face grounded past z −121.3 / y 16.3 (the pre-fix bounce never left z −120.85 /
   y 15.62).
4. `standing_is_blocked_at_the_south_pier` — the sister: standing never crosses z −121.5 at the
   pier (the slim capsule gets no lift off the ~58° face and its 0.25 lip).

`fix_0c_vent2_live`'s `from_the_duct_top_the_hole_drops_bond_onto_the_balcony` sister bound was
updated (y < 16.5 → 16.6) with the reason: post-fix Bond rides the pier ramp instead of bouncing
at its base.

## Honest scope notes

- The pier ramp itself (x ≤ 9) is not walkable to its top by EITHER stance in the port post-fix
  (both stall at the lip, z −121.43): a 58° face exceeds what walking can ascend; the game's way
  past it (if any) is unproven — the balcony's east half (x 10–14) needs no pier and is the
  verified south route. The old "wedge = can't crouch through" theory (fix-0c-vent2 item 3) is
  resolved as the ungrounded bounce, now fixed.
- The winch "elevators" theory from fix-0c-vent2 is DEAD [D]: #351/#364/#377 are thin hanging
  columns (0.39×6.02×0.44) whose SS keys contain no translation — contact-sway props (mask 2 =
  contact kind 5 per `SS_Operate` 0x1a9df8), not rideable platforms. The static descent (balcony
  SE → the east deck 12.14 → `obj_stor2_staircase01` #359 → ground 6.27) is mapped and unprobed
  on the deck underside.
- The ascent to the duct (fix-0c-hatch's open leg 1) remains unidentified (live check filed
  there); this task's tests start from the duct top per the task text.

## What ran

- Crates touched: nf-game (the one grounded-rule change), nf-view (tests). Run before push:
  `nf-game --lib` 237 passed; nf-view binaries `hatch_crouch_live` 4, `fix_0c_vent2_live` 3,
  `fix_0c_hatch_live` 2 (+2 ignored from 101g's unmapped legs), `fix_level_0c_live` 4,
  `walk_0700000c_live` 4, `walkreal_0700000c_live` 18, `playtest_spots_live` 4 — all green.
- Full suite (`tools/test/suite.py`) on the PRE-FIX tree (branch start): **1729 passed, 6 failed,
  72 ignored (58 min)** — the 6 failures are the 3 allowed `disc_golden` tests plus
  `fix_0c_pylon_live` (2) and `probe_pylon_render` (1), which are GPU render tests that cannot
  run in this display-less sandbox (pre-existing; the same tree passed them on the main agent's
  machine per fix-0c-pylon's report). The post-fix full-suite re-run did not fit the window;
  the touched binaries above cover the change's blast radius (fall/jump/ledge paths live in
  `nf-game --lib` 237 and the walkreal/ledge sets).

## Live checks for the main agent

1. PCSX2, 0700000c, hinges burnt, on the balcony (spot `--bond 5.8,15.4,-116.0,3.14,0`): walk
   south into the pier ramp (z −121) — does the real game's Bond climb it (walk or jump), or is
   the east half the route? `gdbwatch.py` on obj+0x60 bit 0x8 (the grounded flag) while pressing
   forward: expect it to STAY set on the ramp contact (the rule this task ported).
2. The east descent: from the balcony SE drop ~2.5 to the east deck (12.14, x 10–15), then
   `obj_stor2_staircase01` #359 down to the ground 6.27 — confirm the game lets Bond drop there
   and the deck underside has no hidden blocker.

## Files

- `crates/nf-game/src/player.rs` — the grounded rule's pick-contact arm (+ the citation comment).
- `crates/nf-view/tests/hatch_crouch_live.rs` — new: the 4 traversal tests.
- `crates/nf-view/tests/hatch_crouch_probe.rs` — new: the kept geometry probes (ramp profile,
  rays, panel collision shut/open, east walk).
- `crates/nf-view/tests/fix_0c_vent2_live.rs` — one sister bound widened with the reason.
- `docs/WORKLOG.md`, `docs/levels/0700000c.md` — appended/updated.
- Shared files: none reordered; `docs/WORKLOG.md` append-only.
