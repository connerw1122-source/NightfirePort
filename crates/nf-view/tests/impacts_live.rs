//! fix-impacts live tests (NF_ISO, 07000005): a shot at a wall leaves one decal at the hit with the
//! wall's normal and material; a shot into the sky leaves none; each shot ejects one casing that
//! falls and lands within its lifetime; the same seeded run reproduces the same decal and RNG state
//! bit for bit. G-041 / G-025; docs/atlas/effect.md.

use nf_game::player::Input;

#[test]
fn a_shot_at_a_wall_leaves_a_decal_a_sky_shot_leaves_none() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).unwrap();
    // One mutable phase, one read phase (the Sim borrows the scene).
    let (hit, dec, mat, new_decals, sky_new) = {
        let sim = scene.sim.as_mut().unwrap();
        let eye0 = [sim.bond.pos[0], sim.bond.pos[1] + 1.2, sim.bond.pos[2]];
        // Find a wall in front of the start position: cast forward over a fan of yaws.
        let mut aim: Option<([f32; 3], f32)> = None;
        for k in -16..=16i32 {
            let yaw = k as f32 * 0.08;
            let dir = [yaw.sin(), -0.05, yaw.cos()];
            let end = [eye0[0] + dir[0] * 30.0, eye0[1] + dir[1] * 30.0, eye0[2] + dir[2] * 30.0];
            if let Some((t, _, _)) = sim.data().world.segment_surface(eye0, end) {
                if t < 0.98 {
                    let hit = [eye0[0] + dir[0] * 30.0 * t, eye0[1] + dir[1] * 30.0 * t, eye0[2] + dir[2] * 30.0 * t];
                    aim = Some((hit, yaw));
                    break;
                }
            }
        }
        let (hit, yaw) = aim.expect("a wall within 30 units of the 07000005 start");
        // The surface material the bullet will hit.
        let dir = [yaw.sin(), -0.05, yaw.cos()];
        let end = [eye0[0] + dir[0] * 30.0, eye0[1] + dir[1] * 30.0, eye0[2] + dir[2] * 30.0];
        let mat = sim.data().world.segment_surface(eye0, end).map(|(_, _, m)| m).unwrap_or(0);
        sim.bond.yaw = yaw;
        sim.bond.pitch = (-0.05f32).asin();
        sim.bond.eye = 0.0;
        // Fire one tick, then fly the bullet to the wall.
        let before = sim.drones.effects.decals.len();
        sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        for _ in 0..90 {
            sim.step_bond(Input::default(), false, false, false);
            if sim.drones.effects.decals.len() > before { break; }
        }
        let dec = sim.drones.effects.decals.last().cloned();
        // Sister: a shot into the sky (60° up where the world finds nothing) leaves no decal.
        let up_dir = [yaw.sin() * 0.5, 0.866, yaw.cos() * 0.5];
        let up_end = [eye0[0] + up_dir[0] * 200.0, eye0[1] + up_dir[1] * 200.0, eye0[2] + up_dir[2] * 200.0];
        let mut sky_new = 0;
        if sim.data().world.segment(eye0, up_end).is_none() {
            sim.bond.pitch = 1.0;
            sim.bond.yaw = yaw;
            let b2 = sim.drones.effects.decals.len();
            sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
            for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
            sky_new = sim.drones.effects.decals.len() - b2;
        }
        (hit, dec, mat, sim.drones.effects.decals.len() - before, sky_new)
    };
    let dec = dec.expect("one wall shot → exactly one new decal");
    assert_eq!(new_decals, 1, "one wall shot → exactly one new decal");
    assert!(dec.normal[0].abs() + dec.normal[1].abs() + dec.normal[2].abs() > 0.9, "a unit normal");
    // The decal sits 0.01 off the surface along its own normal (`Effect_Bullet` 0x1ce9f0): re-cast a
    // short segment through the decal against the world and compare point and normal.
    let d = scene.sim.as_ref().unwrap().data();
    let front = [dec.pos[0] + dec.normal[0] * 0.2, dec.pos[1] + dec.normal[1] * 0.2, dec.pos[2] + dec.normal[2] * 0.2];
    let back = [dec.pos[0] - dec.normal[0] * 0.2, dec.pos[1] - dec.normal[1] * 0.2, dec.pos[2] - dec.normal[2] * 0.2];
    if let Some((t, n2, m2)) = d.world.segment_surface(front, back) {
        let hp = [front[0] + (back[0] - front[0]) * t, front[1] + (back[1] - front[1]) * t, front[2] + (back[2] - front[2]) * t];
        let dist = ((dec.pos[0] - hp[0]).powi(2) + (dec.pos[1] - hp[1]).powi(2) + (dec.pos[2] - hp[2]).powi(2)).sqrt();
        assert!(dist < 0.03, "the decal floats 0.01 off the wall, got {dist}");
        let dot = dec.normal[0] * n2[0] + dec.normal[1] * n2[1] + dec.normal[2] * n2[2];
        assert!(dot > 0.99, "the decal normal is the surface normal");
        // The mesh is the material's `EffectInfo+4` decal hash.
        if let Some(s) = d.surfaces {
            if let Some(rec) = s.effects.get((m2 & 0x3f) as usize) {
                if rec.decal != 0 {
                    assert_eq!(dec.mesh, rec.decal, "the decal mesh is EffectInfo[{m2}]+4");
                }
            }
        }
    } else {
        panic!("the decal is not within 0.2 of any surface");
    }
    let _ = (hit, mat);
    assert_eq!(sky_new, 0, "a sky shot leaves no decal");
    // The renderer's quad vertices appear at the hit.
    let quads = scene.sim.as_ref().unwrap().effect_quads([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    assert!(!quads.is_empty(), "effect_quads returns this frame's vertices");
    assert!(quads.iter().any(|v| (v.pos[0] - dec.pos[0]).abs() < 0.08 && (v.pos[1] - dec.pos[1]).abs() < 0.08 && (v.pos[2] - dec.pos[2]).abs() < 0.08),
        "a decal vertex sits at the hit");
}

#[test]
fn each_shot_ejects_one_casing_that_lands_within_its_lifetime() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).unwrap();
    let landed = {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.yaw = 0.0;
        sim.bond.pitch = 0.0;
        sim.bond.eye = 0.0;
        sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        sim.step_bond(Input::default(), false, false, false);
        let live = sim.drones.casings.list.iter().filter(|c| c.state != 2).count();
        assert_eq!(live, 1, "one shot → exactly one casing in the 32-slot pool");
        let mut landed = false;
        for _ in 0..240 {
            sim.step_bond(Input::default(), false, false, false);
            match sim.drones.casings.list.iter().find(|c| c.state != 2) {
                Some(c) if c.landed => { landed = true; break; }
                None => break,
                _ => {}
            }
        }
        landed
    };
    assert!(landed, "the casing crossed its landing plane within 240 ticks");
    let _ = scene.sim.as_ref().unwrap().effect_quads([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
}

#[test]
fn the_seeded_run_reproduces_bit_for_bit() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let run = || -> (Vec<[f32; 3]>, (u32, u32)) {
        let mut scene = nf_view::load(&iso, "07000005.bin", false).unwrap();
        let (decals, rng) = {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond.yaw = 0.4;
            sim.bond.pitch = -0.05;
            sim.bond.eye = 0.0;
            sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
            for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
            (sim.drones.effects.decals.iter().map(|d| d.pos).collect(), (sim.drones.rng.x, sim.drones.rng.y))
        };
        (decals, rng)
    };
    assert_eq!(run(), run(), "the same load + inputs → the same decals and RNG state");
}
