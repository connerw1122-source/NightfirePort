## Your task: `fix-progress-detected-hook`, the detected counter and the autoaim distances (G-045)

G-045 (docs/GAPS.md, docs/fields/player-stats.md) leaves two pieces of the mission-progress layer open:
1. The **detected counter's live hook**: `PlrMissionStats` has a "times detected" stat, but the port never increments
   it. Find the vision-log call site in the decompile (where a guard's first sight of Bond logs the stat; search
   `PlrStat_Log*` callers in `DroneVision_*` / `NDrone2_*`), port the increment at that point, and pin it with a test:
   a 07000005 guard spotting Bond raises the counter exactly once per detection event as the game counts it.
2. **`Player_InitWeapon`'s per-difficulty autoaim distances**: the port uses fixed values. Read the difficulty arms in
   `Player_InitWeapon` 0x1a7ef8 (the autoaim distance writes), port them keyed on the difficulty runtime
   (`GameState+0x40`), and test that each difficulty gives the game's numbers.

Check `s/fix-progress-remainder` (110d) on origin first. If it already did either part, skip that part and say so.

Files: `crates/nf-game/src/progress.rs`, the vision call site, `crates/nf-game/src/player.rs` (the InitWeapon arms), the
new tests, docs/fields/player-stats.md.

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


**Branch:** `s/fix-progress-detected-hook`.

---
claimed-by: sandbox4 at 2026-10-15T12:38Z

---
## Report (done, 2026-10-15T14:31Z)

Closed by the driver: the report is on `s/fix-progress-detected-hook` (docs/reports/fix-progress-detected-hook.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-15T14:32Z)

