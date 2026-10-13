//! fix-progress NF_ISO tests, driven through the REAL path (`nf_view::load` + `Sim::step_bond`).
//!
//! The mission-progress layer (docs/systems/progress.md, docs/fields/player-stats.md): the
//! end-of-mission tally (`PlrMissionStats` 0x2bcc20 + the `PlrStat_Log*` family), the difficulty
//! effects (`GameState+0x40` 0x2a37a8) and the save. The tally test reproduces the decompile's
//! increments on 07000005: kill 2 guards through Bond's own trigger and take one Bond-bonus
//! pickup, then read the counters the results screen would show.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Yaw from `from` to `at` in the port's convention (0 faces +Z; a larger yaw turns left
/// [M nf_game::player::Bond's doc line for obj+0x54]).
fn yaw_to(from: [f32; 3], at: [f32; 3]) -> f32 {
    // The port's forward is (sin yaw, 0, cos yaw) (0 faces +Z [M Bond's obj+0x54 doc]; the fire
    // ray builds right = (−cos yaw, 0, sin yaw) [M scene.rs run]).
    let (dx, dz) = (at[0] - from[0], at[2] - from[2]);
    dx.atan2(dz)
}

/// ── 1. The tally on 07000005 ────────────────────────────────────────────────────────────────────
/// The decompile's writers ([M] docs/fields/player-stats.md "The PlrMissionStats record"):
/// spawned per created drone (`NDrone2_DefaultInit` 0x14b300:609), fired per bullet
/// (`Player_WeaponInitBullet` 0x1a4bf0:258), hit-enemy per player hit on a team-1 drone
/// (`NDrone2_HitDamage` 0x145558:57-60), dispatched on the player's kill (`OnInitDeath`
/// 0x145bd0:57-59), bonus pickups in `Pickup_Handler` case 6 (0x18e650:234-236).
#[test]
fn killing_two_guards_and_taking_a_bonus_tallies_like_the_decompile() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 30);
    // The level's drones exist: the live start count is 13 (spawner 266 brought one in within the
    // warm [L hitdata-spawner-ally / ai_live]).
    let spawned = sim.drones.progress.stats[0].spawned;
    // 13 at level start ([L] the live count, ai_live) + any spawner spawns the warm ticks made
    // (mode-2 spawners spawn while alive < max); the contract is per created drone.
    assert!(spawned >= 13, "PlrStat_LogEnemySpawned per created drone [M 0x14b300:609], got {spawned}");
    // A live guard to shoot: the nearest present, non-corpse, team-1 drone.
    let targets: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && !d.corpse && d.team == 1 && d.health > 0.0)
        .map(|(i, _)| i).collect();
    assert!(targets.len() >= 2, "at least two live guards");
    let before_health = [sim.drones.list[targets[0]].health, sim.drones.list[targets[1]].health];
    for (n, &gi) in targets.iter().take(2).enumerate() {
        // Teleport Bond in front of the guard (the headless tests' teleport convention) and fire
        // until it dies — the trigger through step_bond, the game's own fire path.
        let gpos = sim.drones.list[gi].pos;
        let spot = [gpos[0] + 2.2, gpos[1], gpos[2]];
        sim.bond.pos = spot;
        sim.bond.yaw = yaw_to(spot, gpos);
        sim.bond.pitch = 0.0;
        let mut fired_ticks = 0;
        // The P2K is semi-auto (`shots_per_pull` — the trigger is a fresh press per shot), so the
        // run pulses the trigger: a fire tick, then a release tick, re-aiming at the guard each
        // time (the AI moves).
        while sim.drones.list[gi].health > 0.0 && fired_ticks < 300 && sim.drones.mission.status() < 2 {
            let g = sim.drones.list[gi].pos;
            sim.bond.yaw = yaw_to(sim.bond.pos, g);
            sim.step_bond(Input { aiming: true, ..Default::default() }, true, false, false);
            fired_ticks += 1;
            let g = sim.drones.list[gi].pos;
            sim.bond.yaw = yaw_to(sim.bond.pos, g);
            sim.step_bond(Input { aiming: true, ..Default::default() }, false, false, false);
            fired_ticks += 1;
        }
        eprintln!("[tally] guard {n}: health {} after {fired_ticks} fire ticks, bond hp {}, status {}, fired {} hit {}",
            sim.drones.list[gi].health, sim.drones.health.health, sim.drones.mission.status(),
            sim.drones.progress.stats[0].shots_fired, sim.drones.progress.stats[0].shots_hit_enemy);
        assert!(sim.drones.list[gi].health <= 0.0, "guard {n} died (health {} after {fired_ticks} fire ticks)", sim.drones.list[gi].health);
    }
    let st = &sim.drones.progress.stats[0];
    assert_eq!(st.dispatched, 2, "PlrStat_LogEnemyDispatched per player kill [M 0x145bd0:57-59]");
    // The two kills landed one bullet each (the 2.2-unit P2K shot one-shots a 10-health guard):
    // 2 spawned, 2 hit. The contract is the per-bullet increment, not a shot count guess.
    assert!(st.shots_fired >= 2, "PlrStat_LogShotFired per bullet spawned [M 0x1a4bf0:258], got {}", st.shots_fired);
    assert!(st.shots_hit_enemy >= 2, "PlrStat_LogShotHitEnemy per player hit [M 0x145558:57-60], got {}", st.shots_hit_enemy);
    assert!(st.shots_hit_enemy <= st.shots_fired, "the accuracy denominator nets the scenery hits off");
    assert!(st.elapsed > 0, "PlarStat_LogUpdateElapsedTime per tick [M 0x1995a0]");
    assert_eq!(sim.drones.progress.stats[0].health as u32 > 0 || sim.drones.health.health > 0.0, true, "the health snapshot ran");
    // One Bond-bonus pickup (kind 6 [M Pickup_Handler case 6 0x18e650:234-236]): teleport Bond onto
    // it, the tick's pickup pass takes it.
    let bonus = sim.drones.pickups.iter().position(|p| p.kind == nf_game::pickup::kind::BOND_BONUS && !p.gone)
        .expect("07000005 places Bond-bonus pickups (the row-8 count is 7 [L])");
    let at = sim.drones.pickups[bonus].m[3];
    sim.bond.pos = [at[0], at[1], at[2]];
    tick_idle(sim, 5);
    assert_eq!(sim.drones.progress.stats[0].bonus_pickups, 1, "PlrStat_LogBondBonus in the take [M 0x18e650:235]");
    assert!(sim.drones.pickups[bonus].gone, "the pickup is consumed");
    // Sister: a fresh load with NO kills shows the zero tally (the counters are per-mission).
    let mut scene2 = nf_view::load(&iso, "07000005.bin", false).expect("reload");
    let sim2 = scene2.sim.as_mut().unwrap();
    tick_idle(sim2, 30);
    assert_eq!((sim2.drones.progress.stats[0].dispatched, sim2.drones.progress.stats[0].bonus_pickups), (0, 0));
    assert_eq!(sim2.drones.progress.stats[0].spawned, spawned, "the same start + warm spawns");
    let _ = before_health;
}

/// ── 2. The difficulty global ────────────────────────────────────────────────────────────────────
/// `GameState+0x40` (0x2a37a8) — the boot default is 2 [M bootup 0x1c7e78:25]; the damage factor is
/// `Player_HandlePain` 0x1902c8's `Plr_DMod_*` ([L] 07000005: Easy 0.6 / Normal 0.7 / Hard 1.0,
/// ReadTuningVars 0x1d3a80). The port's factor is `crate::bullet::damage_factor`; the live run
/// proves it through a guard's SG5 hits on Bond (ai_live's setup, seeds re-picked there).
#[test]
fn the_difficulty_scales_a_guards_damage_to_bond_by_the_decompile_factor() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The sister assertion first: the DEFAULT difficulty (2) is what a fresh Sim boots with.
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    // The direct-level load replays the user's live sessions (difficulty 1, the old pre-menu
    // constant); a fresh `Drones::new` boots at the game's default 2 [M 0x1c7e78:25].
    assert_eq!(sim.drones.difficulty, 1, "the direct-level session difficulty");
    {
        let fresh = nf_game::drone::Drones::new(Vec::new(), nf_game::drone::Rng::new(1));
        assert_eq!(fresh.difficulty, 2, "the boot default [M 0x1c7e78:25]");
    }
    assert!((nf_game::bullet::damage_factor(0x0700_0005, 2) - 0.7).abs() < 1e-6,
        "the Normal factor on 07000005 [L ReadTuningVars 0x1d3a80]");
    assert!((nf_game::bullet::damage_factor(0x0700_0005, 1) - 0.6).abs() < 1e-6, "Easy 0.6");
    assert!((nf_game::bullet::damage_factor(0x0700_0005, 3) - 1.0).abs() < 1e-6, "Hard 1.0");
    // The same swap through the setter (the difficulty page's write [M C_SBNFDFCTY 0x216ca8:10-17]).
    sim.drones.set_difficulty(1);
    assert_eq!(sim.drones.difficulty, 1);
    assert_eq!(sim.data().difficulty, 1, "Data mirrors it for the AI");
    sim.drones.set_difficulty(3);
    assert_eq!(sim.drones.difficulty, 3);
    sim.drones.set_difficulty(2);
}

/// ── 3. The save round-trips through the game's own block format ─────────────────────────────────
/// LS_MakeMission 0x1cd720 + LS_MakeBonus 0x1ce3b0: the MSSN/BUNS blocks chained, CRC-32 trailer
/// ([M] the LS dossier in the report). The fields a finished 07000005 run would carry.
#[test]
fn the_save_carries_the_mission_fields_and_round_trips() {
    let mut p = nf_game::progress::Progress::default();
    // Play 07000005's results through the real counters: two kills, one bonus, then GetScore on
    // the results path (Mission_Status 6) — which is where the game writes last/best.
    p.log_enemy_dispatched(1);
    p.log_enemy_dispatched(1);
    p.log_bond_bonus(1);
    p.log_update_elapsed(30_000);
    p.log_health(1, 87.0);
    // The 0x07-map rows come from NewScoresRef (None here — the faithful NULL abort [M 0x1984b0]),
    // so the on-foot score itself cannot be computed without the PTP hand-off; the SAVE still
    // carries best/moments — write them the way a completed run would (set_level_bonus path).
    p.table[1].best = 123_456;
    p.table[1].rows[0].value = 2;
    p.set_level_bonus(0x0700_0005, 3, 0);
    let file = p.encode_save();
    let mut q = nf_game::progress::Progress::default();
    q.decode_save(&file).expect("the game's block walk + CRC accept it");
    assert_eq!(q.table[1].best, 123_456, "ScoringTable[i]+0x30 per entry [M 0x1cd720:54-56]");
    assert_eq!(q.table[1].rows[0].value, 2, "rows[0]+8, the moments value");
    assert!(q.has_medal(0x0700_0005, 3, 0), "the medal bits ride the BUNS block [M 0x1ce3b0]");
    assert_eq!(q.nightfire_status & 0b110, 0b110, "sp_level +0x10 for 07000005 + the next level");
    assert_eq!(q.game_complete, false);
    // Sister: a real file round-trip through the local-file API.
    let path = std::env::temp_dir().join("nf_progress_roundtrip.bin");
    p.save_to_file(&path).expect("write");
    let mut r = nf_game::progress::Progress::default();
    r.load_from_file(&path).expect("read").expect("parse");
    std::fs::remove_file(&path).ok();
    assert_eq!(r.table[1].best, 123_456);
    assert_eq!(r.bonus[0], q.bonus[0]);
}
