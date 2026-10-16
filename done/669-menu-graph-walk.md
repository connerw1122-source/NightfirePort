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

## Your task: `menu-graph-walk` — explore the whole menu graph and enforce invariants

Write a headless explorer that starts from `FrontEnd::new()` and does a breadth-first walk over the reachable
menu states. A state is (page, cursor, the page's own sub-state), and the edges are every `Action` plus the
ticks. Cap it by a visited set plus a depth bound, and print the coverage (pages reached / all pages).
Invariants checked at every step:
- no panic;
- `cursor < rows()` on every rowed page;
- `page.id()` is a known page id;
- Back from every non-root page leaves the page within N steps;
- every reached page can get back to Main by actions only;
- `requested_level` is never set twice for one confirm;
- no page is a dead end (one where no action changes anything and Back doesn't leave).

Any page the walk can't reach, or any page that only works through direct field writes, is a finding. Fix the menu
code where the game's evidence is clear [M]; otherwise list it in the report. Also add a randomized version: 20
seeded runs of 5000 random actions and ticks, with the same invariants.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/menu-graph-walk`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T06:20Z

---
## Report (done, 2026-10-16T08:04Z)

Closed by the driver: the report is on `s/menu-graph-walk` (docs/reports/menu-graph-walk.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T08:28Z)


---
## Report from the worker (done, 2026-10-16T08:29Z)

