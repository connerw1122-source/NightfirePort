//! fix-impacts + fix-impacts-render live tests (NF_ISO, 07000005): a shot at a wall leaves one
//! decal at the hit with the wall's normal and material, and the render side draws it with the
//! game's own mesh — the scene's Effect-instance pool carries the `EffectInfo+4` mesh hash of the
//! hit material and the per-frame matrix sits at the stored decal pose (`Mat_Align2Dir` 0x1df2a8,
//! `View_DrawObjectGlist` 0x1e6780); a shot into the sky leaves none; each shot ejects one casing
//! whose draw slot carries the P2K's `ammo_data+4` mesh and tracks the integrated casing pose;
//! debris draws its `EffectInfo+0x74` mesh at the particle pose/scale; the same seeded run
//! reproduces the same decals and RNG state bit for bit. G-041 / G-025; docs/atlas/effect.md.

use nf_game::player::Input;
use nf_view::scene::ViewOpts;

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
            if let Some((t, _, _)) = sim.data().world.segment_surface(eye0, end, false) {
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
        let mat = sim.data().world.segment_surface(eye0, end, false).map(|(_, _, m)| m).unwrap_or(0);
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
    if let Some((t, n2, m2)) = d.world.segment_surface(front, back, false) {
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
    // fix-impacts-render: the draw is the game's own mesh. The scene's Effect-instance pool for the
    // decal's `EffectInfo+4` mesh hash exists (`hashtable_set_sub_object_to_entity_gfx(node,
    // EffectInfo[type]+4)` 0x1ce9c8), and the frame matrix of the new decal's slot (the last FIFO
    // index) sits at the stored pose — `Mat_Align2Dir` 0x1df2a8 rotation, translation `hit +
    // normal·0.01` (`View_DrawObjectGlist` 0x1e6780 draws node+0x10's glist at node+0x20).
    let (decal_att, debris_att, debris) = {
        let sim = scene.sim.as_ref().unwrap();
        let slot = sim.drones.effects.decals.len() - 1;
        let att = scene.effect_attachment(0, slot, dec.mesh)
            .expect("the decal mesh's draw slots are in the scene");
        // The debris pool: `Scene::build` instances every `EffectInfo+0x74` hash the level archive
        // resolves — 07000005 resolves all eight [D probe]. Pick the table's first non-zero one
        // (static data, deterministic) and require its slots; track a live particle's pose below
        // only when this wall's material spawned any (debris are gated per material [+0x82]).
        let debris_mesh = sim.data().surfaces.and_then(|s| s.effects.iter().map(|r| r.debris_mesh).find(|m| *m != 0)).unwrap_or(0);
        assert_ne!(debris_mesh, 0, "EffectInfo carries a debris mesh");
        let datt = scene.effect_attachment(2, 0, debris_mesh)
            .expect("the debris mesh's draw slots are in the scene");
        (att, datt, sim.drones.effects.debris.first().cloned())
    };
    let fs = scene.frame_state(None, 0.0, ViewOpts::default());
    let m = fs.attach[decal_att as usize];
    assert!((m[3][0] - dec.pos[0]).abs() < 0.02 && (m[3][1] - dec.pos[1]).abs() < 0.02 && (m[3][2] - dec.pos[2]).abs() < 0.02,
        "the decal slot's matrix sits at the stored pose, got {:?} want {:?}", &m[3][..3], &dec.pos);
    // The matrix rows carry the surface orientation: row 2 (the mesh's +Z) is the surface normal.
    let dot = m[2][0] * dec.normal[0] + m[2][1] * dec.normal[1] + m[2][2] * dec.normal[2];
    assert!(dot > 0.99, "the decal slot's +Z row is the surface normal (dot {dot})");
    // Sister (debris): the debris mesh's slot matrix tracks the particle pose while it lives
    // (`Debris_Update` 0x1aac00 ticks it; the draw is `obj+0xD8` at the scaled `obj+0x90`).
    if let Some(d) = debris.filter(|_| fs.attach[debris_att as usize][3][1] > -9999.0) {
        let dm = fs.attach[debris_att as usize];
        let near = (dm[3][0] - d.pos[0]).abs() < 0.3 && (dm[3][1] - d.pos[1]).abs() < 0.3 && (dm[3][2] - d.pos[2]).abs() < 0.3;
        let far = dm[3][1] <= -9999.0;
        assert!(near || far, "the debris slot is at the particle pose or collapsed (dead), got {:?}", &dm[3][..3]);
        // The draw scale is `Debris_Create`'s 0.4 + FRand(0.1) [M 0x1aaad8] — the matrix rows carry it.
        if near {
            let s0 = (dm[0][0] * dm[0][0] + dm[0][1] * dm[0][1] + dm[0][2] * dm[0][2]).sqrt();
            assert!((s0 - d.scale).abs() < 0.02, "the debris slot is scaled by obj+0xe8, got {s0} want {}", d.scale);
        }
    }
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
}

/// fix-impacts-render: the casing draw slot carries the shooter's `ammo_data+4` mesh (the P2K is
/// weapon 2, ammo byte +0x90 → `casing_mesh[1]`) and its per-frame matrix tracks the sim's
/// integrated casing pose while it flies (`Bullet_casing_update` 0x12faa0's pos + euler spin).
#[test]
fn the_casing_draw_slot_carries_the_p2k_mesh_and_tracks_the_pose() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).unwrap();
    let (casing, att) = {
        let sim = scene.sim.as_mut().unwrap();
        // The P2K's expected casing mesh straight from the tables the sim consults.
        let w = sim.gun.weapon as usize;
        let ammo_byte = sim.weapons.records[w][0x90] as usize;
        let want_mesh = sim.data().surfaces.as_ref().map(|s| s.ammo.casing_mesh[ammo_byte]).unwrap_or(0);
        assert_ne!(want_mesh, 0, "the P2K's ammo record has a casing mesh");
        sim.bond.yaw = 0.0;
        sim.bond.pitch = 0.0;
        sim.bond.eye = 0.0;
        sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        // Fly until the eject's armed delay runs out (P2K: 0) and the casing flies.
        let mut c = None;
        for _ in 0..30 {
            sim.step_bond(Input::default(), false, false, false);
            if let Some(x) = sim.drones.casings.list.iter().find(|c| c.state == 0 && c.mesh == want_mesh) { c = Some(x.clone()); break; }
        }
        let c = c.expect("the P2K's casing flies with the ammo_data mesh");
        let slot = sim.drones.casings.list.iter().position(|x| x.mesh == want_mesh).unwrap();
        let att = scene.effect_attachment(1, slot, want_mesh)
            .expect("the casing mesh's draw slots are in the scene");
        (c, att)
    };
    let fs = scene.frame_state(None, 0.0, ViewOpts::default());
    let m = fs.attach[att as usize];
    assert!((m[3][0] - casing.pos[0]).abs() < 0.05 && (m[3][1] - casing.pos[1]).abs() < 0.05 && (m[3][2] - casing.pos[2]).abs() < 0.05,
        "the casing slot's matrix tracks the integrated pose, got {:?} want {:?}", &m[3][..3], &casing.pos);
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
