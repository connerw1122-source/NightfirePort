## Your task: `hatch-crouch-0c2` — land 680's crouch fix without breaking movement everywhere else

`s/hatch-crouch-0c` (680) was HELD on 2026-10-16. Its new grounded-rule "pick-contact arm" in `player.rs` breaks 6
movement tests on master:
- `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`;
- `fix_ladder_water_live::dismount_into_the_pond_lets_go_of_the_ladder`;
- `walk_0700004a_live::the_descent_plates_fire_and_the_objective_reveals`;
- `fix_pickups_trace_live::with_dev_god_and_infinite_ammo_every_raptor_drop_is_taken_and_the_spare_matches_the_trace`;
- `fix_input_kbd_trace::keyboard_trace_unchanged`;
- its own `hatch_crouch_live::from_the_hatch_top_the_open_hole_drops_the_crouch_onto_the_balcony`, which failed on
  the merged tree.

Several of these are PCSX2-measured traces, so the grounded rule must match the game everywhere, not just under the
0c hatch. Re-derive the pick-contact arm from the decompile (Player_Collide / the grounded test) [M], find what the
traces show that your arm gets wrong, and land a rule that passes all of them AND the 0c crouch. Also, don't commit
scratch probes (`hatch_crouch_probe.rs` was removed in the merge).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/hatch-crouch-0c2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T11:34Z

---
## Report (blocked, 2026-10-16T15:12Z)


---
## Report from the worker (blocked, 2026-10-16T15:12Z)

# hatch-crouch-0c2 — re-landing the grounded rule's pick-contact arm without breaking movement

Task: land 680's (`s/hatch-crouch-0c`) crouch fix on the merged tree. Its `push != 0` grounded arm
broke 6 movement tests on master; the rule must match the game everywhere, not just under the 0c
hatch. Branch `s/hatch-crouch-0c2` (merged `origin/master` `ac1fba78` at `4594943c`).

## Status: PARTIAL — the 0c crouch set is green and 3 of the 6 broken tests are fixed; 3 movement
traces still fail on the final tree (details below). Out of time (sandbox window).

## The find: the pick-contact bit is an up-push signal, not "any contact" — LANDED

**The game's grounded rule [M].** `Player_FeetOnPoint` 0x194188:176-185 grounds (`obj+0x60 |= 8`)
when `HITTEST+0x9c != 0` OR the floor ray lands within `cc + 0.1` (0x194188:106). `+0x9c` is a
**bitmask, not a hit count**: `Collide_Update` 0x1ea398:83 zeroes it per pick; `Collide_Intersect`
0x1ebaa8's tail (disasm 0x1ec138-0x1ec150) ORs each real contact's stack-copy flags into it
(`lhu/sp+0xec = the amemcpy'd HITTEST copy's +0x9c`, the copy at `sp+0x50`, `s0 = sp+0x50`);
`Intersect_CylGeom` 0x109da0 sets bit 0x1 at 0x10a72c-0x10a734 (`lhu 0x9c(s1); ori 1; sh 0x9c(s1)`).

**What sets the bit [M].** The bit block (decompile 0x109da0:471-516 + disasm-verified nesting):
`dot1 = axis·copy[+0x00] > 0.7`, then `dot2 = (copy+0x50 − (bottom + push))·axis < 0`, then the
normalize's `dot3 < 0.7` (implied by dot2). `copy+0x00` is the face normal, written per triangle
(0x109da0:298); `axis` = normalize(+0x20 top − +0x30 bottom) (`Player_Collision` 0x194df0 case
0/6/3: A = pos+0.275, B = pos+0.275−cc standing; A = 0.65−cc, B = 0.45−cc crouch — the port's
`capsule()` matches both); `copy+0x50` tracks the live bottom endpoint through the pick's DeltaP
adds (`Collide_Intersect` 0x1ebaa8:353-373 — REAL+0x50 += DeltaP per real hit), so
`R = copy+0x50 − (bottom + push) = −push_this_contact` and **`dot2 < 0` reads `push·axis > 0`: the
contact's push has an upward component**. A wall contact's push is horizontal (not > 0) and a
head-bump's is downward — neither grounds, which is exactly where 680's `push != 0` over-grounded.

**The landed rule** (`crates/nf-game/src/player.rs` both grounded sites, `world.rs`):
`grounded = walkable || pick || floor-ray-within-cc+0.1`, where `pick` = any capsule contact this
tick whose push has `push[1] > 0` (`capsule_tri_contact`'s `pick` flag; the new
`capsule_pick_contact_scoped`/`capsule_pick_contact` queries share the per-triangle body with
`capsule_push` — no signature changes, no duplicate resolution math). The floor-ray arm and
`walkable` are master's, untouched.

**The one deliberate deviation [S].** The bit block's outer `0.7 < dot1` gate, read as
`normal·axis > 0.7`, would refuse the 0700000c pier ramp (a ~58° face, normal·axis ≈ 0.53) that
the game demonstrably rides grounded (`the_south_pier_ramp_ride_stays_grounded_and_climbs`; at
that slope the skin-rest equilibrium root gap exceeds the ray's `cc + 0.1`, so the pick arm must
fire there). I ported the dot2 arm only. **Live check for the main agent:** PCSX2 0700000c,
`--bond 5.8,15.4,-116.0,3.14,0`, walk south into the pier ramp, `gdbwatch.py` on `obj+0x60` bit
0x8 — expect it to STAY set while riding; then, if a breakpoint on `Collide_Intersect`'s
+0x9c tail OR is feasible, log `uStack_84`'s value for the ramp contact — that settles whether
the game's dot1 operand fires on a 0.53 face (and what `copy+0x50`/`+0x00` really hold).

## The 0c crouch set — GREEN (4/4)

`crates/nf-view/tests/hatch_crouch_live.rs` restored from 680's branch (the revert had dropped
it; the scratch probe stayed out). Two adjustments on the merged tree, both documented:
1. The hatch-open observable re-pointed to the **held pose** (`interp` of the last key vs
   `matrix`) — grates-02's state-driven `SS_Update` wraps the playhead to t=0 while states 0/0x40
   hold the end pose; the old `t == 5.0` read is stale on master (the same re-point grates-02 made
   in `fix_0c_hatch_live`).
2. `fix_0c_vent2_live`'s south-walk bound widened 16.5 → 16.6 (680's documented re-point): post-fix
   Bond rides the pier ramp grounded instead of bouncing at its base (measured y ~16.55).
Also restored: the `docs/levels/0700000c.md` vent row. `fix_0c_vent2_live` 3/3 green,
`fix_0c_hatch_live` 2/2 green (+2 ignored), `fix_level_0c_live` 4/4.

## The 6 broken tests: 3 fixed, 3 still failing

| test | result |
|---|---|
| `hatch_crouch_live` (4) | PASS (above) |
| `fix_ladder_water_live::dismount_into_the_pond_lets_go_of_the_ladder` | PASS (4/4) — falling past the ladder/wall no longer grounds |
| `fix_input_kbd_trace::keyboard_trace_unchanged` | PASS — the input trace is bit-identical |
| `walk_0700004a_live::the_descent_plates_fire_and_the_objective_reveals` | **FAIL** — the shaft's first plate (trigger #277, y −9.87..−7.65) never fires in 120 ticks from `stand_at(3.0, −6.5, −3.4)`: Bond must drop ~1.15 into the plate band and doesn't (early leg; failed at 1.4 s) |
| `fix_pickups_trace_live::with_dev_god_and_infinite_ammo_every_raptor_drop_is_taken_and_the_spare_matches_the_trace` | **FAIL** — only 3 takes (weapon + 2×ammo) vs the killed count: Bond's fight pathing changed, later drops go untaken (fails at line 262, 1.6 s) |
| `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` | **FAIL** — the AI contact parks at the stairwell head (−20.6, 6.3, 14.6) with `bond d 12.2`: BOND trails 12.2 behind and the Lead wait never resolves (427 s timeout) |

**Next session's leads (in order):**
1. The three failures share a shape: Bond must go DOWN (the 0700004a shaft, the pickups fight's
   drop pickups on uneven ground, the 07000001 stairwell descent) and my up-push bit grounds him
   where master let him fall. The prime suspect is edge/nosing contacts: descending past a step
   nosing, `dist > 1e-6` and the escape vector `p − q` can carry a small POSITIVE y even though the
   capsule is falling past — the game's dot2 (`R = copy+0x50 − (bottom+push)`) may exclude them if
   `copy+0x50` is the **pick-start** bottom rather than the live one (then `dot2 < 0` reads
   "the contact below the pick-start bottom", which a fall-past graze fails because q sits above
   the already-fallen bottom). Try `q[1] < b_pre_slide[1]` (the bottom BEFORE this tick's pushes)
   as the pick predicate — it keeps the ramp ride (the contact is below the bottom there) and
   should release the three descents.
2. If that's not enough, gate the bit on `normal[1] > 0` (not 0.7 — the ramp's 0.53 must stay in)
   to exclude down-facing escape vectors.
3. The master baselines for the three were NOT re-measured in this window (the stash-baseline run
   hit the deadline); 680's report measured their pre-fix tree green on all six. The morning's
   walk-stutter merge is an independent suspect for `walkreal_07000001` — worth a master run first.

## What ran (this session, NF_ISO on the onfoot ISO)

- `nf-game --lib` via `cargo test --offline --release` builds; per-binary:
  - `hatch_crouch_live` 4/4, `fix_0c_vent2_live` 3/3, `fix_0c_hatch_live` 2/2 (+2 ign),
    `fix_ladder_water_live` 4/4, `fix_input_kbd_trace` 1/1;
  - `walk_0700004a_live` 0/1, `fix_pickups_trace_live` 1/2, `walkreal_07000001_live` 1/2 (the fails above).
- Full suite (`tools/test/suite.py`) on the final tree: **1816 passed, 11 failed, 72 ignored
  (59 min)**. The 11: 3 `disc_golden` (allowed on the slim ISO), 3 pylon GPU (display-less
  sandbox, pre-existing), 1 `frame_cost_live` (frame budgets, ran while this sandbox was also
  building — likely noise, needs a quiet re-run), `fix_0c_vent2_live` 2 (STALE binary: the suite
  built that test file before the re-points; the current tree passes 3/3), and the 3 movement
  fails above. NOTE: the suite's nf-game build (14:14-14:15) includes the final fix (last source
  edit 14:13:50), so its results are the final tree's.

## Honest scope

- The bit predicate (`push[1] > 0`) is the decompile's dot2 arm with the dot1 gate deliberately
  unported [S above] — the decompiler's pointer-typing made `copy+0x50`'s exact meaning (live
  bottom vs pick-start bottom) the weakest link; the disasm `sh 0x9c(s1)` + the DeltaP tracking
  argument are [M], the semantic reading is [M-with-S].
- `walkable` (the port's own pre-existing signal) stays in the grounded OR — master's behavior
  preserved; the game's rule is arguably `pick || ray` alone, but no test distinguishes them yet.
- `docs/WORKLOG.md` — appended `hatch-crouch-0c2` progress; no shared files reordered.

## Live checks for the main agent

1. The ramp/gate probe (above) — settles the dot1 gate question.
2. 0700004a: `--bond 3.0,-6.5,-3.4,0,0`, walk forward: does the real Bond drop into the plate
   band (trigger #277 fires ch 0x4d) within ~2 s, and does `obj+0x60` bit 0x8 stay CLEAR while he
   falls past the shaft's ledges? (`gdbwatch.py` on obj+0x60.)
3. 07000001: after ch 0xB, watch Bond's `obj+0x60` bit 0x8 during the stairwell descent — the
   game's flag should NOT latch on nosing grazes mid-descent.
