**Base:** merge `origin/master` first and check it contains `crates/nf-view/tests/briefing_plays_live.rs` (the
front-end audit + test fixes landed with it, 2026-10-16 evening). If it does not, fetch again before you start: the old
master still carried 28 tests that contradicted the decompile.

## Your task: `movie-volume-options`, the movie volume follows the options

`crates/nf-view/src/movie.rs` plays movie audio at a fixed `MOVIE_GAIN` = 100*100/0x7fff (the boot default). The
game: `psiStartBackgroundMovie` stores `MovieVolume = SFXGetVolume()` (= SfxMasterVolume, 100 at boot [L 0x30c95c]) and
`audioDecStart` 0x107360:9 calls `changeInputVolume(MovieVolume * 100)`, which calls `sceSdSetParam` on both cores'
input volume (0xf80 / 0x1080) [M 0x1078c8]. Port it:
1. Decompile `changeInputVolume` and the sceSdSetParam path and confirm the scale (is the register value linear against
   0x7fff? what does the SPU2's core input volume do to the PCM?). Document it in docs/systems/movies.md.
2. The player takes the volume at start (`Player::start(.., volume)`) from the options' SFX volume the front end holds
   (`fe.opts`; find where the options page writes the game's SfxMasterVolume and mirror that mapping), and the gain
   applies in `open_pcm_out`'s callback. A volume change mid-movie does what the game does (check whether it reads
   MovieVolume only at start).
3. Tests: the gain at boot equals today's MOVIE_GAIN; an options SFX volume of N gives the game's N*100 input
   volume; sister assertions. No audio output in tests (NF_SILENT).

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/movie-volume-options`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-17T06:26Z

---
## Report (done, 2026-10-17T08:10Z)


---
## Report from the worker (done, 2026-10-17T08:10Z)

