//! save-progress-roundtrip: progress survives a restart, by the real path.
//!
//! The flow the driver asked for: create a codename; complete two missions (the results commit
//! + the save prompt Yes — the 635 card flow); drop the FrontEnd entirely (as an app restart
//! would); build a fresh FrontEnd on the same profile dir and load the codename. Assert the
//! mission list shows the same unlocked rows (including the driving pass-through), the medals
//! and best scores, the dossier rewards, and the MP rewards (the Explosive Scenery gate). Also
//! "Play without saving" leaves no file; a corrupted file reports Load failed (the card
//! message) and nothing loads. Temp profile dir only — never the real one.
//!
//! Evidence: the results save prompt P_NFRESULTS 0x208e78:24-52 + Menu_UpdateMessageBox
//! 0x1f7de8 case 1 (LS_Save under the codename); the load C_SBNFCN 0x20b390 mode 0 →
//! LS_GetLoadStuff 0x1cd2b0 applies the chain (MSSN → the unlock mask 0x2023a0, BNUS → the
//! bonus words); Menu_UnlockMPSettings 0x1ff698 runs on the MP-scenario page's open
//! [M P_MPSCENARIO_Handler 0x217348:23] and re-derives mp_stuff[0x370] (Explosive Scenery,
//! mask bit 0x3e — level 0x07000014's kind-3 descriptor [D 0x2bc920 row 10]) from the bonus
//! words; LOADERROR 0x10f138 for the corrupt load.

use nf_view::menu::{Action, FrontEnd, Outcome, Page, Stats, BTN_RETRY, BTN_YES};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nf-save-roundtrip-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// unlock-popups (619b): with a NEW reward standing the results routing hops to the 007 Bonus
/// page 0x40000038 first [M 0x208e78:48-53]; its own Confirm lands on the mission list [M
/// 0x209670:50-53]. A first completion earns a medal, so the hop happens.
fn through_bonus(fe: &mut FrontEnd, progress: &nf_game::progress::Progress) {
    if fe.page == Page::NfBonus {
        assert!(fe.new_reward, "the hop only happens on a new reward [M 0x1ffa10:78]");
        fe.bonus_enter(progress, fe.results_level, fe.new_reward);
        assert!(fe.action(Action::Confirm), "the bonus page's Confirm [M 0x209670:50-53]");
    }
}

/// The results commit + the save prompt Yes through the real card flow: Confirm opens the
/// "Do you want to save your progress?" box, the second Confirm answers Yes, the existing
/// codename draws the overwrite question, and the tick lands the write [M 0x208e78:24-52 +
/// ASK_OVERWRITE 0x10ee38 + SAVE_DATA 0x10fa08].
fn save_yes_through_card_flow(fe: &mut FrontEnd, progress: &nf_game::progress::Progress) {
    assert!(fe.action(Action::Confirm), "the prompt opens");
    assert!(fe.results_box, "the save prompt stands [M 0x1000206 box 0xb]");
    assert!(fe.action(Action::Confirm), "the Yes answer");
    assert_eq!(fe.card_message(), Some((4, 0x27d)), "ASK_OVERWRITE — the codename exists [M 0x10ee38]");
    assert!(fe.card_answer(BTN_YES), "Yes continues the flow");
    assert_eq!(fe.card_message(), Some((9, 0x26c)), "the saving message [M 0x10fa08 sub-8]");
    fe.tick();
    assert_eq!(fe.card_message(), None, "the write completed and the box closed");
    through_bonus(fe, progress);
    assert_eq!(fe.page, Page::MissionSelect, "the results route on [M 0x208e78:203-211]");
}

/// One action, then the frames a player would wait: the page-change frames and any intro /
/// attract movie run out (menus-e2e-mp's harness — the MP walk gets no page assignments).
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

/// The whole session the driver described, with the snapshot the restart is judged against.
#[test]
fn the_roundtrip_restores_the_progress() {
    let dir = temp_dir("roundtrip");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());

    // ── create the codename through the real keyboard flow ──────────────────────────────────
    // [M C_KEYBOARD 0x213a98's buffer at 0x34e028; the accept arms the create save and the
    // flow's tick writes it under the codename — memcard-save-flow's create_via_flow.]
    fe.cn_open(0x4000_0025);
    fe.cn_typed.clear();
    fe.cn_clear_on_key = false;
    // 8 chars: the game's codename buffer holds 8 + NUL [M the name buffer 0x2e1ae8].
    for c in "RNDTRIP1".chars() {
        fe.cn_type_char(c);
    }
    assert!(fe.cn_press(FrontEnd::CN_KEY_CONFIRM), "the accept arms the save");
    fe.tick();
    assert_eq!(fe.card.ops_done, 1, "the create wrote once");
    assert_eq!(fe.active_profile.as_deref(), Some("RNDTRIP1"));
    let path = nf_game::profile::Profile::path_in(&dir, "RNDTRIP1");
    assert!(path.exists(), "one file per codename [M 0x1f7de8]");

    // ── mission 1: 07000005 (campaign row 1) — the results commit + the save prompt Yes ─────
    let mut progress = nf_game::progress::Progress::default();
    progress.stats[0].dispatched = 2;
    progress.stats[0].shots_fired = 10;
    progress.stats[0].shots_hit_enemy = 6;
    fe.mission = 1;
    fe.mission_ended(Outcome::Complete, Stats { shots_fired: 30, shots_hit: 20, accuracy_pct: 66, time_s: 100 });
    fe.requested_level = None; // the headless leg skips the outro wrap
    assert!(fe.results_enter(&mut progress, 0x0700_0005, 6, 2, None), "the fields built [M 0x208e78:77]");
    assert!(fe.results_improved, "the commit improved (first completion) [M DoneBetter]");
    let medal1 = fe.results.as_ref().expect("the results fields").medal;
    assert!((1..=4).contains(&medal1), "the first completion earns a medal, got {medal1}");
    for k in 1..=4u32 {
        assert_eq!(progress.has_medal(0x0700_0005, k, 0), k <= medal1, "medal bit {k} matches the award");
    }
    // The Explosive-Scenery reward itself: level 0x07000014's kind-3 id-0x3e bit — the same
    // setter Menu_SetLevelBonus's award loop runs [M PlrStarts_ProcessRewardCounter 0x199210's
    // SET arm; D RewardsTable row 10 desc[2] = bit 62 kind 3]. The port can't play Countdown
    // headlessly, so the stamp stands in for that mission's own results commit.
    assert!(!progress.process_reward_counter(0x0700_0014, 2, 0, false), "the reward bit was clear");
    progress.process_reward_counter(0x0700_0014, 2, 0, true); // [M 0x199210's set arm]
    assert!(progress.has_medal(0x0700_0014, 2, 0), "the Countdown reward bit stands");
    // The host merges the live layer into the profile BEFORE the save [M the game writes its
    // MSSN/BNUS from the same live state the results page reads].
    fe.sync_profile_progress(&progress);
    save_yes_through_card_flow(&mut fe, &progress);

    // ── mission 2: 07000001 (Double Cross, row 4 — reached through the driving pass-through) ─
    progress.stats[0].dispatched = 5;
    progress.stats[0].shots_fired = 20;
    progress.stats[0].shots_hit_enemy = 12;
    fe.mission = 4;
    fe.mission_ended(Outcome::Complete, Stats { shots_fired: 44, shots_hit: 30, accuracy_pct: 68, time_s: 190 });
    fe.requested_level = None;
    assert!(fe.results_enter(&mut progress, 0x0700_0001, 6, 2, None), "the fields built");
    let medal2 = fe.results.as_ref().expect("the results fields").medal;
    fe.sync_profile_progress(&progress);
    save_yes_through_card_flow(&mut fe, &progress);
    assert_eq!(fe.card.ops_done, 3, "the create + the two mission saves — nothing else wrote");

    // The pre-restart snapshot, read off the front end the way the pages read it.
    let unlocks1 = fe.packed_unlocks();
    let rows1: Vec<bool> = (0..12).map(|r| fe.unlocked_row(r)).collect();
    let records1: Vec<(u32, u32, u32)> = (0..12).map(|r| fe.record_row(r)).collect();
    let medals1: Vec<Option<(u32, u32)>> = (0..12).map(|r| fe.record_medal(r)).collect();
    let rewards1: Vec<[(bool, u32, u32); 4]> = (0..12).map(|r| fe.reward_rows(r)).collect();
    // Anchors: the boot word (rows 0-1 [D .data]) + row 1's award (sets rows 1+2) + row 4's
    // award (sets rows 4+5) [M 0x1ffa10:63-65]. The driving rows 2/3 keep NO game bits — the
    // pass-through is the port's list view, the word stays the game's.
    assert_eq!(unlocks1, 0b110111, "the game's unlock word, got {unlocks1:#b}");
    assert!(rows1[2] && rows1[3], "the driving pass-through rows stand open in the list");
    assert!(rows1[5] && !rows1[6], "sister: the chain stops behind the on-foot row 5");
    assert!(records1[1].0 > 0 && records1[4].0 > 0, "both best scores recorded");
    assert!(medals1[1].is_some() && medals1[4].is_some(), "both medals recorded");
    assert!(rewards1[1][0].0, "the dossier's first reward row shows for the medalled level");
    // The pre-restart MP gate, computed the way the game computes it on the scenario page open.
    fe.unlock_mp_settings();
    let mask1 = fe.mp_bonus_mask;
    assert!(mask1 >> 0x3e & 1 != 0, "the Explosive-Scenery bit stands pre-restart");

    // ── drop the FrontEnd entirely (as an app restart would) ─────────────────────────────────
    drop(fe);

    // ── a fresh FrontEnd on the same profile dir loads the codename ──────────────────────────
    let mut fe2 = FrontEnd::new();
    fe2.set_profile_dir(dir.clone());
    assert_eq!(fe2.page, Page::Start, "the boot opens the title");
    assert!(act(&mut fe2, Action::Confirm), "the title's Enter");
    assert_eq!(fe2.page, Page::Main, "after the intro movies, the main menu");
    assert!(act(&mut fe2, Action::Confirm), "Main row 0 → the codename page");
    assert_eq!(fe2.page, Page::Codename);
    assert_eq!(fe2.rows(), 3, "the two defaults + the saved codename [M 0x1f7b78]");
    assert!(act(&mut fe2, Action::Down));
    assert!(act(&mut fe2, Action::Down));
    assert!(act(&mut fe2, Action::Confirm), "the codename row loads [M C_SBNFCN 0x20b390]");
    assert_eq!(fe2.page, Page::Difficulty, "the load continues to the difficulty page");
    assert_eq!(fe2.active_profile.as_deref(), Some("RNDTRIP1"));

    // The mission list: the same unlocked rows, driving pass-through included.
    assert_eq!(fe2.packed_unlocks(), unlocks1, "the MSSN status unpacks the same [M 0x2023a0]");
    for r in 0..12 {
        assert_eq!(fe2.unlocked_row(r), rows1[r], "row {r}'s unlock state");
    }
    // The medals and best scores (the Dossier Records' reads [M C_RBDSRECORDS 0x20bf00]).
    for r in 0..12 {
        assert_eq!(fe2.record_row(r), records1[r], "row {r}'s records");
        assert_eq!(fe2.record_medal(r), medals1[r], "row {r}'s medal");
    }
    // The dossier rewards (the cumulative medal gates [M C_RBDSREWARDS 0x20c248:38-74]).
    for r in 0..12 {
        assert_eq!(fe2.reward_rows(r), rewards1[r], "row {r}'s reward rows");
    }
    // Sister: the loaded BNUS word carries the Explosive-Scenery bit.
    assert!(fe2.profile.as_ref().unwrap().progress.bonus[0] >> 0x3e & 1 != 0, "the BNUS bonus bit survived");

    // The MP rewards: walk the real path back to the main menu and into the MP scenario page —
    // its open recomputes the gates [M P_MPSCENARIO_Handler 0x217348:23 → 0x1ff698] — and the
    // Explosive-Scenery gate stands open.
    assert!(act(&mut fe2, Action::Back), "back to the codename page");
    assert_eq!(fe2.page, Page::Codename);
    assert!(act(&mut fe2, Action::Back), "back to the main menu");
    assert_eq!(fe2.page, Page::Main);
    assert!(act(&mut fe2, Action::Down), "the Multiplayer row");
    assert!(act(&mut fe2, Action::Confirm), "Multiplayer");
    assert_eq!(fe2.page, Page::MpMode);
    assert!(act(&mut fe2, Action::Confirm), "Local");
    assert_eq!(fe2.page, Page::MpJoin);
    assert!(act(&mut fe2, Action::Confirm), "join");
    assert!(act(&mut fe2, Action::Confirm), "codename");
    assert!(act(&mut fe2, Action::Confirm), "ready");
    assert_eq!(fe2.page, Page::MpScenario);
    assert!(fe2.mp_explosive_unlocked(), "the Explosive-Scenery gate stands open after the restart");
    assert_eq!(fe2.mp_bonus_mask, mask1, "the recomputed mask matches the pre-restart walk");
    // The kind-2 sister: Double Cross's silver bit (0x39) stands iff the medal was earned
    // [M the switch on the descriptor id 0x38..0x3d].
    assert_eq!(
        fe2.mp_bonus_mask >> 0x39 & 1 != 0,
        medal2 >= 2,
        "mask bit 0x39 follows Double Cross's silver medal"
    );

    // ── sister: a corrupted file reports Load failed and nothing loads ───────────────────────
    let good = std::fs::read(&path).unwrap(); // the roundtrip profile, now corrupted in place
    let mut bad = good.clone();
    let n = bad.len();
    bad[n / 2] ^= 0x08;
    std::fs::write(&path, &bad).unwrap();
    let mut fe3 = FrontEnd::new();
    fe3.set_profile_dir(dir.clone());
    assert!(act(&mut fe3, Action::Confirm), "boot → main");
    assert!(act(&mut fe3, Action::Confirm), "main → the codename page");
    assert_eq!(fe3.page, Page::Codename);
    assert_eq!(fe3.rows(), 3, "the card still lists the row [M MemCard_FILE_CORRUPT's flow]");
    assert!(act(&mut fe3, Action::Down));
    assert!(act(&mut fe3, Action::Down));
    assert!(act(&mut fe3, Action::Confirm), "the load attempt");
    assert_eq!(fe3.card_message(), Some((7, 0x273)), "Load failed! [M LOADERROR 0x10f138]");
    assert_eq!(fe3.card.buttons, 0xc, "Retry+Cancel");
    assert_ne!(fe3.page, Page::Difficulty, "the flow blocked the page change");
    assert!(fe3.active_profile.is_none(), "nothing loaded");
    assert_eq!(fe3.packed_unlocks(), 0x3, "the boot state stands");
    // Fix the file behind the dialog: Retry re-runs the load [M 0x10f138's == 8 chain].
    std::fs::write(&path, &good).unwrap();
    assert!(fe3.card_answer(BTN_RETRY), "Retry re-runs the load");
    assert_eq!(fe3.page, Page::Difficulty, "the load's continuation ran");
    let _ = std::fs::remove_dir_all(&dir);
}

/// "Play without saving": two missions complete the same way (the same results code), but no
/// codename is in play, so nothing writes and a fresh program shows the boot state [M
/// 0x208e78:33-36 — the Default/unsaved path never prompts; the save needs the codename
/// buffer 0x2e1ae8].
#[test]
fn play_without_saving_leaves_no_file() {
    let dir = temp_dir("nosave");
    std::fs::create_dir_all(&dir).unwrap(); // the card exists; the test asserts nothing lands on it
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    assert!(fe.profiles.is_empty(), "the card is empty");

    let mut progress = nf_game::progress::Progress::default();
    for (i, map) in [0x0700_0005u32, 0x0700_0001].into_iter().enumerate() {
        progress.stats[0].dispatched = 2 + i as u32;
        fe.mission_ended(Outcome::Complete, Stats::default());
        fe.requested_level = None;
        fe.sync_profile_progress(&progress);
        assert!(fe.results_enter(&mut progress, map, 6, 2, None), "mission {i} fields built");
        assert!(fe.action(Action::Confirm));
        assert!(!fe.results_box, "no save prompt without a saved codename [M :33-36]");
        through_bonus(&mut fe, &progress);
        assert_eq!(fe.page, Page::MissionSelect, "the routing ran without a save");
    }
    assert!(!fe.save_profile(), "no card write without a codename in play [M the save needs 0x2e1ae8]");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0, "sister: the card holds no file");
    drop(fe);

    let fe2 = FrontEnd::new();
    assert_eq!(fe2.packed_unlocks(), 0x3, "a fresh program boots at the disc state [D .data]");
    let _ = std::fs::remove_dir_all(&dir);
}
