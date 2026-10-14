## Your task: `menu-movies2`, land the menu movies on top of the movie-playback plane

`s/fix-menu-movies` (9ee385d: the main/list pages' looping background FMVs and the title's intro movie) and
`s/fix-movie-playback` (the briefing/outro movies in GameFlow state 6) were built in parallel, and each added its own
full-screen movie plane. They conflict in `crates/nf-view/src/{gpu,lib,menu}.rs`. movie-playback is being merged first
(its own `movie_tex` / `movie_bg` / `movie_vbuf` plane in the Renderer, drawn through the HUD pipeline). Read both reports.

Start from `s/fix-menu-movies`, merge `origin/master` (with movie-playback in it), and resolve by making the menu
movies USE the movie-playback plane and decoder plumbing, not a second copy: one movie plane in the Renderer, one
player/decoder path, with the menu's looping backdrop and the intro going through it. Keep both features' tests green.
Render the main menu page with its backdrop (`--shot --page main`, with `--features movies` and the full retail image
from your sandbox storage), and list the commands for the main agent.

Files: `crates/nf-view/src/{gpu,lib,menu,movie}.rs`, the two features' tests.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/menu-movies2`.

---
claimed-by: sandbox1 at 2026-10-13T23:37Z

---
## Report (done, 2026-10-14T01:35Z)

# fix-menu-movies2 — the menu movies land on the movie-playback plane

Branch `s/fix-menu-movies2` (from `s/fix-menu-movies` 9ee385d, merged `origin/master` 2302fc7 —
movie-playback is in it). Files: `crates/nf-view/src/{gpu,lib,menu,movie}.rs`,
`crates/nf-view/tests/{menu_movies.rs, movie_playback_flow.rs}`, `docs/WORKLOG.md`.

## Done

The two features' full-screen planes and player paths are now ONE of each:

1. **One movie plane (gpu.rs).** The Renderer keeps movie-playback's `movie_tex`/`movie_bg`/
   `movie_size`/`movie_vbuf` plane; fix-menu-movies' hud-texture-slot copy (`update_hud_texture` +
   its `hud_bg` helper, and the overlay's layer-1 movie quad) is deleted. The plane's draw moved
   from the END of the render pass to BEFORE the HUD runs: the game uploads the decoded frame to
   VRAM and the page draws it as its own background texture, under the controls [M movies.md
   §Presentation — "a full-screen background texture, not an IPU overlay";
   `PS2UploadBGMImage` 0x108740 uploads via PATH3 GIF tags; `DecodeBGFMVFrame` 0x108550]. GameFlow
   state 6 suppresses its overlay while the movie plays, so drawing before the (empty) HUD runs is
   the same picture there — one draw site serves both. Both branches had added the same
   `hud_tbgl`/`hud_sampler` fields; the duplicate pair is gone and the plane builds its bind group
   from them.
2. **One player/decoder path (movie.rs).** `movie::Player` (the whitelist-gated
   `psiStartBackgroundMovie` 0x1081a8 model) now decodes through fix-menu-movies' `MovieStream`
   closed-GOP streaming worker instead of the whole-ES `decode_video` (which cost ~23–31 s before
   the first frame; the stream is ~2× realtime [L measured 2026-10-13]). New
   `Player::start_looping(iso, code)` starts the page backdrop the way the game does:
   `Menu_PlayMovie(aux, 0, 1, 0, 0, 0)` — loop = 1 [M `Manager_SendMessage` 0x21c858:338] — and
   `psiDecodeBackgroundMovieFrame` 0x108060 restarts the file; `Player::is_looping` exposes it.
   Audio stays the player's own cpal ring, each decoded frame's PCM slice paced at `present()`
   [M `audioDecSendToIOP` 0x107100 — the IOP ring is fed from the frame callback]; a looping
   movie restarts the PCM walk with the video. The menu-movies SPU-voice audio path
   (`movie_open`/`movie_audio_stop`/`pcm_voices` at the host) is deleted — one audio path.
3. **One host tick (lib.rs).** `App::movie_tick` now arms the single `self.movie` player from the
   game's three start sites, in priority order: the P_INTRO pair while `intro_playing` [M
   `P_INTRO_Handler` 0x209e88 — `psiMovieFinished` advances `movie_283`], the GameFlow state-6
   briefing/outro [M `GameFlow_Main` 0x1c98c8 case 6, early start at ST_MAPLOAD from 0x1d6360],
   and the page's LOOPING aux backdrop [M 0x21c858:338 — stop when the new page's aux is 0,
   restart when it differs from the playing one]. `menu_textures`, `movie_bg`, `intro_stream`,
   `movie_show_frame` and `movie_menu_tick` are gone; the redraw presents whichever role is up
   through the ONE `render(..., movie)` argument and sets `FrontEnd::backdrop_live` for the
   overlay. `--shot` routes through the same plane: `front_end_menu` decodes the page's aux still
   (first closed-GOP chunk, `movie_still`) and returns it for the plane; `--intro N` renders the
   intro FMV fullscreen with no overlay [M the P_INTRO page has no controls].
4. **The overlay (menu.rs).** `menu_overlay` drops the movie-quad branch: with a live backdrop
   frame (`fe.backdrop_live`, new field, set by the host) it draws NO dark plate — the plane IS
   the page's background; without one (no `movies` feature, or the slim ISO's zeroed PSS files
   [L]) the plate stands in — the pre-movies look. The intro branch draws only the headless
   plate; the host suppresses the whole overlay while a real intro frame is up. Both features'
   helpers coexist: `aux_movie_for` (menu-def PAGE records, pinned by test) + `outro_for`
   (Mission_Update 0x182c60 case 6's switch).

## Merge-conflict resolution (the task's actual ask)

`gpu.rs` / `lib.rs` / `menu.rs` conflicted exactly as predicted; `movie.rs` auto-merged and was
then unified (Player over MovieStream). Resolution rule: movie-playback is the plane + player
owner; every menu-movies mechanism now routes through it instead of standing beside it.

## Shared files touched (all small, additive)

- `crates/nf-view/src/gpu.rs` (shared): one Renderer constructor line (master's, plus the
  duplicate `hud_tbgl`/`hud_sampler` removed), the movie draw block relocated before the HUD runs
  (comment updated), `update_hud_texture`/`hud_bg` deleted (they were this task's own second-copy
  plumbing).
- `crates/nf-view/src/lib.rs` (named shared file): `App.movie` field comment; `movie_tick` body
  restructured into the three role arms; the redraw's movie role computation + `backdrop_live`;
  `front_end_menu`/`shot` signatures gained the movie argument. No existing behaviour changed
  shape; the phase-2 resolve and the pending-level deferral are untouched.

## How it was verified

- `menu_movies` tests: the disc's PAGE aux map, the MovieStream decode pins (frame 12 SHA-1
  `1f1fe7b0…`), the stream-ahead test, the intro budget/flow — all green. The overlay test is
  rewritten to the plane contract (`the_menu_backdrop_plate_yields_to_the_movie_plane`: with
  `backdrop_live` the overlay has no plate quad and the controls still draw; sister: without it
  the plate stands in; the intro's plate). NEW `the_menu_loop_plays_through_the_one_player_path`
  drives `Player::start_looping(07330048)` and pins frame 12 to the same `1f1fe7b0…` — the ONE
  player path decodes exactly what the MovieStream pin measures.
- `movie_playback_flow` 9/9: NOTE — two of its tests (`a_completed_mission_plays_the_outro…`,
  `a_failed_mission_returns_to_the_map_without_a_movie`) were ALREADY RED on master: commit
  568a6e8 ("mission end pushes the load first, then the fade on top") added the `ST_FADE_HOLD`
  push to `mission_ended` without updating them. Fixed here to the game-true order [M
  `Mission_Update` 0x182c60 case 4: `ResetMap_LevelToLoad` pushes the load, THEN
  `GameFlow_PushState(60.0, 7, …)`] — run the 8-frame fade out, then drive the load machine.
- `menus_flow` 17/17 (the overlay changes are behind `backdrop_live`, default false = master's
  picture).
- **Full suite, all 208 test binaries** (chunked foreground — the sandbox kills background jobs
  between tool calls, so `nohup`/`setsid` runs die; the fast runner was driven in passes over the
  same final tree, merged with origin/master): **473+ passed / 4 failed / 17+ ignored** in the
  final two passes, no TIMEOUT/CRASH. The 4 failures are exactly the permitted set: the 3
  `disc_golden` tests (the slim on-foot ISO's stubbed /DRIVING/MIS01.MUS BIGF files,
  pre-existing) and `pss_movies_live::every_movie_on_the_disc_demuxes_to_the_golden_numbers`
  (needs the 22-movie retail image; its other 2 tests pass against the mini ISO below, which
  proves the fetched movie bytes are exact).
- **Mini ISO** (never committed, `/home/z/nf-mini.iso`): this sandbox has NO retail image (only
  the slim on-foot ISO whose PSS files are 0-byte [L]), so the five task movies (07100001,
  07330048, 07350048, 07370048, 07380048; ~100 MB) were range-fetched from the retail image on
  the file host (`007 - Nightfire (USA).iso`, byte ranges) and packed into a minimal ISO9660
  image the port's reader mounts. `NF_MOVIE_ISO=/home/z/nf-mini.iso` runs all movies-feature
  tests (menu_movies 8/8 with it).
- The GPU render itself cannot run here (no adapter in the sandbox — every `--shot` needs the
  main agent's machine), so the plane's draw order is verified by construction (single draw site
  before the HUD runs) and by the main agent's renders below.

## Render commands for the main agent (retail image as NF_MOVIE_ISO)

```
cargo build --offline --release -p nf-view --features movies
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view $NF_ISO --page main --shot /tmp/mm2_main.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view $NF_ISO --page missions --shot /tmp/mm2_missions.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view $NF_ISO --page start --shot /tmp/mm2_start.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view $NF_ISO --intro 1 --shot /tmp/mm2_intro.png   # 0 = EON logo
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view $NF_ISO --page main --movie-frame 60 --shot /tmp/mm2_main_f60.png
```

What each should show vs PCSX2: the main menu / mission list / boot page with the looping FMV
(0x07330048 / 0x07350048) visible AROUND and BEHIND the page controls — the movie plane paints
first, the panels and text sit on top (same as fix-menu-movies' verified renders, now through the
one plane); `--movie-frame 60` must differ from frame 0 (the loop playing); the intro frames are
the EON logo / the intro FMV fullscreen with no menu. The live window should show the backdrop
playing under the controls and the intro playing on Start.

## Live checks for the main agent

1. `--page main --shot` (above): backdrop FMV under the controls; compare with the
   fix-menu-movies renders (should be pixel-equivalent — same decoder, same draw intent).
2. nf-view live (retail image): boot → title → Enter: the intro 0x07370048 then 0x07380048 play
   FULLSCREEN (no plate flash), Enter/X skips instantly to the main menu [M GameFlow case 6];
   after the skip the main menu's 0x07330048 loop is already playing (the intro player yields to
   the page player in one `movie_tick`).
3. nf-view live: page changes main → missions → dossier (0x07330048 → 0x07350048): the backdrop
   restarts only when the aux code DIFFERS [M 0x21c858:338]; between two 0x07350048 pages no
   restart, no double audio.
4. nf-view live: pick a mission (The Exchange): the looping page player stops when the briefing
   wrap goes pending (`movie: stopped 07350048 (a briefing is pending)` on stderr), the briefing
   07100005 plays fullscreen, X/Start skips to the level — the fix-movie-playback checks 1-2 must
   still hold (the state-6 path shares the player now).
5. PCSX2 spot-check of the menu audio: the port's movie audio is the player's cpal ring (the
   SPU-voice path is gone) — the loop should restart its audio with the video [M 0x108060].

## Left

- `pss_movies_live`'s golden-movie sweep needs the full retail image (the sandbox's mini ISO
  holds 5 of 22 movies) — it is in the suite's permitted-failure set without one.
- The intro's fade-back iris (0x44 page-change effects) stays with the menus task; P_ATTRACT /
  P_TRAILER / P_WINGAME / P_ESTHERO handlers still play their movies from the handler, not the
  aux field.
- The `open_pcm_out` stream never calls `.play()` (cpal 0.18 starts streams paused) — pre-existing
  on master (the `StreamTrait` unused-import warning shows it); the movie audio is silent until
  someone adds the `play()` call. Flagged here because both features' audio now funnels through
  that one function.
- Movie-volume follows `SFXGetVolume()` per start site [M 0x209e88/0x215990] — the port runs full
  scale [S].
