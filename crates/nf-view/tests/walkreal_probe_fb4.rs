//! fix-final-blockers probe 5: the 07000016 descent (z −85..−120) — the floor grid at several
//! depths, and the stuck point's contacts. Diagnostic only — run with --nocapture.

#[path = "common/walker.rs"]
mod walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn grid(sim: &nf_view::scene::Sim, x0: f32, x1: f32, z0: f32, z1: f32, y: f32, step: f32, depth: f32, tag: &str) {
    eprintln!("=== {tag}: floor from y {y} depth {depth} (rows z, cols x) ===");
    let mut z = z1;
    while z >= z0 {
        let mut row = format!("z {:7.1}: ", z);
        let mut x = x0;
        while x <= x1 {
            let f = sim.world.floor_below([x, y, z], depth);
            match f {
                Some(fy) if y - fy < 2.5 => row.push_str(&format!("{:5.1}", fy)),
                Some(fy) => row.push_str(&format!("{:5.1}", fy)),
                None => row.push_str("  void"),
            }
            x += step;
        }
        eprintln!("{row}");
        z -= step;
    }
}

#[test]
fn dump_descent() {
    let iso = iso();
    let scene = nf_view::load(&iso, "07000016.bin", false).expect("07000016");
    let sim = scene.sim.as_ref().unwrap();
    grid(&sim, 5.0, 11.0, -104.0, -86.0, 1.0, 1.0, 5.0, "the east ledge x 5..11 (z rows −86..−104)");
    grid(&sim, -2.0, 8.0, -112.0, -84.0, 1.0, 2.0, 5.0, "the trench band x −2..8 (floors within 5 of y 1)");
    // The stuck point's contacts.
    let bond = [2.56f32, -3.33, -102.70];
    let down = sim.world.segment_surface([bond[0], bond[1] - 0.05, bond[2]], [bond[0], bond[1] - 4.0, bond[2]]);
    eprintln!("segment down from (2.56,−3.38,−102.7): {down:?}");
    for z in [-101.0, -102.0, -102.7, -103.5, -104.5, -106.0] {
        let f = sim.world.floor_below([2.56, -2.8, z], 10.0);
        let s = sim.world.segment([2.56, -3.3, z - 0.4], [2.56, -3.3, z - 1.6]);
        eprintln!("z {z:7.1}: floor_below(−2.8) {f:?}  southward-segment(−3.3) {s:?}");
    }
    // Doors [0]'s leaf span and the doorway seals at z −114.2.
    let d0 = &sim.drones.doors.list[0];
    eprintln!("door[0] pos {:?} keys {:?}", d0.pos, d0.keys.as_ref().map(|k| (k.len(), k[0].pos, k[1].pos)));
    sim.world.set_dynamic(&[]);
    for x in [1.4f32, 1.8, 2.2, 2.6, 3.0, 3.4, 3.8] {
        let s = sim.world.segment([x, -2.4, -113.4], [x, -2.4, -115.2]);
        eprintln!("door[0] crossing x {x} y −2.4: {s:?}");
    }
}
