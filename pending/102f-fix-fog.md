## Your task: `fog`, distance fog/haze missing outdoors (0c, the 07 windows)

PCSX2 at the user's spots (main agent, 2026-10-13): on 0700000c (`--bond 14.22,16.28,-139.54,0.986` and
`14.22,16.64,-154.43,2.353`) the real game is covered in a thick pale haze: the sea, the far buildings and the cooling
tower fade to near-white; the port draws them crisp under a clear sky. On 07000007 the view out of the windows has a
blue haze in the game and none in the port (101i). Find the game's fog: the per-level/per-cel fog colour, start and end
(the level header or the cel record, the GS fog register setup in the render code) and port it to the world, objects,
characters and the sky as the game applies it. Tests: the fog parameters per level (numbers from the disc); a far vertex's
fog factor; sister assertions. Render both 0c spots with `--shot`.

Files: `crates/nf-view/src/{gpu,scene}.rs`, `docs/systems/render.md`.

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

**Branch:** `s/fog`.
