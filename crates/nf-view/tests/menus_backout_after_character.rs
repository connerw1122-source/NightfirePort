//! The playtest 1015-2231 walk the task pins: pick a character for player 1, back out one page,
//! back to the main MP menu, forward again — and the match can still start afterwards. Driven
//! ONLY through `action()` / `tick()` from `FrontEnd::new()` (menus_e2e.rs style — the older flow
//! tests jumped to pages by assignment and missed exactly this class of bug).
//!
//! Game side of every step:
//! - the setup page's enter clears EVERY agent's ready byte [M P_MPSETUP 0x20d7a8's 0x4c loop,
//!   `pcVar5[1] = '\0'` over the 4 slots] and re-derives the prompt from the scenario's team bit;
//! - the join page's enter memsets the whole mpjoin array (0x40 bytes), zeroes the per-agent
//!   states, and re-seats the default skins/teams [M P_MPJOIN 0x20ca00's 0x4c, `memset(&mpjoin,
//!   0, 0x40)` + the DAT_002dee80/90/a0 skin and DAT_002dee6c/7c/8c/9c team stores] — a re-entered
//!   Join Game page is always a fresh join;
//! - the setup page's Back walks 4 → 3 always, 3 → 2 only in team games, and pops the page
//!   otherwise [M C_RBMPSETUP 0x20de58's 0x6b arm + the propagating return];
//! - the scenario wheel's Back is Main on the Multiplayer row [M P_MPSCENARIO 0x217348:29].

use std::cell::RefCell;
use std::rc::Rc;

use nf_game::mp::MpSettings;
use nf_view::menu::{
    Action, FrontEnd, MpNetAgent, MpNetChoices, MpNetError, MpNetLink, MpSession, Page,
};

/// One action, then the frames a player would wait: the page-change frames, and any intro /
/// attract movie run out (the headless movies are countdowns).
fn act(fe: &mut FrontEnd, a: Action) -> bool {
    let r = fe.action(a);
    for _ in 0..60 {
        fe.tick();
    }
    for _ in 0..5000 {
        if !fe.intro_playing && !fe.attract_playing {
            break;
        }
        fe.tick();
    }
    r
}

/// Boot → title → (intro) → Main.
fn to_main(fe: &mut FrontEnd) {
    assert_eq!(fe.page, Page::Start, "the boot opens the title");
    assert!(act(fe, Action::Confirm), "the title's Enter");
    assert_eq!(fe.page, Page::Main, "after the intro movies, the main menu");
}

/// Main → Multiplayer → Local → join (codename, ready) → the scenario page.
fn to_scenario(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, Action::Down), "the Multiplayer row");
    assert!(act(fe, Action::Confirm), "Multiplayer");
    assert_eq!(fe.page, Page::MpMode);
    assert!(act(fe, Action::Confirm), "Local");
    assert_eq!(fe.page, Page::MpJoin);
    assert!(act(fe, Action::Confirm), "join");
    assert!(act(fe, Action::Confirm), "codename");
    assert!(act(fe, Action::Confirm), "ready");
    assert_eq!(fe.page, Page::MpScenario);
}

/// Arena (row 1) → Skyrail → the setup page on the FFA character prompt. The wheel re-opens on
/// the stored row [M Menu_SelectItemInControl 0x2020f8's scan], so the step down only happens
/// from the fresh row 0.
fn to_ffa_setup_after_character(fe: &mut FrontEnd) {
    if fe.mp_scenario_row == 0 {
        assert!(act(fe, Action::Down), "the Arena row");
    }
    assert_eq!(fe.mp_scenario_row, 1, "the Arena row");
    assert!(act(fe, Action::Confirm), "the scenario");
    assert_eq!(fe.page, Page::MpMap);
    assert!(act(fe, Action::Confirm), "the map");
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 3, "FFA opens on the character prompt [M P_MPSETUP 0x20d7a8]");
    assert!(act(fe, Action::Confirm), "the character pick");
    assert_eq!(fe.mp_setup_state, 4, "the handicap prompt [M C_RBMPSETUP 0x20de58's 0x4b]");
    assert!(act(fe, Action::Confirm), "the handicap pick");
    assert_eq!(fe.page, Page::MpOptions, "the picks land the options hub [M the 0x44 send]");
}

#[test]
fn ffa_backout_after_character_returns_to_a_working_menu_and_a_match_still_starts() {
    let mut fe = FrontEnd::new();
    to_scenario(&mut fe);
    to_ffa_setup_after_character(&mut fe);

    // ── back out one page: the hub's Back re-opens the setup on the character prompt ──
    assert!(act(&mut fe, Action::Back), "the hub's Back");
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 3, "the hub re-enters the setup on the character prompt");
    // The game clears every agent's ready byte on this enter [M P_MPSETUP 0x20d7a8's 0x4c].
    assert!(!fe.mp_join[0].ready, "the setup page's enter clears ready [M 0x20d7a8 0x4c]");

    // ── back to the main MP menu ──
    assert!(act(&mut fe, Action::Back), "the character prompt's Back pops to the map wheel");
    assert_eq!(fe.page, Page::MpMap);
    assert!(act(&mut fe, Action::Back), "the map wheel's Back");
    assert_eq!(fe.page, Page::MpScenario);
    assert!(act(&mut fe, Action::Back), "the scenario wheel's Back is Main [M P_MPSCENARIO:29]");
    assert_eq!(fe.page, Page::Main);
    assert_eq!(fe.cursor, 1, "back on the Multiplayer row");

    // ── forward again: the join page is a FRESH join [M P_MPJOIN 0x20ca00's 0x4c memset] ──
    assert!(act(&mut fe, Action::Confirm), "Multiplayer");
    assert!(act(&mut fe, Action::Confirm), "Local");
    assert_eq!(fe.page, Page::MpJoin);
    assert_eq!(fe.mp_join[0].state, 0, "the re-entered join page resets the agent state");
    assert!(!fe.mp_join[0].joined, "…and the joined byte");
    assert!(!fe.mp_join[0].ready, "…and the ready byte");
    // The panel invites a join again (0x1e8), so ✕ joins.
    assert!(act(&mut fe, Action::Confirm), "the fresh panel's ✕ joins");
    assert!(act(&mut fe, Action::Confirm), "codename");
    assert!(act(&mut fe, Action::Confirm), "ready");
    assert_eq!(fe.page, Page::MpScenario);

    // ── forward through the setup once more; the match must still start ──
    to_ffa_setup_after_character(&mut fe);
    // One human and no bots: Continue refuses with the game's message box.
    assert!(!act(&mut fe, Action::Confirm), "Continue refuses a one-player match");
    assert_eq!(fe.mp_error, 0x39e, "the ≥2-players refusal [M C_SBMPOPTIONS 0x20e818]");
    assert!(act(&mut fe, Action::Confirm), "Enter dismisses the message");
    assert_eq!(fe.mp_error, 0);
    // Add a bot, then start.
    assert!(act(&mut fe, Action::Down), "the AI Bots row");
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(&mut fe, Action::Confirm), "Setup Bot 1");
    assert_eq!(fe.page, Page::MpBotChoose);
    assert!(act(&mut fe, Action::Confirm), "the bot's character");
    assert_eq!(fe.page, Page::MpBotSetup);
    assert!(act(&mut fe, Action::Confirm), "the bot's stats");
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(&mut fe, Action::Back), "the bots list's Back");
    assert_eq!(fe.page, Page::MpOptions);
    assert!(act(&mut fe, Action::Up), "back on Continue");
    assert!(act(&mut fe, Action::Confirm), "Continue with the bot");
    assert_eq!(fe.page, Page::MpConfirm);
    assert!(act(&mut fe, Action::Confirm), "the match starts after the whole walk");
    let want = fe.requested_level.expect("the match requests its arena");
    assert_eq!(want, 0x0700_0024, "Skyrail is the picked arena");
    assert!(fe.mp_match_settings.is_some(), "the settings ride the request");
}

// ── the fake session (the host lobby is the port's only two-human path — no pads) ───────────

struct Fake {
    link: MpNetLink,
    port: Option<u16>,
    busy: bool,
    agents: Vec<MpNetAgent>,
    choices: Option<MpNetChoices>,
    started: Option<MpSettings>,
}

impl Fake {
    fn new() -> Self {
        Fake { link: MpNetLink::Idle, port: None, busy: false, agents: Vec::new(), choices: None, started: None }
    }
    fn join_remote(&mut self, slot: usize, codename: &str) {
        self.agents.retain(|a| a.slot != slot);
        self.agents.push(MpNetAgent { slot, codename: codename.into(), ready: true });
        self.agents.sort_by_key(|a| a.slot);
    }
}

struct FakeHandle(Rc<RefCell<Fake>>);
impl MpSession for FakeHandle {
    fn poll(&mut self) {}
    fn link(&self) -> MpNetLink { self.0.borrow().link }
    fn listen(&mut self, port: u16) -> Result<(), MpNetError> {
        let mut f = self.0.borrow_mut();
        if f.busy { return Err(MpNetError::Busy); }
        f.port = Some(port);
        f.link = MpNetLink::Listening;
        Ok(())
    }
    fn connect(&mut self, _addr: &str) -> Result<(), MpNetError> { Ok(()) }
    fn agents(&self) -> Vec<MpNetAgent> { self.0.borrow().agents.clone() }
    fn host_choices(&self) -> Option<MpNetChoices> { self.0.borrow().choices.clone() }
    fn publish_choices(&mut self, c: &MpNetChoices) { self.0.borrow_mut().choices = Some(c.clone()); }
    fn start_match(&mut self, s: &MpSettings) { self.0.borrow_mut().started = Some(s.clone()); }
    fn match_started(&self) -> Option<MpSettings> { self.0.borrow().started.clone() }
    fn set_codename(&mut self, _name: &str) {}
    fn close(&mut self) { self.0.borrow_mut().link = MpNetLink::Idle; }
}

/// Main → Multiplayer → Host Online → Start Listening → the host lobby (the join page).
fn to_host_lobby(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, Action::Down), "the Multiplayer row");
    assert!(act(fe, Action::Confirm), "Multiplayer");
    assert_eq!(fe.page, Page::MpMode);
    assert!(act(fe, Action::Down), "the Host Online row");
    assert!(act(fe, Action::Confirm), "Host Online");
    assert_eq!(fe.page, Page::MpHost);
    assert!(act(fe, Action::Down), "the Start Listening row");
    assert!(act(fe, Action::Confirm), "Start Listening");
    assert_eq!(fe.page, Page::MpJoin, "the host lobby IS the join page");
}

#[test]
fn two_joined_players_survive_the_same_backout_and_the_match_starts() {
    let fake = Rc::new(RefCell::new(Fake::new()));
    let mut fe = FrontEnd::new();
    fe.mp_session = Some(Box::new(FakeHandle(fake.clone())));
    to_host_lobby(&mut fe);
    assert!(fe.mp_hosting, "host mode armed");
    assert!(fe.mp_join[0].joined && fe.mp_join[0].ready, "the host's own panel is in");
    // A second human lands in panel 2.
    fake.borrow_mut().join_remote(1, "Alec");
    fe.tick();
    assert!(fe.mp_join[1].joined, "the remote agent lands in panel 2");
    assert_eq!(fe.mp_human_count(), 2, "two joined players");

    // The host's ✕ walks on; the usual flow to the setup page and through the character.
    assert!(act(&mut fe, Action::Confirm), "the host's ✕ walks on");
    assert_eq!(fe.page, Page::MpScenario);
    to_ffa_setup_after_character(&mut fe);

    // ── the same back-out: hub → setup → map → scenario → Main ──
    assert!(act(&mut fe, Action::Back), "the hub's Back");
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 3, "the character prompt again");
    assert!(!fe.mp_join[0].ready, "the enter cleared the host's ready [M 0x20d7a8 0x4c]");
    assert!(!fe.mp_join[1].ready, "…and the remote agent's [M the same 4-slot loop]");
    assert!(act(&mut fe, Action::Back), "the character prompt's Back");
    assert_eq!(fe.page, Page::MpMap);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::MpScenario);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Main);

    // ── forward again: host again, the agent lands again, the match starts ──
    assert!(act(&mut fe, Action::Confirm), "Multiplayer");
    assert!(act(&mut fe, Action::Down), "the Host Online row");
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpHost);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Confirm), "Start Listening");
    assert_eq!(fe.page, Page::MpJoin);
    assert!(fe.mp_join[0].joined && fe.mp_join[0].ready, "the host panel is in again");
    fake.borrow_mut().join_remote(1, "Alec");
    fe.tick();
    assert!(fe.mp_join[1].joined, "the agent lands again");
    assert!(act(&mut fe, Action::Confirm), "the host's ✕ walks on");
    assert_eq!(fe.page, Page::MpScenario);
    to_ffa_setup_after_character(&mut fe);
    // Two humans pass the start checks with no bot.
    assert!(act(&mut fe, Action::Confirm), "Continue passes with 2 humans");
    assert_eq!(fe.page, Page::MpConfirm);
    assert!(act(&mut fe, Action::Confirm), "the match starts");
    assert!(fe.mp_match_settings.is_some());
    let s = fe.mp_match_settings.as_ref().unwrap();
    assert_eq!(s.human_count, 2, "both humans ride the settings");
}
