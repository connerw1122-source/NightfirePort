//! probe: 0a exit trigger geometry vs the use sphere
use nf_view::scene::Scene;
#[test]
fn probe_0a_exit_geom() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let mut sc: Scene = match nf_view::load(&iso, "0700000a.bin", false) { Ok(s) => s, Err(e) => panic!("{e}") };
    let sim = sc.sim.as_mut().unwrap();
    let (start, yaw) = sc.start.unwrap_or(([0.0, 2.0, 0.0], 0.0));
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, start, yaw);
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    let exit = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.next_level == 0x0700_000b).expect("exit").clone();
    eprintln!("[0a] exit lo {:?} hi {:?} tris {} centre {:?}", exit.lo, exit.hi, exit.tris.len(), exit.centre());
    for t in exit.tris.iter().take(12) { eprintln!("[0a]   tri {:?}", t); }
    let c = [(exit.lo[0] + exit.hi[0]) * 0.5, exit.lo[1] + 0.1, (exit.lo[2] + exit.hi[2]) * 0.5];
    sim.bond = nf_game::player::Bond::new(c, 0.0);
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    let bp = sim.bond.pos;
    let head = [bp[0], bp[1] + sim.bond.eye, bp[2]];
    eprintln!("[0a] bond at {:?} eye {:.2} head {:?}", bp, sim.bond.eye, head);
    eprintln!("[0a] floor at bond: {:?}", sim.world.floor_below(bp, 6.0));
    eprintln!("[0a] spawn was {:?}", start);
    for &(x, z) in &[(21.7f32, 18.5), (21.7, 21.0), (20.5, 19.8), (23.0, 19.8), (21.7, 17.0), (24.0, 19.8), (19.5, 19.8), (25.0, 19.8), (21.7, 22.5)] {
        for y in [5.0f32, 2.5, 0.5] {
            if let Some(f) = sim.world.floor_below([x, y, z], 6.0) {
                eprintln!("[0a] floor near ({x},{z}) from y{y}: {f:.2}");
                break;
            }
        }
    }
}
