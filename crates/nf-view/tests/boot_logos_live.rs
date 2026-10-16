//! The boot logos (EA 0x07370048, MGM 0x07380048) as P_INTRO plays them (playtest 1016: "what the hell is
//! happening between the EA games and MGM?" — Enter skipped both logos, and the menu music cut in between).
//!
//! Evidence:
//! - P_INTRO_Handler 0x209e88 [M] clears DAT_002e1c49 before each movie, and Menu_StopMovie 0x215a50 [M]
//!   stops nothing while it is 0: the logos cannot be skipped.
//! - Between the two movies the finished arm's SFXStart pair and the movie-2 arm's SFXRemove pair run in
//!   the SAME handler call (movie_283 1 → 2) [M 0x209e88]: no music is heard between the logos. After the
//!   second movie the pair restarts and the title opens (0x44 → 0x40000009) [M 0x209e88].

use nf_view::menu::{self, Action, FrontEnd, ST_MOVIE, ST_RUNNER};

fn at_boot_intro() -> FrontEnd {
    let mut fe = FrontEnd::new();
    for _ in 0..8 {
        fe.tick();
        if fe.flow.get_state() == ST_RUNNER && fe.flow.current_level == menu::FRONTEND_LEVEL { break; }
    }
    fe.start_boot_intro();
    assert_eq!(fe.flow.get_state(), ST_MOVIE, "the boot intro runs in the movie state");
    fe
}

#[test]
fn enter_never_skips_the_boot_logos() {
    let mut fe = at_boot_intro();
    for _ in 0..400 {
        fe.flow.movie_playing = true; // the host's hold while a real movie plays
        fe.action(Action::Confirm);
        fe.tick();
        assert!(fe.intro_playing && fe.flow.get_state() == ST_MOVIE && fe.intro_index == 0,
            "Enter skipped the EA logo [M 0x209e88 clears DAT_002e1c49; 0x215a50 needs it]");
    }
    // Sister: the logos do end when the movies finish (the gate is on the skip, not the flow).
    fe.flow.movie_playing = false;
    fe.intro_movie_finished();
    fe.intro_movie_finished();
    for _ in 0..4 { fe.tick(); }
    assert!(!fe.intro_playing, "the intro ends after the second logo");
}

#[test]
fn no_menu_music_between_the_logos_and_it_returns_after_them() {
    let mut fe = at_boot_intro();
    fe.music.movie_started();
    assert!(!fe.music.jingles_up);
    // EA ends; MGM is next.
    fe.intro_movie_finished();
    assert_eq!(fe.intro_index, 1);
    assert!(!fe.music.jingles_up, "the menu music started between EA and MGM");
    // The state holds over the gap while the host opens MGM (the fe clock can run several ticks).
    for _ in 0..8 { fe.tick(); }
    assert!(fe.intro_playing && fe.flow.get_state() == ST_MOVIE, "the intro ended in the gap between the logos");
    // MGM ends: the pair restarts [M 0x209e88 finished arm].
    fe.intro_movie_finished();
    assert!(fe.music.jingles_up, "the menu music is back after the logos");
}
