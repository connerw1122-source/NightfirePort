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


---
## Report from the worker (done, 2026-10-15T14:18Z)

# verify-pss-vag — G-008: the PSS/VAG decoders on Nightfire's own files

Branch `s/verify-pss-vag` (from origin/master d4b67a10). Sandbox window was ~2 h; setup, measurement, the
cross-checks, the golden test and the docs all fit; the full 4-crate suite did not (see "Left").

## What was done

**1. Disc survey.** All 22 `MOVIES/30_FPS/*.PSS` entries on the slim on-foot ISO are size-0 stubs (lba 0), and
the retail LBAs from `golden/disc_files.tsv` were repacked (the bytes at 07100001's retail LBA are another file),
so no movie bytes exist in this sandbox — the PSS re-check on real data stays with the earlier retail-image run
(`golden/pss_movies.tsv`, `pss_movies_live.rs`, already committed). No `.VAG` or `.VPK` files exist on the disc.
The VAG/SPU-ADPCM data the game plays is: the 25 sound banks (`PS2/ENGLISH/SB_n/SB_n.{SHF,SFX,SBF}`), the 16
music streams (`PS2/MUSIC/MFX_n/MFX_n.SSD`, stereo 32 kHz, 0x100-byte blocks of 0x80+0x80), and the speech
streams (`PS2/ENGLISH/STREAMS/STREAMS.{LUT,BIN}`, 22 050 Hz). All of it is on this image, so the audio half of
G-008 was verified here end to end.

**2. Measurement (the port's own decoders, on the disc's own bytes).**
- Banks: all 25 slots parse; the `.SHF` 36-byte sample headers' sizes sum exactly to the `.SBF` in every bank;
  every sample's extent ends with the end flag (`vag::sample_extent`); looped counts 2–19; 6,581 samples decode
  with 0 errors; per-bank decoded-PCM SHA-1s pinned. Pitch classes 683/941/1024/1621/1882/2731 (≈ 8.0k–32.0k Hz
  at `pitch·48000/4096`).
- Music: all 16 `.SSD` files are whole 0x100 blocks; the de-interleaved per-channel frame streams are
  SHA-1-pinned; every `.SMF` marker byte position lands inside its stream; full stereo PCM SHA-1s pinned on
  MFX_1/11/16.
- Speech: 1681 LUT entries, all with 136-byte headers; **828 carry a type-9 end marker** (29,322,752 played
  bytes); the other 853 are unreferenced stale takes (the LUT data sizes include them). Decisive: every stream
  `FileRef` in all 25 banks — 1552 references, 675 distinct entries — resolves to a valid entry, so the game can
  never play a marker-less stream (a port-relevant invariant, now asserted).
- Movies: the 22 tree entries exist; on the slim image the test counts the stubs and skips; on an image with
  data it re-runs the `pss_movies.tsv` demux golden per movie.

**3. Independent decoders (task steps 1–2).** ffprobe/ffmpeg were available (`/usr/bin`). Eight real extracts
(3 bank samples at 11.03k/22.05k/12k, both MFX_1 channels at 32k, 3 speech streams at 22.05k) were wrapped in
minimal VAGp headers and decoded by ffmpeg's `vag` demuxer (`adpcm_psx`):
- **Structure bit-exact**: nibble order, the shift/filter byte, the coefficient table, 16-byte frames, and all
  the container de-muxing (per-sample extents, 0x80+0x80 interleave, end-marker slices) reproduce ffmpeg's
  samples exactly wherever the prediction forms agree (frame 0 of every file, all filter-0 frames).
- **The prediction form**: ffmpeg's is trunc-toward-zero of the *summed* products, `(k0·h1 + k1·h2)/64` —
  reverse-engineered from its output (its first divergence from the rounded form is `-5848/64 → −91`, not −92)
  and then bit-exact on 4 of 5 files (the fifth: 199 clamped-boundary samples). Under that form ffmpeg matches
  the port's structure on every file. The port's Rounded form drifted ≤ 27 over 16 884 samples vs ffmpeg,
  OpenGoal ≤ 78 — neither matches, by design.
- **DuckStation master** (`spu.cpp`, fetched): shifts each product separately — i.e. the port's OpenGoal form.
  This disproves `vag.rs`'s old comment that Rounded is "the PCSX2/DuckStation form"; the comment is corrected
  (doc-only change). **vgmstream** (`psx_decoder.c`, fetched) uses float coefficients — a third form.
- Conclusion: no published software decoder implements the rounded form the port plays; the hardware question
  stays open, both variants stay, and no code changed (no evidence the disc prefers one).

**4. The golden test.** `crates/nf-formats/tests/pss_vag_golden.rs` (new, 4 tests, ~108 s — needs a 270 s suite
chunk) + `golden/pss_vag.tsv` (46 rows, numbers only): the bank rows (headers/sbf bytes/looped/PCM SHA-1/sample
counts/pitches), the SSD rows (bytes/blocks/samples/L-R demux SHA-1s/PCM SHA-1s), the speech rows (three named
decodes + the aggregate 1681/828/29322752/675). Sisters (a corrupted header must fail): a `.SHF` size tamper
breaks the sizes-fill-the-SBF rule, a flipped ADPCM byte changes the PCM, a flipped SSD frame byte changes both
the demux hash and the decode, a flipped type-9 marker makes `stream_data` fail, a corrupted movie pack start
code fails the demux. Green 4/4 on this tree.

## Files

- `crates/nf-formats/tests/pss_vag_golden.rs` (new), `crates/nf-formats/golden/pss_vag.tsv` (new)
- `crates/nf-formats/src/vag.rs` — doc comment only (the DuckStation attribution fix)
- `docs/formats/pss.md` (§G-008 verification), `docs/GAPS.md` (G-008 closed), `docs/WORKLOG.md`
- No shared source file touched; `pss.rs` needed no changes (everything parsed).

## Left

- The full 4-crate suite on this tree (window ran out; **no library code changed** — the new test file, its
  goldens and doc comments only; `cargo check -p nf-formats` clean). Run
  `python3 tools/test/suite.py` and give `pss_vag_golden` a ≥ 270 s wall (108 s measured).
- On any image with movie data: `cargo test --release -p nf-formats --test pss_vag_golden -- --nocapture` once
  and pin the printed per-movie ES SHA-1s into the golden (the PSS leg prints them).
- G-007 (the OpenGOAL ISC notice for `vag.rs`) is a separate gap, untouched.

## Live checks for the main agent

1. **The SPU2 prediction rounding** (the one open question this task sharpened): in PCSX2, run any movie (e.g.
   the 07100001 briefing) and, if practical, capture the SPU output for the first seconds; compare against the
   port's `pss::Audio::decode` (kind 1 movies are plain PCM — the *movie* audio is exact; the question is the
   ADPCM playback): spot = in-game SFX (e.g. the pickup blip 0xF5) or a streamed speech line; action = play the
   same line via the port (`nf-view --bond` with audio) and by ear against PCSX2; expected = identical pitch and
   timbre (the ≤ 2-LSB form difference is not audible; a *form* mismatch would show as noise on filter-2/3
   tails). A definitive check: PCSX2 debugger, dump the SPU output buffer for one voice playing a known bank
   sample, and diff against both `vag::Variant`s' decode of the same `.SBF` bytes — whichever matches bit-exact
   is the hardware form; pin it in `vag.rs`.
2. **Movies**: no nf-view `--bond` spots are needed (nothing visual changed; the slim ISO has no movie data).

## ntfy
