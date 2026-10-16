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

## Your task: `menu-e2e-local-mp` — every local multiplayer path to a match request

By actions only:
- Main → Multiplayer → Local → join (codename, ready);
- for EVERY scenario row (Quick Game too), EVERY map row and both team/FFA setups: setup (team → character →
  handicap) → options hub (each sub-page: Rules, Player Mods, Enviro Mods, Bots with bot setup) → Continue →
  Confirm → `requested_level` + `mp_match_settings`.

Validate the settings against the choices made, and check that a second Confirm while a request is pending is
refused (the main agent's guard). Then back out from EVERY page at EVERY prompt and walk forward again: the setup
page must re-enter on the right prompt for the scenario's team bit [M P_MPSETUP 0x20d7a8]. Check that locked
rows/scenarios refuse and that the debriefing returns to the scenario page. Anything stuck is a finding: fix it
with [M] evidence where the game's flow is clear.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/menu-e2e-local-mp`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T06:48Z

---
## Report (done, 2026-10-16T08:28Z)

