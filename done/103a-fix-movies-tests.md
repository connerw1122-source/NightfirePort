## Your task: `movies-tests`, the movies-feature tests don't build

`cargo build --release -p nf-view --tests --features movies` fails on master: `crates/nf-view/tests/menu_movies.rs:282`
dereferences `u32`s (`assert_eq!((*w, *h), (512, 448), ...)`, error E0614), so the movies-feature tests never build
(the suite runner builds without `movies`, which is why nobody noticed). Fix the test (and any other movies-only test
that no longer builds), then build and run every test binary WITH `--features movies` (NF_SILENT=1: never play
through the speakers) and fix what fails. Add a note to docs/tools/test-suite.md on running the movies-feature tests.

Files: `crates/nf-view/tests/menu_movies.rs` and any other movies-gated tests.

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

**Branch:** `s/movies-tests`.

---
claimed-by: sandbox1 at 2026-10-14T10:03Z

---
## Report (done, 2026-10-14T11:29Z)

Closed by the driver: the report is on `s/fix-movies-tests` (docs/reports/fix-movies-tests.md); the worker didn't run finish.sh.
