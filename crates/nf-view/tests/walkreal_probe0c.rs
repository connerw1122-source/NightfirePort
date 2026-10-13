//! walkreal-b 0700000c probe: the level's walked-route evidence — the stairwell of floors around
//! ST-1, the gate band's gaps, the yard walls (the findings cited in walkreal_0700000c_live.rs).

#[path = "common/walker.rs"]
mod walker;
use walker::Walker;

#[test]
fn probe_0c_route() {
    let iso = std::env::var("NF_ISO").unwrap();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "0700000c.bin", false, None).unwrap());
    w.wait_enabled(3000, "the intro");
    w.wait(120);
    let sim = w.scene.sim.as_ref().unwrap();
    // The upper walkway's holes (the skylights at z -140/-125 span x 2..10) and the south wall.
    for z in [-140.0f32, -125.0] {
        let mut row = String::new();
        for x in [-2.0f32, 0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0] {
            let f = sim.world.floor_below([x, 15.5, z], 4.0);
            row.push_str(&format!("{}:{}, ", x as i32, f.map(|v| (v * 10.0) as i32).unwrap_or(-1)));
        }
        eprintln!("[p0c] z {z}: {row}");
    }
    let mut row = String::new();
    for x in [-2.0f32, 0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0] {
        let hit = sim.world.segment([x, 14.2, -152.6], [x, 14.2, -154.5]);
        row.push_str(&format!("{}:{}, ", x as i32, hit.is_some()));
    }
    eprintln!("[p0c] the south wall y14.2: {row}");
    // The display case's extent (west rays at the plate band's z).
    for z in [-154.4f32, -154.6, -154.8, -155.0, -155.2] {
        let hit = sim.world.segment([7.4, 12.2, z], [5.0, 12.2, z]);
        eprintln!("[p0c] the case west ray z{z}: blocked {}", hit.is_some());
    }
    // The yard crate wall and the ladder.
    for z in [-176.0f32, -178.0, -180.0, -182.0, -184.0, -186.0] {
        let hit = sim.world.segment([46.0, 1.5, z], [20.0, 1.5, z]);
        eprintln!("[p0c] the yard west ray z{z}: blocked {}", hit.is_some());
    }
    for (c, top, bottom) in walker::ladders(sim) {
        eprintln!("[p0c] ladder ({:.1},{:.1},{:.1}) top {:.1} bottom {:.1}", c[0], c[1], c[2], top, bottom);
    }
    // The gate band's pieces (the pit x 59-70 and the checkerboard east).
    for x in [52.0f32, 56.0, 60.0, 64.0, 78.0, 82.0, 86.0, 90.0, 96.0] {
        let mut r2 = String::new();
        for z in [-207.5f32, -208.8, -209.6, -210.4] {
            let f = sim.world.floor_below([x, 13.5, z], 8.0);
            r2.push_str(&format!("{}:{}, ", z as i32, f.map(|v| (v * 10.0) as i32).unwrap_or(-1)));
        }
        eprintln!("[p0c] band x {x}: {r2}");
    }
    // The ch 0x40 band volumes and the winches' level: the triggers on the route.
    for t in &sim.drones.triggers {
        if t.out == 0x40 && t.kind == 2 {
            eprintln!("[p0c] the ch-0x40 band volume pl{} lo {:?} hi {:?}", t.placement, t.lo, t.hi);
        }
    }
}
