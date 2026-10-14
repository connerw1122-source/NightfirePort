//! fix-guard-weapons: the guard fire model against the game for every weapon guards carry —
//! the per-weapon damage (`weapon_data+0x0C`), fire interval and burst (`DroneWeap_NextBulletTime`
//! 0x179d40 / `BurstDelay` 0x179ba8), the accuracy model (`DroneWeap_DoBulletAccuracy` 0x179740),
//! the hit-time modifiers (`Drone_ModBulletDamage` 0x139f50) and the armour absorption on Bond
//! (`Player_HandlePain` 0x1902c8). Each test drives the REAL input path (`nf_view::load` +
//! `Sim::step_bond`). Opt-in: `NF_ISO`.
//!
//! The model numbers come from 07000001, whose guards carry three plain weapons (agg 0, acc 5):
//! 0x0e (damage 5, burst 1, interval 15 → the 60-clamp), 0x12 (damage 1.5, record burst 12 →
//! the aggression cut to 3), 0x1c (damage 3, burst 1, interval 60). [L ee_start_07000001.bin]:
//! DroneFiring_Accuracy_Easy/Normal/Hard = 0.5/0.7/1.0, Plr_DMod_* = 0.5/0.7/1.2, TooClose
//! distance 3.0. Guards are sighted 120+ ticks (the NewSighting gate), Bond still, 10 units out.

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

const LEVEL: &str = "07000001.bin";
/// [L work/ee_start_07000001.bin @ 0x30caf0/54/58] — `DroneFiring_Accuracy_*` as rewritten by
/// `ReadTuningVars` for level 0x7000001.
const ACC: [f32; 3] = [0.5, 0.7, 1.0];
/// [L work/ee_start_07000001.bin @ 0x30cc50/54/58] — `Plr_DMod_*` (ReadTuningVars group 0x1d3e00).
const DMG: [f32; 3] = [0.5, 0.7, 1.2];

struct Shot { frame: u32, hit: bool }

/// Drive one guard into firing at a still Bond 10 units away and collect `wanted` shots from him
/// alone (Bond topped back to 100 health each tick so the per-hit deltas stay exact). Returns
/// (guard index, the shots with their hit flags, the per-hit health deltas, the final distance).
fn collect_shots(scene: &mut nf_view::scene::Scene, weapon: u32, wanted: u32, warmup_shots: u32) -> (usize, Vec<Shot>, Vec<f32>, f32) {
    // Live guards with the weapon, the standard stats and the standard ×1 damage multiplier —
    // tried in order until one actually opens fire (some are elevated or scripted and never do).
    let candidates: Vec<usize> = {
        let sim = scene.sim.as_ref().unwrap();
        let mut v: Vec<usize> = (0..sim.drones.list.len()).filter(|&i| {
            let d = &sim.drones.list[i];
            d.present && d.weapon == weapon && d.aggression == 0
        }).collect();
        v.sort_by_key(|&i| (sim.drones.list[i].mode != 0, i));
        v
    };
    assert!(!candidates.is_empty(), "no live guard with weapon {weapon:#x}");
    // Park Bond 10 units from the guard along an axis, at the guard's own floor height, facing him
    // (no backshot). Try the four axis directions until he stands level; then warm up until the
    // sighting gate (120 ticks, `DroneWeap_DoBulletAccuracy`) and his first shots.
    let mut guard = usize::MAX;
    let mut w0 = 0usize;
    'guards: for &cand in &candidates {
        let mut placed = false;
        'dirs: for &(dx, dz) in &[(1.0f32, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (0.70710678, 0.70710678), (-0.70710678, 0.70710678), (0.70710678, -0.70710678), (-0.70710678, -0.70710678)] {
            let sim = scene.sim.as_mut().unwrap();
            let g = sim.drones.list[cand].pos;
            sim.bond.pos = [g[0] + dx * 10.0, g[1] + 0.14, g[2] + dz * 10.0];
            sim.bond.yaw = (g[0] - sim.bond.pos[0]).atan2(g[2] - sim.bond.pos[2]);
            for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
            let y = sim.bond.pos[1];
            if (y - (g[1] + 0.14)).abs() < 1.0 { placed = true; break 'dirs; }
        }
        if !placed { eprintln!("[gw] guard {cand} (weapon {weapon:#x}): no level parking spot"); continue 'guards; }
        for warmup in 0..6000 {
            let sim = scene.sim.as_mut().unwrap();
            sim.step_bond(Default::default(), false, false, false);
            sim.drones.health.health = 100.0;
            let d = &sim.drones.list[cand];
            if d.shots.len() >= warmup_shots as usize && d.seen_ticks >= 120 {
                guard = cand; w0 = d.shots.len(); break 'guards;
            }
            if warmup == 5999 { eprintln!("[gw] guard {cand} (weapon {weapon:#x}): no fire in 6000 ticks (shots {}, seen {})", d.shots.len(), d.seen_ticks); }
        }
    }
    assert!(guard != usize::MAX, "no candidate guard with weapon {weapon:#x} opened fire");
    let _ = warmup_shots;
    let mut hit_events: Vec<(u32, f32)> = Vec::new(); // (bullet-arrival frame, health delta) for this guard
    let start_frame = scene.sim.as_ref().unwrap().drones.frame;
    let cap = 90000u32;
    loop {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        sim.drones.health.health = 100.0;
        let got = sim.drones.list[guard].shots.len() - w0;
        for &(f, owner, delta) in &sim.drones.player_hits {
            if owner == guard { hit_events.push((f, delta)); }
        }
        sim.drones.player_hits.clear();
        if got >= wanted as usize + 5 { break; } // +5: let the in-flight bullets land before matching
        assert!(sim.drones.frame - start_frame < cap, "ran out of ticks collecting {} shots (got {})", wanted, got);
    }
    // Match each shot to its bullet's arrival: a hit iff a hit event lands between this shot's
    // frame and the next shot's (flight is a few ticks; the gaps are ≥ 6).
    let frames: Vec<u32> = scene.sim.as_ref().unwrap().drones.list[guard].shots[w0..].to_vec();
    let mut shots = Vec::new();
    let mut deltas = Vec::new();
    let mut hi = 0usize;
    for k in 0..wanted as usize {
        let f = frames[k];
        let next = frames.get(k + 1).copied().unwrap_or(u32::MAX);
        let mut hit = false;
        while hi < hit_events.len() && hit_events[hi].0 < f { hi += 1; }
        while hi < hit_events.len() && hit_events[hi].0 < next {
            hit = true; deltas.push(hit_events[hi].1); hi += 1;
        }
        shots.push(Shot { frame: f, hit });
    }
    let dist = scene.sim.as_ref().unwrap().drones.list[guard].dist;
    (guard, shots, deltas, dist)
}

/// The task's matrix: 3 weapons × 3 difficulties — over 100 shots at 10 units the hit rate equals
/// the game's model (p = (100 − 5·acc) × level accuracy, no TooClose ×2 at 10 units, target still)
/// and every per-hit damage equals the game's (weapon_data+0x0C × Drone+0x100 × Plr_DMod).
/// Weapons: 07000001's 0x0e (interval 15 → the 60-clamp) and 0x12 (record burst 12 → the
/// aggression cut), plus 07000005's SG-5 0x16 (burst 3, interval 4) — the guards' three behaviours.
#[test]
fn the_3x3_hit_rate_and_damage_matrix_matches_the_game() {
    // [L work/ee_start_07000001.bin @ 0x30caf0/54/58 and 0x30cc50/54/58] — the accuracy and
    // Plr_DMod statics as ReadTuningVars leaves them for 0x7000001.
    const L01: ([f32; 3], [f32; 3]) = ([0.5, 0.7, 1.0], [0.5, 0.7, 1.2]);
    // [L work/ee_start_05.bin @ 0x30caf0/54/58 and 0x30cc50/54/58] — the same for 0x7000005.
    const L05: ([f32; 3], [f32; 3]) = ([0.8, 0.9, 1.3], [0.6, 0.7, 1.0]);
    for diff in 1..=3u32 {
        let mut run = |level: &str, weapon: u32, acc_tbl: [f32; 3], dmg_tbl: [f32; 3], scene: &mut nf_view::scene::Scene| {
            let record_damage = {
                let sim = scene.sim.as_ref().unwrap();
                f32::from_le_bytes(sim.weapons.records[weapon as usize][0x0c..0x10].try_into().unwrap())
            };
            let (guard, shots, deltas, dist) = collect_shots(scene, weapon, 100, 5);
            let sim = scene.sim.as_ref().unwrap();
            let (acc, mul) = (sim.drones.list[guard].accuracy, sim.drones.list[guard].damage_mul);
            let expected_rate = (100.0_f32 - 5.0 * acc as f32) * acc_tbl[(diff - 1) as usize] / 100.0;
            let expected_hit = record_damage * mul * dmg_tbl[(diff - 1) as usize];
            let hits = shots.iter().filter(|s| s.hit).count();
            let n = shots.len();
            eprintln!("[gw] {level} diff {diff} weapon {weapon:#x} (guard {guard}, acc {acc}, mul {mul}): {hits}/{n} hits (model {:.2}), per-hit {:?} (model {expected_hit}), dist {dist:.2}",
                expected_rate * 100.0, &deltas[..deltas.len().min(6)]);
            assert!(dist > 8.0 && dist < 12.5, "guard {guard} did not stay at 10 units (dist {dist})");
            let rate = hits as f32 / n as f32;
            assert!((rate - expected_rate).abs() <= 0.12,
                "{level} diff {diff} weapon {weapon:#x}: hit rate {rate:.3} vs model {expected_rate:.3} ({hits}/{n})");
            assert!(!deltas.is_empty(), "{level} diff {diff} weapon {weapon:#x}: no hits landed");
            for (k, &d) in deltas.iter().enumerate() {
                assert_eq!(d, expected_hit, "{level} diff {diff} weapon {weapon:#x}: hit #{k} damage {d} vs model {expected_hit}");
            }
        };
        // 07000001 in one scene, in this order: 0x0e fires on a fresh level; the 0x12 guards only
        // engage once the level is alerted (the 0x0e phase's gunfire does that).
        let mut scene = nf_view::load(&iso(), LEVEL, false).expect("load 07000001");
        scene.sim.as_mut().unwrap().drones.difficulty = diff;
        run(LEVEL, 0x0e, L01.0, L01.1, &mut scene);
        run(LEVEL, 0x12, L01.0, L01.1, &mut scene);
        drop(scene);
        // 07000005's SG-5 (burst 3, interval 4) on a fresh level.
        let mut scene = nf_view::load(&iso(), "07000005.bin", false).expect("load 07000005");
        scene.sim.as_mut().unwrap().drones.difficulty = diff;
        run("07000005.bin", 0x16, L05.0, L05.1, &mut scene);
    }
}

/// Sister: at 10 units (≥ the level's TooClose 3.0) there is NO ×2 — the same guard at 1.5 units
/// hits every shot (`DroneWeap_DoBulletAccuracy`'s dist < 3.0 forced hit) for exactly twice the
/// damage (`Drone_ModBulletDamage` 0x139fac-0x139fc8, TooClose_Damage 2.0 [L ee snapshot 0x30cb1c]).
#[test]
fn too_close_doubles_the_damage_and_forces_the_hit() {
    let mut scene = nf_view::load(&iso(), LEVEL, false).expect("load 07000001");
    scene.sim.as_mut().unwrap().drones.difficulty = 1;
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let guard = {
        let sim = scene.sim.as_ref().unwrap();
        (0..sim.drones.list.len()).find(|&i| {
            let d = &sim.drones.list[i];
            d.present && d.weapon == 0x0e && d.accuracy == 5 && d.aggression == 0 && d.damage_mul == 1.0
        }).expect("no live weapon-0x0e guard")
    };
    let g = scene.sim.as_ref().unwrap().drones.list[guard].pos;
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [g[0] + 1.5, g[1] + 0.14, g[2]];
        sim.bond.yaw = std::f32::consts::PI; // facing back at him
    }
    let mut deltas = Vec::new();
    for t in 0..9000u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        sim.drones.health.health = 100.0;
        for &(_, owner, delta) in &sim.drones.player_hits {
            if owner == guard { deltas.push(delta); }
        }
        sim.drones.player_hits.clear();
        if deltas.len() >= 20 { break; }
        assert!(t < 8900, "the close guard never landed 20 hits (got {})", deltas.len());
    }
    let expected = 5.0_f32 * DMG[0] * 2.0;
    for (k, &d) in deltas.iter().enumerate() {
        assert_eq!(d, expected, "close hit #{k}: {d} vs {expected}");
    }
    let sim = scene.sim.as_ref().unwrap();
    let dist = sim.drones.list[guard].dist;
    eprintln!("[gw] too-close: {} hits at dist {dist:.2}, all exactly {expected}", deltas.len());
    assert!(dist < 3.0, "guard drifted out of the TooClose band (dist {dist})");
}

/// The fire-rate fixes, fail-first: weapon 0x0e's interval 15 is clamped to 60 (`NextBulletTime`
/// 0x179dc4-0x179e04), so every shot gap is ≥ 60 + the ≥ 15 burst delay; weapon 0x12's record
/// burst 12 is cut to `trunc(12 × 0.3)` = 3 rounds (0x179f80-0x17a01c, `BOT_getAggressionMul`).
#[test]
fn the_fire_rate_matches_the_game_interval_clamp_and_burst_cut() {
    let mut scene = nf_view::load(&iso(), LEVEL, false).expect("load 07000001");
    scene.sim.as_mut().unwrap().drones.difficulty = 2;
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // 0x0e: single-shot weapon, so every gap = burst delay + interval ≥ 15 + 60.
    let (_, shots, _, _) = collect_shots(&mut scene, 0x0e, 40, 3);
    let gaps: Vec<u32> = shots.windows(2).map(|p| p[1].frame - p[0].frame).collect();
    let min_gap = *gaps.iter().min().unwrap();
    eprintln!("[gw] 0x0e gaps: min {min_gap} of {gaps:?}");
    assert!(min_gap >= 75, "weapon 0x0e fired faster than the game's 60-tick clamp (min gap {min_gap})");
    // 0x12: bursts of exactly 3 (gap ≤ interval 12 inside a burst, then the ≥ 15 delay).
    let (_, shots, _, _) = collect_shots(&mut scene, 0x12, 40, 3);
    let mut runs: Vec<u32> = Vec::new();
    let mut run = 1u32;
    for w in shots.windows(2) {
        if w[1].frame - w[0].frame <= 12 { run += 1; } else { runs.push(run); run = 1; }
    }
    runs.push(run);
    eprintln!("[gw] 0x12 burst runs: {runs:?}");
    assert!(runs.iter().all(|&r| r == 3), "weapon 0x12 burst lengths {runs:?} — the game cuts the 12-round record to 3 (aggression 0.3)");
}
