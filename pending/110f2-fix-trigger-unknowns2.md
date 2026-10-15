## Your task: `fix-trigger-unknowns2`, finish 110f (held at merge: the sight pause breaks two live-matched tests)

Branch `s/fix-trigger-unknowns` (110f) was HELD on 2026-10-15 06:30 MST. Start from it, merged with `origin/master`.

The main agent's full suite on master + 110f: **1300 passed, 2 failed**:
- `nf-game/tests/ai_live.rs::guard_notices_and_fires_like_the_game` — `noticed after Some(64) ticks (live 61)`. Master
  matches the PCSX2 measurement (61); the ported 15-frame sight pause makes it 64. The pause is either not where the
  game applies it, starts from the wrong stamp, or shouldn't apply to this guard's first sight.
- `nf-view/tests/walkreal_07000001_live.rs::walk_from_the_spawn_to_the_exit_with_the_contact` — `the contact must walk
  his route to the LoadPointActivator plate` (line 218). The escort run breaks.

Do: re-read `DroneVision_ProcessDroneSight` 0x176a28:68-69 and the +0xfb8/+0xfbc stamps (who writes them and when; the
initial values at drone init; whether the pause gates first notice or only re-sighting). Fix the port so both tests pass
with the pause still faithful, and explain in the report why 61 is right. If the pause truly delays first notice in the
game, the 61 measurement must contradict it: find out which is wrong with the trace (work/ai_trace_a.tsv /
ai_trace_b.tsv hold the live sequences). Run the FULL suite (not just the "affected" binaries) on the final tree and
report its total line.

Files: the branch's files.

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


**Branch:** `s/fix-trigger-unknowns2`.
