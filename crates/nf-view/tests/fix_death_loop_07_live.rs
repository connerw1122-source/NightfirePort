//! death-loop-07 — dying on 07000007 must not loop the reload (playtest 1012-1723: "once I
//! died, we went into a replay loop where it kept replaying at the start of the mission over
//! and over again. I had to quickly close out").
//!
//! The fail path, end to end [M]:
//! - the death (health zero, `Player_CheckForDeath` 0x190810) or the guest-shot fail (a punch
//!   on an unarmed party guest → `DroneFunc_SetMissionFailReason` 5 → ch 99 [M 0x145f28:45-56 →
//!   0x13e560]) → MissionState 2 → the 300-tick fail banner [M 0x182c60:52-57] → the 0x40000042
//!   menu [M 0x182c60:77-81];
//! - the menu's REPLAY row 0 = `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` AND
//!   `Player_RamLoad(Bond, 1)` [M P_ENDMISSION_Handler 0x216618:19-21] — the CURRENT level,
//!   WARM, served off the ENTRY checkpoint (`Player_RamLoad` 0x1957a8 restores the buffer's
//!   channel arrays; the entry buffer predates the fail, so no fail channel rides);
//! - the port answers the row two ways: the sim's auto-restart (`fail_choose_restart`) and the
//!   front end's Results REPLAY row (the fix: the row carries `requested_restart`, the host
//!   marks `level_exit_restart` — pre-fix the host served the DYING sim's live capture, whose
//!   carried fail channel re-failed the mission the tick it came back: the replay loop).
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::{Input, RamSave};
use nf_view::menu::{Action, FrontEnd, Outcome, Page, Stats, ST_RUNNER};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn tick(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Input::default(), false, false, false); }
}

/// One tick with the fire button held (action 25 — the ✕ death skip [M 0x190bb0:30]).
fn tick_fire(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Input::default(), true, false, false); }
}

/// The first ENABLED party guest (dtype 10), nearest to Bond (fix-guest-reactions' picker; the
/// idle state-1 guests sit disabled until the party lets the walkers out).
fn pick_guest(sim: &nf_view::scene::Sim) -> (usize, [f32; 3]) {
    let bp = [sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2]];
    let mut best: Option<(usize, f32)> = None;
    for (i, d) in sim.drones.list.iter().enumerate() {
        if !d.present || d.dtype != 10 || d.corpse || !d.enabled { continue; }
        let d2 = (d.pos[0]-bp[0]).powi(2) + (d.pos[2]-bp[2]).powi(2);
        if best.map_or(true, |(_, bd)| d2 < bd) { best = Some((i, d2)); }
    }
    let i = best.expect("an enabled party guest exists").0;
    let pos = sim.drones.list[i].pos;
    (i, pos)
}

/// Stand Bond beside `guest` facing its chest (the fire ray leaves the eye along the look).
fn aim_at(sim: &mut nf_view::scene::Sim, guest: usize) {
    let g = sim.drones.list[guest].pos;
    let eye = [sim.bond.pos[0], sim.bond.pos[1] + sim.bond.eye, sim.bond.pos[2]];
    sim.bond.yaw = (g[0] - eye[0]).atan2(g[2] - eye[2]);
    let horiz = ((g[0] - eye[0]).powi(2) + (g[2] - eye[2]).powi(2)).sqrt();
    sim.bond.pitch = ((g[1] + 0.6 - eye[1]) / horiz).atan() / (std::f32::consts::FRAC_PI_2);
}

/// Walk Bond up to a guest and punch it until it drops (the round spawns at the fire script's
/// strike frame +0x44, so the swing holds the trigger and waits, re-aiming every tick).
fn punch_once(sim: &mut nf_view::scene::Sim, guest: usize) {
    let g = sim.drones.list[guest].pos;
    sim.bond.pos = [g[0] - 0.9, g[1], g[2] + 0.9]; // ~1.27 away, inside the fists' 1.75 range
    sim.gun.weapon = 1; sim.gun.switch_request = 1; sim.gun.switch_state = 0;
    for _ in 0..40 {
        aim_at(sim, guest);
        sim.step_bond(Input::default(), true, false, false);
        for _ in 0..24 { aim_at(sim, guest); sim.step_bond(Input::default(), false, false, false); }
        let g = &sim.drones.list[guest];
        if g.health < 10.0 || g.knocked_out || g.corpse { return; }
    }
}

/// The clean-reload assertions the two cycle tests share: 600 ticks after the reload the
/// mission is still playing, nobody died, no fail channel rides, nothing re-queued.
fn assert_playable_600(sim: &mut nf_view::scene::Sim, cycle: u32) {
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.health.health > 0.0 && !sim.drones.death.active,
        "cycle {cycle}: alive after the reload's 600 ticks — no replay loop");
    assert_eq!(sim.drones.mission.status(), 1,
        "cycle {cycle}: the mission plays on after the reload");
    assert!(!sim.drones.channels.on(99) && !sim.drones.channels.on(0x60) && !sim.drones.channels.on(0x62),
        "cycle {cycle}: no carried fail channel — the entry checkpoint predates the fail [M 0x1957a8:17-20]");
    assert!(sim.drones.level_exit.is_none() && !sim.drones.fail_menu_open(),
        "cycle {cycle}: no restart queued, no menu up — the loop is gone");
}

/// ── Two health-zero deaths, each answered by the sim's auto-restart ────────────────────────────
/// Each cycle: die (1000-damage pain [M 0x190810]), ✕ past the red fade (ch 0x62 [M 0x190bb0:32]),
/// the 0x40000042 menu opens and the port's auto-restart answers it [M 0x216618:19-21], the
/// viewer reloads the CURRENT level warm off the ENTRY checkpoint (`fail_restart_save` — the
/// serve `start_async_load`'s restart arm builds, lib.rs) — and 07 must be playable for 600
/// ticks. Twice.
#[test]
fn two_health_zero_deaths_each_restart_into_a_playable_07() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    // The host's entry checkpoint, captured at the swap before any play (lib.rs swap-in hook).
    let mut entry = { let sim = scene.sim.as_ref().unwrap(); nf_view::entry_checkpoint_save(sim) };
    tick(scene.sim.as_mut().unwrap(), 600); // the party settles; the enabled walkers come out
    for cycle in 1..=2 {
        let served = {
            let sim = scene.sim.as_mut().unwrap();
            assert!(sim.drones.health.health > 0.0 && !sim.drones.death.active, "cycle {cycle}: alive before the death");
            assert_eq!(sim.drones.mission.status(), 1, "cycle {cycle}: the mission is playing");
            // The kill: 1000-damage pain [M Player_CheckForDeath 0x190810].
            let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng);
            h.pain_generic(r, 1000.0, 1, 0x0700_0007, 0);
            tick(sim, 1); // the death starts on the tick after the pain (the start tick returns early)
            assert!(sim.drones.death.active && sim.drones.death.pane_on,
                "cycle {cycle}: the death sequence started [M 0x190810, 0x190bb0:23-25]");
            tick_fire(sim, 1); // ✕ raises ch 0x62 on the next dead tick [M Player_HandleDeath 0x190bb0:32]
            assert!(sim.drones.channels.on(0x62), "cycle {cycle}: the ✕ skip raised the death channel");
            let mut answered = false;
            for _ in 0..30 {
                tick(sim, 1);
                if sim.drones.level_exit.is_some() { answered = true; break; }
            }
            assert!(answered, "cycle {cycle}: the auto-restart answered the 0x40000042 menu [M 0x182c60:77-81]");
            assert_eq!(sim.drones.level_exit, Some(0x0700_0007), "the CURRENT level [M 0x216618:20 GameState._12_4_]");
            assert!(!sim.drones.level_exit_cold, "the warm arm [M 0x216618:20]");
            assert!(sim.drones.level_exit_restart, "on the entry checkpoint [M 0x216618:21 Player_RamLoad]");
            // The viewer's restart serve (start_async_load's restart arm): the ENTRY checkpoint.
            nf_view::fail_restart_save(Some(&entry), sim)
        };
        scene = nf_view::load_with_carry(&iso, "07000007.bin", false, Some(&served)).expect("the warm reload");
        entry = { let sim = scene.sim.as_ref().unwrap(); nf_view::entry_checkpoint_save(sim) };
        assert_playable_600(scene.sim.as_mut().unwrap(), cycle);
    }
}

/// ── Two guest-shot fails (the real punch path), each answered by the sim's auto-restart ────────
/// Punch an enabled party guest [M NDrone2_PunchImpact 0x145f28:45-56 → SetMissionFailReason 5 →
/// ch 99 0x13e560] → MissionState 2 → the 300-tick banner [M 0x182c60:52-57] → the menu → the
/// auto-restart → the ENTRY-checkpoint reload — twice, 600 playable ticks each.
#[test]
fn two_guest_shot_fails_each_restart_into_a_playable_07() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let mut entry = { let sim = scene.sim.as_ref().unwrap(); nf_view::entry_checkpoint_save(sim) };
    tick(scene.sim.as_mut().unwrap(), 600); // the party settles; the punchable walkers come out
    for cycle in 1..=2 {
        let served = {
            let sim = scene.sim.as_mut().unwrap();
            assert_eq!(sim.drones.mission.status(), 1, "cycle {cycle}: the mission is playing");
            let (guest, _) = pick_guest(sim);
            punch_once(sim, guest);
            assert!(sim.drones.channels.on(99),
                "cycle {cycle}: the guest-shot fail raised ch 99 [M 0x145f28:45-56 → 0x13e560]");
            assert_eq!(sim.drones.mission.status(), 2, "cycle {cycle}: MissionState 2 (failed) [M 0x182c60:47]");
            let mut answered = false;
            for _ in 0..340 { // the fail banner's 300 ticks before the menu [M 0x182c60:52-57]
                tick(sim, 1);
                if sim.drones.level_exit.is_some() { answered = true; break; }
            }
            assert!(answered, "cycle {cycle}: the auto-restart answered the fail menu");
            assert_eq!(sim.drones.level_exit, Some(0x0700_0007), "the CURRENT level [M 0x216618:20]");
            assert!(!sim.drones.level_exit_cold && sim.drones.level_exit_restart,
                "warm, on the entry checkpoint [M 0x216618:19-21]");
            nf_view::fail_restart_save(Some(&entry), sim)
        };
        scene = nf_view::load_with_carry(&iso, "07000007.bin", false, Some(&served)).expect("the warm reload");
        entry = { let sim = scene.sim.as_ref().unwrap(); nf_view::entry_checkpoint_save(sim) };
        assert_playable_600(scene.sim.as_mut().unwrap(), cycle);
    }
}

/// ── The fail menu's REPLAY row through the FRONT END serves the entry checkpoint ────────────────
/// The row 0 answer is `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` AND `Player_RamLoad(Bond,
/// 1)` [M 0x216618:19-21]. The front end's Results REPLAY row now carries the RamLoad half
/// (`requested_restart`) and the host marks `level_exit_restart` — the ENTRY-checkpoint serve.
/// Pre-fix the host fell into the live-capture arm and served the DYING sim: its carried fail
/// channel re-failed the mission the tick it came back — the replay loop, reproduced as the
/// sister.
#[test]
fn the_fail_menu_replay_row_serves_the_entry_checkpoint_not_the_dying_state() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let entry = { let sim = scene.sim.as_ref().unwrap(); nf_view::entry_checkpoint_save(sim) };
    let dying_capture = {
        let sim = scene.sim.as_mut().unwrap();
        tick(sim, 600);
        sim.drones.auto_restart = false; // the view answers this one through the fail menu
        let (guest, _) = pick_guest(sim);
        punch_once(sim, guest);
        assert!(sim.drones.channels.on(99) && sim.drones.mission.status() == 2, "the guest-shot fail ran");
        let mut menus = false;
        for _ in 0..340 {
            tick(sim, 1);
            if sim.drones.fail_menu_open() { menus = true; break; }
        }
        assert!(menus, "the 0x40000042 menu is up and waiting [M 0x182c60:77-81]");
        // What the pre-fix host served for a front-end REPLAY (start_async_load's else arm): the
        // dying sim's live capture — ch 99 (and 0x60) ride it.
        RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
            .with_mission(&sim.drones.mission)
    };
    // The front end mirrors the menu (the host's results hook) and the player answers REPLAY.
    let mut fe = FrontEnd::new();
    fe.flow.stack = vec![ST_RUNNER];
    fe.flow.current_level = 0x0700_0007;
    fe.mission = nf_view::menu::CAMPAIGN.iter().position(|c| c.3 == 0x0700_0005).unwrap(); // 07's mission row
    fe.mission_ended(Outcome::Failed, Stats::default());
    assert_eq!(fe.page, Page::Results);
    assert_eq!(fe.requested_level.take(), Some(nf_view::menu::FRONTEND_LEVEL), "the end's own load first");
    assert!(fe.action(Action::Confirm), "REPLAY (row 0)");
    assert_eq!(fe.requested_level, Some(0x0700_0007), "REPLAY → the mission's level [M 0x216618:20]");
    assert!(fe.flow.warm_reset, "the warm arm [M 0x216618:20]");
    assert!(fe.requested_restart, "the row carries the Player_RamLoad half [M 0x216618:21]");
    // The host wiring (lib.rs, the requested_level take): the marker marks the restart serve,
    // exactly as the sim's own auto answer does.
    scene.sim.as_mut().unwrap().drones.level_exit_restart = true;
    let served = nf_view::fail_restart_save(Some(&entry), scene.sim.as_ref().unwrap());
    let mut fixed = nf_view::load_with_carry(&iso, "07000007.bin", false, Some(&served)).expect("the fixed reload");
    assert_playable_600(fixed.sim.as_mut().unwrap(), 1);
    // Sister: the pre-fix serve — the dying capture. The carried ch 99 re-fails the mission the
    // tick it comes back (no input), and the auto-restart answers again: the loop turns.
    let mut looping = nf_view::load_with_carry(&iso, "07000007.bin", false, Some(&dying_capture)).expect("the carried reload");
    {
        let sim = looping.sim.as_mut().unwrap();
        tick(sim, 8);
        assert!(sim.drones.channels.on(99) && sim.drones.mission.status() == 2,
            "the carried fail channel re-fails the mission with no input — the replay loop");
        let mut re_queued = false;
        for _ in 0..340 {
            tick(sim, 1);
            if sim.drones.level_exit.is_some() { re_queued = true; break; }
        }
        assert!(re_queued, "the auto-restart answers again — the loop turns");
    }
}
