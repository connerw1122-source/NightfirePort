//! walkreal-b 0700000d probe: the doors, the yard web, the trench (the findings cited in
//! walkreal_0700000d_live.rs).

#[path = "common/walker.rs"]
mod walker;
use walker::Walker;

#[test]
fn probe_0d_route() {
    let iso = std::env::var("NF_ISO").unwrap();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "0700000d.bin", false, None).unwrap());
    w.wait_enabled(3000, "the intro");
    w.wait(120);
    let sim = w.scene.sim.as_ref().unwrap();
    eprintln!("[p0d] bond {:?}", sim.bond.pos);
    // The doors (the yard gates, the crawl doors, the catwalk doors).
    for d in &sim.drones.doors.list {
        eprintln!("[p0d] door at ({:.1},{:.1},{:.1}) pos01 {:.2} ch_open {} ch_lock {}",
            d.pos[0], d.pos[1], d.pos[2], d.pos01, d.ch_open, d.ch_lock);
    }
    // The dock's south drop and the under-dock trench: the floors south of the start.
    for z in [-206.0f32, -210.0, -216.0, -246.0, -266.0, -272.0] {
        let mut row = String::new();
        for x in [136.0f32, 152.0, 158.0, 164.0, 176.0, 180.0] {
            let f = sim.world.floor_below([x, 14.0, z], 12.0);
            row.push_str(&format!("{}:{}, ", x as i32, f.map(|v| (v * 10.0) as i32).unwrap_or(-1)));
        }
        eprintln!("[p0d] z {z}: {row}");
    }
    // The trench's far side: the yard's north edge rises 3.9.
    for x in [152.0f32, 158.0, 164.0] {
        let mut col = Vec::new();
        let mut top = 6.0f32;
        for _ in 0..8 {
            match sim.world.floor_below([x, top - 0.05, -268.0], 7.0) {
                Some(f) if f < top - 0.3 => { col.push((f * 10.0) as i32); top = f; }
                _ => { top -= 6.5; if top < 0.0 { break; } }
            }
        }
        eprintln!("[p0d] the trench x{x} at z-268: {col:?}");
    }
    // The ST-3 yard web (the nav path nodes at y 4.4).
    for (pi, path) in sim.nav.paths.iter().enumerate() {
        let ys: Vec<i32> = path.nodes.iter().map(|n| (n.pos[1] * 10.0) as i32).collect();
        let ymin = ys.iter().min().unwrap_or(&0);
        let ymax = ys.iter().max().unwrap_or(&0);
        eprintln!("[p0d] nav path {pi}: {} nodes y {:?}..{:?}", path.nodes.len(),
            *ymin as f32 / 10.0, *ymax as f32 / 10.0);
    }
}
