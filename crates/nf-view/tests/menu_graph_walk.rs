//! menu-graph-walk: a headless explorer over the front end's reachable menu states.
//!
//! The walk starts from [`FrontEnd::new()`] and reaches every state by ACTIONS and TICKS only —
//! `action()`, `tick()` and the host-side calls the window makes (`open_pause`, `mission_ended`,
//! `results_enter`/`stats_enter`/`bonus_enter`, `win_movie_finished`, and the documented host
//! `take()` of `requested_level` when a requested level finishes loading). `page`/`cursor`/state
//! fields are never assigned to get somewhere. Movies and the attract need ticks: after each
//! action the driver ticks until `!intro_playing && !attract_playing` (and the movie/load/fade
//! states popped), bounded (the intro budget is 850 ticks [M `INTRO_TICKS`]).
//!
//! Breadth-first walk: a state is (page, cursor, the page's own sub-state), the edges are every
//! [`Action`] plus the tick bursts, capped by a visited set plus a depth bound. Invariants at
//! every step: no panic (a panicking edge is caught and reported, the walk continues), the cursor
//! is inside `rows()` on every rowed page, `page.id()` is a known page id, Back leaves every
//! non-root page within N steps, every reached page can get back to Main by actions only,
//! `requested_level` is never set twice for one confirm, and no page is a dead end.
//!
//! `FrontEnd` is not `Clone` (it holds a `Box<dyn MpSession>`), so the walk stores PATHS and
//! re-drives each node from a fresh `FrontEnd::new()` — deterministic: the headless front end is
//! a pure state machine (fixed 60 Hz, no wall clock, the menus draw no RNG).
//!
//! Roots (all from `FrontEnd::new()`):
//!   1. the raw boot — the title, the intro, the attract;
//!   2. the boot driven to Main by actions — the whole front-end graph (SP + MP + codename +
//!      options pages);
//!   3. the pause root — a mission loaded by actions, then the host's `open_pause()`;
//!   4. the results root — over a loaded mission, so the BFS takes `mission_ended(Complete|Failed)`
//!      + the host's `results_enter`;
//!   5. the win root — the final mission (the host's `--unlock-all` dev flag [M 0x20b8d8:18])
//!      completed, so `just_completed`/`new_reward` route to the bonus/win/cheat-medal pages.
//!
//! A randomized version runs 20 seeded passes of 5000 random actions/ticks over the same
//! invariants.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as _;

use nf_view::menu::{
    Action, FrontEnd, Outcome, Page, Stats, ATTRACT_FRAMES, CAMPAIGN, FRONTEND_LEVEL,
    PAGE_FAIL_MENU, ST_FADE_HOLD, ST_MAPLOAD, ST_MOVIE, ST_RUNNER,
};

// ── the walk's knobs ──────────────────────────────────────────────────────────────────────────

/// The settle bound: the intro is 425 pictures × 2 ticks = 850 [M `INTRO_TICKS`], plus margin
/// for the fade (60 frames [M Mission_Update 0x182c60:74]) and the outro movie. Below the
/// 0xa8c = 2700 attract timeout [M `ATTRACT_FRAMES`], so settles never trip it by accident.
const SETTLE_BOUND: u32 = 1100;
/// The attract edge's tick count: the timeout fires at 0xa8c = 2700 no-input frames [M
/// P_START 0x20a408 / P_MAIN 0x20a7b0 — `Menu_GetNoInputCount() == 0xa8c`], plus margin.
const ATTRACT_TICKS: u32 = ATTRACT_FRAMES + 20;
/// A raw tick burst (one "the ticks" edge) — long enough to cross the card pumps and the net
/// lobby's 10 s countdown (600 ticks [S]) in a handful of edges.
const BURST_TICKS: u32 = 64;
/// The BFS depth bound (edges from the root).
const DEPTH_BOUND: usize = 22;
/// Per-(page,cursor) node cap — slider/wheel value variants explode otherwise; the randomized
/// runs cover the combinations this skips.
const PER_PAGE_CURSOR_CAP: usize = 24;
/// Per-page node cap (all cursors summed).
const PER_PAGE_CAP: usize = 400;
/// Total node cap per walk.
const TOTAL_NODE_CAP: usize = 12000;
/// Back-leaves-the-page probe: N presses (a modal box eats one).
const BACK_STEPS: usize = 8;
/// The reach-Main mini-BFS depth. The credits roll is the game's 0x242 = 578 lines × 14 frames
/// + 18 lines' fade + the 0x13 exit delay [M Menu_SetupCredits 0x1f9a88 stores 0x242;
/// 0x2152b0:169-184] ≈ 8360 frames ≈ 131 bursts — the win page's only route home is the host's
/// movie-end call plus that roll, so the depth covers it. The credits chain is linear (the
/// actions are no-ops there [M P_CREDITS_Handler 0x2152b0 has no input cases]) and the visited
/// set plus the node cap keep every other page's search bounded.
const HOME_DEPTH: usize = 150;
/// The home probe's node cap (the option-value lattices could sprawl otherwise).
const HOME_NODE_CAP: usize = 3000;
/// Wall-clock guard per walk (harness time only — the Sim never sees it).
const WALK_DEADLINE: std::time::Duration = std::time::Duration::from_secs(240);

const ACTIONS: [Action; 7] = [
    Action::Up,
    Action::Down,
    Action::Confirm,
    Action::Back,
    Action::Side,
    Action::Left,
    Action::Right,
];

// ── edges ─────────────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Act(Action),
    /// `BURST_TICKS` raw ticks (no input).
    Burst,
    /// boot-flow: the START intro's skip grace, `ESTHERO_GRACE` ticks (no input) [M P_ESTHERO 0x20a060:18].
    Grace,
    /// The no-input timeout to the attract movie (only the title and the main menu check the
    /// count [M the only `Menu_GetNoInputCount` callers]).
    Attract,
    /// The host's Esc — `open_pause()` [M Player_Update 0x193600:73 MenuManager_Create].
    OpenPause,
    /// The host reporting a mission end — `mission_ended` [M Mission_Update 0x182c60:74].
    MissionEnded(Outcome),
    /// The host builds the up-page's fields the way the game's 0x4c does: `results_enter`
    /// [M P_NFRESULTS 0x208e78 case 0x4c], `stats_enter` [M P_NFSTATS 0x209938], `bonus_enter`
    /// [M P_NFBONUS 0x209670 case 0x4c].
    HostEnter,
    /// The host reporting the win page's background movie end — `win_movie_finished`
    /// [M P_WINGAME 0x215100:23-49].
    WinMovieEnd,
    /// The host pumps the pending load to completion and takes `requested_level` (the documented
    /// host protocol; prologue-only — never offered as a graph edge).
    LoadTake,
    /// The host's `--unlock-all` dev flag [M menu_unlock_everything 0x30d2a7, 0x20b8d8:18]
    /// (prologue-only host configuration, never a graph edge).
    UnlockAll,
    /// The boot ticks (prologue-only): tick until the front-end map is up [M 0x1c98c8 case 1 +
    /// 0x1d6360].
    Boot,
}

/// Whether the quiescent condition holds: no intro/attract movie, no movie/load/fade state.
fn quiescent(fe: &FrontEnd) -> bool {
    !fe.intro_playing
        && !fe.attract_playing
        && !matches!(fe.flow.get_state(), ST_MOVIE | ST_MAPLOAD | ST_FADE_HOLD)
}

/// One invariant sweep over a step (called per tick inside settles too). Returns the problems.
fn check_step(fe: &FrontEnd) -> Vec<String> {
    let mut bad = Vec::new();
    // `cursor < rows()` on every rowed page.
    let rows = fe.rows();
    if rows > 0 && fe.cursor >= rows {
        bad.push(format!(
            "cursor {} out of range on page {:?} (rows {})",
            fe.cursor, fe.page, rows
        ));
    }
    // `page.id()` is a known page id — one of the ported pages', or the failed-results page's
    // own id (page_id() special-cases it [M P_ENDMISSION 0x40000042]).
    let known: Vec<u32> = ALL_PAGES.iter().map(|p| p.id()).chain([PAGE_FAIL_MENU]).collect();
    if !known.contains(&fe.page_id()) {
        bad.push(format!(
            "page {:?} has an unknown id {:#x}",
            fe.page,
            fe.page_id()
        ));
    }
    // The flow's own page variable stays a known id while the menu is up.
    if fe.flow.get_state() == ST_RUNNER && !known.contains(&fe.flow.page) {
        bad.push(format!("flow.page {:#x} is not a known page id", fe.flow.page));
    }
    bad
}

/// The state key: the page, the cursor and the page's own sub-state — everything that gates
/// `action()`/`tick()` behaviour. Audio-only fields (`sfx_out`, the music state) and the
/// per-frame flash/no-input counters are excluded (the attract is explored by its own edge).
fn state_key(fe: &FrontEnd) -> String {
    let mut opts: Vec<_> = fe.opts.iter().collect();
    opts.sort_unstable();
    let mut s = String::with_capacity(1024);
    let _ = write!(
        s,
        "p={:?} c={} fp={:x} st={} lvl={:x} tol={:x} wr={} req={:?} mi={} di={} oc={:?} sts={:?}\
         \nunl={:?} prf={:?} ap={:?} hp={} cl={}\
         \ncq={} cr={} cs={} pt={} ph={:?} po={} pob={:?}\
         \ncdp={} in={} ii={} at={} am={:?} ao={:?} atog={} mv={}\
         \nno={:?} nn={:?} me={}\
         \nms={} mst={} rs={:?} rb={} ri={} rn={} rl={:?} jc={} nr={} cm={} mf={}\
         \nbn={} wm={} wv={}\
         \nct={:?} cc={:x} cf={:x} ck={} cb={:?} cd={:?} cp={} ch={} ce={:?} sec={:?}\
         \nop={:?} sx={} sy={} ssx={} ssy={} savx={} savy={}\
         \nmj={:?} sc={:?} ml={:?} sr={} mr={} or={} br={} bsl={} bch={} bb={:?}\
         \nms2={} msel={} mtl={} mfl={} mff={} mws={} mra={} mpm={:?} mem={:?} mh={} mm={}\
         \nmport={} mhost={} maddr={} mns={:?} mne={:?} mto={} mlc={:?} mhp={:?}\
         \ncard={:?} cred={:?}",
        fe.page, fe.cursor, fe.flow.page, fe.flow.get_state(), fe.flow.current_level,
        fe.flow.level_to_load, fe.flow.warm_reset, fe.requested_level, fe.mission, fe.difficulty,
        fe.outcome, fe.stats, fe.unlocked, fe.profiles, fe.active_profile, fe.profile.is_some(),
        fe.closed, fe.confirming_quit, fe.confirming_restart, fe.confirm_sel, fe.pause_tab,
        fe.pause_hint, fe.pause_objectives_sel, fe.pause_objectives, fe.confirming_default_profile,
        fe.intro_playing, fe.intro_index, fe.attract_playing, fe.attract_movie, fe.attract_opener,
        fe.attract_toggle, fe.menu_movie, fe.notice, fe.net_notice, fe.mp_error,
        fe.mp_scenario, fe.mission_stats.is_some(), fe.results, fe.results_box,
        fe.results_improved, fe.results_save_answer_no, fe.results_level, fe.just_completed,
        fe.new_reward, fe.cheat_medal_sel, fe.medal_from_cheat, fe.bonus.is_some(), fe.win_movie,
        fe.win_movie_vol, fe.cn_typed, fe.cn_cell, fe.cn_from, fe.cn_clear_on_key, fe.cn_box,
        fe.cn_delete_row, fe.cn_profile_mode, fe.cn_cheat_checked, fe.cn_exit, fe.secret, opts,
        fe.screen_x, fe.screen_y, fe.screen_saved_x, fe.screen_saved_y, fe.screen_av_saved_x,
        fe.screen_av_saved_y, fe.mp_join, fe.mp_scenario, fe.mp_level, fe.mp_scenario_row,
        fe.mp_map_row, fe.mp_options_row, fe.mp_bots_row, fe.mp_bot_slot, fe.mp_bot_char,
        fe.mp_bots, fe.mp_setup_state, fe.mp_setup_sel, fe.mp_time_limit, fe.mp_frag_limit,
        fe.mp_friendly_fire, fe.mp_weapon_set, fe.mp_radar, fe.mp_player_mods, fe.mp_envmods,
        fe.mp_handicap, fe.mp_match_settings.is_some(), fe.mp_net_port, fe.mp_hosting,
        fe.mp_net_addr, fe.mp_net_state, fe.mp_net_error, fe.mp_net_timeout, fe.mp_lobby_choices,
        fe.mp_host_published, fe.card, fe.credits,
    );
    s
}

// ── driving one edge ──────────────────────────────────────────────────────────────────────────

/// Ticks until quiescent, bounded, checking the invariants per tick.
fn settle(fe: &mut FrontEnd, problems: &mut Vec<String>) {
    for _ in 0..SETTLE_BOUND {
        fe.tick();
        problems.extend(check_step(fe));
        if quiescent(fe) {
            break;
        }
    }
}

fn settle_quiet(fe: &mut FrontEnd) {
    for _ in 0..SETTLE_BOUND {
        fe.tick();
        if quiescent(fe) {
            break;
        }
    }
}

/// The host's load pump + `take()` — the prologue's road in and `Edge::LoadTake`'s body.
fn load_and_take(fe: &mut FrontEnd) {
    for _ in 0..6000 {
        fe.tick();
        if fe.closed {
            break;
        }
        if fe.flow.get_state() == ST_RUNNER {
            if let Some(v) = fe.requested_level {
                if fe.flow.current_level == v {
                    fe.requested_level = None; // the host take()
                    break;
                }
            }
        }
    }
}

/// The played mission's base map — sp_level[mission]+0xc [M 0x20b8d8:21], what the host passes
/// results_enter/stats_enter as GameState+0x72.
fn mission_base(fe: &FrontEnd) -> u32 {
    CAMPAIGN.get(fe.mission).map_or(0, |c| c.3)
}

/// Applies one edge, settles, runs the invariants, tracks the armed `requested_level` and the
/// host's take of it. Returns whether the node is terminal (the host owns the screen now).
fn apply(fe: &mut FrontEnd, e: Edge, armed: &mut Option<u32>, problems: &mut Vec<String>) -> bool {
    match e {
        Edge::Act(a) => {
            fe.action(a);
            settle(fe, problems);
        }
        Edge::Grace => {
            for _ in 0..nf_view::menu::ESTHERO_GRACE { fe.tick(); }
        }
        Edge::Burst => {
            for _ in 0..BURST_TICKS {
                fe.tick();
                problems.extend(check_step(fe));
            }
        }
        Edge::Attract => {
            for _ in 0..ATTRACT_TICKS {
                fe.tick();
                problems.extend(check_step(fe));
            }
        }
        Edge::OpenPause => {
            fe.open_pause();
            settle(fe, problems);
        }
        Edge::MissionEnded(o) => {
            fe.mission_ended(
                o,
                Stats { shots_fired: 120, shots_hit: 40, accuracy_pct: 33, time_s: 754 },
            );
            settle(fe, problems);
        }
        Edge::HostEnter => match fe.page {
            Page::Results => {
                let mut progress = nf_game::progress::Progress::default();
                // GameState+0x72 = Mission_BaseMapHCode — the played mission's own base map
                // [M 0x208e78:14; sp_level record +0xc], not the flow's post-outro level.
                let base = mission_base(fe);
                fe.results_enter(&mut progress, base, 6, fe.difficulty as u32, None);
            }
            Page::Stats => {
                let mut progress = nf_game::progress::Progress::default();
                let base = mission_base(fe);
                fe.stats_enter(&mut progress, base, 6, fe.difficulty as u32, None);
            }
            Page::NfBonus => {
                let progress = nf_game::progress::Progress::default();
                let level = if fe.results_level != 0 { fe.results_level } else { mission_base(fe) };
                fe.bonus_enter(&progress, level, true);
            }
            _ => {}
        },
        Edge::WinMovieEnd => {
            fe.win_movie_finished();
            settle(fe, problems);
        }
        // Prologue-only edges — they ride paths (drive_path applies path edges through here),
        // but edges_for() never offers them as graph edges.
        Edge::LoadTake => {
            load_and_take(fe);
            settle(fe, problems);
        }
        Edge::UnlockAll => fe.unlock_all = true,
        Edge::Boot => {
            for _ in 0..8 {
                fe.tick();
                if fe.flow.get_state() == ST_RUNNER && fe.flow.current_level == FRONTEND_LEVEL {
                    break;
                }
            }
        }
    }
    // The invariants at every step.
    problems.extend(check_step(fe));
    // `requested_level` is never set twice for one confirm. The host's documented take() runs
    // when the requested level has finished loading (or the menu closed over the request).
    let mut consumed = false;
    match (*armed, fe.requested_level) {
        (None, Some(v)) => *armed = Some(v),
        (Some(v), Some(w)) if v != w => problems.push(format!(
            "requested_level changed {v:#x} → {w:#x} with no host consume in between (a second \
             load for one confirm)"
        )),
        (Some(_), Some(_)) => {}
        (Some(_), None) => {} // consumed below, or a legit clear after a load
        (None, None) => {}
    }
    if let Some(v) = *armed {
        if fe.flow.get_state() == ST_MAPLOAD && fe.flow.level_to_load != v {
            problems.push(format!(
                "a second MAPLOAD push while the {v:#x} request stands (level_to_load {:#x})",
                fe.flow.level_to_load
            ));
        }
        // The host take(): the requested level finished loading, or the menu closed over it.
        if (fe.flow.get_state() == ST_RUNNER && fe.flow.current_level == v) || fe.closed {
            fe.requested_level = None;
            *armed = None;
            consumed = true;
        }
    }
    // Terminal: the menu is gone (the host owns the screen), a match was handed over, or a real
    // level finished loading under the front end.
    fe.closed
        || fe.mp_match_settings.is_some()
        || (consumed && fe.flow.current_level != FRONTEND_LEVEL && fe.flow.current_level != 0)
}

/// Prologue application: the edge, then one plain tick — no invariant sweeps (the prologue is
/// asserted once, by hand, in the roots' constructors).
fn apply_raw(fe: &mut FrontEnd, e: Edge) {
    match e {
        Edge::Act(a) => {
            fe.action(a);
            fe.tick();
        }
        Edge::Burst | Edge::Attract => {}
        Edge::Grace => { for _ in 0..nf_view::menu::ESTHERO_GRACE { fe.tick(); } }
        Edge::OpenPause => {
            fe.open_pause();
            fe.tick();
        }
        Edge::MissionEnded(o) => {
            fe.mission_ended(o, Stats::default());
            fe.tick();
        }
        Edge::HostEnter => {
            let mut progress = nf_game::progress::Progress::default();
            match fe.page {
                Page::Results => {
                    fe.results_enter(&mut progress, fe.flow.current_level, 6, fe.difficulty as u32, None);
                }
                Page::Stats => {
                    fe.stats_enter(&mut progress, fe.flow.current_level, 6, fe.difficulty as u32, None);
                }
                Page::NfBonus => {
                    fe.bonus_enter(&progress, fe.results_level, true);
                }
                _ => {}
            }
        }
        Edge::WinMovieEnd => {
            fe.win_movie_finished();
            fe.tick();
        }
        Edge::LoadTake => load_and_take(fe),
        Edge::UnlockAll => fe.unlock_all = true,
        Edge::Boot => {
            for _ in 0..8 {
                fe.tick();
                if fe.flow.get_state() == ST_RUNNER && fe.flow.current_level == FRONTEND_LEVEL {
                    break;
                }
            }
        }
    }
}

/// Re-drives a path from a fresh [`FrontEnd::new()`] through the prologue. Deterministic: the
/// headless front end is a pure state machine (fixed 60 Hz, no wall clock, no RNG in the menus).
/// Returns the end state, the swept problems and the armed `requested_level` at the end.
fn drive_path(prologue: &[Edge], path: &[Edge]) -> (FrontEnd, Vec<String>, Option<u32>) {
    let mut fe = FrontEnd::new();
    let mut problems = Vec::new();
    let mut armed = None;
    // The prologue applies raw — it is the fixed road in (asserted by its constructor).
    for &e in prologue {
        apply_raw(&mut fe, e);
    }
    for &e in path {
        apply(&mut fe, e, &mut armed, &mut problems);
    }
    (fe, problems, armed)
}

/// The edges worth taking from a state (already filtered by applicability).
fn edges_for(fe: &FrontEnd, host_edges: bool) -> Vec<Edge> {
    let mut es: Vec<Edge> = ACTIONS.iter().map(|&a| Edge::Act(a)).collect();
    es.push(Edge::Burst);
    if matches!(fe.page, Page::Start | Page::Main) && !fe.attract_playing && !fe.intro_playing {
        es.push(Edge::Attract);
    }
    if host_edges {
        if fe.flow.current_level != FRONTEND_LEVEL
            && fe.flow.current_level != 0
            && fe.page != Page::Pause
        {
            es.push(Edge::OpenPause);
        }
        if fe.flow.current_level != FRONTEND_LEVEL && fe.flow.current_level != 0 {
            es.push(Edge::MissionEnded(Outcome::Complete));
            es.push(Edge::MissionEnded(Outcome::Failed));
        }
        if matches!(fe.page, Page::Results | Page::Stats | Page::NfBonus) {
            es.push(Edge::HostEnter);
        }
        if fe.win_movie {
            es.push(Edge::WinMovieEnd);
        }
    }
    es
}

// ── the walk ──────────────────────────────────────────────────────────────────────────────────

struct Walk {
    name: &'static str,
    prologue: Vec<Edge>,
    host_edges: bool,
    findings: Vec<String>,
    visited: HashSet<String>,
    per_page: HashMap<u32, usize>,
    per_page_cursor: HashMap<(u32, usize), usize>,
    queue: VecDeque<(Vec<Edge>, usize)>, // path, depth
    /// The first path that reached each page — the per-page probes re-drive it.
    page_paths: HashMap<u32, Vec<Edge>>,
    dead_ends: Vec<String>,
    pages: HashSet<u32>,
    deadline: std::time::Instant,
}

impl Walk {
    fn new(name: &'static str, prologue: Vec<Edge>, host_edges: bool) -> Self {
        Walk {
            name,
            prologue,
            host_edges,
            findings: Vec::new(),
            visited: HashSet::new(),
            per_page: HashMap::new(),
            per_page_cursor: HashMap::new(),
            queue: VecDeque::new(),
            page_paths: HashMap::new(),
            dead_ends: Vec::new(),
            pages: HashSet::new(),
            deadline: std::time::Instant::now() + WALK_DEADLINE,
        }
    }

    /// Records a reached node: the invariants, the per-page bookkeeping, the queue push.
    /// Terminal nodes (the host owns the screen) are recorded but not expanded.
    fn visit(&mut self, fe: &FrontEnd, path: Vec<Edge>, depth: usize, problems: Vec<String>, terminal: bool) {
        for p in problems {
            self.findings.push(format!("[{}] {}", self.name, p));
        }
        self.pages.insert(fe.page.id());
        self.page_paths.entry(fe.page.id()).or_insert_with(|| path.clone());
        let per = self.per_page.entry(fe.page.id()).or_insert(0);
        *per += 1;
        let perc = self.per_page_cursor.entry((fe.page.id(), fe.cursor)).or_insert(0);
        *perc += 1;
        let key = state_key(fe);
        if *per > PER_PAGE_CAP
            || *perc > PER_PAGE_CURSOR_CAP
            || self.visited.contains(&key)
            || depth > DEPTH_BOUND
            || self.visited.len() >= TOTAL_NODE_CAP
            || std::time::Instant::now() > self.deadline
        {
            return;
        }
        self.visited.insert(key);
        if !terminal {
            self.queue.push_back((path, depth));
        }
    }

    /// The BFS: pops nodes, re-drives their path, expands every edge.
    fn run(&mut self, root: Vec<Edge>) {
        let (fe, problems, _) = drive_path(&self.prologue, &root);
        self.visit(&fe, root, 0, problems, false);
        while let Some((path, depth)) = self.queue.pop_front() {
            if depth >= DEPTH_BOUND || std::time::Instant::now() > self.deadline {
                continue;
            }
            let (node, _, _) = drive_path(&self.prologue, &path);
            let base_key = state_key(&node);
            let edges = edges_for(&node, self.host_edges);
            let mut any_change = false;
            for e in edges {
                // A panicking edge is caught and reported (the next edge re-drives from a fresh
                // FrontEnd — "no panic" per step without losing the rest of the walk).
                let drive = || {
                    let prologue = &self.prologue;
                    let (mut fe2, problems2, mut arm) = drive_path(prologue, &path);
                    let mut problems2 = problems2;
                    let terminal = apply(&mut fe2, e, &mut arm, &mut problems2);
                    (fe2, problems2, terminal, arm)
                };
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(drive)) {
                    Ok((fe2, problems2, terminal, _arm2)) => {
                        let mut p2 = path.clone();
                        p2.push(e);
                        if state_key(&fe2) != base_key {
                            any_change = true;
                        }
                        self.visit(&fe2, p2, depth + 1, problems2, terminal);
                    }
                    Err(err) => {
                        let msg = err
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_else(|| "?".into());
                        self.findings.push(format!(
                            "[{}] PANIC on {:?} at path depth {}: {}",
                            self.name, e, depth, msg
                        ));
                        any_change = true; // a panic is a change — not a dead end
                    }
                }
            }
            // Dead end: no action and no tick burst changed anything (Back included above) and
            // the attract timeout cannot fire here — the page traps the player. The win page is
            // input-dead by the game's own design [M P_WINGAME 0x215100 has only the 0x4c/0x50
            // cases] — its route is the host's movie-end call.
            if !any_change && !fe.win_movie {
                self.dead_ends.push(format!(
                    "[{}] page {:?} is a dead end (path depth {})",
                    self.name,
                    fe.page,
                    path.len()
                ));
            }
        }
    }
}

// ── the roots ─────────────────────────────────────────────────────────────────────────────────

/// Boot → (intro skipped by the second Confirm) → Main. Asserted once here, then used raw.
fn main_prologue() -> Vec<Edge> {
    let mut fe = FrontEnd::new();
    for _ in 0..8 {
        fe.tick();
        if fe.flow.get_state() == ST_RUNNER && fe.flow.current_level == FRONTEND_LEVEL {
            break;
        }
    }
    assert_eq!(fe.flow.get_state(), ST_RUNNER, "boot reaches the front-end map");
    assert!(fe.action(Action::Confirm), "the title's Start starts the intro");
    for _ in 0..nf_view::menu::ESTHERO_GRACE { fe.tick(); } // boot-flow: P_ESTHERO's 3 s skip grace [M 0x20a060:18, Menu_StopMovie 0x215a50]
    assert!(fe.action(Action::Confirm), "the skip action pops the intro [M 0x1c98c8 case 6]");
    fe.tick();
    assert_eq!(fe.page, Page::Main, "the prologue lands on the main menu by actions only");
    // boot-flow: the START intro's 3 s grace (180 ticks) swallows an early skip.
    vec![Edge::Boot, Edge::Act(Action::Confirm), Edge::Grace, Edge::Act(Action::Confirm)]
}

/// Main → Codename → Play-without-saving → Difficulty → Single Player → confirm mission 1 (row 0
/// is the DRIVING mission — the port shows the notice [S port choice] — row 1 is unlocked at
/// boot [D sp_level +0x10]) → the load completes. All by actions; ends over a running mission.
fn mission_prologue() -> Vec<Edge> {
    let mut path = main_prologue();
    path.extend([
        Edge::Act(Action::Confirm), // NightFire → Select Codename
        Edge::Act(Action::Down),    // row 1 = Play without saving
        Edge::Act(Action::Confirm), // arm the confirm box
        Edge::Act(Action::Confirm), // ✕ through it → Select Difficulty
        Edge::Act(Action::Confirm), // the difficulty → Single Player
        Edge::Act(Action::Down),    // row 0 is the driving mission — step to row 1
        Edge::Act(Action::Confirm), // mission 1 (07000005) → the load arms
        Edge::LoadTake,             // the host pumps the load and takes the request
    ]);
    // The prologue road is RAW (one tick per action, no settles) — asserted here:
    let mut fe = FrontEnd::new();
    for &e in &path {
        apply_raw(&mut fe, e);
    }
    assert_eq!(fe.page, Page::MissionSelect, "the prologue reaches Single Player by actions");
    assert_eq!(fe.requested_level, None, "the take() left no pending request");
    assert!(
        fe.flow.current_level != FRONTEND_LEVEL && fe.flow.current_level != 0,
        "the prologue ends over a loaded mission ({:#x})",
        fe.flow.current_level
    );
    path
}

/// The win root's road in: `--unlock-all`, then mission 11 (0x0700001b, the final mission [M
/// 0x1ffa10's just_completed arm]) confirmed and loaded.
fn win_prologue() -> Vec<Edge> {
    let mut path = main_prologue();
    path.push(Edge::UnlockAll);
    path.extend([
        Edge::Act(Action::Confirm), // NightFire → Select Codename
        Edge::Act(Action::Down),    // Play without saving
        Edge::Act(Action::Confirm), // through the box
        Edge::Act(Action::Confirm), // → Select Difficulty
        Edge::Act(Action::Confirm), // → Single Player
    ]);
    for _ in 0..11 {
        path.push(Edge::Act(Action::Down)); // row 11 = the final mission (all rows unlocked)
    }
    path.push(Edge::Act(Action::Confirm));
    path.push(Edge::LoadTake);
    let mut fe = FrontEnd::new();
    for &e in &path {
        apply_raw(&mut fe, e);
    }
    assert_eq!(fe.flow.current_level, 0x0700_001b, "the final mission is loaded");
    assert_eq!(fe.page, Page::MissionSelect);
    assert_eq!(fe.requested_level, None);
    path
}

// ── the per-page probes ───────────────────────────────────────────────────────────────────────

/// Back leaves every non-root page within `BACK_STEPS` presses (a modal box eats one). The win
/// page is by design input-dead [M P_WINGAME 0x215100 has only the 0x4c/0x50 cases] and the
/// credits take no input at all (the roll cannot be skipped) [M P_CREDITS_Handler 0x2152b0 — no
/// input cases] — their routes are the host's movie-end call and the roll's own end.
fn probe_back(name: &str, prologue: &[Edge], path: &[Edge], findings: &mut Vec<String>) {
    let (mut fe, _, _) = drive_path(prologue, path);
    let start = fe.page;
    if matches!(start, Page::Start | Page::WinGame | Page::Credits) {
        return;
    }
    for _ in 0..BACK_STEPS {
        if fe.closed {
            return; // the menu closed — the page is gone
        }
        fe.action(Action::Back);
        settle_quiet(&mut fe);
        if fe.page != start || fe.closed {
            return;
        }
    }
    findings.push(format!(
        "[{name}] Back never leaves page {start:?} within {BACK_STEPS} presses (path depth {})",
        path.len()
    ));
}

/// Every reached page can get back to Main by actions only — a mini-BFS from the page's first
/// reached state. The goal: the Main page, quiescent. Terminal states (menu closed / match
/// handed over) don't expand — the host owns the screen there.
fn probe_home(name: &str, prologue: &[Edge], path: &[Edge], host_edges: bool, findings: &mut Vec<String>) {
    let (fe0, _, _) = drive_path(prologue, path);
    if fe0.page == Page::Main {
        return;
    }
    let goal = fe0.page;
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(state_key(&fe0));
    let mut queue: VecDeque<(Vec<Edge>, usize)> = VecDeque::new();
    queue.push_back((path.to_vec(), 0));
    while let Some((p, d)) = queue.pop_front() {
        if d >= HOME_DEPTH || visited.len() >= HOME_NODE_CAP {
            continue;
        }
        let (fe, _, arm) = drive_path(prologue, &p);
        let _ = arm;
        if fe.page == Page::Main && quiescent(&fe) {
            return;
        }
        if fe.closed || fe.mp_match_settings.is_some() {
            continue;
        }
        // The credits take no input (the roll cannot be skipped) [M P_CREDITS_Handler 0x2152b0
        // has no input cases] — the only edge is the roll's own ticks. The settle's per-action
        // tick would otherwise fork frame-variant states here (the roll is pure ticks).
        let edges: Vec<Edge> = if fe.page == Page::Credits {
            vec![Edge::Burst]
        } else {
            edges_for(&fe, host_edges)
        };
        for e in edges {
            let mut np = p.clone();
            np.push(e);
            let (fe2, _, _) = drive_path(prologue, &np);
            let k = state_key(&fe2);
            if std::env::var("WALK_DEBUG").is_ok() && visited.len() < 12 {
                println!("home-probe: d={d} edge={e:?} → page={:?} cursor={}", fe2.page, fe2.cursor);
            }
            if !visited.contains(&k) {
                visited.insert(k);
                queue.push_back((np, d + 1));
            }
        }
    }
    findings.push(format!(
        "[{name}] page {goal:?} cannot reach Main by actions within {HOME_DEPTH} steps (visited \
         {} nodes, max depth reached {})",
        visited.len(),
        queue.iter().map(|(_, d)| *d).max().unwrap_or(0)
    ));
}

// ── the deterministic BFS test ────────────────────────────────────────────────────────────────

/// Every ported page, for the coverage line and the known-id check.
const ALL_PAGES: [Page; 42] = [
    Page::Start, Page::Main, Page::Codename, Page::Difficulty, Page::MissionSelect, Page::Dossier,
    Page::Records, Page::Rewards, Page::Gadgets, Page::Weapons, Page::Options, Page::Pause,
    Page::Results, Page::Stats, Page::CnSelect, Page::CnMenu, Page::CnName, Page::CnAdvanced,
    Page::CnMp, Page::CnAv, Page::ScreenAdjust, Page::MpJoin, Page::MpScenario, Page::MpMap,
    Page::MpSetup, Page::MpOptions, Page::MpRules, Page::MpPlayerMods, Page::MpEnviroMods,
    Page::MpBots, Page::MpBotChoose, Page::MpBotSetup, Page::MpConfirm, Page::MpDebriefing,
    Page::MpMode, Page::MpHost, Page::MpNetJoin, Page::MpLobby, Page::Credits, Page::NfBonus,
    Page::WinGame, Page::CheatMedal,
];

#[test]
fn the_breadth_first_walk_covers_the_menu_graph_and_holds_the_invariants() {
    let mut findings: Vec<String> = Vec::new();
    // page id → (first path, walk name, host edges) — the probes re-drive the first reach.
    let mut first_reach: HashMap<u32, (Vec<Edge>, &'static str, bool)> = HashMap::new();

    // 1. The raw boot: the title, the intro, the attract.
    let mut w = Walk::new("boot", vec![], false);
    w.run(vec![]);
    for (id, p) in w.page_paths {
        first_reach.entry(id).or_insert((p, "boot", false));
    }
    findings.extend(w.findings.iter().cloned());
    findings.extend(w.dead_ends.iter().cloned());
    println!("[boot] pages {:?} nodes {}", {
        let mut v: Vec<_> = w.pages.iter().copied().collect();
        v.sort();
        v
    }, w.visited.len());

    // 2. The front end proper, from Main by actions.
    let mut w = Walk::new("main", main_prologue(), false);
    w.run(vec![]);
    for (id, p) in w.page_paths {
        first_reach.entry(id).or_insert((p, "main", false));
    }
    findings.extend(w.findings.iter().cloned());
    findings.extend(w.dead_ends.iter().cloned());
    println!("[main] pages {:?} nodes {}", {
        let mut v: Vec<_> = w.pages.iter().copied().collect();
        v.sort();
        v
    }, w.visited.len());

    // 3. The pause root: over a loaded mission, the host's Esc.
    let mut w = Walk::new("pause", mission_prologue(), true);
    w.run(vec![Edge::OpenPause]);
    for (id, p) in w.page_paths {
        first_reach.entry(id).or_insert((p, "pause", true));
    }
    findings.extend(w.findings.iter().cloned());
    findings.extend(w.dead_ends.iter().cloned());
    println!("[pause] pages {:?} nodes {}", {
        let mut v: Vec<_> = w.pages.iter().copied().collect();
        v.sort();
        v
    }, w.visited.len());

    // 4. The results root: over a loaded mission, the host reports the mission end.
    let mut w = Walk::new("results", mission_prologue(), true);
    w.run(vec![]);
    for (id, p) in w.page_paths {
        first_reach.entry(id).or_insert((p, "results", true));
    }
    findings.extend(w.findings.iter().cloned());
    findings.extend(w.dead_ends.iter().cloned());
    println!("[results] pages {:?} nodes {}", {
        let mut v: Vec<_> = w.pages.iter().copied().collect();
        v.sort();
        v
    }, w.visited.len());

    // 5. The win root: the final mission completed — the bonus/win/cheat-medal pages.
    let mut w = Walk::new("win", win_prologue(), true);
    w.run(vec![]);
    for (id, p) in w.page_paths {
        first_reach.entry(id).or_insert((p, "win", true));
    }
    findings.extend(w.findings.iter().cloned());
    findings.extend(w.dead_ends.iter().cloned());
    println!("[win] pages {:?} nodes {}", {
        let mut v: Vec<_> = w.pages.iter().copied().collect();
        v.sort();
        v
    }, w.visited.len());

    // The per-page probes: Back leaves, and every page can reach Main by actions.
    let mut probe_list: Vec<(u32, Vec<Edge>, &'static str, bool)> = first_reach
        .into_iter()
        .map(|(id, v)| (id, v.0, v.1, v.2))
        .collect();
    probe_list.sort_by_key(|(id, ..)| *id);
    for (_id, path, name, host) in &probe_list {
        // The walk's own prologue per name.
        let prologue: Vec<Edge> = match *name {
            "boot" => vec![],
            "main" => main_prologue(),
            "pause" | "results" => mission_prologue(),
            "win" => win_prologue(),
            _ => unreachable!(),
        };
        probe_back(name, &prologue, path, &mut findings);
        probe_home(name, &prologue, path, *host, &mut findings);
    }

    // The coverage line: pages reached / all pages.
    let reached: HashSet<u32> = probe_list.iter().map(|(id, ..)| *id).collect();
    let unreached: Vec<&Page> = ALL_PAGES.iter().filter(|p| !reached.contains(&p.id())).collect();
    println!(
        "menu-graph-walk coverage: {}/{} pages reached",
        reached.len(),
        ALL_PAGES.len()
    );
    println!("unreached pages: {unreached:?}");

    if !findings.is_empty() {
        findings.dedup();
        panic!(
            "menu-graph-walk found {} invariant problems:\n{}",
            findings.len(),
            findings.join("\n")
        );
    }
}

// ── the randomized version ────────────────────────────────────────────────────────────────────

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn twenty_seeded_random_runs_hold_the_invariants() {
    let mut findings: Vec<String> = Vec::new();
    for seed in 0..20u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x1234_5678) | 1);
        let mut fe = FrontEnd::new();
        let mut armed: Option<u32> = None;
        for step in 0..5000u32 {
            if fe.closed || fe.mp_match_settings.is_some() {
                // The host owns the screen — the window relaunches the front end.
                fe = FrontEnd::new();
                armed = None;
                continue;
            }
            let r = rng.next() % 100;
            let e = if r < 58 {
                Edge::Act(ACTIONS[(rng.next() % 7) as usize])
            } else if r < 88 {
                Edge::Burst
            } else if matches!(fe.page, Page::Start | Page::Main)
                && !fe.attract_playing
                && !fe.intro_playing
            {
                Edge::Attract
            } else if r < 92
                && fe.flow.current_level != FRONTEND_LEVEL
                && fe.flow.current_level != 0
                && fe.page != Page::Pause
            {
                Edge::OpenPause
            } else if r < 96
                && fe.flow.current_level != FRONTEND_LEVEL
                && fe.flow.current_level != 0
            {
                Edge::MissionEnded(if rng.next() % 2 == 0 { Outcome::Complete } else { Outcome::Failed })
            } else if matches!(fe.page, Page::Results | Page::Stats | Page::NfBonus) {
                Edge::HostEnter
            } else if fe.win_movie {
                Edge::WinMovieEnd
            } else {
                Edge::Burst
            };
            let mut problems = Vec::new();
            let terminal = apply(&mut fe, e, &mut armed, &mut problems);
            for p in problems {
                findings.push(format!("[rand seed {seed} step {step}] {p}"));
            }
            // The host drains the sound queues every frame.
            fe.sfx_out.clear();
            fe.sfx_remove_out.clear();
            if terminal {
                fe = FrontEnd::new();
                armed = None;
            }
        }
    }
    if !findings.is_empty() {
        findings.dedup();
        panic!(
            "the randomized runs found {} invariant problems:\n{}",
            findings.len(),
            findings.join("\n")
        );
    }
}
