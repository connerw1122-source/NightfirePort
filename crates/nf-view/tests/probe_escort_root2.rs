//! probe 4: per-tick root values of the frozen lead
use nf_game::drone::Data;
use nf_view::scene::Scene;

#[test]
fn probe_root_pertick() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let mut sc: Scene = match nf_view::load(&iso, "07000001.bin", false) { Ok(s) => s, Err(e) => panic!("{e}") };
    let sim = sc.sim.as_mut().unwrap();
    let (start, yaw) = sc.start.unwrap_or(([0.0, 2.0, 0.0], 0.0));
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, start, yaw);
    for t in 0..4000u32 { sim.step_bond(Default::default(), false, false, false); if sim.drones.channels.on(0xb) { break; } }
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); }
    let ci = sim.drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("contact");
    for i in 0..sim.drones.list.len() {
        if i != ci && sim.drones.list[i].team == 1 && sim.drones.list[i].started && !sim.drones.list[i].corpse {
            sim.drones.list[i].pos = [500.0 + i as f32, 50.0, 500.0];
        }
    }
    for t in 0..3000u32 {
        let cp = sim.drones.list[ci].pos;
        let d2 = ((cp[0] - sim.bond.pos[0]).powi(2) + (cp[2] - sim.bond.pos[2]).powi(2)).sqrt();
        if d2 > 4.0 {
            let (dx, dz) = (cp[0] - sim.bond.pos[0], cp[2] - sim.bond.pos[2]);
            let l = dx.hypot(dz).max(1e-3);
            sim.bond.pos = [cp[0] - dx / l * 3.0, cp[1], cp[2] - dz / l * 3.0];
        }
        sim.step_bond(Default::default(), false, false, false);
    }
    // Now the freeze window: log per tick (a fresh Data borrow each tick).
    for t in 0..40u32 {
        let cp = sim.drones.list[ci].pos;
        let d2 = ((cp[0] - sim.bond.pos[0]).powi(2) + (cp[2] - sim.bond.pos[2]).powi(2)).sqrt();
        if d2 > 4.0 {
            let (dx, dz) = (cp[0] - sim.bond.pos[0], cp[2] - sim.bond.pos[2]);
            let l = dx.hypot(dz).max(1e-3);
            sim.bond.pos = [cp[0] - dx / l * 3.0, cp[1], cp[2] - dz / l * 3.0];
        }
        sim.step_bond(Default::default(), false, false, false);
        let (pos, ftime, time, script, danim, speed, yaw) = {
            let d = &sim.drones.list[ci];
            (d.pos, d.anim.ftime, d.anim.time, d.anim.script, d.anim.danim, d.anim.speed, d.yaw)
        };
        let seqs = &sim.seqs;
        let data = Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: seqs, skins: &sim.skins, skeletons: &sim.skeletons,
            nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes,
            sfx: sim.sfx.as_ref(), models: &sim.models, surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: 1,
        };
        let d = &sim.drones.list[ci];
        let sf = d.script_frame(&data);
        let root = sf.and_then(|(q, f)| seqs.get(&q).filter(|s| s.channels.len() >= 3).map(|s| nf_game::anim::root_translation_at(s, f)));
        eprintln!("[rt] t{t}: pos ({:.3},{:.3},{:.3}) ftime {:.2} time {} script {:?} danim {} speed {:.3} yaw {:.2} sf {:?} root {:?}",
            pos[0], pos[1], pos[2], ftime, time, script, danim, speed, yaw, sf, root);
    }
}
