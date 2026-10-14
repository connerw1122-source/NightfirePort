## Your task: `objective-drives`, campaign: the gameplay-driven objectives

docs/reports/campaign-flow.md blocker 6: the campaign runner can't fire 09's ch71 (2/3), 08's ch75 ("Escape with Zoe
in the gondola", 1/2), 4a's ch74, and 0a's objective inputs 0x48/0x49 (the Q-Worm install script player's out 72 and
the security-centre zone; fix-level-chain's report). For each, find the live writer in the level data and the game
code (script players, touch zones, gadget use), and drive it in the runner through the real input path (walk there,
use the gadget, ride the gondola) so each level completes its objectives and chains. Fix any port bug that stops the
writer. Tests (NF_ISO): each of the four completes through the runner; sister assertions.

Files: `crates/nf-view/tests/campaign_flow_live.rs`, fixes as found.

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

**Branch:** `s/objective-drives`.

---
claimed-by: sandbox5 at 2026-10-14T19:40Z
