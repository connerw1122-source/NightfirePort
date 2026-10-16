//! End-to-end menu paths, driven ONLY through `action()` and `tick()` from `FrontEnd::new()` — no
//! page/cursor writes. The playtest 1015-2231 found every one of these broken while each page's own
//! tests (which jumped to the page by assignment) passed: no single-player mission could be started
//! (the first unlocked row is a driving mission and the flow hung in the DRIVING.ELF state), local
//! MP backed out into the wrong setup prompt, a second Enter re-requested the level, and a notice
//! box could never be dismissed.

use nf_view::menu::{Action, FrontEnd, Page};

/// One action, then the frames a player would wait: the page-change frames, and any intro /
/// attract movie run out (the headless movies are countdowns).
fn act(fe: &mut FrontEnd, a: Action) -> bool {
    let r = fe.action(a);
    for _ in 0..60 { fe.tick(); }
    for _ in 0..5000 { if !fe.intro_playing && !fe.attract_playing { break; } fe.tick(); }
    r
}

/// Boot → title → (intro) → Main.
fn to_main(fe: &mut FrontEnd) {
    assert_eq!(fe.page, Page::Start, "the boot opens the title");
    assert!(act(fe, Action::Confirm), "the title's Enter");
    assert_eq!(fe.page, Page::Main, "after the intro movies, the main menu");
}

/// Main → NightFire → Play without saving → the default difficulty → the mission list.
fn to_missions(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, Action::Confirm), "NightFire");
    assert_eq!(fe.page, Page::Codename);
    assert!(act(fe, Action::Down));
    assert_eq!(fe.cursor, 1, "Play without saving");
    assert!(act(fe, Action::Confirm), "the confirm box opens");
    assert!(act(fe, Action::Confirm), "the box's yes");
    assert_eq!(fe.page, Page::Difficulty);
    assert!(act(fe, Action::Confirm), "the difficulty");
    assert_eq!(fe.page, Page::MissionSelect);
}

#[test]
fn a_fresh_profile_can_start_the_first_on_foot_mission() {
    let mut fe = FrontEnd::new();
    to_missions(&mut fe);
    assert_eq!(fe.cursor, 0, "a fresh profile opens on Paris Prelude");
    // Paris Prelude is a DRIVING mission (0x09000001): the port can't play it — a notice, no
    // request, and the flow is NOT left in the DRIVING.ELF state.
    assert!(act(&mut fe, Action::Confirm));
    assert!(fe.notice.is_some(), "the driving notice shows");
    assert_eq!(fe.requested_level, None, "no level is requested for a driving mission");
    assert_eq!(fe.flow.get_state(), 2, "the flow stays in the front end's runner");
    // While the notice stands, other keys do nothing; Enter dismisses it.
    assert!(!act(&mut fe, Action::Down), "the notice swallows Down");
    assert_eq!(fe.cursor, 0);
    assert!(act(&mut fe, Action::Confirm), "Enter dismisses the notice");
    assert!(fe.notice.is_none());
    // The Exchange (07000005) opened through the driving row and starts.
    assert!(act(&mut fe, Action::Down));
    assert_eq!(fe.cursor, 1);
    assert!(fe.unlocked_row(1), "the row after a driving row is unlocked through it");
    assert!(act(&mut fe, Action::Confirm), "The Exchange starts");
    assert_eq!(fe.requested_level, Some(0x0700_0005));
    // A second Enter while the request is pending is refused (the game deletes the menu manager).
    assert!(!act(&mut fe, Action::Confirm), "no second request");
    assert_eq!(fe.requested_level, Some(0x0700_0005));
    // Sister: a row that is NOT behind a driving row stays locked (Alpine Escape needs The Exchange).
    assert!(!fe.unlocked_row(2) || fe.unlock_all, "Alpine Escape stays locked on a fresh profile");
}

#[test]
fn back_walks_the_single_player_chain_to_main() {
    let mut fe = FrontEnd::new();
    to_missions(&mut fe);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Difficulty);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Codename);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Main);
    // …and forward again still works.
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::Codename);
}

/// Main → Multiplayer → Local → join + ready → the scenario page.
fn to_scenario(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, Action::Down));
    assert!(act(fe, Action::Confirm), "Multiplayer");
    assert_eq!(fe.page, Page::MpMode);
    assert!(act(fe, Action::Confirm), "Local");
    assert_eq!(fe.page, Page::MpJoin);
    assert!(act(fe, Action::Confirm), "join");
    assert!(act(fe, Action::Confirm), "codename");
    assert!(act(fe, Action::Confirm), "ready");
    assert_eq!(fe.page, Page::MpScenario);
}

#[test]
fn local_mp_backs_out_of_setup_and_still_reaches_one_match_request() {
    let mut fe = FrontEnd::new();
    to_scenario(&mut fe);
    // Row 1 = Arena (free-for-all) → the map page → setup on the CHARACTER prompt (no team pick).
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Confirm), "the scenario");
    assert_eq!(fe.page, Page::MpMap);
    assert!(act(&mut fe, Action::Confirm), "the map");
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 3, "FFA opens on the character prompt [M P_MPSETUP 0x20d7a8]");
    assert!(act(&mut fe, Action::Confirm), "the character");
    assert_eq!(fe.mp_setup_state, 4, "the handicap prompt");
    assert!(act(&mut fe, Action::Confirm), "the handicap");
    assert_eq!(fe.page, Page::MpOptions);
    // Back out of the options hub: the setup page re-enters on the character prompt again — not
    // the team prompt the stale state left (playtest 1015-2231).
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 3, "back on the character prompt");
    assert!(act(&mut fe, Action::Confirm));
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpOptions);
    // One human and no bots: Continue is refused WITH the game's message box (0x39e "at least two
    // players" [M C_SBMPOPTIONS 0x20e818's 0x39e arm]) — it used to refuse silently.
    assert!(!act(&mut fe, Action::Confirm), "Continue refuses a one-player match");
    assert_eq!(fe.mp_error, 0x39e, "the refusal's message stands");
    assert!(!act(&mut fe, Action::Down), "the message box swallows other keys");
    assert!(act(&mut fe, Action::Confirm), "Enter dismisses the message");
    assert_eq!(fe.mp_error, 0);
    // Add a bot: AI Bots → bot 1 → the character → enabled → back to the hub.
    assert!(act(&mut fe, Action::Down));
    assert_eq!(fe.mp_options_row, 1, "the AI Bots row");
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpBotChoose);
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpBotSetup);
    assert!(fe.mp_bots[0].enabled, "bot 1 is in");
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::MpOptions);
    assert!(act(&mut fe, Action::Up));
    assert_eq!(fe.mp_options_row, 0, "back on Continue");
    assert!(act(&mut fe, Action::Confirm), "Continue with a bot");
    assert_eq!(fe.page, Page::MpConfirm);
    assert!(act(&mut fe, Action::Confirm), "start the match");
    let want = fe.requested_level.expect("the match requests its arena");
    assert_eq!(fe.mp_match_settings.as_ref().map(|s| s.bot_count), Some(1), "the bot rides the settings");
    assert!(fe.mp_match_settings.is_some(), "the settings ride the request");
    // A second Enter is refused — it started a second load of the arena (playtest 1015-2231).
    assert!(!act(&mut fe, Action::Confirm), "no second start");
    assert_eq!(fe.requested_level, Some(want));
}

#[test]
fn quick_game_back_out_and_restart() {
    let mut fe = FrontEnd::new();
    to_scenario(&mut fe);
    // Row 0 = Quick Game → straight to the confirm page [M C_SBMPSCEN:48-145].
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpConfirm);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::MpOptions);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::MpSetup);
    // The quick game's scenario is FFA: the character prompt, not the team one.
    assert_ne!(fe.mp_setup_state, 2, "no team prompt for a free-for-all");
    assert!(act(&mut fe, Action::Confirm));
    assert!(act(&mut fe, Action::Confirm));
    assert!(act(&mut fe, Action::Confirm), "Continue");
    assert_eq!(fe.page, Page::MpConfirm);
    assert!(act(&mut fe, Action::Confirm));
    assert!(fe.requested_level.is_some(), "the match still starts after backing out");
}

#[test]
fn a_notice_is_dismissed_by_back_too_and_swallows_other_keys() {
    let mut fe = FrontEnd::new();
    to_missions(&mut fe);
    assert!(act(&mut fe, Action::Confirm), "Paris Prelude → the notice");
    assert!(fe.notice.is_some());
    assert!(!act(&mut fe, Action::Up));
    assert!(!act(&mut fe, Action::Side));
    assert_eq!(fe.page, Page::MissionSelect, "swallowed keys change nothing");
    assert!(act(&mut fe, Action::Back), "Esc dismisses");
    assert!(fe.notice.is_none());
    assert_eq!(fe.page, Page::MissionSelect, "dismissing doesn't also go back");
}
