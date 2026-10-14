## Your task: `menu-movies2`, land the menu movies on top of the movie-playback plane

`s/fix-menu-movies` (9ee385d: the main/list pages' looping background FMVs and the title's intro movie) and
`s/fix-movie-playback` (the briefing/outro movies in GameFlow state 6) were built in parallel, and each added its own
full-screen movie plane. They conflict in `crates/nf-view/src/{gpu,lib,menu}.rs`. movie-playback is being merged first
(its own `movie_tex` / `movie_bg` / `movie_vbuf` plane in the Renderer, drawn through the HUD pipeline). Read both reports.

Start from `s/fix-menu-movies`, merge `origin/master` (with movie-playback in it), and resolve by making the menu
movies USE the movie-playback plane and decoder plumbing, not a second copy: one movie plane in the Renderer, one
player/decoder path, with the menu's looping backdrop and the intro going through it. Keep both features' tests green.
Render the main menu page with its backdrop (`--shot --page main`, with `--features movies` and the full retail image
from your sandbox storage), and list the commands for the main agent.

Files: `crates/nf-view/src/{gpu,lib,menu,movie}.rs`, the two features' tests.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/menu-movies2`.

---
claimed-by: sandbox1 at 2026-10-13T23:37Z

---
## Report (done, 2026-10-14T01:35Z)

