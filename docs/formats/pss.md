# PSS — the movie program stream (`MOVIES/30_FPS/*.PSS`)

22 files on the disc (`crates/nf-formats/golden/disc_files.tsv`), played by `psiStartBackgroundMovie__FUiScUi`
(0x001081a8) as `sprintf("%s%s%8.8x.pss%s", HostName, DirName, code, EndTag)` with `HostName` = `"cdrom0:\movies\"`
(0x002e8180), `DirName` = `"30_FPS\"` (.sdata 0x0030c690) and `EndTag` = `";1"` — i.e.
`cdrom0:\movies\30_FPS\07100001.pss;1` for hashcode 0x07100001. See docs/systems/movies.md for when each movie
plays. Verified on the retail image against `golden/pss_movies.tsv` (this closes the NF_ISO half of G-008);
`pss.rs`/`vag.rs` were RC2-verified only before that.

## Container

An MPEG-2 program stream (the same shape RC2 uses):

* **Packs** `00 00 01 BA` (MPEG-2 form, 14 bytes + stuffing), a system header `BB` in the first pack, PES packets
  `E0` (video), `BD` (private stream 1: audio), `BE` (padding), and the end code `00 00 01 B9` at the end of the
  file (no trailing bytes after it).
* **Video** PES packets (`E0`) carry an MPEG-2 video elementary stream; PTS (and DTS for B-frames) ride the PES
  headers. Payloads concatenated in file order form the ES that `nf-formats::mpeg2` decodes.
* **Audio** PES packets (`BD`): the payload starts with the 4-byte sub-stream header `FF A0 00 00` — **`A0`, not
  RC2's `A1`** (all 53,374 audio packets on the disc measured; byte 3 is the channel, always 0: Nightfire
  registers one audio stream — `StartUpBGFMV` 0x001083a8 adds only str-id 2, mapped by `_strmap` 0x002e4d58 to
  0xBD / 0xFFA00000 — there is no language selection in the movie path). The demuxer accepts both `A0` and `A1`.

## Audio: 16-bit little-endian PCM (not RC2's SPU ADPCM)

The channel bytes begin with the 0x28-byte `SShd`/`SSbd` header (`SShd` size 0x18, then kind, rate, channels,
interleave, loop start/end as u32s; `SSbd` + body size at 0x20/0x24):

| field | value on every audio-bearing movie (20 of 22) |
|---|---|
| kind | **1 = 16-bit LE PCM** (RC2's movies use 0x10 = SPU ADPCM; both are accepted by `pss.rs`) |
| rate | 48000 Hz |
| channels | 2 |
| interleave | 512 |
| `SSbd` body | `seconds · 48000 · 2 · 2` bytes exactly, e.g. 07100001.PSS: 8,194,048 = 42.67 s |

The body is plain interleaved i16 LE sample pairs (L, R) — `pss::Audio::decode` returns them directly for kind 1;
the RC2 de-interleave + SPU-ADPCM path is kept for kind 0x10. The two movies without audio
(07330048/07350048, byte-identical sizes) carry no `BD` packets at all.

## Video: MPEG-2 Main Profile @ Main Level

Measured on all 22 movies (golden/pss_movies.tsv; picture counts by §6.2.3 start codes `00 00 01 00`):

| field | value |
|---|---|
| size | **512 × 448** (fits the game's 0x200×0x1c0 RGBA32 background, `AllocateMovieMemory` 0x00107d70 = 512·448·4) |
| chroma | 4:2:0, `progressive_sequence = 0` (interlaced), profile 4 (Main) / level 10 (Low) |
| frame rate | code 5 = 30 Hz (the folder name `MOVIES/30_FPS`) |
| pictures | I / P / B mix, e.g. 07100001: 1280 = 72 I + 358 P + 850 B (42.67 s); 073F0048 (win game): 4230 |

Decoding: `nf-formats::mpeg2` (feature `movies`) wraps the pure-Rust MIT crate `oxideav-mpeg12video` 0.0.13 —
decode + §6.1.1.11 B-frame display reorder; frame 0/1000 of 07100001 pin to SHA-1
`54b75479b7bbfd6d8e2492e8fc97072484ccbf3d` / `f965b37f552c898f75df29895c83e63aebe50134`, cross-checked against
ffmpeg on the same ES (IDCT rounding differs between decoders: PSNR 67.8 / 66.3 dB, same geometry).

Decode accuracy (mpeg2-interlaced-decode, 2026-10-15): the crate's decode matches an independent ffmpeg decode
within IDCT rounding on the real streams — `07350048` 62.5–69.6 dB Y-PSNR over 120 frames (I 69.4+, P 62.5+;
the interlaced suspicion was wrong: the crate's own conformance suite — field pictures, dual-prime, 16×8 MC,
custom quant matrices — is 1462/0, and the port matches it). The real presentation bug was **colour**: the
movies are BT.601 **limited range** (07350048's Y spans 14..218 — black 16, white 235), the IPU's RGBA32 CSC
expands that to full range (movies.md: `sceMpegGetPicture` decodes straight into RGBA32), and the old
`yuv420_to_rgb8` used Y raw with the full-range chroma gains — black rendered as RGB(16,16,16) and every movie
sat at ~86 % contrast ("soft, low-contrast ghost"). `yuv420_to_rgb8` now runs the limited→full matrix.
Reference digests: `golden/mpeg2_ref_digests_07350048.tsv` (per-frame type/temporal-reference/PSNR/port-SHA-1 +
reference block means; `mpeg2_reference.rs` replays them) and `golden/mpeg2_ref_psnr_07100005.tsv` (the
briefing's 120-frame PSNR table: I 70.7+, P 65.3+, B 65.1+ dB).

## Tests

`crates/nf-formats/tests/pss_movies_live.rs` (opt-in NF_ISO, like the other golden tests): every movie demuxes to
the golden numbers (packet counts per kind, video bytes, geometry, picture counts, audio format, sample counts);
stream ids of the first briefing pinned (`E0` video, `FF A0 00 00` audio, `BB`/`BE`, end code); audio sample
count = `SSbd body / (2·channels)` and agrees with the video picture count at 30 Hz to <1 s for all 20
audio-bearing movies; SHA-1s of frames 0 and 1000 (feature `movies`). Sister assertions flip one value in each
comparison. The slim on-foot image stubs the movies (size 0) — point `NF_MOVIE_ISO` (or `NF_ISO`) at an image
with movie data (e.g. the retail `007 - Nightfire (USA)` image).

## G-008: the recheck on the disc's own data (pss-vag-recheck)

`crates/nf-formats/tests/pss_movies_g008.rs` (opt-in `NF_MOVIE_ISO`, golden `golden/pss_movies_g008.tsv` —
numbers and SHA-1s only) adds the movie legs G-008 asked for, measured on the retail image:

- **The GOP walk vs the picture count**: the GOP start codes (`00 00 01 B8`) split every movie's video ES into
  spans; every picture header sits inside exactly one GOP (`pics_outside_gops = 0` on all 22), the walk's
  total equals `mpeg2::picture_counts`, and the GOP count, per-GOP min/max and the I/P/B split are pinned.
- **The ES SHA-1s**: the demuxed video and audio elementary streams hash to pinned values — the data-image
  pin G-008 wanted (any byte drift in demuxing is caught).
- **The audio kind/rate/channels and the sample count**: kind 1 (16-bit LE PCM) at 48 kHz, 2 channels, for all
  20 audio-bearing movies; 07330048 and 07350048 (the two menu backdrop loops) carry no audio stream.
- **The audio duration vs the video duration, ±1 frame** (the old live test allowed ±1 s): 15/20 sit within
  one frame; the deltas are pinned in milli-frames. Five movies run an audio bed padded past the video end —
  0730000D −3040, 073B0048 −2880, 07300017 −2160, 073F0048 −1200, 073A0048 −1160 milli-frames (i.e. 1.2–3.0
  frames of extra audio). This is a property of the files, not a decoder error: the sample counts are
  structural (`SSbd body / (2·channels)`), the counts reproduce byte-identically across the two independent
  demux runs (`pss_movies.tsv` from the earlier retail-image task and this recheck), and ffprobe confirms the
  video side independently (0730000D: 1700 frames at 30 fps — the same count the port's scan produces;
  ffmpeg's `pss` demuxer exposes video only, so the audio leg has no third-party counter beyond the structural
  identity). They are recorded in `RECORDED_AUDIO_LONG`; any other file drifting past a frame fails the test.
- **Every picture decodes without error** (feature `movies`): `decode_video` succeeds on all 22 movies, the
  decoded frame count equals the golden picture count, every frame is 512×448, and the packed YUV of ALL
  frames hashes to the per-movie `frames_sha1`. The decoder averages ~50 fps, so the full walk is ~10 min;
  `PIN_ONLY=name1,name2` restricts a re-run to named movies.
- Sisters: a tampered GOP start code (`B8→B9`) moves its pictures outside the walk with the total unchanged;
  a tampered picture coding type drops the scan by exactly one and the walk tracks it; a flipped ES byte
  changes the all-frames hash.

The VAG half (this branch re-verified the unmerged verify-pss-vag work and added the loop points):
`crates/nf-formats/tests/pss_vag_golden.rs` — all 25 banks parse and decode with 0 errors on the slim image,
`.SHF` sizes fill the `.SBF`, every extent ends with the end flag, **every looped sample's loop points are
validated** (frame multiples, start < end, end = the extent) and pinned per bank (`loop_sha1` +
`loop_starts_sum` in `golden/pss_vag.tsv`); the 16 music SSDs de-interleave to pinned hashes; the speech LUT's
828 end-marker streams decode and all 1552 bank `FileRef`s resolve. `vag.rs` keeps both prediction variants;
the DuckStation/ffmpeg/vgmstream attribution survey (2026-10-15) stands: no published software decoder
implements the port's Rounded form, so the SPU2 hardware form stays a live-check question (verify-pss-vag's
report lists the definitive PCSX2 check).
