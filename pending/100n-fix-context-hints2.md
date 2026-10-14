## Your task: `context-hints2`, the hint bar's position, the 007 ring, no prompt at the grapple point

100h-fix-context-hints (merged) shows the hint bar: at the 07000008 safe (`--bond 155.19,43.12,-113.38,1.57,0`) its
text matches PCSX2 exactly ("Hint: Use Watch Laser"). The main agent's side-by-side with PCSX2 at the two spots still
shows:
1. **The bar's position:** PCSX2 draws the hint bar at about y 100/448 (a quarter of the way down, under the HUD top);
   the port draws it at the top edge (y ~0). Use the HUD element's own rect.
2. **The ring:** at both spots PCSX2 swaps the radar ring (bottom left) for the ORANGE 007 icon; the port keeps the
   green radar. Port the ring swap and its sprite.
3. **The 05 grapple point** (`--bond 16.5,29.6,58.6,-2.01,0.24`, PP7 drawn in both): the port shows "Press Action"
   and a Bond-silhouette icon at the top right; PCSX2 shows NO text and no such icon there, only the orange 007 ring.
   Find why the port raises that prompt (the context source, the hint condition: maybe the hint needs the claw drawn or
   a range the spot isn't in) and match the game.
Tests per item at those spots (the bar rect, the ring sprite id, no prompt at the grapple spot with the PP7 drawn);
sister assertions; `--shot` the two spots.

Files: `crates/nf-view/src/hud.rs`, `crates/nf-game/src/player.rs`.

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

**Branch:** `s/context-hints2`.
