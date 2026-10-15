## Your task: `fix-scores-handoff`, the NewScoresRef PTP hand-off (G-045 remainder)

G-045 (docs/GAPS.md; docs/systems/progress.md): the mission-progress layer is ported, but the on-foot score rows'
`NewScoresRef` PTP hand-off isn't. The port aborts the way the game's NULL path does. That's the data the end-of-level
results screen reads.

Do (non-visual, data only):
1. Find `NewScoresRef`'s writers and readers in the decompile: who allocates/fills it at level end, the PTP record
   layout (docs/formats has a PTP note: grep `PTP`), and the row format (stat id, value, medal state).
2. Port the hand-off: at level end the Sim produces the same rows the game would hand to the results screen
   (`progress.rs`), as a plain data struct. No UI.
3. Test (NF_ISO): finish 07000005 through the real exit path (`level_exit` via the truck ride test's helpers or a
   trigger), and assert the rows equal the game's formula for the stats the run produced. Sister: a run with no kills
   gives the zero rows.

Files: `crates/nf-game/src/progress.rs`, docs/systems/progress.md, the new test, docs/GAPS.md (G-045).

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


**Branch:** `s/fix-scores-handoff`.

---
claimed-by: sandbox1 at 2026-10-15T15:20Z

---
## Report (done, 2026-10-15T16:17Z)

