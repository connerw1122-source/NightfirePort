## Your task: `hat-details`, Oddjob's hat: the wall sound and the in-flight cap

User: "It makes a weird sound when it hits a wall. I'm not sure if that's accurate to the game. And also, if you throw
enough of them, you stop seeing them flying in the air." Check the hat's contact sound against the game (the bullet
collision handler's sound arm for the hat record, the material sound) and the cap: the port's projectile / grenade
pool or the draw list drops new hats past some count - the game's limit is the bullet pool (find its size) and the
oldest-first reuse. Fix both to the game's behaviour.
From the user's playtest 1014-1725 of the real-input build (`nf-view <iso> <level> --dev`). Drive the real input
path in tests; fail first on master; render before/after for anything visible; put exact reproduction steps in the
report for the main agent's PCSX2 compare.

Files: `crates/nf-game/src/{grenade,drone}.rs`, the draw.

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

**Branch:** `s/hat-details`.

---
claimed-by: sandbox3 at 2026-10-15T04:45Z

---
## Report (done, 2026-10-15T06:20Z)

Closed by the driver: the report is on `s/fix-hat-details` (docs/reports/fix-hat-details.md); the worker didn't run finish.sh.
