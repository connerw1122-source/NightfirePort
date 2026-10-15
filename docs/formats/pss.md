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

## Tests

`crates/nf-formats/tests/pss_movies_live.rs` (opt-in NF_ISO, like the other golden tests): every movie demuxes to
the golden numbers (packet counts per kind, video bytes, geometry, picture counts, audio format, sample counts);
stream ids of the first briefing pinned (`E0` video, `FF A0 00 00` audio, `BB`/`BE`, end code); audio sample
count = `SSbd body / (2·channels)` and agrees with the video picture count at 30 Hz to <1 s for all 20
audio-bearing movies; SHA-1s of frames 0 and 1000 (feature `movies`). Sister assertions flip one value in each
comparison. The slim on-foot image stubs the movies (size 0) — point `NF_MOVIE_ISO` (or `NF_ISO`) at an image
with movie data (e.g. the retail `007 - Nightfire (USA)` image).

## G-008 verification (2026-10-15, verify-pss-vag)

The slim on-foot image carries the whole VAG/SPU-ADPCM side of the disc, so the audio decoders are now verified
on Nightfire's own files (`crates/nf-formats/tests/pss_vag_golden.rs`, goldens `golden/pss_vag.tsv`, measured on
this image):

* **Banks** (`PS2/ENGLISH/SB_n/SB_n.{SHF,SFX,SBF}`, all 25 slots): headers parse, the 36-byte `.SHF` sample sizes
  sum exactly to the `.SBF` for every bank, every sample's extent ends with the end flag (`vag::sample_extent`),
  looped counts 2–19, per-bank decoded-PCM SHA-1s pinned (0 decode errors across 6,581 samples).
* **Music** (`MFX_n.SSD`, 16 files): whole 0x100 blocks, per-channel de-interleaved frame streams SHA-1-pinned,
  every `.SMF` marker byte position inside the stream, stereo PCM decode SHA-1s pinned on MFX_1/11/16.
* **Speech** (`STREAMS.LUT`/`STREAMS.BIN`): 1681 LUT entries, all 136-byte headers, 828 carry a type-9 end
  marker (29,322,752 played bytes; the other 853 are unreferenced stale takes), three named decodes pinned, and
  every stream `FileRef` in all 25 banks (1552 references, 675 distinct entries) resolves to a valid entry.
* **Movies**: the 22 tree entries exist (size-0 stubs on the slim image, counted and skipped; on an image with
  data each demuxes to its `pss_movies.tsv` row). The ES SHA-1s print under `--nocapture` for a data-image run.

Independent decoders (the rounding question): ffmpeg's `vag` demuxer (`adpcm_psx`) decodes the port's frames
bit-exactly for structure (nibble order, shift/filter byte, coefficients, 16-byte frames) — its prediction is
trunc-toward-zero of the *summed* products (`(k0·h1 + k1·h2) / 64`), measured on 3 bank samples, both MFX_1
channels and 3 speech streams (bit-exact under that form; the port's Rounded drifted ≤ 27 over 16 884 samples,
OpenGoal ≤ 78). DuckStation master (`spu.cpp`) shifts each product separately (the OpenGoal form); vgmstream
(`psx_decoder.c`) uses float coefficients. **No published software decoder implements the rounded form**, so
`vag.rs`'s two variants stay and its attribution note was corrected (the old comment called Rounded "the
PCSX2/DuckStation form" — DuckStation's master source is per-term). Which form the real SPU2 plays is still
open — a live check would settle it (docs/reports/verify-pss-vag.md).
