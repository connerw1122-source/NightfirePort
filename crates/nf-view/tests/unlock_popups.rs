//! unlock-popups: the post-results pages — headless flow tests (numbers/ids only; the disc-side
//! pins live in `unlock_popups_live.rs`).
//!
//! Evidence per assertion in the comments [M = the decompile, L = the menu-def/snapshot dumps]:
//! P_NFBONUS_Handler 0x209670, P_WINGAME_Handler 0x215100, P_CHEATMEDAL_Handler 0x2086b0, the
//! page table 0x2029e0, Menu_GetLevelBonuses 0x1ffe98, Menu_HasMedal 0x202580,
//! Menu_SetLevelBonus 0x1ffa10, Mission_Update 0x182c60, P_NFRESULTS_Handler 0x208e78.

use nf_view::menu::{Action, BonusFields, FrontEnd, Outcome, Page, PAGE_MISSION_SELECT, PAGE_NF_BONUS, PAGE_NF_BONUS as BONUS_ID, PAGE_WIN_GAME, PAGE_WIN_GAME as WIN_ID};

fn fe() -> FrontEnd {
    let mut f = FrontEnd::new();
    f.page = Page::Results;
    f.outcome = Outcome::Complete;
    f
}

/// The sister of `results_confirm_routes_to_bonus_when_new_reward`: with the flag DOWN the same
/// Confirm must land on the mission list (the game's third route, 0x4000001c) [M 0x208e78:40-43].
#[test]
fn results_confirm_routes_to_bonus_when_new_reward() {
    let mut f = fe();
    f.new_reward = true;
    assert!(f.action(Action::Confirm));
    assert_eq!(f.page, Page::NfBonus);
    assert_eq!(f.flow.page, PAGE_NF_BONUS);
}

#[test]
fn results_confirm_without_reward_goes_to_the_mission_list() {
    let mut f = fe();
    f.new_reward = false;
    f.just_completed = false;
    assert!(f.action(Action::Confirm));
    assert_eq!(f.page, Page::MissionSelect);
    assert_eq!(f.flow.page, PAGE_MISSION_SELECT);
}

/// `just_completed` (level == 0x0700001b, the SetLevelBonus return [M 0x1ffa10 tail]) routes to
/// the win page when no reward stands [M 0x208e78:46]; the movie state goes up [M 0x215100:12-21].
#[test]
fn results_confirm_just_completed_routes_to_the_win_page() {
    let mut f = fe();
    f.new_reward = false;
    f.just_completed = true;
    assert!(f.action(Action::Confirm));
    assert_eq!(f.page, Page::WinGame);
    assert_eq!(f.flow.page, PAGE_WIN_GAME);
    assert!(f.win_movie, "P_WINGAME 0x4c starts the background movie");
}

/// P_NFBONUS case 0x4b [M 0x209670:46-57]: with just_completed the win page, else the mission list.
#[test]
fn bonus_confirm_routes_on_just_completed() {
    let mut f = fe();
    f.page = Page::NfBonus;
    f.just_completed = false;
    assert!(f.action(Action::Confirm));
    assert_eq!(f.page, Page::MissionSelect);

    let mut f = fe();
    f.page = Page::NfBonus;
    f.just_completed = true;
    assert!(f.action(Action::Confirm));
    assert_eq!(f.page, Page::WinGame);
    assert!(f.win_movie);
}

/// The win page takes no input [M 0x215100 has only the 0x4c/0x50 cases] and the movie-end path
/// opens the credits with the win page as the source [M 0x215100:48 — msg 0x44 → 0x40000030 arg 1].
#[test]
fn win_movie_end_opens_credits_from_the_win_page() {
    let mut f = fe();
    f.page = Page::WinGame;
    assert!(!f.win_movie_finished(), "before win_enter the movie path is dead");

    f.win_enter();
    f.win_movie_vol = 700;
    // Non-skip input reaches nothing [M P_WINGAME 0x215100 has only the 0x4c/0x50 cases; its
    // 0x68/1 blocks the dispatcher, :21].
    assert!(!f.action(Action::Up), "the win page takes no page input");
    assert_eq!(f.page, Page::WinGame);

    assert!(f.win_movie_finished());
    assert_eq!(f.page, Page::Credits);
    assert_eq!(f.credits_from, Some(Page::WinGame));
    assert_eq!(f.credits.page_from, WIN_ID, "the roll's own from-page id");
    assert!(!f.win_movie_finished(), "a second call is inert (the movie flag is down)");
}

/// fe-audit: the win movie is skippable at once — P_WINGAME's 0x4c sets DAT_002e1c49 = 1 and the
/// grace DAT_002e1c4c = 0 [M 0x215100:13-15], so the skip action 0x19 stops it (Menu_StopMovie
/// 0x215a50:6 via MenuManager_Update 0x21c3d8:37-43) and the next 0x50 opens the credits [M :23-48].
#[test]
fn a_skip_ends_the_win_movie_and_opens_the_credits() {
    let mut f = fe();
    f.page = Page::WinGame;
    f.win_enter();
    assert!(f.action(Action::Confirm), "the skip stops the win movie");
    assert_eq!(f.page, Page::Credits, "the next 0x50 opens the credits [M 0x215100:48]");
    assert_eq!(f.credits_from, Some(Page::WinGame));
    // Sister: before the movie ran (no win_enter) the skip does nothing.
    let mut g = fe();
    g.page = Page::WinGame;
    assert!(!g.action(Action::Confirm), "no movie, no skip");
    assert_eq!(g.page, Page::WinGame);
}

/// P_CHEATMEDAL case 0x4b [M 0x2086b0:21-45]: row 0 the fail-channel arm (no grant), rows 1..5
/// grant 999/1/2/3/4 into `medal_from_cheat` 0x30cc74.
#[test]
fn cheat_medal_rows_grant_the_game_medal_values() {
    let mut f = fe();
    f.page = Page::CheatMedal;
    f.cheat_medal_sel = 0;
    f.action(Action::Confirm);
    assert_eq!(f.medal_from_cheat, 0);

    f.cheat_medal_sel = 1;
    f.action(Action::Confirm);
    assert_eq!(f.medal_from_cheat, 999);

    for (row, medal) in [(2, 1), (3, 2), (4, 3), (5, 4)] {
        f.cheat_medal_sel = row;
        f.action(Action::Confirm);
        assert_eq!(f.medal_from_cheat, medal, "row {row} grants medal {medal}");
    }
}

/// The list walks 6 rows with wrap (the game's list control semantics; the handler switches the
/// selection 0..5 [M 0x2086b0:21-40]).
#[test]
fn cheat_medal_selection_wraps() {
    let mut f = fe();
    f.page = Page::CheatMedal;
    f.cheat_medal_sel = 0;
    f.action(Action::Up);
    assert_eq!(f.cheat_medal_sel, 5);
    f.action(Action::Down);
    assert_eq!(f.cheat_medal_sel, 0);
    assert_eq!(f.rows(), 6);
}

/// P_NFBONUS case 0x4c [M 0x209670:13-43] against the live Progress: the tier lines come from
/// `Menu_HasMedal` (the bonus bits) and the text chain is tier 1 → 4 → 3 → 2 [M :22-41].
#[test]
fn bonus_enter_builds_the_get_level_bonuses_record() {
    let mut progress = nf_game::progress::Progress::default();
    let level = 0x0900_0001u32; // RewardsTable[0] [L ee_start_05 0x2bc920]
    // A gold finish: set_level_bonus grants the tiers' bits down from the medal [M 0x1ffa10:71-82].
    progress.set_level_bonus(level, 3, 0);
    assert!(progress.has_medal(level, 1, 0));
    assert!(progress.has_medal(level, 3, 0));
    assert!(!progress.has_medal(level, 4, 0));

    let mut f = fe();
    f.bonus_enter(&progress, level, true);
    let b: &BonusFields = f.bonus.as_ref().expect("the page's fields built");
    assert_eq!(b.level, level);
    assert!(b.lines[0].has && b.lines[1].has && b.lines[2].has);
    assert!(!b.lines[3].has, "tier 4 (platinum) was not granted");
    // The text order: tier 1, then 4, 3, 2 — filtered by the standing lines [M 0x209670:22-41]:
    // tiers 1, 3, 2 stand → labels [REWARD_TEXT[0][0], REWARD_TEXT[0][2], REWARD_TEXT[0][1]].
    let t = &nf_game::progress::REWARD_TEXT[0];
    assert_eq!(b.text_labels, vec![t[0].0, t[2].0, t[1].0]);
    assert_eq!(b.texture, Some(t[0].1), "the tier-1 texture goes to 0x100001cf");
}

/// The sister of the text test: with the game's DAT_002e1c2c gate down the page sends NO text
/// and no texture [M 0x209670:16-17 — `uVar2 = 0`].
#[test]
fn bonus_enter_without_new_reward_sends_nothing() {
    let mut progress = nf_game::progress::Progress::default();
    let level = 0x0900_0001u32;
    progress.set_level_bonus(level, 4, 0);
    let mut f = fe();
    f.bonus_enter(&progress, level, false);
    let b = f.bonus.as_ref().unwrap();
    assert!(b.text_labels.is_empty());
    assert_eq!(b.texture, None);
    assert!(b.lines.iter().all(|l| l.has));
}

/// An unknown level hash yields empty lines (the game's RewardsTable scan misses → the zeroed
/// record [M 0x1ffe98:11-15 `memset(b_265, 0, 0x34)` + the miss arm]) and no crash.
#[test]
fn bonus_enter_unknown_level_is_the_zero_record() {
    let progress = nf_game::progress::Progress::default();
    let mut f = fe();
    f.bonus_enter(&progress, 0x0bad_f00d, true);
    let b = f.bonus.as_ref().unwrap();
    assert!(b.text_labels.is_empty(), "no medal bits stand on a fresh Progress");
    assert!(b.lines.iter().all(|l| (l.label, l.texture) == (0, 0)));
}

/// The page ids the dispatcher binds [M 0x2029e0:463-466 (0x40000038), :511-513 (0x4000004c),
/// :569-572 (0x40000053)] and the round-trip through from_id.
#[test]
fn page_ids_match_the_dispatcher() {
    assert_eq!(Page::NfBonus.id(), 0x4000_0038);
    assert_eq!(Page::WinGame.id(), 0x4000_0053);
    assert_eq!(Page::CheatMedal.id(), 0x4000_004c);
    assert_eq!(BONUS_ID, 0x4000_0038);
}

/// The REWARD_TEXT static mirrors the [L] snapshot dump (ee_start_05.bin 0x2bc920 +0x1c..+0x3c):
/// every entry carries four (label, texture) pairs with 0x03xxxxxx textures, the first tier's
/// label a small bank-0/1 id.
#[test]
fn reward_text_matches_the_snapshot_dump() {
    let t = &nf_game::progress::REWARD_TEXT;
    assert_eq!(t.len(), 12);
    for (i, e) in t.iter().enumerate() {
        assert_eq!(e.len(), 4, "entry {i}");
        for (tier, (label, tex)) in e.iter().enumerate() {
            assert_ne!(*label, 0, "entry {i} tier {tier} label");
            assert_eq!(tex & 0xff00_0000, 0x0300_0000, "entry {i} tier {tier} texture hash");
        }
    }
    // Spot pins from the dump [L]: 0x09000001 (idx 0) tier 1 = (0x2d6, 0x0300012c); the final
    // level 0x0700001b (idx 11) tier 1 = (0x010001ff, 0x0300012b).
    assert_eq!(t[0][0], (0x2d6, 0x0300_012c));
    assert_eq!(t[11][0], (0x0100_01ff, 0x0300_012b));
}
