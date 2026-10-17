//! mp-host-match-logic (729, audit gap 4): the live host now RUNS the match logic —
//! `update_objects` every frame, `player_killed` on the death path, the 5 s respawn
//! (`MP_ReSpawn`), and the frag-limit handover into the debriefing. Everything here rides the
//! real app path: `load_mp_carry` (the confirm page's load — the menu's settings + seed) and
//! `Sim::step_bond` ticks; the scripted kill enters through the game's own
//! `MP_RegisterBulletHit` 0x1857c0 (the seam the bot pipeline feeds) plus the raw
//! `weapon_data+0x0C` damage the port's hit stand-in applies. Opt-in via NF_ISO.

use nf_game::mp::debrief::MpDebriefResult;
use nf_view::menu::{FrontEnd, Page};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

/// A Quick-Game-style local deathmatch in the real app path: a kill scores (the real
/// `MP_PlayerKilled` rules, not an inline stand-in), the victim respawns 5 s later at the
/// spawn policy's point with the `MP_ReSpawn` health, and the frag limit ends the match into
/// the debriefing (the host's `open_mp_debriefing` handover). frag limit 2 so the first kill
/// respawns (state 0 still) and the second ends the match.
#[test]
fn local_deathmatch_kill_scores_respawns_and_ends_into_the_debriefing() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut settings = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    settings.frag_limit = 2;
    settings.bot_count = 1; // a Quick Game's bot — its slot 4 takes the scripted kill credit
    settings.time_limit = -1; // no time-up latch in the window (the frag limit owns the end)
    let mut scene = nf_view::load_mp_carry(&iso, "07000023.bin", false, None, settings)
        .expect("the arena loads through the app's MP carry path");
    let sim = scene.sim.as_mut().unwrap();
    {
        let m = sim.mp.as_ref().expect("the match builds");
        assert!(m.settings.started, "MP_Start ran [M 0x183a50:207]");
        assert_eq!(m.bots.len(), 1, "the Quick Game's bot entity (704's wiring)");
        assert_eq!(m.game.state, 0, "the match is playing");
        assert!(m.spawns.points.iter().take(m.spawns.count as usize).any(|p| p.used == 1
            && (p.pos[0] - sim.bond.pos[0]).abs() < 0.01 && (p.pos[2] - sim.bond.pos[2]).abs() < 0.01),
            "Bond stands at a registered spawn point");
        // The objects now exist in the live match too (the mode objects build at MP_Start —
        // deathmatch stores none, but the call must not be missing for the other scenarios).
        assert!(m.objects.created.is_empty(), "deathmatch stores no mode objects [M 0x185990 gates]");
    }

    // ---- Kill 1: the bot's hit registers (the game's MP_RegisterBulletHit seam), the raw
    // damage lands, and one app tick runs the death + the real scoring.
    {
        let m = sim.mp.as_mut().unwrap();
        assert!(m.register_bullet_hit(0, 4), "the FFA hit is allowed [M 0x1857c0]");
        assert_eq!(m.game.slots[0].status, 4, "the last-shooter word records the attacker [M 0x1857c0:34]");
    }
    sim.drones.health.health = 0.0;
    sim.step_bond(Default::default(), false, false, false);
    let m = sim.mp.as_ref().unwrap();
    assert!(sim.drones.death.active, "the death sequence started [M 0x190810]");
    assert_eq!(m.game.slots[0].deaths, 1, "every scenario counts the death [M 0x188f70:92-94]");
    assert!(m.game.slots[0].dead, "the slot's dead marker (the obj+0xF4 stand-in)");
    assert_eq!(m.game.slots[4].kills, 1, "the killer's kill count [M 0x188f70:157-163]");
    assert_eq!(m.game.slots[4].streak, 1, "the killer's streak [M 0x188f70]");
    assert!((m.game.slots[4].score - 1.0).abs() < 1e-6,
        "the frag scores: bit30 clear + the deathmatch personal gate [M 0x188f70:179-184]");
    assert_eq!(m.game.state, 0, "frag limit 2 > high 1: the match is still playing");
    assert_eq!(m.game.slots[0].status, 4, "the word survives until the respawn resets it [M 0x185368:56]");

    // ---- The 5 s gate: `Player_HandleDeath` 0x190bb0:41-54 — FRAME_RATE_INT * 5 frames after
    // the stamp. The 300th dead tick respawns (the dead-tick counter starts at 0 on the stamp
    // tick); 299 ticks in: still dead.
    for _ in 0..299 { sim.step_bond(Default::default(), false, false, false); }
    // [M Player_HandleDeath 0x190bb0:41-54] FRAME_RATE_INT * 5 after the stamp
    assert!(sim.drones.death.active && sim.drones.death.ticks == 299, "still inside the delay");
    sim.step_bond(Default::default(), false, false, false);
    let m = sim.mp.as_ref().unwrap();
    assert!(!sim.drones.death.active, "the respawn ended the death sequence [M 0x185368:93 obj+0xff = 3]");
    assert!((sim.drones.health.health - 100.0).abs() < 1e-5,
        "the respawn health = armor_bonus + 100.0 [M 0x185368:65-73] (armor_bonus 0)");
    assert!(!m.game.slots[0].dead, "the slot is alive again [M 0x185368]");
    assert_eq!(m.game.slots[0].status, 0xfffe, "the last-shooter word reset [M 0x185368:56]");
    // The policy's point, re-derived from the match's public state: policy 0 = the least-far
    // candidate (the running-min tracker [M 0x185098:141-146]) among the points over 2.0 sq-dist
    // from the live bot; no RNG (policy 0 draws nothing), first-of-equal wins; the unset
    // fallback is the range start 0 [M 0x185098:165-176]. The bot is inert (704: BotIdle), so
    // its position is the load pick's.
    let bot_pos = m.game.slots[4].pos.expect("the bot's slot rides its live position");
    let mut expect: usize = 0;
    let mut min = 9999.0f32;
    for (i, p) in m.spawns.points.iter().enumerate().take(m.spawns.count as usize) {
        if p.used == 0 { continue; }
        let d = sq([p.pos[0], p.pos[1], p.pos[2]], bot_pos);
        if d > 2.0 && d < min { min = d; expect = i; }
    }
    let sp = &m.spawns.points[expect];
    // [M 0x185098:141-146,165-176] policy 0's least-far point; the y band is [S port engineering] (the load's stand_at convention)
    assert!((sim.bond.pos[0] - sp.pos[0]).abs() < 0.01 && (sim.bond.pos[2] - sp.pos[2]).abs() < 0.01,
        "the body respawns at the policy's point {expect} (x/z; got {:?}, want {:?})",
        [sim.bond.pos[0], sim.bond.pos[2]], [sp.pos[0], sp.pos[2]]);
    assert!(sim.bond.pos[1] > sp.pos[1] - 1.7 && sim.bond.pos[1] < sp.pos[1] + 1.0,
        "the body sits by the spawn's eye height (the load's stand_at convention)");

    // ---- Kill 2 → the frag limit latches → the end states → the debriefing handover.
    {
        let m = sim.mp.as_mut().unwrap();
        // [M MP_RegisterBulletHit 0x1857c0; MP_PlayerKilled 0x188f70:179-184] the second frag scores
        assert!(m.register_bullet_hit(0, 4), "the second hit registers");
    }
    sim.drones.health.health = 0.0;
    sim.step_bond(Default::default(), false, false, false);
    let m = sim.mp.as_ref().unwrap();
    // [M 0x188f70:179-184] the deathmatch frag
    assert!((m.game.slots[4].score - 2.0).abs() < 1e-6, "the second frag scores");
    // The victim stays dead this time: the frag latch leaves state != 0 and `MP_ReSpawn`'s
    // gate refuses [M 0x185368:16-20].
    for _ in 0..6 { sim.step_bond(Default::default(), false, false, false); }
    let m = sim.mp.as_ref().unwrap();
    assert!(m.game.frag_latched, "frag_limit 2 <= high 2 [M 0x184268:170-172]");
    assert_eq!(m.game.state, 5, "0 → 1 (latch) → 3 (who won) → 4 → 5 (waiting) [M 0x184bd0]");
    assert!(m.game.request_debriefing, "the case-4 request [M 0x184bd0:62-78 / GameState+0x1C]");
    // The full window past the gate: the victim stays dead — `MP_ReSpawn`'s state gate refuses
    // [M 0x185368:16-20].
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); }
    let m = sim.mp.as_ref().unwrap();
    assert!(sim.drones.death.active, "no respawn once the match ended [M 0x185368:16-20]");
    // The host's handover — the app glue opens P_MPDEBRIEFING with the board built from the
    // match ([M 0x212198 msg 0x4c]); the frame loop's fe gate then stops the sim ticks.
    let board_rows = m.debriefing().rows.len();
    assert_eq!(board_rows, 2, "the board carries the human and the bot (the settings' counts)");
    let mut fe = FrontEnd::new();
    fe.open_mp_debriefing(sim.mp.as_ref().unwrap());
    // [M 0x212198 msg 0x4c; Who-won 0x1848d0:79-84] the board ranks by points, deaths per slot
    assert_eq!(fe.page, Page::MpDebriefing, "the handover lands on the debriefing page");
    let board = fe.mp_debriefing.as_ref().expect("the board built");
    assert_eq!(board.rows[0].points, 2, "the bot ranks first with the 2 frags");
    assert_eq!(board.rows[0].slot, 4, "the winner is the killer's slot");
    // [M 0x188f70:92-94] every death counts
    assert_eq!(board.rows[1].deaths, 2, "the victim's two deaths on the board");
    assert!(matches!(board.result, MpDebriefResult::Wins(_)), "an FFA win line [M 0x1848d0:79-84]");
}

/// The audit's acceptance (uncited-audit gap 4): a scripted CTF capture scores in the live
/// host the same as in `mp_match_rules.rs`. The live host loads the arena with the CTF
/// scenario (the mode objects build at `MP_Start` now), the carrier picks the enemy flag up
/// through the touch seam, `update_objects`' `MP_FlagUpdate` half scores the capture at the
/// base, and a headless `MpMatch` driven through the same events lands on the same scores.
#[test]
fn ctf_capture_scores_in_the_live_host_like_the_headless_rules() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut settings = nf_game::mp::MpSettings::deathmatch(0x0700_0023);
    settings.scenario = nf_game::mp::scenario::CAPTURE_THE_FLAG; // bit29: a team game
    settings.team_game = 1;
    settings.time_limit = -1;
    let mut scene = nf_view::load_mp_carry(&iso, "07000023.bin", false, None, settings)
        .expect("the arena loads");
    let sim = scene.sim.as_mut().unwrap();
    {
        let m = sim.mp.as_ref().unwrap();
        // [M MP_Start 0x183a50:207; mode objects at MP_Start 0x185990] flags + bases from the registered type-0/1 placements
        assert!(m.settings.started, "the match started");
        let (f0, f1) = (m.objects_rt.flags[0].is_some(), m.objects_rt.flags[1].is_some());
        assert!(f0 && f1, "the live match built both flags at MP_Start (the registered type-0/1 placements)");
        assert!(m.objects_rt.bases[0].is_some() && m.objects_rt.bases[1].is_some(), "both bases link");
    }
    // The scripted capture: team 0's carrier (slot 0) walks onto team 1's flag (the touch
    // stand-in — the spatial test is the caller's [M MP_HitBy 0x1830e0's contact walk]), then
    // stands on team 0's base; `update_objects`' flag half scores through the live tick.
    let (flag1_home, base0) = {
        let m = sim.mp.as_ref().unwrap();
        (m.objects_rt.flags[1].as_ref().unwrap().pos, m.objects_rt.bases[0].unwrap())
    };
    // Headless sister: the same script through the bare rules (the mp_match_rules path).
    let mut head = {
        let m = sim.mp.as_ref().unwrap();
        let mut h = nf_game::mp::MpMatch::init(m.settings.clone());
        h.settings.started = true;
        h.objects_rt = m.objects_rt.clone();
        h.set_slot(0, [flag1_home[0], flag1_home[1], flag1_home[2]]);
        h
    };
    head.touch_flag(0);
    // The live touch (the same seam), then the carrier parks on the base and ticks.
    {
        let m = sim.mp.as_mut().unwrap();
        m.touch_flag(0);
        // [M MP_FlagUpdate 0x1861a8] touch â†’ state 1, carrier = the toucher
        assert_eq!(m.objects_rt.flags[1].as_ref().unwrap().state, 1, "the enemy flag is carried");
        assert_eq!(m.objects_rt.flags[1].as_ref().unwrap().carrier, Some(0));
    }
    let mut head_slot_pos;
    {
        // The carrier's body walks to the base (the real body), then the tick scores it.
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, base0, 0.0);
        let m = sim.mp.as_mut().unwrap();
        m.set_slot(0, sim.bond.pos);
        head_slot_pos = sim.bond.pos;
    }
    head.set_slot(0, head_slot_pos);
    sim.step_bond(Default::default(), false, false, false); // the live flag_update scores here
    let m = sim.mp.as_ref().unwrap();
    assert!((m.game.team_scores[0] - 1.0).abs() < 1e-6, "the capture: the taker's team +1.0 [M 0x1861a8:89-93]");
    assert!((m.game.slots[0].score - 1.0).abs() < 1e-6, "the carrier +1.0 [M 0x1861a8:94-98]");
    assert_eq!(m.objects_rt.flags[1].as_ref().unwrap().state, 0, "the flag reset home [M 0x1861a8:99-112]");
    // Parity: the headless rules land on the same scores through the same events.
    for _ in 0..60 { head.update_objects(1.0 / 60.0, &[false; 8], &mut nf_game::drone::Rng::new(1)); }
    assert!((head.game.team_scores[0] - m.game.team_scores[0]).abs() < 1e-6
        && (head.game.slots[0].score - m.game.slots[0].score).abs() < 1e-6,
        "the live host scores the capture exactly like the headless rules");
    // Sister: the score is not constant — a no-touch run scores nothing.
    // [M 0x1861a8:89-93] sister: only the taker's team scores
    assert!((m.game.team_scores[1]).abs() < 1e-6, "team 1 never touched: no score");
}
