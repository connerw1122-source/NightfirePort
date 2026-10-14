## Your task: `0c-hatch`, 0700000c: the display vent hatch: slow, and Bond can't get through

The user's playtest (results 1013-1758, 0700000c): after burning the four display-vent latches (locks out 14-17 at
(7.09/4.54, 15.70/16.87, -112.35)) "the hatch actually opened very slowly, but climbing through and dropping down, he
still can't get through because he can't crouch through". Find the hatch's open animation speed in the game (its
object's anim/rotation rate) and the space below it: does the game crouch Bond automatically, use a creep/vent volume, or
is the gap tall enough to walk? Fix so the player can go through the way the real game allows. Test: from
`--bond 5.8,15.8,-108.5`, all four latches burned, the hatch is fully open within the game's time, and the real input
path gets Bond through to the display ground below (y ~ 7.3 at (5.4, -113)); sister assertions.

Files: `crates/nf-game/src/{lock,player,props}.rs` as needed.

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

**Branch:** `s/0c-hatch`.
