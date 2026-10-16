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

## Your task: `menu-e2e-singleplayer` — every single-player path from boot to a level request

By actions only, from boot:
- title → intro (skip, and also let it play out) → Main → NightFire;
- each Select Codename branch: New Codename (create + save), Play without saving, an existing saved codename;
- each difficulty → the mission list → every unlocked row.

Assert the exact `requested_level` each row produces against `CAMPAIGN` and the game's sp_level table [M].
The driving rows (0x0900000x) aren't ported yet. The main agent is adding a notice box (`FrontEnd::notice`) and
letting the on-foot row after a driving row unlock through it. Test that the notice shows, that Confirm dismisses
it, and that the next on-foot row is reachable. Then run the results path:
- `mission_ended` Complete/Failed → results / NfBonus / WinGame / MissionSelect routing;
- the save prompt;
- the next row unlocking;
- a restart from the results page.

Also cover Back at every step (the previous page, never a broken state) and a fresh profile vs `unlock_all`.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/menu-e2e-singleplayer`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
