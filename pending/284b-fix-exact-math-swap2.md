## Your task: `exact-math-swap2`, land the exact sine/matrix path on current master

Read `docs/reports/fix-exact-math-swap.md` on `s/fix-exact-math-swap` (571155d). The swap itself (`RotMatrix` 0x1e2368 via
the bit-exact `PS2Sinf`, `Quat_QuatTransToMat` 0x1e3fa8 on the EE FPU, `psiBuildMatrixPalette` 0x111498 on VU0) is
wanted. The main agent merged it on current master (with fix-room-sound, fix-cover-remainder, fix-nav-remainder,
fix-anim-blend2 and fix-gpu-skinning in it) and the suite went **850 passed, 2 failed**:

1. `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` fails again. Your report says the old
   repro no longer reproduced on your base. On current master it does. Bisect which merged change brings it back
   (your branch + each newer master merge in turn) and find the first divergence between the two math paths in that run:
   the same side-by-side method as your report, at the failing tick.
2. `ai_live::grenade_bounce_scares_the_guard_into_the_flee_family` (ai_live.rs:1334, added by fix-cover-remainder after your
   base): "the guard never entered RunAwayFromObject 0x86 (log [(1, 4), (1, 4)])". With the exact matrices the bounce or
   the guard's view of it changes enough to miss the scare. Find which value moves (the grenade's bounce point, the
   guard's facing/LOS, `DroneFunc_ConsiderExplosive`'s distance).

Decide each from the game: the exact path is the game's, so a test that only passes with the host math is pinned to
a host-math accident. Fix the port, or re-pin the test with evidence that the exact path matches the game (a trace,
the decompile). Don't drop the swap to make a test pass.

Files: `crates/nf-formats/src/{placement,anim}.rs`, `crates/nf-game/src/ps2math.rs`, the two tests (only with evidence).

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/exact-math-swap2` (start from `s/fix-exact-math-swap`, then merge `origin/master` into it).
