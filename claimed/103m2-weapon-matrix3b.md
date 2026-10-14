## Your task: `weapon-matrix3b`, weapon matrix: the suite's matrix tests time out

Base: `s/weapon-matrix3` (merge origin/master into it). HELD at merge: the full suite passed everything except
`weapon_matrix_live`, which now TIMES OUT at the suite's 1200 s per-binary limit (last started: matrix_07000014); on
master it finishes inside the limit. The gadget cells and the longer per-cell settles made the in-suite matrix tests
too slow. Keep every check, but make the suite's matrix tests fit well under 1200 s in total: sample fewer levels/ids
in the per-level tests (keep 05, 0c and one late level), keep the gadget-cell test, and leave the full 27-level sweep
to `nf-extract weapon-matrix`. Report the binary's runtime on the final tree and the full `tools/test/suite.py` total.
If 105a-weapon-matrix4 has landed on master first, merge it and keep its changes.

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

**Branch:** `s/weapon-matrix3b`.

---
claimed-by: sandbox3 at 2026-10-14T18:26Z
