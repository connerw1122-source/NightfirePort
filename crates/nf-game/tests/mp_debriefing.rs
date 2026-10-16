//! mp-debriefing-data2: the match end + the debriefing board, headless (no ISO needed).
//!
//! The rules are the decompile's [M]: `MP_PlayerKilled` 0x188f70's credit/score/team writes,
//! `MP_CheckForEndCondition` 0x184268's latches and dispatch, `MP_Update` 0x184bd0's state
//! machine, `MP_SortOutWhoWon` 0x1848d0's status, and `P_MPDEBRIEFING_Handler` 0x212198's
//! rows/ranks/result. Each test keeps a sister assertion that can fail. Adapted from 621's
//! mp-debriefing tests to 402's API (`player_killed`'s explosive/golden_gun/rng, `update`'s rng).

use nf_game::drone::Rng;
use nf_game::mp::debrief::{
    MpDebriefName, MpDebriefResult, RANK_LABELS, RESULT_DRAW_LABEL, RESULT_WINS_LABEL,
    TEAM_MI6_LABEL, TEAM_PHOENIX_LABEL,
};
use nf_game::mp::{MpEndStatus, MpMatch, MpSettings, scenario};

/// An Arena deathmatch with humans humans + bots bots, frag limit 10, time limit 600 s.
fn arena(humans: u32, bots: u32) -> MpMatch {
    let mut s = MpSettings::deathmatch(0x0700_0024);
    s.scenario = scenario::DEATHMATCH;
    s.gametype_mask = scenario::DEATHMATCH;
    s.human_count = humans;
    s.bot_count = bots;
    s.total_players = humans + bots;
    s.frag_limit = 10;
    s.time_limit = 600;
    let mut m = MpMatch::init(s);
    m.start(&mut Rng::new(1));
    m
}

fn name_at(m: &mut MpMatch, slot: usize, s: &str) {
    m.settings.players[slot].codename = s.to_string();
}

#[test]
fn kills_land_and_the_frag_limit_ends_the_match() {
    let mut rng = Rng::new(11);
    let mut m = arena(2, 0);
    name_at(&mut m, 0, "Bond");
    name_at(&mut m, 1, "Drake");
    m.set_slot(0, [1.0, 1.6, 1.0]);
    m.set_slot(1, [9.0, 1.6, 9.0]);
    for _ in 0..10 {
        m.player_killed(1, Some(0), false, false, &mut rng);
    }
    m.update(1.0 / 60.0, false, &mut rng); // the end condition only evaluates per tick [M 0x184bd0:37-41]
    assert_eq!(m.game.slots[1].deaths, 10, "victim deaths +1 each [M 0x188f70:92-94]");
    assert_eq!(m.game.slots[0].kills, 10, "killer kills +1 [M 0x188f70:160]");
    assert_eq!(m.game.slots[0].streak, 10, "streak +1 [M 0x188f70:161-162]");
    assert_eq!(m.game.slots[0].score, 10.0, "Arena kill = +1.0 score [M 0x188f70:179-184]");
    assert_eq!(m.game.slots[1].last_killer, 0, "last-killed-by = the killer [M 0x188f70:164]");
    // 0x184268: the frag latch (0xfd) fires and the dispatch sets state 1.
    assert_eq!(m.game.state, 1, "frag limit reached -> state 1 [M 0x184268:170-184]");
    assert!(m.game.frag_latched, "switch_channels[0xfd]");
    assert!(m.game.high >= 10, "+0x18C tracks the running high score [M 0x184268:149-167]");
    // MP_Update state 1 -> MP_SortOutWhoWon -> state 3 + the status.
    m.update(1.0 / 60.0, false, &mut rng);
    assert_eq!(m.game.state, 3, "the sort moves to state 3 [M 0x1848d0:20]");
    assert_eq!(m.game.end_status, MpEndStatus::Won("Bond".into()), "the FFA exactly-one-qualifier win [M 0x1848d0:62-78]");
    // Sister: a second kill for slot 1 would NOT have ended it below the limit.
    let mut rng2 = Rng::new(12);
    let mut m2 = arena(2, 0);
    m2.player_killed(1, Some(0), false, false, &mut rng2);
    m2.update(1.0 / 60.0, false, &mut rng2);
    assert_eq!(m2.game.state, 0, "one kill stays playing");
}

#[test]
fn the_double_bonus_and_the_suicide_rules() {
    let mut rng = Rng::new(13);
    let mut m = arena(2, 0);
    m.player_killed(1, Some(0), false, true, &mut rng);
    assert_eq!(m.game.slots[0].score, 2.0, "the +2.0 weapon-bonus arm [M 0x188f70:172-177]");
    assert_eq!(m.game.slots[0].kills, 1);
    m.player_killed(0, Some(0), false, false, &mut rng); // suicide
    assert_eq!(m.game.slots[0].kills, 0, "suicide takes the kill back [M 0x188f70:153]");
    assert_eq!(m.game.slots[0].streak, 0, "streak reset [M 0x188f70:152]");
    assert_eq!(m.game.slots[0].score, 1.0, "-1.0 on the suicide [M 0x188f70:167-169,179-184]");
    assert_eq!(m.game.slots[0].last_killer, 0xffff, "last-killed-by cleared [M 0x188f70:154]");
    // Sister: no killer resolved -> nothing but the death.
    let mut m2 = arena(2, 0);
    m2.player_killed(1, None, false, false, &mut rng);
    assert_eq!(m2.game.slots[1].deaths, 1);
    assert_eq!(m2.game.slots[1].kills, 0, "no credit without a killer [M 0x188f70:150-163]");
    assert_eq!(m2.game.slots[0].score, 0.0);
}

#[test]
fn team_arena_scores_move_per_the_kill_kind() {
    let mut rng = Rng::new(14);
    let mut s = MpSettings::deathmatch(0x0700_0024);
    s.scenario = scenario::TEAM_DEATHMATCH;
    s.gametype_mask = scenario::TEAM_DEATHMATCH;
    s.frag_limit = 10;
    s.time_limit = 600;
    let mut m = MpMatch::init(s);
    m.start(&mut Rng::new(5));
    m.settings.players[0].team = 0; // Phoenix
    m.settings.players[1].team = 1; // MI6
    m.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(m.game.team_scores[0], 1.0, "the killer's team (the complement) +1 [M 0x188f70:262-271]");
    assert_eq!(m.game.team_scores[1], 0.0);
    m.player_killed(1, Some(0), false, true, &mut rng); // a Phoenix kill with the bonus
    assert_eq!(m.game.team_scores[0], 3.0, "|delta| lands on the killer's team");
    m.player_killed(0, Some(1), false, false, &mut rng); // MI6 kills Phoenix
    assert_eq!(m.game.team_scores[1], 1.0);
    m.player_killed(1, Some(0), false, false, &mut rng);
    m.settings.players[1].team = 0; // now a teamkill
    m.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(m.game.team_scores[0], 3.0, "a teamkill subtracts |delta| from the victim's own team [M 0x188f70:249-252]");
    // The team announce per the disasm order: team0 < team1 -> 0x1c8 MI6 [M 0x212198:237-241].
    m.game.team_scores = [1.0, 3.0];
    assert_eq!(m.debriefing().result, MpDebriefResult::Team(TEAM_MI6_LABEL), "team0 behind -> MI6 [M 0x212198:237-247]");
    m.game.team_scores = [3.0, 1.0];
    assert_eq!(m.debriefing().result, MpDebriefResult::Team(TEAM_PHOENIX_LABEL), "team0 ahead -> Phoenix");
    m.game.team_scores = [2.0, 2.0];
    assert_eq!(m.debriefing().result, MpDebriefResult::Draw, "equal -> the draw text");
    // Sister: Arena (not Team Arena) never moves team scores.
    let mut a = arena(2, 0);
    a.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(a.game.team_scores, [0.0, 0.0], "the team block gates on 0x20000002 [M 0x188f70:247]");
}

#[test]
fn time_up_runs_the_machine_to_the_debriefing_request() {
    let mut rng = Rng::new(15);
    let mut m = arena(2, 0);
    m.settings.time_limit = 60; // seconds [M the confirm's x0x3c]
    m.game.limit = 60.0;
    for _ in 0..60 {
        m.update(1.0, false, &mut rng);
    }
    assert!(m.game.time_up, "0xfe latched [M 0x184268:142-143]");
    assert_eq!(m.game.state, 2, "the final dispatch sets 2 [M 0x184268:173-184]");
    m.update(1.0, false, &mut rng); // state 2 -> 3 (non-0x10 passes the gate)
    assert_eq!(m.game.state, 3);
    assert_eq!(m.game.end_status, MpEndStatus::None, "the sort never ran — the Game Over text is case 2's own [M 0x184bd0:42-50]");
    m.update(1.0, false, &mut rng); // state 3 -> 4 + GameState+0x24
    assert!(m.game.handover_flag, "GameState+0x24 = 1 [M 0x184bd0:59]");
    m.update(1.0, false, &mut rng); // state 4 -> 5 + the request
    assert!(m.game.request_debriefing, "the 0x40000033 navigation [M 0x184bd0:62-65]");
    assert_eq!(m.game.state, 5);
    // Sister: an unexpired match never leaves state 0.
    let mut m2 = arena(2, 0);
    m2.update(1.0, false, &mut rng);
    assert_eq!(m2.game.state, 0);
}

#[test]
fn demolition_restarts_instead_of_ending() {
    let mut rng = Rng::new(16);
    let mut s = MpSettings::deathmatch(0x0700_0024);
    s.scenario = scenario::DEMOLITION;
    s.gametype_mask = scenario::DEMOLITION;
    s.team_game = 1;
    s.frag_limit = -1;
    s.time_limit = 60;
    let mut m = MpMatch::init(s);
    m.start(&mut Rng::new(2));
    for _ in 0..61 {
        m.update(1.0, false, &mut rng);
    }
    assert_eq!(m.game.state, 6, "the demo arm jumps to 6 [M 0x184268:57-60]");
    assert!(!m.game.time_up, "0xfe is NOT latched on the demo arm [M 0x184268:57-60 — decomp lines 57-59 write only MPGame+0x392]");
    for _ in 0..3 {
        m.update(1.0, false, &mut rng);
    }
    assert!(!m.game.restart_pending, "timer 4.0 s — not past the gate [M 0x184bd0:85: 5.0 < timer, strictly]");
    m.update(1.0, false, &mut rng);
    m.update(1.0, false, &mut rng);
    assert!(m.game.restart_pending, "MP_RestartScenario after 5 s [M 0x184bd0:79-92]");
    assert_eq!(m.game.state, 0, "back to playing [M 0x184bd0:86-87] (the host re-inits from here)");
    // Sister: Protection shares the tail.
    let mut p = MpSettings::deathmatch(0x0700_0024);
    p.scenario = scenario::PROTECTION;
    p.time_limit = 60;
    let mut m2 = MpMatch::init(p);
    m2.start(&mut Rng::new(3));
    for _ in 0..61 {
        m2.update(1.0, false, &mut rng);
    }
    assert_eq!(m2.game.state, 6);
}

#[test]
fn top_agent_elimination_and_the_fewest_deaths_sort() {
    let mut rng = Rng::new(17);
    let mut s = MpSettings::deathmatch(0x0700_0024);
    s.scenario = scenario::DEATH_LIMIT_FFA;
    s.human_count = 2;
    s.bot_count = 1;
    s.total_players = 3;
    s.frag_limit = 3; // the lives limit
    s.time_limit = -1;
    let mut m = MpMatch::init(s);
    m.start(&mut Rng::new(4));
    for i in 0..3 {
        m.set_slot(i, [1.0, 1.6, 1.0]);
    }
    // Slot 0 dies out (the death path marks the slot dead — the state-byte stand-in).
    for _ in 0..3 {
        m.player_killed(0, Some(1), false, false, &mut rng);
    }
    m.update(1.0 / 60.0, false, &mut rng);
    assert_eq!(m.game.state, 0, "one human out of two keeps playing [M 0x184268:120-123]");
    // Slot 1 dies out too -> only the bot remains -> state 1.
    for _ in 0..3 {
        m.player_killed(1, Some(0), false, false, &mut rng);
    }
    m.update(1.0 / 60.0, false, &mut rng);
    assert_eq!(m.game.state, 1, "total-1 <= outs ends it [M 0x184268:120-123]");
    // The sort: slots 0/1 sit at the lives limit (skipped), the bot survives with 0 deaths.
    m.settings.players[2].codename = "Rook".into(); // the bot's full name rides the same field
    m.sort_out_who_won();
    assert_eq!(m.game.end_status, MpEndStatus::Won("Rook".into()), "the lone survivor wins [M 0x1848d0:33-37]");
    // Sister: the tie quirk — the survivor count re-increments after a tie, so a tie only
    // forces the draw when NOTHING later qualifies (the last qualifier was the tie)
    // [M 0x1848d0:32-40: count++ happens before the compare; the zeroing survives only if no
    // later slot passes]. [3,3,1] therefore still crowns the strict winner:
    let mut m2 = arena(3, 0);
    m2.settings.scenario = scenario::DEATH_LIMIT_FFA;
    m2.settings.frag_limit = 10;
    m2.settings.players[2].codename = "C".into();
    for i in 0..3 {
        m2.set_slot(i, [1.0, 1.6, 1.0]);
        m2.game.slots[i].deaths = [3, 3, 1][i];
    }
    m2.sort_out_who_won();
    assert_eq!(m2.game.end_status, MpEndStatus::Won("C".into()), "the count re-increments after the tie [M 0x1848d0:32-40]");
    // ... while a tie as the LAST qualifier is the draw:
    let mut m3 = arena(3, 0);
    m3.settings.scenario = scenario::DEATH_LIMIT_FFA;
    m3.settings.frag_limit = 10;
    for i in 0..3 {
        m3.set_slot(i, [1.0, 1.6, 1.0]);
        m3.game.slots[i].deaths = [3, 3, 10][i]; // slot 2 sits at the limit — skipped
    }
    m3.sort_out_who_won();
    assert_eq!(m3.game.end_status, MpEndStatus::Draw, "the tie at the minimum ends the scan [M 0x1848d0:38-40,49-52]");
    // And a strict single minimum wins.
    let mut m3 = arena(2, 0);
    m3.settings.scenario = scenario::DEATH_LIMIT_FFA;
    m3.settings.frag_limit = 10;
    for i in 0..2 {
        m3.set_slot(i, [1.0, 1.6, 1.0]);
        m3.settings.players[i].codename = ["A", "B"][i].to_string();
        m3.game.slots[i].deaths = [5, 2][i];
    }
    m3.sort_out_who_won();
    assert_eq!(m3.game.end_status, MpEndStatus::Won("B".into()), "fewest deaths [M 0x1848d0:33-37]");
}

#[test]
fn the_debriefing_board_sorts_ranks_and_caps_at_four() {
    let mut m = arena(2, 2);
    // Slots 0-1 humans, 4-5 bots. TOTAL = kills in Arena. The kills/scores are written
    // directly (the credit rules themselves are the tests above).
    m.settings.players[0].codename = "Bond".into();
    m.settings.players[1].codename = "Drake".into();
    m.settings.players[4].skin = 1;
    m.settings.players[5].skin = 2;
    m.game.slots[0].kills = 8;
    m.game.slots[1].kills = 8;
    m.game.slots[4].kills = 3;
    m.game.slots[5].kills = 1;
    m.game.slots[0].score = 8.0;
    m.game.slots[1].score = 8.0;
    m.game.slots[4].score = 3.0;
    m.game.slots[5].score = 1.0;
    let b = m.debriefing();
    assert_eq!(b.rows.len(), 4, "four rows render (the cap) [decomp line 219]");
    // The sort is by TOTAL (Menu_GetMPScore): the two 8-kill slots first (stable: slot 0 built
    // before slot 1), then the bots by their kills.
    assert_eq!(b.rows[0].slot, 0);
    assert_eq!(b.rows[1].slot, 1);
    assert_eq!(b.rows[2].slot, 4);
    assert_eq!(b.rows[3].slot, 5);
    // Tied totals share the rank [M 0x212198:131-143].
    assert_eq!(&b.rows[0].rank, &b.rows[1].rank, "ties share the rank");
    assert_eq!(RANK_LABELS[0], 0x0100_0297, "\"1st\" [L 0x309520]");
    assert_eq!(b.rows[0].rank, 0);
    assert_eq!(b.rows[2].rank, 2, "the third distinct score is rank 2 (0-based) [M 0x212198:135-137]");
    // Names: humans carry codenames, bots the short-name character [M 0x212198:167-177].
    assert_eq!(b.rows[0].name, MpDebriefName::Human("Bond".into()));
    assert_eq!(b.rows[2].name, MpDebriefName::Bot(1));
    assert_eq!(MpMatch::bot_short_name_label(1), Some(0x0100_02e3), "0x10002e2+id [M 0x201918]");
    assert_eq!(MpMatch::bot_short_name_label(0x1d), None, ">= 0x1d is the literal Bot [M 0x201918:97]");
    // The columns [M 0x212198:178-204].
    assert_eq!(b.rows[0].points, 8);
    assert_eq!(b.rows[0].victories, 8);
    assert_eq!(b.rows[0].deaths, 0);
    assert_eq!(b.rows[0].total, 8);
    // The result: the top two tie -> the draw [M 0x212198:222-229].
    assert_eq!(b.result, MpDebriefResult::Draw);
    // Sister: break the tie and the winner's codename shows.
    m.game.slots[0].kills = 9;
    m.game.slots[0].score = 9.0;
    assert_eq!(m.debriefing().result, MpDebriefResult::Wins("Bond".into()));
}

#[test]
fn get_mp_score_follows_the_scenario_table() {
    let mut m = arena(1, 1);
    m.game.slots[0].kills = 7;
    m.game.slots[0].score = 12.0;
    for (word, want) in [
        (scenario::DEATHMATCH, 7),
        (scenario::TEAM_DEATHMATCH, 7),
        (scenario::DEATH_LIMIT_FFA, 12),
        (scenario::CAPTURE_THE_FLAG, 12),
        (scenario::UPLINK, 12),
        (scenario::KOTH_FFA, 12),
        (scenario::KOTH_TEAM, 12),
        (scenario::DEMOLITION, 12),
        (scenario::PROTECTION, 12),
        (scenario::BLUEPRINT, 12),
        (scenario::GOLDENEYE, 12),
        (scenario::ASSASSIN, 12),
        (0, 0),
    ] {
        m.settings.scenario = word;
        m.settings.gametype_mask = word;
        assert_eq!(m.get_mp_score(0), want, "scenario {word:#x} [M 0x203af8]");
    }
}

#[test]
fn a_single_player_board_is_a_win_not_a_draw() {
    let mut m = arena(1, 0);
    m.settings.players[0].codename = "Bond".into();
    m.game.slots[0].kills = 3;
    m.game.slots[0].score = 3.0;
    let b = m.debriefing();
    assert_eq!(b.rows.len(), 1);
    assert_eq!(b.result, MpDebriefResult::Wins("Bond".into()), "*Deviation*: the game compares an uninitialized pair [M 0x212198:222-229]");
}

#[test]
fn the_ffa_qualifier_loop_has_no_obj_gate() {
    // MP_SortOutWhoWon's FFA branch reads only the slot scores — the decomp has no `obj != 0`
    // test there [M 0x1848d0:56-71]. A slot with the limit reached qualifies even though its
    // position was never registered (402 filtered on pos; mp-debriefing-data2 removed it).
    let mut m = arena(2, 0);
    m.game.slots[1].score = 10.0; // frag limit 10; slot 1 has no pos
    assert_eq!(m.sort_out_who_won(), nf_game::mp::Winner::Slot(1), "the score-only qualifier [M 0x1848d0:62-78]");
    // Sister: two qualifiers are the draw even without positions.
    let mut m2 = arena(2, 0);
    m2.game.slots[0].score = 10.0;
    m2.game.slots[1].score = 11.0;
    assert_eq!(m2.sort_out_who_won(), nf_game::mp::Winner::Draw, "0 or >=2 qualifiers draw [M 0x1848d0:79-84]");
}
