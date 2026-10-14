## Your task: `weapon-matrix4`, weapon matrix: a sweep that only shows weapon bugs

docs/reports/fix-reload-hang.md and weapon-matrix3.md: most remaining red cells are the harness, not the weapons —
Bond dies mid-sweep from the level's own rockets/grenades and every later give/draw is eaten (D), the switch-back to
mode twins runs out of press budget at cell edges (B/W), the reload pulse lands inside a fire script (R). Make the
runner measure weapons only: keep Bond alive and unhurt for the sweep (the dev god path, armour/health refill per cell,
or a safe spot), give each cell its own settle so a switch always completes (budget from the game's draw/holster
lengths, not a fixed count), and re-sync the reload check to the idle view. Then run the full 27-level sweep and list
every remaining red cell with its reason; any cell that is a real weapon bug becomes a numbered follow-up in the
report (weapon, level, what's wrong, the game evidence). Panics are bugs.

Files: `crates/nf-view/src/matrix.rs`, `crates/nf-view/tests/weapon_matrix_live.rs`.

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

**Branch:** `s/weapon-matrix4`.

---
claimed-by: sandbox1 at 2026-10-14T16:59Z
