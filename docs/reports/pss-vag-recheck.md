# pss-vag-recheck — GAPS G-008: pss.rs / vag.rs on every movie

Branch `s/pss-vag-recheck` (from origin/master aa3bc1f5). The slim on-foot ISO stubs all 22
`MOVIES/30_FPS/*.PSS` entries (size 0, lba 0), so the retail `007 - Nightfire (USA)` image was downloaded
into the sandbox (2.27 GB, never committed) to run the movie legs on real data.

## What was done

**1. The VAG half, re-verified (this is the recheck).** `s/verify-pss-vag`'s golden test was ported to the
current master tree and re-run: all 25 sound banks parse and decode with 0 errors (6,581 samples), `.SHF`
sizes fill each `.SBF`, every extent ends with the end flag, the 16 music SSDs de-interleave to pinned
hashes, the speech LUT's 828 end-marker streams decode, and all 1552 bank `FileRef`s resolve to a valid
entry (4/4 tests green on the slim image, 111 s). The G-008 loop-point leg was added on top: **every looped
bank sample's loop points are now validated** (28-sample frame multiples, start < end, end = the extent's
last frame — the SPU repeats from the start marker to the end frame) and pinned per bank as `loop_sha1` +
`loop_starts_sum` in `golden/pss_vag.tsv`. `vag.rs` carries the survey-corrected attribution comment
(doc-only diff vs master; both prediction variants stay — no disc evidence prefers one).

**2. The movie half, measured on the retail image** (`crates/nf-formats/tests/pss_movies_g008.rs`, golden
`golden/pss_movies_g008.tsv`, numbers and SHA-1s only, opt-in `NF_MOVIE_ISO`):

- **Demux packet counts**: already golden-tested by `pss_movies_live.rs`; the new file pins the demuxed
  **ES SHA-1s** per movie (video + audio) — the data-image pin G-008 wanted.
- **Video picture count vs the GOP walk**: the GOP start codes split every movie's ES into spans; every
  picture header sits inside exactly one GOP (`pics_outside_gops = 0` on all 22), the walk's total equals
  `mpeg2::picture_counts`, and GOP counts / per-GOP min-max / the I-P-B split are pinned.
- **Audio kind/rate/channels**: kind 1 (16-bit LE PCM), 48 000 Hz, 2 channels on all 20 audio-bearing
  movies; 07330048 and 07350048 (menu backdrop loops) have no audio stream.
- **Audio duration vs video duration, ±1 frame** (tightened from the ±1 s check in `pss_movies_live.rs`):
  15/20 within one frame, deltas pinned in milli-frames. **Five movies disagree — recorded, not fixed**:
  0730000D −3040, 073B0048 −2880, 07300017 −2160, 073F0048 −1200, 073A0048 −1160 milli-frames (audio runs
  1.2–3.0 frames past the video). Evidence this is the file, not the decoder: the sample counts are
  structural (`SSbd body / (2·channels)`), the numbers reproduce byte-identically across two independent
  demux runs (`pss_movies.tsv` came from the earlier retail-image task), and ffprobe confirms the video side
  independently (0730000D: 1700 frames at 30 fps = the port's count; ffmpeg's `pss` demuxer exposes video
  only). They sit in `RECORDED_AUDIO_LONG`; any other file drifting past a frame fails.
- **Every picture decodes without error** (feature `movies`): `decode_video` succeeds on all 22 movies,
  decoded frame counts equal the golden picture counts, every frame is 512×448, and the packed YUV of ALL
  frames hashes to the pinned per-movie `frames_sha1` (~50 fps decoder ≈ 10 min for the disc; a `PIN_ONLY`
  env var restricts re-runs to named movies).

**3. Sister assertions** prove every golden catches tampering: a corrupted `.SHF` size breaks the bank rule;
a flipped ADPCM byte changes the PCM; a flipped SSD frame byte changes the demux hash and the decode; a
flipped type-9 marker fails `stream_data`; a corrupted pack start code fails the demux; `B8→B9` moves a
GOP's pictures outside the walk with the total unchanged; a tampered picture coding type drops the scan by
one and the walk tracks it; a flipped ES byte changes the all-frames YUV hash.

## What was verified (the runs)

- `pss_vag_golden` 4/4 + `pss_movies_g008` 3/3 (skip on stubs) — slim on-foot ISO, `--release`,
  `--test-threads=1`.
- `pss_movies_g008` 5/5 — retail image: the golden test over all 22 movies, the decode test with
  `PIN_ONLY=07100001.PSS` (the pinned-hash assert path), and all three sister tests.
- The full 22-movie decode ran in three passes (20 + 2 + 1 movies; the decoder's ~50 fps exceeded one
  10-minute window); every movie completed error-free and every hash is pinned. One future full
  `every_picture_decodes_without_error` run re-verifies all 22 in one pass.
- **`nf-formats` (the only crate touched) full test run**, slim ISO, `--release --test-threads=1
  --no-fail-fast`: 11 test targets, **117 passed, 3 failed — exactly the 3 `disc_golden` tests the slim-ISO
  policy allows** (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
  `directory_tree_matches_the_golden_manifest`; they compare the full retail file manifest/sizes, which the
  stubbed movies break). Everything else green, including `pss_vag_golden` 4/4 (110 s) and the new
  `pss_movies_g008` 3/3 (skip on stubs).
- The whole 4-crate suite (`tools/test/suite.py`): the sandbox reaps background processes between tool
  calls (observed three times this session — the runner died mid-build each time), and the full run does
  not fit the window in the foreground. The touched-crate run above is complete; `nf-game`/`nf-audio`/
  `nf-view` were not modified (the only source diff is a doc comment in `vag.rs`), so their binaries are
  unchanged; the next message can start `suite.py` on this branch for the full total line.

## Left

- A single-pass full-suite run on the final tree if the sandbox reaped the background runner (the per-crate
  status is above; only `nf-formats` was touched — `vag.rs`'s diff is a doc comment).
- The SPU2 prediction-rounding trace (G-007/G-008's open hardware question; the live check in
  `docs/reports/verify-pss-vag.md` stands: dump one SPU voice's output in PCSX2 and diff against both
  `vag::Variant`s — whichever matches bit-exact is the hardware form).
- G-007 (the OpenGOAL ISC notice for `vag.rs`) — separate gap, untouched.

## Live checks for the main agent

None new. Nothing visual changed (no renders needed). The one open check remains verify-pss-vag's SPU2
rounding dump (SPU voice output vs `vag::Variant` decode of the same `.SBF` bytes).

## ntfy
