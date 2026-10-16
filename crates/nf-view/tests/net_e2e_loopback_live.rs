//! net-e2e-loopback (task 706): the full online match, end to end over
//! 127.0.0.1, every menu step through FrontEnd actions (no direct page
//! writes). The host FE walks Multiplayer → Host Online → types the port →
//! Start Listening → the lobby; the client FE walks Join Online → types
//! 127.0.0.1:port → Connect → the lobby; the host picks Deathmatch / Skyrail /
//! the rules (frag limit 2) and starts; both machines hand over, load the
//! arena through `load_mp_carry` with the same (tunnelled) seed, and lockstep
//! 1200 ticks of scripted input — the per-tick state hashes agree, a scripted
//! kill through the game's own scoring path (`MP_PlayerKilled` 0x188f70) moves
//! both scores, the frag limit ends the match, and both front ends land on the
//! debriefing with the same board. Variant: the same match over a link that
//! drops 5% of the packets and transposes a share of them (the seeded
//! `UdpTransport::inject_fault`) — the lockstep layer's redundancy absorbs it
//! and the hashes still agree. NF_ISO-gated (the arena loads from the disc
//! like `net_match_live`).

use std::time::{Duration, Instant};

use nf_view::menu::{Action, FrontEnd, Page};
use nf_view::mpnet::SessionProxy;

fn iso() -> Option<String> {
    std::env::var("NF_ISO").ok()
}

/// Distinct per test binary: the binaries' tests run in parallel threads.
const PORT: u16 = 27950;
const ARENA: u32 = 0x0700_0024; // MP_MAP_ROWS[0] = Skyrail [D placement]
const KILL_TICK: u32 = 400;
const KILL_TICK2: u32 = 900;
const TICKS: u32 = 1200;
const DT: f32 = 1.0 / 60.0;

/// One action, then the frames a player would wait (menus_e2e_mp's act): the
/// page-change frames, the intro/attract countdowns run out, and — because
/// the FE's tick pumps the net session (mp_net_poll) — a slice of real time
/// for the lobby's exchange. `other` (the peer FE, if any) breathes too, so
/// its session answers the keepalives while this side walks its menus.
fn act(fe: &mut FrontEnd, other: Option<&mut FrontEnd>, a: Action) -> bool {
    let r = fe.action(a);
    match other {
        None => {
            for _ in 0..60 {
                fe.tick();
            }
            for _ in 0..5000 {
                if !fe.intro_playing && !fe.attract_playing { break; }
                fe.tick();
            }
            r
        }
        Some(o) => {
            for _ in 0..60 {
                fe.tick();
                o.tick();
            }
            for _ in 0..5000 {
                if !fe.intro_playing && !fe.attract_playing { break; }
                fe.tick();
                o.tick();
            }
            r
        }
    }
}

fn to_main(fe: &mut FrontEnd) {
    assert_eq!(fe.page, Page::Start, "the boot opens the title");
    assert!(act(fe, None, Action::Confirm), "the title's Enter");
    assert_eq!(fe.page, Page::Main, "after the intro movies, the main menu");
}

/// Main → Multiplayer → Host Online → the port page [M the MpMode arm
/// menu.rs:4633 → Page::MpHost].
fn to_host_page(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, None, Action::Down), "the Multiplayer row");
    assert!(act(fe, None, Action::Confirm), "Multiplayer");
    assert_eq!(fe.page, Page::MpMode, "the mode choice");
    assert!(act(fe, None, Action::Down), "the Host Online row");
    assert!(act(fe, None, Action::Confirm), "Host Online");
    assert_eq!(fe.page, Page::MpHost, "the port page");
}

/// Main → Multiplayer → Join Online → the address page [M menu.rs:4634 →
/// Page::MpNetJoin].
fn to_join_page(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, None, Action::Down), "the Multiplayer row");
    assert!(act(fe, None, Action::Confirm), "Multiplayer");
    assert_eq!(fe.page, Page::MpMode, "the mode choice");
    assert!(act(fe, None, Action::Down), "past Local");
    assert!(act(fe, None, Action::Down), "the Join Online row");
    assert!(act(fe, None, Action::Confirm), "Join Online");
    assert_eq!(fe.page, Page::MpNetJoin, "the address page");
}

/// The port digits through the REAL typing path, then Start Listening — the
/// real `mp_host_begin` (the port gate 1024..=65535, `sess.listen`, the host
/// lobby = the game's Join page) [M menu.rs:5627-5653].
fn host_listen(fe: &mut FrontEnd, port: u16) {
    // The port row opens on the default 27420 [M menu.rs:2057] — the real
    // keyboard path clears it first (Backspace is routed before the menu's
    // Back [M menu.rs:5881]), then types the digits.
    for _ in 0..6 {
        assert!(fe.mp_net_backspace(), "the backspace clears the default port");
    }
    for c in port.to_string().chars() {
        assert!(fe.mp_net_type(c), "the port digit {c} lands");
    }
    assert!(act(fe, None, Action::Down), "the Start Listening row");
    assert!(act(fe, None, Action::Confirm), "Start Listening");
    assert!(fe.mp_hosting, "the host is hosting");
    assert_eq!(fe.page, Page::MpJoin, "the host lobby is the game's Join page");
}

/// The address chars through the REAL typing path, then Connect — the real
/// `mp_net_dial` (the address gate, `sess.connect`, Dialing) — and pump until
/// the lobby opens [M menu.rs:5671-5692, 5715-5722]. The HOST's front end
/// breathes too: its session answers the HELLO, and a front end only pumps
/// its own session inside its own tick.
fn client_connect(fe: &mut FrontEnd, host: &mut FrontEnd, addr: &str) {
    for c in addr.chars() {
        assert!(fe.mp_net_type(c), "the address char {c} lands");
    }
    assert!(act(fe, None, Action::Confirm), "Connect");
    assert_eq!(fe.mp_net_status(), Some("Connecting..."), "dialing");
    let deadline = Instant::now() + Duration::from_secs(10);
    while fe.page != Page::MpLobby {
        fe.tick();
        host.tick();
        std::thread::sleep(Duration::from_millis(2));
        assert!(Instant::now() < deadline, "the client reaches the lobby");
    }
}

/// The scripted per-slot controls (net_match_live's script, both slots move —
/// the lockstep's honest workload).
fn scripted_input(t: u32, slot: u8) -> nf_net::Input {
    let mut i = nf_net::NEUTRAL;
    i.forward = ((t as i32 + slot as i32) % 97) as f32 / 97.0;
    i.strafe = if t % 11 == 0 { -0.5 } else { 0.0 };
    i.turn = ((t % 13) as f32 - 6.0) * 0.002;
    i.look = if t % 29 == 0 { 0.25 } else { 0.0 };
    i.jump = t % 61 == 0;
    i.crouch = t % 43 == 0;
    i.fire = t % 23 == 0;
    i
}

/// Pump both cores until tick `t` is ready on both (a missing input is a
/// stall, never a guess — bounded at 5 s; net_match_live's pump_till).
fn pump_till(h: &SessionProxy, c: &SessionProxy, t: u32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        {
            let mut hb = h.0.borrow_mut();
            hb.poll();
            if hb.ready_inputs(t).is_some() {
                drop(hb);
                let mut cb = c.0.borrow_mut();
                cb.poll();
                if cb.ready_inputs(t).is_some() {
                    return;
                }
            }
        }
        {
            let mut cb = c.0.borrow_mut();
            cb.poll();
        }
        assert!(Instant::now() < deadline, "tick {t}: both inputs arrive within 5 s");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// The kill injection: the game's own scoring path on BOTH machines, the same
/// tick, the same arguments — a cross-sim event, exactly the replay
/// contract's `MpEvent::Killed` shape [M mp/replay.rs apply_event →
/// player_killed; rules.rs 0x188f70].
fn scripted_kill(host: &mut nf_view::scene::Scene, client: &mut nf_view::scene::Scene) {
    for s in [host, client] {
        let sim = s.sim.as_mut().expect("the match sim");
        let (mp, rng) = (&mut sim.mp, &mut sim.drones.rng);
        mp.as_mut().expect("the mp match").player_killed(1, Some(0), false, false, rng);
    }
}

/// The per-tick MP drive the app's frame loop owns (scene.rs:1853 runs it
/// inside step_bond; this harness ticks the movement core directly, so the
/// update runs explicitly — both machines, the same draws).
fn mp_update(s: &mut nf_view::scene::Scene) {
    let sim = s.sim.as_mut().expect("the match sim");
    let (mp, rng) = (&mut sim.mp, &mut sim.drones.rng);
    if let Some(m) = mp {
        m.update(DT, false, rng);
    }
}

/// The full flow, parameterised by the link fault (`None` = clean loopback,
/// `Some((seed, drop‰, reorder‰))` = the 5%-loss variant). Returns nothing —
/// every step asserts.
fn run_e2e(path: &str, fault: Option<(u64, u32, u32)>) {
    // ── the two front ends, wired the way the app wires them (lib.rs:1845's
    // lazy `SessionProxy::host(0)` placeholder; the lobby's listen/connect
    // re-binds carry the fault recipe along) ──
    let (hp, cp) = match fault {
        None => (
            SessionProxy::host(0).expect("the host proxy"),
            SessionProxy::host(0).expect("the client's placeholder proxy"),
        ),
        Some(f) => (
            SessionProxy::host_faulty(0, f).expect("the lossy host proxy"),
            SessionProxy::host_faulty(0, f).expect("the lossy client proxy"),
        ),
    };
    let mut feh = FrontEnd::new();
    feh.mp_session = Some(Box::new(hp.clone()));
    let mut fec = FrontEnd::new();
    fec.mp_session = Some(Box::new(cp.clone()));

    // ── the host: Multiplayer → Host Online → the port → Start Listening ──
    to_host_page(&mut feh);
    host_listen(&mut feh, PORT);

    // ── the client: Join Online → the address → Connect → the lobby ──
    to_join_page(&mut fec);
    client_connect(&mut fec, &mut feh, &format!("127.0.0.1:{PORT}"));

    // ── the lobby: both sides mirror the agents (the host's panels fill from
    // the session; the client's own panel mirrors slot 0) ──
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(feh.mp_join[1].joined && fec.mp_join[0].joined) {
        feh.tick();
        fec.tick();
        std::thread::sleep(Duration::from_millis(2));
        assert!(Instant::now() < deadline, "both lobbies see both agents");
    }
    assert_eq!(feh.mp_join[1].codename, fec.mp_join[0].codename, "the codename rode the handshake");
    assert_eq!(feh.mp_human_count(), 2, "the host counts both humans");
    assert_eq!(feh.mp_net_status(), None, "the host lobby is quiet once the mirror is full");

    // ── the host picks the scenario/map/options and starts: Deathmatch →
    // Skyrail → the defaults → Game Rules (frag limit 10 → 2, so the scripted
    // kills end the match) → the start checks → the confirm ──
    assert!(act(&mut feh, Some(&mut fec), Action::Confirm), "the host's ready panel → the scenario wheel [M menu.rs:4714]");
    assert_eq!(feh.page, Page::MpScenario);
    assert!(act(&mut feh, Some(&mut fec), Action::Down), "the Arena row (row 0 is Quick Game [M MP_SCENARIO_ROWS])");
    assert!(act(&mut feh, Some(&mut fec), Action::Confirm), "Arena (Deathmatch, row 1) → the map wheel");
    assert_eq!(feh.page, Page::MpMap);
    assert!(act(&mut feh, Some(&mut fec), Action::Confirm), "Skyrail (row 0) → the setup");
    assert_eq!(feh.page, Page::MpSetup);
    assert!(act(&mut feh, Some(&mut fec), Action::Confirm), "the character");
    assert!(act(&mut feh, Some(&mut fec), Action::Confirm), "the handicap → the options hub");
    assert_eq!(feh.page, Page::MpOptions);
    assert!(act(&mut feh, Some(&mut fec), Action::Down), "the AI Bots row");
    assert!(act(&mut feh, Some(&mut fec), Action::Down), "the Game Rules row");
    assert!(act(&mut feh, Some(&mut fec), Action::Confirm), "Game Rules");
    assert_eq!(feh.page, Page::MpRules);
    assert!(act(&mut feh, Some(&mut fec), Action::Down), "the points row");
    for _ in 0..8 {
        assert!(act(&mut feh, Some(&mut fec), Action::Left), "the frag limit steps down");
    }
    assert_eq!(feh.mp_frag_limit, 2, "the frag limit is 2 — the scripted kills end the match");
    assert!(act(&mut feh, Some(&mut fec), Action::Confirm), "the rules close to the hub");
    assert_eq!(feh.page, Page::MpOptions);
    assert_eq!(feh.mp_options_row, 2, "the hub's row is where Rules was opened from");
    assert!(act(&mut feh, Some(&mut fec), Action::Up), "back to AI Bots");
    assert!(act(&mut feh, Some(&mut fec), Action::Up), "back to Continue");
    assert!(act(&mut feh, Some(&mut fec), Action::Confirm), "Continue passes the start checks");
    assert_eq!(feh.page, Page::MpConfirm);
    assert!(act(&mut feh, Some(&mut fec), Action::Confirm), "the summary's ✕ starts the match");
    let hs = feh.mp_match_settings.clone().expect("the host's settings ride the request");
    assert_eq!(feh.requested_level, Some(ARENA), "the host requests the arena");

    // ── the client's handover: the tunnelled match payload (mp_confirm_start
    // → sess.start_match → the tag-1 DATA) → mp_client_start ──
    let deadline = Instant::now() + Duration::from_secs(10);
    while fec.requested_level.is_none() {
        fec.tick();
        std::thread::sleep(Duration::from_millis(2));
        assert!(Instant::now() < deadline, "the client receives the match start");
    }
    let cs = fec.mp_match_settings.clone().expect("the client's handover settings");
    assert_eq!(fec.requested_level, Some(ARENA), "the client requests the same arena");
    // The same seed: the settings arrive from the host, the client mirrors
    // them — the level build's Rng::new is the same stream on both machines
    // [M net-match-host; lib.rs mp_seed → scene.rs:6063].
    assert_eq!(cs.seed, hs.seed, "the tunnelled seed is the host's");
    assert_eq!(cs.level_code, hs.level_code);
    assert_eq!(cs.frag_limit, hs.frag_limit, "the frag limit rode the tunnel");
    assert_eq!(cs.human_count, 2, "the client is a human on both machines");
    assert_eq!(cs.player_count, 2);
    assert_eq!(cs.master, 1, "the master flag rides the tunnel [M mp_confirm_start +0x180 = 1]");

    // ── both machines load the arena through the real load path and swap
    // THEIR slot's bond in as the local one ──
    let arena = format!("{ARENA:08x}.bin");
    let mut hscene = nf_view::load_mp_carry(path, &arena, false, None, hs.clone()).expect("the host sim loads");
    let mut cscene = nf_view::load_mp_carry(path, &arena, false, None, cs.clone()).expect("the client sim loads");
    hscene.sim.as_mut().expect("host sim").swap_local_slot(0);
    cscene.sim.as_mut().expect("client sim").swap_local_slot(1);
    assert!(hscene.sim.as_ref().unwrap().mp.as_ref().unwrap().settings.started, "the host's match started [M MP_Start 0x183a50:207]");
    assert!(cscene.sim.as_ref().unwrap().mp.as_ref().unwrap().settings.started, "the client's match started");

    // ── 1200 lockstep ticks: sample → pump → ready → remotes → local →
    // slots → the MP update → the scripted kills at 400/900 → the hash every
    // 15 ticks, asserted equal at every exchange ──
    let mut last_h = 0u64;
    let mut last_c = 0u64;
    let mut kill_scores_checked = 0usize;
    for t in 0..TICKS {
        hp.0.borrow_mut().local_input(t, scripted_input(t, 0));
        cp.0.borrow_mut().local_input(t, scripted_input(t, 1));
        pump_till(&hp, &cp, t);
        let hi = hp.0.borrow_mut().ready_inputs(t).unwrap();
        let ci = cp.0.borrow_mut().ready_inputs(t).unwrap();
        let hi = std::array::from_fn(|i| nf_view::mpnet::to_player_input(&hi[i]));
        let ci = std::array::from_fn(|i| nf_view::mpnet::to_player_input(&ci[i]));
        let hsim = hscene.sim.as_mut().unwrap();
        hsim.step_net_remotes(&hi, 0);
        let ladder = nf_game::player::LadderCtx::default();
        hsim.bond.tick_ctx(&hsim.world, &ladder, &hi[0]);
        let csim = cscene.sim.as_mut().unwrap();
        csim.step_net_remotes(&ci, 1);
        csim.bond.tick_ctx(&csim.world, &ladder, &ci[1]);
        if let Some(m) = hsim.mp.as_mut() {
            m.set_slot(0, hsim.bond.pos);
            if let Some(b) = &hsim.mp_bonds[1] { m.set_slot(1, b.pos); }
        }
        if let Some(m) = csim.mp.as_mut() {
            m.set_slot(1, csim.bond.pos);
            if let Some(b) = &csim.mp_bonds[0] { m.set_slot(0, b.pos); }
        }
        if t == KILL_TICK || t == KILL_TICK2 {
            scripted_kill(&mut hscene, &mut cscene);
        }
        mp_update(&mut hscene);
        mp_update(&mut cscene);
        if t == KILL_TICK {
            kill_scores_checked += 1;
            for (name, s) in [("host", hscene.sim.as_ref().unwrap()), ("client", cscene.sim.as_ref().unwrap())] {
                let g = &s.mp.as_ref().unwrap().game;
                assert_eq!(g.slots[0].kills, 1, "{name}: the kill credits slot 0");
                assert_eq!(g.slots[0].score, 1.0, "{name}: the frag is worth 1 [M rules.rs:136-152]");
                assert_eq!(g.slots[1].deaths, 1, "{name}: the victim's death counts");
                assert!(g.slots[1].dead, "{name}: the victim is dead");
                assert_eq!(g.state, 0, "{name}: one frag short of the limit, the match runs");
            }
        }
        if t == KILL_TICK2 {
            kill_scores_checked += 1;
            for (name, s) in [("host", hscene.sim.as_ref().unwrap()), ("client", cscene.sim.as_ref().unwrap())] {
                let g = &s.mp.as_ref().unwrap().game;
                assert_eq!(g.slots[0].kills, 2, "{name}: the second kill credits slot 0");
                assert_eq!(g.slots[0].score, 2.0, "{name}: the score reaches the limit");
                assert_eq!(g.slots[1].deaths, 2, "{name}: the victim's second death counts");
            }
        }
        if t % 15 == 0 {
            last_h = hscene.sim.as_ref().unwrap().net_state_hash(0);
            last_c = cscene.sim.as_ref().unwrap().net_state_hash(1);
            assert_eq!(last_h, last_c, "tick {t}: the sims' states agree");
            hp.0.borrow_mut().submit_hash(t, last_h);
            cp.0.borrow_mut().submit_hash(t, last_c);
        }
        let _ = hp.0.borrow_mut().poll();
        let _ = cp.0.borrow_mut().poll();
    }
    assert_eq!(kill_scores_checked, 2, "both kill ticks ran");
    assert!(!hp.0.borrow_mut().take_app_events().iter().any(|e| matches!(e, nf_view::mpnet::CoreEvent::Desync { .. })), "no desync over the honest run");
    assert!(!cp.0.borrow_mut().take_app_events().iter().any(|e| matches!(e, nf_view::mpnet::CoreEvent::Desync { .. })), "no desync over the honest run");
    assert_eq!(last_h, last_c, "the final hashes agree");

    // ── the match end: the frag limit latched (state 1 → 3 → 4 → 5 through
    // the MP update) and both machines request the debriefing ──
    for (name, s) in [("host", hscene.sim.as_ref().unwrap()), ("client", cscene.sim.as_ref().unwrap())] {
        let m = s.mp.as_ref().unwrap();
        assert_eq!(m.game.state, 5, "{name}: the end machine ran to the debriefing wait");
        assert!(m.game.request_debriefing, "{name}: the debriefing is requested [M MP_Update case 4]");
    }

    // ── the debriefing hand-off: both front ends open the board from their
    // own match [M open_mp_debriefing ← MP_Update case 4's GameState+0x1C =
    // 0x40000033] and the boards agree on the numbers ──
    feh.open_mp_debriefing(hscene.sim.as_ref().unwrap().mp.as_ref().unwrap());
    fec.open_mp_debriefing(cscene.sim.as_ref().unwrap().mp.as_ref().unwrap());
    assert_eq!(feh.page, Page::MpDebriefing, "the host lands on the debriefing");
    assert_eq!(fec.page, Page::MpDebriefing, "the client lands on the debriefing");
    let rh = feh.mp_debriefing_rows();
    let rc = fec.mp_debriefing_rows();
    assert_eq!(rh.len(), rc.len(), "the boards have the same rows");
    for (i, (a, b)) in rh.iter().zip(rc.iter()).enumerate() {
        assert_eq!(a.0, b.0, "row {i}: the rank label agrees");
        assert_eq!(a.3, b.3, "row {i}: the points agree");
        assert_eq!(a.4, b.4, "row {i}: the victories agree");
        assert_eq!(a.5, b.5, "row {i}: the deaths agree");
        assert_eq!(a.6, b.6, "row {i}: the total agrees");
    }
    // The winner line: the host's board carries the codename (the wire's
    // 208-byte MatchSettings has no name bytes — the client's own copy shows
    // the empty-name row; the numbers are what the lockstep guarantees).
    let (lh, nh) = feh.mp_debriefing_result();
    let (lc, _) = fec.mp_debriefing_result();
    assert_eq!(lh, lc, "the result labels agree");
    assert_eq!(nh, feh.mp_join[0].codename, "the host's board names the host's codename");
}

/// The clean e2e: host and join an online match over 127.0.0.1, lockstep 1200
/// ticks, the hashes agree, the scripted kill scores on both, the match ends
/// into both debriefings.
#[test]
fn host_and_join_an_online_match_end_to_end_over_loopback() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    run_e2e(&path, None);
}

/// The lossy variant: the same e2e over a link that drops 5% of the packets
/// (50‰) and transposes a share of them (80‰), seeded per side — the
/// lockstep's ack-window redundancy absorbs it and every assertion above
/// holds unchanged.
#[test]
fn the_same_match_survives_5_percent_loss_and_reordering() {
    let Some(path) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    run_e2e(&path, Some((0x5EED_7E57, 50, 80)));
}
