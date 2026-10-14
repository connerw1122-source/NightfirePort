## Your task: `08-collision2`, the scoped collision breaks the 01 escort walk

100j-fix-08-collision (`origin/s/fix-08-collision` at 4023ba9, held) scopes Bond's collision to his straddled cels the
game's way [M Player_Collision 0x194df0, Collide_Pick 0x1eb320]. Merged with master (ccaea18) the full suite gives
997 passed, ONE failed, and it's real (the main agent re-ran it alone, the checkout history is clean):
`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`: "the contact must walk his route to the
LoadPointActivator plate": the contact settled at (-5.2, 6.3, 14.1), the plate is at (-33.1, 2.3, 15.4). The test's log
prints `world seg to (-1.0,+0.0): hip None hip+0.5 None feet None` around there. The contact waits for Bond (the escort
logic), so the likely cause is Bond's scoped collision: a place on the 01 route where Bond is stopped, falls, or
snags (a floor or step outside the straddle set, like the 09 stairwell the branch already fixed with the flat-redo
fallback). Find where Bond's walk diverges from master on that route (log both trees' Bond positions per 60 ticks),
fix it the game's way (the straddle set's cels: a missing portal crossing, a floor attribution), and keep the 08 safe
room fix (no invisible wall) and the 09 stairwell. Full suite on the merged tree, 0 failures.

Files: `crates/nf-game/src/{player,world}.rs`, `crates/nf-view/src/scene.rs`.

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

**Branch:** `s/08-collision2`.
