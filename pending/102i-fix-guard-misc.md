## Your task: `guard-misc`, the 0c sniper takes no damage; guards and doors; the 06 start poses

The user's playtest (results 1013-1825):
- 0700000c: "I heard sniper shots, looked at them, and I saw the enemy, but shooting him did nothing." (A rooftop sniper:
  check his hit volume / damage path / whether the port's bullet hits his collision at that range.)
- 07000006: "NPCs still can't open doors."
- 07000006: "as soon as I spawn into the level, the characters are doing crazy models, like weird model stuff" (the
  guards' first frames: a pose/animation blend from frame 0, or unposed bind poses before their start animation).
Reproduce each headlessly and fix from the game's code: the sniper's damage, the drones' door use (`NDrone2_ReachedDestNode`
door case, fix-nav-remainder ported the dispatch: find why 06's doors still don't open), and the drones' first-frame pose
(every drone posed from tick 0). Tests for each; sister assertions.

Files: `crates/nf-game/src/{drone,door,damage}.rs`.

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

**Branch:** `s/guard-misc`.
