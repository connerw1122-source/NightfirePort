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

## Your task: `menu-mouse-parity` — the mouse works everywhere the keyboard does

For every page, assert:
- (a) `mouse_move` over each drawn row (the hit rows the draw code uses: `list_rows_y`, the wheel geometry, the
  tab strip) sets the same cursor the arrows would;
- (b) `mouse_click` on a row does what Enter does on that row;
- (c) clicks outside rows do nothing harmful;
- (d) right-click = Back and the wheel = Up/Down;
- (e) no click ever changes a value it shouldn't. The Host page's port was knocked down by every click until the
  main agent fixed it.

Fuzz it: random mouse moves and clicks interleaved with keys, 20 seeded runs of 5000 steps each, with the
invariants from task 669 (no panic, cursor in range, Back leaves). Report and fix pages where the mouse targets
disagree with the drawn rows (compare against the draw code's positions; the main agent owns the drawing, so
change hit-test numbers only, with the draw source cited).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/menu-mouse-parity`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
