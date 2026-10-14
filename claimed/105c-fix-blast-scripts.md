## Your task: `blast-scripts`, explosions: each blast plays its own effect script

fix-fire-modes3's documented deviation: `spawn_blast` renders the generic one-shot 0x06000052 for every explosion,
while the game passes a per-source effect script to `Explode_Create` 0x17cbd0 (the Samurai overcharge bolt's
0x060007c4 cyan blast at `Bullet_CollisionHandler` 0x12ce80:207-245, and whatever the grenades, rockets, MGL, mines,
destroyables and barrels pass). Census every `Explode_Create` call site and the script it passes, carry the script id
through the port's blast request, and play that script. Tests (NF_ISO): for the Samurai bolt, a frag, a rocket and a
destroyable, the blast plays the game's script id; sister assertion.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report. Note: the main agent's PCSX2 captures have a CRT filter on; use them for presence and placement,
not exact colour or brightness.

Files: `crates/nf-game/src/{explode,drone}.rs`, the blast draw in `crates/nf-view/src/scene.rs` (small).

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

**Branch:** `s/blast-scripts`.

---
claimed-by: sandbox1 at 2026-10-14T18:53Z
