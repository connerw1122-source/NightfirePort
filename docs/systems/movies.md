# Movies (PSS FMVs): when the game plays them, and how they're shown

One player serves everything: `psiStartBackgroundMovie__FUiScUi` (0x001081a8) — `(u32 hashcode, char loop,
u32 volume)`. It gates on the `UsingMovie` whitelist (0x00107eb8, exactly the 22 on-disc hashcodes plus
0x7360048, which is not on the disc), stops any current movie, stores `MoviePlaying`/`LoopingMovie`/`MovieVolume`
(0x0030c6a0/0x0030c6a4/0x0030c6a8), allocates the decoder buffers (`AllocateMovieMemory__Fv` 0x00107d70), builds
`cdrom0:\movies\30_FPS\%8.8x.pss;1` and opens it with `strFileOpen` (0x00108cc0, `sceCdSt*` streaming). The
demux/playback machinery is `StartUpBGFMV` (0x001083a8) → `DecodeBGFMVFrame` (0x00108550) per tick →
`psiSwapBGFMVFrame` (0x001082f8); `psiMovieFinished__Fv` (0x001082e8) reports the end; `psiStopBackgroundMovie__Fv`
(0x00108328) frees and clears `BGFMVPlaying` instantly. Looping restarts the same file inside
`psiDecodeBackgroundMovieFrame` (0x00108060) when `LoopingMovie != 0`.

## Which movie plays where

| where | hashcode | site |
|---|---|---|
| boot logos (page 0x40000032) | 0x7370048 then 0x7380048 | `P_INTRO_Handler` 0x00209e88:62 (counter `movie_283`) |
| attract (page 0x40000035) | 0x73A0048 / 0x73B0048 | `P_ATTRACT_Handler` 0x0020a1f0:36 |
| extras (page 0x40000043) | 0x73B0048 | `P_ESTHERO_Handler` 0x0020a060:24 |
| trailer (page 0x4000004E) | 0x7390048 | `P_TRAILER_Handler` 0x00214f10:20 |
| win game (page 0x40000053) → credits | 0x73F0048 | `P_WINGAME_Handler` 0x00215100:20 |
| mission briefing | 0x071000xx derived from the level | `ResetMap_Load__Fv` 0x001d6360:111 (GameFlow state 3 phase 0) |
| mission end outro | 0x073000xx | `Mission_Update__Fv` 0x00182c60 case 6 sets `LevelToEndTo` → `ResetMap_LevelToLoad` 0x001d3800 |
| menu background | page `M_PAGE+0xC4`, looping | `Manager_SendMessage` 0x0021c858:338 → `Menu_PlayMovie__FUiScScScScUi` 0x00215990 (loop = 1); codes live in FILES.BIN menu data (chunk 0xFFFFFFF0) |

The briefing chain: `ResetMap_LevelToLoad__FUiScSc` (0x001d3800) upgrades the level id to the movie code
(each 0x7000 00xx → 0x7100 00xx) and pushes GameFlow state 3; `ResetMap_Load` phase 0 saves the movie code into
`GameState._16_4_`, masks it back to the plain level, plays the movie and pushes state 6; when the movie ends or
is skipped, phase 2 resolves the level with `GetLevelToLoadFromMovieCode__FUi` (0x001d3560). The per-level
subtitle script keyed by the same code is already documented in docs/systems/levels.md §5.4. Outros return to
the front-end map (0x7000048).

## Skipping

Yes — instantly, no confirm: **action 0x19 (25) newly pressed = X or Start for player 0**
(docs/systems/input.md §3).

* Briefing (GameFlow state 6): `GameFlow_Main__Fv` 0x001c98c8 case 6 — `psiMovieFinished() || Input_Action(-1, 0x19, 1)`
  → `psiStopBackgroundMovie` → pop → level load.
* Menu movies: `MenuManager_Update__Fv` 0x0021c3d8:42 — `Menu_InputAction(-1, 0x19, 4)` → `Menu_StopMovie__FUc`
  (0x00215a50), which honours the unskippable grace period `DAT_002e1c4c` (set by `Menu_PlayMovie`'s last arg,
  decremented by `Menu_UpdateMovieCounter__Fv` 0x00215970; e.g. P_ESTHERO sets 0xb4 = 3 s).
* Page handlers then advance (`0x5f` close / `0x44` page change) and restart the menu music jingles
  (`SFXStart(0x1d7)` / `SFXStart(0x470)`).

## Presentation

A full-screen **background texture**, not an IPU overlay: the IPU (inside the statically linked libmpeg,
`sceMpegGetPicture` 0x0022a3f8) decodes into a 512×448 RGBA32 buffer (`Mem_Malloc(0xe0000)`), the frame is
centred (`DecodeBGFMVFrame` 0x00108550: `(0x200 − w)/2`, `(0x1c0 − h)/2` — every movie is exactly 512×448, so no
borders), uploaded via PATH3 GIF tags (`setLoadImageTags__FPUiPviiii` 0x00107910, `PS2UploadBGMImage__Fv`
0x00108740) and flipped per tick (`SwapBGFMVFrame` 0x001087c8). Pacing: one MPEG frame per game tick with
`PS2FramesToSkip` computed on even vblanks, clamped to 2 (main 0x00109080) — 30 fps NTSC; the fog pass is
suppressed while a movie plays (`psiDrawFogPass` 0x0010b960: `BGFMVPlaying == 0`).

## Audio

One stream, decoded on the IOP: `audioCallback` (0x00108988) fills `AudioDec` (0x30dd40), `audioDecSendToIOP`
per frame; 16-bit PCM 48 kHz stereo (docs/formats/pss.md). Volume: `MovieVolume` = `SFXGetVolume()` captured at
every start site, applied as `changeInputVolume(MovieVolume * 100)` in `audioDecStart` (0x00107360).

## Port status (this branch)

* Demuxer verified on all 22 movies; PCM audio fixed (was RC2's ADPCM-only); MPEG-2 decoding behind the opt-in
  cargo feature `movies` (`oxideav-mpeg12video` 0.0.13, MIT); headless frame dumps
  (`nf-view --movie <code> --frame N --shot out.png`) verified on 07100001 f0 and 073F0048 f600
  (docs/reports/fix-movies.md).
* **Not yet wired**: live playback into the front end (GameFlow states 3/6, the page BG movies and the skip
  action need `nf-view`'s window loop + cpal output; the state machine above is the map to follow). Also open:
  the FILES.BIN page+0xC4 menu-BG codes and the 0x72xxxxxx codes only `GetLevelToLoadFromMovieCode` sees.
