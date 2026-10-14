## Your task: `swim`, swimming is barely there; the ladder in the 02 pond

The user's playtest (results 1013-1825, 07000002 garden pond, `--bond -82.3,5.3,32.05,1.57`): "This is hardly swimming. I
kind of figured it out when I started spamming the crouch button, and I very, very slowly got into the water." "I
could hear my footsteps too." "I can get in the water, but I can't crouch in water. I'm not sure how to swim." "There's a
ladder in the water under that hut. I can't latch onto it and climb it." Find the game's swim (`Player_InWater`, the
swim substate 3: entering by walking in, the swim speed, diving and surfacing with the crouch/jump actions, no
footsteps while swimming: the swim stroke sounds instead) and the water ladder (a ladder object reaching into the
water: the grab test from the swim state). Fix and test through the real input path: walking forward off the bank enters
the swim at the game's speed; crouch dives, jump surfaces; no footstep sounds while swimming; the ladder under the hut
grabs from the water and climbs out; sister assertions.

Files: `crates/nf-game/src/{player,ladder}.rs`, `crates/nf-game/src/sound.rs` (footsteps).

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

**Branch:** `s/swim`.
