## Your task: `mpeg2-interlaced-decode`, the movies decode ghosted (interlaced prediction)

The user reports the menu backgrounds "aren't moving like in PCSX2". Measured 2026-10-15 by the main agent: the
front end's looping backdrop movie `07350048` (MOVIES/30_FPS, 512×448 interlaced MPEG-2 MP@ML, I/P/B — see
docs/formats/pss.md and crates/nf-formats/src/mpeg2.rs) decodes in the port as a soft, low-contrast ghost: the
dancing figure is a faint smear at every frame and consecutive frames barely differ (mean abs RGB diff between
decoded frames 12 and 60 ≈ 31 over a 170×110 region), while PCSX2 shows a crisp black silhouette that appears,
moves and vanishes within 0.3 s. Ghosting that accumulates between I-frames is the classic symptom of wrong
predicted-picture reconstruction — most likely the interlaced cases (field pictures, field/dual-prime prediction in
frame pictures, `top_field_first`, field DCT, the B-picture backward reference), which the third-party
`oxideav_mpeg12video` crate the port uses (feature `movies`) may handle wrongly.

Do:
1. Prove it with numbers: decode `07350048` (and one briefing movie, e.g. `07100005`) frame by frame with the port
   (`crates/nf-view/src/movie.rs` `Movie::decode_frame` / `movie_still`) AND with an independent reference decoder
   (ffmpeg / libavcodec via a pip package such as PyAV or imageio-ffmpeg in the sandbox — install it there, it's
   tool use, never committed). Report per-frame PSNR (Y plane) port vs reference for the first ~120 frames, split by
   picture type. Commit only numbers (PSNR table, SHA-1s) — never frames.
2. Find the cause in the decode path (the crate, our use of it — display reordering of B pictures, field/frame
   handling, the IDCT/clamping, chroma upsampling in `yuv420_to_rgb8`) and fix it: patch it in our tree (vendor the
   minimal fix into a module of ours if the crate can't be patched cleanly; keep its licence notice — check it's
   compatible, see CLAUDE.md's licence table) or port the missing MPEG-2 parts ourselves.
3. A test (NF_ISO-gated) that decodes a stretch of `07350048` and asserts the port's frames match the reference
   decode within a tight PSNR bound (e.g. ≥ 40 dB on every frame) — store the reference's per-frame SHA-1 or a
   small numeric digest, not images; with a sister assertion that the old ghosted decode would fail.
4. While there: `Player::present` (movie.rs) now paces by the wall clock at 30 fps (the main agent changed it); keep
   that.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mpeg2-interlaced-decode`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-15T17:58Z
