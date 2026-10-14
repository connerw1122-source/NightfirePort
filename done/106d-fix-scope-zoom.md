## Your task: `scope-zoom`, every zoomable weapon: zoom steps and overlay

The user has reported the sniper and AIMS-20 scopes several times; 102a-fix-zoom-overlays fixed the overlay quarters.
Audit EVERY zoomable record (the snipers 0x1e-0x28, the AIMS-20 pair, the micro-camera pair, any others with a zoom
table): the zoom steps and their keys (`Player_Weapon` 0x1a24c8:154-187, the per-weapon aim-zoom table, `+0x8D4`),
the zoom-in/out rate, the overlay texture and layout, the crosshair while zoomed, the view-model hide, the sway, and
the sound. Table in the report: weapon -> game vs port; fix the mismatches. Tests (NF_ISO): per weapon the zoom
levels and overlay id equal the game's (numbers); render each overlay once.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report. The main agent's PCSX2 captures have a CRT filter on: use them for presence/placement/layout,
not exact colour or brightness.

Files: `crates/nf-view/src/{scene,hud}.rs` (small, local).

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

**Branch:** `s/scope-zoom`.

---
claimed-by: sandbox3 at 2026-10-14T21:09Z

---
## Report (done, 2026-10-14T22:12Z)

Closed by the driver: the report is on `s/fix-scope-zoom` (docs/reports/fix-scope-zoom.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-14T22:12Z)

