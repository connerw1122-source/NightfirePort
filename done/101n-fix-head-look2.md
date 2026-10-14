## Your task: `head-look2`, the head-look merge breaks two 07000001 walk tests

101c-fix-head-look (branch `s/fix-head-look`, 6c6cf74; it also carries fix-facial3) was held at merge: merged with master
(af0cc38 on the main agent's lane, master 7be3272 + the branch) the full suite gives 897 passed, 2 FAILED:
- `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`
- `fix_pickups_trace_live::with_dev_god_and_infinite_ammo_every_raptor_drop_is_taken_and_the_spare_matches_the_trace`
Both pass on master (4aaadac: 901 passed, 0 failed). Both are 07000001 runs whose outcome depends on the guards'
timing and the RNG stream, so the head-look change most likely changes a drone's tick behaviour (an extra `Rand_*`
draw, a changed anim/aim timing, or the head bone feeding an aim or sight test) rather than only the drawn pose.

Take over `origin/s/fix-head-look`: find which change moves those runs (bisect its commits against the two tests), and
make the head look a pure presentation change where the game's is one (the game's head turn: does it read RNG or feed
the AI? answer from the decompile). If the game really does change the AI timing, re-pick the tests' expectations the
documented way (WORKLOG 0072's seed procedure) with the evidence. Keep the head-look and facial fixes and their tests,
then a full suite on the final tree (merged with master) with 0 failures.

Files: `crates/nf-game/src/{anim,drone}.rs`, `crates/nf-view/src/scene.rs`, the two tests only if justified.

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

**Branch:** `s/head-look2`.

---
claimed-by: sandbox7 at 2026-10-14T04:55Z

---
## Report (done, 2026-10-14T06:00Z)

Closed by the driver: the report is on `s/fix-head-look2` (docs/reports/fix-head-look2.md); the worker didn't run finish.sh.
