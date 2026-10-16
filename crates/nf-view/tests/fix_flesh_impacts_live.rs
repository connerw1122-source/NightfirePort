//! fix-flesh-impacts — a bullet hitting a PERSON must show the game's flesh effect. The game's
//! flow: the bullet's own contact effect is SKIPPED for characters (the anim-coll hit writes
//! HITDATA+0x50 = 0 [M Collide_Jointy 0x1ec180:125/173], and `Effect_Create` 0x1ce688 returns
//! unless `material & 0x3f != 0` [M 0x1ce688:52]) — the flesh visual comes from the VICTIM's own
//! handler: `NDrone2_DealWithObjHit` 0x13fff0 case 5 puffs **Gas_Init(3.0, 6.0, obj+0x20, the hit
//! point, 0, 0, mesh 0x20008bc, kind 5, rgba 0xffffffff, fade, life 0x3c = 60)** at the hit point
//! for every weapon id outside {0x2b, 0x34, 0x35, 0x36, 0x37} whose damage > 0, and the Phoenix
//! Samurai bolt (0x32/0x33) gets **0x200026e at 1.0/16.0** instead [M 0x13fff0:235-256; the life
//! from the disassembly: `sw $v0(=0x3c), ($sp)` at 0x140404/0x140460]. Bond's own hit handler
//! (`Player_DealWithObjHit` 0x194458) has NO gas puff — Bond shows the pain glow, not a contact
//! effect [M: no Gas_Init in 0x194458].

use nf_game::player::Input;

fn load_05() -> Option<nf_view::scene::Scene> {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return None };
    match nf_view::load(&iso, "07000005.bin", false) {
        Ok(s) => Some(s),
        Err(e) => { eprintln!("load 07000005: {e}"); None }
    }
}

fn aim_at(sim: &mut nf_view::scene::Sim, to: [f32; 3]) {
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
}

/// A live guard on 07000005 and a clean firing spot `dist` m in front of him (the guard untouched:
/// the shot comes within a few ticks).
fn guard_and_spot(scene: &mut nf_view::scene::Scene) -> (usize, [f32; 3]) { guard_and_spot_at(scene, 3.5) }

fn guard_and_spot_at(scene: &mut nf_view::scene::Scene, dist: f32) -> (usize, [f32; 3]) {
    let (gi, gpos) = {
        let sim = scene.sim.as_ref().unwrap();
        let (gi, gpos) = sim.drones.list.iter().enumerate()
            .find(|(_, d)| d.present && d.enabled && !d.corpse && d.health > 0.0)
            .map(|(i, d)| (i, d.pos)).expect("07000005 has a live guard");
        (gi, gpos)
    };
    // 3.5 m from the guard toward Bond's start, on the guard's floor height.
    let sim = scene.sim.as_mut().unwrap();
    let eye = sim.bond.pos;
    let d = [gpos[0] - eye[0], 0.0, gpos[2] - eye[2]];
    let l = (d[0] * d[0] + d[2] * d[2]).sqrt().max(1e-6);
    let spot = [gpos[0] - d[0] / l * dist, gpos[1], gpos[2] - d[2] / l * dist];
    sim.bond = nf_game::player::Bond::new(spot, sim.bond.yaw);
    (gi, gpos)
}

/// Fires one P2K round at `point` through the real input path and returns the gas puffs with the
/// blood/bolt meshes that appeared, with the player impacts.
type PuffSnap = (u32, [f32; 3], f32, f32, f32, u8, [f32; 4]);

fn shoot(scene: &mut nf_view::scene::Scene, point: [f32; 3]) -> (Vec<PuffSnap>, Vec<nf_game::drone::PlayerImpact>) {
    let sim = scene.sim.as_mut().unwrap();
    aim_at(sim, point);
    sim.step_bond(Input::default(), true, false, false); // the fire press
    collect(scene, 8)
}

/// Watches the gas pool for `ticks` steps, snapshotting each new blood/energy puff at its FIRST
/// sighting (the puff grows every tick), and collects the impacts (cleared at each tick's head).
fn collect(scene: &mut nf_view::scene::Scene, ticks: u32) -> (Vec<PuffSnap>, Vec<nf_game::drone::PlayerImpact>) {
    let mut out: Vec<(u32, [f32; 3], f32, f32, f32, u8, [f32; 4])> = Vec::new();
    let mut impacts: Vec<nf_game::drone::PlayerImpact> = scene.sim.as_ref().unwrap().drones.player_impacts.clone();
    for _ in 0..ticks {
        let sim = scene.sim.as_mut().unwrap();
        for p in &sim.drones.gas.puffs {
            if matches!(p.mesh, 0x0200_08bc | 0x0200_026e)
                && !out.iter().any(|&(m, q, ..): &(u32, [f32; 3], f32, f32, f32, u8, [f32; 4])| m == p.mesh && (q[0] - p.pos[0]).abs() < 1e-4 && (q[1] - p.pos[1]).abs() < 1e-4 && (q[2] - p.pos[2]).abs() < 1e-4) {
                out.push((p.mesh, p.pos, p.scale, p.scale_rate, p.life, p.kind, p.rgba));
            }
        }
        sim.step_bond(Input::default(), false, false, false);
        impacts.extend(sim.drones.player_impacts.iter().copied());
    }
    (out, impacts)
}

/// The symptom: a P2K BODY shot on a 05 guard leaves no flesh effect — the game puffs
/// `Gas_Init(3.0, 6.0, mesh 0x20008bc, kind 5, white, life 60)` at the hit point
/// [M NDrone2_DealWithObjHit 0x13fff0:245-249]. The port must spawn exactly that: one puff
/// (count 1) of the 0x20008bc mesh, scale 3.0 growing to 6.0 over the 60-tick life, at the hit
/// point the impact reports.
#[test]
fn a_p2k_body_shot_spawns_the_blood_puff_at_the_hit_point() {
    let Some(mut scene) = load_05() else { return };
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (gi, gpos) = guard_and_spot(&mut scene);
    // The torso box centre (part 4): from the live boxes the game's collision uses.
    let point = {
        let sim = scene.sim.as_ref().unwrap();
        let boxes = sim.drones.list[gi].hit_boxes(&sim.data());
        let box4 = boxes.iter().find(|b| b.part == 4).expect("the guard has a torso box");
        let c = box4.m[3];
        [c[0], c[1], c[2]]
    };
    let (puffs, impacts) = shoot(&mut scene, point);
    assert!(impacts.iter().any(|i| matches!(i, nf_game::drone::PlayerImpact::Drone { drone, .. } if *drone == gi)),
        "the body shot must register the guard impact, impacts {impacts:?}");
    let hit_at = impacts.iter().find_map(|i| match i { nf_game::drone::PlayerImpact::Drone { at, .. } => Some(*at), _ => None });
    assert_eq!(puffs.len(), 1, "one blood puff per hit (the game's single Gas_Init), got {puffs:?}");
    let (mesh, pos, scale, scale_rate, life, kind, rgba) = puffs[0];
    assert_eq!(mesh, 0x0200_08bc, "the blood puff mesh [M 0x13fff0:248]");
    if let Some(at) = hit_at {
        let d = ((pos[0] - at[0]).powi(2) + (pos[1] - at[1]).powi(2) + (pos[2] - at[2]).powi(2)).sqrt();
        assert!(d < 0.05, "the puff sits at the hit point (Δ {d})");
    }
    // The puff's parameters: scale 3.0 → 6.0 over the 0x3c = 60-tick life [M 0x140404 sw 0x3c].
    assert!((scale - 3.0).abs() < 1e-4, "scale starts 3.0, got {scale}");
    assert!((scale_rate - 3.0 / 60.0).abs() < 1e-4, "scale_rate = (6−3)/60, got {scale_rate}");
    assert!((life - 60.0).abs() < 1e-3, "life 0x3c = 60 ticks, got {life}");
    assert_eq!(kind, 5, "the game's behaviour kind 5");
    assert!(rgba.iter().all(|&c| (c - 255.0).abs() < 1e-3), "the packed param 0xffffffff (the byte values as floats), rgba {rgba:?}");
    assert!(pos[1] > gpos[1] && pos[1] < gpos[1] + 2.0, "the puff is on the guard's body, pos {pos:?}");
}

/// Sister: every drone hit carries exactly one blood puff — two body shots, two puffs (ids and
/// counts per hit). The game's victim puff is per weapon id with NO body-part branch [M
/// 0x13fff0:235-256: the switch is on `**(u16**)(BU+0x44)` = the weapon record id; the part id
/// HITDATA+0x52 only feeds `NDrone2_DoHitEffects`'s 0x601/0x413 sounds], so a head shot puffs the
/// same mesh — the dedicated head-shot run was tried and the 07000005 doorway geometry blocks the
/// head line at the reachable spots (the ray eats the door frame: World hits above the head box);
/// the main agent's PCSX2 head-shot check is in the report.
#[test]
fn every_drone_hit_carries_exactly_one_blood_puff() {
    let Some(mut scene) = load_05() else { return };
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (gi, _) = guard_and_spot(&mut scene);
    let mut hits = 0usize;
    let mut puffs_seen: Vec<[f32; 3]> = Vec::new();
    for _ in 0..8 {
        for _ in 0..14 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); } // the refire
        let point = {
            let sim = scene.sim.as_ref().unwrap();
            let boxes = sim.drones.list[gi].hit_boxes(&sim.data());
            let b4 = boxes.iter().find(|b| b.part == 4).expect("the guard has a torso box");
            let c = b4.m[3];
            [c[0], c[1], c[2]]
        };
        let (po, im) = shoot(&mut scene, point);
        let drone_hits = im.iter().filter(|i| matches!(i, nf_game::drone::PlayerImpact::Drone { drone, .. } if *drone == gi)).count();
        hits += drone_hits;
        for &(_, pos, ..) in &po {
            if !puffs_seen.iter().any(|q| (q[0] - pos[0]).abs() < 1e-3 && (q[1] - pos[1]).abs() < 1e-3 && (q[2] - pos[2]).abs() < 1e-3) {
                puffs_seen.push(pos);
            }
        }
        if hits >= 3 { break; }
    }
    assert!(hits >= 2, "two body shots should land within the window, hits {hits}");
    assert_eq!(puffs_seen.len(), hits, "exactly one blood puff per drone hit (ids/counts), puffs {puffs_seen:?}");
}

/// Sister: the WALL hit is unchanged — the world path keeps its own EffectInfo puffs/decal and
/// must NOT gain the blood mesh (the game's Effect_Create only sees the wall's own material
/// [M 0x12ce80:192 + 0x1ce688:52]).
#[test]
fn a_wall_hit_is_unchanged_and_spawns_no_blood_puff() {
    let Some(mut scene) = load_05() else { return };
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // A wall whose surface record carries a decal (the world path's own effect): scan the view
    // directions until `segment_surface` resolves a material with a non-zero EffectInfo decal.
    let point = {
        let sim = scene.sim.as_ref().unwrap();
        let e = sim.bond.head();
        let mut found = None;
        for dyaw in [-1.2, -0.8, -0.4, 0.0, 0.4, 0.8, 1.2, 1.6, 2.0, 2.4, 2.8, -1.6, -2.0, -2.4, -2.8] {
            let yaw = sim.bond.yaw + dyaw;
            let dir = [yaw.sin(), 0.0, yaw.cos()];
            for t in [2.0f32, 3.0, 5.0, 8.0, 12.0, 18.0, 25.0] {
                let p = [e[0] + dir[0] * t, e[1], e[2] + dir[2] * t];
                if sim.world.segment(e, p).is_some() {
                    if let Some((_, _, m)) = sim.world.segment_surface(e, p, false) {
                        let has_decal = sim.surfaces.as_ref().and_then(|s| nf_formats::sfx::SurfaceSounds::get(&s.impact, m))
                            .is_some();
                        found = Some([p[0], p[1], p[2]]);
                        if has_decal { break; }
                    }
                    break;
                }
            }
            if found.is_some() { break; }
        }
        found.expect("a wall within reach of Bond's spawn")
    };
    let decals_before = scene.sim.as_ref().unwrap().drones.effects.decals.len();
    let (puffs, impacts) = shoot(&mut scene, point);
    assert!(impacts.iter().any(|i| matches!(i, nf_game::drone::PlayerImpact::World(_))),
        "the wall registers the world impact, impacts {impacts:?} (point {point:?})");
    assert!(!puffs.iter().any(|&(m, ..): &PuffSnap| m == 0x0200_08bc), "no blood puff on a wall hit, got {puffs:?}");
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.effects.decals.len() > decals_before, "the wall decal still spawns (the world path unchanged)");
}

/// Sister: the Phoenix Samurai bolt (0x32/0x33) puffs the energy mesh 0x200026e at 1.0/16.0
/// instead of the blood [M 0x13fff0:235-241]. The bolt is driven through the sim's own bullet
/// queue (the fire path for 0x32 needs the gadget-chain equip; the victim's puff is per weapon id
/// in the same drone-hit arm this task extends).
#[test]
fn the_samurai_bolt_puffs_the_energy_mesh_instead() {
    let Some(mut scene) = load_05() else { return };
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (gi, _gpos) = guard_and_spot(&mut scene);
    {
        let sim = scene.sim.as_mut().unwrap();
        let boxes = sim.drones.list[gi].hit_boxes(&sim.data());
        let b4 = boxes.iter().find(|b| b.part == 4).expect("torso box");
        let e = sim.bond.head();
        let c = b4.m[3];
        let d = [c[0] - e[0], c[1] - e[1], c[2] - e[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let dir = [d[0] / l, d[1] / l, d[2] / l];
        sim.drones.player_bullets.push(nf_game::drone::PlayerBullet {
            pos: e, dir, speed: 60.0, travelled: 0.0, range: 100.0, damage: 10.0, weapon: 0x32,
            first: false, dead: false, bounces: 0, homing: false, target: None, gravity: false, guided_cam: false,
        });
    }
    let mut meshes = Vec::new();
    for _ in 0..8 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        for p in &scene.sim.as_ref().unwrap().drones.gas.puffs {
            if matches!(p.mesh, 0x0200_08bc | 0x0200_026e) { meshes.push(p.mesh); }
        }
    }
    assert!(meshes.contains(&0x0200_026eu32), "the bolt puffs 0x200026e, got {meshes:?}");
    assert!(!meshes.contains(&0x0200_08bc), "the bolt must not puff blood, got {meshes:?}");
}
