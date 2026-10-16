//! The fail page's REPLAY reloads the MISSION's level even when the front-end load the end armed
//! has already landed (menu-e2e-singleplayer finding 1: a tick-delayed REPLAY requested 0x07000048,
//! the menu map, because the load overwrote `flow.current_level` = GameState+0x12 first).

use nf_view::menu::{Action, FrontEnd, Outcome, Page, Stats, ST_RUNNER};

#[test]
fn replay_after_the_end_load_lands_still_reloads_the_mission() {
    let mut fe = FrontEnd::new();
    // The mission is running on 07000005 (the game state a level load leaves; the flow's runner).
    fe.flow.stack = vec![ST_RUNNER];
    fe.flow.current_level = 0x0700_0005;
    fe.mission_ended(Outcome::Failed, Stats::default());
    assert_eq!(fe.page, Page::Results);
    // Let the end's front-end load run its course before the player answers.
    for _ in 0..600 { fe.tick(); let _ = fe.flow.load_tick(); }
    fe.requested_level = None;
    // The load may have moved GameState+0x12 to the menu map — REPLAY must not follow it.
    fe.flow.current_level = 0x0700_0048;
    fe.cursor = 0; // REPLAY [L page 0x40000042: REPLAY = 0]
    assert!(fe.action(Action::Confirm), "REPLAY");
    assert_eq!(fe.requested_level, Some(0x0700_0005), "REPLAY reloads the mission's level, not the menu map");
    // Sister: without a recorded end the old behaviour (the current level) stands.
    let mut fe2 = FrontEnd::new();
    assert_eq!(fe2.ended_level, None);
    fe2.flow.current_level = 0x0700_0006;
    assert_eq!(fe2.ended_level.unwrap_or(fe2.flow.current_level), 0x0700_0006);
}
