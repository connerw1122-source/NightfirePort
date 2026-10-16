//! Local-MP end-to-end, driven ONLY through `action()` / `tick()` / host-side calls from
//! `FrontEnd::new()` — no `page`/`cursor`/state assignments anywhere (the playtest 1015-2231
//! rule: the older flow tests jumped to pages by assignment, so the real paths were never
//! exercised). This file walks EVERY scenario row, EVERY map row, both team/FFA setups, every
//! options-hub sub-page, the back-out of every page at every prompt, the locked rows' refusals
//! and the debriefing's return, ending each full path in `requested_level` + `mp_match_settings`
//! validated against the choices made.

use nf_view::menu::{
    Action, FrontEnd, Page, MP_LOCKED_SCENARIO_DESC, MP_MAP_ROWS, MP_SCENARIO_ROWS,
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

/// Main → Multiplayer → Local → join (codename, ready) → the scenario page [M P_MPJOIN
/// 0x20ca00's ready arm → 0x4000001a].
fn to_scenario(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, Action::Down), "the Multiplayer row");
    assert_eq!(fe.cursor, 1);
    assert!(act(fe, Action::Confirm), "Multiplayer");
    assert_eq!(fe.page, Page::MpMode);
    assert!(act(fe, Action::Confirm), "Local");
    assert_eq!(fe.page, Page::MpJoin);
    assert!(act(fe, Action::Confirm), "join");
    assert_eq!(fe.mp_join[0].state, 1, "the codename prompt [M mp_stuff+0x360]");
    assert!(act(fe, Action::Confirm), "codename");
    assert_eq!(fe.mp_join[0].state, 5, "joined [M C_RBMPCNAME's write]");
    assert!(act(fe, Action::Confirm), "ready");
    assert!(fe.mp_join[0].ready, "mpjoin+1 set [M C_RBMPSTART's state-5 arm]");
    assert_eq!(fe.page, Page::MpScenario);
}

/// The scenario wheel opens on row 0: walk `row` Downs, then Confirm. Returns nothing — the
/// caller asserts the landing (the page differs for Quick Game / locked rows / the map wheel).
fn to_scenario_row(fe: &mut FrontEnd, row: usize) {
    for _ in 0..row {
        assert!(act(fe, Action::Down), "the scenario wheel scrolls to {row}");
    }
    assert_eq!(fe.mp_scenario_row, row);
}

/// From the map wheel: walk `row` Downs, Confirm, and land on the setup page — whose first
/// prompt is the team list for a team scenario and the character list otherwise [M P_MPSETUP
/// 0x20d7a8 picks the prompt from the scenario's team bit].
fn to_map_row(fe: &mut FrontEnd, row: usize, team: bool) {
    assert_eq!(fe.page, Page::MpMap);
    for _ in 0..row {
        assert!(act(fe, Action::Down), "the map wheel scrolls to {row}");
    }
    assert!(act(fe, Action::Confirm), "the map");
    assert_eq!(fe.mp_level, MP_MAP_ROWS[row].3, "the confirm stores the level [M +0x1A8]");
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, if team { 2 } else { 3 }, "the prompt follows the team bit [M P_MPSETUP 0x20d7a8]");
}

/// The setup prompts, Confirm through each (the defaults: team Phoenix, character Bond,
/// handicap "0") → the options hub [M C_RBMPSETUP 0x20de58's 0x4b: 2 → 3 → 4 → 5, then
/// Menu_MPAreWeReady → 0x40000012].
fn setup_through(fe: &mut FrontEnd, team: bool) {
    assert_eq!(fe.page, Page::MpSetup);
    if team {
        assert!(act(fe, Action::Confirm), "the team");
        assert_eq!(fe.mp_setup_state, 3, "then the character [M 2 → 3]");
        assert_eq!(fe.mp_join[0].team, 0, "the selection IS the team [M mpjoin+4]");
    }
    assert!(act(fe, Action::Confirm), "the character");
    assert_eq!(fe.mp_setup_state, 4, "then the Health Handicap [M 3 → 4]");
    assert!(act(fe, Action::Confirm), "the handicap");
    assert_eq!(fe.page, Page::MpOptions);
    assert_eq!(fe.mp_options_row, 0, "the hub opens on Continue");
}

/// From the hub: AI Bots → Setup Bot 1 → the wheel's standing character → the stats page →
/// close → back to the hub's Continue row. One enabled bot = the second player the start
/// checks demand [M C_SBMPOPTIONS's humans+bots > 1].
fn add_bot_slot1(fe: &mut FrontEnd) {
    assert!(act(fe, Action::Down), "the AI Bots row");
    assert_eq!(fe.mp_options_row, 1);
    assert!(act(fe, Action::Confirm), "AI Bots");
    assert_eq!(fe.page, Page::MpBots);
    assert_eq!(fe.mp_bots_row, 1, "the list opens on the slot row [M mp_stuff[0x37a]+1]");
    assert!(act(fe, Action::Confirm), "Setup Bot 1");
    assert_eq!(fe.page, Page::MpBotChoose);
    assert_eq!(fe.mp_bot_char, 6, "the wheel opens on the record's character [M P_MPBOTCHOOSE]");
    assert!(act(fe, Action::Confirm), "the standing character (Snow Guard)");
    assert_eq!(fe.page, Page::MpBotSetup);
    assert!(fe.mp_bots[0].enabled, "the choose enables the record [M C_SBMPBTCHOOSE]");
    assert!(act(fe, Action::Confirm), "the stats page closes [M Manager 0x5f twice]");
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(fe, Action::Up), "the Continue row");
    assert_eq!(fe.mp_bots_row, 0);
    assert!(act(fe, Action::Confirm), "the bots list's Continue [M C_SBBOTS:15]");
    assert_eq!(fe.page, Page::MpOptions);
    assert!(act(fe, Action::Up), "back on the hub's Continue row");
    assert_eq!(fe.mp_options_row, 0);
}

/// The hub's Continue → the start checks → the confirm page → the match request. Returns the
/// requested level code.
fn confirm_the_match(fe: &mut FrontEnd) -> u32 {
    assert!(act(fe, Action::Confirm), "Continue passes the start checks");
    assert_eq!(fe.page, Page::MpConfirm);
    assert!(act(fe, Action::Confirm), "the summary's ✕ starts the match [M P_MPCONFIRM]");
    let want = fe.requested_level.expect("the match requests its arena");
    assert!(fe.mp_match_settings.is_some(), "the settings ride the request");
    assert!(!act(fe, Action::Confirm), "a second ✕ is refused while one is pending [M MenuManager_Delete]");
    assert_eq!(fe.requested_level, Some(want), "the request stands");
    want
}

#[test]
fn every_scenario_row_opens_the_map_or_refuses() {
    for row in 0..MP_SCENARIO_ROWS.len() {
        let reward = MP_SCENARIO_ROWS[row].4;
        let mut fe = FrontEnd::new();
        to_scenario(&mut fe);
        to_scenario_row(&mut fe, row);
        if reward != 0 {
            // Locked rows refuse Confirm and keep the wheel [M C_SBMPSCEN:40-42's +0x10 gate].
            assert!(!act(&mut fe, Action::Confirm), "scenario row {row} is locked and refuses");
            assert_eq!(fe.page, Page::MpScenario, "row {row}: the wheel stays");
            assert_eq!(fe.mp_scenario, 0, "row {row}: nothing stored");
            assert!(fe.mp_row_locked(row), "row {row}: the draw's dimming flag");
            assert_eq!(fe.mp_row_desc(row), MP_LOCKED_SCENARIO_DESC, "row {row}: the locked description [M +0x14]");
            continue;
        }
        assert!(act(&mut fe, Action::Confirm), "row {row} confirms");
        if row == 0 {
            // Quick Game: the fixed config straight to the confirm page [M C_SBMPSCEN:48-145].
            assert_eq!(fe.page, Page::MpConfirm, "row 0 skips the setup pages");
            assert_eq!(fe.mp_scenario, 1, "the quick start forces Deathmatch [M +0x1A4 = 1]");
            assert_eq!(fe.mp_time_limit, 10);
            assert_eq!(fe.mp_frag_limit, 10);
            assert_eq!(fe.mp_bot_count(), 3, "Drake/Kiko/Rook enabled");
            assert!(act(&mut fe, Action::Confirm), "the summary's ✕ starts the match");
            let want = fe.requested_level.expect("the quick game requests its arena");
            assert!(fe.mp_match_settings.is_some());
            assert!(!act(&mut fe, Action::Confirm), "a second ✕ is refused [M MenuManager_Delete]");
            assert_eq!(fe.requested_level, Some(want));
            assert_eq!(want, MP_MAP_ROWS[0].3, "the quick start samples row 0 (Skyrail) [S the headless stand-in for Rand_Random % 7]");
            continue;
        }
        assert_eq!(fe.page, Page::MpMap, "row {row}: the map wheel opens [M → 0x40000013]");
        assert_eq!(fe.mp_scenario, MP_SCENARIO_ROWS[row].3, "row {row}: the confirm stores the scenario word [M 0x20c9c8]");
        // The map wheel's first row lands on the setup page, whose prompt follows the team bit.
        let team = MP_SCENARIO_ROWS[row].3 & 0x2000_0000 != 0;
        to_map_row(&mut fe, 0, team);
        setup_through(&mut fe, team);
        assert_eq!(fe.page, Page::MpOptions, "row {row}: the picks reach the hub");
    }
}

#[test]
fn every_map_row_carries_its_arena_to_the_request() {
    for map in 0..MP_MAP_ROWS.len() {
        let mut fe = FrontEnd::new();
        to_scenario(&mut fe);
        to_scenario_row(&mut fe, 1); // Arena (FFA) — the plain walk
        assert!(act(&mut fe, Action::Confirm), "the scenario");
        to_map_row(&mut fe, map, false);
        setup_through(&mut fe, false);
        if map == 7 {
            // Ravine: the AI Bots row is locked [M P_MPOPTIONS 0x217678:20-25] and one human
            // alone fails the game's own humans+bots > 1 [M C_SBMPOPTIONS 0x20e818's 0x39e arm]
            // — the port has no second pad, so the solo Ravine match correctly never starts.
            assert!(act(&mut fe, Action::Down), "the AI Bots row");
            assert!(!act(&mut fe, Action::Confirm), "Ravine's AI Bots row refuses");
            assert_eq!(fe.page, Page::MpOptions, "the hub stays");
            assert_eq!(fe.mp_row_desc(1), 0x0100_00ae, "the Ravine bots lock text [L 0x010000ae]");
            assert!(act(&mut fe, Action::Up), "back on Continue");
            assert!(!act(&mut fe, Action::Confirm), "Continue refuses 1 human + 0 bots");
            assert_eq!(fe.mp_error, 0x39e, "the two-player box [M C_SBMPOPTIONS]");
            assert!(act(&mut fe, Action::Confirm), "Enter dismisses");
            assert_eq!(fe.requested_level, None, "no Ravine request");
            continue;
        }
        add_bot_slot1(&mut fe);
        let want = confirm_the_match(&mut fe);
        assert_eq!(want, MP_MAP_ROWS[map].3, "map row {map}: the request is the picked arena");
        let s = fe.mp_match_settings.as_ref().unwrap();
        assert_eq!(s.level_code, MP_MAP_ROWS[map].3, "map row {map}: the settings carry the arena [M +0x1A8]");
        assert_eq!(s.scenario, 1, "map row {map}: Arena's scenario word");
        assert_eq!(s.bot_count, 1, "map row {map}: the bot rides");
        assert_eq!(s.human_count, 1);
    }
}

/// Every options-hub sub-page walked by actions: the Rules / Player Mods / Enviro-Mods values
/// move, the bots get set up, and the choices that the game keeps in MPSettings reach
/// `mp_match_settings` — including the friendly-fire handover the port used to drop [M
/// MPSettings+0x198; MP_RegisterBulletHit 0x1857c0:43-57].
#[test]
fn every_options_subpage_moves_its_values_and_rides_them() {
    let mut fe = FrontEnd::new();
    to_scenario(&mut fe);
    to_scenario_row(&mut fe, 1); // Arena
    assert!(act(&mut fe, Action::Confirm));
    to_map_row(&mut fe, 0, false); // Skyrail
    setup_through(&mut fe, false);
    // Game Rules: Duration 10 → 20, Points 10 → 12 [M P_MPRULES 0x20f790 — the Duration list is
    // 1..10, 15, 20, 30, Unlimited; the Points list steps by 1].
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Down));
    assert_eq!(fe.mp_options_row, 2);
    assert!(act(&mut fe, Action::Confirm), "Game Rules");
    assert_eq!(fe.page, Page::MpRules);
    assert!(act(&mut fe, Action::Right), "▶ steps Duration");
    assert_eq!(fe.mp_time_limit, 15);
    assert!(act(&mut fe, Action::Right));
    assert_eq!(fe.mp_time_limit, 20);
    assert!(act(&mut fe, Action::Down), "the Points row");
    assert!(act(&mut fe, Action::Right));
    assert!(act(&mut fe, Action::Right));
    assert_eq!(fe.mp_frag_limit, 12);
    assert!(act(&mut fe, Action::Confirm), "the rules page closes to the hub [M Manager 0x5f]");
    assert_eq!(fe.page, Page::MpOptions);
    // The hub's row is where the Rules page was opened from — 2; Player Mods is the next row.
    assert!(act(&mut fe, Action::Down), "Player Mods");
    assert_eq!(fe.mp_options_row, 3);
    assert!(act(&mut fe, Action::Confirm), "Player Mods");
    assert_eq!(fe.page, Page::MpPlayerMods);
    assert!(act(&mut fe, Action::Right), "friendly fire 0 → 1");
    assert_eq!(fe.mp_friendly_fire, 1);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Right), "weapon set 0 → 1");
    assert!(act(&mut fe, Action::Right), "weapon set 1 → 2");
    assert_eq!(fe.mp_weapon_set, 2);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Right), "the +0x1BC mod toggles on");
    assert_eq!(fe.mp_player_mods[0], 1);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Right), "the +0x1C8 mod toggles off");
    assert_eq!(fe.mp_player_mods[1], 0);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Right), "radar 1 → 0");
    assert_eq!(fe.mp_radar, 0);
    assert!(act(&mut fe, Action::Confirm), "the mods page closes");
    assert_eq!(fe.page, Page::MpOptions);
    // Enviro-Mods: the row after Player Mods — view distance, row 1 on, the LOCKED Explosive
    // Scenery row (◀ ▶ are no-ops [M P_MPENVIROMODS's single 0x10 node]), +0x1D0 on, vehicles
    // Off → Tanks [M 0x210030's rows].
    assert!(act(&mut fe, Action::Down), "Enviro-Mods");
    assert_eq!(fe.mp_options_row, 4);
    assert!(act(&mut fe, Action::Confirm), "Enviro-Mods");
    assert_eq!(fe.page, Page::MpEnviroMods);
    assert!(act(&mut fe, Action::Right), "view distance 2 (Random) → 0");
    assert_eq!(fe.mp_envmods[0], 0);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Right), "row 1 off → on");
    assert_eq!(fe.mp_envmods[1], 1);
    assert!(act(&mut fe, Action::Down));
    assert!(!fe.mp_explosive_unlocked(), "a fresh profile has no Explosive Scenery reward");
    assert!(!act(&mut fe, Action::Right) || fe.mp_envmods[2] == 0, "the locked row's ◀ ▶ cannot move it");
    assert_eq!(fe.mp_envmods[2], 0);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Right), "row 3 off → on");
    assert_eq!(fe.mp_envmods[3], 1);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Right), "vehicles Off → Tanks");
    assert_eq!(fe.mp_envmods[4], 1);
    assert!(act(&mut fe, Action::Confirm), "the enviro page closes");
    assert_eq!(fe.page, Page::MpOptions);
    assert_eq!(fe.mp_envmods[2], 0x10, "the close writes the locked row's node back [M 0x210424]");
    // AI Bots: the hub's row is 4 after Enviro-Mods — 3 Ups to the AI Bots row. Slot 1 → Ninja
    // (row 11) with edited stats; slot 2 refuses the locked tuxedo Bond (12) and takes Rook (2)
    // [M C_SBMPBTCHOOSE's unlock gate].
    for _ in 0..3 {
        assert!(act(&mut fe, Action::Up), "up to the AI Bots row");
    }
    assert_eq!(fe.mp_options_row, 1);
    assert!(act(&mut fe, Action::Confirm), "AI Bots");
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(&mut fe, Action::Confirm), "Setup Bot 1");
    for _ in 0..5 {
        assert!(act(&mut fe, Action::Down), "the wheel scrolls to Ninja");
    }
    assert_eq!(fe.mp_bot_char, 11);
    assert!(act(&mut fe, Action::Confirm), "Ninja is unlocked [M Menu_UnlockMPSkins rows 0-11]");
    assert_eq!(fe.page, Page::MpBotSetup);
    assert!(act(&mut fe, Action::Down), "the Speed row");
    assert!(act(&mut fe, Action::Right), "Speed steps — Ninja's preset Fast (2) wraps to Slow");
    assert_eq!(fe.mp_bots[0].speed, 0);
    for _ in 0..4 {
        assert!(act(&mut fe, Action::Down));
    }
    assert!(act(&mut fe, Action::Right), "Hit points 200 → 250 [M the 50..300 list]");
    assert_eq!(fe.mp_bots[0].hit_points, 250);
    assert!(act(&mut fe, Action::Confirm), "the stats store back [M +0x11 dirty = 1]");
    assert!(fe.mp_bots[0].dirty);
    assert_eq!(fe.page, Page::MpBots);
    assert_eq!(fe.mp_bots_row, 1);
    assert!(act(&mut fe, Action::Down), "Setup Bot 2");
    assert!(act(&mut fe, Action::Confirm), "the slot pick opens the choose wheel [M :20]");
    assert_eq!(fe.mp_bot_char, 7, "slot 2's record character [M DAT_002deecc]");
    for _ in 0..5 {
        assert!(act(&mut fe, Action::Down), "12 = the locked tuxedo Bond");
    }
    assert_eq!(fe.mp_bot_char, 12);
    assert_eq!(fe.mp_row_desc(12), 0x0100_00ad, "the locked character text [L +0x14]");
    assert!(!act(&mut fe, Action::Confirm), "a locked character refuses");
    assert_eq!(fe.page, Page::MpBotChoose);
    for _ in 0..10 {
        assert!(act(&mut fe, Action::Up), "back up to Rook");
    }
    assert_eq!(fe.mp_bot_char, 2);
    assert!(act(&mut fe, Action::Confirm), "Rook is unlocked");
    assert_eq!(fe.page, Page::MpBotSetup);
    assert!(act(&mut fe, Action::Confirm), "close");
    assert_eq!(fe.page, Page::MpBots);
    // The close left the list on the slot row (2): 2 Ups to the Continue row.
    assert_eq!(fe.mp_bots_row, 2);
    assert!(act(&mut fe, Action::Up), "up a row");
    assert!(act(&mut fe, Action::Up), "the Continue row");
    assert_eq!(fe.mp_bots_row, 0);
    assert!(act(&mut fe, Action::Confirm), "the bots list's Continue");
    assert_eq!(fe.page, Page::MpOptions);
    assert!(act(&mut fe, Action::Up), "back on Continue");
    assert_eq!(fe.mp_options_row, 0);
    // 1 human + 2 bots in an FFA passes [M C_SBMPOPTIONS's humans+bots > 1].
    let want = confirm_the_match(&mut fe);
    assert_eq!(want, 0x0700_0024, "Skyrail is the picked arena");
    let s = fe.mp_match_settings.as_ref().unwrap();
    assert_eq!(s.scenario, 1);
    assert_eq!(s.level_code, 0x0700_0024);
    assert_eq!(s.frag_limit, 12, "the Points choice rides [M +0x412]");
    assert_eq!(s.time_limit, 1200, "Duration minutes → seconds [M the ×0x3c]");
    assert_eq!(s.weapon_set, 2, "the weapon-set choice rides [M +0x436]");
    assert_eq!(s.radar, 0, "the radar choice rides [M +0x452]");
    assert_eq!(s.friendly_fire, 1, "the friendly-fire choice rides [M +0x198]");
    assert_eq!(s.bot_count, 2);
    assert_eq!(s.human_count, 1);
    assert_eq!(s.team_game, 0, "Arena is FFA [M MP_Init's (scenario >> 29) & 1]");
    assert_eq!(s.players[0].team, 0, "the setup's team pick rides");
    assert_eq!(s.players[0].skin, 0, "the setup's character pick rides");
    assert_eq!(fe.mp_envmods[2], 0, "the stale locked node masks off at start [M 0x20f708 andi 1]");
    assert_eq!(fe.mp_bots[0].character, 11, "Ninja stays in slot 1");
    assert_eq!(fe.mp_bots[1].character, 2, "Rook stays in slot 2");
}

/// Back out of EVERY page at EVERY prompt and walk forward again, on a TEAM scenario: the setup
/// page must re-enter on the team prompt for the scenario's team bit [M P_MPSETUP 0x20d7a8] —
/// the playtest's stale-prompt bug — and the match still reaches its request.
#[test]
fn back_out_of_every_page_and_walk_forward_again_team() {
    let mut fe = FrontEnd::new();
    to_scenario(&mut fe);
    // The scenario page's Back returns to Main on the Multiplayer row [M P_MPSCENARIO:29]. The
    // join page re-entry: an already-ready panel's ✕ does nothing and ✕ un-joins [M C_RBMPSTART's
    // state arms], so the walk forward re-joins through the codename list.
    assert!(act(&mut fe, Action::Back), "scenario Back → Main");
    assert_eq!(fe.page, Page::Main);
    assert_eq!(fe.cursor, 1, "back on the Multiplayer row");
    assert!(act(&mut fe, Action::Confirm), "Multiplayer");
    assert!(act(&mut fe, Action::Confirm), "Local");
    assert_eq!(fe.page, Page::MpJoin);
    assert!(!act(&mut fe, Action::Confirm), "a ready panel's ✕ falls through");
    assert!(act(&mut fe, Action::Back), "✕ un-joins [M C_RBMPSTART's 0x6b arm]");
    assert_eq!(fe.mp_join[0].state, 0);
    assert!(act(&mut fe, Action::Confirm), "join again");
    assert_eq!(fe.mp_join[0].state, 1);
    assert!(act(&mut fe, Action::Right), "the codename list moves");
    assert_eq!(fe.mp_join[0].codename_row, 1);
    assert!(act(&mut fe, Action::Left), "…and back");
    assert_eq!(fe.mp_join[0].codename_row, 0);
    assert!(act(&mut fe, Action::Confirm), "codename");
    assert!(act(&mut fe, Action::Confirm), "ready");
    assert_eq!(fe.page, Page::MpScenario);
    // Team Arena (row 2) → Fort Knox (map row 1) → the team prompt.
    to_scenario_row(&mut fe, 2);
    assert!(act(&mut fe, Action::Confirm), "Team Arena");
    assert_eq!(fe.mp_scenario, 0x2000_0002);
    assert!(act(&mut fe, Action::Down), "Fort Knox");
    assert!(act(&mut fe, Action::Confirm), "the map");
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 2, "the team prompt [M P_MPSETUP 0x20d7a8]");
    // Back at the team prompt pops to the map wheel [M C_RBMPSETUP's 0x6b: at 2 the page pops];
    // forward again re-enters on the team prompt.
    assert!(act(&mut fe, Action::Back), "the team prompt's Back");
    assert_eq!(fe.page, Page::MpMap);
    assert!(act(&mut fe, Action::Confirm), "the map again");
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 2, "the team bit re-enters on the team prompt");
    // Back at the character prompt returns to the team list in a team game [M 0x6b's 3 → 2];
    // forward again.
    assert!(act(&mut fe, Action::Confirm), "the team (Phoenix)");
    assert_eq!(fe.mp_setup_state, 3);
    assert!(act(&mut fe, Action::Back), "the character prompt's Back");
    assert_eq!(fe.mp_setup_state, 2, "a team game walks back to the team list");
    assert!(act(&mut fe, Action::Confirm), "the team again");
    // Back at the handicap prompt returns to the character list [M 0x6b's 4 → 3]; forward again.
    assert!(act(&mut fe, Action::Confirm), "the character (Bond)");
    assert_eq!(fe.mp_setup_state, 4);
    assert!(act(&mut fe, Action::Back), "the handicap prompt's Back");
    assert_eq!(fe.mp_setup_state, 3);
    assert!(act(&mut fe, Action::Confirm), "the character again");
    assert_eq!(fe.mp_setup_state, 4);
    assert!(act(&mut fe, Action::Confirm), "the handicap");
    assert_eq!(fe.page, Page::MpOptions);
    // The hub's Back re-runs P_MPSETUP's enter: the TEAM prompt, not the stale character one
    // (playtest 1015-2231) [M P_MPSETUP 0x20d7a8]. Forward again through all three prompts.
    assert!(act(&mut fe, Action::Back), "the hub's Back");
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 2, "the hub re-enters the setup on the team prompt");
    assert!(act(&mut fe, Action::Confirm), "the team");
    assert!(act(&mut fe, Action::Confirm), "the character");
    assert!(act(&mut fe, Action::Confirm), "the handicap");
    assert_eq!(fe.page, Page::MpOptions);
    // Each sub-page closes by Confirm AND by Back (the game's msg-0x4b read vs the 0x5f pop).
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Confirm), "Game Rules");
    assert!(act(&mut fe, Action::Back), "the rules page closes by Back too");
    assert_eq!(fe.page, Page::MpOptions);
    assert!(act(&mut fe, Action::Down), "Player Mods");
    assert!(act(&mut fe, Action::Confirm));
    assert!(act(&mut fe, Action::Confirm), "…closes by Confirm");
    assert_eq!(fe.page, Page::MpOptions);
    assert!(act(&mut fe, Action::Down), "Enviro-Mods");
    assert!(act(&mut fe, Action::Confirm));
    assert!(act(&mut fe, Action::Back), "…closes by Back");
    assert_eq!(fe.page, Page::MpOptions);
    // The bots chain: every wheel's Back walks back up one page.
    for _ in 0..4 {
        assert!(act(&mut fe, Action::Up), "back to the hub's Continue");
    }
    assert_eq!(fe.mp_options_row, 0);
    assert!(act(&mut fe, Action::Down), "AI Bots");
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(&mut fe, Action::Back), "the bots list's Back [M C_SBBOTS:15]");
    assert_eq!(fe.page, Page::MpOptions);
    // The hub's row is still the AI Bots row the wheel was opened from — Confirm re-enters it.
    assert_eq!(fe.mp_options_row, 1);
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(&mut fe, Action::Confirm), "Setup Bot 1");
    assert_eq!(fe.page, Page::MpBotChoose);
    assert!(act(&mut fe, Action::Back), "the choose wheel's Back");
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(&mut fe, Action::Confirm), "Setup Bot 1 again");
    // Alura (row 4) is MI6-side: the team game needs both sides [M Menu_IsBotGood].
    for _ in 0..2 {
        assert!(act(&mut fe, Action::Up));
    }
    assert_eq!(fe.mp_bot_char, 4);
    assert!(act(&mut fe, Action::Confirm), "Alura");
    assert_eq!(fe.page, Page::MpBotSetup);
    assert!(act(&mut fe, Action::Back), "the stats page's Back");
    assert_eq!(fe.page, Page::MpBots);
    assert!(act(&mut fe, Action::Up), "the Continue row");
    assert!(act(&mut fe, Action::Confirm), "the bots list's Continue");
    assert_eq!(fe.page, Page::MpOptions);
    assert!(act(&mut fe, Action::Up), "Continue");
    // The confirm page's Back returns to the hub [M P_MPCONFIRM's back]; forward again starts.
    assert!(act(&mut fe, Action::Confirm), "Continue passes the checks");
    assert_eq!(fe.page, Page::MpConfirm);
    assert!(act(&mut fe, Action::Back), "the confirm page's Back");
    assert_eq!(fe.page, Page::MpOptions);
    assert!(act(&mut fe, Action::Confirm), "Continue again");
    assert_eq!(fe.page, Page::MpConfirm);
    // The summary's ✕ starts the match; a second ✕ is refused [M P_MPCONFIRM's MenuManager_Delete].
    assert!(act(&mut fe, Action::Confirm), "the summary's ✕ starts the match");
    let want = fe.requested_level.expect("the match requests its arena");
    assert!(fe.mp_match_settings.is_some(), "the settings ride the request");
    assert!(!act(&mut fe, Action::Confirm), "a second ✕ is refused");
    assert_eq!(fe.requested_level, Some(want), "the request stands");
    assert_eq!(want, 0x0700_0027, "Fort Knox is the picked arena");
    let s = fe.mp_match_settings.as_ref().unwrap();
    assert_eq!(s.scenario, 0x2000_0002, "Team Arena's word rides");
    assert_eq!(s.team_game, 1, "the team bit derives [M MP_Init 0x183438:41]");
    assert_eq!(s.players[0].team, 0, "the human's Phoenix pick rides");
    assert_eq!(s.bot_count, 1);
}

/// The post-match debriefing: the host-side re-entry (the game's quit handoff [M C_GCPAUSE
/// 0x204408:146 — GameState+0x28 = 0x40000033]) lands the page, and its Confirm returns to the
/// scenario page on row 0 [M C_MPDBG 0x217860's 0x4b arm → 0x4000001a].
#[test]
fn the_debriefing_returns_to_the_scenario_page() {
    let mut fe = FrontEnd::new();
    to_scenario(&mut fe);
    assert!(act(&mut fe, Action::Confirm), "Quick Game (row 0) straight to the summary");
    assert_eq!(fe.page, Page::MpConfirm);
    assert!(act(&mut fe, Action::Confirm), "the match starts");
    assert!(fe.requested_level.is_some(), "the quick game's request went out");
    fe.mp_debrief_enter();
    assert_eq!(fe.page, Page::MpDebriefing, "the quit handoff lands the debriefing");
    assert_eq!(fe.page.id(), 0x4000_0033, "[M GameState+0x28 = 0x40000033]");
    assert!(act(&mut fe, Action::Confirm), "the debriefing's ✕");
    assert_eq!(fe.page, Page::MpScenario, "the debriefing returns to the scenario wheel [M C_MPDBG]");
    assert_eq!(fe.mp_scenario_row, 0, "the wheel re-opens on Quick Game [M C_MPDBG's 0x44 arg]");
}
