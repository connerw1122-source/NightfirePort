## Your task: `weapon-matrix4b`, weapon matrix: the in-suite tests must fit the time limit

Base: `s/weapon-matrix4` — merge origin/master into it FIRST (and `s/weapon-matrix3` if 103m2 hasn't landed it; keep
matrix3's gadget-cell test). HELD at merge: the full suite passed everything except `weapon_matrix_live`, which TIMES OUT
at the suite's 1200 s per-binary limit (last started: matrix_0700000d). The per-level tests now sweep at the honest
budgets (~60-90 s a story level), so a test per level for the whole list no longer fits. Keep every check and the
harness fixes, but make the suite binary finish well under 1200 s: a small fixed sample in-suite (05, 0c, one late
level, the gadget cells, the upgraded-runner checks), the full 27-level sweep only through `nf-extract weapon-matrix`
or an `#[ignore]`d test with a reason. Report the binary's runtime and the full `tools/test/suite.py` total.
106b-weapon-matrix5 builds on this.

Files: `crates/nf-view/tests/weapon_matrix_live.rs`, `crates/nf-view/src/matrix.rs` only if needed.

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

**Branch:** `s/weapon-matrix4b`.
