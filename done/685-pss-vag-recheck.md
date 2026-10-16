## Your task: `pss-vag-recheck` — close GAPS G-008: pss.rs / vag.rs on every movie

GAPS G-008: `pss.rs` and `vag.rs` were verified on RC2 data only. Write an NF_ISO opt-in test that walks EVERY
`MOVIES/30_FPS/*.PSS` on the disc:
- the demux packet counts;
- the video picture count vs the GOP walk;
- the audio kind/rate/channels;
- the audio duration vs the video duration (±1 frame);
- that every picture decodes without error.

Also walk every VAG/stream the sound banks reference: decode and check the sample count and loop points. Record
any file that disagrees and fix the decoder with evidence (the format docs, and an independent decoder where one
exists — vgmstream is ISC). Golden: numbers and SHA-1s only. Update docs/formats and close G-008 in GAPS.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/pss-vag-recheck`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T10:36Z

---
## Report (done, 2026-10-16T11:48Z)

