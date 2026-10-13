# fix-movies — the FMVs (PSS)

Branch `s/fix-movies`. The task: verify the PSS demuxer on Nightfire's own movies, decode them, document where
the game plays them, and give nf-view playback with a headless frame dump. G-008 ("pss.rs and vag.rs are
RC2-verified only") is closed for the demuxer side.

## What's done

1. **Demuxer verified on all 22 movies** (`crates/nf-formats/tests/pss_movies_live.rs` + `golden/pss_movies.tsv`).
   Two real RC2-vs-Nightfire differences found and fixed in `crates/nf-formats/src/pss.rs`:
   * **Audio sub-stream header is `FF A0 00 00`, not RC2's `FF A1 00 cc`** — the old demuxer errored
     ("private stream 1 has no audio sub-stream header") 100 KB into the first movie. All 53,374 audio packets on
     the disc measured; the demuxer now accepts both.
   * **The audio is 16-bit little-endian PCM (`SShd` kind 1, 48000 Hz, 2 ch, interleave 512), not SPU ADPCM
     (kind 0x10)** — `Audio::parse` rejected every Nightfire movie outright, and `Audio::decode` would have
     produced noise. `samples()`/`decode()` handle both kinds. Evidence: the `SSbd` body is exactly
     `seconds · 48000 · 4` bytes per movie (07100001: 8,194,048 = 42.67 s), and it disagrees with the video
     length by <1 s on all 20 audio-bearing movies.
   * Everything else matched RC2's model: packs/BB/BE/B9, one audio channel (0) per movie, 22 movies with the
     end code and no trailing bytes.
2. **Video decode** (`crates/nf-formats/src/mpeg2.rs`, new): the movies are MPEG-2 Main@Low, 512×448,
   interlaced, 4:2:0, 30 Hz, I/P/B (e.g. 07100001: 1280 = 72 I + 358 P + 850 B). A crate exists —
   **`oxideav-mpeg12video` 0.0.13, pure Rust, MIT, deps serde_json+thiserror only** — and its docs claim the
   glue is missing, but `video_sequence::decode_video_sequence` works: it decoded all 1280 frames of 07100001.
   Verified against ffmpeg (independent tool): PSNR 67.8 dB (frame 0) / 66.3 dB (frame 1000) — IDCT rounding
   only. The crate is behind the opt-in cargo feature `movies` (`nf-formats/movies`, `nf-view/movies`) so master
   builds and tests unchanged without it. **Dependency approval wanted**: ~0.9 MB of source on crates.io (MIT),
   pulled into `Cargo.lock` (no build without the feature).
3. **When they play** (`docs/systems/movies.md`): one player `psiStartBackgroundMovie__FUiScUi` 0x001081a8
   (`cdrom0:\movies\30_FPS\%8.8x.pss;1`, `UsingMovie` whitelist 0x00107eb8); boot logos 0x7370048/0x7380048
   (P_INTRO 0x00209e88), briefing 0x071000xx via `ResetMap_LevelToLoad` 0x001d3800 + `ResetMap_Load` 0x001d6360
   (GameFlow states 3→6), outros 0x073000xx via `Mission_Update` 0x00182c60, win 0x73F0048, attract/trailer/
   extras pages, looping menu BGs from `M_PAGE+0xC4` (FILES.BIN data) via `Manager_SendMessage` 0x0021c858.
   **Skip = action 0x19 (X/Start), instant** (`GameFlow_Main` 0x001c98c8 case 6; menu side has a grace period,
   `Menu_StopMovie` 0x00215a50). Presentation: 512×448 RGBA32 background, 30 fps, PATH3 upload, fog suppressed
   (`psiDrawFogPass` 0x0010b960). Format doc: `docs/formats/pss.md`.
4. **nf-view** (`crates/nf-view/src/movie.rs`, new; small `--movie` branch in `lib.rs::run()` — shared file,
   noted): `Movie::load` (game's own filename construction), PCM audio decode, and the headless dump
   `nf-view <iso> --movie <code> --frame N --shot out.png` (feature `movies`). Verified live: 07100001 frame 0
   (satellite-briefing imagery) and 073F0048 frame 600 (credits footage) render correctly — PNGs rendered and
   eyeballed; the main agent should re-check them (render list below).

## How it was verified

* The slim on-foot ISO stubs the movies (all 22 entries lba=0/size=0 — that's why 3 `disc_golden` tests are
  red on it). I fetched the retail `007 - Nightfire (USA)` image, and **all 22 movie extents hash-match
  `golden/disc_files.tsv`** (and every one of the golden manifest's 177 path+lba+size entries matches), so the
  movie bytes are the retail disc's.
* `NF_MOVIE_ISO=/…/007retail.iso NF_ISO=<onfoot> cargo test --offline --release -j2 --no-fail-fast -p
  nf-formats -p nf-view -- --test-threads=1`: nf-formats all green (63 lib + disc_golden 11/14 with the 3
  allowed slim-ISO fails + driving_golden + weapon_live + **pss_movies_live 3/3**) and every nf-view test
  binary green until the run hit my 500 s command timeout (the nf-view live suite is long; my changes are
  additive and feature-gated, and the binaries that ran all passed).
* The SHA-1 test (`video_frames_decode_to_known_sha1s`, feature `movies`) pins frame 0 = `54b75479…` and frame
  1000 = `f965b37f…` of 07100001 with a sister (different frame → different hash); it passes with
  `--features movies` against the retail image.
* Every golden comparison has a sister assertion; the pss unit tests cover PCM round-trip + the RC2 ADPCM path
  still parsing + a wrong kind still erroring.

## What's left

* **Live playback in nf-view** (window loop + cpal): the wiring map is docs/systems/movies.md — GameFlow state 6
  for briefings, `MenuManager_Update` for menu BGs, skip on action 0x19. Only the headless dump + decode path
  landed. The audio path out (cpal sink fed by `Audio::decode`) is unwritten.
* The FILES.BIN `M_PAGE+0xC4` menu-BG hashcodes (which page shows which looping movie) — data extraction from
  the menu scripts, not code.
* 0x72xxxxxx movie codes reach `GetLevelToLoadFromMovieCode` but no ACTION.ELF code produces them (DRIVING.ELF
  handoff or FILES.BIN — unchecked). 0x7360048 is whitelisted and hard-asserted in `strFileOpen` but absent
  from the disc.
* Full-suite run was cut at my command timeout (see above); a clean full pass belongs in the next checkpoint.

## Live checks for the main agent

Renders (no GPU here; each should show what's named):
* `nf-view <retail iso> --movie 07100001 --frame 0 --shot f0.png` — satellite-map briefing frame with the
  targeting reticle overlay, correct colours (not washed out / not BGR-swapped).
* `nf-view <retail iso> --movie 073F0048 --frame 600 --shot f600.png` — live-action credits footage
  (composer at a keyboard), correct aspect (512×448, no borders).

GDB/fieldprobe: none needed this round; the movie path was verified from file bytes and decompile only. One
question a live check could settle later: whether the IPU-display path applies any additional YUV→RGB
coefficient choice (my CSC is BT.601 limited-range; frames eyeball-correct against ffmpeg's).
