## Your task: `zoom-overlays`, the Micro-Camera and AMES 20 overlays drawn as a 2x2 grid

The user's playtest (results 1013-1825): zoomed with the Micro-Camera (0x54) on 07000005 and with the AMES 20 (the user's 07000016 rifle, HUD 'Burst';
find its id from the name table) the port draws the overlay as a 2x2 GRID: four copies of the frame, each with its own rounded
corners, so a black cross runs through the middle of the view. PCSX2 at the same spots (main agent, 2026-10-13):
- the Micro-Camera's viewfinder is ONE full-screen frame: rounded corners only at the four screen corners, the scale marks
  down the left and right, the two brackets at mid-height, a small centre circle, "Micro-Camera" bottom right. It is
  plainly one quarter sprite mirrored into the four quadrants (U flipped on the right half, V flipped on the bottom half);
  the port draws the quarter sprite unmirrored four times;
- the sniper rifle (0x1e, 0700000c) and the crossbow (0x11, 07000016) scopes show ONE centred oval: black outside, a
  thin dark rim, the four thick posts, a thin circle and a thin cross with two ticks. The port's sniper overlay on 0c
  matches. The AMES 20's real overlay wasn't captured yet: find which overlay the game gives it (its weapon record) and
  draw that one, whole.
Find the game's overlay draw (fix-viewmodel-zoom found each zoomable weapon's own overlay; the HUD sprite draw with
its flip flags / UV order, `Player_Zoom` 0x1a1a70 and the zoom HUD call) and draw each overlay the way the game does
(quadrant mirroring, sprite size and placement at 4:3, centred on wide windows with black side bars). Tests: for 0x54,
the AMES 20, 0x11, 0x1e and every other weapon with a zoom overlay, the drawn quads (positions, UVs and flips) form one frame: no
seam in the screen centre; sister assertions. List `--shot` commands for each zoomed overlay.

Files: `crates/nf-view/src/hud.rs` (the overlay draw), `crates/nf-game/src/gun.rs` only if the overlay choice is wrong.

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

**Branch:** `s/zoom-overlays`.
