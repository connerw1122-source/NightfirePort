## Your task: `fix-escort-01b`, the escort fix breaks weapons on 05

Base: `s/fix-escort-01` (merge origin/master into it). HELD at merge: master (ed718f8) + this branch fails three
tests that pass on master:
- `weapon_matrix_live::upgraded_runner_checks_the_new_behaviours_on_07000005`: "0x2a must spawn and blast its grenade"
  (the MGL's impact grenade no longer spawns or explodes);
- `fix_weapon_sounds_live::every_weapon_on_07000005_draws_fires_and_reloads_with_sounds`: "0x41: [639], 0x42: [1051,
  1481], 0x43: [1486, 1487, 1488] never played";
- `probe_escort_root::probe_root_block` panics at probe_escort_root.rs:22 "run script" — a probe must not fail in the
  suite: make it skip cleanly when its input is absent, or `#[ignore]` it with a reason.
The escort's drone.rs/triggers.rs changes reach the player's projectiles or the weapon/sound path on 05 (a shared
bullet list, the explosion queue, a trigger/channel change that the 05 weapons read, or Bond's state). Find it and keep
the escort walk and the meshless-trigger work. Run the full `tools/test/suite.py`; total in the report.

Files: `crates/nf-game/src/{drone,triggers}.rs`, the probe.

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

**Branch:** `s/fix-escort-01b`.

---
claimed-by: sandbox4 at 2026-10-14T18:46Z

---
## Report (blocked, 2026-10-14T19:49Z)

