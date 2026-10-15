//! fix-hit-locations live tests: the guard hit-location pipeline through the REAL path the viewer
//! uses (`nf_view::load` + `Sim::step_bond` — the P2K fired from the aim, the bullet walked, the
//! guard's state machine taking the msg-8 hit).
//!
//! The evidence (this session, docs/reports/fix-hit-locations.md):
//! - `NDrone2_HitDamage` 0x145558: the per-zone multipliers — head `DroneDamage_Head` (10 on the
//!   05 tuning [L]) with the silent-kill byte +0x3A, legs ×`DroneDamage_Legs` (0.75), torso ×1;
//!   the difficulty factor ×; a head hit on a mode-0xD drone (`+0xC5`, the party guests) reads
//!   2×Torso WITHOUT the silent kill [M 0x145558:64-80].
//! - `NDrone2_DoHitEffects` 0x1458e0:102-113: damage > 0.5 and weapon+0x70 without 0x180 → the hit
//!   sound at the hit point — 0x601 when HITDATA+0x52 is 5 (head), else 0x413.
//! - `DroneAnim_LocationImpactAnim` 0x13c8b8: the flinch 0x4C — variant 5 + Rand(8) unless Bond
//!   aims, then by the part: 4/5 → 5, 0x31-0x33 → 13, 0x35-0x37 → 12.
//! - `DroneAnim_LocationDeathAnim` 0x13c780: Bond aiming and part 4/5 → 0x42 with variant 1 when
//!   the hit ray's heading is within 90° of the drone's yaw, else 0.
//! - `NDrone2_BulletImpact` 0x1461c8: the death state by the mode byte — 0x1B/0x1C → 0x92, 0x1D →
//!   0xBF, 0x1E → 0xF2, else 0x44 — and `DroneFunc_SendHurtMessage` 0x145e28 on EVERY bullet hit.
//! - `NDrone2_DealWithObjHit` case 5 second switch (0x140300-0x14034c): the blood/impact puff —
//!   `Gas_Init(3.0, 6.0, …, 0x20008bc, 5, …)` gated on the unaligned f32 at weapondef+0x06 > 0,
//!   skipped for 0x2b/0x34-0x37, the stun guns 0x32/0x33 → 0x200026e with 17.0.
//!
//! Opt-in via `NF_ISO`.

use nf_game::drone::{anim_state, state, Data};
use nf_game::player::Input;
use nf_game::sound::Cmd;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    scene
}

/// The per-tick `Data` bundle the drone code needs (mirrors scene.rs's own construction).
fn data(sim: &nf_view::scene::Sim) -> Data {
    Data {
        tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
        world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
        skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
        hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
        surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
    }
}

/// The centre of the collision box with bone id `part` (5 head, 4 torso, 0x31-0x38 legs).
fn box_center(sim: &nf_view::scene::Sim, gi: usize, part: i16) -> Option<[f32; 3]> {
    let d = data(sim);
    sim.drones.list[gi].hit_boxes(&d).iter().find(|b| b.part == part)
        .map(|b| [b.m[3][0], b.m[3][1], b.m[3][2]])
}

/// Point Bond's view at `to` from his eye.
fn aim_at(sim: &mut nf_view::scene::Sim, to: [f32; 3]) {
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
}

/// Stand Bond `dist` units in front of the guard (−Z side) at floor height.
fn face_guard(sim: &mut nf_view::scene::Sim, gi: usize, dist: f32) {
    let gp = sim.drones.list[gi].pos;
    let floor = sim.world.floor_below(gp, 6.0).unwrap_or(gp[1]);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [gp[0], floor + 1.1, gp[2] - dist], 0.0);
}

fn tick(sim: &mut nf_view::scene::Sim, aiming: bool, n: u32) {
    for _ in 0..n { sim.step_bond(Input { aiming, ..Default::default() }, false, false, false); }
}

/// Fire one tick with the aim held.
fn fire(sim: &mut nf_view::scene::Sim) {
    sim.step_bond(Input { aiming: true, ..Default::default() }, true, false, false);
}

/// A drawn armed enemy guard.
fn first_drawn_guard(sim: &nf_view::scene::Sim, skip: usize) -> Option<usize> {
    sim.drones.list.iter().enumerate()
        .filter(|(i, dr)| *i >= skip && dr.present && dr.enabled && dr.drawn && !dr.corpse && dr.health > 0.0 && dr.team == 1 && (2..0x47).contains(&dr.weapon))
        .map(|(i, _)| i).next()
}

/// The audio outbox flushed once (every node becomes a `Cmd::Start`).
fn starts(sim: &mut nf_view::scene::Sim) -> Vec<u32> {
    let listener = ([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    sim.drones.audio.update(None, listener, &|_| true);
    sim.drones.audio.out.iter().filter_map(|c| match c {
        Cmd::Start { id, .. } => Some(*id), _ => None,
    }).collect()
}

/// The difficulty factor of `NDrone2_HitDamage` 0x145558:134-149 (the tuning constants are the 05
/// level statics [L checked]: Easy 1.5 / Normal 1.0 / Hard 0.8).
fn difficulty_k(difficulty: u32) -> f32 {
    match difficulty { 1 => 1.5, 3 | 4 => 0.8, _ => 1.0 }
}

/// `DroneAnim_LocationDeathAnim` 0x13c780:22-31: variant 1 when the hit ray's heading is within
/// 90° of the drone's yaw, else 0.
fn death_variant(ray: [f32; 3], yaw: f32) -> u16 {
    let a = ray[0].atan2(ray[2]);
    let diff = (a - yaw).abs();
    let diff = diff.min((std::f32::consts::TAU - diff).abs());
    if diff <= std::f32::consts::FRAC_PI_2 + 1e-4 { 1 } else { 0 }
}

/// ── The headline test: a P2K head shot vs body shot on a 05 guard ───────────────────────────────
/// A head shot deals `weapon+0x0C (3.5) × DroneDamage_Head (10) × difficulty`, sets the silent-kill
/// byte (+0x3A, no death talk) and kills through the 0x42 shot-death anim; the body shot deals
/// ×Torso (1.0) and plays the zone hit sound 0x413 (the head 0x601). Both leave the blood/impact
/// gas puff 0x20008bc at the hit point (DealWithObjHit case 5's second switch).
#[test]
fn p2k_head_shot_deals_the_games_damage_and_plays_the_shot_death() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let k = difficulty_k(scene.sim.as_ref().unwrap().drones.difficulty);
    let head_expected = 3.5 * 10.0 * k;
    let torso_expected = 3.5 * k;

    // The HEAD shot: one shot, one kill, silent.
    let gi = {
        let sim = scene.sim.as_mut().unwrap();
        first_drawn_guard(sim, 0).expect("07000005 starts with armed drawn guards")
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        face_guard(sim, gi, 4.0);
        tick(sim, false, 6);
        let hp0 = sim.drones.list[gi].health;
        let mut aimed = false;
        for _ in 0..60 {
            if let Some(c) = box_center(sim, gi, 5) { aim_at(sim, c); }
            sim.step_bond(Input { aiming: true, ..Default::default() }, false, false, false);
            if sim.drones.bond_aiming { aimed = true; break; }
        }
        assert!(aimed, "the P2K raises to aim within a second");
        // One aimed shot at the head box.
        let mut hit_part = None;
        for _ in 0..30 {
            if let Some(c) = box_center(sim, gi, 5) { aim_at(sim, c); }
            fire(sim);
            if let Some(h) = sim.drones.list[gi].hit { hit_part = Some(h); break; }
            if sim.drones.list[gi].health <= 0.0 { break; }
        }
        let h = hit_part.expect("the head shot registered a Hit on the guard");
        assert_eq!(h.part, 5, "the ray hit the head box (part 5); hit {h:?}");
        assert_eq!(h.weapon, 2, "the shooter's weapon is the P2K");
        let dr = &sim.drones.list[gi];
        assert!(dr.health <= 0.0, "a head shot (3.5 × 10 × {k}) one-shots a 10 hp guard");
        assert!((dr.last_damage - head_expected).abs() < 0.01,
            "head damage {} == 3.5 × DroneDamage_Head(10) × {k}", dr.last_damage);
        assert!(dr.silent_kill, "a head kill sets the silent-kill byte +0x3A [M 0x145558:78]");
        assert!(hp0 - dr.last_damage <= 0.0 || dr.health <= 0.0, "the head shot killed");
        eprintln!("[hit-loc] head: damage {} (expected {head_expected}), part {}, ray {:?}", dr.last_damage, h.part, h.ray);
        // The shot-death anim 0x42 with the direction variant (Bond aims + part 4/5).
        let want = death_variant(h.ray, dr.yaw);
        let mut got = None;
        for _ in 0..40 {
            sim.step_bond(Input { aiming: true, ..Default::default() }, false, false, false);
            if sim.drones.list[gi].anim.state == anim_state::DEATH_SHOT { got = Some(sim.drones.list[gi].anim.variant); break; }
        }
        let dr = &sim.drones.list[gi];
        assert_eq!(dr.anim.state, anim_state::DEATH_SHOT,
            "an aimed head kill plays the 0x42 shot-death, not 0x40 (state {:#x})", dr.anim.state);
        assert_eq!(got, Some(want), "the 0x42 variant follows the ray-vs-yaw rule [M 0x13c780:22-31]");
        assert!(dr.log.iter().any(|(_, s)| *s == state::DEATH_ANIM), "Death_Anim 0x44 entered");
        eprintln!("[hit-loc] death: anim state {:#x} variant {} (want {want}), danim {}", dr.anim.state, dr.anim.variant, dr.anim.danim);
        // The zone hit sound 0x601 (head) + the blood puff 0x20008bc at the hit point.
        let sfx = starts(sim);
        assert!(sfx.contains(&0x601), "the head hit sound 0x601 played [M 0x1458e0:107-108]; got {sfx:?}");
        assert!(sim.drones.gas.puffs.iter().any(|p| p.mesh == 0x0200_08bc),
            "the blood puff 0x20008bc spawned at the hit point [M 0x13fff0 case 5]");
    }

    // The BODY shot on the next guard: ×Torso, sound 0x413, and the 0x42 death on the kill.
    let bi = {
        let sim = scene.sim.as_ref().unwrap();
        first_drawn_guard(sim, 0).expect("a second armed drawn guard exists")
    };
    let sim = scene.sim.as_mut().unwrap();
    face_guard(sim, bi, 4.0);
    tick(sim, false, 6);
    let mut shot_part = None;
    for _ in 0..30 {
        if let Some(c) = box_center(sim, bi, 4) { aim_at(sim, c); }
        fire(sim);
        if let Some(h) = sim.drones.list[bi].hit { shot_part = Some(h); break; }
    }
    let h = shot_part.expect("the body shot registered a Hit");
    {
        let dr = &sim.drones.list[bi];
        assert_eq!(h.part, 4, "the ray hit the torso box (part 4); hit {h:?}");
        assert!((dr.last_damage - torso_expected).abs() < 0.01,
            "torso damage {} == 3.5 × DroneDamage_Torso(1.0) × {k}", dr.last_damage);
        assert!(dr.health > 0.0, "one body shot does not kill a 10 hp guard");
        assert!(!dr.silent_kill, "a body shot sets no silent kill");
        eprintln!("[hit-loc] body: damage {} (expected {torso_expected}), part {}", dr.last_damage, h.part);
    }
    // Finish him: chest shots every 2.2 s (the flinch cooldown) until dead; the kill plays 0x42.
    for _ in 0..6 {
        for _ in 0..132 {
            if let Some(c) = box_center(sim, bi, 4) { aim_at(sim, c); }
            sim.step_bond(Input { aiming: true, ..Default::default() }, false, false, false);
        }
        fire(sim);
        if sim.drones.list[bi].health <= 0.0 { break; }
    }
    assert!(sim.drones.list[bi].health <= 0.0, "the chest shots kill the guard (health {})",
        sim.drones.list[bi].health);
    let mut death = None;
    for _ in 0..40 {
        sim.step_bond(Input { aiming: true, ..Default::default() }, false, false, false);
        let st = sim.drones.list[bi].anim.state;
        if st == anim_state::DEATH_SHOT || st == anim_state::DEATH { death = Some(st); break; }
    }
    {
        let dr = &sim.drones.list[bi];
        assert_eq!(dr.anim.state, anim_state::DEATH_SHOT,
            "an aimed torso kill also plays the 0x42 shot-death [M 0x13c780 part 4|5] (state {:#x})", dr.anim.state);
        let want = death_variant(dr.hit.expect("a recorded hit").ray, dr.yaw);
        assert!(death.is_some(), "the death anim started within 40 ticks");
        eprintln!("[hit-loc] body kill: anim state {:#x} variant {} (want {want})", dr.anim.state, dr.anim.variant);
    }
    let sfx = starts(sim);
    assert!(sfx.contains(&0x413), "the body hit sound 0x413 played [M 0x1458e0:110-112]; got {sfx:?}");
}

/// ── The zone flinch: leg stagger and the legs multiplier ─────────────────────────────────────────
/// A leg hit deals ×DroneDamage_Legs (0.75) and — when the guard is in an aim-family state (the
/// per-state (resume, allow) pairs of `DroneFunc_HandleImpact` 0x1469b8) and Bond aims — plays the
/// flinch 0x4C with the leg variant (0x31-0x33 → 13, 0x35-0x37 → 12) [M 0x13c8b8:85-93].
#[test]
fn p2k_leg_shot_deals_the_legs_multiplier_and_plays_the_leg_stagger() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let k = difficulty_k(scene.sim.as_ref().unwrap().drones.difficulty);
    let leg_expected = 3.5 * 0.75 * k;
    let gi = {
        let sim = scene.sim.as_mut().unwrap();
        first_drawn_guard(sim, 0).expect("07000005 starts with armed drawn guards")
    };
    let sim = scene.sim.as_mut().unwrap();
    face_guard(sim, gi, 5.0);
    // Hold the aim in the open until the guard aims back (an allow-1 state — the aim family).
    let mut ready = false;
    for _ in 0..1200 {
        tick(sim, true, 1);
        let s = sim.drones.list[gi].state;
        if matches!(s, state::AIM_STAND | state::AIM_STAND_FIRE | state::AIM_STAND_RELOAD | state::AIM_BACKOFF
            | 0x6f | 0x70 | 0x73 | 0x75 | state::COMBAT_NO_MOVE | state::COMBAT_NO_ROUTE) { ready = true; break; }
    }
    let s = sim.drones.list[gi].state;
    assert!(ready, "the guard reached an aim-family state (state {:#x})", s);
    // One aimed shot at a leg box; the part decides the variant.
    let mut leg_part = None;
    for part in [0x35i16, 0x32, 0x31, 0x36] {
        let Some(c) = box_center(sim, gi, part) else { continue };
        aim_at(sim, c);
        fire(sim);
        if let Some(h) = sim.drones.list[gi].hit {
            if (0x31..=0x38).contains(&h.part) { leg_part = Some(h.part); break; }
            // A stray torso/head hit — wait out the flinch window and retry the leg.
            for _ in 0..130 { tick(sim, true, 1); }
        } else {
            for _ in 0..10 { tick(sim, true, 1); }
        }
    }
    let part = leg_part.expect("an aimed leg shot hit a leg box");
    let dr = &sim.drones.list[gi];
    assert!((dr.last_damage - leg_expected).abs() < 0.01,
        "leg damage {} == 3.5 × DroneDamage_Legs(0.75) × {k}", dr.last_damage);
    let want = if (0x31..=0x33).contains(&part) { 13 } else { 12 };
    let mut got = None;
    for _ in 0..20 {
        sim.step_bond(Input { aiming: true, ..Default::default() }, false, false, false);
        if sim.drones.list[gi].anim.state == anim_state::IMPACT { got = Some(sim.drones.list[gi].anim.variant); break; }
    }
    assert_eq!(sim.drones.list[gi].anim.state, anim_state::IMPACT,
        "an aimed hit on an aiming guard plays the flinch 0x4C (state {:#x})", sim.drones.list[gi].anim.state);
    assert_eq!(got, Some(want), "the leg flinch variant follows the part ({} → {want})", part);
    eprintln!("[hit-loc] leg: part {part:#x}, damage {}, flinch variant {} (want {want})",
        sim.drones.list[gi].last_damage, sim.drones.list[gi].anim.variant);
}

/// ── The mode byte: a party guest's head shot reads 2×Torso, no silent kill ──────────────────────
/// `NDrone2_HitDamage` 0x145558:64-66: `Drone+0xC5 == 0xD` (the mode the party guests get via
/// `GetDTYPENEW` 0x14c938 dtype 0xF → 0xD) sends a head hit down the 2×Torso arm and never sets
/// the silent kill. The port passed mode 0 (the head arm: ×10 + silent kill).
#[test]
fn party_guest_head_shot_deals_two_torso_without_the_silent_kill() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let k = difficulty_k(scene.sim.as_ref().unwrap().drones.difficulty);
    let sim = scene.sim.as_mut().unwrap();
    let Some(gi) = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.enabled && dr.drawn && !dr.corpse && dr.health > 0.0 && dr.mode == 0xd)
        .map(|(i, _)| i).next() else { panic!("07000007 has drawn mode-0xD guests") };
    face_guard(sim, gi, 3.0);
    tick(sim, false, 6);
    let mut aimed = false;
    for _ in 0..60 {
        if let Some(c) = box_center(sim, gi, 5) { aim_at(sim, c); }
        sim.step_bond(Input { aiming: true, ..Default::default() }, false, false, false);
        if sim.drones.bond_aiming { aimed = true; break; }
    }
    assert!(aimed, "the aim raises");
    let mut shot = None;
    for _ in 0..30 {
        if let Some(c) = box_center(sim, gi, 5) { aim_at(sim, c); }
        fire(sim);
        if let Some(h) = sim.drones.list[gi].hit { shot = Some(h); break; }
    }
    let h = shot.expect("the guest head shot registered a Hit");
    assert_eq!(h.part, 5, "the ray hit the head box; hit {h:?}");
    let dr = &sim.drones.list[gi];
    let expected = if dr.damageable { 3.5 * 2.0 * k } else { 3.5 };
    assert!((dr.last_damage - expected).abs() < 0.01,
        "a mode-0xD guest's head damage {} == 3.5 × 2 × Torso × {k} (damageable {})", dr.last_damage, dr.damageable);
    assert!(!dr.silent_kill, "the 2×Torso arm never sets the silent kill [M 0x145558:64-66]");
    eprintln!("[hit-loc] guest: mode {:#x} damage {} (expected {expected}), silent_kill {}",
        dr.mode, dr.last_damage, dr.silent_kill);
}
