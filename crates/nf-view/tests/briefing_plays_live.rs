//! Playtest 1016: "cutscene didnt play when I started the exchange, it went straight to the loading
//! screen then to game". ResetMap_Load phase 0 pushes GameFlow state 6 with no frame budget of its own
//! [M 0x1d6360:101] and starts the movie in the same phase [M 0x1d6360:111 psiStartBackgroundMovie], so
//! BGFMVPlaying = 1 [M 0x1081a8] and case 6 holds until psiMovieFinished [M 0x1c98c8 case 6, 0x1082e8]. The port's
//! countdown popped state 6 on its first tick because the host had not reported the decode-ahead
//! player as playing.
//!
//! This drives the real `FrontEnd` and the window's own glue (`nf_view::mission_movie_glue`) with the
//! window's worst-case clock: up to 8 front-end ticks per redraw (`fe_clock`). Needs a retail image with
//! movie data (`NF_ISO`); silent.

#[cfg(feature = "movies")]
#[test]
fn the_exchange_briefing_plays_before_the_level_loads() {
    use nf_view::menu::{self, Action, FrontEnd, Page, ST_MOVIE, ST_RUNNER};
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Some(iso) = std::env::var_os("NF_ISO") else { return };
    let iso = iso.to_string_lossy().to_string();
    let mut fe = FrontEnd::new();
    for _ in 0..8 {
        fe.tick();
        if fe.flow.get_state() == ST_RUNNER && fe.flow.current_level == menu::FRONTEND_LEVEL { break; }
    }
    // START → P_ESTHERO's intro (runs out headless) → Main → NightFire → Play without saving → the
    // box → difficulty → Single Player, the same walk as movie_playback_flow.
    fe.action(Action::Confirm);
    for _ in 0..3 + menu::ESTHERO_TICKS { fe.tick(); }
    assert_eq!(fe.page, Page::Main);
    fe.action(Action::Confirm);
    for _ in 0..menu::DELAYED_PAGE_TICKS + menu::MAIN_OPEN_BLOCK { fe.tick(); }
    fe.action(Action::Confirm);
    fe.action(Action::Confirm);
    fe.action(Action::Confirm);
    fe.unlock_all = true;
    fe.cursor = 1;
    assert!(fe.action(Action::Confirm), "The Exchange");
    assert_eq!(fe.flow.level_to_load, 0x0710_0005);

    let mut slot: Option<nf_view::movie::Player> = None;
    let mut state6_frames = 0u32;
    let mut saw_player = false;
    for _ in 0..600 {
        // The window's RedrawRequested order: the movie glue, then the front-end ticks.
        nf_view::mission_movie_glue(&mut fe, &mut slot, &iso);
        for _ in 0..8 { fe.tick(); }
        if let Some(p) = slot.as_mut() {
            if p.code() == 0x0710_0005 { saw_player = true; let _ = p.present(); }
        }
        if fe.flow.get_state() == ST_MOVIE { state6_frames += 1; }
        if state6_frames >= 120 { break; }
        if fe.flow.phase2_resolved.is_some() { break; }
    }
    assert!(saw_player, "the briefing 07100005 never started");
    assert!(state6_frames >= 120,
        "state 6 lasted {state6_frames} redraws: the briefing was cut before it played [M 0x1c98c8 case 6 holds while BGFMVPlaying]");
    assert!(fe.flow.phase2_resolved.is_none(), "the level load must wait for the briefing");

    // Sister: the skip still ends it and the load resolves to the level (the hold is not a hang).
    fe.flow.skip_movie();
    if let Some(p) = slot.take() { p.stop(); }
    let mut resolved = None;
    for _ in 0..20 {
        nf_view::mission_movie_glue(&mut fe, &mut slot, &iso);
        for _ in 0..8 { fe.tick(); }
        if let Some(r) = fe.flow.phase2_resolved { resolved = Some(r); break; }
    }
    assert_eq!(resolved, Some(0x0700_0005), "after the skip the load resolves to The Exchange [M 0x1d3560]");
}
