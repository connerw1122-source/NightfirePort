## Your task: `ladder-water2`, redo the ladder-into-water fix without breaking the walk-on grab

`s/fix-ladder-water` (109i, NOT merged) fixed two playtest bugs on 07000002 (read its report
`docs/reports/fix-ladder-water.md`): the infinite re-latch when climbing down the hut ladder into the pond, and the
cut-scene skip leaving the voice and subtitle running. Start from that branch (`git merge origin/s/fix-ladder-water`).

The main agent's merge suite caught a regression, reproduced alone (both pass on master and on master + 109h + 109b;
both fail with 109i): walking into a ladder on dry ground no longer grabs it.

```
cargo test --release -p nf-view --test fix_ladder2_live
test camera_looks_during_the_climb ... FAILED
test weapon_saved_on_grab_restored_on_dismount_and_hands_cannot_fire ... FAILED
panicked at crates\nf-view\tests\fix_ladder2_live.rs:50:5: walking into the ladder never attached
```

The test stands Bond on the floor in front of a 07000002 ladder facing it and holds forward for 900 ticks; with 109i
he never reaches SS_CLIMB. The new pick (the swim capsule arm, the mesh refine, the dropped radial stage-2 stand-in)
must keep the WALK grab working exactly as the game does: `Player_Collision` 0x194df0's capsule switch has a walk arm
too, and `Player_CollLadder` 0x196118 must still fire from it. Find which part of the new pick rejects the walk-on
case (print the sphere test, the refine result and the substate per tick in the failing test), fix it against the
decompile, and keep 109i's own tests passing. Also: two tests that build `nf_view::scene::Msg` literals
(`fix_objective_banner_live.rs`, `fix_objective_messages_live.rs`) don't compile with 109i's new `script` field —
add `script: false` to each.

Files: `crates/nf-game/src/ladder.rs`, `crates/nf-game/src/player.rs`, the two Msg-literal tests above,
`crates/nf-view/tests/fix_ladder_water_live.rs`.


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

**Branch:** `s/ladder-water2`.
