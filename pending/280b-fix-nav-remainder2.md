## Your task: `nav-remainder2`, re-enable the ally-wait state and the NoBunching scan on evidence

fix-nav-remainder is merged (master 5fc6e51): the `NDrone2_ReachedDestNode` 0x153358 dispatch, the GoToGoalPosition
dispatch, the kick nodes. Its last commit (61fa5ce) switched OFF two parts because they broke 07000001:

1. **The ally-wait state entry (0x21, from the 0x20 nodes on the "Mayhew Route").** With it on, the contact stalls at
   (3.5, 6.3, 7.1) in state 0x21 forever (`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`), and
   the courtyard shoji ping-pongs 0x8↔0x10 (`fix_mission_live::the_07000001_exit_works_end_to_end`). The arm gates on
   `switch_channels[0x99]`, which nothing in the port sets on this walk. Find 0x99's writer in the game (a trigger,
   touch volume or script event near that node; `Trigger_Calc`, the level's placements) or the state's own exit (Bond's
   approach, like AllyLeadWait's dist ≤ 3.0), port it, and re-enable the entry.
2. **The NoBunching tick scan** (the 0x5E "drone ahead" flags +0x31C). With it on, the 07000001 death-drop tests
   (`fix_objects_live::a_death_draws_its_drop_and_nothing_else_moved`, `render2_live::dropped_guns_dim_to_the_cel_ambient`)
   lose the guard they shoot. Find why: the scan moving him out of the test's line (fix-ai-traces2 made those tests track his
   live position), or the scan firing where the game's wouldn't (its gates in the decompile).

If the evidence needs the real game, write a precise live-check request (spot, `gdbwatch.py` on `switch_channels+0x99` …)
in your report and park that part. Don't loosen the tests.

Files: `crates/nf-game/src/drone.rs`, `crates/nf-game/tests/nav_kick_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/nav-remainder2`.
