## Your task: `playtest-spots`, a start spot and the needed gear for every playtest check

The user runs playtest checks from a local launcher (one nf-view launch per check, with arguments). They asked: when a
check needs something (the grapple needs the Q-Claw), start Bond AT the place, with the gear, not at the level start.
nf-view now takes `--bond x,y,z,yaw,pitch` (the window honours it) and `--give id,id` (equips weapons/gadgets, draws the
first).

Write `docs/playtest/spots.md`: for each of these features, the level, a `--bond` spot where the player can try it
straight away (standing on a floor, facing it), and the `--give` list, taken from the existing tests and evidence (the
tests already stand Bond at these places: grep them), each with its source:
the Q-Claw grapple points on 05 (#281/#282), the laser on the 08 safe bolts and the fuseboxes (07000008/14), the 0c
display vent and the hatch, the 05 cable (wire), the 06 searchlight beams, the 05 gate guards' ledge, the 02 water bank and
the 02 vent, the 07 Kimiko scene trigger, the 16 corridor door, the 0c jetpack photograph spot (camera 0x54), the 0a Q-Worm
PC, the 12 lift buttons, 0x2E Scorpion targets (a guard in view on 05).
Check every spot headlessly: a test that loads each level, places Bond there, runs 60 ticks, and asserts he's grounded
(not falling) and the feature's object is within reach (distance and line of sight). Numbers only, no game bytes.

Files: `docs/playtest/spots.md` (new), `crates/nf-view/tests/playtest_spots_live.rs` (new).

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

**Branch:** `s/playtest-spots`.

---
claimed-by: sandbox3 at 2026-10-15T17:06Z
