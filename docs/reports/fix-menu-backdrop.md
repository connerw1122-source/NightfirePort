# fix-menu-backdrop — the menus' background FMV decodes now

Branch `s/fix-menu-backdrop` (from `origin/master` b65d6f7, merged `origin/master` 6691ef9 —
fix-menus-real2 landed the ISO-precedence fix independently while this task ran; both sides kept).
Files: `crates/nf-view/src/{movie,lib}.rs`, `crates/nf-view/tests/{menu_backdrop_still.rs (new),
menu_movies.rs}`. No shared-file hooks beyond `lib.rs` (a task-named file here).

## The bug

Every front-end page drew the dark plate: `--movie-frame <N>` on the retail bytes always printed
"[front end] movie 07330048: no frame N in the first chunk — the dark plate stands in".

Root cause (measured, then fixed): `gop_chunks` splits the video ES at CLOSED-GOP start codes and
prepends a chunk starting at 0 "for a leading open GOP". Its guard `bounds.first() != Some(&0)`
also fired when the first GOP is CLOSED — which it is on every Nightfire movie, the first GOP
sitting right after the 145-byte sequence header (probe on retail bytes: 07100001/07330048/07350048
all `gop[0] = (145, closed)`). Chunk 0 was therefore `(0, 145)` — the sequence header alone, zero
GOPs — and decodes to ZERO frames [L probe: `chunk0 decode: 0 frames`, while the whole ES decodes
1280/1800/1800 pictures exactly matching golden/pss_movies.tsv]. `movie_still` read
`chunks.first()`, so every still failed at every N. The live loop was NOT broken (the streaming
worker skips the empty chunk and decodes chunk 1 — the pre-fix windowed-loop test already passed;
only the first frame arrived one wasted chunk later).

Second bug: `front_end_menu`'s still read `scene_iso_name()` (the bare `NF_ISO` env), not the ISO
argument — `nf-view <retail.iso> 07000048 --page main` reported the slim on-foot image's 0-byte PSS
stubs [D ISO9660 parse: all 22 PSS entries are 0 bytes on the slim image]. fix-menus-real2's
`NF_MOVIE_ISO`-over-env fix for the same line merged mid-task; the resolution keeps
`movie_iso_name(iso_path)`: NF_MOVIE_ISO over the ISO argument (the live loop's
`App::movie_iso_path` precedence), so both the argument and the env work.

Third bug (drive-by, test-only): `menu_movies.rs` did not compile under `--features movies`
(`(*w, *h)` on `present()`'s owned `u32`s — nobody ran it feature-on since the signature changed),
and its briefing test's `Movie::load(&iso, "0x7100001")` can never resolve (a 7-digit stem fails
`load`'s 8-char code check and becomes the file `7100001.PSS`). Both fixed in the test file.

## Fixes (each cites its evidence)

1. **`movie.rs gop_chunks`** — prepend 0 only when open GOPs precede the first closed one
   (`bounds.first().is_none_or(|&at| at > head_end)`). Chunk 0 now carries the first GOP
   [ISO 13818-2 §6.4.3: a closed-GOP-aligned slice with the sequence header prepended is
   self-contained; the game's backdrop frames come from the same stream through
   `psiStartBackgroundMovie` 0x001081a8 → `DecodeBGFMVFrame` 0x00108550 → `PS2UploadBGMImage`
   0x108740; the page plays it looping via `Menu_PlayMovie` [M Manager_SendMessage 0x21c858:338]].
   Unit test `the_first_gop_chunk_starts_at_the_first_gop` (fails pre-fix: chunk 0 was (0,145)),
   sister: the chunks' frame sum equals the ES picture count (no pictures lost at boundaries).
2. **`movie.rs movie_still`** — walks whole closed-GOP chunks in order until `n` is covered
   (frame 12 of the menu loops rides chunk 1: 10 pictures per GOP [L golden: 180 I / 1800 total]),
   instead of `chunks.first()` only. Cross-checked against the whole-ES decode: frame 12's RGB8
   SHA-1 = `1f1fe7b0…`, the exact pin `menu_movies::the_menu_loop_movie_decodes` measures through
   the streaming path — one decoder, one pixel truth. Beyond the last picture it names the count.
3. **`lib.rs`** — `movie_iso_name(iso_path)` (NF_MOVIE_ISO over the ISO argument) feeds
   `movie_still`; `front_end_menu` takes the resolved ISO path from `run()`.

## Verification (fail-first)

`crates/nf-view/tests/menu_backdrop_still.rs` (needs `--features movies` + an image with movie
data; the retail bytes were range-fetched from the file host's `007 - Nightfire (USA).iso` into a
mini ISO as fix-menu-movies2 did — `/home/z/my-project/nf-mini.iso`, not committed; the demux
counts of the fetched files match golden exactly, so the bytes are exact):

- Pre-fix run: `the_main_page_backdrop_still_decodes_a_frame` → "07330048 frame 2 (the main page's
  backdrop): no frame 2 in the first chunk"; same for 07350048 frame 6 and frame 12 — the user's
  exact symptom, reproduced headlessly. The windowed-loop test passed pre-fix (see above).
- Post-fix: 5/5 green — 07330048 frame 2 and 07350048 frame 6 decode as real 512×448 pictures (a
  flat-colour frame fails the sister), frame 12 matches the `1f1fe7b0…` whole-ES pin (sister:
  frame 13 differs), the windowed player starts and advances on the main page (4+ distinct frames
  through `Player::start_looping(0x0733_0048).present()`), and `movie_iso_name` prefers
  NF_MOVIE_ISO, then the ISO argument, over NF_ISO.
- Neighbours on the merged tree: menu_movies 7/7, movie_playback_flow 10/10, menus_flow 34/34,
  lib tests 10/10.

## Suite (final tree = merged origin/master, chunked foreground)

The sandbox kills background jobs between tool calls (4th session running), so the fast runner ran
in foreground `--only` chunks over the whole 241-binary build: **874 passed / 3 failed / 27+
ignored**, no CRASH. The 3 failures are the permitted `disc_golden` set (the slim ISO's stubbed
BIGF/DISC data). `levels_sweep_live` green. **`weapon_matrix_live` TIMEOUT'd at my chunk limits
(180 s, then 240 s — it progresses level by level, `matrix_0700000a` → `matrix_0700000c`), and a
re-run at the default 600 s limit produced an empty log before the deadline; it is a slow
asset-matrix binary unrelated to this task's files (movies/menus do not touch weapon matrices) and
was green in past sessions' full runs — the driver's own suite pass should re-classify it at the
default limit.** Not run (deadline): the dump_*/probe_* scratch binaries beyond those chunked.

## Notes for the next session

- `pss_movies_live`'s 22-movie golden sweep still wants the full retail image (the mini ISO holds
  5 movies); its menu-movie subset passes against the mini bytes.
- The movie audio is silent until `open_pcm_out` calls `.play()` (cpal 0.18 starts paused) —
  pre-existing on master, flagged by fix-menu-movies2, still open.
- The repro probes were removed from the tree before the commit (they broke the non-movies build);
  their numbers live in this report. The mini-ISO builder is scratch (`scripts/build_mini_iso.py`
  in the sandbox, not committed — it fetches retail byte ranges).

## Live checks for the main agent

nf-view (retail image as NF_MOVIE_ISO, or as the ISO argument — both work now):

1. `NF_MOVIE_ISO=<retail.iso> ./target/release/nf-view <iso> 07000048 --page main --movie-frame 12 --shot main_f12.png`
   → the orange lens panel, Bond in the tuxedo and the 007 ring visible behind the main page's
   controls (the frame is the FMV, not the dark plate); compare with PCSX2's main page.
2. `... --page missions --movie-frame 30 --shot missions_f30.png` → the 07350048 backdrop
   (list pages' loop) under the mission list.
3. `--movie-frame 2` vs `--movie-frame 12` shots differ (the still walks chunks; the loop plays).
4. Live window: on the main page the backdrop PLAYS (frames advance under the controls); changing
   main → missions restarts the movie only because the aux code differs [M 0x21c858:338].
5. PCSX2: the movie's own frames at 30 fps are the page background — the port's decoded frame
   should be pixel-equivalent modulo the decoder (the `1f1fe7b0…` pin guards it).

Still open from the playtest verdict: the windowed loop's first-frame latency is now one chunk
decode (~0.15 s) instead of two; if the user still sees a black flash on page change, the next
suspect is the plane's texture upload timing, not the demux.
