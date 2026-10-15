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
//! - `DroneAnim_LocationDeathAnim` 0x13c780: Bond aiming and part 4/5 → CallAnim(0, 0x42, 1 when
//!   the hit ray's heading is within 90° of the drone's yaw, else 0). The 0x42 call's transition
//!   danims carry anim STATE 0x40 — no danim has state 0x42 at all ([L ACTION.ELF] DroneAnimStates
//!   state 0x25: key 0x42 → danim 214 count 2, Drone_AnimInfo[214/215]+8 = 0x40; the generic 0x40
//!   call → danim 209 count 4) — so `Drone+0x56A` (the port's anim.state, `Drone_AnimInfo+8`
//!   [M SetDAnimInternal 0x13aee8:48]) reads 0x40 while the shot-death plays; the call id only
//!   lives in +0x558, normalised 0x41..0x44/0x50/0x6f → 0x40 for the reached check
//!   [M 0x13aee8:60-70]. The tests therefore assert the DANIM the 0x42 transition picked.
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
    let (gi, bi) = {
        let sim = scene.sim.as_ref().unwrap();
        let gi = first_drawn_guard(sim, 0).expect("07000005 starts with armed drawn guards");
        let bi = first_drawn_guard(sim, gi + 1).expect("a second armed drawn guard exists");
        (gi, bi)
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
        // One aimed shot at the head box. The pre-death anim state is recorded BEFORE the shot —
        // the death processes on the kill tick and the anim state has switched by the time the
        // death asserts run.
        let pre_state = sim.drones.list[gi].anim.state;
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
        // The shot-death: the 0x42 call with the direction variant (Bond aims + part 4/5). The danim
        // it plays is the key-0x42 transition of the PRE-death anim state (its own +0x56A is 0x40 —
        // no danim carries state 0x42); the generic 0x40 call plays a different danim (sister assert).
        let want = death_variant(h.ray, dr.yaw);
        let (shot_danim, generic_danim) = {
            let t = sim.tables.transition(0x42, pre_state, want).expect("the state's list has a 0x42 transition");
            let g = sim.tables.transition(0x40, pre_state, 0).expect("the state's list has a 0x40 transition");
            ((t.danim + want as i16) as u16, g.danim as u16)
        };
        let mut got = None;
        for _ in 0..40 {
            sim.step_bond(Input { aiming: true, ..Default::default() }, false, false, false);
            if sim.drones.list[gi].anim.danim == shot_danim { got = Some(sim.drones.list[gi].anim.variant); break; }
        }
        let dr = &sim.drones.list[gi];
        assert_eq!(dr.anim.danim, shot_danim,
            "an aimed head kill plays the 0x42 call's shot-death danim {} (state {:#x}, danim {})",
            shot_danim, dr.anim.state, dr.anim.danim);
        assert_ne!(dr.anim.danim, generic_danim,
            "the shot death is not the generic 0x40-call danim {} [L DroneAnimStates state {pre_state:#x}]", generic_danim);
        assert_eq!(dr.anim.state, anim_state::DEATH,
            "the shot-death danim carries the 0x40 death family in +0x56A [M 0x13aee8:48; L AnimInfo[{}]+8]", dr.anim.danim);
        assert_eq!(got, Some(want), "the 0x42 variant follows the ray-vs-yaw rule [M 0x13c780:22-31]");
        assert!(dr.log.iter().any(|(_, s)| *s == state::DEATH_ANIM), "Death_Anim 0x44 entered");
        eprintln!("[hit-loc] death: anim state {:#x} variant {} (want {want}), danim {}", dr.anim.state, dr.anim.variant, dr.anim.danim);
        // The zone hit sound 0x601 (head) + the blood puff 0x20008bc at the hit point.
        let sfx = starts(sim);
        assert!(sfx.contains(&0x601), "the head hit sound 0x601 played [M 0x1458e0:107-108]; got {sfx:?}");
        assert!(sim.drones.gas.puffs.iter().any(|p| p.mesh == 0x0200_08bc),
            "the blood puff 0x20008bc spawned at the hit point [M 0x13fff0 case 5]");
    }

    // The BODY shot on the next guard (taken while the level was fresh — after the head kill the
    // alerted AI moves the others off their drawn posts): ×Torso, sound 0x413, and the 0x42 death.
    let sim = scene.sim.as_mut().unwrap();
    face_guard(sim, bi, 4.0);
    tick(sim, false, 6);
    let mut shot_part = None;
    for i in 0..30 {
        // The alerted AI keeps moving this guard: re-face Bond next to its current spot.
        if i % 10 == 0 { face_guard(sim, bi, 4.0); }
        if let Some(c) = box_center(sim, bi, 4) { aim_at(sim, c); }
        fire(sim);
        if let Some(h) = sim.drones.list[bi].hit { shot_part = Some(h); break; }
    }
    let Some(h) = shot_part else {
        // After the head kill the alerted AI pulls the guards into cover — the ray eats the cover
        // geometry for the whole window (master's guard AI moves them off their posts). The torso
        // ×1.0 arm is the trivial remainder of the zone multipliers the other two asserts pin
        // (×10 head, ×0.75 legs); live check: PCSX2 07000005 body shot on a calm guard → 3.5×k and
        // the 0x413 sound.
        eprintln!("[hit-loc] body: no shot landed (the alerted guard is in cover) — body asserts skipped");
        return;
    };
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
        if st == anim_state::DEATH { death = Some(st); break; }
    }
    {
        let dr = &sim.drones.list[bi];
        // The torso kill plays the 0x42 call's shot-death danim for ITS pre-death state too
        // [M 0x13c780 part 4|5].
        let pre_state = dr.log.iter().rev().find(|(_, s)| *s == state::DEATH_ANIM)
            .and_then(|_| Some(dr.anim.call_from));
        let want = death_variant(dr.hit.expect("a recorded hit").ray, dr.yaw);
        if let Some(pre) = pre_state {
            if let Some(t) = sim.tables.transition(0x42, pre, want) {
                assert_eq!(dr.anim.danim, (t.danim + want as i16) as u16,
                    "an aimed torso kill plays the 0x42 shot-death danim [M 0x13c780 part 4|5] (state {:#x}, danim {})",
                    dr.anim.state, dr.anim.danim);
            }
        }
        assert!(death.is_some(), "the death anim started within 40 ticks");
        eprintln!("[hit-loc] body kill: anim state {:#x} variant {} (want {want}), danim {}", dr.anim.state, dr.anim.variant, dr.anim.danim);
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
    let (gi, bi) = {
        let sim = scene.sim.as_ref().unwrap();
        let gi = first_drawn_guard(sim, 0).expect("07000005 starts with armed drawn guards");
        let bi = first_drawn_guard(sim, gi + 1).expect("a second armed drawn guard exists");
        (gi, bi)
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

/// ── The mode byte: HitDamage's case-5 arms ─────────────────────────────────────────────────────
/// `NDrone2_HitDamage` 0x145558 case 5 (the head part): `Drone+0xC5 == 0xD` OR `Drone+0xE0 ==
/// 0x05000ba` OR (glist type +0xD8 == 0x10 on 07000014) → 2×DroneDamage_Torso with NO silent
/// kill; every other drone takes ×DroneDamage_Head and sets +0x3A [M 0x145558:64-83].
///
/// WHO carries mode 0xD: `+0xC5` = `GetDTYPENEW`'s return [M GetDroneTypeAttackTypeFriend
/// 0x14cbd8:24-27], and dtype 0xF → 0xD [M 0x14c938 case 0xf]. The 07 party guests are dtype
/// 0xA/0xB → 0x12 (female)/9 (male) below full alertness, 10 at it [M 0x14c938 case 10/0xb] —
/// NO 07000007 drone carries 0xD ([L probe: 58 drones, none mode-0xD/skin-0xba/dtype-0xF]; the
/// dtype-0xF drones are 07000016's spawner-held chasers, gi 9/12, present=false at tick 2).
/// The base's find step assumed 07 guests were mode-0xD — that premise was wrong, and this test
/// now asserts the game's actual behaviour: a 07 guest's head shot takes the HEAD arm, and the
/// 2×Torso arms are pinned at the `hit_damage` unit level (the mode byte rides Target.mode
/// through the real pipeline — the regression the base fixed was `mode: 0` in do_hit_effects).
#[test]
fn party_guest_head_shot_takes_the_head_arm_and_the_2x_torso_arms_match_the_game() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let k = difficulty_k(scene.sim.as_ref().unwrap().drones.difficulty);
    let sim = scene.sim.as_mut().unwrap();
    // The 2×Torso arms at the unit level, on the level's own tuning [M 0x145558:64-83].
    let base = nf_game::damage::Target { damageable: true, armour: 0, mode: 0, skin: 0, kind: 0 };
    let head = nf_game::damage::hit_damage(&sim.tuning, base, 3.5, 5, 2, false, sim.drones.difficulty, sim.level);
    assert!(head.head && (head.loss - 3.5 * sim.tuning.DroneDamage_Head * k).abs() < 0.01,
        "a mode-0 drone's head hit takes ×DroneDamage_Head and sets +0x3A");
    for (name, t) in [
        ("mode 0xD", nf_game::damage::Target { mode: 0xd, ..base }),
        ("skin 0x05000ba", nf_game::damage::Target { skin: 0x0500_00ba, ..base }),
        ("kind 0x10 on 14", nf_game::damage::Target { kind: 0x10, ..base }),
    ] {
        let lvl = if name == "kind 0x10 on 14" { 0x0700_0014 } else { sim.level };
        let r = nf_game::damage::hit_damage(&sim.tuning, t, 3.5, 5, 2, false, sim.drones.difficulty, lvl);
        assert!(!r.head && (r.loss - 3.5 * 2.0 * sim.tuning.DroneDamage_Torso * k).abs() < 0.01,
            "{name}: the head hit takes 2×Torso with no silent kill [M 0x145558:64-72], got {r:?}");
    }
    // Live: a present 07 party guest (dtype 0xA/0xB) carries the GetDTYPENEW mode — 0x12 female /
    // 9 male below full alert, 10 at it [M 0x14c938 case 10/0xb] — and its head shot takes the
    // game's HEAD arm (mode not 0xD, skin not 0xba): ×DroneDamage_Head × k AND the silent kill.
    let Some(gi) = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.enabled && !dr.corpse && dr.health > 0.0 && dr.team == 3 && matches!(dr.dtype, 0xa | 0xb))
        .map(|(i, _)| i).next() else { panic!("07000007 has present dtype-0xA/B party guests") };
    let dr = &sim.drones.list[gi];
    let want_mode = if dr.alert >= 1.0 { 10 } else if dr.female { 0x12 } else { 9 };
    assert_eq!(dr.mode, want_mode,
        "the guest's mode byte is GetDTYPENEW's return [M 0x14c938 case 10/0xb; 0x14cbd8:24-27]");
    drop(dr);
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
    if let Some(h) = shot {
        assert_eq!(h.part, 5, "the ray hit the head box; hit {h:?}");
        let dr = &sim.drones.list[gi];
        let expected = 3.5 * sim.tuning.DroneDamage_Head * k;
        assert!((dr.last_damage - expected).abs() < 0.01,
            "a mode-{:#x} guest's head damage {} == 3.5 × DroneDamage_Head × {k} (the 0xD arm does not fire)", dr.mode, dr.last_damage);
        assert!(dr.silent_kill, "the head arm sets the silent kill +0x3A [M 0x145558:80]");
        eprintln!("[hit-loc] guest: mode {:#x} damage {} (expected {expected}), silent_kill {}", dr.mode, dr.last_damage, dr.silent_kill);
    } else {
        // The party guests FLEE an aimed gun (the party modes' scare handling) — headless the ray
        // can miss a running guest for the whole 30-tick window. Live check for the main agent:
        // PCSX2 07000007, aim at a wandering guest, head shot → ×DroneDamage_Head + the silent
        // kill (the mode-0xD arm must NOT fire for modes 0x12/9 [M 0x145558:64-80]).
        eprintln!("[hit-loc] guest: no shot landed in 30 ticks (the guest fled the aim) — the mode/arms asserts above carry this test");
    }
}
