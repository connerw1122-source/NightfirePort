//! fix-menu-backdrop: the front end's backdrop still (`movie_still`, the `--page main --shot`
//! path) decoded zero frames on the retail bytes — every `--movie-frame N` printed
//! "no frame N in the first chunk — the dark plate stands in" and every page drew the dark plate.
//!
//! Root cause: `gop_chunks`' leading-open-GOP guard (`bounds.first() != Some(&0)`) also fired when
//! the FIRST GOP is CLOSED — which it is on every Nightfire movie (the first GOP sits right after
//! the 145-byte sequence header) — so chunk 0 was `(0, head_end)`: the sequence header alone, zero
//! GOPs, zero decoded frames. `movie_still` reads `chunks.first()`, so every still failed.
//!
//! Game side: the page's backdrop IS the aux movie's own picture — `Menu_PlayMovie(aux, 0, 1, …)`
//! [M `Manager_SendMessage` 0x21c858:338] starts it through `psiStartBackgroundMovie` 0x001081a8,
//! `DecodeBGFMVFrame` 0x00108550 decodes each frame and `PS2UploadBGMImage` 0x108740 uploads it as
//! the page's background texture. The port's still is that first frame headlessly.
//!
//! Skips unless NF_MOVIE_ISO/NF_ISO points at an image with the movie data (the retail image or
//! the mini ISO; the slim on-foot ISO's PSS files are 0-byte stubs [L]).

fn iso_path() -> Option<String> {
    std::env::var_os("NF_MOVIE_ISO").or_else(|| std::env::var_os("NF_ISO")).map(|p| p.to_string_lossy().into_owned())
}

/// The main page's backdrop still decodes: frame 2 (inside the FIRST closed-GOP chunk — 10
/// pictures per GOP [L golden/pss_movies.tsv: 180 I pictures / 1800 total]) is a real 512×448
/// picture, not the dark plate.
#[cfg(feature = "movies")]
#[test]
fn the_main_page_backdrop_still_decodes_a_frame() {
    let Some(path) = iso_path() else { eprintln!("skip: no NF_MOVIE_ISO/NF_ISO with movie data"); return };
    let img = match nf_view::movie::movie_still(&path, 0x0733_0048, 2) {
        Ok(i) => i,
        Err(e) => panic!("07330048 frame 2 (the main page's backdrop): {e:#}"),
    };
    assert_eq!((img.width, img.height), (512, 448), "the game's movie geometry [M movies.md]");
    assert_eq!(img.rgba.len(), 512 * 448 * 4);
    // Sister: the frame is a picture, not one flat colour (the plate stand-in is a solid quad).
    let first = img.rgba[0];
    assert!(img.rgba.iter().any(|&b| b != first), "frame 2 is flat colour — no real picture decoded");
}

/// The list pages' backdrop still decodes too (07350048, frame 6).
#[cfg(feature = "movies")]
#[test]
fn the_list_pages_backdrop_still_decodes_a_frame() {
    let Some(path) = iso_path() else { eprintln!("skip: no NF_MOVIE_ISO/NF_ISO with movie data"); return };
    let img = match nf_view::movie::movie_still(&path, 0x0735_0048, 6) {
        Ok(i) => i,
        Err(e) => panic!("07350048 frame 6 (the list pages' backdrop): {e:#}"),
    };
    assert_eq!((img.width, img.height), (512, 448));
    let first = img.rgba[0];
    assert!(img.rgba.iter().any(|&b| b != first));
}

/// Frame 12 lives in the SECOND closed-GOP chunk (10 pictures per GOP) — the still must walk
/// chunks past the first, and what it decodes must be the SAME picture the whole-ES decode pins
/// (`menu_movies::the_menu_loop_movie_decodes`: RGB8 SHA-1 `1f1fe7b0…`).
#[cfg(feature = "movies")]
#[test]
fn the_backdrop_still_frame12_matches_the_whole_es_decode() {
    let Some(path) = iso_path() else { eprintln!("skip: no NF_MOVIE_ISO/NF_ISO with movie data"); return };
    let img = match nf_view::movie::movie_still(&path, 0x0733_0048, 12) {
        Ok(i) => i,
        Err(e) => panic!("07330048 frame 12: {e:#}"),
    };
    let sha = nf_formats::sha1::sha1(&img.rgba);
    assert_eq!(
        &sha[..4], &[0x1f, 0x1f, 0xe7, 0xb0],
        "frame 12 of 07330048 differs from the whole-ES decode's pin — the chunk walk reorders or rescales pictures"
    );
    // Sister: frame 13 hashes differently (the walk advances, not repeats).
    let n13 = nf_view::movie::movie_still(&path, 0x0733_0048, 13).unwrap();
    assert_ne!(nf_formats::sha1::sha1(&n13.rgba)[..4], sha[..4], "frames 12 and 13 hashed the same");
}

/// The windowed loop's player starts on the main page and ADVANCES (the live backdrop is a
/// playing movie, not a still): `Player::start_looping` (the game's loop=1 [M 0x21c858:338])
/// yields real distinct frames through `present()`.
#[cfg(feature = "movies")]
#[test]
fn the_windowed_loop_player_starts_and_advances_on_the_main_page() {
    let Some(path) = iso_path() else { eprintln!("skip: no NF_MOVIE_ISO/NF_ISO with movie data"); return };
    let mut p = match nf_view::movie::Player::start_looping(&path, 0x0733_0048) {
        Ok(p) => p,
        Err(e) => panic!("the main page's backdrop player refused to start: {e:?}"),
    };
    let mut first: Option<(u32, u32)> = None;
    let mut distinct = std::collections::HashSet::new();
    for _ in 0..600 {
        // One present per 60 Hz tick with a small sleep: the worker decodes ~10-picture chunks in
        // ~0.15 s, so 6 s of ticks is plenty for the first four frames.
        if let Some((w, h, rgba)) = p.present() {
            if first.is_none() { first = Some((w, h)); }
            distinct.insert(nf_formats::sha1::sha1(rgba));
            if distinct.len() > 3 { break; }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    p.stop();
    assert_eq!(first, Some((512, 448)), "the backdrop plane gets the movie's own geometry");
    assert!(distinct.len() > 3, "the loop played {} distinct frames in 6 s — it does not advance", distinct.len());
}

/// The still path reads the image the movies are on: NF_MOVIE_ISO over the ISO ARGUMENT (the live
/// loop's `movie_iso_path` precedence) — never the bare NF_ISO env again (with
/// `nf-view <retail.iso> 07000048 --page main` the still used to read the slim image's 0-byte PSS
/// stubs and reported them, not the retail bytes).
#[test]
fn the_still_path_prefers_nf_movie_iso_then_the_iso_argument() {
    // The suite runs --test-threads=1; set and restore around the check.
    std::env::remove_var("NF_MOVIE_ISO");
    std::env::set_var("NF_ISO", "/slim/onfoot.iso");
    assert_eq!(nf_view::movie_iso_name("/retail/usa.iso"), "/retail/usa.iso", "the ISO argument wins over NF_ISO");
    std::env::set_var("NF_MOVIE_ISO", "/retail/usa.iso");
    assert_eq!(nf_view::movie_iso_name("/slim/onfoot.iso"), "/retail/usa.iso", "NF_MOVIE_ISO wins (the live loop's precedence)");
    std::env::remove_var("NF_MOVIE_ISO");
    std::env::remove_var("NF_ISO");
}
