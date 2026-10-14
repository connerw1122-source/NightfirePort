## Your task: `fix-guard-weapons2`, guard weapons: the branch fails on current master

Base: `s/fix-guard-weapons` — merge origin/master into it FIRST. HELD at merge: on current master (ed718f8, with
fix-weapon-fx-levels' drone.rs changes) this branch alone fails six tests:
- its own `fix_guard_weapons_live`: `the_3x3_hit_rate_and_damage_matrix_matches_the_game` "ran out of ticks collecting
  100 shots (got 90)"; `the_fire_rate_matches_the_game_interval_clamp_and_burst_cut` "ran out of ticks collecting 40
  shots (got 4)"; `too_close_doubles_the_damage_and_forces_the_hit` "close hit #0: 2.5 vs 5";
- and on master-passing tests: `fix_progress_live::the_difficulty_scales_a_guards_damage_to_bond_by_the_decompile_factor`,
  `render2_live::dropped_guns_dim_to_the_cel_ambient`, `fix_objects_live::a_death_draws_its_drop_and_nothing_else_moved`
  (also `walkreal_07000001_live` in the batch run).
Find what in the merge with master changes the guard fire path or the damage scale (a second difficulty/armour factor
applied twice, the fire interval reset by another branch's hook, a dropped-gun path the branch altered), fix it so
the branch's numbers match the game on current master, and keep the three other tests green. Run the full
`tools/test/suite.py` on the final tree; total in the report.

Files: `crates/nf-game/src/{drone,bullet}.rs`.

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

**Branch:** `s/fix-guard-weapons2`.

---
claimed-by: sandbox7 at 2026-10-14T18:02Z
