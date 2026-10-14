//! probe 2: the 01 lead with guards removed — pure AllyLead root-motion rate and where it halts
use nf_view::scene::Scene;

#[test]
fn probe_lead_clean() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let mut sc: Scene = match nf_view::load(&iso, "07000001.bin", false) { Ok(s) => s, Err(e) => panic!("{e}") };
    let sim = sc.sim.as_mut().unwrap();
    let (start, yaw) = sc.start.unwrap_or(([0.0, 2.0, 0.0], 0.0));
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, start, yaw);
    for t in 0..4000u32 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.channels.on(0xb) { break; }
    }
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); }
    let ci = sim.drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("contact");
    // Remove every armed started guard (team 1) far away — the lead runs clean.
    for i in 0..sim.drones.list.len() {
        if i != ci && sim.drones.list[i].team == 1 && sim.drones.list[i].started && !sim.drones.list[i].corpse {
            sim.drones.list[i].pos = [500.0 + i as f32, 50.0, 500.0];
        }
    }
    let plate = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::TOUCH && t.out == 0x10).map(|t| t.centre()).unwrap();
    eprintln!("[probe2] plate centre {:?}", plate);
    let mut last = sim.drones.list[ci].pos;
    let mut stuck_at: Option<u32> = None;
    for t in 0..24000u32 {
        // Bond follows 2.0 behind, facing the contact (keeps him drawn + lead band).
        let cp = sim.drones.list[ci].pos;
        let d2 = ((cp[0] - sim.bond.pos[0]).powi(2) + (cp[2] - sim.bond.pos[2]).powi(2)).sqrt();
        if d2 > 4.0 {
            let (dx, dz) = (cp[0] - sim.bond.pos[0], cp[2] - sim.bond.pos[2]);
            let l = dx.hypot(dz).max(1e-3);
            sim.bond.pos = [cp[0] - dx / l * 3.0, cp[1], cp[2] - dz / l * 3.0];
            sim.bond.yaw = (cp[0] - sim.bond.pos[0]).atan2(cp[2] - sim.bond.pos[2]);
        }
        sim.step_bond(Default::default(), false, false, false);
        if t % 1000 == 0 {
            let d = &sim.drones.list[ci];
            let moved = ((d.pos[0] - last[0]).powi(2) + (d.pos[2] - last[2]).powi(2)).sqrt();
            eprintln!("[probe2] t{t}: pos ({:.2},{:.2},{:.2}) state {:#x} ftime {:.1} drawn {} route {:?} target ({:.1},{:.1},{:.1}) moved1000 {:.2} ch16 {} exit {:?}",
                d.pos[0], d.pos[1], d.pos[2], d.state, d.anim.ftime, d.drawn,
                d.route.as_ref().map(|r| (r.index, r.nodes.len())),
                d.route.as_ref().map(|r| r.target).unwrap_or([0.0;3])[0],
                d.route.as_ref().map(|r| r.target).unwrap_or([0.0;3])[1],
                d.route.as_ref().map(|r| r.target).unwrap_or([0.0;3])[2],
                moved, sim.drones.channels.on(0x10), sim.drones.level_exit);
            last = d.pos;
            if moved < 0.3 && stuck_at.is_none() && t > 1000 { stuck_at = Some(t); }
        }
        if sim.drones.level_exit.is_some() { eprintln!("[probe2] LEVEL EXIT at t{t}: {:?}", sim.drones.level_exit); break; }
    }
    let d = &sc.sim.as_ref().unwrap().drones.list[ci];
    eprintln!("[probe2] final: pos ({:.2},{:.2},{:.2}) state {:#x} stuck_at {stuck_at:?} exit {:?}", d.pos[0], d.pos[1], d.pos[2], d.state, sc.sim.as_ref().unwrap().drones.level_exit);
}
