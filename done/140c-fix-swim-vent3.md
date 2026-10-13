## Your task: `swim-vent3`, the last two reds, then land the swim fix and the vent crawl

Read `docs/reports/fix-swim-vent2.md` on `s/fix-swim-vent2` (8b6374a) first. It fixed 13 of the 15 failures from evidence
(the one-sided ray test now follows the game's STORED plane: `Intersect_RayGeom` 0x10a8c8; triangles whose stored normal
opposes the winding are re-wound in `collision.rs`; the 12 unit tests re-wound to the disc convention; the 08 cage wall is
one-sided per its stored plane). Two remain, and the full four-crate suite has not been run on the branch:

1. `fix_mission_live::the_07000001_exit_works_end_to_end` (line 347): Mayhew stalls at (−33.0, 4.54, 11.8), short of the
   exit plate; a down-ray around the plate (−36.6, 3.3 → −2.7, 11.8) finds no floor within 6. Dump the collision triangles
   under the plate's bbox from the raw 0x2E blocks (`probe_planes`), check their stored planes, and find why the pad isn't
   found: the re-wind, the cel scoping (`Collide_StraddleCels`), or the water-material drop. It passes on master.
2. `walkreal_0700000c_live::walk_start_to_the_st1_plate_and_the_photograph` (line 94): ch 0x1b never fires (fails in
   0.75 s, so early in the walk). Probably the same pad question for plate #506. It passes on master.

**First step: run the full suite** on the branch merged with `origin/master` (in the background, right away), so you know
every red, not just these two. Then fix from evidence. Don't loosen either test.

Files: as fix-swim-vent2 (`crates/nf-formats/src/collision.rs`, `crates/nf-game/src/{world,player}.rs`, the examples),
plus `crates/nf-view/tests/{fix_mission_live,walkreal_0700000c_live}.rs` only if the evidence says their expectation is wrong.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/swim-vent3` (start from `s/fix-swim-vent2`, then merge `origin/master` into it).

---
claimed-by: sandbox5 at 2026-10-13T14:26Z

---
## Report (done, 2026-10-13T16:30Z)

