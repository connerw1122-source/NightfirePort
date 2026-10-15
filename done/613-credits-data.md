## Your task: `credits-data`, the credits roll's content and timing (no drawing)

AV Options → Credits (and the end of the game, `P_WINGAME_Handler` 0x215100) play the credits page
`P_CREDITS_Handler` 0x2152b0. The main agent draws it; you port **what it shows and when**.

Deliver:
1. `docs/systems/credits.md`: where the credit lines come from (label ranges in USATxt.dat or a table — cite the
   address), the line kinds (heading / name / gap / image) and their fonts/colours/positions as the handler sets
   them, the scroll speed and timing (per frame, in the game's 512×448 units), the end condition and what the page
   returns to, and any music/movie it starts.
2. In the port: a credits model in `crates/nf-view/src/menu.rs` (or a new module): the ordered line list built from
   the game's data, and a `tick()` giving each visible line's y at a given frame, plus the exit; hook it to a
   `Page::Credits` the main agent can draw (state + getters only).
3. Tests: the line count/order matches the data (numbers/ids only), the scroll position after N frames matches the
   handler's rate, the roll ends and returns to the right page; each with a sister assertion that can fail.

Not in scope: drawing (the main agent does).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/credits-data`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-15T18:24Z

---
## Report (done, 2026-10-15T19:13Z)

