## Your task: `fix-shading-parity2`, shading parity: rebuild on master's letterbox

Base: `s/fix-shading-parity` — merge origin/master FIRST. HELD: master now has 108a-fix-cutscene-letterbox, which
changed the same render plumbing; the merge conflicts in `crates/nf-view/src/{gpu,lib}.rs`,
`tests/fix_0c_pylon_live.rs` and `tests/probe_pylon_render.rs`. Keep BOTH: the letterbox bars and the cut-scene 4:3
framing, and your per-cel light pick (View_DrawCelGlist 0x1e6538's cel sphere vs obj+0x80) and the smoothed edges.
Also port the open item from your report: `View_SetupRenderModes2` 0x1e7908 (lights picked from the camera for skins
with glists + attachments) — find which skins hit it. Render the Kimiko close-up (07000007 `--bond 94.2,44.6,-45.9,0,0`
frames 600/1000/1400) and the 05 trench before/after and describe them. Run the full `tools/test/suite.py`.

Files: `crates/nf-view/src/{gpu,lib,scene}.rs`, the two pylon tests.

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

**Branch:** `s/fix-shading-parity2`.
