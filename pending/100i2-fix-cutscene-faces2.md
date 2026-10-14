## Your task: `fix-cutscene-faces2`, cut-scene faces: merge onto master's facial rule

Base: `s/fix-cutscene-faces` (finished 2026-10-14 09:11, never merged) — merge origin/master into it FIRST. The merge
conflicts in `crates/nf-game/src/anim.rs` `script_pose_together` (two hunks) and
`crates/nf-view/tests/fix_cutscenes2_live.rs`: master (fix-facial3 + the exact-math swap) tells a facial window from a
body window by CHANNEL COUNT (`s.channels.len() != skin.facial`), while this branch uses the game's own rule — the
sequence flags byte bit 0x8 [M `AnimProcessScriptCmds` 0x1c1b38]. Keep ONE rule, the game's (flags & 0x8), applied
everywhere master used the count (`script_pose_drone` too), and keep master's exact-math code paths. Then verify the
renders: `nf-view <iso> 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 399 --shot` and `--frame 1200` (the Kimiko
scene: the full cast with moving faces) before and after, plus the 02 intro at `--bond -82.30,5.30,32.05,1.570,-0.100
--frame 170` (both actors present, lit dark). Run the full `tools/test/suite.py`; total in the report.

Files: `crates/nf-game/src/anim.rs`, the two test files.

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

**Branch:** `s/fix-cutscene-faces2`.
