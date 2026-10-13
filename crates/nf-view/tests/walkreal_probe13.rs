//! walkreal-b2 probe v2: run the real warm chain (12 → 4A → 13 legs) and at the entrance pin
//! dump the drones/objects/segments there. Diagnostic — run with --nocapture.

#[path = "common/walker_b2.rs"]
mod walker;
#[path = "common/tower2.rs"]
mod tower2;

use walker::{Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn probe_13_entrance_warm() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000012.bin", false, None).unwrap());
    tower2::walk_12(&mut w);
    tower2::walk_12_exit(&mut w);
    let save = {
        let sim = w.scene.sim.as_ref().unwrap();
        nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request,
            &sim.drones.inventory, &sim.drones.channels).with_mission(&sim.drones.mission)
    };
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "0700004a.bin", false, Some(&save)).unwrap());
    w.wait_enabled(3000, "the 4A intro");
    for _ in 0..600 { w.tick(nf_game::player::Input::default(), false); }
    let save = {
        let sim = w.scene.sim.as_ref().unwrap();
        nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request,
            &sim.drones.inventory, &sim.drones.channels).with_mission(&sim.drones.mission)
    };
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000013.bin", false, Some(&save)).unwrap());
    w.wait_enabled(3000, "the 13 intro");
    w.wait(120);
    w.walk_to(Wp::new(5.0, -368.4, 26.5).r(1.5), "13: the lobby east lane");
    w.walk_to(Wp::new(3.0, -368.4, 45.0).r(1.5), "13: the lobby mid");
    w.walk_to(Wp::new(-6.0, -368.2, 60.0).r(1.5), "13: the lobby south");
    // Walk toward the entrance; when progress stalls, dump.
    let target = [-6.2f32, -368.0, 66.3];
    let mut last_z = 100.0;
    for tick in 0..3000 {
        let sim = w.scene.sim.as_ref().unwrap();
        let bp = sim.bond.pos;
        let dz = target[2] - bp[2];
        let dx = target[0] - bp[0];
        let dist = (dx * dx + dz * dz).sqrt();
        if dist < 0.8 { eprintln!("[probe] reached the entrance at tick {tick}"); break; }
        let desired = dx.atan2(dz);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let input = nf_game::player::Input { turn: err.clamp(-0.05, 0.05), forward: 1.0, ..Default::default() };
        w.tick(input, false);
        if tick % 120 == 0 || (bp[2] - last_z).abs() < 0.001 && tick > 600 {
            let sim = w.scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            eprintln!("tick {tick}: bond=({:.2},{:.2},{:.2})", bp[0], bp[1], bp[2]);
            for (i, d) in sim.drones.list.iter().enumerate() {
                if walker::dxz(d.pos, bp) < 4.0 && !d.corpse {
                    eprintln!("  drone[{i}] skin={:#x} pos=({:.2},{:.2},{:.2}) state={:#x} started={} team={}",
                        d.skin, d.pos[0], d.pos[1], d.pos[2], d.state, d.started, d.team);
                }
            }
            if let Some((t, surf)) = sim.world.segment_hit(bp, target) {
                eprintln!("  seg→target: t={t:.3} surf=({:.2},{:.2},{:.2})",
                    surf[0], surf[1], surf[2]);
            } else { eprintln!("  seg→target: clear"); }
            last_z = bp[2];
            if tick > 2400 { break; }
        }
    }
}
