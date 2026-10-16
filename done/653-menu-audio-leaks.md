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

---
## Report (done, 2026-10-16T04:05Z)

# menu-audio-leaks (653) — the menu-music pair's SFXRemove side + the backdrop leaks

**Status: done** for the playtest audio leaks (results id 1015-1738); the "old menu with no
background" gets a flow fix + a live-check ask. Branch `s/menu-audio-leaks` from origin/master
(306c5d93).

## Files

- `crates/nf-view/src/menu.rs` — the front end's remove channel: `sfx_remove_out` (own line),
  `remove_pair()` = `SFXRemove(0,0x1d7)+(0,0x470)` [M the twelve handlers' 0x4c arm removes
  before restarting — 0x20a408:17-22; P_INTRO 0x209e88:38-39,53-54; P_ATTRACT 0x20a1f0:45-46],
  `drain_sfx_removes()`; `open_pair_sfx` removes before it restarts (the gate kept); the intro
  start silences the pair + clears the static [M P_INTRO's 0x50 frame-1] and the intro end
  restarts it [M :23-29]; `attract_open` kills the pair [M 0x20a1f0:16-35],
  `attract_movie_finished` removes+restarts [M :45-50]; `open_pause` zeroes `menu_movie`
  [M MenuManager_Create 0x21b908:125 — P_PAUSE itself starts no pair, M-negative 0x203c28].
- `crates/nf-view/src/lib.rs` — the host carries the removes BEFORE the same frame's starts
  (the 0x4c order [M]); `close_front_end()` removes the pair through the driver when the front
  end closes (a briefing resolve, the pause's pick, the resume) — the game's exit lets the
  menus fade + the level's `SFXLoadSoundBank` reset the items [M C_SBNFMAP 0x20b8d8 case 0x4b,
  no explicit remove]; the port's banks swap without that reset, so the close removes the pair
  explicitly (KillSFX-all semantics [M IRX 0x1f88 → 0x1ea4]). `movie_tick` stops the looping
  page backdrop when `self.fe` is None (no page wants it — the game's page-change stop arm
  0x21c858:334-338 with no page at all) and state 6 replaces a looping occupant of the one
  BGFMV slot [M psiStartBackgroundMovie 0x1081a8] — the backdrop (the menu music) no longer
  plays under the EA logo or on into gameplay.
- `crates/nf-audio/src/sfx.rs` — 2 tests (no behaviour change: `Driver::remove` already
  implements KillSFX-all).
- `crates/nf-view/tests/menu_sfx_flow.rs` — 5 headless flow tests.

## Why the playtest heard what it heard (the decomp map)

- `SFXRemove(0, id)` removes **every** instance of the id (KillSFX walks the item list, an
  immediate key-off) [M IRX 0x1f88/0x1ea4]; the pair is **looped by design** and dies only by
  removal. The port only ever STARTED it → every page open stacked another looping item ("a
  lot of different music songs playing"), and the last one played on through gameplay and the
  pause ("music continues after I leave the menu").
- In the real game the pair is **silent under the intro movies**: P_INTRO's 0x50 frame-1
  removes it and only the movie-2 end restarts it [M 0x209e88:37-47,23-29]. The port started a
  fresh pair exactly when the intro began (the leak under the EA logo).
- The pause page never touches the pair; `MenuManager_Create`/`Delete` pause the whole mixer
  and zero `menu_movie` [M 0x21b908:121-125, 0x21c2b0:11-15]. The port now zeroes the static
  on the pause open; the sound hold (`Sound::set_paused`) already covers the mixer pause.
- The boot order [M bootup 0x1c7e78 → P_LANGUAGE → P_PS2MEMCARDINIT → P_INTRO → P_START]
  starts no pair before the intro; the port's own flow (Title → intro → Main [L menus_flow])
  keeps its boot pair but the intro start now removes it.

## Verification

- The crates my tests touch, on the final tree: **nf-audio lib 11/11** (+`remove_kills_every_
  instance_of_the_id`, +`a_bank_miss_starts_nothing` [M the IOP lookup miss]); **nf-audio
  integration bins all green**; **menu_sfx_flow 14/14** (5 new: the boot 0x4c's remove+start
  order, the intro start's silence + static clear, the movie-2 end's remove+restart, the
  attract end's remove+restart, the pause open's no-pair/static-clear); **menus_flow 42/42**
  (the intro-start page flip contract held), **mp_menu_flow 9/9**, **nf-view lib 15/15**.
- Full suite (fast runner) on the branch-point tree: `1622 passed, 7 failed, 66 ignored
  (62 min)` — the 7 are the 3 allowed `disc_golden` slim-ISO fails, the 3 documented pylon
  Vulkan-ICD fails, and the pre-existing `walkreal_07000001_live` LEAD_HIDE budget flake. My
  changes landed after that build; the touched-crate runs above are the final-tree evidence.

## What's left

- **Finding 4's visual confirm**: with state 6 replacing a looping occupant, the intro movies
  actually start now; the flow's post-intro page is Main with its 0x07330048 aux
  (`aux_movie_for`, [D the menu-def PAGE records]). The headless tests can't see pixels — the
  main agent renders.
- The level-music player on the front-end close (`Sound.music`): the game's exit fade
  (SFXFadeDown(1)) + the level's own music start replace it; the port's next level start does
  replace the player, but if the playtest still leaks after this round, that path is next.
- The front-end bank's stream data for 0x1d7/0x470: if the port's `Sound.streams` lacks the
  front-end stream file, the pair items are silent (the user's "menu music" may have been the
  backdrop movie's own soundtrack — which the backdrop-stop fix handles). One live check
  settles it.

## Live checks for the main agent

1. Boot → title → confirm → the EA logo: expect **no menu music under it**; after movie 2 the
   pair restarts once (no stacking). Watch for `movie: stopped … (state 6 replaces the
   backdrop)` in the port log.
2. Menu-browse for a while, then open a mission: the menu music must stop when the front end
   closes (`close_front_end`); nothing sounds during gameplay.
3. Pause mid-mission: silence (the sound hold); after unpause, no menu music returns until a
   front-end page reopens.
4. The post-intro screen: the main menu **with its 0x07330048 backdrop** (the playtest's "old
   menu with no background" gone). If a wrong page still shows, capture the page id.
5. `Driver::ids()` over a long menu session: at most one item per pair id at any instant.
