## Your task: `nav-remainder2`, land the route special nodes without stalling Mayhew

Read `docs/reports/fix-nav-remainder.md` on `s/fix-nav-remainder` (45c06ce). The `NDrone2_ReachedDestNode` 0x153358 dispatch,
the kick nodes on 0c/12 and the GoToGoalPosition dispatch are wanted. The main agent merged it onto master (7a614ba) and the
full suite went **809 passed, 4 failed**, all on 07000001:

1. `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` (line 107): **the contact stalls at (3.5, 6.3,
   7.1) in state 0x21** (the new ally-wait state, entered from a 0x20 node on the "Mayhew Route") for 10 rounds. Your
   arm gates on `switch_channels[0x99]`; nothing in the port sets 0x99 on this walk, so he waits forever. Find 0x99's
   writer in the game (the trigger/script that releases the ally wait: `Trigger_Calc`, a touch volume near that node, a
   script event) and port it, or show from the decompile that state 0x21 also exits on Bond's approach (the AllyLeadWait
   pattern, `Bond inside 3.0`).
2. `fix_mission_live::the_07000001_exit_works_end_to_end` (line 349): "the courtyard shoji reached OPEN (state 4) at least
   once … an activation every tick kept it ping-ponging 0x8↔0x10". A door is now re-activated every tick, probably the
   door-2 node dispatch firing repeatedly now that it also runs for `move_to_goal`. Check `NDrone2_DoorIsOpen` 0x14aaa0 and
   the once-per-arrival rule.
3. `fix_objects_live::a_death_draws_its_drop_and_nothing_else_moved` (line 264: no drop at the corpse) and
   `render2_live::dropped_guns_dim_to_the_cel_ambient` (line 258: "the guard dropped his weapon"): the 07000001 guard these
   tests shoot no longer dies/drops at the expected spot. Likely the same route change moves him (GoToGoalPosition now
   working nodes). Check whether the tests' aim tracks him (fix-ai-traces2 made them re-aim at his live position) and
   whether he's now in a kick/action/fiddle state where damage or death is handled differently.

Fix each from the decompile, don't loosen the tests, and run the full four-crate suite (in the background, first thing).

Files: `crates/nf-game/src/drone.rs`, `crates/nf-game/tests/nav_kick_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/nav-remainder2` (start from `s/fix-nav-remainder`, then merge `origin/master` into it).
