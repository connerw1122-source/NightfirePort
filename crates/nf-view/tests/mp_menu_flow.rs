//! mp-menu-flow: the multiplayer front-end flow, headless (no ISO needed).
//!
//! The page graph is the decompile's [M]: Main menu row 1 → C_GOMULTIPLAYER 0x216a78's delayed
//! page change → P_MPJOIN 0x40000019; the join page's ready arm (C_RBMPSTART 0x20d048's state-5
//! ✕ + P_MPJOIN 0x20ca00's Menu_MPAreWeReady) → P_MPSCENARIO 0x4000001a; C_SBMPSCEN 0x20c600's
//! confirm → P_MPMAP 0x40000013; C_SBMPMAP 0x217538's confirm → P_MPSETUP 0x40000051; the
//! per-agent picks → P_MPOPTIONS 0x40000012; C_SBMPOPTIONS row 0 → P_MPCONFIRM 0x40000049, whose
//! confirm (P_MPCONFIRM 0x20ec00:53-94) hands the match over. Each step keeps a sister assertion
//! that can fail.

use nf_view::menu::{Action, FrontEnd, Page, MP_MAP_ROWS, MP_OPTIONS_ROWS, MP_SCENARIO_ROWS};

/// Main menu → Multiplayer → Join (the main menu's row 1 confirm).
fn enter_mp(fe: &mut FrontEnd) {
    fe.tick(); // boot → the runner state [M 0x1c98c8 case 1] — the flow machine must be live for
               // the confirm's ResetMap_LevelToLoad push to be accepted
    fe.page = Page::Main;
    fe.cursor = 1; // Multiplayer [M MAIN_ITEMS 0x149; C_GOMULTIPLAYER fires on row 1]
    assert!(fe.action(Action::Confirm), "Multiplayer confirm must navigate");
    assert_eq!(fe.page, Page::MpJoin, "C_GOMULTIPLAYER's delayed msg 0x44 lands on 0x40000019");
    assert_eq!(fe.page.id(), 0x4000_0019);
}

/// The keyboard join: agent 1 joins, picks the default codename, readies.
fn join_agent1(fe: &mut FrontEnd) {
    fe.cursor = 0;
    assert!(fe.action(Action::Confirm), "✕ joins agent 1");
    assert_eq!(fe.mp_join[0].state, 1, "the panel's state 1 = Choose Codename [M mp_stuff+0x360]");
    assert_eq!(fe.mp_join[1].state, 0, "agents 2-4 stay unjoined (no pads — keyboard only)");
    assert!(fe.action(Action::Confirm), "✕ picks the codename");
    assert_eq!(fe.mp_join[0].state, 5, "state 5 = joined [M C_RBMPCNAME's write]");
    assert_eq!(fe.mp_join[0].codename, "Player 1", "the template codename [M Txt 0x1c3 + slot]");
    assert!(fe.action(Action::Confirm), "✕ readies");
    assert!(fe.mp_join[0].ready, "mpjoin+0x01 set [M C_RBMPSTART's state-5 arm]");
}

#[test]
fn main_to_join_to_scenario_flows() {
    let mut fe = FrontEnd::new();
    enter_mp(&mut fe);
    join_agent1(&mut fe);
    assert_eq!(fe.page, Page::MpScenario, "the ready arm reaches the scenario page [M → 0x4000001a]");
    assert_eq!(fe.page.id(), 0x4000_001a);
    // Sister: the scenario wheel opens on the row matching MPSettings+0x1A4 — fresh 0 = Quick
    // Game [M Menu_SelectItemInControl's fallback row 0].
    assert_eq!(fe.mp_scenario_row, 0, "a fresh profile's scenario cursor is Quick Game");
}

#[test]
fn scenario_confirm_moves_to_map_and_back_out() {
    let mut fe = FrontEnd::new();
    enter_mp(&mut fe);
    join_agent1(&mut fe);
    // Walk to a real scenario (row 1 = Arena / Deathmatch) and confirm.
    fe.mp_scenario_row = 1;
    assert!(fe.action(Action::Confirm), "the scenario confirm navigates");
    assert_eq!(fe.page, Page::MpMap, "C_SBMPSCEN's confirm opens the map page [M → 0x40000013]");
    assert_eq!(fe.mp_scenario, 1, "the confirm stores the scenario word [M MPSettings+0x1A4]");
    // Sister: the map confirm stores the level code and lands on the setup page.
    fe.mp_map_row = 0; // Skyrail 0x07000024
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_level, 0x0700_0024, "C_SBMPMAP's confirm stores the level code [M +0x1A8]");
    // Back out the whole chain [M the pages' own backs].
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::MpMap, "setup's Back returns to the map page");
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::MpScenario, "map's Back returns to the scenario page");
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::Main, "scenario's Back returns to the main menu [M → 0x40000002]");
    assert_eq!(fe.cursor, 1, "back on the Multiplayer row");
}

#[test]
fn setup_ffa_then_options_then_rules_back_out() {
    let mut fe = FrontEnd::new();
    enter_mp(&mut fe);
    join_agent1(&mut fe);
    fe.mp_scenario_row = 1; // Arena (FFA)
    fe.action(Action::Confirm);
    fe.mp_map_row = 0;
    fe.action(Action::Confirm);
    assert_eq!(fe.mp_setup_state, 3, "FFA asks Choose Character [M P_MPSETUP's mp_stuff state 3]");
    // FFA: the character confirm asks the Health Handicap, whose confirm lands on the hub [M
    // C_RBMPSETUP 0x20de58's 0x4b: 3 → 4 → 5 + Menu_MPAreWeReady → 0x40000012].
    fe.mp_setup_sel = 0; // Bond
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.mp_setup_state, 4, "then the Health Handicap [M the state-4 list −75..+100]");
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpOptions, "the picks flow to P_MPOPTIONS [M → 0x40000012]");
    assert_eq!(fe.mp_join[0].skin, 0);
    // Rules page: change the limits, close, and see the values kept.
    assert!(fe.action(Action::Down)); // AI Bots row
    assert!(fe.action(Action::Down)); // Game Rules
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpRules);
    let (t0, f0) = (fe.mp_time_limit, fe.mp_frag_limit);
    assert!(fe.action(Action::Right), "▶ steps Duration");
    assert_ne!(fe.mp_time_limit, t0, "the duration list moved");
    assert!(fe.action(Action::Down));
    assert!(fe.action(Action::Right));
    assert_eq!(fe.mp_frag_limit, f0 + 1, "the points list stepped");
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpOptions, "the rules page closes to the hub [M Manager 0x5f]");
    // Sister: Player Mods and Enviro-Mods close the same way.
    fe.mp_options_row = 3;
    fe.action(Action::Confirm);
    assert_eq!(fe.page, Page::MpPlayerMods);
    fe.action(Action::Confirm);
    assert_eq!(fe.page, Page::MpOptions);
    fe.mp_options_row = 4;
    fe.action(Action::Confirm);
    assert_eq!(fe.page, Page::MpEnviroMods);
    fe.action(Action::Back);
    assert_eq!(fe.page, Page::MpOptions);
}

#[test]
fn locked_scenario_and_ravine_bots_gate() {
    let mut fe = FrontEnd::new();
    enter_mp(&mut fe);
    join_agent1(&mut fe);
    // Uplink is row 4 — reward-locked on a fresh profile.
    fe.mp_scenario_row = 4;
    assert!(!fe.action(Action::Confirm), "a locked scenario refuses [M C_SBMPSCEN:40-42]");
    assert_eq!(fe.page, Page::MpScenario, "the page did not change");
    // Sister: the unlock_all shortcut opens Uplink [M the gate's other arm].
    fe.unlock_all = true;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpMap);
    fe.unlock_all = false;   // back to the fresh profile for the Ravine checks
    // Ravine (row 7) hides the AI Bots row.
    fe.mp_map_row = 7;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.mp_level, 0x0700_004b);
    fe.mp_setup_state = 4;
    fe.action(Action::Confirm); // the handicap pick → options
    assert_eq!(fe.page, Page::MpOptions);
    assert!(!fe.mp_options_ai_bots_open(), "Ravine clears the AI Bots row [M 0x217678:20-25]");
    fe.mp_options_row = 1;
    assert!(!fe.action(Action::Confirm), "the locked AI Bots row refuses");
    assert_eq!(fe.page, Page::MpOptions);
}

#[test]
fn bots_flow_picks_and_counts() {
    let mut fe = FrontEnd::new();
    enter_mp(&mut fe);
    join_agent1(&mut fe);
    fe.mp_scenario_row = 1;
    fe.action(Action::Confirm);
    fe.mp_map_row = 1; // Fort Knox — bots allowed
    fe.action(Action::Confirm);
    fe.action(Action::Confirm); // FFA character pick → options
    assert_eq!(fe.page, Page::MpOptions);
    // → AI Bots → Setup Bot 1 → the wheel opens on the record's character [M 6 = Snow Guard].
    fe.mp_options_row = 1;
    fe.action(Action::Confirm);
    assert_eq!(fe.page, Page::MpBots);
    assert_eq!(fe.mp_bots_row, 1);
    fe.action(Action::Confirm);
    assert_eq!(fe.page, Page::MpBotChoose);
    assert_eq!(fe.mp_bot_char, 6, "the wheel opens on the slot's character [M P_MPBOTCHOOSE]");
    // A locked character (Jaws, row 24) refuses; a free one (Ninja, row 11) lands on bot setup.
    fe.mp_bot_char = 24;
    assert!(!fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpBotChoose);
    fe.mp_bot_char = 11;
    assert!(fe.action(Action::Confirm), "Ninja (preset 11) is unlocked");
    assert_eq!(fe.page, Page::MpBotSetup);
    assert!(fe.mp_bots[0].enabled, "the choose sets record+0x0E [M C_SBMPBTCHOOSE]");
    assert_eq!(fe.mp_bots[0].character, 11);
    assert!(!fe.mp_bots[0].team_mi6, "Ninja's preset side is Phoenix [M Menu_IsBotGood]");
    assert_eq!(fe.mp_bot_count(), 1, "the enabled bot counts [M Menu_PrepareBots]");
    // Confirm closes back to the bots list [M Manager 0x5f twice].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpBots);
    // Continue → the hub → the start checks: 1 human + 1 bot = 2 players, FFA ok.
    fe.mp_bots_row = 0;          // the Continue row (the confirm left the cursor on the slot row)
    fe.action(Action::Confirm); // row 0 = Continue
    assert_eq!(fe.page, Page::MpOptions);
    fe.mp_options_row = 0;
    assert!(fe.action(Action::Confirm), "the start checks pass with 2 players");
    assert_eq!(fe.page, Page::MpConfirm);
}

#[test]
fn start_checks_refuse_single_player() {
    let mut fe = FrontEnd::new();
    enter_mp(&mut fe);
    join_agent1(&mut fe);
    fe.mp_scenario_row = 1;
    fe.action(Action::Confirm);
    fe.mp_map_row = 1;
    fe.action(Action::Confirm);
    fe.action(Action::Confirm);
    fe.mp_options_row = 0;
    assert!(!fe.action(Action::Confirm), "one player alone cannot start [M the 0x39e box]");
    assert_eq!(fe.mp_error, 0x39e, "the error label id lands in mp_error");
    assert_eq!(fe.page, Page::MpOptions, "the page stays");
    // Sister: the same single human WITH a bot passes (the bots flow test covers that path
    // end-to-end; here enable a bot directly and retry).
    fe.mp_bots[0].enabled = true;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpConfirm);
}

#[test]
fn quick_game_lands_on_confirm_with_three_bots() {
    let mut fe = FrontEnd::new();
    enter_mp(&mut fe);
    join_agent1(&mut fe);
    // Quick Game is row 0 — straight to the confirm page [M C_SBMPSCEN:48-145].
    fe.mp_scenario_row = 0;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpConfirm, "Quick Game skips the setup pages [M → 0x40000049]");
    assert_eq!(fe.mp_scenario, 1, "the quick start forces Deathmatch [M +0x1A4 = 1]");
    assert_eq!(fe.mp_time_limit, 10);
    assert_eq!(fe.mp_frag_limit, 10);
    assert_eq!(fe.mp_bot_count(), 3, "the quick start enables Drake/Kiko/Rook");
    assert_eq!(fe.mp_bots[0].character, 1);
    assert_eq!(fe.mp_bots[1].character, 3);
    assert_eq!(fe.mp_bots[2].character, 2);
    assert!(fe.mp_bots.iter().take(3).all(|b| !b.team_mi6), "all three are Phoenix");
    // Sister: the confirm's handover — the match settings mirror P_MPCONFIRM's writes and the
    // level request rides the loader.
    assert!(fe.action(Action::Confirm), "the confirm page's ✕ starts the match");
    let s = fe.mp_match_settings.as_ref().expect("the match config lands for the host");
    assert_eq!(s.scenario, 1);
    assert_eq!(s.level_code, fe.mp_level);
    assert_eq!(s.human_count, 1);
    assert_eq!(s.bot_count, 3);
    assert_eq!(s.time_limit, 600, "the confirm converts minutes to seconds [M the ×0x3c]");
    assert_eq!(s.master, 1, "[M +0x180 = 1]");
    assert_eq!(s.players[0].team, 1, "the quick start's slot-0 team is MI6 [M disasm 0x20c7f8]");
    assert_eq!(fe.requested_level, Some(fe.mp_level), "the arena request rides the loader");
    // The flow machine runs the load to the arena [M ResetMap_LevelToLoad + the phases; the
    // confirm's fade hold (8 headless frames) sits on top of the load state].
    for _ in 0..16 { fe.tick(); }
    while fe.flow.get_state() == nf_view::menu::ST_MAPLOAD { fe.flow.load_tick(); }
    assert_eq!(fe.flow.current_level, fe.mp_level, "the loader reaches the arena");
}

#[test]
fn team_scenario_asks_team_then_character() {
    let mut fe = FrontEnd::new();
    enter_mp(&mut fe);
    join_agent1(&mut fe);
    // Team Arena (row 2, scenario 0x20000002) has the team bit.
    fe.mp_scenario_row = 2;
    fe.action(Action::Confirm);
    fe.mp_map_row = 1;
    fe.action(Action::Confirm);
    assert_eq!(fe.mp_setup_state, 2, "a team game asks Choose Team [M P_MPSETUP's state 2]");
    // The team list has exactly two items (Phoenix/MI6).
    fe.mp_setup_sel = 1;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.mp_join[0].team, 1, "the selection IS the team [M mpjoin+4]");
    assert_eq!(fe.mp_setup_state, 3, "then the character list [M C_RBMPSETUP's 2 → 3]");
    fe.mp_setup_sel = 4; // Alura — MI6 side
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.mp_setup_state, 4, "then the Health Handicap [M C_RBMPSETUP's 3 → 4]");
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpOptions);
    // Sister: a lone MI6 human in a team game fails the both-sides check.
    fe.mp_options_row = 0;
    assert!(!fe.action(Action::Confirm), "an empty Phoenix side refuses [M the 0x389 box]");
    assert_eq!(fe.mp_error, 0x389);
}

#[test]
fn the_lists_match_the_game_rows_headless() {
    // The tables the flow walks carry the game's own row counts and ids (the ISO-gated
    // mp_menu_lists_live.rs re-pins the bytes from the disc).
    assert_eq!(MP_SCENARIO_ROWS.len(), 13, "mp_scenario's 13 rows [M symtab size 312 = 13×0x18]");
    assert_eq!(MP_MAP_ROWS.len(), 8, "mp_level's 8 rows [M symtab 192]");
    assert_eq!(MP_OPTIONS_ROWS.len(), 5, "mp_options' 5 rows [M the confirm's switch]");
    assert_eq!(MP_SCENARIO_ROWS[1].3, 1, "Arena = Deathmatch [M MPSettings+0x1A4 = 1]");
    assert_eq!(MP_SCENARIO_ROWS[2].3, 0x2000_0002, "Team Arena");
    assert_eq!(MP_SCENARIO_ROWS[12].3, 0x6000_1000, "Team KOTH");
    // Sister: the map rows' level codes are the arenas the ISO ships.
    for r in MP_MAP_ROWS { assert_eq!((r.3 >> 16) & 0xffff, 0x0700, "an on-foot arena code"); }
    assert!(MP_MAP_ROWS.iter().any(|r| r.3 == 0x0700_004b), "Ravine is the 8th arena");
    assert!(!MP_MAP_ROWS.iter().any(|r| r.3 == 0x0700_0021), "0x07000021 is absent from the table [D]");
}
