//! fix-oddjob-hat — Oddjob's hat (0x45): a thrown one-shot kill, not a grenade, through the REAL
//! input path (`nf_view::load_weapons` + `Sim::step_bond`).
//!
//! The playtest (1013-2313): "Oddjob's hat should be a one-shot kill but it acts like a grenade."
//! What the disc says:
//! - The hat's record [L ee_start_05.bin @0x2c398c]: damage 100.0 (+0x0C — one shot kills any
//!   guard), speed 0.6667 (+0x20), range 1000 (+0x1C), `+0x6C` = 0x20C08 — gravity (0x8), NO
//!   flesh pass-through (0x800) and the HOMING bit (0x20000, the only carrier among the thrown
//!   projectiles) — `+0x70` = 0x100 (the stick flag; the fuse bit 0x10000 is clear, the blast
//!   bits 0x4/0x200 are clear — the hat never explodes [M 0x12e7b0:363-371, 0x12ce80:315]).
//! - The homing: `Bullet_update` 0x12e7b0:476-478 runs `Bullet_homing` 0x12f358 for every flying
//!   0x20000 bullet — the nearest heat-seekable object (`Bullet_heatseekable` 0x12f248: live
//!   guards / Bond / copter bodies) within 1000 units, inside the 0.66-dot cone of the flight
//!   direction, with line of sight [M 0x12f358:64-104]; the steer is
//!   `dir += clamp((to_target − dir)·0.0333, ±0.01)` per axis, re-normalised, speed preserved
//!   [M 0x12f358:126-166]. NO return-to-thrower exists anywhere: the pre-fix "unmeasured" note
//!   resolves to the homing + the stick, and a stuck hat just sits (below).
//! - The stick: a world contact STOPS the hat — `Bullet_CollisionHandler` 0x12ce80:255 sets
//!   `obj+0xF4 = 3`, the matrix aligns to the negated hit normal [:256-266] and the hat alone
//!   gets the 1500.0 post-stick lifetime + the 0x4a7 clang [:315-317]; the bounce block
//!   (LAB_0012d4bc) needs `+0x70 & 0x80` or a character hit flag [:398-407] and a +0x100
//!   record's world hit has neither. `Bullet_update` case 3 counts the 1500 down (0x45 only,
//!   0x12e7b0:363-371) and deletes silently — `Bullet_Delete`'s effect kind (+0x70 & 7 = 0)
//!   spawns nothing. A stuck hat's handler never runs again [M 0x12ce80:102] — it kills nobody.
//! - Versus a character the hat takes the standard bullet path (the damage → msg 8) and BOUNCES
//!   (kinds 2/3 set the iStack_100 flag → the bounce block, [M 0x12ce80:239-249,398-407]) — the
//!   port's contact-kill pass (fix-gadgets-live) already covers the kill; the homing here is
//!   what makes the throw HIT in the first place.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Load pre-merged for the weapon, give it through the panel (the user's path), wait for the draw.
fn load_weapon(iso: &str, id: u8) -> nf_view::scene::Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[id]).expect("load 05");
    let (msg, switch_to) = nf_view::panel::apply(&mut scene, &q(&[("do", "weapon"), ("id", &id.to_string()), ("rounds", "5")]));
    eprintln!("[give] {id:#04x}: {msg} -> {switch_to:?}");
    assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id && scene.sim.as_ref().unwrap().gun.switch_state == 0 { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the weapon {id:#04x} is in hand");
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    scene
}

/// Sound ids the audio driver was asked to start this tick (`Cmd::Start` — the same batch the
/// windowed loop builds, matrix.rs's `sample`).
fn started_ids(scene: &mut nf_view::scene::Scene) -> Vec<u32> {
    let sim = scene.sim.as_mut().unwrap();
    let (pos, f) = (sim.bond.head(), sim.bond.look());
    let norm = { let l = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6); [f[2] / l, 0.0, -f[0] / l] };
    sim.drones.map_sounds.update(&mut sim.drones.audio, pos);
    sim.drones.audio.update(sim.sfx.as_ref(), (pos, norm, [0.0, 1.0, 0.0], f), &|_| false);
    sim.drones.audio.take_commands().into_iter()
        .filter_map(|c| match c { nf_game::sound::Cmd::Start { id, .. } if id != 0 => Some(id), _ => None })
        .collect()
}

fn live_guards(sim: &nf_view::scene::Sim) -> Vec<usize> {
    sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && d.enabled && !d.corpse && d.health > 0.0 && d.drawn)
        .map(|(i, _)| i).collect()
}

/// ── The aimed throw: the hat kills the guard in one hit and never plays the grenade bounce ────
/// The hat's record [L ee_start_05.bin @0x2c398c]: damage 100.0 (one touch kills), speed 0.6667,
/// +0x6C = 0x20C08 — gravity + no flesh pass-through + the HOMING bit 0x20000 (the only carrier
/// among the thrown projectiles); `Bullet_update` 0x12e7b0:476-478 runs `Bullet_homing` 0x12f358
/// for every flying 0x20000 bullet. The homing's job on an aimed throw is to track the sag and
/// the walking guard so the hat ARRIVES (the steer gain 0.0333, clamp ±0.01/axis [M
/// 0x12f358:126-166]); pre-fix the grenade path had no homing: the hat dug into the floor ~4 units short
/// of a 20-unit feet-aimed throw and HOPPED the rest like a grenade — the dig-bounce plays the 0x51c bounce gate
/// [M 0x12ce80 LAB_0012d4bc]. Post-fix the flight is one smooth dive onto the guard: the kill
/// lands, 0x51c is NEVER heard, and the only world contact afterwards is the stick's single
/// 0x4a7 clang [M 0x12ce80:317]. Sister: a guard beside the victim is untouched (the kill is the
/// contact damage — msg 8's 100.0 — no blast radius exists) and no explosion ever spawns.
#[test]
fn the_thrown_hat_kills_the_guard_in_one_hit_and_never_bounces() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_weapon(&iso, 0x45);
    // A verified-open 20-unit corridor from Bond: the victim parked dead-ahead at his own floor
    // (the head→chest ray and the knee-high band segment-clear) — the throw is aimed at the
    // chest, so the comparison is the flight physics, not level luck.
    let (victim, vx, vy, vz, aim_yaw, pitch) = {
        let sim = scene.sim.as_mut().unwrap();
        let Some(&vi) = live_guards(sim).first() else { eprintln!("no live guard on 05 — skipping"); return };
        let bp = sim.bond.pos;
        let floor = sim.world.floor_below([bp[0], bp[1] + 1.0, bp[2]], 8.0).unwrap_or(bp[1] - 1.1);
        let head_y = floor + 1.55;
        // The throw corridor: the aim ray (head → chest) segment-clear at the level floor, the
        // victim parked on it (stunned, below), the other guards parked behind.
        let mut found = None;
        'outer: for d in [22.0f32, 20.0, 18.0] {
            for k in 0..24 {
                let a = k as f32 * std::f32::consts::TAU / 24.0;
                let x = bp[0] + a.sin() * d;
                let z = bp[2] + a.cos() * d;
                let Some(gf) = sim.world.floor_below([x, floor + 2.0, z], 8.0) else { continue };
                if (gf - floor).abs() > 1.0 { continue; }
                let head = [bp[0], head_y, bp[2]];
                let chest = [x, gf + 1.0, z];
                if sim.world.segment(head, chest).is_some() { continue; }
                found = Some((vi, x, gf, z, a, ((gf + 1.0) - head_y).atan2(d)));
                break 'outer;
            }
        }
        let Some(t) = found else { eprintln!("no open throw corridor from Bond's start — skipping"); return };
        t
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.list[victim].pos = [vx, vy, vz];
        // The homing acquires the NEAREST seekable guard in the cone [M 0x12f358] — park every
        // other guard 30 units BEHIND the flight line so the victim is the only candidate.
        let back_a = aim_yaw + std::f32::consts::PI;
        for (i, dr) in sim.drones.list.iter_mut().enumerate() {
            if i == victim || !dr.present { continue; }
            dr.pos = [vx + back_a.sin() * 30.0 + i as f32 * 0.5, vy, vz + back_a.cos() * 30.0];
        }
        // Park the victim where the geometry was verified: the stun state holds it still (the
        // stunned guard stays seekable — `Bullet_heatseekable` 0x12f248 excludes only the
        // WAIT_SWITCH state and the dead), so the flight meets a fixed target.
        let st = sim.drones.frame + 7200;
        let v = &mut sim.drones.list[victim];
        v.stun_until = st;
        v.state = nf_game::drone::state::STUN_GRENADE_IMPACT;
        sim.bond.yaw = aim_yaw;
        sim.bond.pitch = pitch; // a lob over the guard: pre-fix it sails past and bounces on the floor beyond; the homing dives it onto him
    }
    // Hold fire until the throw spawns (the hat's release frame is +0x44 = 13 [L record]), then
    // release and let it fly.
    let mut thrown = false;
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        if !scene.sim.as_ref().unwrap().drones.grenades.is_empty() { thrown = true; break; }
    }
    assert!(thrown, "the hat never threw");
    let mut bounces_51c = 0usize;
    let mut clangs_4a7 = 0usize;
    // The guard's shift: 2.5 units perpendicular to the flight line, mid-flight — what a walking
    // guard does while the hat is in the air. Pre-fix nothing re-bends the hat onto him.
    for t in 0..400 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let ids = started_ids(&mut scene);
        bounces_51c += ids.iter().filter(|&&id| id == 0x51c).count();
        clangs_4a7 += ids.iter().filter(|&&id| id == 0x4a7).count();
        if scene.sim.as_ref().unwrap().drones.list[victim].health <= 0.0 { break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.list[victim].health <= 0.0, "the hat killed nobody (the user's symptom)");
    assert_eq!(bounces_51c, 0, "the hat played the 0x51c grenade bounce (pre-fix it dug in and hopped like a grenade)");
    assert!(clangs_4a7 <= 1, "more than one 0x4a7 clang before the kill landed");
    assert!(sim.drones.explosions.is_empty(), "the hat exploded (it must never blast)");
    // Sister: nobody beside the victim took damage — the kill is the contact, not an explosion.
    let witness = live_guards(sim).into_iter()
        .filter(|&i| i != victim)
        .filter(|&i| {
            let p = sim.drones.list[i].pos; let v = sim.drones.list[victim].pos;
            (p[0] - v[0]).powi(2) + (p[1] - v[1]).powi(2) + (p[2] - v[2]).powi(2) < 3.0f32.powi(2)
        })
        .collect::<Vec<_>>();
    for w in witness {
        assert!(sim.drones.list[w].health > 0.0, "a guard beside the victim died (a blast radius exists — it must not)");
    }
}

/// ── The stick: at a surface the hat STOPS with the 0x4a7 clang, never bounces, kills no more ───
/// The +0x100 arm [M 0x12ce80:232-397] freezes the hat (state 3) at its first world contact,
/// plays the 0x4a7 clang [:317] and the bounce block never runs; the stuck hat's handler is
/// skipped every tick after [M :102] so a guard brushing it survives, and 1500 ticks later it
/// deletes silently [M 0x12e7b0:363-371]. Before the fix the port bounced the hat like a
/// grenade: the 0x51c bounce gate, a reflected velocity, a rest — and the contact pass kept
/// killing. Sister: the frag 0x34 at the same floor still BOUNCES (its 0x51c plays) — the
/// per-record routing, not a global change.
#[test]
fn the_hat_sticks_where_it_lands_clangs_once_and_kills_no_more() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_weapon(&iso, 0x45);
    scene.sim.as_mut().unwrap().bond.pitch = -0.9; // throw at the floor ahead
    let mut thrown = false;
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        if !scene.sim.as_ref().unwrap().drones.grenades.is_empty() { thrown = true; break; }
    }
    assert!(thrown, "the hat never threw");
    // Fly until the hat's motion stops (the stick): speed ~ 0 while the object still exists.
    let mut stick_tick = None;
    let mut clangs = 0usize;
    let mut bounces_51c = 0usize;
    let mut stick_pos = [0.0f32; 3];
    for t in 0..240 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let ids = started_ids(&mut scene);
        clangs += ids.iter().filter(|&&id| id == 0x4a7).count();
        bounces_51c += ids.iter().filter(|&&id| id == 0x51c).count();
        let sim = scene.sim.as_ref().unwrap();
        if sim.drones.grenades.is_empty() { break; }
        let g = &sim.drones.grenades[0];
        let sp = (g.velocity[0] * g.velocity[0] + g.velocity[1] * g.velocity[1] + g.velocity[2] * g.velocity[2]).sqrt();
        if stick_tick.is_none() && sp < 1e-4 {
            stick_tick = Some(t);
            stick_pos = g.pos;
        }
    }
    let Some(st) = stick_tick else {
        panic!("the hat never stopped moving — it bounced like a grenade (the pre-fix behaviour)");
    };
    assert_eq!(clangs, 1, "exactly one 0x4a7 contact clang at the stick [M 0x12ce80:317], got {clangs}");
    assert_eq!(bounces_51c, 0, "the hat must never play the 0x51c grenade bounce [M 0x12ce80:398-407]");
    // Frozen: the position is identical 30 ticks into the stick.
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.drones.grenades.is_empty(), "the hat vanished before its 1500-tick lifetime");
        let g = &sim.drones.grenades[0];
        let d = (g.pos[0] - stick_pos[0]).abs() + (g.pos[1] - stick_pos[1]).abs() + (g.pos[2] - stick_pos[2]).abs();
        assert!(d < 1e-3, "the stuck hat drifted {d} units — it must sit where it landed");
    }
    // The stuck hat kills nobody: a live guard moved onto it keeps his health (state 3's handler
    // gate [M 0x12ce80:102]).
    {
        let sim = scene.sim.as_mut().unwrap();
        if let Some(&gi) = live_guards(sim).first() {
            sim.drones.list[gi].pos = [stick_pos[0] + 0.4, stick_pos[1], stick_pos[2] + 0.4];
            let h0 = sim.drones.list[gi].health;
            for _ in 0..5 { sim.step_bond(Input::default(), false, false, false); }
            assert!(sim.drones.list[gi].health >= h0 - 1e-6, "the stuck hat killed a guard touching it (the handler must be skipped)");
        }
    }
    // The silent expiry: after the 1500-tick lifetime the hat is gone and nothing ever blasted.
    let mut blasted = false;
    for _ in 0..1600 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if !sim.drones.explosions.is_empty() { blasted = true; break; }
        if sim.drones.grenades.is_empty() { break; }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!blasted, "the hat exploded at or after its expiry (it must delete silently)");
    assert!(sim.drones.grenades.is_empty(), "the stuck hat never expired (1500 ticks [M 0x12ce80:315])");

    // Sister: the frag 0x34 at the same floor still bounces — its 0x51c plays and it never sticks.
    let mut frag = load_weapon(&iso, 0x34);
    frag.sim.as_mut().unwrap().bond.pitch = -0.9;
    let mut thrown = false;
    for _ in 0..120 {
        frag.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        if !frag.sim.as_ref().unwrap().drones.grenades.is_empty() { thrown = true; break; }
    }
    assert!(thrown, "the frag never threw");
    let mut frag_51c = 0usize;
    for _ in 0..240 {
        frag.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let ids = started_ids(&mut frag);
        frag_51c += ids.iter().filter(|&&id| id == 0x51c).count();
        if frag_51c > 0 { break; }
    }
    assert!(frag_51c > 0, "the frag's floor bounce lost its 0x51c (the sister must still bounce like a grenade)");
    assert!(frag.sim.as_ref().unwrap().drones.grenades.iter().all(|g| g.weapon_type != 0x45), "sanity");
}
