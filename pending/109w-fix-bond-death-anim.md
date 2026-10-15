## Your task: `bond-death-anim`, Bond's death animation before the fail menu

The user (playtest 1015-0046, 07000005, dying to satchel blasts, then Restart): restart health is right now, but
"missing death animation". The real game plays Bond's death (the camera falls/tilts, the death NIS — `SP_Update`
0x1dbda8's death-script path, `Player_Death*`) before the fail/continue menu. Find the game's death sequence (the
player substate on death, the camera motion, any death script, its length) and port it so the menu comes up after it,
as in the game. Tests (NF_ISO): health 0 → the death sequence's frames run for its measured length → the menu. List the
live check for the main agent (what the camera does, how long) in your report.

Files: `crates/nf-game/src/player.rs`, `crates/nf-view/src/scene.rs`, a new test file.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/bond-death-anim`.
