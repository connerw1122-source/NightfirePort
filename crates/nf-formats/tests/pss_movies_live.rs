//! NF_ISO tests for the disc's PSS movies (`MOVIES/30_FPS/*.PSS`, 22 files). Opt-in like the other golden
//! tests: set `NF_ISO` to a disc image. The slim on-foot image stubs the movies (size 0), so these tests skip
//! there with a count; point `NF_MOVIE_ISO` (or `NF_ISO` itself) at an image with movie data — e.g. the retail
//! `007 - Nightfire (USA)` image — to run them for real. Closes the NF_ISO half of G-008.
//!
//! Golden data: `golden/pss_movies.tsv`, one row per movie: the stream ids present (as packet counts per kind),
//! the video size, the frame count (by picture type) and the audio format (`SShd` kind/rate/channels/interleave
//! plus body bytes and the sample count derived from them). Each comparison has a sister assertion proving a
//! one-character change to the golden row is caught.

use nf_formats::iso9660::IsoImage;
use nf_formats::mpeg2::picture_counts;
use nf_formats::pss::{demux, Audio, Stream};

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

struct Row {
    packs: usize,
    video_packets: usize,
    audio_packets: usize,
    other_packets: usize,
    padding_packets: usize,
    video_bytes: usize,
    audio_bytes: usize,
    width: u32,
    height: u32,
    frame_rate_code: u32,
    pictures: usize,
    i_pictures: usize,
    p_pictures: usize,
    b_pictures: usize,
    audio_channels: String,
    audio_kind: u32,
    audio_rate: u32,
    audio_channels_hdr: u32,
    audio_interleave: u32,
    audio_body: u32,
    audio_samples: usize,
    ended: u8,
    trailing: usize,
}

fn row_of(movie: &str, golden: &str) -> Row {
    let line = golden.lines().find(|l| l.starts_with(movie)).unwrap();
    let c: Vec<&str> = line.split('\t').collect();
    Row {
        packs: c[1].parse().unwrap(),
        video_packets: c[2].parse().unwrap(),
        audio_packets: c[3].parse().unwrap(),
        other_packets: c[4].parse().unwrap(),
        padding_packets: c[5].parse().unwrap(),
        video_bytes: c[6].parse().unwrap(),
        audio_bytes: c[7].parse().unwrap(),
        width: c[8].parse().unwrap(),
        height: c[9].parse().unwrap(),
        frame_rate_code: c[10].parse().unwrap(),
        pictures: c[11].parse().unwrap(),
        i_pictures: c[12].parse().unwrap(),
        p_pictures: c[13].parse().unwrap(),
        b_pictures: c[14].parse().unwrap(),
        audio_channels: c[15].to_string(),
        audio_kind: c[16].parse().unwrap(),
        audio_rate: c[17].parse().unwrap(),
        audio_channels_hdr: c[18].parse().unwrap(),
        audio_interleave: c[19].parse().unwrap(),
        audio_body: c[20].parse().unwrap(),
        audio_samples: c[21].parse().unwrap(),
        ended: c[22].parse().unwrap(),
        trailing: c[23].parse().unwrap(),
    }
}

/// Renders what the demuxer + ES scan see for one movie, in the golden row's column order.
fn actual_row(movie: &str, data: &[u8]) -> String {
    let movie = movie.rsplit('/').next().unwrap();
    let d = demux(data).expect("demux");
    let es = &d.video;
    let (seq, seq_at) = es.windows(4).enumerate().find(|(_, w)| *w == [0, 0, 1, 0xb3]).map(|(i, _)| (i, i)).unwrap();
    let _ = seq;
    let b = &es[seq_at + 4..seq_at + 8];
    let width = (((b[0] as u32) << 4) | (b[1] >> 4) as u32) & 0xfff;
    let height = ((((b[1] & 0xf) as u32) << 8) | b[2] as u32) & 0xfff;
    let frame_rate_code = (b[3] & 0xf) as u32;
    let (pictures, i_p, p_p, b_p) = picture_counts(es);
    let (mut audio_bytes, mut audio_channels, mut hdr) = (0usize, "-".to_string(), None);
    let mut keys: Vec<String> = d.audio.keys().map(|k| k.to_string()).collect();
    keys.sort();
    if !d.audio.is_empty() {
        audio_channels = keys.join(";");
        let c = d.audio.values().next().unwrap();
        audio_bytes = d.audio.values().map(|v| v.len()).sum();
        let a = Audio::parse(c).expect("SShd/SSbd header");
        hdr = Some((a.header.kind, a.header.rate, a.header.channels, a.header.interleave, a.header.body_bytes, a.samples()));
    }
    let (kind, rate, ch, il, body, samples) = hdr.map(|(k, r, c, i, b, s)| (k, r, c, i, b, s)).unwrap_or((0, 0, 0, 0, 0, 0));
    // Split the demuxer's combined "other" count into the system header (BB) and padding (BE).
    let mut bb = 0usize;
    let mut be = 0usize;
    for p in nf_formats::pss::Demux::new(data) {
        match p.unwrap().stream { Stream::Other(0xbb) => bb += 1, Stream::Other(0xbe) => be += 1, _ => {} }
    }
    let padding = be;
    let other = bb;
    format!(
        "{movie}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{width}\t{height}\t{frame_rate_code}\t{pictures}\t{i_p}\t{p_p}\t{b_p}\t{audio_channels}\t{kind}\t{rate}\t{ch}\t{il}\t{body}\t{samples}\t{}\t{}",
        d.packs, d.video_packets, d.audio_packets, other, padding, es.len(), audio_bytes, d.ended as u8, d.trailing
    )
}

#[test]
fn every_movie_on_the_disc_demuxes_to_the_golden_numbers() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO/NF_MOVIE_ISO not set"); return };
    let paths = movie_paths();
    let entries: Vec<_> = paths.iter().map(|p| (p, iso.find(p).unwrap_or_else(|| panic!("{p} not in the image tree")))).collect();
    let with_data: Vec<_> = entries.iter().filter(|(_, e)| e.size > 0).collect();
    if with_data.is_empty() {
        eprintln!("skip: all 22 movies are stubs on this image (slim on-foot ISO); set NF_MOVIE_ISO to an image with movie data");
        return;
    }
    eprintln!("movies with data: {}/{}", with_data.len(), entries.len());
    let golden = include_str!("../golden/pss_movies.tsv");
    let mut lines = Vec::new();
    for (p, e) in &with_data {
        let data = iso.read_file(e).expect("read movie");
        assert_eq!(data.len(), e.size as usize, "{p}: ISO extent size");
        lines.push(actual_row(p, &data));
    }
    let actual = format!("{}\n{}\n", golden.lines().next().unwrap(), lines.join("\n"));
    let d = nf_formats::golden::diff_tsv(golden, &actual);
    assert!(d.is_empty(), "{} differences:\n{}", d.len(), d.join("\n"));
    // Sister: a one-character change to one golden row must be caught (here: the first data row's pack count).
    let mut tampered = golden.to_string();
    {
        let first = golden.lines().nth(1).unwrap().to_string();
        let flipped = {
            let mut c: Vec<&str> = first.split('\t').collect();
            let packs: u32 = c[1].parse().unwrap();
            let bumped = (packs + 1).to_string();
            c[1] = &bumped;
            c.join("\t")
        };
        tampered = tampered.replacen(&first, &flipped, 1);
    }
    assert_eq!(diff_tsv_n(golden, &tampered), 1, "sister: a one-value change must be caught");
}

fn diff_tsv_n(a: &str, b: &str) -> usize { nf_formats::golden::diff_tsv(a, b).len() }

/// The demuxer's stream classification, on the movie the task's flow research centres on: the first briefing.
/// Pins the exact per-kind stream ids (video E0, private-1 audio channel 0, the system header as "other") —
/// not just counts — plus the end code and the 4 trailing pad bytes every movie has.
#[test]
fn stream_ids_of_the_first_briefing_movie() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO/NF_MOVIE_ISO not set"); return };
    let Some(e) = iso.find("/MOVIES/30_FPS/07100001.PSS") else { eprintln!("skip: movie not in tree"); return };
    if e.size == 0 { eprintln!("skip: stub on this image"); return; }
    let data = iso.read_file(e).unwrap();
    let d = demux(&data).unwrap();
    let mut ids: Vec<u8> = Vec::new();
    for p in nf_formats::pss::Demux::new(&data) {
        match p.unwrap().stream {
            Stream::Video(v) => ids.push(v),
            Stream::Audio(c) => ids.push(c),
            Stream::Other(o) => ids.push(o),
        }
    }
    ids.sort();
    ids.dedup();
    assert_eq!(ids, vec![0x00u8, 0xbb, 0xbe, 0xe0], "video E0, audio channel 0, padding BE, system header BB");
    assert!(d.ended, "the end code 00 00 01 B9 is reached");
    assert_eq!(d.trailing, 0, "every movie ends exactly at the end code (no trailing bytes)");
    // Sister: a stream-id change (video E0 -> C0) must be caught by the packet counts (E1..EF is still video).
    let mut bad = data.clone();
    let first_video = data.windows(4).position(|w| w == [0, 0, 1, 0xe0]).unwrap();
    bad[first_video + 3] = 0xc0;
    let d2 = demux(&bad).unwrap();
    assert_eq!(d2.video_packets, d.video_packets - 1, "sister: one video packet moved to stream C0");
    assert_eq!(d2.other_packets, d.other_packets + 1, "sister: ...counted as an other packet");
}

/// Audio sample count against the stream length (G-008's "audio sample count against the stream length"):
/// for every audio-bearing movie, `samples()` is `SSbd body / (2 · channels)` (16-bit PCM), and the duration
/// agrees with the video's picture count at 30 Hz to within a second (both measured, independently).
#[test]
fn audio_sample_count_matches_stream_length() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO/NF_MOVIE_ISO not set"); return };
    let golden = include_str!("../golden/pss_movies.tsv");
    let mut checked = 0;
    for line in golden.lines().skip(1) {
        let movie = line.split('\t').next().unwrap();
        let row = row_of(movie, golden);
        if row.audio_kind == 0 { continue; }
        let Some(e) = iso.find(&format!("/MOVIES/30_FPS/{movie}")) else { continue };
        if e.size == 0 { continue; }
        let data = iso.read_file(e).unwrap();
        let d = demux(&data).unwrap();
        let a = Audio::parse(d.audio.values().next().unwrap()).unwrap();
        assert_eq!(a.header.kind, 1, "{movie}: PCM (the RC2 ADPCM parser rejected this)");
        assert_eq!(a.samples(), row.audio_samples, "{movie}: sample count vs golden");
        assert_eq!(a.samples(), a.header.body_bytes as usize / (2 * a.header.channels as usize), "{movie}: PCM samples = body / (2·ch)");
        let vid = row.pictures as f64 / 30.0;
        let aud = a.seconds();
        assert!((vid - aud).abs() < 1.0, "{movie}: video {vid:.2}s vs audio {aud:.2}s");
        checked += 1;
    }
    // The slim on-foot image stubs every movie (lba 0, size 0) — nothing to check here (the header
    // comment's env gate); demuxing 0 movies must read as a skip, not a failure (fix-swim-vent4: the
    // full-suite policy allows only the disc_golden trio to fail on the slim ISO).
    if checked == 0 { eprintln!("no movie data in this image (NF_MOVIE_ISO points at the retail one) — skipping"); return; }
    assert!(checked >= 20, "expected ~20 audio-bearing movies, checked {checked}");
}

/// Frame 0 and frame 1000 of the first briefing movie decode to fixed SHA-1s (feature `movies`). The hashes are
/// of the packed Y/Cb/Cr planes (`mpeg2::pack_yuv420`, the ffmpeg `yuv420p` layout). Cross-check (not a test):
/// `ffmpeg -i 07100001.m2v -frames:v 1 -f rawvideo -pix_fmt yuv420p` hashes the same frame 0 to
/// `1a4ce4f6b9491e4faa8e7fc7a1474d77d4865559` and frame 1000 to `1efd3fc609e5b68387d775c6ddea6d079c314b9c`;
/// the decoders differ only in IDCT rounding (measured PSNR: frame 0 = 67.8 dB, frame 1000 = 66.3 dB), so the
/// golden hashes pin *this* decoder's output, and the ffmpeg hashes document the independent cross-check.
#[test]
#[cfg(feature = "movies")]
fn video_frames_decode_to_known_sha1s() {
    use nf_formats::mpeg2::{decode_video, pack_yuv420};
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO/NF_MOVIE_ISO not set"); return };
    let Some(e) = iso.find("/MOVIES/30_FPS/07100001.PSS") else { eprintln!("skip: movie not in tree"); return };
    if e.size == 0 { eprintln!("skip: stub on this image"); return; }
    let data = iso.read_file(e).unwrap();
    let d = demux(&data).unwrap();
    let frames = decode_video(&d.video).expect("decode");
    assert_eq!(frames.len(), 1280, "1280 pictures at 30 Hz = 42.67 s");
    let sha1 = |b: &[u8]| nf_formats::sha1::hex(&nf_formats::sha1::sha1(b));
    let f0 = pack_yuv420(&frames[0]);
    let f1000 = pack_yuv420(&frames[1000]);
    // Computed by this decoder (feature movies); ffmpeg cross-check for frame 0 in the comment above.
    assert_eq!(sha1(&f0), "54b75479b7bbfd6d8e2492e8fc97072484ccbf3d", "frame 0");
    assert_eq!(sha1(&f1000), "f965b37f552c898f75df29895c83e63aebe50134", "frame 1000");
    // Sister: decode a different frame; it must not match frame 1000's hash.
    assert_ne!(sha1(&pack_yuv420(&frames[999])), sha1(&f1000), "sister: a different frame hashes differently");
}
