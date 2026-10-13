## Your task: `fix-movies`, the FMVs (PSS): the intro, the briefings and the cut-ins between levels

The game plays PSS movies (`MOVIES/30_FPS/*.PSS` on the disc) at boot, before missions and between levels. The port plays none. `crates/nf-formats/src/pss.rs` (the demuxer) and `vag.rs` were copied from the user's RC2 port and are only verified on RC2's files (GAPS G-008).

1. **Verify the demuxer on Nightfire's movies:** an NF_ISO test over every PSS on the disc gives the stream ids, the packet counts, the video size, the frame count and the audio format. Golden numbers only, with a sister assertion.
2. **Decode:** the video is MPEG-2 (IPU-style elementary stream) and the audio is in the PSS's audio stream (find its format: PCM or ADPCM). Use a pure-Rust MPEG-2 decoder if one exists as a crate; otherwise write a minimal I/P/B decoder for the profile the files use. Adding a crate is a dependency: name it, its licence and its size in your report, and keep it behind a cargo feature `movies` so master builds without it until the user approves.
3. **When they play:** find the game's movie calls (`PlayMovie`, `Movie_*`, `FMV_*` in the symtab), which movie plays where (boot, the level loads, mission ends), and whether it can be skipped. Write it in `docs/systems/movies.md`.
4. **Playback in nf-view:** a full-screen video plane plus the audio through the existing cpal output, with skip on the game's skip button. You have no GPU: build the decoder and a headless frame dump (`--movie <name> --frame N --shot`), and the main agent checks the pixels.

Tests: decode frame 0 and frame N of one movie to known SHA-1s (computed by your decoder, and cross-checked against ffmpeg if you can run it in the sandbox, as an independent tool); audio sample count against the stream length.

Files: `crates/nf-formats/src/pss.rs`, a new `crates/nf-formats/src/mpeg2.rs` (or the crate glue), `crates/nf-view/src/movie.rs` (new), and `docs/formats/pss.md`.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-movies`.

---
claimed-by: sandbox4 at 2026-10-13T13:37Z
