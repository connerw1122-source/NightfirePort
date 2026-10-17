//! The movie player never runs faster than real time (playtest 1015-2142: "the opening movie ... has
//! the 007 white dots, those go by in a microsecond"). The decoder warms up behind the clock; the old
//! catch-up then showed one movie frame per RENDER frame (100+ fps on a fast display). Presented at a
//! fast render rate, the picture must change at most at the game's 30 fps (main 0x00109080 decodes on
//! every second 60 Hz vblank [M]).
//!
//! Needs a retail image with movie data: `NF_MOVIE_ISO` (falls back to `NF_ISO`); silent (NF_SILENT).

#[cfg(feature = "movies")]
#[test]
fn the_intro_never_plays_faster_than_30_fps() {
    // SAFETY: set before any audio thread starts.
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Some(path) = std::env::var_os("NF_MOVIE_ISO").or_else(|| std::env::var_os("NF_ISO")) else {
        eprintln!("skip: no NF_MOVIE_ISO / NF_ISO"); return;
    };
    // 0x07370048 is the first intro movie (the 007 logo) [the intro pair, menu::INTRO_MOVIES].
    let code = nf_view::menu::INTRO_MOVIES[0];
    let mut p = match nf_view::movie::Player::start(path.to_string_lossy().as_ref(), code, 100) {
        Ok(p) => p,
        Err(e) => { eprintln!("skip: {code:08x} not startable here: {e:?}"); return; }
    };
    // Wait for the first picture (the decoder's warm-up), presenting fast the whole time.
    let t0 = std::time::Instant::now();
    let mut last: Option<u64> = None;
    while last.is_none() && t0.elapsed().as_secs() < 60 {
        last = p.present().map(|(_, _, rgba)| hash(rgba));
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
    let Some(mut prev) = last else { panic!("the intro decoded no frame in 60 s") };
    // Two seconds at a ~250 Hz render rate: count picture changes.
    let start = std::time::Instant::now();
    let mut changes = 0u32;
    while start.elapsed().as_secs_f64() < 2.0 {
        if let Some((_, _, rgba)) = p.present() {
            let h = hash(rgba);
            if h != prev { changes += 1; prev = h; }
        }
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
    let secs = start.elapsed().as_secs_f64();
    eprintln!("[pace] {changes} picture changes in {secs:.2} s at ~250 presents/s");
    assert!(changes as f64 <= secs * 30.0 + 2.0, "at most 30 frames a second: {changes} in {secs:.2} s");
    // Sister: the movie really plays (a frozen picture would pass the cap trivially).
    assert!(changes >= 10, "the picture advances: {changes} changes in {secs:.2} s");
}

#[cfg(feature = "movies")]
fn hash(b: &[u8]) -> u64 {
    // FNV-1a over a sparse sample: cheap per present, changes with the picture.
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &x in b.iter().step_by(97) { h ^= x as u64; h = h.wrapping_mul(0x0100_0000_01b3); }
    h
}
