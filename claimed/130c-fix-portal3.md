## Your task: `portal3`, the 06 courtyard holes persist: the eye-cel state

Read `docs/reports/fix-portal2.md` on `s/fix-portal2` (42ba472) first. Its chain pass (`View_AddCels` 0x1e6ba0's `+0x18`
walk with `Vision_InView` 0x1e89e0) is evidence-based and kept. The main agent merged it and rendered the same spots
(`nf-view <iso> <level> --bond … --frame 120 --shot`, walk on vs `NF_NOPORTAL=1`), and did NOT push it:

- **07000006 `--bond 7.24,42.03,-39.31,10.161,0.428 --frame 120`: unchanged.** The masonry under the left walkway and
  the lower right castle wall are still missing with the walk on, with sky showing through; with `NF_NOPORTAL=1` they
  draw. This is exactly your report's case: "If the hole persists at frame 120 … the residue is the persistent
  `Camera_CheckLocation` state carried along the approach … which would point at the state machine."
- 07000007 window spot: unchanged (the room stays hidden, as you argued the game does). Leave it.
- The other two spots: identical on/off, good.

**Do:**
1. Reproduce headlessly at that exact spot through the same path `--bond` takes (Bond placed, then 120 ticks): log the
   viewer cel the port carries each tick (`ViewerCel`), what `build_FindCel` 0x1c8cb0 would return fresh at the eye,
   and which World cels the walk + chain pass keep vs the frustum-only set. Find which wall cels are dropped and why.
2. Decide from the decompile what the game's state is after a placement jump. `--bond` places Bond the way the dev
   teleport does; the game's own equivalents are the level start and `Player_Enable` with a matrix. Does
   `Camera_CheckLocation` 0x1bae90 re-find the cel (a FindCel) when the move crosses no portal and leaves the box, or
   keep a stale one? Make the port follow it, and make the dev teleport and `--bond` reset the viewer cel the way the
   game's own placement does.
3. Also walk there: a headless walk along the user's route (the truck → courtyard, `the_07000006_walk_keeps_the_level_visible`'s
   route, extended up to this spot and this view) must keep those wall cels visible. That's the case the user saw.
4. Pin the fix with a test at this spot that lists the wall cels, failing first.
5. Run the full four-crate suite (in the background, first thing) and report the count.

Files: `crates/nf-view/src/{scene,gpu,lib}.rs`, `crates/nf-view/src/dev.rs` (the teleport reset), `crates/nf-view/tests/fix_portal*_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/portal3` (start from `s/fix-portal2`, then merge `origin/master` into it).

---
claimed-by: sandbox1 at 2026-10-13T15:01Z
