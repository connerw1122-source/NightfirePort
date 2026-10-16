//! boot-flow: the movie worker decodes GOP by GOP (open GOPs with the previous GOP's anchors in front), so
//! the first picture of a long movie arrives at once. 0x073B0048 (the P_ESTHERO intro after START) used to
//! sit black ~20 s: its first closed-GOP chunk held most of the movie and decoded whole before any frame.
//! The split must not change a single pixel: the streamed frames equal `movie_still`'s reference decode
//! (whole closed-GOP chunks) at the same picture numbers, across open-GOP boundaries.
//! Needs a retail image with movie data (`NF_ISO`); silent.

#[cfg(feature = "movies")]
#[test]
fn the_intro_streams_from_the_start_and_matches_the_reference_decode() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Some(path) = std::env::var_os("NF_ISO") else { return };
    let path = path.to_string_lossy().to_string();
    let iso = nf_formats::iso9660::IsoImage::open(&path).expect("iso");
    let Ok(m) = nf_view::movie::Movie::load(&iso, "073B0048") else { eprintln!("skip: no movie data"); return };
    let t0 = std::time::Instant::now();
    let mut s = nf_view::movie::MovieStream::start(&m, false).expect("stream");
    let mut frames = Vec::new();
    let mut first_at = None;
    while frames.len() < 80 && t0.elapsed().as_secs() < 60 {
        if s.advance() {
            if first_at.is_none() { first_at = Some(t0.elapsed().as_secs_f32()); }
            frames.push(nf_formats::mpeg2::yuv420_to_rgb8(s.frame().unwrap()));
        } else {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    let first_at = first_at.expect("no frame at all");
    assert!(first_at < 3.0, "the first picture took {first_at:.1} s (the old whole-chunk decode took ~20 s)");
    assert_eq!(frames.len(), 80, "80 frames streamed");
    // Pictures 10 (first GOP, closed), 40 and 70 (later, open GOPs decoded with a stripped prefix).
    for n in [10usize, 40, 70] {
        let r = nf_view::movie::movie_still(&path, 0x073b_0048, n).expect("reference still");
        assert!(frames[n] == r.rgba, "picture {n}: the streamed frame differs from the reference decode");
    }
    // Sister: neighbouring pictures differ (the comparison can fail).
    assert!(frames[40] != frames[41] || frames[40] != frames[70], "pictures should differ");
}
