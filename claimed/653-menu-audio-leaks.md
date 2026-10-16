The user's playtest 2026-10-15 (results id 1015-1738) said: - "during the EA Games logo movie, the main menu music was playing ... it seems like there's a lot of different
  music songs playing";
- "when I paused, I heard menu music";
- the pause menu: "I'm still hearing the music play, and it continues to play even after I leave the menu";
- after letting the EA movie play out, "It skipped itself to one of our old menus where there's no background".

Main agent findings so far (master 0f5f07f4):
1. `App::movie_tick` (crates/nf-view/src/lib.rs) returns early when `self.fe` is None. A looping page-backdrop
   movie (0x07350048 / 0x07330048, which carries its own soundtrack — the menu music) is therefore never stopped
   when the front end closes without a briefing movie (the phase-2 resolve sets `self.fe = None`). It plays on
   through gameplay and the pause. Fix: stop a looping backdrop whenever no front-end page wants it.
2. The page-open sfx pair `SFX_MENU_OPEN_MOVE` 0x1d7 (spare slot 0) / `SFX_MENU_OPEN_SELECT` 0x470 (spare slot 1),
   started on 07000005 through `Driver::start(out, id, None, 0, None)`, returns 0 and emits NO voice command, yet
   the item stays in `driver.ids()` forever (alive after 120 `update` frames with every voice reported ended).
   Find why (a streamed effect with no stream data? a spare-bank path that never keys on?). Make the items play
   and end the way `SFXStart` does [M], or not start at all when the game's lookup would miss.
3. Find every other path where a sound or music source survives the front end closing or a pause: the intro pair
   vs the backdrop (the 212 report: music under the EA logo, several tracks at once), and the level music vs the
   front-end map.
4. Reproduce the "old menu with no background" after the intro ends (the intro_index / state-6 pop / first page
   path) headless, and fix it from the game's flow [M].
Tests: headless flow tests for each case (movie player state, driver items, pages) with sister assertions. No
drawing changes. Tests must stay silent (NF_SILENT).

UPDATE (main agent, later the same day): 625 attract-mode's report identifies 0x1d7/0x470 as **the front-end menu
MUSIC pair**, not blips. The game removes it with `SFXRemove(0,0x1d7)` / `SFXRemove(0,0x470)`: on every
page-open before restarting it [M the twelve handlers' msg-0x4c], on the attract open [M P_ATTRACT_Handler
0x20a1f0], and wherever the front end ends. The port's `open_pair_sfx` only ever STARTS it; nothing removes it
when the front end closes (into a level, after the intro, under the pause). That matches the user's "music
continues after I leave the menu" and the never-ending driver items in finding 2. Port every SFXRemove/stop site
for the pair (front-end exit to a level, the intro/EA movies, attract, and whatever the game does on the pause
page), and make the host carry it out through the driver (a remove event beside `drain_sfx`). Check how 625's
attract code on master handles the pair and build on it rather than duplicating it.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/menu-audio-leaks`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T02:52Z
