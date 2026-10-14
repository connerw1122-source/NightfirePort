//! probe: the 07000001 contact's AllyLead route — where the chain goes, where it stalls
use nf_view::scene::Scene;

#[test]
fn probe_lead_route() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let mut sc: Scene = match nf_view::load(&iso, "07000001.bin", false) { Ok(s) => s, Err(e) => panic!("{e}") };
    let sim = sc.sim.as_mut().unwrap();
    let (start, yaw) = sc.start.unwrap_or(([0.0, 2.0, 0.0], 0.0));
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, start, yaw);
    // The intro writes ch 0xB (the contact's enable) at ~2119.
    let mut enabled = None;
    for t in 0..4000u32 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.channels.on(0xb) { enabled = Some(t); break; }
    }
    eprintln!("[probe] ch 0xB at tick {:?}", enabled);
    for _ in 0..600 { sim.step_bond(Default::default(), false, false, false); }
    let ci = sim.drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("contact");
    eprintln!("[probe] contact idx {ci}");
    // Walk Bond next to the contact so the lead resumes, then follow him for 6000 ticks, logging.
    for t in 0..6000u32 {
        sim.step_bond(Default::default(), false, false, false);
        if t % 300 == 0 {
            let d = &sim.drones.list[ci];
            eprintln!("[probe] t{t}: contact pos ({:.1},{:.1},{:.1}) state {:#x} anim {:#x} route {:?} blocked {} armed {:.1} bond d {:.1} target {:?}",
                d.pos[0], d.pos[1], d.pos[2], d.state, d.anim.state,
                d.route.as_ref().map(|r| (r.path, r.index, r.nodes.len())), d.route_blocked, d.armed_peer_dist,
                (sim.bond.pos[0] - d.pos[0]).hypot(sim.bond.pos[2] - d.pos[2]), d.route.as_ref().map(|r| r.target));
            // keep Bond near him
            let cp = sim.drones.list[ci].pos;
            sim.bond.pos = [cp[0] + 2.0, cp[1], cp[2]];
        }
    }
    // The route chain dump.
    let sim = sc.sim.as_ref().unwrap();
    let d = &sim.drones.list[ci];
    if let Some(r) = &d.route {
        let p = &sim.nav.paths[r.path];
        eprintln!("[probe] route path {} (kind {:#x}) nodes {} chain {:?}", r.path, p.kind, p.nodes.len(), r.nodes);
        for (k, &n) in r.nodes.iter().enumerate() {
            let nd = &p.nodes[n as usize];
            eprintln!("[probe]   chain[{k}] node {n} pos ({:.1},{:.1},{:.1}) flags {:#x} adj {}", nd.pos[0], nd.pos[1], nd.pos[2], nd.flags, p.adj[n as usize].len());
        }
        eprintln!("[probe] index {} dest {:?}", r.index, r.dest);
    } else { eprintln!("[probe] no route"); }
    // The plate.
    if let Some(t) = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::TOUCH && t.out == 0x10) {
        eprintln!("[probe] plate lo {:?} hi {:?} centre {:?}", t.lo, t.hi, t.centre());
    }
    // The exit.
    for t in sim.drones.triggers.iter().filter(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL) {
        eprintln!("[probe] exit lo {:?} hi {:?} gate {} next 0x{:08x} tris {}", t.lo, t.hi, t.gate, t.next_level, t.tris.len());
    }
}
