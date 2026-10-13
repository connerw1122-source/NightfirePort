## Your task: `swim-vent4`, finish the two reds and land the swim fix and the vent crawl

Read `docs/reports/fix-swim-vent3.md` on `s/fix-swim-vent3` (ca559c3) first: it ruled out the static causes and left
two dynamic ones with clear next steps. Continue from that branch.

1. **07000001 exit (`fix_mission_live:347`)**: the pads and every query are fine. Mayhew reaches the west stairs about
   400 ticks earlier than on master (one stall-nudge ahead), works door 4, then ping-pongs at the stairs' bottom between
   walk (AllyLead 0x1d), `DSTATE_OpenDoor` 0x81 and the 180° turn: the route target flips ahead/behind. Root-cause
   the flip: `follow()`'s creep advance vs `next_node` order on the last legs, the arrival radius, the door-4
   re-entry. Use `NDrone2_ReachedDestNode` 0x153358 and the LinkCreep target (Drone+0x670) in the decompile. Fix the
   port's logic; don't re-seed or re-time the test.
2. **0700000c ST-1 plate (`walkreal_0700000c_live:94`)**: the touch works (a poke to the pad centre fires ch 0x1b). The
   walker parks 0.03 outside the band's z edge and its approach variants slid into the x ≥ 7.0 void. Do what the
   report says: turn in place first at (7.2, −153.1), walk the corridor west to x ≤ 6.5, then due south over the pad's
   wide part to (5.84, −154.15). This is a walker change in `tests/common/walker.rs` or the test's route; the game's
   Bond walks onto the pad, so that's the right expectation.
3. **Run the full four-crate suite first thing, in the background**, so you have every red early; it has never
   completed on this branch.

Files: `crates/nf-game/src/drone.rs` (the follow/next_node arms, minimal), `crates/nf-view/tests/common/walker.rs`,
`crates/nf-view/tests/walkreal_0700000c_live.rs` (the route only), plus the swim-vent branch's own files.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/swim-vent4` (start from `s/fix-swim-vent3`, then merge `origin/master` into it).

---
claimed-by: sandbox3 at 2026-10-13T16:47Z

---
## Report (done, 2026-10-13T18:18Z)

