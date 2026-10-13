## Your task: `doors-remainder`, doors: the use test against the mesh, the labels, the proxy path

GAPS G-035's remainder: the use sphere vs the collision mesh (the port uses a bounding-sphere stand-in for
`Player_Activate`'s pick against the door), the door text labels 0x2000003/0x200000b (the "locked" messages), and the exact
proxy-list path of `Door_ReactToHit` (from `Collide_Update`, not `Door_Update`). Census first: for every door on the 27
levels, can a player standing at the door's approach open it with E (the game's pick), and does a locked door show its
label? Then port what fails. Don't touch the door->portal links (the portal work owns those).

Files: `crates/nf-game/src/door.rs`, `crates/nf-view/src/scene.rs` (the pick, minimal), the HUD label hook.

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

**Branch:** `s/doors-remainder`.
