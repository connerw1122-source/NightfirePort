## Your task: `laser-watch-look`, the laser watch: match the real game's beam and wrist

The user, repeatedly: "the laser isnt visible", "The laser watch sucks", "Still not right... look at it in the
emulator". The main agent measured the real game in PCSX2 — read `docs/live/2026-10-14-weapon-measurements-pcsx2.md`
("The laser watch 0x4e") on master first: on the press the wrist turns so the watch face points along the aim and a red
beam runs from the watch face to the crosshair point (wide pink-white glow at the watch, tapering to a thin red line
ending at the aimed-at surface); the meter drains ~31/s while held. Compare the port at 07000005's spawn
(`nf-view <iso> 07000005 --dev`, select the Laser with the gadget cycle, hold fire): the wrist anim (the fire script
+0xB0/+0xB4 of 0x4e and its view model), the beam's start point (the watch face bone, not the eye), its width taper,
colour and glow, its end at the crosshair hit, and the drain rate. Fix every difference; render the port at the
same spot before/after next to the description. Tests (NF_ISO) for the beam endpoints and the drain per tick through
the real input path.

Files: `crates/nf-view/src/scene.rs` (the beam + view model), `crates/nf-game/src/gadget.rs`.

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

**Branch:** `s/laser-watch-look`.

---
claimed-by: sandbox1 at 2026-10-15T05:27Z
