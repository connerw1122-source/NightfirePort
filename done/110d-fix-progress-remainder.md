## Your task: `progress-remainder`, the mission-progress layer's open items

GAPS G-045 (read the row and `docs/systems/progress.md`): the progress layer is ported except (1) the "detected"
counter's live hook — find the vision-log call site that the game uses to count Bond being detected and wire
`PlrStat_Log*` there; (2) `Player_InitWeapon` 0x1a7ef8's per-difficulty autoaim distances (read them from the
decompile/ELF data and use them in the auto-aim); (3) the `NewScoresRef` PTP hand-off for the on-foot score rows (what
the port should do where the game aborts on NULL). NF_ISO tests for each: a guard spotting Bond increments detected
exactly as the game's rule says; the autoaim distance per difficulty; the score rows. Non-visual.

Files: `crates/nf-game/src/progress.rs`, `crates/nf-game/src/drone.rs` (the hook, additive), a new test file.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/fix-progress-remainder`.

---
claimed-by: sandbox3 at 2026-10-15T09:14Z

---
## Report (done, 2026-10-15T10:49Z)

