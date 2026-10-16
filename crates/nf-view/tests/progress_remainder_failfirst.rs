//! fix-progress-remainder FAIL-FIRST probes — run against the UNFIXED source to record the
//! symptoms (the driver's "show it failing first"). Master-safe API only (no new fields):
//! 1. a guard spotting Bond never increments `PlrMissionStats+0x00` (the vision-log call site is
//!    unhooked) — the `detected >= 1` assert fails on master;
//! 2. the Q-Claw's auto-aim stops at `Autoaim_Range` 25 (its weapon_data+0x1c cap 50 is not
//!    seeded) — the snap-at-30 assert fails on master;
//! 3. `PlrStat_GetScore` on a 0x09 map returns None (the port models the NULL `NewScoresRef`,
//!    whose bootup-set zeroed buffer 0x31ca38 makes the game's abort dead code) — the `expect`
//!    fails on master.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

fn yaw_to(from: [f32; 3], at: [f32; 3]) -> f32 {
    let (dx, dz) = (at[0] - from[0], at[2] - from[2]);
    dx.atan2(dz)
}

#[test]
fn fail1_a_guard_spotting_bond_increments_detected() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The direct load's difficulty 1 gate [M 0x136be0:49] holds back 07000006's param2=2
    // placements, so the idle gate guard (placement 487, the pre-gate list[15]) is list[13].
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let d15_idx = 13;
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 3000);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [33.5, 39.0, 22.5], 0.0);
    let mut saw = false;
    for _ in 0..600 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.list[d15_idx].los { saw = true; }
    }
    assert!(saw, "precondition: the idle gate guard saw Bond");
    for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.progress.stats[0].detected >= 1,
        "a guard spotting Bond increments detected [M 0x1780a8:36 → 0x199960]");
}

#[test]
fn fail2_the_q_claw_auto_aim_snaps_at_30_units() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 3000);
    let guard = sim.drones.list.iter().position(|d| d.present && d.enabled && d.team == 1
        && !d.corpse && d.health > 0.0).expect("a live guard");
    let g = sim.drones.list[guard].pos;
    let mut spot = None;
    for a in 0..16 {
        let ang = a as f32 * std::f32::consts::FRAC_PI_8;
        let p = [g[0] + 30.0 * ang.sin(), g[1], g[2] + 30.0 * ang.cos()];
        let b = nf_game::player::Bond::stand_at(&sim.world, p, 0.0);
        if (b.pos[1] - g[1]).abs() > 3.0 { continue; }
        let head = [b.pos[0], b.pos[1] + 1.0, b.pos[2]];
        let target = [g[0], g[1] + 0.6, g[2]];
        if sim.world.line_of_sight(head, target) { spot = Some((b, p)); break; }
    }
    let Some((b, p)) = spot else { panic!("no open 30-unit spot") };
    sim.bond = b;
    sim.gun.weapon = 0x50; // the Q-Claw: weapon_data+0x1c = 50.0 [L ee_start_05]
    sim.auto_aim = true;
    let mut snapped = false;
    for _ in 0..60 {
        sim.bond.yaw = yaw_to(sim.bond.pos, sim.drones.list[guard].pos);
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.aim_target == Some(guard) { snapped = true; break; }
    }
    assert!(snapped, "the claw's auto-aim reaches 30 units (its +0x1c cap [M 0x1a7d64])");
}

#[test]
fn fail3_the_driving_map_scores_through_the_zeroed_new_scores_buffer() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 10);
    let scored = sim.drones.progress.get_score(0x0900_0001, 6, 2, true)
        .expect("GetScore on a 0x09 map reads the zeroed PTPDATA rows — no abort [M 0x1c7e78:86]");
    assert_eq!(scored.last, 0);
}
