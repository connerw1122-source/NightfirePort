## Your task: `weapon-matrix3`, weapon matrix: the watch gadgets and a clean sweep

Base: origin/master. If `s/weapon-matrix2` is not on master yet, merge it into your branch first (it carries the
upgraded runner `nf-extract weapon-matrix` and docs/reports/weapon-matrix2.md, which this task builds on).

After 103i-fix-reload-hang (merge `s/reload-hang` if it isn't on master yet): (1) add the gadget-cycle path to the
matrix runner so the watch gadgets (0x4A-0x55: Stunner, Laser, Q-Claw, Micro-Camera; carried on every single-player
map, not given) get real cells: equip through `Player_WeaponChange` mode=1 (bounds 0x4A..0x5E), fire, check the beam
draw / dart / camera view and the sound; (2) re-run the full 27-level sweep and fix or explain every red cell that
remains, with the game evidence. Panics are bugs. The report's table must end with the red count per mark and a reason
per remaining red group.

Files: `crates/nf-view/src/matrix.rs`, `crates/nf-view/tests/weapon_matrix_live.rs`, and whatever the red cells point
at.

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

**Branch:** `s/weapon-matrix3`.

---
claimed-by: sandbox5 at 2026-10-14T14:47Z

---
## Report (done, 2026-10-14T16:22Z)

