//! fix-progress-detected-hook NF_ISO test — the 07000005 pin of the detected counter's live hook.
//!
//! G-045 names the level explicitly: a 07000005 guard spotting Bond raises the counter exactly
//! once per detection event as the game counts it. The game's rule [M]:
//! `DroneVision_HaveOpponentSight` 0x175ef0 (called from `DroneVision_ProcessDroneSight`
//! 0x176a28:216-231 on every LOS tick) passes `DroneFunc_ReactionTime` and calls
//! `DroneVision_LogSeenOpponent` 0x1780a8 — the opponent is the player, the `Drone+0x42` latch
//! is clear, team `Drone+0x44` != 2 (team 3 only on 0x07000009/0x0700000a, `sltiu` unsigned at
//! 0x178108 — 0x07000007's weapon gate does not exist here), then `+0x42 = 1` (0x178194, the
//! delay slot) and `PlrStat_LogEnemyDetectedPlayer(0)` 0x199960 → `PlrMissionStats[0] += 1`
//! behind the SP gate (`slot < 8`, non-MP `[0x2a4920] == 0`, `Mission_Status() ∈ {0,1}` — the
//! disasm 0x19996c-0x1999ac). fix-progress-remainder (110d) wired the hook and pinned
//! 07000006/07000007 (progress_remainder_live.rs); this file is the 07000005 pin.
//!
//! The latch has no clearer in ACTION.ELF [M whole-decompile grep: `+0x42` byte writes on the
//! drone — only 0x178194's `sb v0(+1), 0x42`], so each guard contributes exactly one increment
//! per detection event: the counter always equals the number of latched drones.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Yaw from `from` to `at` in the port's convention (0 faces +Z).
fn yaw_to(from: [f32; 3], at: [f32; 3]) -> f32 {
    let (dx, dz) = (at[0] - from[0], at[2] - from[2]);
    dx.atan2(dz)
}

/// The detection invariant: `PlrMissionStats+0x00` equals the number of drones whose `Drone+0x42`
/// latch stands [M 0x1780a8: the latch is the only thing gating the log per drone, and 0x199960
/// increments unconditionally behind the SP gate]. Each new detection event adds exactly one.
fn latched(sim: &nf_view::scene::Sim) -> usize {
    sim.drones.list.iter().filter(|d| d.detected_logged).count()
}

/// Find a live team-1 guard and a stand spot `r` units away (compass walk) with LOS to Bond.
/// Returns (guard index, spot). Modeled on progress_remainder_live's party search.
fn find_guard_and_spot(sim: &mut nf_view::scene::Sim, r: f32) -> Option<(usize, [f32; 3])> {
    for gi in 0..sim.drones.list.len() {
        let d = sim.drones.list[gi].clone();
        if !d.present || !d.enabled || d.team != 1 || !d.vision_on || d.corpse { continue; }
        let gpos = d.pos;
        for a in 0..16 {
            let ang = a as f32 * std::f32::consts::FRAC_PI_8;
            let p = [gpos[0] + r * ang.sin(), gpos[1], gpos[2] + r * ang.cos()];
            let b = nf_game::player::Bond::stand_at(&sim.world, p, yaw_to(p, gpos));
            if (b.pos[1] - gpos[1]).abs() > 2.5 { continue; }
            sim.bond = b;
            for _ in 0..15 { sim.step_bond(Input::default(), false, false, false); }
            if sim.drones.list[gi].los { return Some((gi, sim.bond.pos)); }
            // Restore the warm state before the next candidate: back off to the far corner so the
            // next spot's 15 settle ticks start from a fresh sight window.
            sim.bond = nf_game::player::Bond::stand_at(&sim.world,
                [gpos[0] + 60.0 * ang.sin(), gpos[1], gpos[2] + 60.0 * ang.cos()], yaw_to(p, gpos));
            for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
        }
    }
    None
}

/// ── A 07000005 guard spotting Bond raises the counter exactly once per detection event ─────────
#[test]
fn a_07000005_guard_spotting_bond_counts_one_detection() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    // The warm: the guards' vision comes up with the IDLE enter (the 3000-tick convention of
    // fix_npcs_live / progress_remainder_live).
    tick_idle(sim, 3000);
    assert_eq!(sim.drones.progress.stats[0].detected, 0,
        "no detections before any guard sees Bond");
    assert_eq!(latched(sim), 0, "no latches before any guard sees Bond");

    // A live guard and an open stand spot 4 units out.
    let Some((guard, spot)) = find_guard_and_spot(sim, 4.0) else {
        panic!("no team-1 guard on 07000005 with an open 4-unit stand spot");
    };
    eprintln!("[05] guard {guard} at {:?}, spot {:?}", sim.drones.list[guard].pos, spot);
    assert!(sim.drones.list[guard].los, "precondition: the guard has LOS to Bond");

    // Past the reaction time the HaveOpponentSight ack fires LogSeenOpponent: exactly one count.
    let mut saw_count = None;
    for _ in 0..400 {
        sim.step_bond(Input::default(), false, false, false);
        let det = sim.drones.progress.stats[0].detected;
        if det >= 1 { saw_count = Some(det); break; }
    }
    let after_first = saw_count.expect("a guard spotting Bond increments detected [M 0x1780a8:36 → 0x199960]");
    assert!(sim.drones.list[guard].detected_logged, "the spotting guard latched [M 0x1780a8:35]");
    assert_eq!(after_first as usize, latched(sim),
        "the counter equals the number of latched drones (one log per drone [M 0x1780a8:10,35])");

    // The +0x42 latch is permanent: 600 more exposed ticks add nothing for the drones already
    // latched — any growth must come from a NEW guard's first sight, one each (the invariant).
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.drones.progress.stats[0].detected as usize, latched(sim),
        "the invariant holds after long exposure: exactly one count per detection event");
    assert!(sim.drones.list[guard].detected_logged, "the first guard's latch holds");
    assert!(sim.drones.progress.stats[0].detected >= after_first,
        "the counter never decreases (no +0x42 clearer [M whole-decompile grep])");

    // Sister: leave and come back — the SAME guard's re-detection must not recount.
    let g = sim.drones.list[guard].pos;
    let back = [g[0] + 40.0, g[1], g[2] + 40.0];
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, back, yaw_to(back, g));
    for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); } // break the sight
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, spot, yaw_to(spot, g));
    for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); } // re-spot
    let grew = (sim.drones.progress.stats[0].detected - after_first) as usize;
    let new_latches = latched(sim) - after_first as usize;
    eprintln!("[05] re-exposure: counter +{grew}, new latches {new_latches}");
    assert_eq!(grew, new_latches,
        "re-detection by a latched guard never recounts; only new guards' first sights add [M 0x1780a8:10]");
    assert_eq!(sim.drones.progress.stats[0].detected as usize, latched(sim),
        "the invariant holds after the leave-and-return");
}
