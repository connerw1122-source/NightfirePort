//! Throwaway probe (fix-swim-vent3). READ-ONLY: why does Mayhew stall 3.6 short of the 07000001
//! exit plate (−33.105, 2.301, 15.429)? The disc HAS the descending step pads (obj 137, tops
//! stored-up: y 3.959@z12.2 .. y 1.809@z15.4 .. y 0.643@z18.7). This casts the same rays the
//! sim's walk paths use over the stall → plate corridor and prints every hit, so the missing
//! query shows itself. Also probes the 0700000c ST-1 plate #506 area when given its box.
//!
//! Run: cargo run --offline --release -p nf-view --example probe_pad -- <iso> <level> x0 x1 z0 z1 [ytop ybot]

fn main() {
    let mut args = std::env::args().skip(1);
    let iso = args.next().unwrap_or_else(|| std::env::var("NF_ISO").unwrap_or_default());
    let level = args.next().unwrap_or_else(|| "07000001.bin".into());
    let nums: Vec<f32> = args.by_ref().filter_map(|s| s.parse().ok()).collect();
    let (x0, x1, z0, z1) = (nums[0], nums[1], nums[2], nums[3]);
    let (yt, yb) = if nums.len() >= 6 { (nums[4], nums[5]) } else { (5.0, -4.0) };
    eprintln!("loading {iso} {level} …");
    let scene = nf_view::load(&iso, &level, false).expect("load");
    let sim = scene.sim.as_ref().expect("sim");

    let mut x = x0;
    while x <= x1 {
        let mut z = z0;
        while z <= z1 {
            let from = [x, yt, z];
            let to = [x, yb, z];
            let a = sim.world.segment_surface(from, to, false);
            let b = sim.world.segment_surface(from, to, true);
            let c = sim.world.segment(from, to).map(|t| from[1] + (to[1] - from[1]) * t);
            let d = sim.world.segment_hit(from, to);
            let fmt = |h: &Option<(f32, [f32; 3], u8)>| match h {
                Some((t, n, m)) => format!("y {:.2} n [{:.2} {:.2} {:.2}] m {:02x}", from[1] + (to[1] - from[1]) * t, n[0], n[1], n[2], m),
                None => "—".into(),
            };
            println!("({x:.2},{z:.2}) surface[now] {} surface[nowater] {} segment y {} seg_hit {}", fmt(&a), fmt(&b),
                c.map_or("—".into(), |y| format!("{y:.2}")), d.map_or("—".into(), |(t, n)| format!("y {:.2} n[{:.2}]", from[1] + (to[1] - from[1]) * t, n[1])));
            z += 1.0;
        }
        x += 1.0;
    }

    // The boundary walls (Network::build's posts, nav.rs:66) near the query box: the port's AI
    // bounds come from an UP ray per boundary node — one-sided now, so the post heights moved.
    println!("--- boundary walls near x {x0}..{x1} z {z0}..{z1} ---");
    for w in &sim.nav.walls {
        let inb = |p: [f32; 3]| p[0] >= x0 - 2.0 && p[0] <= x1 + 2.0 && p[2] >= z0 - 2.0 && p[2] <= z1 + 2.0;
        if inb(w.a) || inb(w.b) {
            println!("wall a ({:.2},{:.2},{:.2}) h {:.1} — b ({:.2},{:.2},{:.2}) h {:.1} flags {:x}", w.a[0], w.a[1], w.a[2], w.ha, w.b[0], w.b[1], w.b[2], w.hb, w.flags);
        }
    }

    // move_by's probes at Mayhew's stall pose (drone.rs move_by): the walk step toward the route
    // target, probed at pos.y and pos.y+0.5.
    let pos = [nums.get(6).copied().unwrap_or(-33.015), 4.539, nums.get(7).copied().unwrap_or(11.80)];
    let tgt = [nums.get(8).copied().unwrap_or(-33.112), 0.0, nums.get(9).copied().unwrap_or(14.347)];
    let dx = tgt[0] - pos[0]; let dz = tgt[2] - pos[2];
    let l = (dx * dx + dz * dz).sqrt();
    let step = [dx / l * 0.02, dz / l * 0.02];
    let to = [pos[0] + step[0], pos[1], pos[2] + step[1]];
    let sgn = |v: f32| if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 };
    let end = [to[0] + sgn(step[0]) * 0.4, to[1], to[2] + sgn(step[1]) * 0.4];
    for y in [pos[1], pos[1] + 0.5] {
        let hit = sim.world.segment([pos[0], y, pos[2]], [end[0], y, end[2]]);
        println!("move_by probe y {y}: from ({:.3},{y},{:.3}) to ({:.3},{y},{:.3}) -> {}", pos[0], pos[2], end[0], end[2],
            hit.map_or("CLEAR".into(), |t| format!("HIT at t {:.3}", t)));
    // feet_on_point at the stall pose.
    let fp = sim.world.segment([pos[0], pos[1], pos[2]], [pos[0], pos[1] - 1.6, pos[2]]);
    println!("feet_on_point: {}", fp.map_or("MISS".into(), |t| format!("floor y {:.3}", pos[1] - 1.6 * t)));
}

    // follow()'s block test at the stall pose: bounds_node_test(0.2, feet, target) over ALL walls.
    let feet = [pos[0], pos[1] - 1.0, pos[2]];
    let tgt = [tgt[0], 2.676, tgt[2]];
    let blocked = !sim.nav.bounds_node_test(0.2, feet, tgt);
    println!("follow block test feet {:?} -> target {:?}: blocked {}", feet, tgt, blocked);
    for w in sim.nav.walls.iter().filter(|w| {
        let n = [w.a[0], w.a[1], w.a[2]];
        (n[0] - feet[0]).powi(2) + (n[2] - feet[2]).powi(2) < 25.0
    }) {
        println!("  near wall a ({:.2},{:.2},{:.2}) h {:.1} b ({:.2},{:.2},{:.2}) h {:.1} flags {:x}", w.a[0], w.a[1], w.a[2], w.ha, w.b[0], w.b[1], w.b[2], w.hb, w.flags);
    }
}
