## Your task: `weapon-matrix5`, weapon matrix: all 27 levels, harness gaps closed

Base: origin/master (weapon-matrix4 and weapon-matrix3b should be on it; if not, merge `s/weapon-matrix4`). Close
weapon-matrix4's harness gaps: the watch pair 0x54/0x55's mode press ORs the fire edge [M `Player_Weapon` 0x1a24c8,
action 9 mask 4]; the away switch after the Sentinel waits for `missile_cam` to clear plus the game's 15-tick restore;
re-run the Wolfram 0x04 on 0700004a and the MGL 0x2a on 0700000d. Then sweep ALL 27 levels (`nf-extract
weapon-matrix`), and for every remaining red cell write a numbered weapon finding (weapon, level, what's wrong, the
game evidence). Keep the in-suite `weapon_matrix_live` under 1200 s (report its runtime). Panics are bugs.

Files: `crates/nf-view/src/matrix.rs`, `crates/nf-view/tests/weapon_matrix_live.rs`.

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

**Branch:** `s/weapon-matrix5`.
