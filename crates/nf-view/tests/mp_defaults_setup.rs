//! mp-defaults-and-setup: the MP boot defaults (bootup_bootup 0x1c7e78), the Explosive-Scenery
//! reward gate (Menu_UnlockMPSettings 0x1ff698 + P_MPENVIROMODS 0x210030) and the three-step
//! per-agent setup flow (P_MPSETUP 0x20d7a8 + C_RBMPSETUP 0x20de58), headless — ids/numbers only.

use nf_view::menu::{Action, FrontEnd, Page, MP_CHARACTERS, MP_HANDICAPS};

/// A fresh front-end parked on the setup page with a team scenario (Team Arena, the PCSX2
/// comparison's page): Choose Team → Choose Character → Health Handicap.
fn team_setup_fe() -> FrontEnd {
    let mut fe = FrontEnd::new();
    fe.tick(); // the flow machine must be live for the confirms [M mp_menu_flow's harness]
    fe.page = Page::MpMap;
    fe.mp_scenario = 0x2000_0002; // Team Arena [M MP_SCENARIO_ROWS[2].3]
    fe.mp_map_row = 1;
    assert!(fe.action(Action::Confirm), "the map confirm opens the setup page");
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 2, "a team game asks Choose Team [M P_MPSETUP's state 2]");
    fe
}

/// A fresh front-end parked on the setup page with an FFA scenario (Arena): Choose Character
/// directly [M P_MPSETUP's state 3 for the no-team-bit arm].
fn ffa_setup_fe() -> FrontEnd {
    let mut fe = FrontEnd::new();
    fe.tick();
    fe.page = Page::MpMap;
    fe.mp_scenario = 1; // Arena, FFA
    fe.mp_map_row = 0;
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpSetup);
    assert_eq!(fe.mp_setup_state, 3, "FFA asks Choose Character [M P_MPSETUP's state 3]");
    fe
}

#[test]
fn boot_defaults_match_bootup_bootup() {
    // [M bootup_bootup 0x1c7e78: memset(MPSettings, 0, 0x1dc) then the field stores — +0x198=0,
    // +0x19C=10, +0x1A0=10, +0x1A4=1, +0x1AC=1, +0x1B4=0, +0x1B8=0, +0x1BC=0, +0x1C0=2, +0x1C4=1,
    // +0x1C8=1, +0x1CC=0, +0x1D0=0, +0x1D4=0; the roster +0x2c = 0. The EE snapshots read the same
    // [L ee_start_05.bin at MPSettings 0x2a47a0].]
    let fe = FrontEnd::new();
    assert_eq!(fe.mp_player_mods, [0, 1], "+0x1BC=0, +0x1C8=1 (Location Damage On) [M 0x1c7ffc/0x1c7ff8]");
    assert_eq!(fe.mp_envmods, [2, 0, 0, 0, 0], "+0x1C0=2 (Random) [M 0x1c7fac], the rest 0");
    assert_eq!(fe.mp_friendly_fire, 0, "+0x198 = 0 [M 0x1c7fe8]");
    assert_eq!(fe.mp_frag_limit, 10, "+0x19C = 10 [M 0x1c7fb4]");
    assert_eq!(fe.mp_time_limit, 10, "+0x1A0 = 10 [M 0x1c7fe4]");
    assert_eq!(fe.mp_weapon_set, 0, "+0x1B4 = 0 [M 0x1c7fe0]");
    assert_eq!(fe.mp_radar, 1, "+0x1C4 = 1 [M 0x1c8000]");
    assert_eq!(fe.mp_handicap, 0, "the roster's +0x2c boots 0 [M bootup_bootup's roster loop]");
    assert_eq!(fe.mp_bonus_mask, 0, "a fresh profile's bonus words read 0 [L ee snapshots]");
}

#[test]
fn explosive_scenery_locked_by_default() {
    // [M Menu_UnlockMPSettings 0x1ff698 sets mp_stuff[0x370] iff the profile's bonus mask has
    // reward bit 0x3e (type-3 id 0x3e — the shipped table's only type-3 entry, level 0x07000014
    // counter 3 [D 0x2bc920 row 10]); a fresh profile has no bits, so the row is locked.]
    let mut fe = FrontEnd::new();
    assert!(!fe.mp_explosive_unlocked(), "no reward bit → locked");
    assert_eq!(fe.mp_enviro_value_label(2), 0x0100_0076, "the locked row binds the single 0x1000076 node [M 0x210030's locked branch]");

    // ◀ ▶ cannot move the single-node row [M the locked list holds one node; no 0x49 arm].
    fe.page = Page::MpEnviroMods;
    fe.cursor = 2;
    assert!(fe.action(Action::Left), "the input is consumed");
    assert!(fe.action(Action::Right));
    assert_eq!(fe.mp_envmods[2], 0, "the locked row's value did not move");

    // Closing the page stores the current node's value — 0x10 while locked [M 0x210464's
    // msg-0x4b read-back of the Locked node].
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.page, Page::MpOptions);
    assert_eq!(fe.mp_envmods[2], 0x10, "the visit leaves +0x1D4 = 0x10 [M the single 0x10 node]");

    // Match start masks it back to 0/1 [M P_MPCONFIRM 0x20f708's andi 1 → 0x20f710].
    fe.mp_join[0].joined = true;
    assert!(fe.mp_confirm_start());
    assert_eq!(fe.mp_envmods[2], 0, "the &1 mask at match start");
}

#[test]
fn explosive_scenery_unlocks_with_reward_bit_0x3e() {
    let mut fe = FrontEnd::new();
    fe.mp_bonus_mask = 1u64 << 0x3e;
    assert!(fe.mp_explosive_unlocked(), "bit 0x3e = the Explosive-Scenery reward [M 0x1ff698's type-3/0x3e arm]");

    // The unlocked row's list is On(1) then Off(0) [M the unlocked branch binds 0x1b7@1, 0x1b6@0];
    // a stale locked 0x10 has no node to match, so the first-added node (On) shows [M
    // List_SendMessage's msg-0x1e no-match keeps the first node current].
    fe.page = Page::MpEnviroMods;
    fe.cursor = 2;
    fe.mp_envmods[2] = 0x10;
    assert_eq!(fe.mp_enviro_value_label(2), 0x1b7, "the stale 0x10 displays the first-added On node");
    assert!(fe.action(Action::Right));
    assert_eq!(fe.mp_envmods[2], 1, "the first ◀ ▶ moves off the On node");
    assert!(fe.action(Action::Right));
    assert_eq!(fe.mp_envmods[2], 0, "then Off");
    assert_eq!(fe.mp_enviro_value_label(2), 0x1b6);
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.mp_envmods[2], 0, "the close stores the current node's value");
}

#[test]
fn enviro_value_labels_match_the_msg_0x4c_lists() {
    // [M P_MPENVIROMODS 0x210030's msg-0x4c node lists: row 0 = 0x1b0/0x1b1/0x1b2 at 0/1/2;
    // rows 1/3 On-Off; vehicles 0x1b6/0x10002d8/0x10002d9/0x1b2.]
    let mut fe = FrontEnd::new();
    assert_eq!(fe.mp_enviro_value_label(0), 0x1b2, "row 0 boots 2 = the third item");
    fe.mp_envmods[1] = 1;
    assert_eq!(fe.mp_enviro_value_label(1), 0x1b7, "On");
    fe.mp_envmods[3] = 1;
    assert_eq!(fe.mp_enviro_value_label(3), 0x1b7, "On");
    fe.mp_envmods[4] = 2;
    assert_eq!(fe.mp_enviro_value_label(4), 0x0100_02d9, "Helicopters");
    fe.mp_envmods[4] = 1;
    assert_eq!(fe.mp_enviro_value_label(4), 0x0100_02d8, "Tanks");
    fe.mp_envmods[4] = 0;
    assert_eq!(fe.mp_enviro_value_label(4), 0x1b6, "Off");
}

#[test]
fn quick_start_writes_helicopters_and_clears_explosive() {
    // [M C_SBMPSCEN 0x20c600's row-0 stores: 0x20c734 +0x1CC = 2 (Helicopters), 0x20c748
    // +0x1D4 = 0; +0x1C0 is not written — the boot 2 persists.]
    let mut fe = FrontEnd::new();
    fe.mp_quick_start();
    assert_eq!(fe.mp_envmods[4], 2, "vehicles = Helicopters [M 0x20c734]");
    assert_eq!(fe.mp_envmods[2], 0, "Explosive Scenery cleared [M 0x20c748]");
    assert_eq!(fe.mp_envmods[0], 2, "view distance keeps the boot Random [M no +0x1C0 store]");
    assert_eq!(fe.mp_player_mods, [0, 0], "+0x1BC/+0x1C8 untouched — the boot 0/1 persist");
}

#[test]
fn team_setup_three_prompts_with_labels_images_values() {
    let mut fe = team_setup_fe();

    // Step 1 — Choose Team [M P_MPSETUP binds 0x37c and the Phoenix/MI6 items 0x1c7@0, 0x1c8@1].
    assert_eq!(fe.mp_setup_prompt_label(), 0x37c, "Choose Team");
    assert_eq!(fe.mp_setup_image(), 0x0300_01a3, "Phoenix's image on selection 0 [M the 0x49/0x54 arm]");
    assert_eq!(fe.mp_setup_value_text(None), "<0x1c7>", "the Phoenix item's label id");
    assert!(fe.action(Action::Right));
    assert_eq!(fe.mp_setup_image(), 0x0300_01a2, "MI6's image on selection 1 [M value 1 → 0x30001a2]");
    assert_eq!(fe.mp_setup_value_text(None), "<0x1c8>", "the MI6 item's label id");
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.mp_join[0].team, 1, "the selection IS the team [M mpjoin+4]");

    // Step 2 — Choose Character [M 0x37d; the image = Menu_GetItemFromHash(0x2df8f8, v, 0x1d)'s
    // item+0 = the character row's preview].
    assert_eq!(fe.mp_setup_state, 3);
    assert_eq!(fe.mp_setup_prompt_label(), 0x37d, "Choose Character");
    assert_eq!(fe.mp_setup_image(), MP_CHARACTERS[0].0, "Bond's portrait on the stored skin 0");
    assert!(fe.action(Action::Right));
    assert!(fe.action(Action::Right));
    assert_eq!(fe.mp_setup_image(), MP_CHARACTERS[2].0, "Rook's portrait on selection 2");
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.mp_join[0].skin, 2, "mpjoin+8 = the id [M 0x4b's state-3 arm]");

    // Step 3 — Health Handicap [M 0x296, the green cross 0x30001a1, the −75..+100 nodes; the fresh
    // roster +0x2c = 0 re-selects "0"].
    assert_eq!(fe.mp_setup_state, 4);
    assert_eq!(fe.mp_setup_prompt_label(), 0x296, "Health Handicap");
    assert_eq!(fe.mp_setup_image(), 0x0300_01a1, "the green cross [M the state-4 arm's 0x24 bind]");
    assert_eq!(fe.mp_setup_value_text(None), "0", "the fresh roster's handicap selects the 0 node [D 0x30d3a8]");
    assert!(fe.action(Action::Right));
    assert_eq!(fe.mp_setup_value_text(None), "+25", "[D 0x30d3b0]");
    assert!(fe.action(Action::Right));
    assert_eq!(fe.mp_setup_value_text(None), "+50", "[D 0x30d3b8]");
    assert!(fe.action(Action::Confirm));
    assert_eq!(fe.mp_handicap, 50, "MPSettings+slot·0x30+0x2c = the picked value [M 0x4b's state-4 arm]");

    // All picked → state 5 + the options hub [M Menu_MPAreWeReady → the 0x44/0x40000012 send].
    assert_eq!(fe.mp_setup_state, 5);
    assert!(fe.mp_join[0].ready, "mpjoin+1 = 1 [M the state-4 arm's DAT_002dee69 store]");
    assert_eq!(fe.page, Page::MpOptions);
    assert_eq!(fe.page.id(), 0x4000_0012, "P_MPOPTIONS' page id [M the 0x40000012 send]");
}

#[test]
fn handicap_reselects_the_stored_value() {
    // [M the state-4 entry re-selects MPSettings+slot·0x30+0x2c with msg 0x1e.]
    let mut fe = ffa_setup_fe();
    fe.mp_handicap = 25;
    assert!(fe.action(Action::Confirm)); // character → handicap
    assert_eq!(fe.mp_setup_state, 4);
    assert_eq!(fe.mp_setup_value_text(None), "+25", "the stored +25 node is current again");
    assert_eq!(MP_HANDICAPS[4], (25, "+25"));
}

#[test]
fn setup_back_walks_the_prompts() {
    // [M C_RBMPSETUP 0x20de58's 0x6b arm: 4 → 3 always, 3 → 2 only in team games.]
    let mut fe = team_setup_fe();
    assert!(fe.action(Action::Confirm)); // team → character
    assert!(fe.action(Action::Confirm)); // character → handicap
    assert_eq!(fe.mp_setup_state, 4);
    assert!(fe.action(Action::Back));
    assert_eq!(fe.mp_setup_state, 3, "the handicap steps back to the character");
    assert!(fe.action(Action::Back));
    assert_eq!(fe.mp_setup_state, 2, "a team game's character steps back to the team");
    assert_eq!(fe.mp_setup_sel, fe.mp_join[0].team as usize, "re-selected the stored team");
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::MpMap, "at the team prompt the page pops to the map wheel");

    // FFA: the character's back pops the page [M the 0x6b arm only re-asks the team in team games].
    let mut fe = ffa_setup_fe();
    assert!(fe.action(Action::Back));
    assert_eq!(fe.page, Page::MpMap);
}

#[test]
fn ffa_setup_never_asks_the_team() {
    let mut fe = ffa_setup_fe();
    assert!(fe.action(Action::Confirm)); // character → handicap
    assert_eq!(fe.mp_setup_state, 4);
    assert!(fe.action(Action::Confirm)); // handicap → done
    assert_eq!(fe.mp_setup_state, 5);
    assert_eq!(fe.page, Page::MpOptions);
}

#[test]
fn handicaps_match_the_handlers_nodes() {
    // [M C_RBMPSETUP's eight msg-0x10 nodes, values −75..+100; [D] the strings 0x30d390..0x30d3c8
    // — the same literals P_CNMPOPTIONS 0x2144d8's MPSettings+0x44 row uses.]
    assert_eq!(MP_HANDICAPS.len(), 8);
    let vals: Vec<i32> = MP_HANDICAPS.iter().map(|h| h.0).collect();
    assert_eq!(vals, [-75, -50, -25, 0, 25, 50, 75, 100]);
    let texts: Vec<&str> = MP_HANDICAPS.iter().map(|h| h.1).collect();
    assert_eq!(texts, ["-75", "-50", "-25", "0", "+25", "+50", "+75", "+100"]);
}
