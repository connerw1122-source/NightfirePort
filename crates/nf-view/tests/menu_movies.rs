//! fix-menu-movies tests: the front end's background movies and the intro.
//!
//! Evidence: the menu-def's PAGE records (kind-8 entry 08000002 in 07000048.bin; PAGE −0x10
//! {u32 page id, u32 file hash, u32 aux movie, u8 kind, u32 size}, walked by `MenuManager_Create`
//! 0x21b908 which stores the aux code at M_PAGE+0xC4); `Manager_SendMessage` 0x21c858:338 plays the
//! page's aux code as a LOOPING background movie on page change (`Menu_PlayMovie(aux, 0, 1, 0, 0, 0)`
//! — loop = 1, and `psiStopBackgroundMovie` when the new page has none); `P_INTRO_Handler` 0x209e88
//! plays 0x07370048 then 0x07380048 (loop = 0) between the title and the front end. The movies load
//! through `psiStartBackgroundMovie` 0x001081a8 (`cdrom0:\movies\30_FPS\%8.8x.pss;1`).
//!
//! The movie-data tests need an image with real PSS data (the slim on-foot ISO stubs every movie
//! [L]); they skip unless NF_MOVIE_ISO/NF_ISO points at one.

use nf_formats::iso9660::IsoImage;
use nf_formats::levelbin::LevelBin;

fn iso() -> Option<IsoImage> {
    let path = std::env::var_os("NF_MOVIE_ISO").or_else(|| std::env::var_os("NF_ISO"))?;
    Some(IsoImage::open(path).unwrap())
}

/// The image the menu-def comes from: any image with FILES.BIN (the slim on-foot ISO has it; only
/// its PSS files are stubbed [L]).
fn def_iso() -> Option<IsoImage> {
    let path = std::env::var_os("NF_ISO")?;
    Some(IsoImage::open(path).unwrap())
}

/// The menu-def's raw bytes: the kind-8 entry `08000002` inside FILES.BIN's `07000048.bin`.
fn menu_def(iso: &IsoImage) -> Vec<u8> {
    let (entries, bin) = nf_formats::disc::files_bin_entries(iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000048.BIN")).expect("07000048.bin in FILES.BIN");
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let lb = LevelBin::parse(&data).unwrap();
    let e = lb.entries.iter().find(|e| e.kind == 8 && e.name == "08000002").expect("the 08000002 menu-def entry");
    lb.data(&data, e).to_vec()
}

/// Walks the menu-def's tag stream exactly like `MenuManager_Create` 0x21b908 and returns every
/// PAGE record as (page id, file hash, aux movie, kind). Per-record sizes follow the decompile's
/// reads; PAGE bodies are skipped by their size field (the decompile's own skip for non-matching
/// files, `puStack_d0 += iVar8` — the body walk and the skip land on the same next tag).
fn page_records(def: &[u8]) -> Vec<(u32, u32, u32, u8)> {
    let mut p = 0usize;
    let rd = |n: usize, p: &mut usize| -> u32 {
        let mut v = 0u32;
        for k in 0..n { v |= (def[*p + k] as u32) << (8 * k); }
        *p += n;
        v
    };
    let _version = rd(4, &mut p);            // the prologue's discarded leading dword [M 0x21b908]
    let mut out = Vec::new();
    loop {
        let at = p;
        let tag = rd(4, &mut p);
        match tag {
            // PAGE −0x10: {id, file hash, aux movie, u8 kind, u32 size}; the size spans the record's
            // body after the size field.
            0xfffffff0 => {
                let id = rd(4, &mut p);
                let hash = rd(4, &mut p);
                let aux = rd(4, &mut p);
                let kind = rd(1, &mut p) as u8;
                let size = rd(4, &mut p) as usize;
                out.push((id, hash, aux, kind));
                p += size;
            }
            // CONTROL −0x07 {u32 name, u8, u8 type, u16 x, y, w, h, f1, u32 d1, u16 f2, u32 d2, u32 d3}.
            0xfffffff9 => { p += 30; }
            // NAME −0x08: a 32-byte string [M `puStack_d0 + 0x20`].
            0xfffffff8 => { p += 0x20; }
            // SCRIPT −0x09 {u32, u32}.
            0xfffffff7 => { p += 8; }
            // MSG −0x0a {u8 msg, u32 a, u32 b}.
            0xfffffff6 => { p += 9; }
            // KFRAME −0x0b: 5×u16 + u32.
            0xfffffff5 => { p += 14; }
            // KMSG −0x0c {u32 time, u8, u32, u32}.
            0xfffffff4 => { p += 13; }
            // MGRMSG −0x0d {u8 msg, u32 a, u32 b}.
            0xfffffff3 => { p += 9; }
            // SPRITE −0x05 {8×u16, u32, u32, f32, f32}.
            0xfffffffb => { p += 32; }
            // COMPONENT −0x04 {u16 count, u32 texture hash, 4×u16}.
            0xfffffffc => { p += 14; }
            // SET −0x03 (u16).
            0xfffffffd => { p += 2; }
            // SKIP −0x02 (u32).
            0xfffffffe => { p += 4; }
            // END −0x0e ends the stream [M the switch's exit].
            0xfffffff2 => break,
            other => panic!("unexpected tag {other:#x} at {at:#x} (the walk must match 0x21b908)"),
        }
    }
    out
}

/// The PAGE records' aux movie codes ARE the page → movie mapping [M 0x21b908:169-183 stores the
/// third dword at M_PAGE+0xC4; 0x21c858:338 plays it]. This test prints the disc's own map and pins
/// the port's `menu::aux_movie_for` to it.
#[test]
fn the_page_aux_movie_map_comes_from_the_page_data() {
    let Some(iso) = def_iso() else { eprintln!("skip: NF_ISO not set"); return };
    let def = menu_def(&iso);
    let recs = page_records(&def);
    eprintln!("--- PAGE records (id, file hash, aux movie, kind):");
    for (id, hash, aux, kind) in &recs {
        eprintln!("  page {id:#010x}  file {hash:#010x}  aux {aux:#010x}  kind {kind}");
    }
    // The game builds pages whose record hash matches the menu-def file's hashcode 0x80000002
    // [M bootup 0x1c7e78 sets GameState+0x20 = 0x80000002; every record printed carries it].
    let mine: Vec<(u32, u32)> = recs.iter().filter(|(_, h, _, _)| *h == 0x8000_0002).map(|(i, _, a, _)| (*i, *a)).collect();
    // The port's mapping must equal the disc's for every page the port drives [M aux_movie_for's
    // contract]: main = 0x07330048, the boot/list/results pages = 0x07350048, the intro and title
    // layout = none (P_INTRO plays its movies from the handler, 0x209e88).
    use nf_view::menu::Page;
    for page in [Page::Start, Page::Main, Page::Codename, Page::Difficulty, Page::MissionSelect, Page::Dossier, Page::Options, Page::Pause, Page::Results] {
        let want = mine.iter().find(|(i, _)| *i == page.id()).map(|(_, a)| *a).unwrap_or(0);
        assert_eq!(nf_view::menu::aux_movie_for(page.id()), want,
            "page {page:?} ({:#010x}): the port's aux movie must be the PAGE record's", page.id());
    }
    // The task's headline mapping, from the data itself:
    assert_eq!(nf_view::menu::aux_movie_for(Page::Main.id()), 0x0733_0048, "the main menu's backdrop is its PAGE record's 0x07330048 [L]");
    assert_eq!(nf_view::menu::aux_movie_for(Page::Start.id()), 0x0735_0048, "the boot page's backdrop is 0x07350048 [L]");
    assert_eq!(nf_view::menu::aux_movie_for(Page::MissionSelect.id()), 0x0735_0048, "the list pages' backdrop is 0x07350048 [L]");
    assert_eq!(nf_view::menu::aux_movie_for(0x4000_0032), 0, "P_INTRO has no aux movie — the handler plays 0x7370048/0x7380048 [M 0x209e88]");
}

/// The menu loop movie decodes with the game's own geometry, and a frame's RGB8 SHA-1 pins the
/// decoder end to end (numbers only — the main agent compares the pixels).
#[cfg(feature = "movies")]
#[test]
fn the_menu_loop_movie_decodes() {
    let Some(iso) = iso() else { eprintln!("skip: NF_MOVIE_ISO/NF_ISO not set"); return };
    let m = match nf_view::movie::Movie::load(&iso, "07330048") { Ok(m) => m, Err(e) => { eprintln!("skip: {e:#}"); return } };
    assert_eq!(m.code, 0x0733_0048);
    assert_eq!((m.width, m.height, m.frame_rate_code), (512, 448, 5), "the game's movie geometry [M movies.md]");
    let (pics, i, p, b) = nf_formats::mpeg2::picture_counts(&m.demuxed.video);
    eprintln!("07330048.PSS: {pics} pictures (I{i} P{p} B{b})");
    assert!(pics > 30, "a menu loop has more than a second of pictures");
    // The decoder pins: frame 12 of the main-menu loop, RGB8 (the yuv420_to_rgb8 RGBA buffer), first
    // 4 SHA-1 bytes + the full digest's low word. A changed decoder, a wrong chunk split or a
    // corrupted PSS moves them.
    let sha = |n: usize| { let f = m.decode_frame(n).unwrap(); nf_formats::sha1::sha1(&nf_formats::mpeg2::yuv420_to_rgb8(&f)) };
    let s12 = sha(12);
    eprintln!("frame 12 sha1 {:02x}{:02x}{:02x}{:02x}", s12[0], s12[1], s12[2], s12[3]);
    assert_eq!(&s12[..4], &[0x21, 0xd0, 0x73, 0x9d], "frame 12 of 07330048 decoded differently than measured");
    // Sister: a different frame hashes differently (the pin is not a constant hash).
    assert_ne!(sha(60)[..4], s12[..4], "frames 12 and 60 hashed the same");
}

/// The streaming decoder: the worker decodes ahead of playback, and 3 s of advance produces
/// distinct real frames at the movie's own rate.
#[cfg(feature = "movies")]
#[test]
fn the_menu_loop_streams_ahead_of_playback() {
    let Some(iso) = iso() else { eprintln!("skip: NF_MOVIE_ISO/NF_ISO not set"); return };
    let m = match nf_view::movie::Movie::load(&iso, "07330048") { Ok(m) => m, Err(e) => { eprintln!("skip: {e:#}"); return } };
    let mut s = nf_view::movie::MovieStream::start(&m, true).unwrap();
    assert_eq!(s.code, 0x0733_0048);
    assert_eq!(s.pictures, 1800, "the loop's picture count [L measured 2026-10-13: I180 P1620]");
    // 90 advance calls = 3 s of 30 fps playback; the worker keeps up (measured ~2× realtime).
    let mut hashes = Vec::new();
    for k in 0..90 {
        for _ in 0..400 { if s.advance() { break; } std::thread::sleep(std::time::Duration::from_millis(10)); }
        let f = s.frame().unwrap_or_else(|| panic!("frame {k} never decoded"));
        hashes.push(nf_formats::sha1::sha1(&nf_formats::mpeg2::yuv420_to_rgb8(f)));
    }
    s.stop();
    let unique: std::collections::HashSet<[u8; 20]> = hashes.iter().copied().collect();
    eprintln!("90 played frames, {} distinct", unique.len());
    assert!(unique.len() > 80, "playback shows {} distinct frames in 3 s — the stream repeats pictures", unique.len());
    // Sister: consecutive frames differ (a frozen decoder shows one picture 90 times).
    assert!(hashes.windows(2).any(|w| w[0] != w[1]), "playback advances frame to frame");
}

/// The P_INTRO run: Start arms it, Confirm (action 0x19's keyboard stand-in) skips it, the
/// countdown pops it — and the intro movies' picture counts pin the headless tick budget.
#[cfg(feature = "movies")]
#[test]
fn the_intro_movie_counts_pin_the_tick_budget() {
    let Some(iso) = iso() else { eprintln!("skip: NF_MOVIE_ISO/NF_ISO not set"); return };
    let mut total = 0usize;
    for code in nf_view::menu::INTRO_MOVIES {
        let m = match nf_view::movie::Movie::load(&iso, &format!("{code:08X}")) { Ok(m) => m, Err(e) => { eprintln!("skip: {e:#}"); return } };
        let n = nf_formats::mpeg2::picture_counts(&m.demuxed.video).0;
        eprintln!("{code:08X}: {n} pictures = {} s at 30 fps", n as f64 / 30.0);
        total += n;
    }
    // play_movie's tick budget = both movies at 30 fps, one frame per two 60 Hz ticks.
    assert_eq!(nf_view::menu::INTRO_TICKS, (total as u32) * 2, "the headless budget = the real movies' length");
}

/// The intro flow (no ISO needed): Start arms the intro run, Confirm skips it (the game's action
/// 0x19 [M GameFlow_Main 0x1c98c8 case 6]), and other input waits.
#[test]
fn the_intro_plays_then_the_main_menu_and_skips_on_confirm() {
    use nf_view::menu::{Action, FrontEnd, Page, ST_MOVIE, ST_RUNNER};
    let mut fe = FrontEnd::new();
    for _ in 0..8 { fe.tick(); }
    assert!(fe.action(Action::Confirm), "Start arms the intro");
    assert!(fe.intro_playing, "the P_INTRO run is up [M 0x209e88]");
    assert_eq!(fe.flow.get_state(), ST_MOVIE);
    // boot-flow: START plays P_ESTHERO's 0x073B0048 [M 0x20a060]; the boot P_INTRO logos run before the title.
    assert_eq!(fe.flow.movie_frames, nf_view::menu::ESTHERO_TICKS, "the budget is the real movie's length");
    assert_eq!(fe.intro_movie_code(), nf_view::menu::ESTHERO_MOVIE, "START plays the 0x073B intro, not the boot logos");
    assert_eq!(fe.page, Page::Main, "the page the movie lands on is already the main menu");
    // Other input does nothing while the movie owns the screen (the intro page has no controls).
    assert!(!fe.action(Action::Down), "the cursor does not move during the intro");
    assert_eq!(fe.cursor, 0);
    // Confirm = action 0x19: the skip, after the 3 s grace.
    for _ in 0..nf_view::menu::ESTHERO_GRACE { fe.tick(); } // boot-flow: P_ESTHERO's 3 s skip grace [M 0x20a060:18]
    assert!(fe.action(Action::Confirm));
    assert!(!fe.intro_playing);
    assert_eq!(fe.flow.get_state(), ST_RUNNER, "the movie state popped [M case 6]");
    assert_eq!(fe.page, Page::Main);
    // Sister: without a skip the run ends by itself at the budget.
    let mut fe = FrontEnd::new();
    for _ in 0..8 { fe.tick(); }
    fe.action(Action::Confirm);
    for _ in 0..nf_view::menu::ESTHERO_TICKS { fe.flow.main_tick(); }
    fe.tick();
    assert!(!fe.intro_playing, "the run ends when the movie ends [M 0x20a060]");
    assert_eq!(fe.page, Page::Main);
}

/// The ONE movie plane (fix-movie-playback's, gpu.rs) draws the backdrop BEFORE the overlay's
/// quads, so with a live backdrop frame the overlay draws NO dark plate — the plane IS the page's
/// background [M movies.md §Presentation; PS2UploadBGMImage 0x108740]; without a live frame (no
/// `movies` feature, or the slim ISO's zeroed PSS files [L]) the plate stands in — the pre-movies
/// look. fix-menu-movies2: the old hud-texture-slot contract (the movie quad as the first run) is
/// replaced by `FrontEnd::backdrop_live`.
#[test]
fn the_menu_backdrop_plate_yields_to_the_movie_plane() {
    use nf_view::menu::{FrontEnd, Page};
    let font = crate_font();
    // plate(&mut h, …, gs(0x180c08, 0xff)): the dark plate's exact vertex colour (menu.rs).
    let plate = [(0x18u32 as f32) / 128.0, (0x0cu32 as f32) / 128.0, (0x08u32 as f32) / 128.0, 0.9921875];
    let has_plate = |f: &nf_view::gpu::HudFrame| f.v.iter().any(|q| q.c == plate);
    // The font texture, so the pages' text quads exist (the plate itself is an ndc_quad on the
    // same plain texture).
    let textures = vec![(0x0300_0003u32, nf_view::scene::TextureImage { key: (usize::MAX, 0x0300_0003), width: 4, height: 4, rgba: vec![255; 64], more: Vec::new(), rate: 1 })];
    // With the plane live: no plate quad — and the page's controls still draw.
    let mut fe = FrontEnd::new();
    fe.page = Page::Main;
    fe.backdrop_live = true;
    let f = nf_view::menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    assert!(!f.runs.is_empty(), "the page controls still draw over the movie plane");
    assert!(!has_plate(&f), "no dark plate while the movie plane carries the backdrop");
    // Sister: without a live frame the plate stands in (the pre-movies look).
    let mut fe = FrontEnd::new();
    fe.page = Page::Main;
    let f2 = nf_view::menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    assert!(has_plate(&f2), "the dark plate stands in without a live backdrop frame");
    // The intro page: the headless plate stands in there too (the host suppresses the whole
    // overlay while a real intro frame is up — the game shows no controls during P_INTRO [M 0x209e88]).
    let mut fe = FrontEnd::new();
    fe.intro_playing = true;
    let f3 = nf_view::menu::menu_overlay(&fe, &font, &font, None, &textures, 4.0 / 3.0);
    assert!(has_plate(&f3), "the intro's plate stands in without a live intro frame");
}

/// The menu loop plays through the ONE player path (`Player::start_looping` — the same
/// `MovieStream` closed-GOP decoder the streaming test pins, wrapped in the movie-playback
/// player): the backdrop frames reach `present()` as 512×448 RGBA and pin to the same decode the
/// MovieStream test measures. Needs an image with movie data; skips without one.
#[cfg(feature = "movies")]
#[test]
fn the_menu_loop_plays_through_the_one_player_path() {
    let path = std::env::var_os("NF_MOVIE_ISO").or_else(|| std::env::var_os("NF_ISO")).unwrap_or_default();
    let mut p = match nf_view::movie::Player::start_looping(path.to_string_lossy().as_ref(), 0x0733_0048) {
        Ok(p) => p,
        Err(e) => { eprintln!("skip: {e:?}"); return }
    };
    assert!(p.is_looping(), "the page backdrop loops [M 0x21c858:338 loop = 1]");
    assert!(!p.finished(), "a looping backdrop never reports finished [M 0x00108060 restarts]");
    // present() paces one frame per second call; collect the first 13 distinct frames and pin
    // frame 12 to the measured decode (the same SHA-1 the MovieStream test pins — the ONE decoder
    // path produces the same pixels through the player).
    let mut hashes: Vec<[u8; 20]> = Vec::new();
    let mut prev: Option<Vec<u8>> = None;
    for _ in 0..3000 {
        match p.present() {
            Some((w, h, rgba)) => {
                // present() hands out owned w/h (both sides of the fix-movies-tests merge fixed
                // this line's old (&u32, &u32) shape that failed the --features-movies build).
                assert_eq!((w, h), (512, 448), "the game's movie geometry [M movies.md]");
                let v = rgba.to_vec();
                if prev.as_deref() != Some(v.as_slice()) { hashes.push(nf_formats::sha1::sha1(&v)); prev = Some(v); }
                if hashes.len() >= 13 { break; }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            None => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
    assert!(hashes.len() >= 13, "the looping player produced only {} frames", hashes.len());
    assert_eq!(&hashes[12][..4], &[0x21, 0xd0, 0x73, 0x9d], "frame 12 of 07330048 through the ONE player path decodes exactly like the measured pin");
    p.stop();
}

fn crate_font() -> nf_formats::hud::Font {
    // The same stand-in font the menus_flow tests build (the layout only needs glyphs).
    let glyph = |code: u16| nf_formats::hud::Glyph { u: 0, v: 0, w: 8, h: 12, yoff: 0, pre: 0, post: 8, code, kerns: vec![] };
    nf_formats::hud::Font { first: 33, last: 126, gap: 0.0, space: 3.0, glyphs: (33..=126).map(glyph).collect() }
}
