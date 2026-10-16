# menu-audio-state (672) — the menu music/sfx state through every flow

**Status: done** for the menu-music state machine + the host glue, headless-verified through
every flow the playtest named. Branch `s/menu-audio-state` from origin/master (ad761677, still
master's head — no merge needed).

## The symptom

After watching the whole intro "audio breaks and there's no main menu music"; menu music kept
playing in gameplay and during the pause (the earlier rounds' fixes are on master). The flows
were never exercised end to end against the music state.

## What the headless flows found and what changed

The harness: `crates/nf-view/tests/menu_audio_flows.rs` — an `App` stand-in (the `fe` is
`Option`, the ONE BGFMV slot is a simulated player) that drives the FrontEnd ONLY by
`action()`/`tick()` and the host-side calls the window makes: `menu_after`'s sfx drain, the
movie glue's music calls at `movie_tick`'s sites, the page-enter hook (below), the async-load
swap's front-end arm, the pause's `get_or_insert_with`. A driver model mirrors the IOP surface
(items per id with KillSFX semantics, the resident bank deciding audibility [D SBINFO.SBI +
SB_0.SFX/SB_1.SFX], the music-system track, the volume). Invariants assert on EVERY frame:
at most one audible menu-music source, never jingles + the credits track, and none during
gameplay (the `StartJingle` gate over a mission bank [D SB_12 carries neither jingle]).

Bugs the walk found, all fixed:

1. **The jingle played UNDER the intro movies.** The port flips `fe.page` to Main at the intro
   START (the menus_flow draw contract), so the page-enter hook fired Main's 0x4c and restarted
   the pair while the EA logo was up. The game's up-page is P_INTRO until the movies end [M
   0x209e88's page change follows the movies]. Fix: the extracted `fe_music_step` holds the
   page transition while `intro_playing`; the intro-end restart (open_pair_sfx) collapses the
   pair, and the real page enter after it sends nothing [M 0x20a7b0's gate].
2. **The main menu sat SILENT after a skipped intro.** The direct-action skip arm cleared
   `menu_movie` but never restarted the pair; the window's own skip path (the state-6 intercept
   → `flow.skip_movie`) funnelled through tick()'s intro-end block, whose sfx-channel restart
   drained only at the NEXT keypress, and the music channel's `jingles_up` mirror was left
   down. The game's StopMovie makes the next 0x50 see psiMovieFinished and the finish arm
   restarts the pair [M 0x209e88:23-28]. Fix: the skip arm runs that restart, and
   `open_pair_sfx`/`remove_pair` now ALSO drive the music channel — the restart lands at the
   next REDRAW, not the next input (the sfx channel drains only in `menu_after`).
3. **The pair played under the briefing after a mission pick.** The game DELETES the menu
   manager at the pick [M C_SBNFMAP case 0x4b; MenuManager_Delete 0x21c2b0:11-15]; the port's
   manager only goes at the async swap, so the jingle (looped by design) played through the
   briefing and the load. Fix: the SP pick arm and the MP match start kill the pair there
   (`menu_movie = 0` + `remove_pair`, the delete's reset [M 0x21c2b0:15]).
4. **The bank alternation reset at the pause.** The gameplay pause creates its manager with
   `get_or_insert_with(FrontEnd::new)` — a FRESH `MenuMusic` — so the quit-to-front-end load
   re-played the boot's jingle instead of the alternation's other one. The game's
   `DAT_002e1b11` is a .bss byte that outlives every manager (and rides the PTP data across
   boots) [M 0x1dceb8:94-101, Boot_LoadPTPData 0x1c8560:61]; `MusicMasterVolume` @ 0x30c958 is
   the same kind of static [L]. Fix: the App stashes the statics on every manager drop
   (`close_front_end`, the swap's `fe = None` arm) and restores them into every create (the
   pause's `get_or_insert_with`, `demo_results` — which previously copied from a fe that is
   long gone by the time the results open).
5. **The credits enter started the pair for under a frame.** `enter_page(Credits)` was on the
   pair-restart list, but the credits' 0x4c is the SWAP: the pair removed, `menu_movie = 0`,
   `SFXStartMusic(0x27, 0)` [M 0x2152b0:89-94]. Fix: the credits clear the static only; the
   swap runs in `MenuMusic::page_enter`. `open_pause` also mirrors `menu_movie` into
   `jingles_up` [M 0x21b908:125's reset].

Also: `menu_music_tick`'s transition+drain is extracted as `fe_music_step`
(menu_music.rs) — the headless flows drive the SAME glue the window runs, and the window's
wrapper only applies the returned commands to the driver.

## Verification

- `cargo test -p nf-view --test menu_audio_flows` — 9 flows, each with a sister that can fail:
  boot (intro full → the between-movies restart → Main, one 0x1d7 instance, no 0x470 start);
  the skipped intro (both skip paths — the pair is up on Main); the attract (natural end AND
  skip — the pair removed under the trailer, back after); the backdrop pages vs the group-C
  list-page walk (the pair survives, no restart while up); the credits swap (the pair out,
  track 0x27 in, the ENTRY volume restored on the leave, the pair back); the SP mission start
  (the pair dead from the pick through the briefing and the load, the results restart GATED
  over the mission, the front-end reload flips to 0x38 and 0x470 sounds); the pause (the hold
  up, no jingle sounds, the resume silent); quit to the front end (the fade-down, the same
  manager walks to Main, the flip, the OTHER jingle); the MP setup chain (nothing restarts) +
  the debriefing gate over the arena.
- The pre-existing suites the change touches (menus_flow, menus_e2e, mp_menu_flow,
  menu_sfx_flow, menu_music_flow) run clean; `menu_music_flow`'s quit-fade unit test was
  updated for the constructor's boot-pair queue (menu-audio-state).
- The fast runner on the final tree: see the total line below.
- **What ran before the push:** the nf-view menu binaries above; the fast runner on this tree
  (its per-binary lines are in `suite.out` at the repo root of the sandbox).

## What's left

- The MP match-end → debriefing host glue does not exist yet (no headless path reaches
  Page::MpDebriefing); the page's own 0x4c restart + the over-the-arena gate are tested at the
  page level. When the glue lands, the flow test is ready to extend.
- The mission-event → results glue is still the demo hook (F8/F9) on this tree; the
  results-over-mission gate is tested through that stand-in (the same `fe_music_step` the real
  event would drive).

## Live checks for the main agent

1. Boot, skip the intro immediately after it starts: the main menu music must come up AT ONCE
   (no keypress needed) — this was the silent-after-skip bug.
2. Watch the whole intro: no menu music under the EA logo or the second movie; the jingle
   returns between the movies and stays on Main.
3. Idle on the main menu: the attract trailer plays with its OWN audio (no jingle under it);
   after it (or after skipping it), the jingle is back without a keypress.
4. Start a mission: from the pick through the briefing and the loading screen, silence; no
   menu music in gameplay or under the pause; nothing returns on resume.
5. Pause → Quit: the 2 s music fade-down, then the main menu with the OTHER jingle of the pair
   (the alternation survives the pause's fresh manager).
6. `Driver::ids()` over a long menu session: at most one instance per jingle id at any instant.
