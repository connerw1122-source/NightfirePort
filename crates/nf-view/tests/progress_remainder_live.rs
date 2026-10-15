//! fix-progress-remainder NF_ISO tests, driven through the REAL path (`nf_view::load` +
//! `Sim::step_bond`).
//!
//! The three G-045 remainder items (docs/systems/progress.md):
//! 1. the "detected" counter's live hook — `DroneVision_LogSeenOpponent` 0x1780a8 (called from
//!    `DroneVision_HaveOpponentSight` 0x175ef0 on every LOS tick past the reaction time) sets
//!    its `Drone+0x42` latch and calls `PlrStat_LogEnemyDetectedPlayer(0)` 0x199960;
//! 2. the auto-aim's distance cap — `Player_AutoAim` 0x1a7c98 seeds the running best with
//!    `Autoaim_Range` (25.0 [L]) except the Q-Claw (0x50/0x51), whose cap is its own weapon_data
//!    record's +0x1c (50.0 / 100.0 [L ee_start_05]); `Check_AutoAim` 0x1a73f8 takes a candidate
//!    only closer than that. (`Player_InitWeapon` 0x1a7ef8's MAX_AUTO_AIM_DIST_NEAR/MIDL are
//!    write-only in ACTION.ELF — see the report; the port does not thread them.)
//! 3. the `NewScoresRef` hand-off — bootup 0x1c7e78:86 points 0x30cc6c at the zeroed PTPDATA
//!    tail 0x31ca38, so `PlrStat_GetScore` 0x1984b0's 0x09-map branch reads zero rows (no NULL
//!    abort in the shipped game); the copy multiplies row 7's value by 100 [M 0x1984b0:52-54].

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

/// The detection invariant: `PlrMissionStats+0x00` always equals the number of drones whose
/// `Drone+0x42` latch stands (the latch is never cleared — ACTION.ELF has no writer [M grep] —
/// and each drone's first qualifying sight tick adds exactly one).
fn latched(sim: &nf_view::scene::Sim) -> usize {
    sim.drones.list.iter().filter(|d| d.detected_logged).count()
}

/// ── 1. A guard spotting Bond increments `detected` exactly once per guard ───────────────────────
/// The rule [M DroneVision_LogSeenOpponent 0x1780a8:10-36]: the opponent is the player, the +0x42
/// latch is clear, team +0x44 != 2 (team 3 only on 0x07000009/0x0700000A), then the latch sets and
/// `PlrStat_LogEnemyDetectedPlayer(0)` runs behind the SP gate [M 0x199960]. On 07000006 the d15
/// gate guard sees Bond from the spawn-floor spot (fix_npcs_live's idle-guard sister).
#[test]
fn a_guard_spotting_bond_counts_one_detection_per_guard() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    // The warm: fix_npcs_live's sister runs 3000 ticks before placing Bond — the guards' vision
    // comes up with the IDLE enter (0x15bf18:30), not at tick 0.
    tick_idle(sim, 3000);
    assert_eq!(sim.drones.progress.stats[0].detected, 0, "no detections before Bond is seen");
    // The idle gate guard d15's spot (fix_npcs_live): an enabled IDLE guard on the spawn floor.
    assert!(sim.drones.list[15].present && sim.drones.list[15].enabled
        && sim.drones.list[15].team == 1, "d15 is an enabled team-1 guard");
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [33.5, 39.0, 22.5], 0.0);
    let mut saw = false;
    for _ in 0..600 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.list[15].los { saw = true; }
    }
    assert!(saw, "d15 has LOS to Bond (the fix_npcs_live sister saw him within 600 ticks)");
    // Past the reaction time the HaveOpponentSight ack fires LogSeenOpponent: exactly one count.
    for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    let after_first = sim.drones.progress.stats[0].detected;
    assert!(after_first >= 1, "a guard spotting Bond increments detected [M 0x1780a8:36]");
    assert_eq!(after_first as usize, latched(sim),
        "detected == the number of latched drones (one log per drone [M 0x1780a8:10,35])");
    // The latch holds: 600 more exposed ticks add nothing new for the drones already latched.
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let grew = sim.drones.progress.stats[0].detected - after_first;
    assert!((grew as usize) <= latched(sim) - after_first as usize,
        "only first-time detections add (the +0x42 latch [M 0x1780a8:10]); grew {grew}, latched {}",
        latched(sim));
    assert_eq!(sim.drones.progress.stats[0].detected as usize, latched(sim),
        "the invariant holds after long exposure");
}

/// ── 2. The 0x07000007 weapon gate and the team-3 map gate ───────────────────────────────────────
/// LogSeenOpponent's two extra gates [M 0x1780a8:12-34]: on the party level a detection only
/// counts while Bond's current weapon (obj+0xdc → +0x62 — `Player_AutoAim` 0x1a7c98:0x1a7d04
/// reads the same byte) is outside the excluded band `>1 && >=0x47 && not in [0x6a..0x71]`; and a
/// team-3 drone (civilians/guests) only counts on 0x07000009/0x0700000A. The Q-Claw (0x50) is in
/// the excluded band, the bare hands (0) are not.
#[test]
fn the_party_weapon_and_team_gates_follow_the_decompile() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let sim = scene.sim.as_mut().unwrap();
    // The warm: the guards' vision comes up with the IDLE enter (fix_npcs_live's 3000-tick warm).
    tick_idle(sim, 3000);
    // A live team-1 guard on the party level (the security detail; the guests are team 3), plus a
    // stand spot with an open ray — the guards stand indoors, so walk the candidates × compass
    // until `los` holds. Bond reads as the hands class 0x47 (the excluded band) for the whole
    // search: no sight counts.
    let mut found = None;
    'guards: for gi in 0..sim.drones.list.len() {
        let d = &sim.drones.list[gi];
        if !d.present || !d.enabled || d.team != 1 || !d.vision_on || d.corpse { continue; }
        let gpos = d.pos;
        for a in 0..8 {
            let ang = a as f32 * std::f32::consts::FRAC_PI_4;
            let p = [gpos[0] + 3.0 * ang.sin(), gpos[1], gpos[2] + 3.0 * ang.cos()];
            let b = nf_game::player::Bond::stand_at(&sim.world, p, yaw_to(p, gpos));
            if (b.pos[1] - gpos[1]).abs() > 2.5 { continue; }
            sim.bond = b;
            for _ in 0..15 {
                // The per-tick poke: the run's class byte (Drones.bond_weapon, scene.rs from
                // gun.weapon/switch_request) re-derives every tick, and a weapon alone re-queues
                // the loadout's pending request — both fields go to the hands class 0x47
                // (InitWeapon 0x1a7ef8:137-139's +0x64; the excluded band, b >= 0x47).
                sim.gun.weapon = 0x47;
                sim.gun.switch_request = 0x47;
                sim.step_bond(Input::default(), false, false, false);
            }
            if sim.drones.list[gi].los { found = Some(gi); break 'guards; }
        }
    }
    let Some(guard) = found else { panic!("no team-1 guard on 07000007 with an open stand spot") };
    let gpos = sim.drones.list[guard].pos;
    let mut saw = false;
    for _ in 0..400 {
        // The hands class 0x47 held (excluded band) — see the search loop's poke note.
        sim.gun.weapon = 0x47;
        sim.gun.switch_request = 0x47;
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.list[guard].los && sim.drones.list[guard].seen_ticks > 5 { saw = true; }
        // Hold the stand in case the guard's state machine moved Bond's floor reference.
        if !sim.drones.list[guard].los {
            sim.bond = nf_game::player::Bond::stand_at(&sim.world,
                [gpos[0] + 3.0, gpos[1], gpos[2]], yaw_to([gpos[0] + 3.0, gpos[1], gpos[2]], gpos));
        }
    }
    eprintln!("[party] guard {guard} los {} seen_ticks {} dist {:.1}",
        sim.drones.list[guard].los, sim.drones.list[guard].seen_ticks, sim.drones.list[guard].dist);
    assert!(saw, "the guard had Bond in sight (seen_ticks past the reaction window)");
    assert_eq!(sim.drones.progress.stats[0].detected, 0,
        "the hands class 0x47 (>= 0x47, not in 0x6a..0x71) is excluded on 0x07000007 [M 0x1780a8:25-26]");
    assert!(!sim.drones.list[guard].detected_logged, "no latch while the weapon is excluded");
    // An owned gun out (2, the P2K — class < 0x47): the same exposure now counts, once.
    let mut counted = false;
    for _ in 0..60 {
        sim.gun.weapon = 2;
        sim.gun.switch_request = 2;
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.progress.stats[0].detected >= 1 { counted = true; break; }
    }
    assert!(counted, "the same guard with an unexcluded weapon counts [M 0x1780a8:35-36]");
    // The team-3 gate: a party guest (team 3) seeing Bond on 07000007 adds nothing — team 3 only
    // counts on 0x07000009/0x0700000A [M 0x1780a8:12 — the `+0x44 < 3 || != 3 || map ∈ {9,A}`
    // chain]. The guests' first sight must not move the counter past the guards' counts.
    let guest = sim.drones.list.iter().position(|d| d.present && d.enabled && d.team == 3
        && d.vision_on && !d.corpse);
    if let Some(gi) = guest {
        let before = sim.drones.progress.stats[0].detected;
        let latch_before = latched(sim);
        let gpos = sim.drones.list[gi].pos;
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [gpos[0] + 1.5, gpos[1], gpos[2]],
            yaw_to([gpos[0] + 1.5, gpos[1], gpos[2]], gpos));
        for _ in 0..400 { sim.step_bond(Input::default(), false, false, false); }
        let grew = sim.drones.progress.stats[0].detected - before;
        let new_team3 = (latch_before..=latched(sim))
            .map(|_| sim.drones.list.iter().filter(|d| d.team == 3 && d.detected_logged).count())
            .min().unwrap_or(0);
        eprintln!("[team3] guest {gi}: detected +{grew}, team-3 latches now {new_team3}");
        assert_eq!(sim.drones.list[gi].detected_logged, false,
            "a team-3 guest's sight on 0x07000007 does not latch [M 0x1780a8:12]");
        assert_eq!(grew as usize, latched(sim) - latch_before,
            "the counter tracks the guards' latches only");
    } else {
        eprintln!("[team3] no live team-3 guest with vision on this load — the team-3 map gate is \
            covered by the guard test's invariant (guest sight would break it)");
    }
}

/// ── 3. The auto-aim's distance cap: the Q-Claw reaches 50, the guns 25 ──────────────────────────
/// [M Player_AutoAim 0x1a7c98 disasm 0x1a7d30-0x1a7d68]: the running best starts at
/// `Autoaim_Range` (0x30cd54, 25.0 [L]) — except weapon 0x50/0x51, whose seed is
/// `weapon_data[type]+0x1c` (50.0 / 100.0 [L ee_start_05]); `Check_AutoAim` 0x1a73f8 disasm
/// 0x1a75a4-0x1a75b0 rejects candidates at `*param_3 < dist`.
#[test]
fn the_q_claw_auto_aim_reaches_past_the_gun_range() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    // The warm (the guards' vision comes up with the IDLE enter — the 3000-tick convention).
    tick_idle(sim, 3000);
    // A live team-1 guard, and an open spot 30 units from it (inside the claw's 50, outside the
    // guns' 25). Walk the compass from the guard for a spot with open LOS.
    let guard = sim.drones.list.iter().position(|d| d.present && d.enabled && d.team == 1
        && !d.corpse && d.health > 0.0).expect("07000005 has a live guard");
    let g = sim.drones.list[guard].pos;
    let mut spot = None;
    for a in 0..16 {
        let ang = a as f32 * std::f32::consts::FRAC_PI_8;
        let p = [g[0] + 30.0 * ang.sin(), g[1], g[2] + 30.0 * ang.cos()];
        let b = nf_game::player::Bond::stand_at(&sim.world, p, 0.0);
        // stand_at snaps to the floor — require the snap to stay near the guard's height and the
        // LOS to hold (a spot behind a wall or on another floor is rejected).
        if (b.pos[1] - g[1]).abs() > 3.0 { continue; }
        let head = [b.pos[0], b.pos[1] + 1.0, b.pos[2]];
        let target = [g[0], g[1] + 0.6, g[2]];
        if sim.world.line_of_sight(head, target) { spot = Some((b, head, target)); break; }
    }
    let Some((b, head, target)) = spot else { panic!("no open 30-unit spot around the guard at {g:?}") };
    sim.bond = b;
    sim.bond.yaw = yaw_to(sim.bond.pos, g);
    tick_idle(sim, 2);
    assert!((30.0 - dist(head, target)).abs() < 4.0, "the spot is ~30 units from the guard");
    assert!(sim.world.line_of_sight(head, target), "open LOS from the spot to the guard");
    // The claw out and auto-aim on (the profile's default [M profile.rs auto_aim: true]; the Sim
    // defaults it off for the raw viewer — the options path turns it on, the test mirrors that).
    // The scan runs OUTSIDE aim mode [M 0x1a7c98's aim-bit branch] — no aiming input here.
    sim.gun.weapon = 0x50;
    sim.auto_aim = true;
    let mut snapped = false;
    for _ in 0..60 {
        sim.bond.yaw = yaw_to(sim.bond.pos, sim.drones.list[guard].pos);
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.aim_target == Some(guard) { snapped = true; break; }
    }
    assert!(snapped, "the Q-Claw's auto-aim snaps at 30 units (its weapon_data+0x1c cap 50 \
        [M 0x1a7d64; L ee_start_05])");
    // Sister: the same spot with the P2K (weapon 2, the start weapon) finds nothing — the guns'
    // cap is Autoaim_Range 25 [M 0x1a7d30].
    sim.gun.weapon = 2;
    sim.drones.aim_target = None;
    for _ in 0..60 {
        sim.bond.yaw = yaw_to(sim.bond.pos, sim.drones.list[guard].pos);
        sim.step_bond(Input::default(), false, false, false);
    }
    assert_ne!(sim.drones.aim_target, Some(guard),
        "the P2K's auto-aim does not reach 30 units (the Autoaim_Range 25 cap [M 0x1a73f8])");
}

/// ── 4. The NewScoresRef hand-off: the 0x09 maps read the zeroed PTPDATA rows ────────────────────
/// [M bootup 0x1c7e78:86 `NewScoresRef = &DAT_0031ca38`] — the NULL abort in `PlrStat_GetScore`
/// 0x1984b0:45 is dead code; the shipped game reads the zero rows (every count 0 → nothing scores,
/// the entry's last = 0). [L ee_start_05: the pointer is 0x31ca38, rows 0..8 all zero.]
#[test]
fn the_driving_maps_score_through_the_zeroed_new_scores_buffer() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 10);
    // The boot state: Some(zeroed 9×3) — the port's stand-in for bootup's set pointer.
    assert!(sim.drones.progress.new_scores.is_some(), "bootup always sets NewScoresRef [M 0x1c7e78:86]");
    assert!(sim.drones.progress.new_scores.as_ref().unwrap().iter().all(|r| r == &[0, 0, 0]),
        "the buffer is the zeroed PTPDATA tail [L ee_start_05: rows 0..8 zero]");
    // 0x09000001 (Driving_Paris) is a table entry with the +0x38 flag 0 → the NewScoresRef branch.
    // The game returns a zero score, not the NULL abort.
    let scored = sim.drones.progress.get_score(0x0900_0001, 6, 2, true)
        .expect("GetScore on a 0x09 map reads the zero rows — no abort [M 0x1984b0:45 dead]");
    assert_eq!(scored.last, 0, "zero rows → nothing scores [M 0x1984b0:63 skip]");
    assert_eq!(scored.base, 0);
    // The row-7 ×100 at the copy [M 0x1984b0:52-54]: the driving module's par seconds become the
    // 1/100-s unit the row carries. A hand-off-shaped buffer (the PTP rows after a driving leg)
    // lands in the entry's rows verbatim except row 7.
    sim.drones.progress.new_scores = Some([
        [7, 5, 1000], [0, 0, 0], [0, 0, 0], [0, 0, 0], [1, 0, 500], [0, 0, 0], [0, 0, 0],
        [90, 60, 1000], [0, 0, 0],
    ]);
    let _ = sim.drones.progress.get_score(0x0900_0001, 6, 2, true).unwrap();
    let idx = sim.drones.progress.table.iter().position(|e| e.hash == 0x0900_0001).unwrap();
    let rows = &sim.drones.progress.table[idx].rows;
    assert_eq!((rows[0].value, rows[0].count, rows[0].target), (7, 5, 1000),
        "the copy writes value/count/target = [0]/[1]/[2] [M 0x1984b0:56-61]");
    assert_eq!(rows[7].value, 9000, "row 7's value is ×100 at the copy [M 0x1984b0:52-54]");
    assert_eq!(rows[4].value, 1, "the other rows copy raw");
    // Row 7's ratio math reads the ×100'd value: t = 9000/100 = 90 s, count 60 → 60/90.
    assert!((rows[7].ratio - (60.0f32 / 90.0)).abs() < 1e-4,
        "the time ratio runs on the ×100'd value [M 0x1984b0 row 7 + the copy]");
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}
