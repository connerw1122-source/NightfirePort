## Your task: `cutscene-models`, REGRESSION: cut-scene characters are missing

The user's playtest (results 1013-1758): in the 07000008 briefcase/safe cut-scene "Kiko is the only one showing besides
me... I can't see any other character models. Very concerning because I used to see some of them." And on 07000007:
"The only model that's consistently showing up in cutscenes is my player character, Bond." This is a REGRESSION: an
earlier playtest showed the actors. Candidates merged in the last two days (`git log --merges origin/master`):
fix-script-commands (0a7268d, decoder trailing byte), fix-facial / fix-facial2 (366ceff, bc231e8), fix-anim-blend2
(e679d6e), fix-gpu-skinning (942e2ca: GPU palette path), fix-portal6 (8ee0791), fix-spawners / spawner-remainder,
fix-cover/nav-remainder.

Bisect it: write a headless test that runs each cut-scene on 07000005/06/07/08 (start the scene through its real trigger
or `Script_AnimStart`) and counts the characters actually DRAWN (the scene's draw list: skinned meshes with a non-empty
palette, inside the view frustum, not culled), compared with the actors the script creates. Run it on master and on the
commits before each candidate merge (`git worktree add`, a separate target dir) to find the merge that broke it, then
fix the cause (with the game address the right behaviour comes from). Also check headless `--shot` renders of a
mid-scene frame show the actors. 291-fix-cutscene-actors (pending) is about actor positions/skins in general: this task
is the regression only; note in its report what's left for 291.

Files: whatever the bisect points at; the test `crates/nf-view/tests/cutscene_models_live.rs` (new).

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

**Branch:** `s/cutscene-models`.

---
claimed-by: sandbox4 at 2026-10-14T01:09Z
