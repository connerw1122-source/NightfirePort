## Your task: `swim2`, the swim fix breaks the 0c ladder-top exit

102h-fix-swim (`origin/s/fix-swim` dc59866, held) breaks `walkreal_0700000c_live::walk_start_to_the_st1_plate_and_the_photograph`
(the main agent reproduced it on master + the swim branch alone): after "the ladder climb: ladder climbed to y 11.74 at
+364 ticks", the walk to the walkway's south end STICKS: "bond=(9.03,11.74,-189.99) target=(2.80,11.60,-208.40)
grounded=false substate=0 crouched=true floor_ny=1.00 segment_to_target=Some(0.029)". So Bond leaves the ladder top
CROUCHED and ungrounded, a wall/segment right in front of him. The swim branch changed ladder.rs and player.rs (the
pond ladder grab from the water); the land ladder's top exit changed with it. Fix the exit so land ladders behave as on
master (the game's ladder-top dismount: `Player_CollLadder` 0x196118 / the climb substate's exit) while the pond ladder
still grabs from the water, keep the swim fixes, and make the full suite green on the merged tree (0 failures). Also
the entry depth item from the report is queued separately as 103d: don't duplicate it.

Files: `crates/nf-game/src/{ladder,player}.rs`.

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

**Branch:** `s/swim2`.
