Context (2026-10-15): the user's playtest found the menus broken end to end even though every page
had passed its own unit tests.
- No single-player mission could be started.
- Local MP couldn't reach a match.
- Backing out of MP setup left the wrong prompt.
- A second Enter during a load started a second load.
- A notice box could never be dismissed.

The existing flow tests set `fe.page = ...` directly instead of getting there by actions, so the real paths were
never exercised. **Rule for this task: drive the FrontEnd ONLY through `action()`, `mouse_*()`, `tick()` and the
host-side calls the window makes. Never assign `page`/`cursor`/state fields to get somewhere.** The movies and the
attract need ticks to finish: after each action, tick until `!intro_playing && !attract_playing` (bounded). Every
test needs sister assertions.

## Your task: `pause-results-flows` — the in-mission menus, end to end

By actions only, on a live 07000005 scene (NF_ISO) with the host glue where it's reachable headless:
- the pause page: Esc open, every tab (Mission / Objectives / Controls / Score, Left/Right and Tab), Continue,
  Restart → YES / NO, Quit → YES / NO;
- the fail menu (die): Restart / Quit;
- mission complete → results → NfBonus/MissionSelect.

Assert the page and request at each step, that the sim is paused while a menu is up, that Restart gives full health
and the entry checkpoint, and that Quit returns to the front end with the menu working. Also cover repeated presses
during the transitions (no double requests), and the 5 s loading-screen hold (`LOADING_HOLD`): input during the
load is ignored.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/pause-results-flows`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
