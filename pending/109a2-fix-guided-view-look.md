## Your task: `guided-view-look`, the guided rocket's view must look like the game's

109a (`fix-sentinel-camera-play`, merged) put the window's camera on the guided rocket. The ride LOOKS nothing like
the game. The user: "Guided missiles still don't work… I mean the visual." What the port shows, from the main agent's
capture at 07000005's spawn (`nf-view <iso> 07000005 --dev --give 0x2c,0x2a`, fire in 'Guided'):
- the camera sits INSIDE the rocket's own model: a big dark octagonal tube fills the screen (the rocket mesh/exhaust
  drawn around the eye);
- no red tint, no overlay;
- the health ring is still drawn.

The real game (`docs/live/2026-10-13-weapon-modes-pcsx2.md`, the AT-420 Sentinel section, read it on master):
- a red-tinted full-screen camera rides the missile, with a red HUD overlay:
  - a centre ring;
  - horizontal bracket lines left and right of it, angled lines below;
  - faint horizontal scanlines;
- the health bar and the crosshair are gone while it flies; the kill counters, clock, radar and ammo/mode lanes stay;
- on impact, one or two frames of red static (dense red horizontal noise lines), then the view snaps back to Bond with
  the explosion visible.

Find the game's code for all of this: `Player_PositionCamera` 0x18f5d8 case 0xc, the substate-10 HUD, the overlay's
texture and sprite layout (the HUD pane list / `Hud_*` draw for the missile camera), the tint (a screen colour
filter — find which call sets it), the static, and whether the game hides the rocket's own model from its camera (or
places the eye ahead of it). Port it exactly, with the overlay sprites from the disc textures. Render the port with
`--shot` during a ride (a scenario or frame flag that fires the Sentinel) and put the picture next to the description
in your report. Tests (NF_ISO) for the overlay's draw list during the ride, the hidden health pane, the hidden rocket
mesh, and the static frames at impact.

Files: `crates/nf-view/src/hud.rs`, `crates/nf-view/src/scene.rs`, the renderer file that applies screen tints
(find it), a new test file.

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

**Branch:** `s/guided-view-look`.
