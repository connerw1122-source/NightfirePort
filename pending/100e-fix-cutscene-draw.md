## Your task: `cutscene-draw`, cut-scene actors exist but are not drawn (a headless repro)

101b-fix-cutscene-models reported "the regression does not reproduce headlessly": every scene's actors are live with
finite poses and pass the palette's room gate (its new `tests/cutscene_models_live.rs`). But the RENDERED frames show
no actors. The main agent (2026-10-13, master 7be3272's playtest build):

    nf-view <iso> 07000007 --bond 94.2,44.6,-45.9,0,0 --frame <N> --shot k.png      (N = 150, 300, 600, 900, 1200, 1500)

Bond stands in TouchOnce #412's volume, so the Kimiko scene (0x06000073) starts at once; "(disabled by a script)" is
printed for every N. Every frame shows the script camera panning around the library: the table with the lamp, the
paintings, the doors, and NO characters at all: no Kimiko, no Bond, no guests. Same with and without `NF_NOPORTAL=1`.
The user sees the same in the window ("the only model consistently showing up in cutscenes is my player character";
on 08 "Kiko is the only one showing besides me").

So the actors exist in the sim but are not drawn. Find where they drop between `skin_palette` and the GPU draw: the
skin layout (G-043 GPU skinning: are script actors' vertices in `skin_layout` / does their palette slot get written,
`col` alpha, `vis` masks, the character's draw range in `Renderer::render`), the `frame_state` hidden flag, the
camera-relative placement (are they drawn at the far y −10000?), the material/draw filter for `Layer::Character` under
a script camera, or the 120 fps palette blend (`lerp_palette` / `reattach_view_model`, fix-interp-ghosting: it touches
the palette while a script camera runs). Bisect with the `--shot` above across the suspect merges (gpu-skinning
942e2ca, anim-blend2 e679d6e, facial 366ceff/bc231e8, script-commands 0a7268d, interp-ghosting) in a worktree with its
own target dir. Fix the cause; then a test that RENDERS (or builds the exact GPU draw list the window draws) a frame of
the Kimiko scene and asserts Kimiko's and Bond's skinned triangles are on screen (projected inside the viewport, depth
in range), failing on master; sister assertions. Merge `origin/s/fix-cutscene-models` first (its census tests stay).

Files: `crates/nf-view/src/{scene,gpu,lib}.rs` as the cause dictates, tests.

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

**Branch:** `s/cutscene-draw`.
