//! pause-results-flows: the in-mission menus end to end, on a live 07000005 (NF_ISO) — the pause
//! page (Esc open, the four tabs via Left/Right and Tab, Continue, Restart → YES/NO, Quit → YES/NO),
//! the fail menu (die → Restart / Quit), and mission complete → results → NfBonus/MissionSelect.
//!
//! Driving discipline (the task rule): every page change rides [`FrontEnd::action`] / `tick()` —
//! never a page/cursor assignment to get somewhere. The window's host glue (lib.rs) is mirrored
//! where the headless tests can reach it; every mirror cites its lib.rs arm.
//!
//! Game evidence: the pause open [M Player_Update 0x193600:71-78 — action 0x1e, `GameState._36_4_ == 0`,
//! `ScriptCam == 0`, then `MenuManager_Create(0x80000002, 0x4000004b, …, 0x1e, 1, 3)` + `GS_PauseGame(1)`];
//! the paused object pump [M Game_Run 0x1c9790:30-35]; the pause rows CONTINUE/RESTART/QUIT and their
//! YES/NO boxes [L page 0x4000004b's d1=6 list, 0x5df/0x5e0; M C_GCPAUSE_Handler 0x204408:134-154 —
//! restart = BaseMap COLD 0x204408:139, quit = +0x28 0x4000001c + the front-end map 0x204408:142-149];
//! the fail menu [M Mission_Update 0x182c60:77-81 → page 0x40000042, P_ENDMISSION_Handler 0x216618:19-27
//! — row 0 = the CURRENT level WARM, quit = the front end]; the results chain [M Mission_Update
//! 0x182c60:85-122 + P_NFRESULTS 0x208e78 + P_NFBONUS 0x209670]; the load's own queued-page
//! MenuManager_Create [M ResetMap_Load 0x1d6360:160-163].

use nf_view::menu::{self, Action, FrontEnd, Outcome, Page, Stats};
use nf_view::scene::{Scene, Sim};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_level(iso: &str, level: &str) -> Scene {
    nf_view::load(iso, level, false).expect("load")
}

fn tick(sim: &mut Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// One tick with the fire button held — the ✕ skip (action 25) [M 0x190bb0:30-33].
fn tick_fire(sim: &mut Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), true, false, false); }
}

/// One action, then the frames a player would wait (menus_e2e's driver).
fn act(fe: &mut FrontEnd, a: Action) -> bool {
    let r = fe.action(a);
    for _ in 0..60 { fe.tick(); }
    r
}

/// lib.rs:2871-2887 — the window's Esc arm over a live mission: seed the front end, aim it at the
/// live level (GameState+0xc), pick the campaign row from the mission's base map, ride the runner
/// state, and open the pause page. The grab toggle before it is window-only.
fn esc_in_mission(fe: &mut Option<FrontEnd>, sim: &Sim) -> bool {
    if fe.is_none() { *fe = Some(FrontEnd::new()); }
    let fe = fe.as_mut().unwrap();
    fe.flow.current_level = sim.level;                       // lib.rs:2871 (self.level_code)
    let base = sim.drones.mission.base_map;                  // lib.rs:2875 Mission_BaseMapHCode 0x1830a0
    fe.mission = menu::CAMPAIGN.iter().position(|c| c.3 == base)
        .unwrap_or_else(|| panic!("the base map 0x{base:08x} is not a campaign row"));  // lib.rs:2881
    fe.flow.stack = vec![menu::ST_RUNNER];                   // lib.rs:2886 — the game's state 2 under Mission_Update
    fe.open_pause()                                          // lib.rs:2887 → menu.rs open_pause
}

/// lib.rs:1822-1851 — `App::menu_after`: the host consumes the front end's requests once per frame.
/// Returns the level the host must load (the pending_level); the caller drops the fe when `closed`.
fn menu_after(fe: &mut FrontEnd, sim: &mut Sim) -> Option<u32> {
    let mut pending = None;
    if let Some(level) = fe.requested_level.take() {         // lib.rs:1822
        if !fe.flow.warm_reset { sim.drones.level_exit_cold = true; }  // lib.rs:1825 [M 0x204408:139 the cold arm]
        pending = Some(level);
    }
    pending
}

/// lib.rs:3090/3400 — the redraw's tick gate: the sim steps ONLY when no front end stands; with
/// one up, the frame ticks the front end instead.
fn host_frame(scene: &mut Scene, fe: &mut Option<FrontEnd>) {
    match fe {
        Some(fe) => fe.tick(),
        None => scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false),
    }
}

/// The window's per-frame pause populate (lib.rs:3431-3433 → `pause_objective_rows` lib.rs:3885)
/// [M C_GCPAUSE_Handler 0x204408:251-271 — Mission_NumVisObjectives 0x182540 rows].
fn feed_pause_objectives(fe: &mut FrontEnd, sim: &Sim) {
    let m = &sim.drones.mission;
    fe.pause_objectives = (0..m.num_vis_objectives()).filter_map(|i| m.objective_state(i))
        .map(|o| menu::PauseObjective { text: o.text, hint: o.hint, done: o.state == 4 })
        .collect();
}

fn chain_iso_lang(iso_path: &str) -> Option<nf_formats::hud::Lang> {
    let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("USATxt.dat"))?;
    let bytes = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::hud::Lang::parse(&bytes).ok()
}

/// ── 1. Esc opens the pause page; the sim stands still until the menu closes ────────────────────
#[test]
fn esc_opens_the_pause_page_and_the_sim_stands_still_until_it_closes() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_level(&iso, "07000005.bin");
    tick(scene.sim.as_mut().unwrap(), 10);
    let mut fe: Option<FrontEnd> = None;

    // Esc over the live mission opens the pause page on its MISSION tab [M 0x193600:71-78].
    assert!(esc_in_mission(&mut fe, scene.sim.as_ref().unwrap()), "Esc opens the pause page");
    {
        let fe = fe.as_mut().unwrap();
        assert_eq!(fe.page, Page::Pause);
        assert_eq!(fe.pause_tab, 0, "opens on MISSION");
        assert_eq!(fe.cursor, 0, "opens on CONTINUE [L page 0x4000004b d1=6: 0x16a/0x16b/0x16c]");
        assert!(!fe.confirming_restart && !fe.confirming_quit, "no confirm box armed");
        assert_eq!(fe.confirm_sel, 1, "the YES/NO box opens on NO [L 0x5df/0x5e0]");
    }
    // The sim is frozen while the menu stands: the host's tick gate (lib.rs:3090) is the port's
    // GS_PauseGame(1) [M Game_Run 0x1c9790:30-35 — the whole object pump is skipped while paused].
    let (frame0, pos0) = { let s = scene.sim.as_ref().unwrap(); (s.drones.frame, s.bond.pos) };
    for _ in 0..120 { host_frame(&mut scene, &mut fe); }
    {
        let s = scene.sim.as_ref().unwrap();
        assert_eq!(s.drones.frame, frame0, "the sim is frozen while the pause page stands");
        assert_eq!(s.bond.pos, pos0, "Bond stands still under the menu");
    }
    // Esc again resumes — the close is manager-level, not a per-tab gate [M MenuManager_Monitor
    // 0x21d508:27-38: the manager's close action (+0x1c8 = Create's param_5, the pause's 0x1e
    // [M 0x193600:73-75]) must differ from the fail menu's 0x28 [M 0x182c60:78] and be pressed;
    // the manager+0x7b == 7 in :35-38 is the CREATE-time sentinel [M 0x21b908:131 — the only
    // writer in the decompile], always true for a live manager and vacuous; the delete runs
    // GS_PauseGame(0) [M 0x21c2b0:11-15]].
    let closed = {
        let fe = fe.as_mut().unwrap();
        assert!(act(fe, Action::Back), "Esc/Back resumes");
        assert!(fe.closed && fe.requested_level.is_none(), "a resume, not a load");
        fe.closed
    };
    assert!(closed);
    fe = None;                                                // lib.rs:1861 close_front_end
    for _ in 0..30 { host_frame(&mut scene, &mut fe); }
    let s = scene.sim.as_ref().unwrap();
    assert!(s.drones.frame > frame0, "the sim runs again after the menu closed");
}

/// ── 2. The four tabs cycle with Left/Right and the Tab key ─────────────────────────────────────
#[test]
fn the_pause_tabs_cycle_with_left_right_and_side() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // 07000007 — The Exchange's next part: the level-reached rule announces its rows at spawn
    // [M 0x1827d8:41-54 — case 0 announces the records whose level is reached; 05's own first
    // objective waits for its enable-channel trigger].
    let mut scene = load_level(&iso, "07000007.bin");
    tick(scene.sim.as_mut().unwrap(), 2);
    let n = scene.sim.as_ref().unwrap().drones.mission.num_vis_objectives();
    let mut fe: Option<FrontEnd> = None;
    assert!(esc_in_mission(&mut fe, scene.sim.as_ref().unwrap()));
    let fe = fe.as_mut().unwrap();
    feed_pause_objectives(fe, scene.sim.as_ref().unwrap());
    assert!(n >= 1, "the live mission's visible objectives feed the OBJECTIVES tab");

    // Right → OBJECTIVES; entering the tab re-selects its LAST row [M 0x204408 msg 0x1d = numVis-1].
    assert!(act(fe, Action::Right));
    assert_eq!(fe.pause_tab, 1, "OBJECTIVES");
    assert_eq!(fe.pause_objectives_sel, n - 1, "the list opens on its last row [M 0x204408:0x204840]");
    if n > 1 {
        assert!(act(fe, Action::Up), "the list scrolls");
        assert_eq!(fe.pause_objectives_sel, n - 2);
    }
    // A row's ✕ hint opens on Confirm and pops on the next key [L the ~I/~J hint binds;
    // the view's 0x31d fallback for hint-less rows].
    assert!(act(fe, Action::Confirm), "the row's hint opens");
    assert!(fe.pause_hint.is_some(), "the ✕ hint box stands");
    assert!(act(fe, Action::Confirm), "the hint pops");
    assert!(fe.pause_hint.is_none());
    assert!(act(fe, Action::Right)); assert_eq!(fe.pause_tab, 2, "CONTROLS");
    assert!(act(fe, Action::Right)); assert_eq!(fe.pause_tab, 3, "SCORE");
    assert!(act(fe, Action::Right)); assert_eq!(fe.pause_tab, 0, "the strip wraps");
    assert!(act(fe, Action::Left));  assert_eq!(fe.pause_tab, 3, "Left cycles back");
    assert!(act(fe, Action::Left));  assert_eq!(fe.pause_tab, 2);
    // Tab (the game's ~H Next) steps one tab [L the page hint 0x201; the four tabs are the page's
    // type-1 controls].
    assert!(act(fe, Action::Side));  assert_eq!(fe.pause_tab, 3);
    // Esc resumes from a non-MISSION tab too [M 0x21d508 — the close action is the manager's].
    assert!(act(fe, Action::Back));
    assert!(fe.closed);
}

/// ── 3. Continue closes the menu; the mission resumes ───────────────────────────────────────────
#[test]
fn pause_continue_resumes_the_mission() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_level(&iso, "07000005.bin");
    tick(scene.sim.as_mut().unwrap(), 10);
    let mut fe: Option<FrontEnd> = None;
    assert!(esc_in_mission(&mut fe, scene.sim.as_ref().unwrap()));
    let frame0 = scene.sim.as_ref().unwrap().drones.frame;
    {
        let fe = fe.as_mut().unwrap();
        assert!(act(fe, Action::Confirm), "CONTINUE confirms [M 0x204408:186-192 — the case-0 arm deletes the manager]");
        assert!(fe.closed && fe.requested_level.is_none(), "a resume, not a load");
    }
    fe = None;                                                // lib.rs:1861
    for _ in 0..30 { host_frame(&mut scene, &mut fe); }
    assert!(scene.sim.as_ref().unwrap().drones.frame > frame0, "the mission resumed");
}

/// ── 4. Pause Restart: the base map, COLD, on full health (the entry checkpoint) ─────────────────
#[test]
fn pause_restart_reloads_the_base_map_cold_on_full_health() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_level(&iso, "07000005.bin");
    tick(scene.sim.as_mut().unwrap(), 10);
    let mut fe: Option<FrontEnd> = None;
    assert!(esc_in_mission(&mut fe, scene.sim.as_ref().unwrap()));
    {
        let fe = fe.as_mut().unwrap();
        // Down to RESTART; its Confirm arms the YES/NO box, opening on NO [M 0x204408 case 1
        // binds 0x1ff "RESTART: ARE YOU YOU SURE?"; L 0x5df/0x5e0 open on NO].
        assert!(act(fe, Action::Down));
        assert_eq!(fe.cursor, 1);
        assert!(act(fe, Action::Confirm));
        assert!(fe.confirming_restart && !fe.confirming_quit);
        assert_eq!(fe.confirm_sel, 1, "the box opens on NO");
        // NO closes the box and requests nothing; the handler enters state_ignore_167, which
        // swallows exactly ONE further 0x4b before the rows answer again [M 0x204408:165-171 —
        // the d1=6 arm returns on it once], so the player's next Confirm re-arms RESTART.
        assert!(act(fe, Action::Confirm));
        assert!(!fe.confirming_restart && fe.requested_level.is_none() && !fe.closed);
        // Re-open and answer YES [M 0x204408:134-141 — ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)].
        assert!(act(fe, Action::Confirm));
        assert!(act(fe, Action::Up));
        assert_eq!(fe.confirm_sel, 0, "YES");
        assert!(act(fe, Action::Confirm));
        assert_eq!(fe.requested_level, Some(0x0700_0005),
            "Restart → the mission's base map [M 0x204408:139; CAMPAIGN[1].3 = 0x07000005]");
        assert!(!fe.flow.warm_reset, "the cold arm [M 0x204408:139 — param 2 = 0]");
        assert!(fe.closed, "the host closes the menu [M 0x204408:153 MenuManager_Delete]");
    }
    // The host takes the request exactly once (lib.rs:1822) — a repeated press cannot double-request.
    let pending = menu_after(fe.as_mut().unwrap(), scene.sim.as_mut().unwrap());
    assert_eq!(pending, Some(0x0700_0005), "the host takes the restart request once");
    assert!(fe.as_mut().unwrap().requested_level.is_none(), "a second menu_after takes nothing");
    // The host's load: cold ⇒ no carry (lib.rs:1976-1990) — the fresh load the restart serves.
    let mut reloaded = load_level(&iso, "07000005.bin");
    {
        let sim = reloaded.sim.as_mut().unwrap();
        tick(sim, 5);
        assert_eq!(sim.drones.health.health, 100.0, "cold load → full health [M Player_Init 0x1910f8:128-138]");
        assert_eq!(sim.drones.health.armour, 0.0, "cold load → no armour");
        assert_eq!(sim.drones.mission.status(), 1, "the mission plays again [M 0x1d5080 Mission_Init]");
        assert!(!sim.drones.channels.on(99), "Init_SwitchChannels zeroed the channels [M 0x1d5080:44-46]");
        // The host re-captures the entry checkpoint after the load (lib.rs:2141).
        let entry = nf_view::entry_checkpoint_save(sim);
        assert_eq!(entry.health, 100.0, "the entry checkpoint holds the fresh load's health");
        tick(sim, 360);
        assert_eq!(sim.drones.mission.status(), 1, "no replay loop on the cold reload");
        assert!(sim.drones.level_exit.is_none(), "no restart queued");
    }
}

/// ── 5. Pause Quit: the front end comes back, and its mission list works ────────────────────────
#[test]
fn pause_quit_returns_to_a_working_front_end() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_level(&iso, "07000005.bin");
    tick(scene.sim.as_mut().unwrap(), 10);
    let mut fe: Option<FrontEnd> = None;
    assert!(esc_in_mission(&mut fe, scene.sim.as_ref().unwrap()));
    {
        let fe = fe.as_mut().unwrap();
        // Down twice to QUIT; YES [M 0x204408:142-149 — +0x1c = 0x4000001c + ResetMap_LevelToLoad(0x7000048, 0, 0)].
        assert!(act(fe, Action::Down));
        assert!(act(fe, Action::Down));
        assert_eq!(fe.cursor, 2);
        assert!(act(fe, Action::Confirm));
        assert!(fe.confirming_quit && !fe.confirming_restart);
        assert!(act(fe, Action::Up));
        assert!(act(fe, Action::Confirm));
        assert_eq!(fe.requested_level, Some(menu::FRONTEND_LEVEL), "Quit → the front-end map [M 0x204408:147]");
        assert_eq!(fe.flow.page, menu::PAGE_RESULTS_QUIT_TARGET, "+0x1c = 0x4000001c [M 0x204408:144]");
    }
    let pending = menu_after(fe.as_mut().unwrap(), scene.sim.as_mut().unwrap());
    assert_eq!(pending, Some(menu::FRONTEND_LEVEL), "the host loads the front-end map");
    // The real front-end archive builds (the host's swap keeps the fe for it, lib.rs:2052-2075).
    let _fe_map = load_level(&iso, "07000048.bin");
    // The load's own MenuManager_Create re-opens the queued page [M 0x1d6360:160-163] — the host's
    // swap branch calls reopen_queued_page for exactly this (lib.rs:2074).
    // fe-audit: the quit arm already put the mission list up (the port's stand-in for the manager
    // the load creates on +0x1c), so the re-open finds it in place.
    {
        let fe = fe.as_mut().unwrap();
        let _ = fe.reopen_queued_page();
        assert_eq!(fe.flow.page, menu::PAGE_RESULTS_QUIT_TARGET);
        assert_eq!(fe.page, Page::MissionSelect, "the queued mission list is up [M 0x204408:141-152, 0x1d6360:160-163]");
        assert_eq!(fe.cursor, 1, "on the highest unlocked row [M P_NFMAP 0x20b608:58-64]");
    }
    // The list works: row 0 is the driving Paris Prelude (a notice, no request — [S port substitute:
    // the game sets level_chosen (C_SBNFMAP 0x20b8d8:24-40) and P_NFMAP hands over to DRIVING.ELF
    // (0x20b608:15-27); the port has no driving engine]), the next row is The Exchange.
    {
        let fe = fe.as_mut().unwrap();
        assert!(fe.unlocked_row(1), "a fresh profile's unlocked rows");
        assert!(act(fe, Action::Up), "Up to Paris Prelude");
        assert!(act(fe, Action::Confirm), "the driving row confirms");
        assert!(fe.notice.is_some(), "the driving notice shows");
        assert_eq!(fe.requested_level, None, "no level for a driving mission");
        assert!(act(fe, Action::Confirm), "Enter dismisses the notice");
        assert!(act(fe, Action::Down));
        assert!(act(fe, Action::Confirm), "The Exchange starts");
        assert_eq!(fe.requested_level, Some(0x0700_0005), "the mission start rides the request");
        // A repeated press cannot double-request [M the pick deletes the manager — C_SBNFMAP
        // 0x20b8d8:36-38 MenuManager_Delete; the port's MissionSelect guard].
        assert!(!fe.action(Action::Confirm), "a second Confirm is refused while a request stands");
        assert_eq!(fe.requested_level, Some(0x0700_0005), "still one request");
    }
}

/// ── 6. Dying opens the fail menu; Restart re-serves the entry checkpoint ───────────────────────
#[test]
fn dying_opens_the_fail_menu_and_restart_re_serves_the_entry_checkpoint() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_level(&iso, "07000005.bin");
    let entry = {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        // The entry checkpoint the host captured at the load (lib.rs:2141).
        nf_view::entry_checkpoint_save(sim)
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.auto_restart = false;                      // the view answers the menu itself
    }
    // Kill Bond the real way (500-damage pain → the death sequence [M Player_CheckForDeath 0x190810])
    // and ✕ past the red fade (action 25 raises ch 0x62 [M Player_HandleDeath 0x190bb0:30-33]).
    {
        let sim = scene.sim.as_mut().unwrap();
        let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng);
        h.pain_generic(r, 500.0, 1, 0x0700_0005, 0);
    }
    tick(scene.sim.as_mut().unwrap(), 1);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.death.active && sim.drones.death.pane_on, "the death sequence started");
    }
    tick_fire(scene.sim.as_mut().unwrap(), 1);
    tick(scene.sim.as_mut().unwrap(), 6);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x62), "the death channel rose [M 0x190bb0:32]");
        assert!(sim.drones.mission.menu_open, "the 0x40000042 fail menu opened [M 0x182c60:77-81]");
        assert!(sim.drones.fail_menu_open(), "the menu waits for the player's choice");
        assert!(sim.drones.mission.menu_choice.is_none(), "nothing chosen yet");
    }
    // The host's results hook (lib.rs:3296) mirrors the menu onto the front end's FAILED page.
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![menu::ST_RUNNER];
    fe.flow.current_level = 0x0700_0005;
    fe.mission = menu::CAMPAIGN.iter().position(|c| c.3 == 0x0700_0005).unwrap();
    fe.mission_ended(Outcome::Failed, Stats::default());
    {
        assert_eq!(fe.page, Page::Results);
        assert_eq!(fe.rows(), 2, "REPLAY / QUIT [L page 0x40000042: 0x5bc value 0, 0x16c value 2]");
        // fe-audit: the failure loads nothing — the fail menu sits over the paused mission [M
        // Mission_Update 0x182c60:77-80; cases 2/3 only set LevelToEndTo].
        assert_eq!(fe.requested_level, None, "the failure requests no level [M 0x182c60:77-80]");
        // Row 0 (REPLAY) = the CURRENT level, WARM [M P_ENDMISSION_Handler 0x216618:19-21].
        assert!(fe.action(Action::Confirm));
        assert_eq!(fe.requested_level, Some(0x0700_0005), "row 0 → the current level");
        assert!(fe.flow.warm_reset, "row 0 is the warm arm [M 0x216618:20 — param 2 = 1]");
        assert!(fe.closed, "the arm deletes the manager [M 0x216618:30 MenuManager_Delete]");
    }
    // The sim answers Restart the same way (fix_restart_target's chain).
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.fail_choose_restart();
        assert_eq!(sim.drones.level_exit, Some(0x0700_0005), "Restart → the current level [M 0x216618:20]");
        assert!(!sim.drones.level_exit_cold, "the warm arm");
        assert!(sim.drones.level_exit_restart, "on the entry checkpoint [M 0x216618:21]");
    }
    // The host serves the restart from the ENTRY checkpoint (lib.rs:1976-1990 restart arm →
    // fail_restart_save(entry_save, …)) [M Player_RamLoad 0x1957a8 — IsWarmReset-gated restore].
    let served = {
        let sim = scene.sim.as_ref().unwrap();
        nf_view::fail_restart_save(Some(&entry), sim)
    };
    assert_eq!(served.health, entry.health.max(50.0), "the 50.0 ContinueHealthBoost floor [M 0x1957a8]");
    let mut reloaded = nf_view::load_with_carry(&iso, "07000005.bin", false, Some(&served)).expect("the warm reload");
    {
        let sim = reloaded.sim.as_mut().unwrap();
        tick(sim, 5);
        assert_eq!(sim.drones.health.health, 100.0, "the entry checkpoint's full health comes back");
        assert_eq!(sim.drones.mission.status(), 1, "the mission plays again");
        assert!(!sim.drones.channels.on(0x62) && !sim.drones.channels.on(99),
            "the entry checkpoint's clean channels — no replay loop [M 0x1957a8:17-20]");
        tick(sim, 360);
        assert_eq!(sim.drones.mission.status(), 1, "360 ticks without a re-fail");
        assert!(sim.drones.level_exit.is_none(), "no restart queued");
    }
}

/// Sister: the fail menu's QUIT row and the sim's own quit answer.
#[test]
fn the_fail_menu_quit_row_returns_to_the_front_end() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_level(&iso, "07000005.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        sim.drones.auto_restart = false;
        let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng);
        h.pain_generic(r, 500.0, 1, 0x0700_0005, 0);
    }
    tick(scene.sim.as_mut().unwrap(), 1);                     // the death activates on its first tick
    tick_fire(scene.sim.as_mut().unwrap(), 1);
    tick(scene.sim.as_mut().unwrap(), 6);
    assert!(scene.sim.as_ref().unwrap().drones.mission.menu_open, "the fail menu is up");
    // The front end's QUIT row (cursor 1 = item value 2) [M 0x216618:24-26].
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![menu::ST_RUNNER];
    fe.flow.current_level = 0x0700_0005;
    fe.mission_ended(Outcome::Failed, Stats::default());
    assert_eq!(fe.requested_level, None, "the failure requests no level [M 0x182c60:77-80]");
    assert!(act(&mut fe, Action::Down), "to the QUIT row");
    assert_eq!(fe.cursor, 1);
    assert!(act(&mut fe, Action::Confirm));
    assert_eq!(fe.requested_level, Some(menu::FRONTEND_LEVEL), "row 1 → the front end [M 0x216618:25]");
    assert_eq!(fe.flow.page, menu::PAGE_RESULTS_QUIT_TARGET, "+0x1c = 0x4000001c [M 0x216618:25]");
    assert_eq!(fe.page, Page::MissionSelect, "the front end reopens on the mission list [M 0x20b608]");
    // The sim's own answer matches [M 0x216618:24-26]: the front end, COLD.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.fail_choose_quit();
        assert_eq!(sim.drones.level_exit, Some(menu::FRONTEND_LEVEL), "quit → the front-end map");
        assert!(sim.drones.level_exit_cold, "the quit is the cold arm");
        assert!(!sim.drones.level_exit_restart);
    }
}

/// ── 7. Mission complete → results → the bonus page / the mission list ──────────────────────────
#[test]
fn completing_the_mission_reaches_the_results_and_the_next_pages() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let lang = chain_iso_lang(&iso);
    let mut scene = load_level(&iso, "07000005.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 10);
        sim.drones.set_channel(0x64);                          // ch 100: mission complete [M 0x182c60:85]
    }
    // Case 6: the 240-tick banner → case 7's pause → case 4 queues the end sequence.
    let end_to = loop {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        if let Some(next) = sim.drones.mission_end { break Some(next); }
        if sim.drones.frame > 2000 { break None; }
    };
    assert_eq!(end_to, Some(menu::outro_for(0x0700_0005)),
        "the end sequence queued [M 0x182c60:89-116 — 05 has no outro, the default is the front-end map]");
    assert_eq!(end_to, Some(menu::FRONTEND_LEVEL));
    // The host's results hook (lib.rs:3296): mission_ended + results_enter against the live progress.
    let mut progress = nf_game::progress::Progress::default();
    progress.set_nightfire_status(0x3);                        // boot: rows 0,1 unlocked [D sp_level]
    let difficulty = progress.difficulty;
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![menu::ST_RUNNER];
    fe.flow.current_level = 0x0700_0005;
    fe.mission = menu::CAMPAIGN.iter().position(|c| c.3 == 0x0700_0005).unwrap();
    fe.mission_ended(Outcome::Complete, Stats::default());
    assert_eq!(fe.page, Page::Results);
    assert_eq!(fe.requested_level.take(), Some(menu::outro_for(0x0700_0005)), "the end code rode the request");
    // The wrap: the plain front-end load completes in the model (no outro for 05 — no movie wrap,
    // [M 0x1d3800's switch has no 0x7000048]); the results page stays queued [M 0x182c60:72-74 set
    // +0x1c = 0x40000036 before the load] and the load's own MenuManager_Create re-opens it
    // [M 0x1d6360:160-163].
    let mut landed = false;
    for _ in 0..64 {
        fe.tick();
        if fe.flow.get_state() == menu::ST_RUNNER && fe.flow.current_level == menu::FRONTEND_LEVEL { landed = true; break; }
    }
    assert!(landed, "the front-end map completed in the model");
    assert_eq!(fe.flow.page, menu::PAGE_RESULTS, "+0x1c = 0x40000036 [M 0x1d6360]");
    assert_eq!(fe.page, Page::Results, "the results page is up");
    assert!(fe.results_enter(&mut progress, 0x0700_0005, 6, difficulty, lang.as_ref()),
        "the results fields build [M P_NFRESULTS 0x208e78:77]");
    // The award [M Menu_SetLevelBonus 0x1ffa10:62-66]: the completed row stamps and the NEXT row unlocks.
    assert_eq!(progress.nightfire_status() & (1 << 2), 1 << 2, "Alpine Escape unlocks through The Exchange");
    assert!(fe.unlocked_row(2), "the list offers the next mission");
    // fe-audit: 0x5e opens the stats [M 0x208e78:225-226 → 0x40000037]; the port's 0x5e key here is
    // Extra [S unverified — U-011]; Side is 0x5d, the Dossier [M :222-224]. △ comes back.
    assert!(act(&mut fe, Action::Extra), "0x5e opens the stats");
    assert_eq!(fe.page, Page::Stats);
    assert!(act(&mut fe, Action::Back), "△ returns to the results");
    assert_eq!(fe.page, Page::Results);
    // Confirm routes [M 0x208e78:39-52]: a fresh profile has no saved codename → no save box;
    // a new reward hops to the 007 Bonus page 0x40000038, otherwise straight to the list.
    assert!(act(&mut fe, Action::Confirm));
    if fe.new_reward {
        assert_eq!(fe.page, Page::NfBonus, "the reward hops to the bonus page [M 0x208e78:48-53]");
        assert_eq!(fe.flow.page, 0x4000_0038, "NfBonus is page 0x40000038 [M 0x2029e0:463-465]");
        assert!(act(&mut fe, Action::Confirm), "the bonus page confirms [M P_NFBONUS 0x209670:47-57]");
    }
    assert_eq!(fe.page, Page::MissionSelect, "the mission list is up [M 0x208e78:43]");
    // The list offers the freshly unlocked row and starting it rides the request [M C_SBNFMAP 0x20b8d8].
    assert!(fe.unlocked_row(2));
    // Navigate to The Exchange's row by the observable cursor only (the list opens on the highest
    // unlocked row [M P_NFMAP 0x20b608:57-64]; Down wraps).
    for _ in 0..12 { if fe.cursor == 1 { break; } assert!(act(&mut fe, Action::Down)); }
    assert_eq!(fe.cursor, 1, "The Exchange's row");
    assert!(act(&mut fe, Action::Confirm), "The Exchange starts");
    assert_eq!(fe.requested_level, Some(0x0700_0005), "the mission start rides the request");
    // A repeated press cannot double-request [M C_SBNFMAP 0x20b8d8:36-38 MenuManager_Delete;
    // the port's MissionSelect guard].
    assert!(!fe.action(Action::Confirm), "a second Confirm is refused while a request stands");
    assert_eq!(fe.requested_level, Some(0x0700_0005), "still one request");
}

/// ── 8. The 5 s loading hold and the double-press guards ────────────────────────────────────────
#[test]
fn the_loading_hold_window_and_the_double_press_guards() {
    // The port's own loading-screen minimum [S — a PC-port choice; the game's cold-entry minimum
    // is its loading clip [M 0x1d6360:101-113], restarts have none [M 0x1d6360:39,184], and the
    // whole load state 0x1d6360:121-171 carries no timer or frame minimum — the tail pops at
    // once (:166-170)].
    assert_eq!(nf_view::LOADING_HOLD, 5.0, "the 5 s hold");
    assert_eq!((nf_view::LOADING_HOLD * 60.0) as u32, 300, "300 ticks at the fixed 60 Hz");
    // While the hold runs, the window ends the frame before anything ticks (lib.rs:2998 → the
    // poll_async_load gate), keys return before the menu (lib.rs:2790) and the mouse arms are
    // gated on async_load.is_none() (lib.rs:2729/2737/2750) — input during the load is ignored.
    // The front-end half of that guard: the host takes a request exactly once (lib.rs:1822), so a
    // repeated press cannot double-fire a load — here on the fail page (which itself requests
    // nothing [M 0x182c60:77-80]; its row does [M 0x216618:19-21]):
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![menu::ST_RUNNER];
    fe.flow.current_level = 0x0700_0005;
    fe.mission_ended(Outcome::Failed, Stats::default());
    assert_eq!(fe.requested_level, None, "the fail menu opens with no request");
    assert!(fe.action(Action::Confirm), "the row answers");
    assert_eq!(fe.requested_level.take(), Some(0x0700_0005), "the host takes the row's request");
    assert_eq!(fe.requested_level.take(), None, "a second take gets nothing — one request, one load");
}
