//! 632 mp-hud-data — headless: a match's HUD snapshot matches the sim state (numbers/ids only,
//! no ISO). Every test pairs its assertion with a sister that would fail if the data model were
//! wrong. Evidence: docs/systems/mp-hud-data.md; the decomp files carry the [M] tags
//! (MP_GetRadarObjects 0x189840, MP_GetPlayerScore 0x189730, MP_GetTeamScore 0x18b338,
//! HUD_MPUpdatePane 0x19f0a0, HUD_RadarUpdate 0x19f6e8, Timer_Seconds2String 0x1deaa8,
//! Menu_GetMPScore 0x203af8, P_MPDEBRIEFING's sort 0x212198).

use nf_game::drone::Rng;
use nf_game::mp::hud::{MpHudSnapshot, RadarKind};
use nf_game::mp::{scenario, MpMatch, MpSettings};

const DT: f32 = 1.0 / 60.0;

/// `time_limit` is SECONDS here (the port's settings carry the confirm-converted value — the
/// game ×60s the minutes at P_MPCONFIRM 0x20ec00:146 before MP_Init copies it [M 0x183438:39]).
fn make(scenario_word: u32, frag: i32, time_secs: i32) -> MpMatch {
    let mut s = MpSettings::deathmatch(0x07000011);
    s.scenario = scenario_word;
    s.frag_limit = frag;
    s.time_limit = time_secs;
    s.human_count = 2;
    s.player_count = 2;
    s.total_players = 2;
    let mut m = MpMatch::init(s);
    m.start(&mut Rng::new(1));
    m
}

fn join(m: &mut MpMatch, slot: usize, team: u32) {
    m.settings.players[slot].team = team;
    m.set_slot(slot, [10.0 + 10.0 * slot as f32, 0.0, 0.0]);
}

/// The score rows read the sim's own numbers in slot order; the display score follows
/// `Menu_GetMPScore` (kills for Arena, score for the scoring scenarios) and the sort is the
/// game's descending board order [M 0x189730, 0x203af8:44-54, 0x212198:112-130].
#[test]
fn snapshot_rows_match_the_sim_state() {
    let mut rng = Rng::new(7);
    let mut m = make(scenario::DEATHMATCH, 20, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 0);
    // Slot 1 leads 2 kills to 0, then takes a death (a negative-score slot 0 reads as its score).
    m.player_killed(0, Some(1), false, false, &mut rng);
    m.player_killed(0, Some(1), false, false, &mut rng);
    m.player_killed(1, Some(0), false, false, &mut rng);

    let snap = MpHudSnapshot::build(&m, 0, true);
    assert_eq!(snap.viewer_slot, 0);
    assert_eq!(snap.rows.len(), 2, "two humans, two rows");
    assert_eq!(snap.rows[0].slot, 0);
    assert_eq!(snap.rows[1].slot, 1);
    assert_eq!(snap.rows[0].kills, 1, "the row reads the sim's own counters");
    assert_eq!(snap.rows[0].deaths, 2);
    assert_eq!(snap.rows[1].kills, 2);
    assert_eq!(snap.rows[1].deaths, 1);
    // Arena's display score is the KILLS [M 0x203af8's 0x1/0x20000002 arm] — sister: the score
    // f32 (-1.0 vs 2.0) would sort the other way and the assertion would flip.
    assert_eq!(snap.rows[0].display_score(scenario::DEATHMATCH), 1.0);
    assert_eq!(snap.rows[1].display_score(scenario::DEATHMATCH), 2.0);
    let sorted: Vec<usize> = snap.sorted_rows().iter().map(|r| r.slot).collect();
    assert_eq!(sorted, vec![1, 0], "the board sorts descending by display score");
    // The score text is two lines for Arena (kills\n deaths) [M 0x189730:20-27]; Assassin/KOTH
    // print the score line too [M :13-19]. Sister: a 3-line read here would flip the count.
    let (n, _, _, _) = snap.rows[1].score_lines(scenario::DEATHMATCH);
    assert_eq!(n, 2);
    let mut koth = make(scenario::KOTH_FFA, 20, -1);
    join(&mut koth, 0, 0);
    koth.game.slots[0].score = 7.0;
    let snap2 = MpHudSnapshot::build(&koth, 0, true);
    let (n, score, _, _) = snap2.rows[0].score_lines(scenario::KOTH_FFA);
    assert_eq!(n, 3);
    assert_eq!(score, 7.0, "KOTH's line 1 is the score");
    assert_eq!(snap2.rows[0].display_score(scenario::KOTH_FFA), 7.0);
}

/// The timer is `limit − clock` clamped, printed mm:ss by `Timer_Seconds2String` mode 2 —
/// negatives clamp to 00:00, minutes wrap past 60 (no hours in mode 2) [M 0x184268:144-147,
/// 0x1deaa8:17-33]. Sister: a non-clamping read would print a negative.
#[test]
fn timer_matches_the_clock() {
    let mut m = make(scenario::DEATHMATCH, 20, 600);
    join(&mut m, 0, 0);
    join(&mut m, 1, 0);
    tick_seconds(&mut m, 90); // 90 s of the 600 s limit
    let snap = MpHudSnapshot::build(&m, 0, true);
    assert!((snap.time_left - 510.0).abs() < 1.0, "limit − clock, in seconds (got {})", snap.time_left);
    assert_eq!(snap.timer_string(), "08:30");
    // The clamp: past the limit the game's centisecond input goes negative and prints 00:00
    // [M 0x1deaa8:21-25]. Sister: without the clamp the string would go negative.
    let mut expired = snap.clone();
    expired.time_left = -3.0;
    assert_eq!(expired.timer_string(), "00:00");
    // The minute wrap: mode 2 prints (secs/60) % 60 — 61 minutes shows 01:00 [M 0x1deaa8:29-33].
    let mut hour = snap.clone();
    hour.time_left = 61.0 * 60.0;
    assert_eq!(hour.timer_string(), "01:00");
}

fn tick_seconds(m: &mut MpMatch, secs: u32) {
    let mut rng = Rng::new(3);
    for _ in 0..secs * 60 {
        m.update(DT, false, &mut rng);
    }
}

/// The radar list reads the sim: the viewer is never a blip, dead slots drop out, the team
/// colors are the game's packed values, and the viewer's radar toggle gates the whole list
/// [M 0x189840:22-96]. Sister assertions flip the color/skip logic.
#[test]
fn radar_blips_match_the_match_state() {
    let mut rng = Rng::new(7);
    // A TEAM game (the radar colors branch on team_game) [M 0x189840:54-62].
    let mut m = make(scenario::TEAM_DEATHMATCH, 20, -1);
    join(&mut m, 0, 0);
    join(&mut m, 1, 1);
    let snap = MpHudSnapshot::build(&m, 0, true);
    assert_eq!(snap.radar.len(), 1, "the viewer is not a blip; slot 1 is");
    let b = &snap.radar[0];
    assert_eq!(b.kind, RadarKind::Player.as_u16());
    assert_eq!(b.slot, Some(1));
    assert_eq!(b.pos, [20.0, 0.0, 0.0], "the slot's live position");
    assert_eq!(b.argb, 0x2d61_d2ff, "team 1 is the blue blip");
    // Sister: team 0's blip is the red one.
    let snap_t0 = MpHudSnapshot::build(&m, 1, true);
    assert_eq!(snap_t0.radar[0].argb, 0xd22d_35ff);
    // A dead slot drops out [M 0x189840's state gates; the port's dead is the stand-in].
    m.player_killed(1, Some(0), false, false, &mut rng);
    let snap_dead = MpHudSnapshot::build(&m, 0, true);
    assert!(snap_dead.radar.is_empty(), "the dead opponent is no blip");
    // The viewer's radar toggle gates the list BEFORE any blip is built [M 0x189840:22-24].
    m.game.slots[1].dead = false;
    m.set_slot(1, [20.0, 0.0, 0.0]);
    let snap_off = MpHudSnapshot::build(&m, 0, false);
    assert!(snap_off.radar.is_empty(), "radar off = an empty list");
    // FFA: everyone neutral white [M 0x189840:54-56].
    let mut ffa = make(scenario::DEATHMATCH, 20, -1);
    join(&mut ffa, 0, 0);
    join(&mut ffa, 1, 0);
    let snap_ffa = MpHudSnapshot::build(&ffa, 0, true);
    assert_eq!(snap_ffa.radar[0].argb, 0x7f7f_7fff);
}

/// The objective status: the flag carriers come from the objects' carrier pointers (the same
/// datum `MP_HasTeamFlag` reads for the pane icons [M 0x18b430, 0x19f0a0:89-100]), the uplink
/// owner/controller/rgb read the terminals' state [M 0x18ba80, 0x186d98], and the Assassin rows
/// carry the target marker [M 0x18aa70, 0x19f0a0:102-113].
#[test]
fn objective_status_matches_the_objects() {
    let mut rng = Rng::new(7);
    // CTF: two flags on their bases at start, blips in the game's colors [M 0x189840:98-121].
    let mut ctf = make(scenario::CAPTURE_THE_FLAG, 3, -1);
    join(&mut ctf, 0, 0);
    join(&mut ctf, 1, 1);
    // The mode objects: the port builds them from registered MP objects — a type-0 flag per team
    // (the placement's param-0 type, `MP_RegisterMPObject` 0x185990's dispatch).
    ctf.register_mp_object([5.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0, [0, 0, 0, 0, 0, 0, 0], 0x1001);
    ctf.register_mp_object([45.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0, [0, 1, 0, 0, 0, 0, 0], 0x1002);
    ctf.create_mode_objects(&mut Rng::new(9));
    let snap = MpHudSnapshot::build(&ctf, 0, true);
    assert_eq!(snap.flags, [None, None], "both flags at base at start");
    let flag_blips: Vec<&nf_game::mp::hud::MpRadarObj> =
        snap.radar.iter().filter(|b| b.kind == RadarKind::Flag.as_u16()).collect();
    assert_eq!(flag_blips.len(), 2, "both flags on the radar");
    assert_eq!(flag_blips[0].argb, 0xd22d_35ff, "flag 0 red [M 0x189840:103-104]");
    assert_eq!(flag_blips[1].argb, 0x2d61_d2ff, "flag 1 blue");
    assert_eq!(flag_blips[0].pos, [5.0, 0.0, 0.0]);
    // The carrier: pick up flag 0 → the snapshot's flags[0] reads the sim's carrier.
    if let Some(f) = ctf.objects_rt.flags[0].as_mut() {
        f.state = 1;
        f.carrier = Some(1);
        f.pos = [20.0, 0.0, 0.0];
    }
    let snap2 = MpHudSnapshot::build(&ctf, 0, true);
    assert_eq!(snap2.flags[0], Some(1), "the carrier slot rides the snapshot");
    // Uplink: owner/controller/rgb [M 0x186d98, 0x18ba80].
    let mut up = make(scenario::UPLINK, 3, -1);
    join(&mut up, 0, 0);
    join(&mut up, 1, 1);
    up.register_mp_object([9.0, 0.0, 0.0], [0.0, 0.0, 0.0], 2, [2, 0, 0, 0, 0, 0, 0], 0x1003);
    up.create_mode_objects(&mut Rng::new(9));
    up.objects_rt.uplinks[0].state = 1;
    up.objects_rt.uplinks[0].controller = 1;
    let snap3 = MpHudSnapshot::build(&up, 0, true);
    assert_eq!(snap3.uplinks.len(), 1);
    let u = &snap3.uplinks[0];
    assert_eq!(u.owner, 1);
    assert_eq!(u.controller, 1);
    assert_eq!(u.rgb, [0x2d, 0x61, 0xd2], "team 1's live RGB [M 0x186d98:41-47]");
    let ub = snap3.radar.iter().find(|b| b.kind == RadarKind::Uplink.as_u16()).expect("the uplink blip");
    assert_eq!(ub.argb, 0x2d61_d2ff, "the RGB packs into the blip [M 0x189840:138-140]");
    // Assassin: the target row carries the marker [M 0x18aa70:38-41].
    let mut asr = make(scenario::ASSASSIN, 20, -1);
    join(&mut asr, 0, 0);
    join(&mut asr, 1, 0);
    asr.assassin = Some(0);
    asr.assassin_target = Some(1);
    let snap4 = MpHudSnapshot::build(&asr, 0, true);
    assert_eq!(snap4.assassin, Some(0));
    assert_eq!(snap4.target, Some(1));
    assert!(snap4.rows.iter().find(|r| r.slot == 1).unwrap().is_target);
    assert!(!snap4.rows.iter().find(|r| r.slot == 0).unwrap().is_target);
}
