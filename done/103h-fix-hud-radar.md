## Your task: `hud-radar`, the HUD radar: blips, range, rotation

The radar ring bottom-left: check against the game what it shows (enemy blips, objectives, allies; their colours and
sizes), its range and scale, rotation with Bond's yaw, height cues, and when it hides (cut-scenes, the context ring
swap from 100h). Port what differs, citing the HUD code (`HUD_*` radar functions). Tests at two spots on 05 and 06 with
guards around: the blip positions on the ring equal the game's projection (numbers); sister assertions.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report; a test on the drawn set alone is not enough (three fixes today passed their tests while the
rendered frame still showed the bug).

Files: `crates/nf-view/src/hud.rs`.

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

**Branch:** `s/hud-radar`.

---
claimed-by: sandbox5 at 2026-10-14T12:29Z

---
## Report (done, 2026-10-14T14:47Z)

Closed by the driver: the report is on `s/fix-hud-radar` (docs/reports/fix-hud-radar.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-14T14:54Z)

