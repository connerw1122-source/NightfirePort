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

## Your task: `menu-audio-state` — the menu music/sfx state is right through every flow

The user: after watching the whole intro "audio breaks and there's no main menu music"; the attract trailer
played without its own audio (fixed by the main agent: the attract now removes the jingle pair like P_ATTRACT
0x20a1f0:20-22); menu music kept playing in gameplay and during the pause. Using `menu_music.rs`
(`MenuMusic` + its `take_cmds`) and the host glue in `lib.rs` (`movie_tick`, `menu_after`, the page-enter
music hook), drive these flows headless by actions and assert the music command stream and the jingle state at
each point against the game [M frontend-music.md]:
- boot: intro played fully / skipped → title → Main;
- the attract (natural end and skipped);
- every page with a backdrop movie;
- the credits;
- SP mission start (the music must stop for the level);
- the pause page (held, not removed);
- quit to the front end;
- MP match start and the debriefing.

Invariant: never two menu-music sources at once, and none during gameplay. Where the host glue can't be driven
headless, extract it into a testable function.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/menu-audio-state`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T07:00Z

---
## Report (done, 2026-10-16T08:30Z)

Closed by the driver: the report is on `s/menu-audio-state` (docs/reports/menu-audio-state.md); the worker didn't run finish.sh.
