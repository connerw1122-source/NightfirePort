//! crash-fix (playtest 1016: "at the end of a starting level cutscene, it froze then crashed"): the
//! briefing starts during the map-load fade (decode-ahead) and nobody pulls frames until state 6. The
//! worker's channel was unbounded, so it decoded the whole movie into RAM ahead of playback. The worker
//! must stop STREAM_AHEAD pictures ahead, and stopping it while it waits on a full channel must not
//! deadlock. Needs a retail image with movie data (`NF_ISO`); silent.

#[cfg(feature = "movies")]
#[test]
fn an_unread_stream_stays_bounded_and_stops_promptly() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Some(path) = std::env::var_os("NF_ISO") else { return };
    let iso = nf_formats::iso9660::IsoImage::open(path.to_string_lossy().as_ref()).expect("iso");
    let Ok(m) = nf_view::movie::Movie::load(&iso, "07100005") else { eprintln!("skip: no movie data"); return };
    let mut s = nf_view::movie::MovieStream::start(&m, false).expect("stream");
    // Let the worker run unread for a while (the decode-ahead window), then count what it queued.
    std::thread::sleep(std::time::Duration::from_secs(8));
    let mut queued = 0usize;
    while s.advance() { queued += 1; }
    // While this loop drains, the worker refills from the GOP it already decoded (one GOP's pictures,
    // 12-15 in these movies), so the bound is STREAM_AHEAD plus a GOP, not thousands.
    eprintln!("queued unread: {queued}");
    assert!(queued <= nf_view::movie::STREAM_AHEAD + 30,
        "{queued} pictures queued unread; the worker must stop {} ahead", nf_view::movie::STREAM_AHEAD);
    // Sister: the worker did decode ahead (the bound is what stopped it, not a dead worker).
    assert!(queued >= nf_view::movie::STREAM_AHEAD / 2, "only {queued} pictures decoded in 8 s");
    // Refill, then stop with the worker blocked on a full channel.
    std::thread::sleep(std::time::Duration::from_secs(4));
    let t = std::time::Instant::now();
    s.stop();
    assert!(t.elapsed().as_secs_f32() < 5.0, "stop took {:.1} s", t.elapsed().as_secs_f32());
}
