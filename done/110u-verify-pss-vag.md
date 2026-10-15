## Your task: `verify-pss-vag`, check the movie and audio decoders on Nightfire's own files (G-008)

G-008 (docs/GAPS.md): `crates/nf-formats/src/pss.rs` and `vag.rs` came from the RC2 port and were verified only on RC2's
files. Verify them on this disc: `MOVIES/30_FPS/*.PSS` (and any other PSS/VAG/VAG-in-VPK the disc carries).

Do:
1. For every PSS on the disc: demux it (video MPEG-2 elementary stream + the audio stream), count the frames, the
   audio blocks and the duration, and check each against an independent tool where one exists in work/third_party
   (ffprobe if present; otherwise the PSS header fields and the MPEG sequence headers). Report mismatches and their cause.
2. For the audio: decode a sample of each format variant (interleave size, channels, rate) and check the sample counts
   against the header durations. Where an independent decoder exists (vgmstream — ISC, reuse allowed), compare the PCM
   bit-exactly on 3 files.
3. Test (NF_ISO, numbers only, no game bytes): golden counts per file (frames, audio samples, SHA-1 of the demuxed
   elementary streams), each with a sister assertion that a corrupted header fails.

Files: `crates/nf-formats/src/pss.rs` / `vag.rs` (fixes only if needed), `crates/nf-formats/tests/pss_vag_golden.rs`
(new), docs/formats/pss.md, docs/GAPS.md (G-008).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).


**Branch:** `s/verify-pss-vag`.

---
claimed-by: sandbox5 at 2026-10-15T13:21Z

---
## Report (done, 2026-10-15T14:17Z)

