//! net-online-lobby: the online Host / Join front-end flow, headless (no ISO, no sockets, no
//! threads). The real transport is 051's deterministic UDP lockstep; here a FakeSession (the
//! test advises the transitions) rides the same `MpSession` trait the front end holds, so the
//! page flow is proven end to end:
//!
//! Main row 1 → PAGE_MP_MODE (Local / Host Online / Join Online) [S
//! docs/systems/net-online-lobby.md §2] → Host: PAGE_MP_HOST (port 27420 + Start Listening) →
//! the game's 0x40000019 as the host lobby (remote agents in panels 2-4) → the usual
//! Scenario → Map → Setup → Options → Confirm walk with the choices published and the match
//! start riding the session [M P_MPCONFIRM 0x20ec00's handover]; Join: PAGE_MP_NETJOIN (typed
//! IP:PORT, Connecting / refused / timed-out) → PAGE_MP_LOBBY (the host's choices mirrored
//! read-only) → the match handover. Every test keeps a sister assertion that can fail.

use std::cell::RefCell;
use std::rc::Rc;

use nf_game::mp::MpSettings;
use nf_view::menu::{self,
    mp_net_addr_ok, Action, FrontEnd, MpNetAgent, MpNetChoices, MpNetError, MpNetLink, MpSession,
    MpNetState, Page, MP_MAP_ROWS, MP_NET_TIMEOUT_TICKS, PAGE_MP_HOST, PAGE_MP_LOBBY, PAGE_MP_MODE,
    PAGE_MP_NETJOIN,
};

// ── the fake session ─────────────────────────────────────────────────────────────────────────

/// The fake's state; the test drives it through the advise/join helpers.
struct Fake {
    link: MpNetLink,
    port: Option<u16>,
    dialed: Option<String>,
    busy: bool,
    agents: Vec<MpNetAgent>,
    choices: Option<MpNetChoices>,
    started: Option<MpSettings>,
    closed: usize,
    codename: String,
    /// poll() turns a Dialing link into Connected (the handshake landed).
    advise_connected: bool,
    /// poll() turns a Dialing link into Refused.
    advise_refused: bool,
    /// poll() turns a Dialing link into TimedOut.
    advise_timeout: bool,
}

impl Fake {
    fn new() -> Self {
        Fake {
            link: MpNetLink::Idle, port: None, dialed: None, busy: false,
            agents: Vec::new(), choices: None, started: None, closed: 0, codename: String::new(),
            advise_connected: false, advise_refused: false, advise_timeout: false,
        }
    }
    fn ensure_local(&mut self) {
        if !self.agents.iter().any(|a| a.slot == 0) {
            self.agents.insert(0, MpNetAgent { slot: 0, codename: self.codename.clone(), ready: true });
        } else if let Some(me) = self.agents.iter_mut().find(|a| a.slot == 0) {
            me.codename = self.codename.clone();
        }
    }
    /// A remote agent joins slot 1..3 (panel 2-4).
    fn join_remote(&mut self, slot: usize, codename: &str) {
        self.agents.retain(|a| a.slot != slot);
        self.agents.push(MpNetAgent { slot, codename: codename.into(), ready: true });
        self.agents.sort_by_key(|a| a.slot);
    }
}

/// The handle the front end owns; the test keeps the Rc side to advise transitions.
struct FakeHandle(Rc<RefCell<Fake>>);
impl MpSession for FakeHandle {
    fn poll(&mut self) {
        let mut f = self.0.borrow_mut();
        if f.link == MpNetLink::Dialing {
            if f.advise_refused { f.link = MpNetLink::Refused; }
            else if f.advise_timeout { f.link = MpNetLink::TimedOut; }
            else if f.advise_connected { f.link = MpNetLink::Connected; }
        }
    }
    fn link(&self) -> MpNetLink { self.0.borrow().link }
    fn listen(&mut self, port: u16) -> Result<(), MpNetError> {
        let mut f = self.0.borrow_mut();
        if f.busy { return Err(MpNetError::Busy); }
        f.port = Some(port);
        f.link = MpNetLink::Listening;
        Ok(())
    }
    fn connect(&mut self, addr: &str) -> Result<(), MpNetError> {
        let mut f = self.0.borrow_mut();
        f.dialed = Some(addr.into());
        f.link = MpNetLink::Dialing;
        Ok(())
    }
    fn agents(&self) -> Vec<MpNetAgent> { self.0.borrow().agents.clone() }
    fn host_choices(&self) -> Option<MpNetChoices> { self.0.borrow().choices.clone() }
    fn publish_choices(&mut self, c: &MpNetChoices) { self.0.borrow_mut().choices = Some(c.clone()); }
    fn start_match(&mut self, s: &MpSettings) { self.0.borrow_mut().started = Some(s.clone()); }
    fn match_started(&self) -> Option<MpSettings> { self.0.borrow().started.clone() }
    fn set_codename(&mut self, name: &str) {
        let mut f = self.0.borrow_mut();
        f.codename = name.into();
        f.ensure_local();
    }
    fn close(&mut self) {
        let mut f = self.0.borrow_mut();
        f.closed += 1;
        f.link = MpNetLink::Idle;
    }
}

/// A FrontEnd with the fake wired.
fn fe_with_session() -> (FrontEnd, Rc<RefCell<Fake>>) {
    let fake = Rc::new(RefCell::new(Fake::new()));
    let mut fe = FrontEnd::new();
    fe.mp_session = Some(Box::new(FakeHandle(fake.clone())));
    (fe, fake)
}

/// Main → Multiplayer → the port's mode page.
fn to_mode(fe: &mut FrontEnd) {
    fe.tick(); // boot → the runner state [M 0x1c98c8 case 1]
    fe.page = Page::Main;
    fe.cursor = 1; // Multiplayer [M MAIN_ITEMS 0x149]
    assert!(fe.action(Action::Confirm), "Multiplayer confirm must navigate");
    assert_eq!(fe.page, Page::MpMode, "the mode choice is up [S]");
    assert_eq!(fe.page.id(), PAGE_MP_MODE);
}

// ── the host side ────────────────────────────────────────────────────────────────────────────

#[test]
fn host_flow_listen_agents_choices_and_start() {
    let (mut fe, fake) = fe_with_session();
    to_mode(&mut fe);
    // Host Online → the host page; the port default, then the port row is a TYPED field
    // (the playtest 1015-2142 "why does the port count down": ◀ ▶ / click-stepping is gone).
    fe.cursor = 1;
    assert!(fe.action(Action::Confirm), "Host Online confirm must navigate");
    assert_eq!(fe.page, Page::MpHost);
    assert_eq!(fe.page.id(), PAGE_MP_HOST);
    assert_eq!(fe.mp_net_port, 27420, "the plan's default port [S]");
    assert!(!fe.action(Action::Left), "◀ no longer steps the port");
    assert!(!fe.action(Action::Right), "▶ no longer steps the port");
    assert_eq!(fe.mp_net_port, 27420, "the arrows left it alone");
    for _ in 0..5 { assert!(fe.mp_net_backspace()); }
    assert_eq!(fe.mp_net_port, 0, "Backspace clears digit by digit");
    for c in "30000".chars() { assert!(fe.mp_net_type(c)); }
    assert_eq!(fe.mp_net_port, 30000, "typed digits build the port");
    assert!(!fe.mp_net_type('7'), "a sixth digit would pass 65535 — refused");
    assert!(!fe.mp_net_type('.'), "only digits type on the port row");
    for _ in 0..5 { fe.mp_net_backspace(); }
    // Start refuses a port outside 1024..=65535 (the typed field can hold e.g. 80), and stays put.
    for _ in 0..5 { fe.mp_net_backspace(); }
    for c in "80".chars() { fe.mp_net_type(c); }
    fe.cursor = 1;
    assert!(!fe.action(Action::Confirm), "Start refuses port 80");
    assert_eq!(fe.mp_net_error, Some(MpNetError::BadAddress));
    assert!(!fe.mp_hosting, "nothing listens on a refused port");
    fe.cursor = 0;
    for _ in 0..5 { fe.mp_net_backspace(); }
    for c in "27420".chars() { fe.mp_net_type(c); }
    assert_eq!(fe.mp_net_port, 27420);
    // Start Listening (row 1).
    fe.cursor = 1;
    assert!(fe.action(Action::Confirm), "Start Listening must navigate");
    assert_eq!(fe.page, Page::MpJoin, "the host lobby IS the game's Join Game page");
    assert_eq!(fe.page.id(), 0x4000_0019);
    assert!(fe.mp_hosting, "host mode armed");
    assert_eq!(fake.borrow().port, Some(27420), "the session bound the port");
    assert_eq!(fake.borrow().link, MpNetLink::Listening);
    assert!(fe.mp_join[0].joined, "the host's own panel joins");
    assert!(fe.mp_join[0].ready, "the host is ready by definition [S]");
    assert_eq!(fe.mp_join[0].codename, "Player 1", "the template codename [M Txt 0x1c3 + slot]");
    // A remote agent arrives into panel 2 with its codename.
    fake.borrow_mut().join_remote(1, "Alec");
    fe.tick();
    assert!(fe.mp_join[1].joined, "the remote agent lands in panel 2");
    assert_eq!(fe.mp_join[1].codename, "Alec");
    assert_eq!(fe.rows(), 4, "the Join page keeps its four panels");
    // Confirm on the host's own (ready) panel walks to the scenario page [S §2.1].
    fe.cursor = 0;
    assert!(fe.action(Action::Confirm), "the host's ✕ walks on");
    assert_eq!(fe.page, Page::MpScenario);
    assert_eq!(fe.page.id(), 0x4000_001a);
    // The usual flow: Scenario → Map → Setup → Options → Confirm.
    fe.mp_scenario_row = 1; // Arena / Deathmatch
    assert!(fe.action(Action::Confirm), "the scenario confirm navigates");
    for _ in 0..menu::CLOSE_IRIS_PAGE_TICKS + 2 { fe.tick(); } // [M C_SBMPSCEN 0x20c600:45, G-048(1)]
    assert_eq!(fe.page, Page::MpMap);
    assert!(fe.action(Action::Confirm), "the map confirm navigates");
    assert_eq!(fe.page, Page::MpSetup);
    // 611b (mp-defaults-and-setup) made the setup page the game's prompt chain: team (team games
    // only) → character → handicap, then the options hub [M C_RBMPSETUP 0x20de58's 0x4b states 2-4].
    // Arena is FFA, so two picks: the character, then the handicap.
    assert!(fe.action(Action::Confirm), "the character pick moves to the handicap prompt");
    assert_eq!(fe.page, Page::MpSetup, "the handicap prompt is still the setup page");
    assert!(fe.action(Action::Confirm), "the handicap pick navigates to the options hub");
    assert_eq!(fe.page, Page::MpOptions);
    assert!(fe.action(Action::Confirm), "Continue runs the start checks");
    assert_eq!(fe.page, Page::MpConfirm, "2 humans pass the start checks [M C_SBMPOPTIONS 0x20e818]");
    // The publish diff rides the tick — the walk above changed the words, so tick once.
    fe.tick();
    // Sister: the choices were published and they are the host's words.
    let c = fake.borrow().choices.clone().expect("the host published its choices");
    assert_eq!(c.scenario, fe.mp_scenario);
    assert_eq!(c.level, fe.mp_level);
    assert_eq!(c.time_limit, fe.mp_time_limit);
    // Start the match — the settings ride the session.
    assert!(fe.action(Action::Confirm), "the summary's ✕ starts");
    assert!(fe.mp_match_settings.is_some(), "the host handed the match over");
    let s = fake.borrow().started.clone().expect("the match start rode the session");
    assert_eq!(s.level_code, fe.mp_level);
    assert_eq!(s.human_count, 2, "the host + the remote agent");
    assert_eq!(s.master, 1, "the host's copy keeps master = 1 [M +0x180 = 1]");
}

#[test]
fn host_port_busy_reports_and_back_cancels() {
    let (mut fe, fake) = fe_with_session();
    to_mode(&mut fe);
    fe.cursor = 1;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpHost);
    // The port is taken: the page stays and reports.
    fake.borrow_mut().busy = true;
    fe.cursor = 1;
    assert!(!fe.action(Action::Confirm), "a busy listen fails");
    assert_eq!(fe.page, Page::MpHost, "the page stays");
    assert!(!fe.mp_hosting);
    assert_eq!(fe.mp_net_status(), Some("Port already in use"));
    // The port frees up: the listen lands.
    fake.borrow_mut().busy = false;
    assert!(fe.action(Action::Confirm), "the retry listens");
    assert_eq!(fe.page, Page::MpJoin);
    assert!(fe.mp_hosting);
    // A remote agent is in; Back leaves and cancels cleanly.
    fake.borrow_mut().join_remote(1, "Alec");
    fe.tick();
    assert!(fe.mp_join[1].joined);
    fe.cursor = 0;
    assert!(fe.action(Action::Back), "the host lobby's Back leaves");
    assert_eq!(fe.page, Page::MpMode);
    assert!(!fe.mp_hosting, "host mode dropped");
    assert!(!fe.mp_join[0].joined, "the panels reset");
    assert!(!fe.mp_join[1].joined, "the remote panel reset too");
    assert_eq!(fake.borrow().closed, 1, "the session closed once");
    assert_eq!(fake.borrow().link, MpNetLink::Idle);
}

// ── the join side ────────────────────────────────────────────────────────────────────────────

#[test]
fn join_flow_type_connect_lobby_and_match_start() {
    let (mut fe, fake) = fe_with_session();
    to_mode(&mut fe);
    fe.cursor = 2;
    assert!(fe.action(Action::Confirm), "Join Online confirm must navigate");
    assert_eq!(fe.page, Page::MpNetJoin);
    assert_eq!(fe.page.id(), PAGE_MP_NETJOIN);
    // The typing: digits, dots, colon; letters never land; Backspace deletes.
    for c in "127.0.0.1:27420".chars() { assert!(fe.mp_net_type(c), "{c} must type"); }
    assert!(!fe.mp_net_type('x'), "letters never land");
    assert!(fe.mp_net_backspace(), "Backspace deletes");
    assert_eq!(fe.mp_net_addr, "127.0.0.1:2742");
    assert!(fe.mp_net_type('0'), "the corrected digit lands");
    assert_eq!(fe.mp_net_addr, "127.0.0.1:27420");
    // Paste keeps the simple set only; nothing simple surviving pastes nothing.
    assert!(!fe.mp_net_paste("abc"), "nothing simple survives → nothing lands");
    assert_eq!(fe.mp_net_addr, "127.0.0.1:27420", "letters never paste");
    assert!(fe.mp_net_paste(":5"), "digits and the colon paste");
    assert_eq!(fe.mp_net_addr, "127.0.0.1:27420:5", "…appended (the next dial's gate catches it)");
    assert!(fe.mp_net_backspace());
    assert!(fe.mp_net_backspace());
    assert_eq!(fe.mp_net_addr, "127.0.0.1:27420");
    // Connect → Connecting…
    assert!(fe.action(Action::Confirm), "Connect dials");
    assert_eq!(fe.mp_net_state, MpNetState::Dialing);
    assert_eq!(fake.borrow().dialed.as_deref(), Some("127.0.0.1:27420"));
    assert_eq!(fe.mp_net_status(), Some("Connecting..."));
    // The handshake lands → the lobby.
    fake.borrow_mut().advise_connected = true;
    fe.tick();
    assert_eq!(fe.page, Page::MpLobby, "connected → the lobby [S]");
    assert_eq!(fe.page.id(), PAGE_MP_LOBBY);
    assert_eq!(fe.mp_net_state, MpNetState::InLobby);
    assert!(fe.mp_join[0].joined, "the client's own panel mirrors");
    assert_eq!(fe.mp_join[0].codename, "Player 1");
    assert_eq!(fe.mp_net_status(), Some("Waiting for the host..."), "no choices yet");
    // The host's choices arrive: the mirror shows them.
    let published = MpNetChoices {
        scenario: 1, level: MP_MAP_ROWS[0].3, weapon_set: 0, radar: 1, friendly_fire: 0,
        frag_limit: 10, time_limit: 10, player_mods: [0, 0], envmods: [1, 0, 0, 0, 0],
    };
    fake.borrow_mut().choices = Some(published.clone());
    fe.tick();
    assert_eq!(fe.mp_lobby_choices, Some(published.clone()), "the mirror took the publish");
    let lines = fe.mp_lobby_lines(None);
    assert_eq!(lines[0].0, 0x3d4, "the confirm page's own Map pair [M P_MPCONFIRM]");
    assert_eq!(lines[0].1, format!("<{:#x}>", MP_MAP_ROWS[0].1), "the map line resolves through the shared table");
    assert_eq!(lines[4], (0x3d6, "10".to_string()), "the duration line");
    // A remote agent arrives; the lobby's panels mirror it.
    fake.borrow_mut().join_remote(1, "Alec");
    fe.tick();
    assert!(fe.mp_join[1].joined && fe.mp_join[1].codename == "Alec");
    // The host starts the match: the same handover the host's confirm page runs.
    let mut s = MpSettings::deathmatch(MP_MAP_ROWS[0].3);
    s.scenario = 1;
    s.level_code = MP_MAP_ROWS[0].3;
    fake.borrow_mut().started = Some(s);
    fe.tick();
    assert!(fe.mp_match_settings.is_some(), "the match handover ran");
    assert_eq!(fe.requested_level, Some(MP_MAP_ROWS[0].3), "the level request rides the load");
    assert_eq!(fe.mp_net_state, MpNetState::Idle, "the lobby is over");
}

#[test]
fn join_refused_reports_and_back_cancels() {
    let (mut fe, fake) = fe_with_session();
    to_mode(&mut fe);
    fe.cursor = 2;
    assert!(fe.action(Action::Confirm));
    for c in "10.0.0.9:27420".chars() { fe.mp_net_type(c); }
    assert!(fe.action(Action::Confirm), "Connect dials");
    assert_eq!(fe.mp_net_state, MpNetState::Dialing);
    // The host refuses: the page reports and keeps the address for a retry.
    fake.borrow_mut().advise_refused = true;
    fe.tick();
    assert_eq!(fe.mp_net_state, MpNetState::Refused);
    assert_eq!(fe.mp_net_status(), Some("Connection refused"));
    assert_eq!(fe.page, Page::MpNetJoin, "the page stays");
    assert_eq!(fake.borrow().closed, 1, "the attempt closed");
    assert_eq!(fe.mp_net_addr, "10.0.0.9:27420", "the address survives for a retry");
    // A retry dials again.
    fake.borrow_mut().advise_refused = false;
    assert!(fe.action(Action::Confirm), "the retry dials");
    assert_eq!(fe.mp_net_state, MpNetState::Dialing);
    // And Back cancels cleanly.
    assert!(fe.action(Action::Back), "Back cancels");
    assert_eq!(fe.page, Page::MpMode);
    assert_eq!(fe.mp_net_state, MpNetState::Idle);
    assert_eq!(fake.borrow().closed, 2, "the retry's session closed too");
}

#[test]
fn join_times_out_at_600_ticks() {
    let (mut fe, _fake) = fe_with_session();
    to_mode(&mut fe);
    fe.cursor = 2;
    assert!(fe.action(Action::Confirm));
    for c in "10.0.0.9:27420".chars() { fe.mp_net_type(c); }
    assert!(fe.action(Action::Confirm), "Connect dials");
    assert_eq!(fe.mp_net_timeout, MP_NET_TIMEOUT_TICKS, "the full budget [S 600 ticks = 10 s]");
    // Sister: one tick short of the budget still shows Connecting…
    for _ in 0..MP_NET_TIMEOUT_TICKS - 1 { fe.tick(); }
    assert_eq!(fe.mp_net_state, MpNetState::Dialing);
    assert_eq!(fe.mp_net_status(), Some("Connecting..."));
    // The last tick fires the timeout.
    fe.tick();
    assert_eq!(fe.mp_net_state, MpNetState::TimedOut);
    assert_eq!(fe.mp_net_status(), Some("Connection timed out"));
    assert_eq!(fe.page, Page::MpNetJoin);
}

// ── the gates ────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_address_gate_and_bad_address_report() {
    assert!(mp_net_addr_ok("127.0.0.1:27420"));
    assert!(mp_net_addr_ok("255.255.255.255:65535"));
    assert!(mp_net_addr_ok("0.0.0.0:1"));
    assert!(!mp_net_addr_ok("127.0.0.1:0"), "port 0 is not a port");
    assert!(!mp_net_addr_ok("127.0.0.1:65536"), "above the UDP range");
    assert!(!mp_net_addr_ok("256.0.0.1:27420"), "an octet above 255");
    assert!(!mp_net_addr_ok("1270.0.1:27420"), "a part above 3 digits");
    assert!(!mp_net_addr_ok("127.0.0.1"), "no port");
    assert!(!mp_net_addr_ok(":27420"), "no host");
    assert!(!mp_net_addr_ok("nightfire.local:27420"), "the typed set has no letters [S]");
    // Through the page: a bad address keeps the page and reports it.
    let (mut fe, _fake) = fe_with_session();
    to_mode(&mut fe);
    fe.cursor = 2;
    assert!(fe.action(Action::Confirm));
    for c in "127.0.0.1".chars() { fe.mp_net_type(c); }
    assert!(!fe.action(Action::Confirm), "a portless address refuses");
    assert_eq!(fe.page, Page::MpNetJoin);
    assert_eq!(fe.mp_net_state, MpNetState::Idle, "no dial started");
    assert_eq!(fe.mp_net_status(), Some("Enter an address as IP:PORT"));
}

#[test]
fn without_a_session_the_pages_report_it() {
    let mut fe = FrontEnd::new();
    to_mode(&mut fe);
    // Host side.
    fe.cursor = 1;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpHost);
    fe.cursor = 1;
    assert!(!fe.action(Action::Confirm), "Start Listening without a session fails");
    assert_eq!(fe.page, Page::MpHost);
    assert!(!fe.mp_hosting);
    assert_eq!(fe.mp_net_status(), Some("No network session is wired"));
    // Join side.
    assert!(fe.action(Action::Back));
    fe.cursor = 2;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpNetJoin);
    for c in "127.0.0.1:27420".chars() { fe.mp_net_type(c); }
    assert!(!fe.action(Action::Confirm), "Connect without a session fails");
    assert_eq!(fe.mp_net_state, MpNetState::Idle);
    assert_eq!(fe.mp_net_status(), Some("No network session is wired"));
}

#[test]
fn the_mode_page_keeps_local_and_back() {
    let mut fe = FrontEnd::new();
    to_mode(&mut fe);
    // Row 0 Local: the game's own couch page [M the pre-port route].
    assert!(fe.action(Action::Confirm), "Local confirm must navigate");
    assert_eq!(fe.page, Page::MpJoin);
    assert_eq!(fe.page.id(), 0x4000_0019);
    // Back out to the mode page, then Back to Main.
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::MpMode);
    fe.cursor = 2;
    assert!(fe.action(Action::Confirm));
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::MpMode);
    assert!(fe.action(Action::Back), "the mode page's Back reaches Main");
    assert_eq!(fe.page, Page::Main);
    assert_eq!(fe.cursor, 1, "the cursor lands back on Multiplayer");
}

// ── the mouse rows ───────────────────────────────────────────────────────────────────────────

#[test]
fn mouse_rows_hover_and_click_the_new_pages() {
    let (mut fe, fake) = fe_with_session();
    to_mode(&mut fe);
    // Hover the Host Online row (the hit rows are 143/175/207), click confirms it.
    assert!(fe.mouse_move(320.0, 183.0), "hovering row 1 moves the cursor");
    assert_eq!(fe.cursor, 1);
    assert!(fe.mouse_click(320.0, 183.0), "the click confirms");
    assert_eq!(fe.page, Page::MpHost);
    // Clicks on the port row never change the port (the old ◀ ▶ value columns knocked it down on
    // every click — the playtest 1015-2142 report).
    fe.mouse_move(320.0, 151.0);
    assert_eq!(fe.cursor, 0);
    fe.mouse_click(370.0, 151.0);
    fe.mouse_click(450.0, 151.0);
    assert_eq!(fe.mp_net_port, 27420, "clicks leave the port alone");
    // The Start row's click listens (the move lands the cursor on row 1 first).
    assert!(fe.mouse_move(320.0, 183.0), "hovering row 1 moves the cursor");
    assert!(fe.mouse_click(320.0, 183.0), "the Start row's click confirms");
    assert_eq!(fe.page, Page::MpJoin);
    assert!(fe.mp_hosting);
    assert_eq!(fake.borrow().port, Some(27420));
    // The join side: the address row's click dials. The drawn address line is the page's one row
    // — the value label at (200,250,240×18), middle y 259 [M menu_overlay's MpHost/MpNetJoin arm;
    // menu-mouse-parity moved the hit row from the stale 160 to the drawn position].
    assert!(fe.action(Action::Back)); // leave the host lobby
    fe.cursor = 2;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpNetJoin);
    for c in "127.0.0.1:27420".chars() { fe.mp_net_type(c); }
    assert!(!fe.mouse_click(320.0, 168.0), "sister: a click off the drawn row does nothing");
    assert_eq!(fe.mp_net_state, MpNetState::Idle, "the off-row click did not dial");
    assert!(fe.mouse_click(320.0, 259.0), "the address row's click confirms");
    assert_eq!(fe.mp_net_state, MpNetState::Dialing);
}
