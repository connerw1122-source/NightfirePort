//! 402 mp-match-rules — headless per-scenario matches reaching the game's win conditions with the
//! right scores (numbers only). Every test pairs its rule assertion with a sister assertion that
//! would fail if the rule were wrong (the docs-first testing rule). Rules and addresses:
//! `docs/systems/mp-match-rules.md`; the decomp files carry the [M] tags.

use nf_game::drone::Rng;
use nf_game::mp::{scenario, MpMatch, MpSettings, Winner};

const DT: f32 = 1.0 / 60.0;

fn make(scenario: u32, frag: i32, time_min: i32) -> MpMatch {
    let mut s = MpSettings::deathmatch(0x07000011);
    s.scenario = scenario;
    s.frag_limit = frag;
    s.time_limit = time_min;
    let mut m = MpMatch::init(s);
    m.start(&mut Rng::new(1));
    m
}

fn join(m: &mut MpMatch, slot: usize, team: u32) {
    m.settings.players[slot].team = team;
    m.set_slot(slot, [10.0 + 10.0 * slot as f32, 0.0, 0.0]);
}

fn tick(m: &mut MpMatch, frames: u32, rng: &mut Rng) {
    for _ in 0..frames {
        m.update_objects(DT, &[false; 8], rng);
        m.update(DT, false, rng);
    }
}

/// Deathmatch reaches the frag limit with the right scores and picks the winner [M 0x184268:170-184,
/// 0x1848d0:55-86]. Sister: the self-kill numbers (−1 score, kills −1) fail if the sign were wrong.
#[test]
fn deathmatch_frag_limit_and_self_kill() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::DEATHMATCH, 3, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 0);
    m.player_killed(1, Some(0), false, false, &mut rng);
    m.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(m.game.slots[0].score, 2.0);
    assert_eq!(m.game.state, 0, "under the limit the match keeps playing");
    m.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(m.game.slots[0].score, 3.0);
    assert_eq!(m.game.slots[0].kills, 3);
    assert_eq!(m.game.slots[0].streak, 3);
    assert_eq!(m.game.slots[1].deaths, 3);
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.game.state, 1, "the frag latch enters state 1 [M 0x184268:173-184]");
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.game.state, 3, "state 1 runs SortOutWhoWon into 3 [M 0x184bd0:51-52]");
    assert_eq!(m.sort_out_who_won(), Winner::Slot(0));
    // sister: a self-kill costs a kill and a point, and clears the last killer [M 0x188f70:151-155]
    let mut m2 = make(scenario::DEATHMATCH, 20, -1);
    join(&mut m2, 0, 0);
    m2.player_killed(0, Some(0), false, false, &mut rng);
    assert_eq!(m2.game.slots[0].score, -1.0);
    assert_eq!(m2.game.slots[0].kills, -1);
    assert_eq!(m2.game.slots[0].last_killer, 0xffff);
    assert_eq!(m2.game.state, 0, "a self-kill never ends the match");
}

/// Team Arena: enemy kills feed the killer's team, team-kills and self-kills feed the victim's
/// team [M 0x188f70:246-274]. Sister: the same enemy kill in Deathmatch leaves the team scores 0.
#[test]
fn team_arena_team_scores() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::TEAM_DEATHMATCH, 10, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 1);
    join(&mut m, 2, 0);
    m.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(m.game.team_scores, [1.0, 0.0]);
    assert_eq!(m.game.slots[0].score, 1.0, "the personal score still lands (bit30 = 0)");
    m.player_killed(2, Some(0), false, false, &mut rng); // a team-kill
    assert_eq!(m.game.team_scores, [0.0, 0.0], "the team-kill subtracts |Δ| from the victim's team");
    assert_eq!(m.game.slots[0].score, 2.0, "the team-kill still added the personal point [M :179-184]");
    assert_eq!(m.game.slots[0].kills, 1, "no +kills for a team-kill [M :157-163]");
    m.player_killed(0, Some(0), false, false, &mut rng); // self-kill: the team pays 1
    assert_eq!(m.game.team_scores, [-1.0, 0.0]);
    // sister: the FFA scenario has no team-score effect at all
    let mut ffa = make(scenario::DEATHMATCH, 10, -1);
    join(&mut ffa, 0, 0);
    join(&mut ffa, 1, 0);
    ffa.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(ffa.game.team_scores, [0.0, 0.0]);
}

/// CTF: an enemy picks up the flag and carrying it to the enemy base scores team +1 and carrier
/// +1, the flag resets home [M 0x1861a8:74-112]. Sister: an own-team touch does not pick up, and
/// standing short of the base scores nothing.
#[test]
fn ctf_capture_scores_team_and_carrier() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::CAPTURE_THE_FLAG, 5, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 1);
    // register flag 0 (team 0's) and the two bases (type 0/1, team in config word 1)
    m.register_mp_object([30.0, 1.0, 30.0], [0.0, 0.0, 1.0], 0, [0, 0, 0, 0, 0, 0, 0], 0);
    m.register_mp_object([1.0, 1.0, 1.0], [0.0, 0.0, 1.0], 1, [1, 0, 0, 0, 0, 0, 0], 0);
    m.register_mp_object([31.0, 1.0, 31.0], [0.0, 0.0, 1.0], 1, [1, 1, 0, 0, 0, 0, 0], 0);
    m.create_mode_objects(&mut rng);
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().state, 0);
    m.touch_flag(0); // team 0 touching its own flag: nothing
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().state, 0, "own-team touch never picks up");
    m.touch_flag(1); // team 1 takes flag 0
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().state, 1);
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().carrier, Some(1));
    assert_eq!(m.game.slots[1].status_bits & 1, 1, "the carrying bit sets [M 0x18a518:88-96]");
    // carry it toward team 1's base at (31,1,31) — first short of the capture radius
    m.set_slot(1, [33.0, 0.0, 33.0]);
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.game.team_scores, [0.0, 0.0], "short of the base nothing scores");
    m.set_slot(1, [31.0, 1.0, 31.0]); // the capture point (base +1.0 y), sq-dist 0 < 2.0
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.game.team_scores, [0.0, 1.0], "the taker's team scores [M 0x1861a8:89-93]");
    assert_eq!(m.game.slots[1].score, 1.0);
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().state, 0, "the flag resets home");
    assert_eq!(m.game.slots[1].status_bits & 1, 0, "the carrying bit clears");
}

/// CTF: a dropped flag returns home after 30 s of frames, or ~2 frames when the carrier died to an
/// explosion, or at once on a teammate's touch [M 0x1861a8:195-259, 0x188f70:276-299].
#[test]
fn ctf_flag_drop_and_return() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::CAPTURE_THE_FLAG, 20, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 1);
    m.register_mp_object([30.0, 1.0, 30.0], [0.0, 0.0, 1.0], 0, [0, 0, 0, 0, 0, 0, 0], 0);
    m.create_mode_objects(&mut rng);
    m.touch_flag(1); // team 1 takes flag 0
    m.player_killed(1, Some(0), false, false, &mut rng); // a normal kill drops it
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().state, 2, "the flag drops on the carrier's death");
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().carrier, None);
    // sister: the teammate touch returns the dropped flag at once [M 0x1861a8:195-221]
    m.touch_flag(0);
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().state, 0, "a teammate's touch returns it");
    // again, and let the 30 s timer run it home (1801 frames)
    m.touch_flag(1);
    m.player_killed(1, Some(0), true, false, &mut rng); // the explosive kill preloads the counter
    tick(&mut m, 2, &mut rng);
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().state, 0, "an explosive drop returns in ~2 frames");
    // a normal drop needs the full return timer (the game's pre-increment check expires on the
    // 1802nd frame: t = 1801 > 30*60 [M 0x1861a8:226-259])
    m.touch_flag(1);
    m.player_killed(1, Some(0), false, false, &mut rng);
    tick(&mut m, 1801, &mut rng);
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().state, 2, "still dropped at 1801 frames");
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.objects_rt.flags[0].as_ref().unwrap().state, 0, "returned after 30 s");
}

/// Uplink: a claimed terminal drips 0.2/s to the team score (= Σ personals) and the controller's
/// personal score; an enemy touch flips it at once [M 0x186d98:24-114].
#[test]
fn uplink_drip_flip_and_team_sum() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::UPLINK, 3, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 1);
    m.register_mp_object([5.0, 0.0, 5.0], [0.0, 0.0, 1.0], 2, [2, 0, 0, 0, 0, 0, 0], 0);
    m.create_mode_objects(&mut rng);
    tick(&mut m, 60, &mut rng);
    assert_eq!(m.game.team_scores, [0.0, 0.0], "a neutral terminal scores nothing");
    m.touch_uplink(0, 0); // team 0 claims it
    assert_eq!(m.objects_rt.uplinks[0].state, 0);
    tick(&mut m, 300, &mut rng); // 5 s at 0.2/s
    assert!((m.game.slots[0].score - 1.0).abs() < 1e-4, "the controller drips 0.2/s for 5 s");
    assert!((m.game.team_scores[0] - 1.0).abs() < 1e-4, "the team score = Σ personals [M :66-80]");
    m.touch_uplink(0, 1); // the enemy flips it at once [M :24-52]
    assert_eq!(m.objects_rt.uplinks[0].state, 1);
    assert_eq!(m.objects_rt.uplinks[0].controller, 1);
    let before = m.game.slots[0].score;
    tick(&mut m, 60, &mut rng);
    assert!((m.game.slots[0].score - before).abs() < 1e-6, "the flipped terminal no longer pays slot 0");
    // and the drip reaches the frag limit → state 1 (15 s at 0.2/s plus a margin — the game's
    // per-frame 0.2/60 f32 accumulation truncates at the (int) cast in the high-score compare);
    // the ticks keep running, so the latch has already carried the match through the end machine
    // (1 → SortOutWhoWon → 3 → 4 → 5)
    tick(&mut m, 300 * 3 + 60, &mut rng);
    assert!(m.game.frag_latched, "the frag latch fired on the drip [M 0x184268:170-184]");
    assert!(m.game.state >= 3, "the end machine ran past the who-won state");
}

/// Top Agent (0x10): deaths are lives; the round ends in state 1 when all but one are out; the
/// winner is the unique fewest-deaths survivor; out players cannot respawn [M 0x184268:85-135,
/// 0x1848d0:21-54, 0x185368:16-20]. Sister: kills never add score in 0x10.
#[test]
fn top_agent_lives_round_end_and_no_kill_score() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::DEATH_LIMIT_FFA, 2, -1);
    m.settings.total_players = 3;
    m.settings.player_count = 3;
    join(&mut m, 0, 0);
    join(&mut m, 1, 0);
    join(&mut m, 2, 0);
    m.player_killed(1, Some(0), false, false, &mut rng);
    assert!((m.game.slots[1].score + 1.0).abs() < 1e-6, "the lives copy drops per death [M 0x188f70:96-98]");
    assert_eq!(m.game.slots[0].score, 0.0, "kills never score in 0x10 (not one of the four)");
    assert!(m.respawn_allowed(1), "still alive after 1 death of 2");
    m.player_killed(1, None, false, false, &mut rng);
    m.player_killed(2, None, false, false, &mut rng);
    m.player_killed(2, None, false, false, &mut rng);
    assert!(!m.respawn_allowed(1), "2 deaths of a 2-life limit = out [M 0x185368:16-20]");
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.game.state, 1, "(total − 1) out ends the round [M 0x184268:103-135]");
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.game.state, 3, "0x10's time-up state falls through to SortOutWhoWon [M 0x184bd0:42-52]");
    assert_eq!(m.sort_out_who_won(), Winner::Slot(0), "the unique survivor with the fewest deaths wins");
    // sister: a tie at the minimum is a draw [M 0x1848d0:38-40]
    let mut tie = make(scenario::DEATH_LIMIT_FFA, 2, -1);
    tie.settings.total_players = 2;
    tie.settings.player_count = 2;
    join(&mut tie, 0, 0);
    join(&mut tie, 1, 0);
    tie.player_killed(0, Some(1), false, false, &mut rng);
    tie.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(tie.sort_out_who_won(), Winner::Draw);
}

/// Demolition: destroying the site scores the attacking team (team 1) and the last hitter, and
/// enters state 6; the defender destroying their own site is an own-goal; the 5 s restart resets
/// the round, not the scores [M 0x1870a0:125-165, 0x183fc0:40-82].
#[test]
fn demolition_destroy_own_goal_and_restart() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::DEMOLITION, 5, -1);
    join(&mut m, 0, 0); // the defender team (0)
    join(&mut m, 1, 1); // the attacker team
    m.register_mp_object([5.0, 0.0, 5.0], [0.0, 0.0, 1.0], 3, [3, 0, 0, 0, 0, 0, 0], 0);
    m.create_mode_objects(&mut rng);
    assert_eq!(m.objects_rt.site.as_ref().unwrap().state, 2000, "the HP init [M 0x185de8:174]");
    m.damage_site(1500.0, 1);
    assert_eq!(m.objects_rt.site.as_ref().unwrap().state, 500);
    assert_eq!(m.game.state, 0, "still standing");
    m.damage_site(1000.0, 1);
    assert_eq!(m.game.team_scores, [0.0, 1.0], "the attacking team scores [M 0x1870a0:125-132]");
    assert_eq!(m.game.slots[1].score, 1.0, "the last hitter's personal point");
    assert_eq!(m.game.state, 6, "the round is over at once [M :160]");
    tick(&mut m, 300, &mut rng); // the 5 s restart timer [M 0x184bd0:79-92]
    assert_eq!(m.game.state, 0, "back to playing");
    assert_eq!(m.game.team_scores, [0.0, 1.0], "the restart keeps the scores (multi-round)");
    assert_eq!(m.objects_rt.site.as_ref().unwrap().state, 2000, "the site HP re-inits");
    assert_eq!(m.game.slots[0].last_killer, 0xffff, "the per-slot words reset [M 0x183fc0:55-82]");
    // sister: the defender destroying their own site is an own-goal [M 0x1870a0:133-140]
    let mut og = make(scenario::DEMOLITION, 5, -1);
    join(&mut og, 0, 0);
    join(&mut og, 1, 1);
    og.register_mp_object([5.0, 0.0, 5.0], [0.0, 0.0, 1.0], 3, [3, 0, 0, 0, 0, 0, 0], 0);
    og.create_mode_objects(&mut rng);
    og.damage_site(3000.0, 0);
    assert_eq!(og.game.team_scores, [-1.0, 1.0]);
    assert_eq!(og.game.slots[0].score, 0.0, "the own-goal hands the personal point back");
}

/// Protection: the round timeout scores the **defending** team (1) and enters state 6 [M
/// 0x1870a0:112-124, 0x184268:57-60]. Sister: before the timeout nothing scores.
#[test]
fn protection_timeout_scores_defenders() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::PROTECTION, 5, -1); // the limit defaults to 60 s [M 0x183438:51-54]
    join(&mut m, 0, 0);
    join(&mut m, 1, 1);
    m.register_mp_object([5.0, 0.0, 5.0], [0.0, 0.0, 1.0], 8, [8, 1, 0, 0, 0, 0, 0], 0);
    m.create_mode_objects(&mut rng);
    tick(&mut m, 60 * 59, &mut rng);
    assert_eq!(m.game.team_scores, [0.0, 0.0], "no score before the timeout");
    assert_eq!(m.game.state, 0);
    tick(&mut m, 60 * 2, &mut rng);
    assert_eq!(m.game.team_scores, [0.0, 1.0], "the defending team scores on the timeout");
    assert_eq!(m.game.state, 6, "the round-over state [M 0x184268:58-60]");
    assert_eq!(m.objects_rt.site.as_ref().unwrap().sub, 1, "the site is done");
}

/// GoldenEye Strike: both items carried by one team launch the strike (+1 each carrier, a random
/// opposing target); the completed strike kills the target and scores the opposite team [M
/// 0x187568:356-384, 96-179]. Sister: items carried by different teams never launch.
#[test]
fn goldeneye_strike_launch_and_complete() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::GOLDENEYE, 10, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 0);
    join(&mut m, 2, 1);
    m.register_mp_object([1.0, 0.0, 1.0], [0.0, 0.0, 1.0], 6, [6, 0, 0, 0, 0, 0, 0], 0);
    m.register_mp_object([2.0, 0.0, 2.0], [0.0, 0.0, 1.0], 7, [7, 0, 0, 0, 0, 0, 0], 0);
    m.create_mode_objects(&mut rng);
    m.touch_ge_item(0, 0);
    m.touch_ge_item(1, 2); // different teams: no launch
    tick(&mut m, 2, &mut rng);
    assert_eq!(m.objects_rt.ge_target, None, "mixed carriers never launch [M :356-367]");
    m.player_killed(2, Some(0), false, false, &mut rng); // the mixed carrier dies — the item drops
    assert_eq!(m.objects_rt.ge[1].as_ref().unwrap().state, 2);
    m.touch_ge_item(1, 1); // slot 1 (team 0) picks the dropped item up
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.objects_rt.ge[0].as_ref().unwrap().state, 3, "the strike is in flight");
    assert_eq!(m.game.slots[0].score, 1.0, "each carrier +1 [M :368-377]");
    assert_eq!(m.game.slots[1].score, 1.0);
    let target = m.objects_rt.ge_target.expect("a random opposing target");
    assert_eq!(target, 2, "the only team-1 slot");
    m.ge_strike_complete(&mut rng);
    assert_eq!(m.game.team_scores, [1.0, 0.0], "the opposite team scores [M :110-119]");
    assert_eq!(m.game.slots[2].deaths, 2, "the mixed-carrier kill plus the strike kill (no credit)");
    assert!(m.game.slots[2].dead, "the strike kills the target");
    assert_eq!(m.objects_rt.ge[0].as_ref().unwrap().state, 0, "the items respawn free");
    // sister: the dropped item respawns at a pool spawn after 30 s [M :187-228]
    m.touch_ge_item(0, 0);
    m.player_killed(0, Some(2), false, false, &mut rng);
    assert_eq!(m.objects_rt.ge[0].as_ref().unwrap().state, 2);
    tick(&mut m, 1802, &mut rng);
    assert_eq!(m.objects_rt.ge[0].as_ref().unwrap().state, 0);
}

/// Industrial Espionage: the carrier reaching their own team's base scores team +1 and carrier +1
/// and the blueprint respawns [M 0x1882d8:50-79, 0x188970:27-55]. Sister: short of the base
/// nothing scores, and the death drop resets to free after 30 s.
#[test]
fn blueprint_delivery_and_drop() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::BLUEPRINT, 5, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 1);
    m.register_mp_object([1.0, 0.0, 1.0], [0.0, 0.0, 1.0], 5, [5, 0, 0, 0, 0, 0, 0], 0);
    m.register_mp_object([40.0, 0.0, 40.0], [0.0, 0.0, 1.0], 4, [4, 0, 0, 0, 0, 0, 0], 0);
    m.register_mp_object([50.0, 0.0, 50.0], [0.0, 0.0, 1.0], 4, [4, 1, 0, 0, 0, 0, 0], 0);
    m.create_mode_objects(&mut rng);
    m.touch_blueprint(0);
    m.set_slot(0, [45.0, 0.0, 45.0]); // short of team 0's base at (40,0,40)
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.game.team_scores, [0.0, 0.0], "short of the base nothing scores");
    m.set_slot(0, [40.0, 0.0, 40.0]);
    tick(&mut m, 1, &mut rng);
    assert_eq!(m.game.team_scores, [1.0, 0.0], "the carrier's team scores [M 0x188970:27-30]");
    assert_eq!(m.game.slots[0].score, 1.0);
    assert_eq!(m.objects_rt.blueprint.as_ref().unwrap().state, 0, "the blueprint respawns free");
    // sister: the death drop resets to free where it lies after 30 s [M 0x1882d8:147-180]
    m.touch_blueprint(1);
    m.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(m.objects_rt.blueprint.as_ref().unwrap().state, 2);
    tick(&mut m, 1802, &mut rng);
    assert_eq!(m.objects_rt.blueprint.as_ref().unwrap().state, 0);
    assert_eq!(m.objects_rt.blueprint.as_ref().unwrap().carrier, None);
}

/// King of the Hill: every alive player on the hill drips 0.2/s personally (everyone at once); in
/// the team game the scoring player's team score = Σ that team's personals [M 0x186ae0:78-151].
/// Sister: a dead player on the hill scores nothing.
#[test]
fn koth_drip_ffa_and_team() {
    let mut rng = Rng::new(7);
    let mut ffa = make(scenario::KOTH_FFA, 3, -1);
    join(&mut ffa, 0, 0);
    join(&mut ffa, 1, 0);
    let hill = [true, true, false, false, false, false, false, false];
    for _ in 0..300 {
        ffa.update_objects(DT, &hill, &mut rng);
        ffa.update(DT, false, &mut rng);
    }
    assert!((ffa.game.slots[0].score - 1.0).abs() < 1e-4, "0.2/s for 5 s on the hill");
    assert!((ffa.game.slots[1].score - 1.0).abs() < 1e-4, "everyone on the hill scores at once");
    let dead = [true, false, false, false, false, false, false, false];
    ffa.game.slots[1].dead = true;
    ffa.game.slots[1].score = 0.0;
    for _ in 0..60 {
        ffa.update_objects(DT, &dead, &mut rng);
        ffa.update(DT, false, &mut rng);
    }
    assert!(ffa.game.slots[1].score.abs() < 1e-5, "the dead do not score [M 0x186ae0:86]");
    // the team game recomputes the team score as Σ personals
    let mut team = make(scenario::KOTH_TEAM, 3, -1);
    join(&mut team, 0, 0);
    join(&mut team, 1, 1);
    let one = [true, false, false, false, false, false, false, false];
    for _ in 0..300 {
        team.update_objects(DT, &one, &mut rng);
        team.update(DT, false, &mut rng);
    }
    assert!((team.game.team_scores[0] - 1.0).abs() < 1e-4, "the hill team's score = Σ personals [M :122-137]");
    assert_eq!(team.game.team_scores[1], 0.0);
}

/// Assassination (0x400): the assassin killing the target +5.0; the target killing the assassin +3.0
/// and the roles reset (the killer inherits the assassin role) [M 0x188f70:229-243, 0x18aa70]. 
/// Sister: any other kill scores nothing in 0x400.
#[test]
fn assassin_scores_and_role_reset() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::ASSASSIN, 20, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 0);
    join(&mut m, 2, 0);
    m.assassin = Some(0);
    m.assassin_target = Some(1);
    m.settings.players[0].team = 1; // the assassin's marker [M 0x18aa70:38-41]
    m.player_killed(1, Some(0), false, false, &mut rng);
    assert_eq!(m.game.slots[0].score, 5.0, "the assassin's target kill +5 [M 0x188f70:229-233]");
    assert_eq!(m.game.slots[0].kills, 1, "the kill itself still counts");
    // a non-target kill scores nothing (sister)
    m.player_killed(2, Some(0), false, false, &mut rng);
    assert_eq!(m.game.slots[0].score, 5.0);
    // the target kills the assassin: +3 and the roles reset with the killer as the new assassin
    m.player_killed(0, Some(1), false, false, &mut rng);
    assert_eq!(m.game.slots[1].score, 3.0, "the target's revenge +3 [M :234-242]");
    assert_eq!(m.assassin, Some(1), "the killer inherits the assassin role [M 0x18aa70:24-28]");
    assert_ne!(m.assassin_target, Some(1), "the new target is somebody else");
    assert_eq!(m.settings.players[1].team, 1, "the marker follows the assassin");
}

/// The friendly-fire gate of `MP_RegisterBulletHit` 0x1857c0:33-57 — FF off blocks same-team hits
/// (with the 3 s warn timer when on), enemy and FFA hits always allowed. Sister: the FFA scenario
/// never gates.
#[test]
fn friendly_fire_gate() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::TEAM_DEATHMATCH, 10, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 0);
    join(&mut m, 2, 1);
    assert!(!m.register_bullet_hit(1, 0), "FF off blocks the same-team hit [M 0x1857c0:43-45]");
    assert!(m.register_bullet_hit(2, 0), "enemy hits are allowed");
    assert_eq!(m.game.slots[1].status, 0xfffe, "the blocked hit never wrote the last-shooter word");
    assert_eq!(m.game.slots[2].status, 0, "the allowed hit records the attacker on the victim [M :34]");
    m.settings.friendly_fire = 1;
    assert!(m.register_bullet_hit(1, 0), "FF on allows the same-team hit");
    assert_eq!(m.game.slots[1].timers[0], 180, "the 3 s warn cooldown sets [M :56]");
    // the FFA scenario never gates
    let mut ffa = make(scenario::DEATHMATCH, 10, -1);
    join(&mut ffa, 0, 0);
    join(&mut ffa, 1, 0);
    assert!(ffa.register_bullet_hit(1, 0), "FFA same-team hits are always allowed");
    assert_eq!(ffa.game.slots[1].timers[0], 0, "no warn timer outside team games");
}

/// The respawn rules of `MP_ReSpawn` 0x185368: health = armor_bonus + 100 [M :65-73], the per-slot
/// reset, and the state gate. Sister: a respawn in the round-over state is refused.
#[test]
fn respawn_health_and_gates() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::DEATHMATCH, 10, -1);
    m.settings.players[0].armor_bonus = 50;
    join(&mut m, 0, 0);
    join(&mut m, 1, 0);
    m.register_spawn_point([3.0, 5.0, 3.0], [0.0, 0.0, 1.0], 0, &nf_game::world::CollisionWorld::from_triangles(vec![]));
    m.player_killed(0, Some(1), false, false, &mut rng);
    assert!(m.respawn_allowed(0));
    let idx = m.get_spawn_point(0, 0, &mut rng);
    let hp = m.do_respawn(0, idx);
    assert_eq!(hp, 150.0, "armor 50 + 100 [M 0x185368:65-73]");
    assert!(!m.game.slots[0].dead);
    assert_eq!(m.game.slots[0].streak, 0, "the streak resets [M :55-57]");
    assert_eq!(m.game.slots[0].status, 0xfffe, "the last-shooter word resets to 0xfffe");
    assert_eq!(m.game.slots[0].status_bits, 0, "the status bits clear");
    // sister: the round-over state blocks respawns [M 0x185368:16-20]
    m.game.state = 3;
    assert!(!m.respawn_allowed(0));
}
