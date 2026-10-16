//! menu-e2e-singleplayer: every single-player path from boot to a level request, driven ONLY
//! through `action()` / `tick()` and the host-side calls the window makes (`set_profile_dir`,
//! `requested_level.take()` + the flow state the host owns around `mission_ended` [M lib.rs
//! demo_results 2076-2077], `results_enter`, `bonus_enter`, `sync_profile_progress`,
//! `win_movie_finished`, the card flow's dialog answers). No `page`/`cursor`/state-field
//! writes to get anywhere. The playtest 1015-1738 found the menus broken end to end even
//! though every page had passed its own unit tests — those set `fe.page = ...` directly, so
//! the real paths were never exercised.

use nf_view::menu::{self, Action, CAMPAIGN, FrontEnd, Outcome, Page, Stats};

/// One action, then the frames a player would wait: the page-change frames, and any intro /
/// attract movie run out (the headless movies are countdowns). The host's movie_tick consumes
/// each phase-2 resolve per leg [M lib.rs 2326], so the walker does the same.
fn act(fe: &mut FrontEnd, a: Action) -> bool {
    let r = fe.action(a);
    for _ in 0..60 {
        fe.tick();
        let _ = fe.flow.phase2_resolved.take();
    }
    for _ in 0..5000 {
        if !fe.intro_playing && !fe.attract_playing {
            break;
        }
        fe.tick();
        let _ = fe.flow.phase2_resolved.take();
    }
    r
}

/// A scratch card dir per test (the same pattern codename_entry / memcard_save_flow use).
fn card_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nf-sp-e2e-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The (row, column) of a name-grid cell id in [`FrontEnd::CN_GRID`].
fn grid_rc(cell: u32) -> (usize, usize) {
    for (r, row) in FrontEnd::CN_GRID.iter().enumerate() {
        if let Some(c) = row.iter().position(|&x| x == cell) {
            return (r, c);
        }
    }
    panic!("cell {cell} is not in the name grid");
}

/// The d-pad actions that walk the grid cursor from `from` to `to` over real cells only (the
/// grid's rows have different lengths; a clamped move never lands, so the walk never asks for
/// one).
fn grid_walk(from: u32, to: u32) -> Vec<Action> {
    use std::collections::VecDeque;
    let pos = grid_rc(from);
    let goal = grid_rc(to);
    if pos == goal {
        return vec![];
    }
    let rows = FrontEnd::CN_GRID.len();
    let cols = |r: usize| FrontEnd::CN_GRID[r].len();
    let mut prev: Vec<Vec<Option<((usize, usize), Action)>>> =
        (0..rows).map(|r| vec![None; cols(r)]).collect();
    let mut q = VecDeque::new();
    prev[pos.0][pos.1] = Some((pos, Action::Up)); // the visit marker, never walked
    q.push_back(pos);
    while let Some((r, c)) = q.pop_front() {
        if (r, c) == goal {
            break;
        }
        for (a, np) in [
            (Action::Up, r.checked_sub(1).map(|nr| (nr, c))),
            (Action::Down, Some(r + 1).filter(|&nr| nr < rows).map(|nr| (nr, c))),
            (Action::Left, c.checked_sub(1).map(|nc| (r, nc))),
            (Action::Right, Some(c + 1).filter(|&nc| nc < cols(r)).map(|nc| (r, nc))),
        ] {
            let Some((nr, nc)) = np else { continue };
            if nc >= cols(nr) {
                continue; // the shorter rows have no cell there
            }
            if prev[nr][nc].is_some() {
                continue;
            }
            prev[nr][nc] = Some(((r, c), a));
            q.push_back((nr, nc));
        }
    }
    let mut path = Vec::new();
    let mut cur = goal;
    while cur != pos {
        let ((pr, pc), a) = prev[cur.0][cur.1].expect("the grid walk is connected");
        path.push(a);
        cur = (pr, pc);
    }
    path.reverse();
    path
}

/// Types `name` on the name grid the way the window feeds it: d-pad moves and presses.
fn type_on_grid(fe: &mut FrontEnd, name: &str) {
    for ch in name.chars() {
        let cell = ch.to_ascii_uppercase() as u32;
        for a in grid_walk(fe.cn_cursor_cell(), cell) {
            assert!(fe.action(a), "the grid walk to {ch} lands");
        }
        assert_eq!(fe.cn_cursor_cell(), cell, "the cursor stands on {ch}");
        assert!(fe.action(Action::Confirm), "the {ch} press types");
    }
    for a in grid_walk(fe.cn_cursor_cell(), FrontEnd::CN_KEY_CONFIRM) {
        assert!(fe.action(a), "the walk back to the ✓ lands");
    }
    assert_eq!(fe.cn_cursor_cell(), FrontEnd::CN_KEY_CONFIRM);
}

/// Runs a mission end's load out on ticks (the fade and the loader are headless-fast) until
/// the front-end map is back under the runner state. Returns the phase-2 resolve when the end
/// wrapped through an outro movie.
fn drive_end_to_map(fe: &mut FrontEnd) -> Option<u32> {
    let mut resolved = None;
    for _ in 0..400 {
        fe.tick();
        if let Some(r) = fe.flow.phase2_resolved.take() {
            resolved = Some(r);
        }
        if fe.flow.get_state() == menu::ST_RUNNER && fe.flow.current_level == menu::FRONTEND_LEVEL {
            return resolved;
        }
    }
    panic!("the front-end load never finished");
}

/// Boot → title → (intro plays out) → Main.
fn to_main(fe: &mut FrontEnd) {
    assert_eq!(fe.page, Page::Start, "the boot opens the title");
    assert!(fe.action(Action::Confirm), "the title's Enter starts the intro");
    assert!(fe.intro_playing, "the intro movies play after the title");
    assert_eq!(fe.page, Page::Main, "the page flips at once behind the movie");
    assert!(!fe.action(Action::Down), "the intro swallows other input");
    for _ in 0..5000 {
        if !fe.intro_playing && !fe.attract_playing {
            break;
        }
        fe.tick();
    }
    assert!(!fe.intro_playing, "the intro ran out on its own ticks");
    assert_eq!(fe.page, Page::Main);
}

/// Main → NightFire (Select Codename).
fn to_codename(fe: &mut FrontEnd) {
    to_main(fe);
    assert!(act(fe, Action::Confirm), "NightFire");
    assert_eq!(fe.page, Page::Codename);
}

/// Codename page → Play without saving (two Confirms: the confirm box) → the default
/// difficulty (the middle row) → the mission list.
fn to_missions(fe: &mut FrontEnd) {
    to_codename(fe);
    assert!(act(fe, Action::Down));
    assert_eq!(fe.cursor, 1, "Play without saving");
    assert!(act(fe, Action::Confirm), "the confirm box opens");
    assert!(act(fe, Action::Confirm), "the box's yes");
    assert_eq!(fe.page, Page::Difficulty);
    assert_eq!(fe.cursor, 1, "the wheel starts centred on Agent [M C_SBNFDFCTY]");
    assert!(act(fe, Action::Confirm), "the difficulty");
    assert_eq!(fe.page, Page::MissionSelect);
}

/// Fresh profile → the driving notice on row 0 → dismiss → The Exchange requested.
/// Returns the requested level (already taken, the host consumed it).
fn fresh_profile_to_the_exchange(fe: &mut FrontEnd) -> u32 {
    to_missions(fe);
    assert_eq!(fe.cursor, 0, "a fresh profile opens on Paris Prelude");
    assert!(act(fe, Action::Confirm), "Paris Prelude");
    assert!(fe.notice.is_some(), "the driving notice shows");
    assert_eq!(fe.requested_level, None, "a driving row requests nothing");
    assert!(act(fe, Action::Confirm), "Enter dismisses the notice");
    assert!(fe.notice.is_none());
    assert_eq!(fe.cursor, 0);
    assert!(act(fe, Action::Down), "Down to The Exchange");
    assert_eq!(fe.cursor, 1);
    assert!(fe.unlocked_row(1), "the row after a driving row opens through it");
    assert!(act(fe, Action::Confirm), "The Exchange starts");
    let level = fe.requested_level.expect("the level request");
    assert_eq!(level, CAMPAIGN[1].3);
    assert_eq!(level, 0x0700_0005);
    // A second Enter while the request stands is refused (the game deletes the menu manager
    // with it) — the playtest's "a second Enter during a load started a second load".
    assert!(!fe.action(Action::Confirm), "no second request");
    assert_eq!(fe.requested_level.take(), Some(level)); // [host] the level loads
    level
}

// ── boot / intro ─────────────────────────────────────────────────────────────────────────────

#[test]
fn boot_reaches_main_through_the_intro_both_ways() {
    // Skipped: the second Enter pops the movie state and Main is already up.
    let mut fe = FrontEnd::new();
    assert_eq!(fe.page, Page::Start);
    assert!(fe.action(Action::Confirm), "the title's Enter");
    assert!(fe.intro_playing);
    assert!(fe.action(Action::Confirm), "the skip press");
    assert!(!fe.intro_playing, "Enter skips the intro");
    for _ in 0..60 {
        fe.tick();
    }
    assert_eq!(fe.page, Page::Main);
    assert!(act(&mut fe, Action::Confirm), "NightFire still opens after a skip");
    assert_eq!(fe.page, Page::Codename);

    // Played out: the movies end on their own ticks and Main stands.
    let mut fe = FrontEnd::new();
    to_main(&mut fe);
    assert!(act(&mut fe, Action::Confirm), "NightFire");
    assert_eq!(fe.page, Page::Codename);
}

// ── Back at every step ───────────────────────────────────────────────────────────────────────

#[test]
fn back_walks_every_single_player_step_to_the_title() {
    let mut fe = FrontEnd::new();
    to_missions(&mut fe);
    // The Dossier opens off the mission list with □ and returns with Back.
    assert!(act(&mut fe, Action::Side), "□ opens the Dossier");
    assert_eq!(fe.page, Page::Dossier);
    assert_eq!(fe.cursor, 0, "the Dossier wheel opens at Records [M 0x216de0:9-16]");
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::MissionSelect);
    // Down the chain: list → difficulty → codename → main → title.
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Difficulty);
    assert_eq!(fe.cursor, 1, "the wheel re-opens on the chosen difficulty");
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Codename);
    assert_eq!(fe.cursor, 0);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Main);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Start, "Back walks all the way to the title");
    // …and forward again still works.
    assert!(act(&mut fe, Action::Confirm), "the title");
    assert_eq!(fe.page, Page::Main);
    assert!(act(&mut fe, Action::Confirm), "NightFire");
    assert_eq!(fe.page, Page::Codename);
}

// ── the Select Codename branches ─────────────────────────────────────────────────────────────

#[test]
fn select_codename_play_without_saving_asks_and_back_cancels() {
    let mut fe = FrontEnd::new();
    to_codename(&mut fe);
    assert_eq!(fe.rows(), 2, "no card rows without a card");
    assert!(act(&mut fe, Action::Down));
    assert_eq!(fe.cursor, 1, "Play without saving");
    assert!(act(&mut fe, Action::Confirm), "the confirm box opens");
    assert!(act(&mut fe, Action::Back), "△ cancels the box");
    assert_eq!(fe.page, Page::Codename, "the cancel stays on the page");
    assert!(act(&mut fe, Action::Confirm), "the box opens again");
    assert!(act(&mut fe, Action::Confirm), "the box's yes");
    assert_eq!(fe.page, Page::Difficulty);
    assert_eq!(fe.cursor, 1, "Agent, the middle row");
}

#[test]
fn select_codename_new_codename_validates_creates_saves_and_reloads() {
    let dir = card_dir("new-codename");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone()); // [host] the card wiring before the menus run
    to_codename(&mut fe);
    // Row 0 "New Codename" → the name grid: the game's proposal, the ✓ focused.
    assert!(act(&mut fe, Action::Confirm), "New Codename");
    assert_eq!(fe.page, Page::CnName);
    assert_eq!(fe.cn_typed_name(), "BOND", "the game's proposal [L ee_snapshot 0x30d530]");
    assert_eq!(fe.cn_cursor_cell(), FrontEnd::CN_KEY_CONFIRM, "the ✓ holds the focus");
    // The empty-name refusal through the real path: Delete clears the proposal, ✓ refuses.
    for a in grid_walk(FrontEnd::CN_KEY_CONFIRM, FrontEnd::CN_KEY_DELETE) {
        assert!(fe.action(a), "the walk to ← lands");
    }
    assert!(fe.action(Action::Confirm), "← clears the proposal");
    assert_eq!(fe.cn_typed_name(), "");
    for a in grid_walk(FrontEnd::CN_KEY_DELETE, FrontEnd::CN_KEY_CONFIRM) {
        assert!(fe.action(a), "the walk back to the ✓ lands");
    }
    assert!(!fe.action(Action::Confirm), "the empty name is refused");
    assert_eq!(fe.cn_box, Some((6, 0x3bc)), "the validator's box [M 0x213a98:47-51]");
    assert!(fe.action(Action::Confirm), "the box's ok closes it");
    assert_eq!(fe.cn_box, None);
    // Type MONEYP on the grid, through the d-pad path, and submit on ✓.
    type_on_grid(&mut fe, "MONEYP");
    assert_eq!(fe.cn_typed_name(), "MONEYP");
    assert!(act(&mut fe, Action::Confirm), "the ✓ submits");
    // The create saved through the card flow and landed on the wheel.
    assert_eq!(fe.page, Page::CnMenu, "the completion routes to the wheel [M 0x213a98:58]");
    assert_eq!(fe.cursor, 1, "the wheel opens on Controller Setup [L PCSX2 m30]");
    assert_eq!(fe.active_profile.as_deref(), Some("MONEYP"));
    assert!(nf_game::profile::Profile::path_in(&dir, "MONEYP").exists(), "the file is on the card");
    // Back to the main menu and re-open: the saved codename is listed and loads.
    assert!(act(&mut fe, Action::Back), "the wheel goes back");
    assert_eq!(fe.page, Page::CnSelect);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Main);
    assert_eq!(fe.cursor, 2, "the Codenames row");
    assert!(act(&mut fe, Action::Down), "Down wraps to NightFire");
    assert_eq!(fe.cursor, 0);
    assert!(act(&mut fe, Action::Confirm), "NightFire");
    assert_eq!(fe.page, Page::Codename);
    assert_eq!(fe.rows(), 3, "the card's codename is listed [M Menu_EnumSaves]");
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Down));
    assert_eq!(fe.cursor, 2, "MONEYP's row");
    assert!(act(&mut fe, Action::Confirm), "the saved codename loads");
    assert_eq!(fe.page, Page::Difficulty);
    assert_eq!(fe.active_profile.as_deref(), Some("MONEYP"));
    assert!(fe.unlocked_row(0) && fe.unlocked_row(1), "the fresh card keeps the boot unlocks");
    assert!(!fe.unlocked_row(2));
    assert!(act(&mut fe, Action::Confirm), "the difficulty");
    assert_eq!(fe.page, Page::MissionSelect);
    let _ = std::fs::remove_dir_all(&dir);
}

// ── every difficulty → the mission list ──────────────────────────────────────────────────────

#[test]
fn each_difficulty_reaches_the_mission_list() {
    let mut fe = FrontEnd::new();
    to_codename(&mut fe);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Confirm), "the confirm box");
    assert!(act(&mut fe, Action::Confirm), "the box's yes");
    assert_eq!(fe.page, Page::Difficulty);
    // The wheel CLAMPS, it never wraps [M 0x2221b20:186-210; C_SBNFDFCTY 0x216ca8:25-28 range 0..2]:
    // Down runs to the bottom and is refused there.
    assert!(act(&mut fe, Action::Down));
    assert_eq!(fe.cursor, 2);
    assert!(!act(&mut fe, Action::Down), "the wheel clamps at the bottom");
    assert_eq!(fe.cursor, 2);
    // Every row confirms into the mission list; Back re-opens the wheel on the chosen row.
    for d in [2usize, 1, 0] {
        while fe.cursor < d {
            assert!(act(&mut fe, Action::Down));
        }
        while fe.cursor > d {
            assert!(act(&mut fe, Action::Up));
        }
        assert_eq!(fe.cursor, d);
        assert!(act(&mut fe, Action::Confirm), "the difficulty row confirms");
        assert_eq!(fe.page, Page::MissionSelect);
        assert_eq!(fe.difficulty, d, "the choice rode the confirm");
        assert_eq!(fe.cursor, 0, "the list opens on the first unlocked row");
        assert!(act(&mut fe, Action::Back));
        assert_eq!(fe.page, Page::Difficulty);
        assert_eq!(fe.cursor, d, "Back re-opens the wheel on the chosen row");
    }
}

// ── every unlocked row → the exact level request ─────────────────────────────────────────────

#[test]
fn every_unlocked_row_requests_its_campaign_level() {
    // The two ports of the game's sp_level table agree [M CAMPAIGN 0x2df2e0; SP_LEVEL_HASHES].
    for (i, &(_, _, _, level)) in CAMPAIGN.iter().enumerate() {
        assert_eq!(level, nf_game::progress::SP_LEVEL_HASHES[i], "CAMPAIGN row {i} == sp_level[{i}]");
    }

    // A fresh profile: rows 0 and 1 are unlocked [D sp_level's boot state].
    let mut fe = FrontEnd::new();
    fresh_profile_to_the_exchange(&mut fe);

    // [host --unlock-all] every row answers with its own sp_level hash; the driving rows show
    // the notice instead and the next on-foot row opens through them.
    let mut fe = FrontEnd::new();
    fe.unlock_all = true;
    to_missions(&mut fe);
    assert_eq!(fe.cursor, 0);
    for i in 0..CAMPAIGN.len() {
        if i > 0 {
            assert!(act(&mut fe, Action::Down), "Down to row {i}");
            assert_eq!(fe.cursor, i, "the wheel walks one row");
        }
        let level = CAMPAIGN[i].3;
        let driving = nf_game::ptp::is_driving_level(level);
        assert_eq!(driving, menu::is_driving_level(level), "row {i}: the driving set matches");
        assert!(act(&mut fe, Action::Confirm), "row {i} confirms");
        if driving {
            assert!(fe.notice.is_some(), "row {i}: the driving notice shows");
            assert_eq!(fe.requested_level, None, "row {i}: no level request");
            assert_eq!(fe.flow.get_state(), menu::ST_RUNNER, "row {i}: the flow stays");
            // Confirm dismisses; a second driving row's notice is dismissed with Back too.
            assert!(act(&mut fe, Action::Confirm), "row {i}: Enter dismisses");
            assert!(fe.notice.is_none());
        } else {
            assert!(fe.notice.is_none(), "row {i}: an on-foot row starts at once");
            assert_eq!(
                fe.requested_level.take(),
                Some(level),
                "row {i} requests its sp_level hash"
            );
        }
    }
}

// ── a completed mission: results → save → the next row ───────────────────────────────────────

#[test]
fn a_completed_mission_routes_the_results_save_and_the_next_row() {
    let dir = card_dir("complete");
    let mut fe = FrontEnd::new();
    fe.set_profile_dir(dir.clone());
    // A saved codename first, through the real branches (the save prompt needs one).
    to_codename(&mut fe);
    assert!(act(&mut fe, Action::Confirm), "New Codename");
    assert_eq!(fe.page, Page::CnName);
    type_on_grid(&mut fe, "BOND007");
    assert!(act(&mut fe, Action::Confirm), "the ✓ submits");
    assert_eq!(fe.page, Page::CnMenu);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::CnSelect);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Main);
    assert!(act(&mut fe, Action::Down), "Down wraps to NightFire");
    assert!(act(&mut fe, Action::Confirm), "NightFire");
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Down));
    assert_eq!(fe.cursor, 2, "BOND007's row");
    assert!(act(&mut fe, Action::Confirm), "the saved codename loads");
    assert_eq!(fe.page, Page::Difficulty);
    assert!(act(&mut fe, Action::Confirm), "Agent");
    assert_eq!(fe.page, Page::MissionSelect);

    // Paris Prelude is the driving row: the notice, dismiss, The Exchange. The second Enter
    // is refused while the request stands — the playtest's "a second Enter started a second load".
    let level = {
        assert!(act(&mut fe, Action::Confirm), "Paris Prelude");
        assert!(fe.notice.is_some());
        assert!(act(&mut fe, Action::Confirm), "dismiss");
        assert!(act(&mut fe, Action::Down));
        assert!(act(&mut fe, Action::Confirm), "The Exchange starts");
        let l = fe.requested_level.expect("the request");
        assert_eq!(l, 0x0700_0005);
        assert!(!fe.action(Action::Confirm), "no second request");
        assert_eq!(fe.requested_level.take(), Some(l)); // [host] the level loads
        l
    };

    // [host] the level plays (the runner state under Mission_Update 0x182c60) and ends complete.
    let mut progress = nf_game::progress::Progress::default();
    progress.nightfire_status = fe.packed_unlocks(); // swap_in_scene's seed [M lib.rs 2036]
    progress.stats[0].dispatched = 30;
    progress.stats[0].shots_fired = 100;
    progress.stats[0].shots_hit_enemy = 100;
    fe.flow.stack = vec![menu::ST_RUNNER];
    fe.flow.current_level = level;
    fe.mission_ended(
        Outcome::Complete,
        Stats { shots_fired: 30, shots_hit: 20, accuracy_pct: 66, time_s: 100 },
    );
    // The Exchange has no outro code: the front-end map loads plain, the results page stands.
    assert_eq!(fe.requested_level.take(), Some(menu::FRONTEND_LEVEL), "no outro for 07000005");
    assert_eq!(drive_end_to_map(&mut fe), None, "a plain map load has no phase-2 resolve");
    assert_eq!(fe.page, Page::Results);
    assert_eq!(fe.flow.page, menu::PAGE_RESULTS);

    // The results fields + the award [M P_NFRESULTS 0x208e78 case 0x4c].
    assert!(fe.results_enter(&mut progress, level, 6, fe.difficulty as u32, None), "the fields build");
    assert!(fe.results_improved, "the commit improved (first completion)");
    assert!(fe.results.is_some());
    assert!(fe.new_reward, "a first medal stands [M 0x1ffa10:74-79]");
    assert!(fe.unlocked_row(1), "the completed row stamps");
    assert!(fe.unlocked_row(2), "the next row unlocks");
    assert!(fe.unlocked_row(4), "Double Cross opens through the chained driving rows");

    // Confirm #1 opens the save prompt (improved + a saved codename in play); #2 answers Yes.
    assert!(act(&mut fe, Action::Confirm), "the results' first Confirm");
    assert!(fe.results_box, "the save prompt stands [M 0x1000206]");
    assert!(act(&mut fe, Action::Confirm), "Yes requests the save");
    assert_eq!(fe.card_message(), Some((4, 0x27d)), "ASK_OVERWRITE [M 0x10ee38]");
    assert!(fe.card_answer(menu::BTN_YES), "the overwrite question's Yes");
    fe.tick();
    assert_eq!(fe.card_message(), None, "the flow closed on completion");
    // The routing hops to the 007 Bonus page while the new reward stands.
    assert_eq!(fe.page, Page::NfBonus);
    assert_eq!(fe.flow.page, menu::PAGE_NF_BONUS);
    fe.bonus_enter(&progress, fe.results_level, fe.new_reward); // [host]
    assert!(fe.bonus.is_some(), "the bonus fields built on the page enter");
    assert!(act(&mut fe, Action::Confirm), "the bonus page's Confirm");
    // → the mission list, opened on the HIGHEST unlocked row [M P_NFMAP 0x20b608 case 0x4c].
    assert_eq!(fe.page, Page::MissionSelect);
    assert_eq!(fe.flow.page, menu::PAGE_MISSION_SELECT);
    let want_cursor = (0..CAMPAIGN.len()).rev().find(|&i| fe.unlocked_row(i)).unwrap();
    assert_eq!(fe.cursor, want_cursor, "the list opens on the highest unlocked row");
    assert_eq!(want_cursor, 4, "Double Cross, through the two driving rows");
    // The next on-foot row is reachable and starts.
    assert!(act(&mut fe, Action::Confirm), "Double Cross starts");
    assert_eq!(fe.requested_level.take(), Some(0x0700_0001));

    // The written file carries the run's progress.
    let path = nf_game::profile::Profile::path_in(&dir, "BOND007");
    let back = nf_game::profile::Profile::load_file(&path).expect("the card's file loads");
    let status = back.progress.nightfire_status();
    assert_eq!(status & 0b111, 0b111, "rows 0-2 ride the save");

    // Reloading the codename brings the unlocks back.
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Difficulty);
    assert!(act(&mut fe, Action::Back));
    assert_eq!(fe.page, Page::Codename);
    assert!(act(&mut fe, Action::Down));
    assert!(act(&mut fe, Action::Down));
    assert_eq!(fe.cursor, 2);
    assert!(act(&mut fe, Action::Confirm), "the saved codename loads");
    assert_eq!(fe.page, Page::Difficulty);
    assert!(fe.unlocked_row(1) && fe.unlocked_row(2), "the card's unlocks came back");
    let _ = std::fs::remove_dir_all(&dir);
}

// ── a failed mission: the fail menu's restart and quit ───────────────────────────────────────

#[test]
fn a_failed_mission_restarts_and_quits_from_the_results_page() {
    let mut fe = FrontEnd::new();
    let level = fresh_profile_to_the_exchange(&mut fe);

    // [host] the level runs and FAILS.
    fe.flow.stack = vec![menu::ST_RUNNER];
    fe.flow.current_level = level;
    fe.mission_ended(Outcome::Failed, Stats::default());
    assert_eq!(fe.requested_level.take(), Some(menu::FRONTEND_LEVEL), "failures skip the outro");
    assert_eq!(fe.page, Page::Results);
    assert_eq!(fe.rows(), 2, "the fail menu's REPLAY / QUIT [L page 0x40000042]");
    assert_eq!(fe.cursor, 0);
    // The menu answers BEFORE the armed front-end load lands: on the disc the mission-failed
    // screen shows while 0x7000048 streams in, and REPLAY reads GameState+0x12 (still the
    // mission) at answer time [M P_ENDMISSION 0x216618:19-21]. The port's headless load lands
    // on the next ticks, so this leg feeds raw actions — the same one-frame window a player's
    // press rides.
    assert!(fe.action(Action::Back), "△ lands on the second row (the game's own arm)");
    assert_eq!(fe.cursor, 1);
    assert!(fe.action(Action::Up));
    assert_eq!(fe.cursor, 0);
    assert!(fe.action(Action::Confirm), "REPLAY");
    assert_eq!(fe.requested_level, Some(0x0700_0005), "the current level rides the restart");
    assert!(fe.flow.warm_reset, "the warm arm");
    assert_eq!(fe.requested_level.take(), Some(0x0700_0005)); // [host] the restart loads
    for _ in 0..120 {
        fe.tick();
    } // the loads land

    // QUIT: the front end, cold (the same answer-before-the-load window).
    let mut fe2 = FrontEnd::new();
    let level2 = fresh_profile_to_the_exchange(&mut fe2);
    fe2.flow.stack = vec![menu::ST_RUNNER];
    fe2.flow.current_level = level2;
    fe2.mission_ended(Outcome::Failed, Stats::default());
    fe2.requested_level.take();
    assert!(fe2.action(Action::Down));
    assert_eq!(fe2.cursor, 1, "the QUIT row");
    assert!(fe2.action(Action::Confirm), "QUIT");
    assert_eq!(fe2.requested_level, Some(menu::FRONTEND_LEVEL), "QUIT loads the front end");
    assert_eq!(fe2.flow.page, menu::PAGE_RESULTS_QUIT_TARGET, "+0x28 = the mission list");
    for _ in 0..120 {
        fe2.tick();
    }
}

// ── the final mission: results → bonus → win → credits ───────────────────────────────────────

#[test]
fn the_final_mission_routes_to_the_win_page_and_credits() {
    let mut fe = FrontEnd::new();
    fe.unlock_all = true; // [host --unlock-all] the only way to Equinox without the campaign
    to_missions(&mut fe);
    for _ in 0..11 {
        assert!(act(&mut fe, Action::Down), "Down the wheel");
    }
    assert_eq!(fe.cursor, 11, "Equinox");
    assert!(act(&mut fe, Action::Confirm), "Equinox starts");
    let level = fe.requested_level.take().expect("the request");
    assert_eq!(level, 0x0700_001b);

    // [host] the level plays and completes — the game's final mission.
    let mut progress = nf_game::progress::Progress::default();
    progress.nightfire_status = fe.packed_unlocks();
    progress.stats[0].dispatched = 15;
    progress.stats[0].shots_fired = 200;
    progress.stats[0].shots_hit_enemy = 200;
    fe.flow.stack = vec![menu::ST_RUNNER];
    fe.flow.current_level = level;
    fe.mission_ended(
        Outcome::Complete,
        Stats { shots_fired: 30, shots_hit: 20, accuracy_pct: 66, time_s: 100 },
    );
    assert_eq!(fe.requested_level.take(), Some(0x0730_001b), "the final level wraps to its outro");
    assert_eq!(drive_end_to_map(&mut fe), Some(menu::FRONTEND_LEVEL), "the outro resolves to the map");
    assert_eq!(fe.page, Page::Results);
    assert_eq!(fe.flow.page, menu::PAGE_RESULTS);
    assert!(fe.results_enter(&mut progress, level, 6, fe.difficulty as u32, None));
    assert!(fe.just_completed, "the final mission sets just_completed [M 0x1ffa10 tail]");
    // The first Confirm routes: a new reward stands → the 007 Bonus page first.
    assert!(act(&mut fe, Action::Confirm), "the results' first Confirm");
    if fe.new_reward {
        assert_eq!(fe.page, Page::NfBonus, "the new-reward hop [M 0x208e78:48-53]");
        fe.bonus_enter(&progress, fe.results_level, fe.new_reward); // [host]
        assert!(fe.bonus.is_some());
        assert!(act(&mut fe, Action::Confirm), "the bonus page's Confirm");
    }
    // just_completed → the win page [M 0x209670:46-57].
    assert_eq!(fe.page, Page::WinGame);
    assert_eq!(fe.flow.page, menu::PAGE_WIN_GAME);
    assert!(!act(&mut fe, Action::Confirm), "the win page has no controls [M P_WINGAME]");
    assert!(fe.win_movie_finished(), "[host] the background movie's end");
    assert_eq!(fe.page, Page::Credits, "the credits open with the win source [M 0x215100:48]");
}
