# mpeg2-interlaced-decode — the movies were decoded right; they were *converted* wrong

Branch `s/mpeg2-interlaced-decode`, task 605. Sandbox 18:34–20:32 UTC 2026-10-15.

## What's done

**The suspected bug was disproven with numbers first.** The task suspected wrong interlaced predicted-picture
reconstruction (field pictures, field/dual-prime prediction, `top_field_first`, field DCT, B backward
references). Evidence against:

- The crate's own conformance suite (`oxideav-mpeg12video` 0.0.13, run in a copy with `oxideav-core` resolved
  to the cached 0.1.39): **1462 passed / 0 failed** — including bit-exact fixtures for field-picture pairs
  (dual-prime + 16×8 MC), custom quant matrices, `intra_vlc_format`, interlaced + half-MV, IBBP reorder.
- Synthetic ffmpeg-encoded streams (512×448, `-flags +ilme+ildct -top 1 -alternate_scan 1`, ±B-frames, plus a
  full-noise stream that forces field MC/field DCT): the port's whole-ES decode matches ffmpeg's reference at
  **65.9–76.0 dB Y-PSNR** (MAD 0.01–0.02 — IDCT rounding only).
- The REAL streams, range-fetched from the retail image on the file host (`007 - Nightfire (USA).iso`, HTTP
  Range, sectors only; SHA-1s verified against `golden/disc_files.tsv`: `71394e0d…` for 07350048,
  `cc4c74f7…` for 07330048, `91baca13…` for 07100005): **07350048 = 62.5–69.6 dB over the first 120 frames**
  (I 69.4–69.6, P 62.5–64.7; 1800/1800 frames decode 1:1), **07100005 = 65.1–76.1 dB** (I 70.7+, P 65.3+,
  B 65.1+, n=7/33/80 over 120 frames). Per-frame tables committed:
  `crates/nf-formats/golden/mpeg2_ref_digests_07350048.tsv` (frame, type, temporal_reference, PSNR, port SHA-1,
  reference block means) and `golden/mpeg2_ref_psnr_07100005.tsv`. The golden row 180 I + 1620 P + **0 B**
  also settles the structure: frame pictures at 30 fps (ffmpeg: exactly 60.00 s), so field *pictures* were
  never in play.

**The real bug — colour, not prediction.** The streams are BT.601 **limited range**: 07350048's Y spans
14..218 (black = 16, white = 235). The game's picture goes through the IPU's hardware CSC into RGBA32
([M movies.md: `sceMpegGetPicture` decodes straight into a 512×448 RGBA32 buffer]; the IPU/PCSX2 `IPU::CSC`
expands Y 16–235 → full range). The port's `yuv420_to_rgb8` used Y **raw** with the full-range chroma gains
(1.402/0.344/0.714/1.772): black rendered as RGB(16,16,16), white as 235, everything sat at ~86 % contrast —
the "soft, low-contrast ghost" (the silhouette is soft in the *source art* too — it's a shadow dancer — but
the crushed blacks and desaturated chroma made it mushy). **Fixed** (`crates/nf-formats/src/mpeg2.rs`): the
BT.601 limited→full matrix (1.1644·(Y−16), 1.5960/0.3918/0.8130/2.0172 chroma) — the darkest figure pixels now
reach RGB 0 (were 16+), verified by re-decoding frame 12 of 07330048 (the 007 title card) and eyeballing the
PNG.

**Also fixed while there:** `VideoFrame::coding_type` reported the crate's 0-based enum (`as u8`) — every I
picture said 0 (rendered "?" in dumps) and every P said 1 ("I"); it now reports the spec codes 1=I/2=P/3=B
(the per-type PSNR split above depends on it).

**The test** (`crates/nf-formats/tests/mpeg2_reference.rs`, NF_ISO + `movies`-gated, skips on images without
movie data like `pss_movies_live`):

- `backdrop_07350048_matches_the_ffmpeg_reference`: decodes the whole ES, then for each of the 120 digest
  frames requires the exact SHA-1 pin of the port's packed YUV420, the coding type + temporal reference, and
  the port's recomputed 16×14 grid of 32×32-block Y means vs the **reference's** committed grid at
  **MAD ≤ 2.0** — a port-vs-reference assertion that runs without ffmpeg (a ghosting decode like the
  symptom's ~31 mean-abs-diff would blow through it). Sisters: frame 120 must not hash like frame 119.
- `the_reference_digest_is_well_formed_and_above_the_bound`: 120 rows, spec type codes, recorded PSNR ≥ 40 dB
  (worst recorded 62.52 dB); sister: a tampered row fails the parser.

**Kept:** `Player::present`'s wall-clock 30 fps pacing — untouched.

**Tooling** (committed, small): `crates/nf-formats/examples/dump_mpeg2_yuv.rs` (ES → packed YUV420 + per-frame
type/temporal-reference log) and `dump_pss_es.rs` (PSS → our demuxer's video ES) — the two halves of any future
decoder-vs-reference comparison.

**Pin updates:** the CSC fix legitimately changes RGB8 output, so the menu-backdrop pins moved
`1f1fe7b0…` → `21d0739d…` (`menu_movies.rs` ×2, `menu_backdrop_still.rs`, and the doc comments citing them);
re-measured from the whole-ES decode with the fixed conversion, and the streaming path still agrees
(`menu_movies` 5/5 + `menu_backdrop_still` 7/7 green against a 3-movie mini ISO as `NF_MOVIE_ISO`).
`docs/formats/pss.md` gained the decode-accuracy section.

## How it was verified

- `cargo test --offline --release -p nf-formats --features movies --test mpeg2_reference` on the mini ISO:
  **2 passed / 0 failed** (47 s — the whole-ES decode dominates).
- `cargo test ... -p nf-view --features movies --test menu_movies --test menu_backdrop_still` on the mini ISO:
  **5 + 7 passed / 0 failed**.
- `cargo test ... -p nf-formats --features movies --test pss_movies_live` on the mini ISO: 2 passed, 2 failed —
  the two failures are the tests needing all 22 movies (`every_movie…`, `audio_sample_count…` walk the golden's
  22 paths; the mini ISO holds 3). Pre-existing data availability, not a regression; on the slim ISO (the
  suite's image) they skip.
- Synthetic-stream PSNR runs (progressive, interlaced ±B, noise) as above.
- Full suite on the final tree (merged with origin/master 678c9195): the total line below.

## Render commands for the main agent

The visual confirmation (no GPU here):

- `NF_MOVIE_ISO=<retail image> cargo run --release -p nf-view --features movies -- --movie 07350048 --frame 12 --shot /tmp/b12.png`
  — the list-page backdrop; with the fix the sunset should be saturated and the shadow-dancer silhouette a
  true black against it (previously grey-brown).
- Same for `--movie 07330048 --frame 12` — the 007 title card: tuxedo black now RGB≈0 (it was ≈16–25).
- The frame's own content is intentionally soft (a blurred silhouette in the source art — verified identical
  to ffmpeg's decode), so the bar is "same contrast/colours as PCSX2", not "sharper".

## What's left (owned by the next task, with the repro)

1. The per-frame PSNR tables here are whole-ES decodes; the same comparison through the **streaming** path
   (`MovieStream`, the closed-GOP chunk worker) is pinned indirectly (the frame-12 pin equals the whole-ES
   decode's; `menu_movies` walks 90 advances) but a 120-frame chunked-vs-whole digest sweep would close it
   completely. The chunk boundaries only sit at closed GOPs, so this should be a formality.
2. The digest golden pins the CURRENT crate's decode; if `oxideav-mpeg12video` is ever bumped, regenerate
   (`dump_pss_es` + `dump_mpeg2_yuv` + the python block-grid snippet in the report history) and update the
   pin — the test's failure message says exactly which frame moved.
3. The chroma upsampling in `yuv420_to_rgb8` is still nearest-neighbour frame-aligned; interlaced 4:2:0
   chroma is field-aligned, so fast vertical motion can show ~½-row chroma softness (luma unaffected — not
   part of the reported symptom). Field-aware chroma upsampling would be the next refinement if the main
   agent's render shows it.

## Live checks for the main agent

1. Render the two `--movie --frame 12 --shot` commands above from the retail image and confirm the
   contrast/colours vs PCSX2's front end (the fix's visual acceptance).
2. If convenient: a PCSX2 screenshot of the same frame (07350048 frame 12, list page) next to the port's
   shot — the source silhouette is soft, so compare blacks and saturation, not sharpness.

## Files touched

- `crates/nf-formats/src/mpeg2.rs` (the CSC fix + coding-type mapping; both with the evidence in comments).
- `crates/nf-formats/tests/mpeg2_reference.rs` (new), `crates/nf-formats/golden/mpeg2_ref_digests_07350048.tsv`
  (new), `crates/nf-formats/golden/mpeg2_ref_psnr_07100005.tsv` (new).
- `crates/nf-formats/examples/{dump_mpeg2_yuv,dump_pss_es}.rs` (new tooling).
- `crates/nf-view/src/movie.rs` (doc comment pin), `crates/nf-view/tests/{menu_movies,menu_backdrop_still}.rs`
  (re-pinned).
- `docs/formats/pss.md`, `docs/WORKLOG.md`.

## Full-suite result on the final tree

The fast runner (`tools/test/suite.py`) on the final tree (this branch merged with origin/master 678c9195,
348 test binaries): **build green in 326 s; the run was still in progress at push time (20:02 UTC) with
~107/348 binaries done and zero FAILED/TIMEOUT/CRASH lines** — the sandbox's 2-hour window ends before the
full run can print its total (the whole-ES decode tests and the 070000xx live missions dominate the tail).
The runner keeps going in the background; the per-binary results stream into `suite.log` and the total will
land in `suite.out` — the next session should read `tail -1 suite.out` before touching the movies code.
Everything this task touched ran green in the targeted passes above (nf-formats `mpeg2_reference` 2/2 with
`--features movies` on movie data; nf-view `menu_movies` 5/5 + `menu_backdrop_still` 7/7; both feature/no-
feature builds of nf-formats' tests compile).

## ntfy failed

All three fallbacks failed with the ntfy.sh daily quota (the same 429 the 110p session hit earlier today):

1. `curl -sS -H "Title: NightfirePort" -d "mpeg2-interlaced-decode: done …" https://ntfy.sh/nightfire-yoda-8k3x`
   → `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. python urllib → `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. `wget -q -O- --post-data "…" https://ntfy.sh/nightfire-yoda-8k3x` → no output (429), exit status recorded as failure.
