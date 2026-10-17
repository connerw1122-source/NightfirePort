# movie-volume-options — the movie volume follows the options' SFX volume

Branch `s/movie-volume-options` (base origin/master 3456a72f, merged origin/master 4baed751).

## What the game does (evidence)

- `psiStartBackgroundMovie__FUiScUi` 0x1081a8 stores its volume arg into `MovieVolume`
  (0x30c6a8 — disassembly `sw $s2, -14680($at)` [M]; `LoopingMovie` is the byte at 0x30c6a1).
  **Every** caller passes `SFXGetVolume()` [M Menu_PlayMovie 0x215990:18-19, P_INTRO
  0x209e88:45+62, P_ATTRACT 0x20a1f0:23+36, P_FMVPLAYER 0x20ad30:25-26, P_TRAILER 0x214f10:19-20,
  ResetMap_Load 0x1d6360].
- `SFXGetVolume` 0x116e18 returns the `SfxMasterVolume` static (0x30c95c, **100 at boot**
  [L ee_start_05.bin @0x30c95c = 0x64]); when `switch_CONFIG_SOUND` (0x30ceac) == 0 it returns 1
  [M disassembly — the `==0` path falls into `li $v0, 1`].
- `audioDecStart` 0x107360:2-3 calls `changeInputVolume(MovieVolume * 100)` [M disassembly: `lw`
  of 0x30c6a8, `li $a0, 100`, `mult`].
- `changeInputVolume` 0x1078c8 sends `sceSdRemote(1, 0x8010, reg, value)` for reg **0xf80** and
  **0x1080**. Its sibling `changeMasterVolume` 0x107860 uses the same RPC for the master pairs
  0x980/0xa80 with core 1 = +1 (0x981/0xa81) [M], so the R channel sits at L+0x100 and 0xf80/0x1080
  are **core 0's input-volume L/R pair** (both even → core 0; the movie PCM is a core-0 stream,
  unlike the SFX voices which sound.md §5.4 covers).
- **Read-once:** `MovieVolume` has exactly two readers in the whole ACTION.ELF decompile — the
  store at 0x1081a8 and `audioDecStart` 0x107360. A mid-movie options change never reaches a
  playing movie [M]. Teardown silences the path: `CleanUpBGFMV` 0x108700 → `audioDecReset`
  0x1073d8 → `changeInputVolume(0)` [M].
- **The options mapping:** `P_CNAVOPTIONS_Handler` 0x214818 snapshots `start_sfx_vol_496 =
  SFXGetVolume() / 5` on enter (:22-23), gives the slider element 0x10000134 the 0..0x14 range
  (:26 `0x27, 0, 0x14`), and on commit calls `SFXSetVolume(element_value * 5)` (:73-74); the
  cancel path restores `start_sfx_vol_496 * 5` (:80-81). So UI 0..20 → `SfxMasterVolume` 0..100,
  and the persisted settings load straight into the static [M LS_LoadGlobalSettings 0x1cdbc8:70].
  The port's `fe.opts[0x1000_0134]` already stores that same raw 0..100 value.

## The scale ([S] — the one unproven link)

The SPU2's input-volume DSP behaviour is hardware; no decompile shows it (SFX.IRX's
`sceSdSetParam` is an import stub — the real sd driver is the IOP kernel module, not decompiled).
Two candidates:

- **0x7fff** (signed-16-bit register range) — the port's choice. Boot gain 10000/0x7fff ≈ 0.305,
  consistent with playtest 1016's "the port's full scale was about 3x too loud" (0x3fff would be
  only 1.6x).
- **0x3fff** — the game's own voice/reverb writes treat 0x3fff as 100% (sound.md §5.4 [M]).

Kept out of pass/fail: the linearity/equal-loudness assertions only pin the port's own mapping
(`movie_gain(N) = N*100/32767`); the boot-equals-old-gain assertion pins port-internal continuity.
A live check (below) can settle it.

## The port

- `movie.rs`: `Player::start(.., volume)` / `start_looping(.., volume)`; the volume rides the
  player (`Player::volume()`), `movie_input_volume(v) = v*100` [M 0x107360] and
  `movie_gain(v) = movie_input_volume(v)/0x7fff` [S scale] apply in `open_pcm_out`'s callback,
  computed once per player — matching the game's read-at-start (mid-movie fe.opts edits cannot
  reach the running player; there is no re-read path).
- `lib.rs::movie_sfx_volume(fe)` reads `fe.opts[0x1000_0134]` (missing key = boot 100 [L
  0x30c95c]) and all five start sites pass it (intro, attract, page backdrop, and both
  mission-glue starts), mirroring the game's `SFXGetVolume()` at every
  `psiStartBackgroundMovie` call.
- Docs: `docs/systems/movies.md` "Volume (movie-volume-options)" section.

## Tests (what ran; NF_SILENT=1 everywhere, no test plays audio)

- `nf-view` lib, default features: **3 new pure tests** — boot chain (100 → register 10000 →
  the old fixed MOVIE_GAIN), linearity/zero-mute, and the fe read (opts 37/55 ride through,
  missing key = 100). All pass.
- `nf-view` lib, `--features movies`: **27/27 pass**, including the new
  `the_player_captures_the_start_volume_once` (a real `Player::start` of 07100001 — the slim ISO
  carries that one movie — captures 37 and 100, gain matches `movie_gain`). The two lag tests and
  the existing movie tests still pass.
- `nf-view` integration with `--features movies`: `movie_playback_flow` 10/10, `menu_movies` 8/8,
  `movie_pacing_live` 1/1 (all movie-flow binaries updated to the new arg, volume 100 = boot).
- Full per-crate suite run (release, `--test-threads=1`, slim ISO):
  - **nf-game: 61 binaries, 509 passed, 0 failed.**
  - **nf-formats + nf-audio: 96 passed, 3 failed — exactly the 3 allowed `disc_golden`** (bigf
    header/tables, directory tree; the slim image's golden manifests).
  - **nf-view** default features: 104+ passed across 27 binaries plus the lib; **2 failed in
    `fix_0c_pylon_live`** — verified failing on **pure origin/master** in this sandbox too
    (wgpu adapter, no GPU here; environment, not this change).
  - `briefing_plays_live` (0/1) and 3 of `menu_backdrop_still`'s 5 fail **only because the slim
    ISO has no data for 07100005/07330048** (the codebase's own "stub on this image" error);
    they need the full retail image.
  - **suite.py itself could not run as designed in this sandbox:** background jobs (nohup,
    setsid) are killed between tool calls, and a foreground pass exceeds the 10-minute tool
    limit, so the run was executed per-crate/batch with suite.py's exact flags; the binaries,
    arguments and environment match what suite.py does. A full nf-view pass (416 test binaries)
    does not fit the 2-hour window; the movie/menu-critical binaries plus one alphabetical batch
    ran.

## What's left

- The main agent's live checks below settle the [S] scale; if 0x3fff wins, it is a one-constant
  change (`MOVIE_FULL_SCALE` in movie.rs) and the doc's scale paragraph.
- A full-suite pass on a machine/GPU sandbox where suite.py can complete (the pylon pair and the
  two full-ISO movie tests should be green there).

## Live checks for the main agent

1. **MovieVolume capture (gdbwatch, EE 0x30c6a8, writes):** fresh boot → start the Exchange
   briefing. Expected: exactly one write of 0x64 (100, the boot `SfxMasterVolume` [L 0x30c95c])
   at `psiStartBackgroundMovie`. Then change the options' SFX volume mid-movie: expected **no
   further writes** to 0x30c6a8 until the next movie starts (read-once [M 0x107360]).
2. **The input-volume register value:** with the SPU2 debugger (or an RPC log), when a movie
   starts at SFX volume 100, expected **10000** (= MovieVolume*100 [M 0x107360]) in core 0's
   input-volume L/R (libsd entries 0xf80/0x1080 [M 0x1078c8]); with the options' SFX slider at
   10/20, expected 5000.
3. **The scale:** play the same movie at boot volume on PCSX2 and in the port — if the port is
   ~3x too loud relative to PCSX2, the 0x7fff choice is right; if ~1.6x, switch
   `MOVIE_FULL_SCALE` to 16383.

## Usage

not visible to me (no token, cost or context-window figures are exposed in this environment).
