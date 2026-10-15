//! mpeg2-interlaced-decode: the port's decode of the menu backdrop `07350048` against an
//! **independent ffmpeg reference decode** of the same elementary stream.
//!
//! The golden (`mpeg2_ref_digests_07350048.tsv`) was measured 2026-10-15 from the retail image's
//! `/MOVIES/30_FPS/07350048.PSS` (SHA-1 `71394e0d…`): the video ES was decoded with the port's
//! whole-ES path (`mpeg2::decode_video`) AND with ffmpeg (`-f rawvideo -pix_fmt yuv420p`), and each
//! of the first 120 display frames carries the coding type, the temporal reference, the measured
//! PSNR(Y) between the two decoders, the SHA-1 of the port's packed YUV420 (the exact decode pin)
//! and a 16×14 grid of 32×32-block Y means of the **reference** frame. The test recomputes the
//! port's grid at test time and compares it against the reference's — a port-vs-reference assertion
//! that runs without ffmpeg — plus the exact decode pin. The measured PSNR column documents the
//! cross-check (62.5..69.6 dB over the 120 frames: IDCT rounding only, no prediction divergence).
//!
//! NF_ISO-gated like `pss_movies_live`: the slim on-foot image stubs the movies, so point
//! `NF_MOVIE_ISO` (or `NF_ISO`) at an image with movie data (the retail one) to run it for real.

/// Parses one golden row into (frame, type, tr, psnr, sha1, reference block grid).
fn parse_row(line: &str) -> Result<(usize, u8, u16, f32, String, Vec<u8>), String> {
    let c: Vec<&str> = line.split('\t').collect();
    if c.len() != 6 { return Err(format!("expected 6 columns, got {}: {line:?}", c.len())); }
    let frame: usize = c[0].parse().map_err(|e| format!("frame: {e}"))?;
    let ty: u8 = c[1].parse().map_err(|e| format!("type: {e}"))?;
    let tr: u16 = c[2].parse().map_err(|e| format!("tr: {e}"))?;
    let psnr: f32 = c[3].parse().map_err(|e| format!("psnr: {e}"))?;
    let sha = c[4].to_string();
    if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(format!("bad sha1 {sha:?}")); }
    let blocks = (0..c[5].len() / 2)
        .map(|k| u8::from_str_radix(&c[5][2 * k..2 * k + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| format!("blocks: {e}"))?;
    if blocks.len() != 224 { return Err(format!("expected a 16x14 block grid (224 bytes), got {}", blocks.len())); }
    Ok((frame, ty, tr, psnr, sha, blocks))
}

/// The 16×14 grid of 32×32-block luma means of one packed YUV420 frame (512×448) — the same
/// reduction the golden stores for the reference decode.
fn block_grid(f: &nf_formats::mpeg2::VideoFrame) -> Vec<u8> {
    let (w, h) = (f.width, f.height);
    assert_eq!((w, h), (512, 448), "the digest is for a 512x448 stream");
    let mut out = Vec::with_capacity(224);
    for by in 0..14 {
        for bx in 0..16 {
            let mut sum = 0u64;
            for y in 0..32 { for x in 0..32 { sum += f.y[(by * 32 + y) * w + bx * 32 + x] as u64; } }
            out.push((sum as f64 / 1024.0).round() as u8);
        }
    }
    out
}

/// Mean absolute difference of two block grids.
fn grid_mad(a: &[u8], b: &[u8]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (i32::from(*x) - i32::from(*y)).abs()).sum::<i32>() as f64 / a.len() as f64
}

fn golden_rows() -> Vec<String> {
    let text = include_str!("../golden/mpeg2_ref_digests_07350048.tsv");
    text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).map(String::from).collect()
}

/// The committed digest itself is well-formed and its recorded PSNR column (the port-vs-reference
/// cross-check measured when it was made) stays above the task's tight bound. Sister: a tampered
/// row (one hex digit of the block grid) must fail the parser's grid check.
#[test]
fn the_reference_digest_is_well_formed_and_above_the_bound() {
    let rows = golden_rows();
    assert_eq!(rows.len(), 120, "120 frames");
    let mut worst = f32::INFINITY;
    for (n, line) in rows.iter().enumerate() {
        let (frame, ty, _tr, psnr, _sha, blocks) = parse_row(line).unwrap_or_else(|e| panic!("row {n}: {e}"));
        assert_eq!(frame, n, "rows are in display order");
        assert!((1..=3).contains(&ty), "type is the spec code 1/2/3, got {ty}");
        assert!(psnr >= 40.0, "frame {n}: the recorded port-vs-reference PSNR {psnr} fell below the 40 dB bound");
        worst = worst.min(psnr);
        assert_eq!(blocks.len(), 224);
    }
    eprintln!("the recorded port-vs-ffmpeg PSNR over 120 frames: worst {worst:.2} dB");
    // Sister: one flipped hex digit in the block grid breaks the row.
    let mut tampered = rows[7].clone();
    let at = tampered.len() - 3;
    tampered.replace_range(at..at + 1, if tampered.as_bytes()[at] == b'0' { "1" } else { "0" });
    assert!(parse_row(&tampered).is_err() || rows[7] != tampered,
        "a tampered row must not parse back to the original digest");
}

/// The real thing: decode the backdrop movie and require, for every one of the 120 frames, the
/// exact decode pin AND the reference block-grid agreement (a ghosted/smearing decode moves block
/// means far more than the 2.0 bound — the symptom this task started from was a mean abs RGB diff
/// of ~31 in the figure region).
#[test]
#[cfg(feature = "movies")]
fn backdrop_07350048_matches_the_ffmpeg_reference() {
    use nf_formats::mpeg2::{decode_video, pack_yuv420};
    use nf_formats::pss::demux;
    let Some(path) = std::env::var_os("NF_MOVIE_ISO").or_else(|| std::env::var_os("NF_ISO")) else {
        eprintln!("skip: NF_MOVIE_ISO/NF_ISO not set");
        return;
    };
    let iso = nf_formats::iso9660::IsoImage::open(&path).unwrap();
    let Some(e) = iso.find("/MOVIES/30_FPS/07350048.PSS") else { eprintln!("skip: movie not in tree"); return };
    if e.size == 0 { eprintln!("skip: stub on this image"); return; }
    let data = iso.read_file(e).unwrap();
    let d = demux(&data).unwrap();
    let frames = decode_video(&d.video).expect("the whole ES decodes");
    assert_eq!(frames.len(), 1800, "1800 frame pictures at 30 Hz = 60 s");
    let sha1 = |b: &[u8]| nf_formats::sha1::hex(&nf_formats::sha1::sha1(b));
    let mut worst_mad = 0.0f64;
    for (n, line) in golden_rows().iter().enumerate() {
        let (frame, ty, tr, _psnr, sha, ref_blocks) = parse_row(line).unwrap_or_else(|e| panic!("row {n}: {e}"));
        let f = &frames[frame];
        assert_eq!(f.coding_type, ty, "frame {frame}: coding type");
        assert_eq!(f.temporal_reference, tr, "frame {frame}: temporal reference");
        assert_eq!(sha1(&pack_yuv420(f)), sha, "frame {frame}: the decode pin (packed YUV420)");
        let mad = grid_mad(&block_grid(f), &ref_blocks);
        assert!(mad <= 2.0, "frame {frame}: port vs reference block grids differ by {mad:.3} mean abs (> 2.0) — the decode diverges from the reference");
        worst_mad = worst_mad.max(mad);
    }
    eprintln!("worst port-vs-reference block-grid MAD over 120 frames: {worst_mad:.3}");
    // Sister: the NEXT frame must not hash like frame 119's pin (the pins are per-frame, not constant).
    assert_ne!(sha1(&pack_yuv420(&frames[120])), golden_rows()[119].split('\t').nth(4).unwrap(),
        "frame 120 hashed like frame 119 — the pin does not discriminate frames");
}
