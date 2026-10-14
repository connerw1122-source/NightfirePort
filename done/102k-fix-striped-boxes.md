## Your task: `striped-boxes`, red/yellow striped boxes drawn on 07000005

`nf-view <iso> 07000005 --bond 16.67,30.04,58.60,1.755,-0.001 --skip-cutscenes --frame 30 --shot a.png` with
`NF_NOPORTAL=1` draws a box textured in bold red and yellow diagonal stripes on the trench's right, against the castle
wall. An earlier fix removed "red/yellow boxes" on 0700000c's roof; this one is on 05 (and the user's 0c check couldn't
find the 0c ones, so check both). Such stripes are usually an editor/marker texture the game never draws (a trigger
volume, a collision-only brush, a hint object). Identify the instance (placement index, class, mesh hash, texture
name/hash), find the game's rule that keeps it from being drawn (a class with no render, a render flag, a texture flag,
the cel pass's skip), and apply it to every level. Test: a census of every level for instances using that texture or
class: 0 drawn; sister assertions.

Files: `crates/nf-view/src/scene.rs`.

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

**Branch:** `s/striped-boxes`.

---
claimed-by: sandbox3 at 2026-10-14T09:38Z

---
## Report (done, 2026-10-14T11:15Z)

Closed by the driver: the report is on `s/fix-striped-boxes` (docs/reports/fix-striped-boxes.md); the worker didn't run finish.sh.
