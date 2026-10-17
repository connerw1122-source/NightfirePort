//! G-008's movie legs over every `MOVIES/30_FPS/*.PSS` with data, on the disc's own bytes. Opt-in like
//! `pss_movies_live.rs`: the slim on-foot image stubs all 22 movies (size 0), so every test here skips there;
//! point `NF_MOVIE_ISO` (or `NF_ISO`) at the retail `007 - Nightfire (USA)` image to run them for real.
//!
//! Each movie is checked against `golden/pss_movies_g008.tsv` (numbers and SHA-1s only):
//! - **the GOP walk**: the GOP start codes (`00 00 01 B8`) split the video ES into spans; every picture header
//!   (`00 00 01 00`, coding type 1..=3) must sit inside exactly one GOP, and the walk's picture total must
//!   equal `mpeg2::picture_counts` and the golden `pictures` column (with the per-type split);
//! - **the audio kind/rate/channels** (kind 1 PCM at 48 kHz stereo) and the sample count
//!   (`SSbd` body / (2·channels));
//! - **the audio duration against the video duration to within ONE frame** at 30 Hz — G-008 tightens the
//!   ±1 s check in `pss_movies_live.rs`; the measured delta is pinned in milli-frames (`delta_milliframes`);
//! - **every picture decodes without error** (feature `movies`): the decoded frame count equals the picture
//!   count and the packed YUV of ALL frames hashes to the golden per-movie `frames_sha1`.
//!
//! The ES-level SHA-1s (`video_es_sha1`, `audio_es_sha1`) pin the demuxed streams byte-exactly — the pin
//! G-007/G-008 asked for from one data-image run. `PIN_GOLDEN=1` prints the actual rows for re-pinning.
//!
//! Sister assertions: a tampered GOP start code moves its pictures outside the walk; a tampered picture
//! coding type drops both counts in lockstep; a tampered ES byte changes the all-frames hash.

use nf_formats::golden::diff_tsv;
use nf_formats::iso9660::IsoImage;
use nf_formats::mpeg2::picture_counts;
use nf_formats::pss::{demux, Audio};
use nf_formats::sha1::hex;
#[cfg(feature = "movies")]
use nf_formats::sha1::Sha1;

/// The movies image: `NF_MOVIE_ISO` if set, else `NF_ISO`. None → skip.
fn iso() -> Option<IsoImage> {
    let path = std::env::var_os("NF_MOVIE_ISO").or_else(|| std::env::var_os("NF_ISO"))?;
    Some(IsoImage::open(&path).unwrap_or_else(|e| panic!("movie iso {path:?}: {e}")))
}

/// The disc's 22 movie paths under `/MOVIES/30_FPS`, sorted, from the golden manifest itself.
fn movie_paths() -> Vec<String> {
    let mut v: Vec<String> = include_str!("../golden/disc_files.tsv")
        .lines()
        .skip(1)
        .map(|l| l.split('\t').next().unwrap().to_string())
        .filter(|p| p.starts_with("/MOVIES/30_FPS/") && p.ends_with(".PSS"))
        .collect();
    v.sort();
    assert_eq!(v.len(), 22, "the golden disc manifest holds 22 movies");
    v
}

/// Every movie entry with data on this image (on the slim on-foot ISO: none).
fn movies_with_data(iso: &IsoImage) -> Vec<(String, Vec<u8>)> {
    let mut v = Vec::new();
    for p in movie_paths() {
        let Some(e) = iso.find(&p) else { panic!("{p} not in the image tree") };
        if e.size > 0 {
            let data = iso.read_file(&e).unwrap_or_else(|err| panic!("{p}: {err}"));
            assert_eq!(data.len(), e.size as usize, "{p}: ISO extent size");
            v.push((p, data));
        }
    }
    v
}

/// Walks the GOP structure of an MPEG-2 elementary stream:
/// `(gop_count, pictures_per_gop, pictures_before_the_first_gop)`. Each picture header with a valid
/// coding type (1..=3, the same match `picture_counts` makes) belongs to the last GOP starting at or
/// before it; MPEG-2 forbids start-code emulation, so byte matches are the real headers.
fn gop_walk(es: &[u8]) -> (usize, Vec<usize>, usize) {
    let mut gop_at: Vec<usize> = Vec::new();
    let mut pic_at: Vec<usize> = Vec::new();
    for i in 0..es.len().saturating_sub(5) {
        if es[i] == 0 && es[i + 1] == 0 && es[i + 2] == 1 {
            match es[i + 3] {
                0xb8 => gop_at.push(i),
                0x00 => {
                    let tc = (u16::from_be_bytes([es[i + 4], es[i + 5]]) >> 3) as usize & 7;
                    if (1..=3).contains(&tc) {
                        pic_at.push(i);
                    }
                }
                _ => {}
            }
        }
    }
    let mut per_gop = vec![0usize; gop_at.len()];
    let mut outside = 0usize;
    for &p in &pic_at {
        match gop_at.partition_point(|&g| g <= p) {
            0 => outside += 1,
            k => per_gop[k - 1] += 1,
        }
    }
    (gop_at.len(), per_gop, outside)
}

struct Row {
    video_es_sha1: String,
    audio_es_sha1: String,
    gops: usize,
    gop_pictures: usize,
    pictures: usize,
    i_pictures: usize,
    p_pictures: usize,
    b_pictures: usize,
    pics_outside_gops: usize,
    pics_per_gop_min: usize,
    pics_per_gop_max: usize,
    audio_kind: u32,
    audio_rate: u32,
    audio_channels: u32,
    audio_samples: usize,
    delta_milliframes: i64,
    frames_sha1: String,
}

fn golden_row(movie: &str, golden: &str) -> Option<Row> {
    let line = golden.lines().find(|l| l.starts_with(movie))?;
    let c: Vec<&str> = line.split('\t').collect();
    Some(Row {
        video_es_sha1: c[1].into(),
        audio_es_sha1: c[2].into(),
        gops: c[3].parse().unwrap(),
        gop_pictures: c[4].parse().unwrap(),
        pictures: c[5].parse().unwrap(),
        i_pictures: c[6].parse().unwrap(),
        p_pictures: c[7].parse().unwrap(),
        b_pictures: c[8].parse().unwrap(),
        pics_outside_gops: c[9].parse().unwrap(),
        pics_per_gop_min: c[10].parse().unwrap(),
        pics_per_gop_max: c[11].parse().unwrap(),
        audio_kind: c[12].parse().unwrap(),
        audio_rate: c[13].parse().unwrap(),
        audio_channels: c[14].parse().unwrap(),
        audio_samples: c[15].parse().unwrap(),
        delta_milliframes: c[16].parse().unwrap(),
        frames_sha1: c[17].into(),
    })
}

/// Renders one movie's row in the golden's column order. `frames_sha1` is "-" here (the decode test pins it).
fn actual_row(path: &str, data: &[u8]) -> (String, Row) {
    let movie = path.rsplit('/').next().unwrap();
    let d = demux(data).unwrap_or_else(|e| panic!("{path}: demux: {e}"));
    let es = &d.video;
    let (gops, per_gop, outside) = gop_walk(es);
    let (pictures, i_p, p_p, b_p) = picture_counts(es);
    let (kind, rate, ch, samples) = if d.audio.is_empty() {
        (0u32, 0u32, 0u32, 0usize)
    } else {
        let body = d.audio.values().next().unwrap();
        let a = Audio::parse(body).unwrap_or_else(|e| panic!("{path}: SSHd/SSbd: {e}"));
        (a.header.kind, a.header.rate, a.header.channels as u32, a.samples())
    };
    // G-008: audio duration vs video duration within ONE frame (1/30 s) — measured in milli-frames.
    let delta_milliframes = if samples == 0 {
        0
    } else {
        let secs = pictures as f64 / 30.0 - samples as f64 / rate as f64;
        (secs * 30.0 * 1000.0).round() as i64
    };
    let audio_es = d.audio.values().next().map(|v| hex(&nf_formats::sha1::sha1(v))).unwrap_or_else(|| "-".into());
    let row = Row {
        video_es_sha1: hex(&nf_formats::sha1::sha1(es)),
        audio_es_sha1: audio_es,
        gops,
        gop_pictures: per_gop.iter().sum(),
        pictures,
        i_pictures: i_p,
        p_pictures: p_p,
        b_pictures: b_p,
        pics_outside_gops: outside,
        pics_per_gop_min: per_gop.iter().copied().min().unwrap_or(0),
        pics_per_gop_max: per_gop.iter().copied().max().unwrap_or(0),
        audio_kind: kind,
        audio_rate: rate,
        audio_channels: ch,
        audio_samples: samples,
        delta_milliframes,
        frames_sha1: "-".into(),
    };
    let line = format!(
        "{movie}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        row.video_es_sha1, row.audio_es_sha1, row.gops, row.gop_pictures, row.pictures,
        row.i_pictures, row.p_pictures, row.b_pictures, row.pics_outside_gops,
        row.pics_per_gop_min, row.pics_per_gop_max, row.audio_kind, row.audio_rate,
        row.audio_channels, row.audio_samples, row.delta_milliframes, row.frames_sha1,
    );
    (line, row)
}

/// The recorded audio-long movies (delta in milli-frames, negative = the audio runs longer than the
/// video): the muxer pads the audio bed past the video end on these five (docs/formats/pss.md G-008).
const RECORDED_AUDIO_LONG: &[&str] = &["0730000D.PSS", "07300017.PSS", "073A0048.PSS", "073B0048.PSS", "073F0048.PSS"];

const HEADER: &str = "movie\tvideo_es_sha1\taudio_es_sha1\tgops\tgop_pictures\tpictures\ti_pictures\tp_pictures\tb_pictures\tpics_outside_gops\tpics_per_gop_min\tpics_per_gop_max\taudio_kind\taudio_rate\taudio_channels\taudio_samples\tdelta_milliframes\tframes_sha1";

/// The GOP walk and the duration check for every movie with data. Skips on stub-only images.
#[test]
fn gop_walk_and_durations_match_the_golden() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO/NF_MOVIE_ISO not set"); return };
    let movies = movies_with_data(&iso);
    if movies.is_empty() {
        eprintln!("skip: all 22 movies are stubs on this image (slim on-foot ISO); set NF_MOVIE_ISO to the retail image");
        return;
    }
    eprintln!("movies with data: {}/22", movies.len());
    let golden = include_str!("../golden/pss_movies_g008.tsv");
    // Pass 1: render every row and run the structural checks (no golden needed) - the GOP walk's own
    // invariants and the +-1-frame duration leg.
    let mut rows: Vec<(String, String, Row)> = Vec::new();
    for (p, data) in &movies {
        let (line, row) = actual_row(p, data);
        let name = p.rsplit('/').next().unwrap().to_string();
        // The video picture count vs the GOP walk: every picture sits in exactly one GOP, totals agree.
        assert_eq!(row.gop_pictures, row.pictures, "{name}: GOP-walk pictures vs picture_counts");
        assert_eq!(row.pics_outside_gops, 0, "{name}: pictures outside any GOP");
        if row.audio_samples > 0 {
            assert_eq!(row.audio_kind, 1, "{name}: PCM (the RC2 ADPCM parser rejected this)");
        }
        rows.push((name, line, row));
    }
    let actual = format!("{HEADER}\n{}\n", rows.iter().map(|(_, l, _)| l.clone()).collect::<Vec<_>>().join("\n"));
    if std::env::var_os("PIN_GOLDEN").is_some() { println!("{actual}"); }
    // Pass 2: the golden comparison - every measured number and ES SHA-1 must hold.
    for (name, _, row) in &rows {
        let g = golden_row(name, golden).unwrap_or_else(|| panic!("{name}: no golden row"));
        assert_eq!(row.pictures, g.pictures, "{name}: picture count vs golden");
        assert_eq!(row.i_pictures, g.i_pictures, "{name}: I pictures vs golden");
        assert_eq!(row.p_pictures, g.p_pictures, "{name}: P pictures vs golden");
        assert_eq!(row.b_pictures, g.b_pictures, "{name}: B pictures vs golden");
        assert_eq!(row.gops, g.gops, "{name}: GOP count vs golden");
        assert_eq!(row.gop_pictures, g.gop_pictures, "{name}: GOP-walk total vs golden");
        assert_eq!(row.pics_outside_gops, g.pics_outside_gops, "{name}: outside-GOP pictures vs golden");
        assert_eq!((row.pics_per_gop_min, row.pics_per_gop_max), (g.pics_per_gop_min, g.pics_per_gop_max), "{name}: per-GOP min/max vs golden");
        assert_eq!((row.audio_kind, row.audio_rate, row.audio_channels), (g.audio_kind, g.audio_rate, g.audio_channels), "{name}: audio format vs golden");
        assert_eq!(row.audio_samples, g.audio_samples, "{name}: audio samples vs golden");
        // The duration leg, tightened to +-1 frame (1000 milli-frames). Five movies carry an audio bed
        // padded past the video end by 1.2-3.0 frames - a property of the FILE, not a decoder error: the
        // sample counts are structural (SSbd body / (2*channels)), reproduced byte-identically by the
        // earlier retail-image run (pss_movies.tsv), and ffprobe independently confirms the video frame
        // count (0730000D: 1700 frames at 30 fps). They are recorded here and in docs/formats/pss.md;
        // any OTHER file drifting past a frame fails.
        if !RECORDED_AUDIO_LONG.contains(&name.as_str()) {
            assert!(row.delta_milliframes.abs() <= 1000, "{name}: audio vs video duration drifts {} milli-frames (> 1 frame)", row.delta_milliframes);
        }
        assert_eq!(row.delta_milliframes, g.delta_milliframes, "{name}: duration delta vs golden");
        assert_eq!(row.video_es_sha1, g.video_es_sha1, "{name}: video ES SHA-1 vs golden");
        assert_eq!(row.audio_es_sha1, g.audio_es_sha1, "{name}: audio ES SHA-1 vs golden");
    }
    // The frames_sha1 column belongs to the decode test (feature `movies`); this test's rows carry "-" there.
    let strip_last = |t: &str| t.lines().map(|l| l.split('\t').take(17).collect::<Vec<_>>().join("\t")).collect::<Vec<_>>().join("\n");
    let d = diff_tsv(&strip_last(&golden), &strip_last(&actual));
    assert!(d.is_empty(), "{} g008 differences:\n{}", d.len(), d.join("\n"));
}

/// Sister (on the video ES itself — the walk's own input): a tampered GOP start code moves that GOP's
/// pictures outside the walk, and the picture total is unchanged (the walk must see the tamper, not lose
/// pictures). Tampering the ES avoids PES packet-boundary gaps in the pack layer.
#[test]
fn gop_walk_sees_a_tampered_gop_start_code() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO/NF_MOVIE_ISO not set"); return };
    let movies = movies_with_data(&iso);
    let Some((p, data)) = movies.first() else { eprintln!("skip: no movie data on this image"); return };
    let name = p.rsplit('/').next().unwrap();
    let d = demux(data).unwrap();
    let (gops, per_gop, outside) = gop_walk(&d.video);
    assert!(gops > 0 && outside == 0, "{name}: clean baseline");
    // 0xB8 → 0xB9 (the sequence-end code): the GOP header disappears, its pictures go outside.
    let mut bad = d.video.clone();
    let at = bad.windows(4).position(|w| w == [0, 0, 1, 0xb8]).unwrap();
    bad[at + 3] = 0xb9;
    let (gops2, per_gop2, outside2) = gop_walk(&bad);
    assert_eq!(gops2, gops - 1, "{name}: sister — one GOP start code gone");
    assert!(outside2 > 0, "{name}: sister — the orphaned pictures land outside the walk");
    assert_eq!(per_gop2.iter().sum::<usize>() + outside2, per_gop.iter().sum::<usize>(), "{name}: sister — the picture total is unchanged");
}

/// Sister (on the video ES): a tampered picture coding type drops the picture count by exactly one, and
/// the GOP walk tracks the scan (the two counters must move in lockstep).
#[test]
fn picture_counts_see_a_tampered_coding_type() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO/NF_MOVIE_ISO not set"); return };
    let movies = movies_with_data(&iso);
    let Some((p, data)) = movies.first() else { eprintln!("skip: no movie data on this image"); return };
    let name = p.rsplit('/').next().unwrap();
    let d = demux(data).unwrap();
    let (pictures, ..) = picture_counts(&d.video);
    let mut bad = d.video.clone();
    // The first picture header: coding type bits are byte +5 bits 3..5 → set type 4 (not I/P/B).
    let at = bad.windows(4).position(|w| w == [0, 0, 1, 0x00]).unwrap();
    bad[at + 5] = (bad[at + 5] & 0xc7) | (4 << 3);
    let (pictures2, ..) = picture_counts(&bad);
    assert_eq!(pictures2, pictures - 1, "{name}: sister — one picture header left the count");
    // The walk must track the scan on the tampered stream too (both counters see the same headers).
    let (_, per_gop2, outside2) = gop_walk(&bad);
    assert_eq!(per_gop2.iter().sum::<usize>() + outside2, pictures2, "{name}: sister — the walk tracks the scan");
}

/// G-008: every picture of every movie decodes without error (feature `movies`), the decoded frame count
/// equals the golden picture count, every frame is 512x448 (the measured constant - mpeg2.rs provenance:
/// Main Profile @ Main Level, the game decodes on the IPU into a 512x448 buffer), and the packed YUV of
/// ALL frames hashes to the golden `frames_sha1` (pinned from the retail-image run). First run: the
/// golden column holds "-" and the hashes are printed for pinning (PIN_GOLDEN=1).
#[cfg(feature = "movies")]
#[test]
fn every_picture_decodes_without_error() {
    use nf_formats::mpeg2::{decode_video, pack_yuv420};
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO/NF_MOVIE_ISO not set"); return };
    let movies = movies_with_data(&iso);
    if movies.is_empty() { eprintln!("skip: all 22 movies are stubs on this image (slim on-foot ISO)"); return; }
    eprintln!("movies with data: {}/22", movies.len());
    let golden = include_str!("../golden/pss_movies_g008.tsv");
    for (p, data) in &movies {
        let name = p.rsplit('/').next().unwrap();
        // PIN_ONLY=07100001.PSS,073F0048.PSS restricts the (slow) decode to the named movies - the
        // re-pinning helper for a partial run.
        if let Ok(only) = std::env::var("PIN_ONLY") {
            if !only.split(',').any(|m| m == name) { continue; }
        }
        let g = golden_row(name, golden).unwrap_or_else(|| panic!("{name}: no golden row"));
        let d = demux(data).unwrap();
        let frames = decode_video(&d.video).unwrap_or_else(|e| panic!("{name}: DECODE FAILED: {e}"));
        assert_eq!(frames.len(), g.pictures, "{name}: decoded frames vs the golden picture count");
        let mut h = Sha1::new();
        for f in &frames {
            assert_eq!((f.width, f.height), (512, 448), "{name}: frame geometry (the measured constant)");
            h.update(&pack_yuv420(f));
        }
        let sha = hex(&h.finish());
        if g.frames_sha1 == "-" {
            println!("{name}\t{sha}");
        } else {
            assert_eq!(sha, g.frames_sha1, "{name}: all-frames YUV hash vs golden");
        }
    }
}

/// Sister (feature `movies`, first movie only): one flipped byte inside the video ES must change the
/// all-frames YUV hash (or fail the decode) - the golden hash catches single-byte content tampering.
#[cfg(feature = "movies")]
#[test]
fn a_tampered_es_byte_changes_the_decoded_frames() {
    use nf_formats::mpeg2::{decode_video, pack_yuv420};
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO/NF_MOVIE_ISO not set"); return };
    let movies = movies_with_data(&iso);
    let Some((p, data)) = movies.first() else { eprintln!("skip: no movie data on this image"); return };
    let name = p.rsplit('/').next().unwrap();
    let d = demux(data).unwrap();
    let frames = decode_video(&d.video).unwrap_or_else(|e| panic!("{name}: baseline decode: {e}"));
    let mut h = Sha1::new();
    for f in &frames { h.update(&pack_yuv420(f)); }
    let clean = hex(&h.finish());
    // Flip a byte well past the headers (the middle of the ES: a slice payload of some later picture).
    let mut bad = d.video.clone();
    let at = bad.len() / 2;
    bad[at] ^= 0x40;
    if let Ok(bad_frames) = decode_video(&bad) {
        let mut h = Sha1::new();
        for f in &bad_frames { h.update(&pack_yuv420(f)); }
        assert_ne!(hex(&h.finish()), clean, "{name}: sister - a flipped ES byte must change the decode");
    }
}
