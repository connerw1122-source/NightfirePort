## Your task: `fix-guard-weapons3`, guard weapons: still failing on current master

Base: `s/fix-guard-weapons2` — merge origin/master FIRST. HELD again: master + this branch (+ fix-aim-raise2 in the same
batch, which touches only the player's aim block) fails two tests that pass on master:
- `fix_guard_weapons_live::the_3x3_hit_rate_and_damage_matrix_matches_the_game`: "07000005.bin diff 3 weapon 0x16: hit
  rate 0.520 vs model 0.975 (52/100)" — your own matrix test, on the merged tree;
- `nf-game ai_live::guard_takes_cover_like_the_game`: "UnderCoverAim lasted 90 (live 37)" — this exact symptom was
  seen before when a change shifted the drones' RNG stream (docs/merge-log.md 275-fix-anim-blend: 90 vs the live 37);
  any new `Rand_*` draw in the guard fire path that the game doesn't make, or a draw order change, does it.
Find both: why difficulty 3 / weapon 0x16 hits half as often as the model on the merged tree (the TooClose,
first-moved, sighting-time gates against master's current Bond/drone state), and which draw moved the RNG. Match the
game's draw order exactly. Run the full `tools/test/suite.py` on the final tree; total in the report.

Files: `crates/nf-game/src/{drone,bullet}.rs`.

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

**Branch:** `s/fix-guard-weapons3`.

---
claimed-by: sandbox7 at 2026-10-14T22:38Z

---
## Report (done, 2026-10-14T23:45Z)

Closed by the driver: the report is on `s/fix-guard-weapons3` (docs/reports/fix-guard-weapons3.md); the worker didn't run finish.sh.
