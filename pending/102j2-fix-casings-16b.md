## Your task: `fix-casings-16b`, the 16 arrival door fix breaks the 01 doors

Base: `s/fix-casings-16` (merge origin/master into it). HELD at merge: with this branch, two tests that pass on master
fail — `level_end_live::door_e_opens_then_e_closes_on_07000001` and
`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` (bisected: master + the other two branches of
the batch pass both; adding this branch fails both). The suspect is the per-tick `straddle_walkin` in door.rs (and its
drone.rs hook): on 01 it opens/holds/closes doors the game doesn't, or changes the escort's path through a door. Find
the game's actual gate for the straddle walk-in (`Door_Update`'s proximity math, which doors and which states it
applies to) and narrow the port to it, so 16's arrival door still works and 01's door E / escort walk behave as on
master. Run both tests plus `doors_live`, `doors_01_live` and the full `tools/test/suite.py`; totals in the report.

Files: `crates/nf-game/src/{door,drone}.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/fix-casings-16b`.
