## Your task: `window-view`, 07000007: the view out of the windows

PCSX2 comparison (main agent, 2026-10-13) at `--bond 61.32,44.11,-52.58,3.749,-0.056` on 07000007, looking out of a
window: the real game shows the snowy slope with a blue haze over it (distance fog) and nothing else; the port draws a
lit stone building block floating in front of the slope, and the snow texture is smeared/stretched with no haze. Part of
it may be `NF_NOPORTAL=1` (the playtest launcher turns portal culling off until 130g-fix-portal-sweep lands): render the
spot with and without it. Find what the block is (instance/cel) and why the game doesn't draw it from inside, the
outdoor fog the game applies (fog colour/range per cel or per level) and the snow's UVs (texture matrix/scroll or wrap
mode). Tests: at that spot the block's instance is not in the drawn set; the fog parameters for that cel; the snow's UVs
match the disc's; sister assertions.

Files: `crates/nf-view/src/{scene,gpu}.rs`.

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

**Branch:** `s/window-view`.
