//! helicopter-live: the helicopter actor through the real paths (nf_view::load, step_bond) —
//! the framelist flight (07000005 #241, gate 0 [D the placement]), the cone spot + the attack
//! (07000012 #707, gate ch 37 [D]), the destruction through the game's damage (0700000b #328 —
//! the case-6 explode arm + ch 0x66 [M Copter_Update 0x17dbb8 cases 4/5/6 + the level switch]).
//! Evidence in docs/systems/helicopter.md; the probes are tests/probe_helicopter*.rs.

use nf_game::player::Input;
use nf_game::world::V3;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn wait(sim: &mut nf_view::scene::Sim, max: u32, pred: impl Fn(&nf_view::scene::Sim) -> bool) -> bool {
    for _ in 0..max {
        sim.step_bond(Input::default(), false, false, false);
        if pred(sim) { return true; }
    }
    pred(sim)
}

/// Aim Bond's view at `target` through the input and tap the trigger — the real fire path
/// (the walker_b2::shoot pattern, aimed at a copter part instead of a drone torso).
fn shoot_at(sim: &mut nf_view::scene::Sim, target: V3, t: u32) {
    let (bp, eye) = (sim.bond.pos, sim.bond.eye - sim.bond.crouch_drop);
    let dx = target[0] - bp[0];
    let dz = target[2] - bp[2];
    let dy = target[1] - (bp[1] + eye);
    let desired = dx.atan2(dz);
    let err = {
        let mut e = desired - sim.bond.yaw;
        while e > std::f32::consts::PI { e -= std::f32::consts::TAU; }
        while e < -std::f32::consts::PI { e += std::f32::consts::TAU; }
        e
    };
    let want_pitch = (dy / ((dx * dx + dz * dz).sqrt().max(0.5))).atan() / std::f32::consts::FRAC_PI_2;
    let perr = want_pitch - sim.bond.pitch;
    // The body sphere is ~8 units [D the probe's part radius] — a loose window still hits it.
    let on_target = err.abs() < 0.28 && perr.abs() < 0.28 && t % 3 == 0;
    let input = Input { turn: err.clamp(-0.08, 0.08), look: perr.clamp(-0.08, 0.08), ..Default::default() };
    sim.step_bond(input, on_target, false, false);
}

/// The copter's live body-part position (the flight script's stream-2 entity — the body,
/// `Copter_GetBody` 0x17e4b8's parts[2]) falling back to the record matrix.
fn copter_body_pos(sim: &nf_view::scene::Sim, ci: usize) -> Option<V3> {
    // The body's read sphere rides the record's spline position (the port's documented
    // deviation: the per-part spheres need the script entities' world poses).
    Some(sim.drones.copters[ci].pos())
}

#[test]
fn the_island_patrol_copter_flies_its_framelist() {
    let scene = nf_view::load(&iso(), "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_ref().unwrap();
    // 07000005 map50 #241: kind 0x2F, route id 1, framelist 4 (382 keys from the placement pos),
    // divisor 2000 → dt = 382/2000 per tick, gate 0 (flies from load) [D the probe].
    let ci = sim.drones.copters.iter().position(|c| c.placement == 241).expect("copter #241");
    let c = &sim.drones.copters[ci];
    assert_eq!(c.gate, 0, "gate 0 [D #241 params]");
    assert_eq!(c.out, 75, "out 0x4B [D #241 params]");
    let path = c.path.as_ref().expect("the framelist loaded");
    assert_eq!(path.count, 382, "count = the framelist's key count [D the 0x19 block]");
    assert!((path.dt - 382.0 / 2000.0).abs() < 1e-6, "dt = count/divisor [M 0x17d340:33-41]");
    assert_eq!(path.keys.len(), 382, "the keys parsed [D]");
    // Parked = key 0 = the placement's position [D the block's k0 == the placement pos].
    let k0 = path.keys[0].pos;
    assert!((k0[0] - 95.922).abs() < 0.01 && (k0[1] - 65.2753).abs() < 0.01 && (k0[2] + 50.1759).abs() < 0.01,
        "key 0 = the placement pos [D the probe], got {k0:?}");
    // The spline advance: the body rides the framelist at the record's own playhead [M Copter_Move
    // 0x17d628: t += node+0x0c, Spline_Interp(list, 0, count, t, ...) mode 2]; the playhead rate =
    // dt per tick (the state-0 tick sets state 1 without moving).
    let mut scene = nf_view::load(&iso(), "07000005.bin", false).expect("reload");
    let sim = scene.sim.as_mut().unwrap();
    let start = sim.drones.copters[ci].pos();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let c = &sim.drones.copters[ci];
    let t = c.t;
    let dt = c.path.as_ref().unwrap().dt;
    assert!((t / 599.0 - dt).abs() < 0.01 * dt,
        "the playhead advanced at dt/tick [M 0x17d340:33-41], got t={t} (dt {dt})");
    // The record's matrix is the pose at the PRE-advance playhead (the game interp's t then adds
    // node+0x0c [M 0x17d628: Spline_Interp, then t += dt]).
    let expect = nf_game::door::interp(&c.path.as_ref().unwrap().keys, 0, c.path.as_ref().unwrap().count, t - dt, true)
        .expect("the spline at t");
    let got = c.pos();
    let dist = ((got[0] - expect[3][0]).powi(2) + (got[1] - expect[3][1]).powi(2) + (got[2] - expect[3][2]).powi(2)).sqrt();
    assert!(dist < 0.01, "the body rides the framelist at t={t} (want {:?}, got {got:?})", &expect[3]);
    let flown = ((got[0] - start[0]).powi(2) + (got[1] - start[1]).powi(2) + (got[2] - start[2]).powi(2)).sqrt();
    assert!(flown > 20.0, "the copter has left the pad after 600 ticks (moved {flown})");
}

#[test]
fn the_island_copter_parks_until_ticked_and_0b_parks_until_its_gate() {
    // Sister: before the gate rises the copter does nothing [M 0x17dbb8 head — the gate early return].
    let scene = nf_view::load(&iso(), "0700000b.bin", false).expect("load 0700000b");
    let sim = scene.sim.as_ref().unwrap();
    // 0700000b map50 #328: gate 42 (0x2A), out 102 (0x66), arrival 100, framelist 18 (984 keys),
    // divisor 3500 [D the probe].
    let ci = sim.drones.copters.iter().position(|c| c.placement == 328).expect("copter #328");
    let c = &sim.drones.copters[ci];
    assert_eq!(c.gate, 42, "gate 0x2A [D #328 params]");
    assert_eq!(c.out, 102, "out 0x66 [D #328 params]");
    assert_eq!(c.arrival, 100, "+0x74 = 100 [D #328 params — the wrap-flag arm]");
    assert_eq!(c.path.as_ref().unwrap().count, 984, "count [D the 0x19 block]");
    let parked = c.pos();
    let mut scene = nf_view::load(&iso(), "0700000b.bin", false).expect("reload");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); }
    let c = &sim.drones.copters[ci];
    assert!(!c.started, "the gate is off — the copter never starts [M 0x17dbb8 head]");
    assert!((c.pos()[0] - parked[0]).abs() < 1e-4 && (c.pos()[2] - parked[2]).abs() < 1e-4,
        "parked at the placement until the gate opens");
}

#[test]
fn the_tower2b_hind_spots_and_fires_at_bond_in_its_cone() {
    // 07000012 map38 #707: gate 37 (0x25), the attack Hind (the map3 template: Gun_A, MuzzleFlash_A,
    // RocketPod [D]) — case 1's cone spot → case 2's FireGun/FireMissile [M 0x17dbb8].
    let mut scene = nf_view::load(&iso(), "07000012.bin", false).expect("load 07000012");
    let sim = scene.sim.as_mut().unwrap();
    let ci = sim.drones.copters.iter().position(|c| c.placement == 707).expect("copter #707");
    assert_eq!(sim.drones.copters[ci].gate, 37, "gate 0x25 [D #707 params]");
    assert_eq!(sim.drones.copters[ci].out, 101, "out 0x65 [D #707 params]");
    // The gate ch 37's writer is upstream of this level's on-disc triggers (the level doc carries
    // no 0x25 writer [D docs/levels/07000012.md]) — the documented stand-in write, everything after
    // is the real machine.
    let frame = sim.drones.frame;
    sim.drones.channels.set(37, 1, frame);
    let cpos = sim.drones.copters[ci].pos();
    // Bond into the cone: along the body's facing (the matrix row 2, Mat_GetDir 0x1e3398 [M]),
    // level and clear of the spawn furniture.
    let fwd = { let c = &sim.drones.copters[ci]; [c.matrix[2][0], c.matrix[2][1], c.matrix[2][2]] };
    let spots: Vec<V3> = [-6.0f32, -2.0, 2.0, 6.0].iter().map(|dy| [
        cpos[0] + fwd[0] * 30.0, (cpos[1] + fwd[1] * 30.0 + dy).max(0.5), cpos[2] + fwd[2] * 30.0,
    ]).collect();
    let mut fired = false;
    for spot in &spots {
        sim.bond.pos = *spot;
        sim.bond.vel = [0.0; 3];
        let before = sim.drones.bullets.len();
        let hit = wait(sim, 700, |s| {
            s.drones.bullets.iter().skip(before).any(|b| b.owner == usize::MAX)
        });
        if hit { fired = true; break; }
        eprintln!("[copter] no fire from {spot:?} (state {})", sim.drones.copters[ci].state);
    }
    assert!(fired, "the Hind fires at Bond inside the cone [M 0x17dbb8 case 1→2, 0x17d828/0x17d9f0]");
    // The game's weapon ids [M 0x17d828:86-89 / 0x17d9f0:81-86 — 0x71 gun, 0x6d missile off-0x08].
    assert!(sim.drones.bullets.iter().any(|b| b.owner == usize::MAX && matches!(b.weapon, 0x71 | 0x6d)),
        "the bullets carry the game's copter weapon ids");
    // Sister: outside the cone (behind the body), no fire.
    let mut scene = nf_view::load(&iso(), "07000012.bin", false).expect("reload");
    let sim = scene.sim.as_mut().unwrap();
    let ci = sim.drones.copters.iter().position(|c| c.placement == 707).unwrap();
    sim.drones.channels.set(37, 1, sim.drones.frame);
    let c = &sim.drones.copters[ci];
    let back = [c.pos()[0] - c.matrix[2][0] * 30.0, (c.pos()[1] - 6.0).max(0.5), c.pos()[2] - c.matrix[2][2] * 30.0];
    sim.bond.pos = back;
    sim.bond.vel = [0.0; 3];
    wait(sim, 700, |_| false);
    assert!(!sim.drones.bullets.iter().any(|b| b.owner == usize::MAX),
        "no fire outside the cone [M the cone gate]");
}

#[test]
fn the_tower_copter_is_destroyed_by_the_games_damage() {
    // 0700000b takes the case-6 explode arm [M 0x17e2a8's level switch]; the destruction writes
    // out = 102 (0x66) [M cases 4/5/6]. The damage rides the real bullet pass (the parts' HITDATA
    // sums × the Bits multipliers [L ELF 0x2a37e8]).
    let mut scene = nf_view::load(&iso(), "0700000b.bin", false).expect("load 0700000b");
    let sim = scene.sim.as_mut().unwrap();
    let ci = sim.drones.copters.iter().position(|c| c.placement == 328).expect("copter #328");
    sim.drones.channels.set(42, 1, sim.drones.frame);
    // Bond under the roof deck's copter; wait for the flight script's body part to exist.
    sim.bond.pos = [20.5, 58.0, 24.4]; // the roof deck beside the copter (the deck 57.03 + the root [D walkreal_0700000b])
    sim.bond.vel = [0.0; 3];
    assert!(wait(sim, 400, |s| copter_body_pos(s, ci).is_some()), "the flight script's parts live");
    let hp0 = sim.drones.copters[ci].hp;
    assert!((hp0 - 299.0).abs() < 0.01, "hp = 299 x d1 [M Copter_Create 0x17d340:76-94]");
    // Shoot the body: the rifle's ~30 damage × Bits[2] = 1.0 [L the Bits table] — ~11 hits.
    let mut hit_any = false;
    for t in 0..16000u32 {
        let target = match copter_body_pos(sim, ci) { Some(p) => p, None => break };
        if t % 60 == 0 { // the ammo top-up (the walker's health top-up allowance) — through the inventory
            let wd = sim.weapons.clone();
            let w = sim.gun.weapon;
            sim.drones.inventory.equip_ammo(&wd, w as usize, 200);
        }
        { // ride alongside the flying body every tick, just outside its sphere (no falls)
            let cp = sim.drones.copters[ci].pos();
            sim.bond.pos = [cp[0] + 9.0, cp[1], cp[2]];
            sim.bond.vel = [0.0; 3];
        }
        shoot_at(sim, target, t);
        sim.drones.health.health = 100.0; // the health top-up AFTER the tick (the walker's exact allowance)
        hit_any |= sim.drones.copters[ci].damage > 0.0;
        if t % 400 == 0 {
            let c = &sim.drones.copters[ci];
            eprintln!("[copter] t={t} dmg {:.1}/{:.0} state {} clip {} spare {} bullets {} bond {:?} disabled {} health {:.0} gun {}",
                c.damage, c.hp, c.state, sim.gun.clip, sim.gun.spare, sim.drones.player_bullets.len(), &sim.bond.pos, sim.disabled, sim.drones.health.health, sim.gun.weapon);
        }
        if sim.drones.copters[ci].done { break; }
    }
    let c = &sim.drones.copters[ci];
    assert!(hit_any, "the bullets reached the parts (the real bullet pass)");
    assert!(c.done && c.state == 6, "the case-6 explode arm on 0x0b [M 0x17e2a8], got state {}", c.state);
    assert!(c.damage >= c.hp, "the Bits-weighted damage crossed hp ({:.0}/{:.0})", c.damage, c.hp);
    assert!(sim.drones.channels.on(102), "the destruction writes ch 0x66 [M case 6]");
    // Sister: below the threshold the channel never writes — reload the level, land 3 hits.
    let mut scene = nf_view::load(&iso(), "0700000b.bin", false).expect("reload");
    let sim = scene.sim.as_mut().unwrap();
    let ci = sim.drones.copters.iter().position(|c| c.placement == 328).unwrap();
    sim.drones.channels.set(42, 1, sim.drones.frame);
    sim.bond.pos = [20.5, 58.0, 24.4]; // the roof deck beside the copter (the deck 57.03 + the root [D walkreal_0700000b])
    sim.bond.vel = [0.0; 3];
    wait(sim, 400, |s| copter_body_pos(s, ci).is_some());
    for t in 0..90u32 {
        let target = match copter_body_pos(sim, ci) { Some(p) => p, None => break };
        shoot_at(sim, target, t);
    }
    assert!(sim.drones.copters[ci].damage > 0.0 && sim.drones.copters[ci].damage < sim.drones.copters[ci].hp,
        "3-ish hits stay under hp");
    assert!(!sim.drones.channels.on(102), "no destruction, no ch 0x66 [M the damage gate]");
}
