## Your task: `fix-weapon-anims2`, weapon anims: the three suite failures on the merge

Base: `s/fix-weapon-anims` (merge origin/master into it). HELD at merge: the full suite on master + this branch
(+ fix-striped-boxes2, which touches only the misc draw gate) failed 3 tests that pass on master:

1. `gadget_select_live::the_camera_pickup_equips_through_the_gadget_cycle` — "the weapon cycle must stay in 1..0x47":
   the WEAPON cycle (input `weapon_cycle: 1`, a 70-tick settle per press) landed on a gadget id (0x4a..0x5e). The game's
   cycle never selects a gadget (bounds 1..0x47 [M 0x1a7120]). Find why: a switch request queued while the slower
   holster/draw is still running, the var-swap/raise state changing which record the cycle starts from, or the
   double-advance fix leaving a stale request. This one is a behaviour bug until shown otherwise.
2. `gadget_laser_live::the_beam_end_equals_the_crosshair_rays_hit` — "the beam is parked" (`sim.laser_beam` None at
   the end of its aim loop): the laser never fired in the window. Check the aim raise (+0xB8) / the idle-counter reset
   against the gadget's fire gate.
3. `fix_weapon_sounds_live::every_weapon_on_07000005_draws_fires_and_reloads_with_sounds` — 0x32 [1057, 1059, 1479],
   0x33 [1056, 1058, 1479] and 0x41 [639] never played.

For each: decide from the game's code whether the branch's new timing is right (then the TEST's window/settle was
tuned to the old 2x-fast switches: extend it, citing the game's tick counts) or wrong (fix the code). Do not loosen an
assertion's meaning. Run `tools/test/suite.py` on the final tree (all binaries) and give the total in the report.

Files: `crates/nf-view/src/scene.rs` (the view-script block), the three test files as justified.

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

**Branch:** `s/fix-weapon-anims2`.
