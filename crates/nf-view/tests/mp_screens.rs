//! mp-screens: the multiplayer front-end pages' data/state pins (numbers/ids only) and the
//! spot-shot's movie gates. The draw itself is the main agent's task; these pin the data it
//! draws and the window-parity decision the F12 shot makes.
//!
//! Evidence tags: [M] the decompile (file:line), [D] disc, [L] EE snapshot. A guess is never a
//! hard assertion.

use nf_view::menu::{FrontEnd, MP_CONFIRM_WEAPON_LABELS, MP_HANDICAPS, MP_MAP_ROWS, MP_SCENARIO_ROWS};

// ── the spot shot's movie gates (the F12 all-dark fix) ──────────────────────────────────────

/// The gates mirror the redraw's `movie_up` composition: the looping page backdrop draws UNDER
/// the overlay (Blank::Never — the overlay skips its dark plate while the frame exists), the
/// intro/attract FMVs own the screen once a frame lands (WithFrame), the state-6 briefing blits
/// over the empty backdrop unconditionally (Always). Sister: the aux case must NOT blank.
#[test]
fn spot_movie_gates_mirror_the_redraw() {
    use nf_view::Blank;
    // A front-end page with its looping aux backdrop: the overlay draws over it.
    assert_eq!(nf_view::spot_movie_gates(false, false, false, true, false, true), (true, Blank::Never));
    // The decode-ahead hold (the briefing's pending player) stops the aux — no movie, the
    // overlay draws normally (the redraw's aux_up gate has the same !movie_pending).
    assert_eq!(nf_view::spot_movie_gates(false, false, false, true, true, true), (false, Blank::Never));
    // The intro owns the screen once a frame exists (the redraw pairs the blank with the frame).
    assert_eq!(nf_view::spot_movie_gates(false, true, false, false, false, true), (true, Blank::WithFrame));
    // The attract movie the same way.
    assert_eq!(nf_view::spot_movie_gates(false, false, true, false, false, true), (true, Blank::WithFrame));
    // The state-6 briefing blits over the empty backdrop even before the first frame.
    assert_eq!(nf_view::spot_movie_gates(true, false, false, false, false, true), (true, Blank::Always));
    // Plain gameplay (no fe, no movie): no frame pull, the normal overlay path.
    assert_eq!(nf_view::spot_movie_gates(false, false, false, false, false, false), (false, Blank::Never));
    // Sister: the intro must never be Blank::Never (an all-dark F12 on the intro is the bug).
    assert_ne!(nf_view::spot_movie_gates(false, true, false, false, false, true).1, Blank::Never);
}

// ── the Confirm page's data (the formerly-empty draw) ───────────────────────────────────────

/// P_MPCONFIRM 0x20ec00's 0x4c arm: the ids and values the draw code renders. The lang-less
/// binds render the "<0x…>" fallback, so the row strings pin the ID the fallback shows plus the
/// value — the ISO pass pins the real strings.
#[test]
fn mp_confirm_summary_ffa_ids_and_values() {
    let mut fe = FrontEnd::new();
    // A free-for-all Deathmatch on Skyrail, weapon set 3, time 10, points 5, friendly fire off.
    fe.mp_scenario = MP_SCENARIO_ROWS[1].3; // 0x0000_0001 Arena/Deathmatch [D the parse]
    fe.mp_level = MP_MAP_ROWS[0].3;         // 0x0700_0024 Skyrail
    fe.mp_weapon_set = 3;
    fe.mp_time_limit = 10;
    fe.mp_frag_limit = 5;
    fe.mp_friendly_fire = 0;
    fe.mp_join[0].joined = true;
    fe.mp_join[0].team = 1; // MI6 — blue in a free-for-all too [M 0x20ec00:176-186]
    fe.mp_bots[0].enabled = true;
    fe.mp_bots[0].team_mi6 = false; // red column in a team game; FFA fills blue only [M :320-352]
    let s = fe.mp_confirm_summary(None);
    assert!(!s.team_game);
    assert_eq!(s.header_label, 0x295, "the FFA header label [M 0x20ec00:162-166]");
    assert_eq!(s.rows.map_label, 0x3d4);
    assert_eq!(s.rows.map_name_label, MP_MAP_ROWS[0].1, "the arena's mp_level name (record+4, type 8) [M :281-286]");
    assert_eq!(s.rows.scenario_name_label, MP_SCENARIO_ROWS[1].1, "the scenario's mp_scenario name (record+4, type 13) [M :287-296]");
    assert_eq!(s.rows.weapon_label, 0x1a6, "weapon set 3's label [M the 0x248-270 switch case 3]");
    assert_eq!(s.rows.time_label, 0x3d8, "TIME for a non-0x10 scenario [M :273-279]");
    assert_eq!(s.rows.points_label, 0x3d6);
    assert!(s.rows.time_row.ends_with(": 10"), "the sprintf %s: %d value [M :277-279], got {}", s.rows.time_row);
    assert!(s.rows.time_row.contains("<0x3d8>"), "the lang-less bind's fallback shows the label id, got {}", s.rows.time_row);
    assert!(s.rows.points_row.ends_with(": 5"), "got {}", s.rows.points_row);
    assert!(s.rows.friendly_fire_row.contains("<0x1b6>"), "friendly fire OFF 0x1b6 at +0x198 == 0 [M :313-318], got {}", s.rows.friendly_fire_row);
    // The panels: one joined human, blue; the FFA bot column is blue-only even for a Phoenix bot.
    assert_eq!(s.humans.len(), 1);
    assert!(s.humans[0].blue);
    assert_eq!(s.humans[0].handicap, 0);
    assert_eq!(s.bots_blue, 1, "a free-for-all fills the blue column only [M :337-345]");
    assert_eq!(s.bots_red, 0);
    // Sister: the same setup with the weapon set at the table's end must pin the last label.
    fe.mp_weapon_set = 10;
    assert_eq!(fe.mp_confirm_summary(None).rows.weapon_label, MP_CONFIRM_WEAPON_LABELS[10], "the switch's 0x1b2 case [M :268-270]");
}

/// The team split: the header flips to 0x1c8, a Phoenix human goes red, a MI6 bot fills blue and
/// a Phoenix bot red [M 0x20ec00:176-186, :337-352]. Sister: the FFA run above pins the other
/// branch of every flipped value.
#[test]
fn mp_confirm_summary_team_split() {
    let mut fe = FrontEnd::new();
    fe.mp_scenario = MP_SCENARIO_ROWS[1].3 | 0x2000_0000; // the team bit [M 0x20ec00:160]
    fe.mp_level = MP_MAP_ROWS[0].3;
    fe.mp_time_limit = -1;
    fe.mp_frag_limit = -1;
    fe.mp_join[0].joined = true;
    fe.mp_join[0].team = 0; // Phoenix → red [M the else arm :186-195]
    fe.mp_join[1].joined = true;
    fe.mp_join[1].team = 1; // MI6 → blue
    fe.mp_bots[0].enabled = true;
    fe.mp_bots[0].team_mi6 = true;
    fe.mp_bots[1].enabled = true;
    fe.mp_bots[1].team_mi6 = false;
    fe.mp_bots[2].enabled = false;
    let s = fe.mp_confirm_summary(None);
    assert!(s.team_game);
    assert_eq!(s.header_label, 0x1c8, "the team header label [M :167-172]");
    let (blue, red): (Vec<_>, Vec<_>) = s.humans.iter().partition(|h| h.blue);
    assert_eq!(blue.len(), 1, "one MI6 human on blue [M :176-215]");
    assert_eq!(red.len(), 1, "one Phoenix human on red [M :176-215]");
    // The −1 limits render the NONE label 0x3ca [M :273-279, :305-311].
    assert!(s.rows.time_row.contains("<0x3ca>"), "TIME NONE at −1, got {}", s.rows.time_row);
    assert!(s.rows.points_row.contains("<0x3ca>"), "POINTS NONE at −1, got {}", s.rows.points_row);
    assert_eq!(s.bots_blue, 1);
    assert_eq!(s.bots_red, 1, "the team game splits by the bot's team byte [M :337-352]");
    // Sister: the disabled bot is in neither column.
    assert_eq!(s.bots_blue + s.bots_red, 2);
}

// ── the Setup page's data (the formerly-empty draw) ─────────────────────────────────────────

/// P_MPSETUP 0x20d7a8's 0x4c: the FFA prompt 0x37d (character), the unjoined panels hidden
/// (state 0), the handicap defaults 1/3/2/0 for the joined agent [M :20-23]. Sister: the team
/// game flips the prompt to 0x37c and the pick to the 0x1c7/0x1c8 pair.
#[test]
fn mp_setup_panels_ffa_and_team() {
    let mut fe = FrontEnd::new();
    fe.mp_scenario = MP_SCENARIO_ROWS[1].3; // FFA
    fe.mp_join[0].joined = true;
    fe.mp_join[0].skin = 0; // Bond [D MP_CHARACTERS[0]]
    fe.mp_setup_state = 3;
    fe.mp_handicap = 25;
    let panels = fe.mp_setup_panels(None);
    assert!(panels[0].joined);
    assert_eq!(panels[0].state, 3, "the FFA state (character list) [M 0x20d7a8:29-43]");
    assert_eq!(panels[0].prompt_label, 0x37d);
    assert!(panels[0].pick.contains("<0x21a>"), "agent 1's pick binds the skin's name label (Bond 0x21a [D MP_CHARACTERS[0].1]), got {}", panels[0].pick);
    assert_eq!(panels[0].handicap, Some(25), "agent 1 carries the walked-in handicap");
    for p in 1..4 {
        assert!(!panels[p].joined, "panels 2-4 are unjoined (the port is keyboard-only) [M the joined gate :73-82]");
        assert_eq!(panels[p].state, 0);
        assert!(panels[p].pick.is_empty());
        assert_eq!(panels[p].handicap, None);
    }
    // The team game: prompt 0x37c, the pick from mpjoin+4 [M 0x20d7a8:44-54].
    fe.mp_scenario |= 0x2000_0000;
    fe.mp_join[0].team = 1;
    let panels = fe.mp_setup_panels(None);
    assert_eq!(panels[0].state, 2);
    assert_eq!(panels[0].prompt_label, 0x37c);
    assert!(panels[0].pick.contains("<0x1c8>"), "team 1 renders MI6 0x1c8, got {}", panels[0].pick);
    // Sister: the Phoenix side renders 0x1c7.
    fe.mp_join[0].team = 0;
    assert!(fe.mp_setup_panels(None)[0].pick.contains("<0x1c7>"));
}

/// The state-4 handicap list: the pick is the raw .data string (not a Txt label) [D
/// 0x30d390..0x30d3c8; the port's MP_HANDICAPS carries the same strings]. Sister: a wrong
/// selection index shows a different node.
#[test]
fn mp_setup_handicap_state_shows_the_data_string() {
    let mut fe = FrontEnd::new();
    fe.mp_scenario = MP_SCENARIO_ROWS[1].3;
    fe.mp_join[0].joined = true;
    fe.mp_setup_state = 4;
    fe.mp_setup_sel = 4; // "+25" [D MP_HANDICAPS[4] == 0x30d3a0's string]
    let panels = fe.mp_setup_panels(None);
    assert_eq!(panels[0].state, 4);
    assert_eq!(panels[0].prompt_label, 0x37d);
    assert_eq!(panels[0].pick, MP_HANDICAPS[4].1, "the handicap list's items are the raw strings [D 0x30d390..0x30d3c8]");
    assert_eq!(panels[0].pick, "+25");
    fe.mp_setup_sel = 0;
    assert_eq!(fe.mp_setup_panels(None)[0].pick, "-75", "sister: another node renders its own string");
}
