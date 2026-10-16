//! mp-match-end live tests: a local MP match ends and returns through the debriefing, by the
//! real path — the arena loads the way the confirm's handover does (`load_mp_carry`, the host's
//! own loader), the match ticks through `step_bond`'s per-tick `MP_Update` 0x184bd0, the end
//! condition fires [M `MP_CheckForEndCondition` 0x184268], and the host's handover
//! (`Sim::take_mp_debrief_request` → the `App::mp_results` shape) opens the debriefing page with
//! its board, ranks and winner line; Confirm returns to the scenario page [M C_MPDBG 0x217860:8-11]
//! and a second match can start. Opt-in via `NF_ISO`.

use nf_game::mp::debrief::{MpDebriefName, MpDebriefResult, RESULT_DRAW_LABEL, RESULT_WINS_LABEL};
use nf_game::player::Input;
use nf_view::menu::{Action, FrontEnd, Page};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Atlantis (0x07000023 [D MP_MAP_ROWS row 4]), 1 human + 1 bot, the given limits — the settings
/// `mp_confirm_start` builds and the host hands `load_mp_carry`.
fn arena_settings(frag_limit: i32, time_limit: i32) -> nf_game::mp::MpSettings {
    let mut s = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    s.frag_limit = frag_limit;
    s.time_limit = time_limit;
    s.human_count = 1;
    s.player_count = 1;
    s.bot_count = 1;
    s.total_players = 2;
    s.players[0].codename = "Player 1".into();
    s
}

/// The host handover's front end, exactly `App::mp_results`'s shape (the App itself needs a
/// window; the shape is what the task's asserts run against): the runner-state flow, the
/// front-end map requested, the MP menu state restored from the match's settings
/// [M Menu_RestoreMPSettings 0x202848], the debriefing page up with the board.
fn handover_fe(board: nf_game::mp::debrief::MpDebriefing, settings: &nf_game::mp::MpSettings) -> FrontEnd {
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![nf_view::menu::ST_RUNNER];
    fe.flow.current_level = nf_view::menu::FRONTEND_LEVEL;
    fe.restore_mp_match(settings);
    fe.show_mp_debriefing(board);
    fe.requested_level = Some(nf_view::menu::FRONTEND_LEVEL);
    fe.flow.reset_map_level_to_load(nf_view::menu::FRONTEND_LEVEL, false, false);
    fe.flow.push_state(nf_view::menu::ST_FADE_HOLD);
    fe
}

/// The frag-limit end: one frag (the scoring entry `MpMatch::player_killed`, the port's
/// MP_RegisterKill — the sim's drone-death wiring is phase 2 and will call this same fn), then
/// the real per-tick `MP_Update` walks states 1 → 3 → 4 → 5 and the request arrives.
#[test]
fn frag_limit_end_hands_over_to_the_debriefing_and_confirm_returns() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_mp_carry(&iso, "07000023.bin", false, None, arena_settings(1, -1))
        .expect("the arena loads with the match core");
    {
        let sim = scene.sim.as_ref().unwrap();
        let m = sim.mp.as_ref().expect("the match core came up with the arena");
        assert_eq!(m.settings.frag_limit, 1, "the confirm's frag limit rode the load");
        assert_eq!(m.game.state, 0, "state 0 (playing) at the start [M 0x184bd0]");
    }
    // One frag: the human (slot 0) kills the bot (slot 1).
    {
        let sim = scene.sim.as_mut().unwrap();
        let m = sim.mp.as_mut().unwrap();
        m.player_killed(1, Some(0), false, false, &mut sim.drones.rng);
    }
    // Tick the real machine until the handover request (states 1, 3, 4, 5 — a handful of ticks).
    let deadline = 600;
    let (board, settings) = (1..=deadline).find_map(|_| {
        let sim = scene.sim.as_mut().unwrap();
        let r = sim.take_mp_debrief_request();
        if r.is_some() { return r; }
        sim.step_bond(Input::default(), false, false, false);
        None
    }).unwrap_or_else(|| panic!("no debriefing handover within {deadline} ticks of the frag"));
    assert_eq!(scene.sim.as_ref().unwrap().mp.as_ref().unwrap().game.state, 5,
        "the machine waits in state 5 for the front-end load [M 0x184bd0:78]");
    // The board: "Player 1" 1st with the single frag; the winner line binds the codename.
    assert_eq!(board.rows.len(), 2, "both slots rank (the game renders at most 4) [M 0x212198:219]");
    assert_eq!(board.rows[0].slot, 0);
    assert_eq!(board.rows[0].rank, 0, "1st");
    assert_eq!(board.rows[0].name, MpDebriefName::Human("Player 1".into()));
    assert_eq!(board.rows[0].points, 1, "the frag's point [M 0x212198:178-185]");
    assert!(matches!(board.result, MpDebriefResult::Wins(_)), "a non-team game names the winner [M 0x212198:222-229]");

    // The host handover: the debriefing page with the board, ranks and winner line.
    let mut fe = handover_fe(board, &settings);
    assert_eq!(fe.page, Page::MpDebriefing, "the case-4 navigation lands on 0x40000033 [M 0x184bd0:63]");
    assert_eq!(fe.page.id(), 0x4000_0033);
    let rows = fe.mp_debriefing_rows();
    assert_eq!(rows[0].0, 0x0100_0297, "the 1st rank caption [L 0x309520]");
    let (label, arg) = fe.mp_debriefing_result();
    assert_eq!(label, RESULT_WINS_LABEL, "\"%s wins the match.\" [L USATxt]");
    assert_eq!(arg, "Player 1");
    // The level request is the host's [M 0x184bd0:64 ResetMap_LevelToLoad(0x7000048)]: the
    // drain (movie_tick) loads the front-end map and the request clears.
    assert_eq!(fe.requested_level.take(), Some(nf_view::menu::FRONTEND_LEVEL), "the front-end map 0x07000048");

    // Confirm → the scenario page [M C_MPDBG 0x217860:8-11], with the match's choices restored.
    assert!(fe.action(Action::Confirm), "the debriefing's Confirm navigates");
    assert_eq!(fe.page, Page::MpScenario, "Confirm returns to the scenario page (0x4000001a)");
    assert_eq!(fe.page.id(), 0x4000_001a);
    assert_eq!(fe.mp_level, 0x0700_0023, "the arena restored [M Menu_RestoreMPSettings 0x202848]");
    assert_eq!(fe.mp_frag_limit, 1, "the frag limit restored");
    assert!(fe.mp_join[0].joined && fe.mp_join[0].ready, "agent 1 stays joined+ready through the restore");

    // A second match can start: the confirm hands the same arena over again.
    assert!(fe.mp_confirm_start(), "the second match's confirm hands the arena over");
    assert_eq!(fe.requested_level, Some(0x0700_0023), "the arena re-requested");
    let s2 = fe.mp_match_settings.as_ref().expect("the second match's settings ride the handover");
    assert_eq!(s2.frag_limit, 1);
    assert_eq!(s2.human_count, 1, "the restored join survives into the second match");
}

/// The time-limit end (1 minute, the confirm's minimum [M the ×0x3c]): 3600 real ticks latch
/// 0xfe, the machine runs 2 → 3 → 4 → 5, and the handover opens a board whose result line is
/// the draw text (no frags — 0 vs 0).
#[test]
fn time_limit_end_hands_over_with_the_draw_line() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_mp_carry(&iso, "07000023.bin", false, None, arena_settings(-1, 60))
        .expect("the arena loads with the match core");
    let (board, _settings) = (1..=4200).find_map(|_| {
        let sim = scene.sim.as_mut().unwrap();
        let r = sim.take_mp_debrief_request();
        if r.is_some() { return r; }
        sim.step_bond(Input::default(), false, false, false);
        None
    }).expect("no debriefing handover within 4200 ticks of the 1-minute limit");
    let m = scene.sim.as_ref().unwrap().mp.as_ref().unwrap();
    assert!(m.game.time_up, "0xfe latched [M 0x184268:142-143]");
    assert_eq!(m.game.state, 5);
    assert_eq!(board.rows.len(), 2);
    let (label, arg) = match &board.result {
        MpDebriefResult::Draw => (RESULT_DRAW_LABEL, String::new()),
        other => panic!("a scoreless match must draw, got {other:?}"),
    };
    assert_eq!(label, 0x0100_02a0, "\"Match was a draw\" [L USATxt]");
    assert!(arg.is_empty());
}
