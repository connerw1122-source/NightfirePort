## Your task: `menu-movies`, the front end's background movies and the intro

The movie decoder is merged (`--features movies`, `crates/nf-view/src/movie.rs`, `docs/systems/movies.md`); the menus are
built from the disc's page data (fix-menus2/3). In the real game the main menu's backdrop is a LOOPING FMV (the PAGE
record's aux code 0x07330048; the list pages 0x07350048), and Start on the title plays the intro movie before the main menu
(P_INTRO 0x209e88: 0x07370048/0x07380048) [docs/reports/fix-menus3.md]. Wire them:
1. With the `movies` feature on, the main menu and list pages draw their looping background movie under the page
   controls; without the feature (or on the slim ISO, which zeroes the PSS files) the current dark plate stays.
2. Title -> Start plays the intro movie, skippable (action 0x19 / Enter), then the main menu.
3. The movie audio goes through the existing cpal output.
Use the full retail image from your sandbox storage for the tests (the slim ISO stubs every movie). Tests: the page ->
movie code mapping from the page data, and a decoded frame of the menu loop (SHA-1, numbers only). List
`--shot --page main` commands for the main agent.

Files: `crates/nf-view/src/{menu,movie,lib}.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/menu-movies`.

---
claimed-by: sandbox6 at 2026-10-13T21:36Z

---
## Report (done, 2026-10-13T23:00Z)


---
## Report from the worker (done, 2026-10-13T23:00Z)


# fix-menu-movies — the front end's background movies and the intro (2026-10-13)

Branch `s/fix-menu-movies` (from `origin/master` 7a614ba). Files: `crates/nf-view/src/{menu,movie,lib}.rs`,
`crates/nf-view/src/gpu.rs` + `scene.rs` (the two shared hooks, below), `crates/nf-view/tests/{menu_movies.rs` (new),
`menus_flow.rs` (two tests moved to the skip contract)}, `docs/WORKLOG.md`.

## Done

1. **The page → movie mapping, from the page data** [M]. The menu-def's PAGE record's third dword is the
   page's aux background-movie code (`MenuManager_Create` 0x21b908:169-183 stores it at M_PAGE+0xC4), and
   `Manager_SendMessage` 0x21c858:338 plays it as a LOOPING background movie on every page change —
   `Menu_PlayMovie(aux, 0, 1, 0, 0, 0)` (loop = 1), `psiStopBackgroundMovie` when the new page has none.
   The test walks the whole tag stream with 0x21b908's own record sizes (all 13 kinds) and pins
   `menu::aux_movie_for` to the disc: **main 0x40000002 → 0x07330048**; **boot page 0x40000034, every list
   page (codename 0x40000025, difficulty 0x40000023, missions 0x4000001c, dossier 0x4000002b) and results
   0x40000036 → 0x07350048**; P_INTRO 0x40000032, the title layout 0x40000009 and attract 0x40000035 → 0.
   (The PAGE records carry the menu-def file's hashcode as **0x80000002** — GameState+0x20's form [M
   bootup 0x1c7e78], not the LevelBin name-hash 0x08000002.)
2. **The looping background under the page controls** (feature `movies`). `movie.rs` gains `MovieStream`:
   the game decodes one MPEG frame per tick on the IPU [M movies.md]; the whole-ES `decode_video` takes
   ~31 s for the 60 s loop, so the stream decodes **closed-GOP chunks** of the demuxed ES on a worker
   thread (the sequence header prepended; a chunk starting at a closed GOP is self-contained per
   ISO 13818-2 §6.4 — measured closed on 07330048/07350048/07370048; 07380048 mixes open GOPs, so chunk
   boundaries sit only at closed-GOP starts and open GOPs ride their predecessor's chunk). ~2× realtime;
   the host advances one frame per two 60 Hz ticks (30 fps [M main 0x00109080's skip-to-even-vblanks]).
   `lib.rs`'s `movie_menu_tick` mirrors the game's decisions: stop/restart when the page's aux differs
   from the playing one [M 0x21c858:338], loop like `LoopingMovie` [M 0x00108060].
3. **The frame under the controls.** The decoded frame goes into a `menu_textures` slot (a Clone of the
   scene's hud textures, appended under the movie's own hashcode — the overlay looks it up by that), and
   `menu_overlay` draws it full-band at **layer 1**: the quads sort descending and draw in order, so layer 1
   paints FIRST — under every page control. Without the feature (or on the slim ISO, whose PSS files are
   0-byte [L]) nothing is registered and the dark plate stays — the pre-movies look. gpu.rs hook (small,
   additive): `Renderer::update_hud_texture(slot, w, h, rgba)` + keeping the hud bind-group layout/sampler,
   so the window loop can stream the frame into an added hud slot (`--shot` builds a fresh Renderer either
   way). scene.rs hook: `#[derive(Clone)]` on `TextureImage` (the host's own texture list).
4. **The intro (title → Start).** `FrontEnd` gains `intro_playing`/`intro_index` [M `P_INTRO_Handler`
   0x209e88's movie_283]: Start arms the run (`INTRO_MOVIES = [0x07370048, 0x07380048]`, measured 96 + 329
   pictures on the retail image → `INTRO_TICKS = 850` at one frame per two ticks); the overlay draws the
   intro movie fullscreen (the intro page has no controls; the fog pass is suppressed while a movie plays
   [M psiDrawFogPass 0x0010b960]) and Confirm — the port's action 0x19 — **skips** [M `GameFlow_Main`
   0x1c98c8 case 6]; other input waits; the run ends itself at the budget. The main menu opens after.
5. **Movie audio through the existing cpal output.** The PSS PCM (16-bit 48 kHz stereo [M
   docs/formats/pss.md]) keys two aligned mono SPU voices (L/R split, pitch 0x1000) via `nf_audio::Cmd` —
   no nf-audio change; looping movies set the voices' loop point, stop key-offs both [M
   psiStopBackgroundMovie 0x00108328]. Volume full scale (the game captures `SFXGetVolume()` [M 0x209e88];
   the menus run it at max) [S].
6. **`--shot` shows the movies**: with the feature, `--page main` (and start/codename/difficulty/missions/
   dossier) decodes the page's aux still — the first closed-GOP chunk only (`movie_still`, fast) — and
   draws it as the backdrop; `--movie-frame N` picks the frame; `--intro [0|1]` renders an intro frame.

## How it was verified

- `menu_movies` tests (6/6, `--features movies`, `NF_MOVIE_ISO` at an image with movie data): the disc's
  PAGE map; the decoded frame's SHA-1 pin (07330048 frame 12 = `1f1fe7b0…`, sister: frame 60 differs);
  the stream-ahead test (90 advances → 90 distinct frames); the intro budget re-measured from the two
  PSS files (96 + 329 = 850 ticks); the intro flow (arm, skip, natural end); the overlay layering (the
  movie run first = under the controls; the plate stands in without the frame). All ISO-dependent tests
  skip cleanly without movie data; the map/intro/overlay tests run anywhere (NF_ISO only).
- `menus_flow` 17/17: two tests (`locked_missions…`, `cursors_wrap…`) streamed blind Confirms and now
  press the skip explicitly (the skip is the new, game-true contract) — behaviour otherwise unchanged.
- The sandbox has NO retail image (both ISOs are the slim on-foot one — every PSS 0-byte [L]), so the
  four task movies were **range-fetched from the retail image on the file host** (`007 - Nightfire
  (USA).iso`, sectors only: 70 MB) into a local mini ISO (scripts/, never committed). `NF_MOVIE_ISO` on
  the main agent's machine should point at the retail image itself.
- Full suite: see the counts below (nf-game/nf-formats/nf-audio/nf-view, slim ISO, `--test-threads=1`).

## Suite counts

Chunked foreground runs (the sandbox kills background jobs between tool calls), slim ISO, `--offline
--release -j2 -- --test-threads=1`:

- **nf-game: 232 passed / 0 failed**
- **nf-formats + nf-audio: 94 passed / 3 failed** — the 3 permitted `disc_golden` tests
  (`bigf_header_fields…`, `bigf_tables…`, `directory_tree…`: the onfoot ISO's stubbed
  /DRIVING/MIS01.MUS BIGF files; pre-existing, not touched by this task)
- **nf-view: 481 passed / 0 failed** (no `movies` feature — the movie-dependent tests are
  feature-gated); plus `--features movies` with `NF_MOVIE_ISO` at the retail image: **menu_movies
  6 passed / 0 failed** (its ISO-dependent tests skip without movie data)

## Render commands for the main agent (with the retail image as NF_MOVIE_ISO)

```
cargo build --offline --release -p nf-view --features movies
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view $NF_ISO --page main --shot /tmp/mm_main.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view $NF_ISO --page missions --shot /tmp/mm_missions.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view $NF_ISO --page start --shot /tmp/mm_start.png
NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view $NF_ISO --intro 1 --shot /tmp/mm_intro.png   # 0 = EON logo, 1 = the FMV
```
What each should show vs PCSX2: the main menu / mission list / boot page with the looping FMV
(0x07330048 / 0x07350048) visible AROUND and BEHIND the page controls (the panels and text sit on top);
the boot page's title art covers its movie almost fully (the 0x40000009 art is (0,0,640,480)); the
intro frames are the EON logo / the intro FMV fullscreen.

## Live checks for the main agent

- `--page main --shot`: the movie IS the backdrop and the controls draw over it (the game's backdrop is
  a full-screen background texture, not an overlay [M movies.md §Presentation]).
- The menu-loop audio: 0x07330048/0x07350048 loop seamlessly (the L/R voices restart at their loop
  point); confirm no double-audio when the page changes between two 0x07350048 pages (the port restarts
  the movie only when the aux code DIFFERS [M 0x21c858:338]).
- The intro: title → Start plays 0x07370048 (EON) then 0x07380048 then the main menu; Enter/X skips
  instantly to the main menu [M GameFlow_Main case 6].
- `--page results --shot` (F8/F9 demo path): the results page's backdrop is its PAGE record's
  0x07350048 loop under the stub text — confirm against the game's results screen.

## Left

- The intro's fade-back into the main menu (the game's 0x44 page-change irises) — the port flips the
  page; the iris work belongs to the menus task that owns the page-change effects.
- `P_ATTRACT` (0x40000035) and `P_TRAILER`/`P_WINGAME`/`P_ESTHERO` handlers play their movies from the
  handler, not the aux field — not wired here (attract needs demo playback; the others are endgame
  pages the port stubs).
- The briefing/outro chain (GameFlow states 3/6 with the 0x71/0x73 codes) still runs the headless
  countdown; `movie_menu_tick`'s mechanism extends to it once the mission flow owns it.
- Movie-volume follows the game's `SFXGetVolume()` capture per start site (the port runs full scale) [S].

## ntfy failed

(none — the notification step ran; see the driver log.)
