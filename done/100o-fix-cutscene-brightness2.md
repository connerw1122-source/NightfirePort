## Your task: `cutscene-brightness2`, the brightness fix makes cut-scene actors vanish

102g-fix-cutscene-brightness (`origin/s/fix-cutscene-brightness` 0cbfbf8, held) makes the cut-scene actors VANISH.
The main agent rendered the user's spot in the 07000002 intro with
`nf-view <iso> 07000002 --bond -82.30,5.30,32.05,1.570,-0.100 --frame 170 --shot x.png`:
- master (6cbe522): Bond (blue suit, left, in the doorway) and the bald man in the white shirt (centre, by the
  computer desk) are both drawn, too bright (the bug: near-white skin and shirt).
- master + the branch: the same room, the same camera, NO characters at all.
So the lighting change zeroes or culls the actors instead of dimming them (a zero light scale, alpha 0 in the colour
buffer, the palette's colour slot, or a NaN). Find why, fix it so they draw lit the way the game lights them (PCSX2:
dark, matching the room), and add a test that RENDERS that frame (or builds the exact GPU draw list) and asserts the
two actors' triangles are on screen with a non-zero, non-white colour; sister assertions. Merge master first.

Files: `crates/nf-view/src/scene.rs`, `crates/nf-game/src/nis.rs`.

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

**Branch:** `s/cutscene-brightness2`.

---
claimed-by: sandbox3 at 2026-10-14T09:04Z

---
## Report (done, 2026-10-14T09:38Z)

Closed by the driver: the report is on `s/fix-cutscene-brightness2` (docs/reports/fix-cutscene-brightness2.md); the worker didn't run finish.sh.
