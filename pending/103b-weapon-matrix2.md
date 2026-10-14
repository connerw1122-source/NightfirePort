## Your task: `weapon-matrix2`, re-run the weapon matrix on today's master and fix the red cells

100d-weapon-matrix (`nf-extract weapon-matrix <iso>`, `crates/nf-view/tests/weapon_matrix_live.rs`,
docs/reports/weapon-matrix.md) ran BEFORE today's weapon fixes landed: assets (100a), sounds (100b), effects (100c),
fire modes (100f/100l), gadgets (100k), silencer (102b), grenades (102c), zoom overlays (102a). Re-run it on master, put
the new table in the report next to the old one, and fix every remaining red cell (or give each a reason with the game
evidence). Any panic is a bug. Then make the matrix runner check the new behaviours too: fire-mode switching per
weapon (the twin record and the mode label), the guided camera for 0x2C, impact vs delay for the MGL, the gadgets' beam
draws. Numbers only in the report.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report; a test on the drawn set alone is not enough (three fixes today passed their tests while the
rendered frame still showed the bug).

Files: `crates/nf-view/src/matrix.rs`, `crates/nf-view/tests/weapon_matrix_live.rs`, `docs/reports/weapon-matrix.md`,
and whatever the red cells point at.

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

**Branch:** `s/weapon-matrix2`.
