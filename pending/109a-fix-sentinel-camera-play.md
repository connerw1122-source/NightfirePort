## Your task: `sentinel-camera-play`, the Sentinel's rocket camera doesn't happen in play

User: "The rocket launcher doesn't do the first person. The rockets are way too slow." (07000005 `--give 0x2c,0x2a`)
and on 08: "The rocket launcher, for some reason, doesn't go into first person." The tests (weapon_matrix gcam,
fix_weapon_fire_modes_live) say the guided camera rides - so the real play path differs: the give lands the
UNGUIDED twin 0x2d, the mode button B doesn't reach 0x2c, the camera ride needs the fire held, the window's view
doesn't use `missile_cam`, or the first-person view model hides it. Find it through the real input path and the
window's camera code (lib.rs) and fix it. The speed: fix-fire-modes3 applied `+0x6C & 0x200` -> speed x0.25
[M Bullet_init 0x12c5a0:188-190] to the Sentinel (0.25 -> 0.0625 u/tick); the user says it's far too slow. Re-read
the disassembly of that arm (which field it scales, whether the guided rocket's speed is set elsewhere - the camera
steering code may set its own speed each tick); the main agent will measure the real rocket speed in PCSX2.
This comes from the user's playtest 1014-1725 of the real-input build (`nf-view <iso> <level> --dev`, mouse +
keyboard). The user was very disappointed: several merged fixes passed their tests yet were wrong or broken in play.
Your test MUST drive the real input path (`Sim::step_bond` with the `Input` the window builds - lib.rs maps the mouse
and keys), not internal setters, and must fail first on master. The main agent will compare against PCSX2 after you
finish; put the exact steps to reproduce in the report.

Files: `crates/nf-view/src/{scene,lib}.rs`, `crates/nf-game/src/drone.rs` (small).

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

**Branch:** `s/sentinel-camera-play`.
