## Your task: `heads-07`, 07 heads and faces look awful

User (07000007): "It looks awful. I took an F12." (F12 spots on 07000007: `--bond 39.78,44.10,-70.86,-4.083,-0.108` frame 635, `--bond 41.71,44.13,-72.05,-1.488,-0.154` frame 1102, `--bond 40.58,44.13,-70.77,0.682,-0.080` frame 1488, `--bond 33.40,44.10,-70.63,1.569,0.000` frame 43).
Compare the guests' and guards' heads at those spots with the game: the head skin's texture/UVs, the hair/hat
attachment, the head bone's pose (101c-fix-head-look's head angle), the facial morph state (108c is porting the GPU
morphs - coordinate: if the faces are mid-morph garbage, say so), and the lighting (108b). Fix what is wrong in the
head path; render the four spots before/after.
From the user's playtest 1014-1725 of the real-input build (`nf-view <iso> <level> --dev`). Drive the real input
path in tests; fail first on master; render before/after for anything visible; put exact reproduction steps in the
report for the main agent's PCSX2 compare.

Files: `crates/nf-view/src/scene.rs`, `crates/nf-game/src/anim.rs`.

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

**Branch:** `s/heads-07`.

---
claimed-by: sandbox7 at 2026-10-15T05:14Z
